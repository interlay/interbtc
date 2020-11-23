use crate::types::*;
use crate::Error;
use crate::Script;
use codec::{Decode, Encode};
use sp_core::H160;
use sp_std::str::FromStr;
use sp_std::vec::Vec;
#[cfg(feature = "std")]
use sp_std::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Encode, Decode, Clone, Ord, PartialOrd, PartialEq, Eq, Debug, Copy)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum Payload {
    P2SH(H160),
    P2PKH(H160),
    P2WPKH(u8, H160),
}

impl Payload {
    pub fn from_script(script: &Script) -> Result<Self, Error> {
        if script.is_p2sh() {
            // 0xa9 (OP_HASH160) - 0x14 (20 bytes hash) - <20 bytes script hash> - 0x87 (OP_EQUAL)
            Ok(Self::P2SH(H160::from_slice(&script.as_bytes()[2..22])))
        } else if script.is_p2pkh() {
            // 0x76 (OP_DUP) - 0xa9 (OP_HASH160) - 0x14 (20 bytes len) - <20 bytes pubkey hash> - 0x88 (OP_EQUALVERIFY) - 0xac (OP_CHECKSIG)
            Ok(Self::P2PKH(H160::from_slice(&script.as_bytes()[3..23])))
        } else if script.is_p2wpkh() {
            Ok(Self::P2WPKH(
                // first byte is version
                script.as_bytes()[0],
                // 0x14 (20 bytes len) - <20 bytes hash>
                H160::from_slice(&script.as_bytes()[2..]),
            ))
        } else {
            Err(Error::InvalidBtcAddress)
        }
    }

    pub fn to_script(&self) -> Script {
        match self {
            Payload::P2SH(script_hash) => {
                let mut script = Script::new();
                script.append(OpCode::OpHash160);
                script.append(HASH160_SIZE_HEX);
                script.append(script_hash);
                script.append(OpCode::OpEqual);
                script
            }
            Payload::P2PKH(pub_key_hash) => {
                let mut script = Script::new();
                script.append(OpCode::OpDup);
                script.append(OpCode::OpHash160);
                script.append(HASH160_SIZE_HEX);
                script.append(pub_key_hash);
                script.append(OpCode::OpEqualVerify);
                script.append(OpCode::OpCheckSig);
                script
            }
            Payload::P2WPKH(_, pub_key_hash) => {
                let mut script = Script::new();
                script.append(OpCode::Op0);
                script.append(pub_key_hash);
                script
            }
        }
    }

    pub fn hash(&self) -> H160 {
        match *self {
            Payload::P2SH(hash) => hash,
            Payload::P2PKH(hash) => hash,
            Payload::P2WPKH(_, hash) => hash,
        }
    }

    #[cfg(feature = "std")]
    pub fn random() -> Self {
        Payload::P2SH(H160::random())
    }
}

impl Default for Payload {
    fn default() -> Self {
        Self::P2SH(H160::zero())
    }
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, Copy)]
#[cfg_attr(feature = "std", derive(serde::Serialize))]
pub enum Network {
    Mainnet,
    Testnet,
    Regtest,
}

impl Default for Network {
    fn default() -> Self {
        Network::Mainnet
    }
}

#[derive(Encode, Decode, Default, Clone, PartialEq, Eq, Debug, Copy)]
#[cfg_attr(feature = "std", derive(serde::Serialize))]
pub struct Address {
    pub payload: Payload,
    pub network: Network,
}

impl Address {
    pub fn new_p2sh(hash: H160, network: Network) -> Self {
        Self {
            payload: Payload::P2SH(hash),
            network,
        }
    }

    pub fn new_p2pkh(hash: H160, network: Network) -> Self {
        Self {
            payload: Payload::P2PKH(hash),
            network,
        }
    }

    pub fn new_p2wpkh(version: u8, hash: H160, network: Network) -> Self {
        Self {
            payload: Payload::P2WPKH(version, hash),
            network,
        }
    }
}

