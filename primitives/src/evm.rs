use crate::CurrencyId;
use codec::{Decode, Encode, Error as CodecError};
use primitive_types::H160;

/// EVM Address
pub type EvmAddress = H160;

const CURRENCY_PREFIX_LEN: usize = 4;
/// Currency address prefix
pub const CURRENCY_ADDRESS_PREFIX: [u8; CURRENCY_PREFIX_LEN] = *b"cap/";

pub fn is_currency_precompile(address: &EvmAddress) -> bool {
    address.as_bytes().starts_with(&CURRENCY_ADDRESS_PREFIX)
}

/// CurrencyId to H160([u8; 20]) encoding
impl From<CurrencyId> for EvmAddress {
    fn from(currency_id: CurrencyId) -> Self {
        let mut address = [0u8; 20];
        let encoded = currency_id.encode();
        address[0..CURRENCY_PREFIX_LEN].copy_from_slice(&CURRENCY_ADDRESS_PREFIX);
        address[CURRENCY_PREFIX_LEN..CURRENCY_PREFIX_LEN + encoded.len()].copy_from_slice(&encoded);
        EvmAddress::from_slice(&address)
    }
}

/// H160([u8; 20]) to CurrencyId decoding
impl TryFrom<EvmAddress> for CurrencyId {
    type Error = CodecError;

    fn try_from(evm_address: EvmAddress) -> Result<Self, Self::Error> {
        if !is_currency_precompile(&evm_address) {
            return Err(CodecError::from("Not currency precompile"));
        }
        CurrencyId::decode(&mut &evm_address.as_bytes()[CURRENCY_PREFIX_LEN..H160::len_bytes()])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LpToken, TokenSymbol};
    use codec::MaxEncodedLen;

    #[test]
    fn encode_currency_id_as_evm_address() {
        let currency_ids = vec![
            CurrencyId::Token(TokenSymbol::DOT),
            CurrencyId::Token(TokenSymbol::IBTC),
            CurrencyId::Token(TokenSymbol::INTR),
            CurrencyId::Token(TokenSymbol::KSM),
            CurrencyId::Token(TokenSymbol::KBTC),
            CurrencyId::Token(TokenSymbol::KINT),
            CurrencyId::ForeignAsset(u32::MAX),
            CurrencyId::LendToken(u32::MAX),
            CurrencyId::LpToken(LpToken::Token(TokenSymbol::IBTC), LpToken::Token(TokenSymbol::INTR)),
            CurrencyId::LpToken(LpToken::ForeignAsset(u32::MAX), LpToken::ForeignAsset(u32::MAX)),
            CurrencyId::LpToken(LpToken::StableLpToken(u32::MAX), LpToken::StableLpToken(u32::MAX)),
            CurrencyId::StableLpToken(u32::MAX),
        ];

        let max_encoded_len = currency_ids
            .iter()
            .map(|currency_id| currency_id.encode().len())
            .max()
            .unwrap();

        assert_eq!(max_encoded_len, CurrencyId::max_encoded_len());
        assert!(
            max_encoded_len < H160::len_bytes() - CURRENCY_PREFIX_LEN,
            "Currency cannot be encoded to address"
        );

        for currency_id in currency_ids {
            assert_eq!(
                currency_id,
                CurrencyId::try_from(EvmAddress::from(currency_id)).unwrap(),
            );
        }
    }
}
