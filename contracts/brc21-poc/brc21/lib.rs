#![cfg_attr(not(feature = "std"), no_std, no_main)]

use bitcoin::{
    compat::ConvertFromInterlayBitcoin,
    types::{FullTransactionProof, Transaction as InterlayTransaction},
};

use brc21_inscription::{get_brc21_inscriptions, Brc21Inscription, Brc21Operation};
use ink::{env::Environment, prelude::vec::Vec};
use ord::Inscription;

mod brc21_inscription;
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

#[ink::chain_extension]
pub trait DoSomethingInRuntime {
    type ErrorCode = RuntimeErr;

    /// Note: this gives the operation a corresponding `func_id` (1101 in this case),
    /// and the chain-side chain extension will get the `func_id` to do further operations.
    #[ink(extension = 1101)]
    fn get_and_verify_bitcoin_payment(full_proof: FullTransactionProof, address: Vec<u8>) -> Option<u64>;

    #[ink(extension = 1102)]
    fn verify_inclusion(full_proof: &FullTransactionProof) -> bool;
}

/// A POC implementation for the BRC-21 Token Standard
///
/// ## Minting
///
/// 1. Mint the locked tokens on Bitcoin via an inscription
/// 2. Lock the underlying token in this contract and proof the the inscription locks the same amount of tokens
/// Indexers now accept the Bitcoin-minted BRC21 as minted
///
/// ## Redeeming
///
/// 1. Redeem BRC21 on Bitcoin
/// 2. Proof BRC21 redeem to this contract and unlock tokens
#[ink::contract(env = crate::CustomEnvironment)]
mod brc21 {
    use crate::brc21_inscription::Brc21Inscription;
    use ink::{prelude::string::String};

    use super::*;

    #[ink(event)]
    pub struct Mint {
        /// Token ticker
        ticker: String,
        /// Token amount
        amount: u128,
        /// Account that minted the tokens
        #[ink(topic)]
        account: AccountId, /* Bitcoin inscription transaction id
                             * TODO: add to event
                             * #[ink(topic)]
                             * inscription_tx_id: Vec<u8> */
    }

    #[ink(event)]
    pub struct Redeem {
        /// Token ticker
        ticker: String,
        /// Token amount
        amount: u128,
        /// Account that redeemed the tokens
        #[ink(topic)]
        account: AccountId, /* Bitcoin redeem transaction id
                             * TODO: add to event
                             * #[ink(topic)]
                             * redeem_tx_id: Vec<u8> */
    }

    #[ink(storage)]
    pub struct Brc21 {
        /// Ticker of the token, assuming one BRC21 contract per token
        ticker: String,
        /// Locked tokens
        locked: u128,
    }

    impl Brc21 {
        /// Constructor that initializes the locks to their default value
        #[ink(constructor, payable)]
        pub fn new(ticker: String) -> Self {
            let locked = 0;
            Self { ticker, locked }
        }

        /// Returns the token ticker
        #[ink(message)]
        pub fn get_ticker(&self) -> String {
            self.ticker.clone()
        }

        /// Returns the currently locked tokens
        #[ink(message)]
        pub fn get_locked(&self) -> u128 {
            self.locked
        }

        /// Lock tokens to an account and validate the minting on Bitcoin
        /// - Lock the tokens of an account
        /// - Ensure that the inscription op is "mint"
        /// - Ensure that the inscription ticker matches the token ticker
        /// - Ensure that the inscription locks the same amount of tokens
        /// - Ensure that the source chain is "INTERLAY"
        #[ink(message, payable)]
        pub fn mint(&mut self, full_proof: FullTransactionProof) {
            let is_included = self.env().extension().verify_inclusion(&full_proof).unwrap();
            assert!(is_included);

            let tx = full_proof.user_tx_proof.transaction.to_rust_bitcoin().unwrap();

            let brc21_inscriptions = get_brc21_inscriptions(&tx);

            for inscription in brc21_inscriptions {
                if let Brc21Inscription {
                    op: Brc21Operation::Mint { amount, src },
                    tick,
                } = inscription
                {
                    if tick != self.ticker || src != "INTERLAY" {
                        continue;
                    }

                    self.locked += amount as u128;

                    self.env().emit_event(Mint {
                        ticker: self.ticker.clone(),
                        amount: amount as u128,
                        account: self.env().caller(),
                        // inscription_tx_id: Vec::new(),
                    });
                }
            }
        }