/// Extract the bech32 address prefix which describes the network
fn find_bech32_prefix(bech32: &str) -> &str {
    match bech32.rfind('1') {
        None => bech32,
        Some(sep) => bech32.split_at(sep).0,
    }
}

/// Decodes an address string, only used for tests
impl FromStr for Address {
    type Err = Error;

    // See: https://github.com/rust-bitcoin/rust-bitcoin/blob/926cff0741dd0dc177ecf92b289020a593406f6f/src/util/address.rs#L405
    fn from_str(addr: &str) -> Result<Self, Self::Err> {
        let bech32_network = match find_bech32_prefix(addr) {
            "bc" | "BC" => Some(Network::Mainnet),
            "tb" | "TB" => Some(Network::Testnet),
            "bcrt" | "BCRT" => Some(Network::Regtest),
            _ => None,
        };

        if let Some(network) = bech32_network {
            let (_, payload) = bech32::decode(addr)?;
            if payload.is_empty() {
                return Err(Error::EmptyBech32Payload);
            }

            let (version, program): (bech32::u5, Vec<u8>) = {
                let (v, p5) = payload.split_at(1);
                (v[0], bech32::FromBase32::from_base32(p5)?)
            };

            if version.to_u8() > 16 {
                return Err(Error::InvalidWitnessVersion(version.to_u8()));
            }

            if program.len() < 2 || program.len() > 40 {
                return Err(Error::InvalidWitnessProgramLength(program.len()));
            }

            if version.to_u8() == 0 && (program.len() != 20 && program.len() != 32) {
                return Err(Error::InvalidSegWitV0ProgramLength(program.len()));
            }

            return Ok(Self {
                payload: Payload::P2WPKH(version.to_u8(), H160::from_slice(&program[..])),
                network,
            });
        }

        let mut data = [0u8; 32];
        let len = bs58::decode(addr).with_check(None).into(&mut data)?;

        // https://en.bitcoin.it/wiki/List_of_address_prefixes
        match data[0] {
            // 0x00 - Pubkey hash
            0 => Ok(Self {
                payload: Payload::P2PKH(H160::from_slice(&data[1..len])),
                network: Network::Mainnet,
            }),
            // 0x05 - Script hash
            5 => Ok(Self {
                payload: Payload::P2SH(H160::from_slice(&data[1..len])),
                network: Network::Mainnet,
            }),
            // 0x6F - Testnet pubkey hash
            111 => Ok(Self {
                payload: Payload::P2PKH(H160::from_slice(&data[1..len])),
                network: Network::Testnet,
            }),
            // 0xC4 - Testnet script hash
            196 => Ok(Self {
                payload: Payload::P2SH(H160::from_slice(&data[1..len])),
                network: Network::Testnet,
            }),
            _ => Err(Error::InvalidBtcAddress),
        }
    }
}

#[cfg(feature = "std")]
impl Display for Address {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        match self.payload {
            Payload::P2SH(hash) => {
                let mut prefixed = [0; 21];
                prefixed[0] = match self.network {
                    Network::Mainnet => 5,
                    Network::Testnet | Network::Regtest => 196,
                };
                prefixed[1..].copy_from_slice(&hash[..]);
                fmt.write_str(&bs58::encode(&prefixed[..]).with_check().into_string())
            }
            Payload::P2PKH(hash) => {
                let mut prefixed = [0; 21];
                prefixed[0] = match self.network {
                    Network::Mainnet => 0,
                    Network::Testnet | Network::Regtest => 111,
                };
                prefixed[1..].copy_from_slice(&hash[..]);
                fmt.write_str(&bs58::encode(&prefixed[..]).with_check().into_string())
            }
            Payload::P2WPKH(version, hash) => {
                let hrp = match self.network {
                    Network::Mainnet => "bc",
                    Network::Testnet => "tb",
                    Network::Regtest => "bcrt",
                };
                let mut bech32_writer = bech32::Bech32Writer::new(hrp, fmt)?;
                bech32::WriteBase32::write_u5(
                    &mut bech32_writer,
                    bech32::u5::try_from_u8(version).unwrap(),
                )?;
                bech32::ToBase32::write_base32(&hash, &mut bech32_writer)
            }
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    macro_rules! assert_err {
        ($result:expr, $err:pat) => {{
            match $result {
                Err($err) => (),
                Ok(v) => panic!("assertion failed: Ok({:?})", v),
                _ => panic!("expected: Err(_)"),
            }
        }};
    }

