#![cfg_attr(not(feature = "std"), no_std, no_main)]

/// Creates a swap contract where Alice locks DOT with a price limit in a contract that
/// Bob can a acquire by sending BTC on the Bitcoin chain that Alice provided.
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
#[ink::contract]
mod btc_swap {
    use ink::storage::Mapping;

    use bitcoin::{Address as BtcAddress};
    use bitcoin::types::{MerkleProof, Transaction};

    /// Defines the limit order contract
    #[derive(scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(
            Debug,
            PartialEq,
            Eq,
            scale_info::TypeInfo,
            ink::storage::traits::StorageLayout
        )
    )]
    pub struct LimitOrder {
        /// The BTC address to send BTC to
        /// FIXME: trait bound not satisfied
        btc_address: BtcAddress,
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
                btc_address,
                min_satoshis,
                plancks: offer,
            };
            self.orders.insert(caller, &order);
        }

        #[ink(message)]
        pub fn execute_trade(
            &mut self,
            counterparty: AccountId,
            merkle_proof: MerkleProof,
            transaction: Transaction,
            length_bound: u32
        ) {
            let caller = self.env().caller();
            let order = self.orders.get(&counterparty).unwrap();
            

            // FIXME: How to call the BTC relay?
            let transferred_sats = btc_relay::get_and_verify_bitcoin_payment(
                merkle_proof,
                transaction,
                length_bound,
                order.btc_address,
            );

            assert!(transferred_sats >= order.min_satoshis);

            self.env().transfer(caller, order.plancks)
        }
    }
}