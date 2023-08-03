#![cfg_attr(not(feature = "std"), no_std, no_main)]

use bitcoin::{
    compat::ConvertFromInterlayBitcoin,
    types::{FullTransactionProof, Transaction},
};
use brc21::{Brc21, Brc21Operation};
use ink::{env::Environment, prelude::vec::Vec};
    use ord::Inscription;

mod brc21;
mod ord;

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum CustomEnvironment {}

impl Environment for CustomEnvironment {
    const MAX_EVENT_TOPICS: usize = <ink::env::DefaultEnvironment as Environment>::MAX_EVENT_TOPICS;

    type AccountId = <ink::env::DefaultEnvironment as Environment>::AccountId;
    type Balance = <ink::env::DefaultEnvironment as Environment>::Balance;
    type Hash = <ink::env::DefaultEnvironment as Environment>::Hash;
    type BlockNumber = <ink::env::DefaultEnvironment as Environment>::BlockNumber;
    type Timestamp = <ink::env::DefaultEnvironment as Environment>::Timestamp;

    type ChainExtension = DoSomethingInRuntime;
}

#[ink::chain_extension]
pub trait DoSomethingInRuntime {
    type ErrorCode = RuntimeErr;

    /// Note: this gives the operation a corresponding `func_id` (1101 in this case),
    /// and the chain-side chain extension will get the `func_id` to do further operations.
    #[ink(extension = 1101)]
    fn get_and_verify_bitcoin_payment(full_proof: FullTransactionProof, address: Vec<u8>) -> Option<u64>;
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum RuntimeErr {
    SomeFailure,
}

impl ink::env::chain_extension::FromStatusCode for RuntimeErr {
    fn from_status_code(status_code: u32) -> Result<(), Self> {
        match status_code {
            0 => Ok(()),
            1 => Err(Self::SomeFailure),
            _ => panic!("encountered unknown status code"),
        }
    }
}

/// Creates a swap contract where Alice locks DOT with a price limit in a contract that
/// Bob can acquire by sending BTC on the Bitcoin chain that Alice provided.
///
/// Note: this is a proof of concept protocol and should not be used in production due to flaws in
/// the protocol and implementation.
///
/// ## Protocol
///
/// - Alice provides a BTC address, a price limit, and a DOT amount to lock in the contract.
/// - Bob sends BTC to the address provided by Alice.
/// - Anyone (Alice, Bob, or a third party) provides a BTC transaction proof to the contract.
/// The proof triggers a DOT transfer from the contract to Bob.
#[ink::contract(env = crate::CustomEnvironment)]
mod btc_swap {
    use super::*;
    use bitcoin::Address as BtcAddress;
    use ink::storage::Mapping;
    use scale::Encode;

    /// Defines the limit order contract
    #[derive(scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(Debug, PartialEq, Eq, scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct LimitOrder {
        /// The BTC address to send BTC to
        /// can't store as `BtcAddress`, since `StorageLayout` is not implemented, and we can't derive it
        /// due to the hashes inside it not implementing `StorageLayout`
        btc_address: Vec<u8>,
        /// The price limit for the BTC denoted in satoshis
        min_satoshis: u64,
        /// The DOT amount to lock in the contract denoted in planck
        plancks: u128,
    }

    /// Defines the storage of your contract.
    /// Add new fields to the below struct in order
    /// to add new static storage fields to your contract.
    #[ink(storage)]
    pub struct BtcSwap {
        /// Stores a mapping from an account to a limit order.
        orders: ink::storage::Mapping<AccountId, LimitOrder>,
    }

    impl BtcSwap {
        #[ink(constructor)]
        pub fn new() -> Self {
            let orders = Mapping::default();
            Self { orders }
        }

        #[ink(message, payable)]
        pub fn create_trade(&mut self, btc_address: BtcAddress, min_satoshis: u64) {
            assert!(min_satoshis > 0);

            let caller = self.env().caller();
            let offer = self.env().transferred_value();

            let order = LimitOrder {
                btc_address: btc_address.encode(),
                min_satoshis,
                plancks: offer,
            };
            self.orders.insert(caller, &order);
        }

        #[ink(message)]
        pub fn execute_trade(&mut self, counterparty: AccountId, full_proof: FullTransactionProof) {
            let caller = self.env().caller();
            let order = self.orders.get(&counterparty).unwrap();

            let transferred_sats = self
                .env()
                .extension()
                .get_and_verify_bitcoin_payment(full_proof, order.btc_address)
                .unwrap()
                .unwrap_or(0);

            assert!(transferred_sats >= order.min_satoshis);

            self.env().transfer(caller, order.plancks).unwrap();
        }

        #[ink(message)]
        pub fn brc21_test(&mut self, interlay_tx: Transaction) {
            let tx = interlay_tx.to_rust_bitcoin().unwrap();
            let has_taproot_outputs = tx.output.iter().any(|x| x.script_pubkey.is_v1_p2tr());
            if has_taproot_outputs {
                self.env().transfer(self.env().caller(), 1).unwrap();
            } else {
                self.env().transfer(self.env().caller(), 2).unwrap();
            }

            let inscriptions = Inscription::from_transaction(&tx);

            let body_bytes = inscriptions[0].clone().inscription.into_body().unwrap();
            let parsed: Brc21 = serde_json::from_slice(&body_bytes).unwrap();

            match parsed {
                Brc21 {
                    op: Brc21Operation::Transfer { amt },
                    tick,
                } => {
                    todo!()
                }
                Brc21 {
                    op: Brc21Operation::Mint { amt, src },
                    tick,
                } => {
                    todo!()
                }
                Brc21 {
                    op: Brc21Operation::Redeem { acc, amt, dest },
                    tick,
                } => {
                    todo!()
                }
                Brc21 {
                    op: Brc21Operation::Deploy { id, max, src },
                    tick,
                } => {
                    todo!()
                }
            }
        }
    }
}