    #[test]
    fn should_not_decode_invalid_address() {
        assert_err!(
            Address::from_str(&"000000000000"),
            Error::Base58(bs58::decode::Error::InvalidCharacter {
                character: '0',
                index: 0
            })
        );

        assert_err!(
            Address::from_str(&"3EktnHQD7RiAE6uzMj2ZifT9YgRrkS"),
            Error::Base58(bs58::decode::Error::InvalidChecksum {
                checksum: [57, 144, 213, 176],
                expected_checksum: [123, 155, 216, 75]
            })
        );
    }

    fn assert_address_eq(address: &str, decoded: Address) {
        assert_eq!(Address::from_str(address).unwrap(), decoded);
        assert_eq!(
            Address::from_str(address).unwrap().to_string(),
            address.to_string()
        );
    }

    #[test]
    fn should_decode_valid_address() {
        // Mainnet P2SH
        assert_address_eq(
            "3EktnHQD7RiAE6uzMj2ZifT9YgRrkSgzQX",
            Address {
                payload: Payload::P2SH(H160::from_slice(&[
                    143, 85, 86, 59, 154, 25, 243, 33, 194, 17, 233, 185, 243, 140, 223, 104, 110,
                    160, 120, 69,
                ])),
                network: Network::Mainnet,
            },
        );

        // Mainnet P2PKH
        assert_address_eq(
            "17VZNX1SN5NtKa8UQFxwQbFeFc3iqRYhem",
            Address {
                payload: Payload::P2PKH(H160::from_slice(&[
                    71, 55, 108, 111, 83, 125, 98, 23, 122, 44, 65, 196, 202, 155, 69, 130, 154,
                    185, 144, 131,
                ])),
                network: Network::Mainnet,
            },
        );

        // Mainnet P2WPKH
        assert_address_eq(
            "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4",
            Address {
                payload: Payload::P2WPKH(
                    0,
                    H160::from_slice(&[
                        117, 30, 118, 232, 25, 145, 150, 212, 84, 148, 28, 69, 209, 179, 163, 35,
                        241, 67, 59, 214,
                    ]),
                ),
                network: Network::Mainnet,
            },
        );

        // Testnet P2SH
        assert_address_eq(
            "2MzQwSSnBHWHqSAqtTVQ6v47XtaisrJa1Vc",
            Address {
                payload: Payload::P2SH(H160::from_slice(&[
                    78, 159, 57, 202, 70, 136, 255, 16, 33, 40, 234, 76, 205, 163, 65, 5, 50, 67,
                    5, 176,
                ])),
                network: Network::Testnet,
            },
        );

        // Testnet P2PKH
        assert_address_eq(
            "mipcBbFg9gMiCh81Kj8tqqdgoZub1ZJRfn",
            Address {
                payload: Payload::P2PKH(H160::from_slice(&[
                    36, 63, 19, 148, 244, 69, 84, 244, 206, 63, 214, 134, 73, 193, 154, 220, 72,
                    60, 233, 36,
                ])),
                network: Network::Testnet,
            },
        );

        // Testnet P2WPKH
        assert_address_eq(
            "tb1qw508d6qejxtdg4y5r3zarvary0c5xw7kxpjzsx",
            Address {
                payload: Payload::P2WPKH(
                    0,
                    H160::from_slice(&[
                        117, 30, 118, 232, 25, 145, 150, 212, 84, 148, 28, 69, 209, 179, 163, 35,
                        241, 67, 59, 214,
                    ]),
                ),
                network: Network::Testnet,
            },
        );
    }
}