        /// Unlock tokens to an account and decrease their lock amount
        ///
        /// TODO: add the inscription parsing
        /// TODO: add the BTC relay arguments
        #[ink(message, payable)]
        pub fn redeem(&mut self, account: AccountId, full_proof: FullTransactionProof) {
            let is_included = self.env().extension().verify_inclusion(&full_proof).unwrap();
            assert!(is_included);

            let tx = full_proof.user_tx_proof.transaction.to_rust_bitcoin().unwrap();

            let brc21_inscriptions = get_brc21_inscriptions(&tx);

            for inscription in brc21_inscriptions {
                if let Brc21Inscription {
                    op: Brc21Operation::Redeem { amount, dest, acc },
                    tick,
                } = inscription
                {
                    if tick != self.ticker || dest != "INTERLAY" {
                        continue;
                    }

                    let mut account_bytes = [0u8; 32];
                    if hex::decode_to_slice(acc, &mut account_bytes as &mut [u8]).is_ok() {
                        if let Ok(account) = TryFrom::try_from(account_bytes) {
                            assert!(self.locked >= amount as u128, "not enough locked tokens");

                            self.env().emit_event(Redeem {
                                ticker: self.ticker.clone(),
                                amount: amount as u128,
                                account,
                                // redeem_tx_id: Vec::new(),
                            });
                        }
                    }
                }
            }
        }
    }

    /// Unit tests in Rust are normally defined within such a `#[cfg(test)]`
    /// module and test functions are marked with a `#[test]` attribute.
    /// The below code is technically just normal Rust code.
    #[cfg(test)]
    mod tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;

        // Define event types used by this contract
        type Event = <Brc21 as ::ink::reflect::ContractEventBase>::Type;

        const DEFAULT_TICKER: &str = "INTR";

        fn decode_event(event: &ink::env::test::EmittedEvent) -> Event {
            <Event as scale::Decode>::decode(&mut &event.data[..]).expect("encountered invalid contract event data")
        }

        /// Helper function to for mint event tests
        fn assert_mint_event(event: &ink::env::test::EmittedEvent, ticker: &str, amount: u128, account: AccountId) {
            let decoded_event = decode_event(event);
            match decoded_event {
                Event::Mint(mint) => {
                    assert_eq!(mint.ticker, ticker);
                    assert_eq!(mint.amount, amount);
                    assert_eq!(mint.account, account);
                }
                _ => panic!("Expected Mint event"),
            }
        }

        /// Helper function to for redeem event tests
        fn assert_redeem_event(event: &ink::env::test::EmittedEvent, ticker: &str, amount: u128, account: AccountId) {
            let decoded_event = decode_event(event);
            match decoded_event {
                Event::Redeem(redeem) => {
                    assert_eq!(redeem.ticker, ticker);
                    assert_eq!(redeem.amount, amount);
                    assert_eq!(redeem.account, account);
                }
                _ => panic!("Expected Redeem event"),
            }
        }

        /// Test if the default constructor does its job.
        #[ink::test]
        fn new_works() {
            let brc21 = Brc21::new(DEFAULT_TICKER.to_string());
            assert_eq!(brc21.get_ticker(), DEFAULT_TICKER);
            assert_eq!(brc21.get_locked(), 0);
        }

//         /// Test if minting works
//         #[ink::test]
//         fn mint_works() {
//             let mut brc21 = Brc21::new(DEFAULT_TICKER.to_string());
// 
//             // Load the default accounts
//             let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
// 
//             // Alice mints 100 coins
//             // Default caller is the Alice account 0x01
//             brc21.mint(100);
//             assert_eq!(brc21.get_locked(), 100);
// 
//             // Check that the event was emitted
//             let emitted_events = ink::env::test::recorded_events().collect::<Vec<_>>();
//             assert_eq!(emitted_events.len(), 1);
//             assert_mint_event(
//                 &emitted_events[0],
//                 DEFAULT_TICKER,
//                 100,
//                 AccountId::from([0x01; 32]), // Alice
//             );
// 
//             // Bob mints 50 coins
//             ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
//             brc21.mint(50);
//             assert_eq!(brc21.get_locked(), 150);
// 
//             // Check that the event was emitted
//             let emitted_events = ink::env::test::recorded_events().collect::<Vec<_>>();
//             assert_eq!(emitted_events.len(), 2);
//             assert_mint_event(
//                 &emitted_events[1],
//                 DEFAULT_TICKER,
//                 50,
//                 AccountId::from([0x02; 32]), // Bob
//             );
//         }
// 
//         /// Test if redeeming works
//         #[ink::test]
//         fn redeem_works() {
//             let mut brc21 = Brc21::new(DEFAULT_TICKER.to_string());
// 
//             // Load the default accounts
//             let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
// 
//             // Alice mints 100 coins
//             // Default caller is the Alice account 0x01
//             brc21.mint(100);
//             assert_eq!(brc21.get_locked(), 100);
// 
//             // Bob redeems 50 coins
//             ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
//             brc21.redeem(accounts.bob, 50);
//             assert_eq!(brc21.get_locked(), 50);
// 
//             // Check that the event was emitted
//             let emitted_events = ink::env::test::recorded_events().collect::<Vec<_>>();
//             assert_eq!(emitted_events.len(), 2);
//             assert_redeem_event(
//                 &emitted_events[1],
//                 DEFAULT_TICKER,
//                 50,
//                 AccountId::from([0x02; 32]), // Bob
//             );
//         }
    }

    /// This is how you'd write end-to-end (E2E) or integration tests for ink! contracts.
    ///
    /// When running these you need to make sure that you:
    /// - Compile the tests with the `e2e-tests` feature flag enabled (`--features e2e-tests`)
    /// - Are running a Substrate node which contains `pallet-contracts` in the background
    #[cfg(all(test, feature = "e2e-tests"))]
    mod e2e_tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;

        /// A helper function used for calling contract messages.
        use ink_e2e::build_message;

        /// The End-to-End test `Result` type.
        type E2EResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

        /// We test that we can upload and instantiate the contract using its default constructor.
        #[ink_e2e::test]
        async fn default_works(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
            // Given
            let constructor = Brc21Ref::default();

            // When
            let contract_account_id = client
                .instantiate("brc21", &ink_e2e::alice(), constructor, 0, None)
                .await
                .expect("instantiate failed")
                .account_id;

            // Then
            let get = build_message::<Brc21Ref>(contract_account_id.clone()).call(|brc21| brc21.get());
            let get_result = client.call_dry_run(&ink_e2e::alice(), &get, 0, None).await;
            assert!(matches!(get_result.return_value(), false));

            Ok(())
        }

        /// We test that we can read and write a value from the on-chain contract contract.
        #[ink_e2e::test]
        async fn it_works(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
            // Given
            let constructor = Brc21Ref::new(false);
            let contract_account_id = client
                .instantiate("brc21", &ink_e2e::bob(), constructor, 0, None)
                .await
                .expect("instantiate failed")
                .account_id;

            let get = build_message::<Brc21Ref>(contract_account_id.clone()).call(|brc21| brc21.get());
            let get_result = client.call_dry_run(&ink_e2e::bob(), &get, 0, None).await;
            assert!(matches!(get_result.return_value(), false));

            // When
            let flip = build_message::<Brc21Ref>(contract_account_id.clone()).call(|brc21| brc21.flip());
            let _flip_result = client.call(&ink_e2e::bob(), flip, 0, None).await.expect("flip failed");

            // Then
            let get = build_message::<Brc21Ref>(contract_account_id.clone()).call(|brc21| brc21.get());
            let get_result = client.call_dry_run(&ink_e2e::bob(), &get, 0, None).await;
            assert!(matches!(get_result.return_value(), true));

            Ok(())
        }
    }
}
