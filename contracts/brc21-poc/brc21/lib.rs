#![cfg_attr(not(feature = "std"), no_std, no_main)]

mod brc21_inscription;
mod ord;

use bitcoin::{
    compat::{ConvertFromInterlayBitcoin, ConvertToInterlayBitcoin},
    types::FullTransactionProof,
};
use brc21_inscription::*;
use ink::{env::Environment, prelude::vec::Vec, primitives::AccountId};
use lite_json::json_parser::parse_json;
use ord::Inscription;

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
    use bitcoin::types::H256Le;
    use ink::prelude::string::String;

    use super::*;

    #[ink(event)]
    pub struct MintEvent {
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
    #[cfg_attr(test, derive(PartialEq, Debug))]
    pub struct RedeemEvent {
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

        txid: H256Le,
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
        pub fn mint(&mut self, full_proof: Vec<u8>) {
            use scale::Decode;
            let full_proof: FullTransactionProof = Decode::decode(&mut &full_proof[..]).unwrap();
            let is_included = self.env().extension().verify_inclusion(&full_proof).unwrap();
            assert!(is_included);

            self.do_mint(self.env().caller(), full_proof)
        }

        /// Unlock tokens to an account and decrease their lock amount
        ///
        /// TODO: add the inscription parsing
        /// TODO: add the BTC relay arguments
        #[ink(message, payable)]
        pub fn redeem(&mut self, full_proof: Vec<u8>) {
            use scale::Decode;
            let full_proof: FullTransactionProof = Decode::decode(&mut &full_proof[..]).unwrap();
            let is_included = self.env().extension().verify_inclusion(&full_proof).unwrap();
            assert!(is_included);

            self.do_redeem(full_proof)
        }

        pub fn do_mint(&mut self, caller: AccountId, full_proof: FullTransactionProof) {
            let tx = full_proof.user_tx_proof.transaction.to_rust_bitcoin().unwrap();

            for inscription in Inscription::from_transaction(&tx) {
                let body_bytes = inscription.inscription.into_body().unwrap();
                let body_str = core::str::from_utf8(&body_bytes).unwrap();
                if let Some(mint) = parse_mint(body_str) {
                    if mint.ticker != self.ticker || mint.src != "INTERLAY" {
                        continue;
                    }

                    self.locked += mint.amount as u128;

                    self.env().emit_event(MintEvent {
                        ticker: self.ticker.clone(),
                        amount: mint.amount as u128,
                        account: caller,
                        // inscription_tx_id: Vec::new(),
                    });
                }
            }
        }

        pub fn do_redeem(&mut self, full_proof: FullTransactionProof) {
            let tx = full_proof.user_tx_proof.transaction.to_rust_bitcoin().unwrap();

            for inscription in Inscription::from_transaction(&tx) {
                let body_bytes = inscription.inscription.into_body().unwrap();
                let body_str = core::str::from_utf8(&body_bytes).unwrap();
                if let Some(redeem) = parse_redeem(body_str) {
                    if redeem.ticker != self.ticker || redeem.dest != "INTERLAY" {
                        continue;
                    }

                    self.locked -= redeem.amount as u128;

                    // calculate txid. We could have calculated this directly on the interlay
                    // type, but here we use to_interlay just to show how it works
                    let txid = tx.txid().to_interlay().unwrap();

                    self.env().emit_event(RedeemEvent {
                        ticker: self.ticker.clone(),
                        amount: redeem.amount as u128,
                        account: redeem.account,
                        txid,
                    });
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
                Event::MintEvent(mint) => {
                    assert_eq!(mint.ticker, ticker);
                    assert_eq!(mint.amount, amount);
                    assert_eq!(mint.account, account);
                }
                _ => panic!("Expected Mint event"),
            }
        }

        /// Helper function to for redeem event tests
        fn assert_redeem_event(
            event: &ink::env::test::EmittedEvent,
            ticker: &str,
            amount: u128,
            account: AccountId,
            txid: H256Le,
        ) {
            let decoded_event = decode_event(event);
            match decoded_event {
                Event::RedeemEvent(redeem) => {
                    assert_eq!(
                        redeem,
                        RedeemEvent {
                            account,
                            amount,
                            ticker: ticker.to_owned(),
                            txid
                        }
                    );
                }
                _ => panic!("Expected Redeem event"),
            }
        }

        fn alice() -> AccountId {
            let mut account_bytes = [0u8; 32];
            hex::decode_to_slice(
                "d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d",
                &mut account_bytes as &mut [u8],
            )
            .unwrap();
            let alice = TryFrom::try_from(account_bytes).unwrap();
            return alice;
        }

        #[test]
        fn mint_works() {
            // a proof of an inscription with the following content:
            // {
            //     "p": "brc-21",
            //     "op": "mint",
            //     "tick": "INTR",
            //     "amt": "102",
            //     "src": "INTERLAY"
            // }

            let input_hex = "010000000400a5430a02f7abf2a041b6f095c50251b1a9cd0d0823b8fb4f019513032708ab2d0000000000fdffffff0c010100fe899490fd1e1e8fdfca3eec4bc6c50797a4d48b838f6f411857317a03dbbc37d778bc3f63a3c0e9004b2b8c0110b826dc9ac450ed7cfabbfa1a2e0bc1acf8910220bcb3dcca07be4f0adca39637f8bdb4c834c949dba8e82477deaefca09ac6000bac0063036f72640101106170706c69636174696f6e2f6a736f6e004c657b0a202020202270223a20226272632d3231222c0a20202020226f70223a20226d696e74222c0a20202020227469636b223a2022494e5452222c0a2020202022616d74223a2022313032222c0a2020202022737263223a2022494e5445524c4159220a7d0a6884c1bcb3dcca07be4f0adca39637f8bdb4c834c949dba8e82477deaefca09ac6000b041027000000000000885120acc3f6b76c66989933a3a6b8dde9e433c814231bed5e36ff652cf04604d5d9d301000000006901000097f880f50bd19381336deb0adc53a6d73c550ee517c2d0d7f5b508e6ca12d3580000000000000000000000000000000000000000000000000000000000ffff7fb99ad36400000020ce9de6e0e014e8a99d90c3acdb92eff1833dd6e0e95c5b30556e0fdde17418664e2c5de484c3477fba8dc90ce5d50bd537578dd665b94e172ae9a1aa4ff7dd3f020000002001000101000000000300000008e31fdb6f7807dd0f626cfe1eb34c6ae87d9611c2b6d67885161fa6e779be674ff9bf1343bbc1fbe67776b0f2644a568c67538fbf74fdd565da224896d9640914020000000401018e0500000400ffffffff04800000000000000000000000000000000000000000000000000000000000000000083404950000000000885120c233338cda51f9daa2478ae991fe6d9ebd417be058624c745d5a25468d3aeede0000000000000000986a24aa21a9ede0554045adb9e0a0ee9cee26514a3b8f73e3fa31f5fa4cd561d30e41253d90900100000000b500000097f880f50bd19381336deb0adc53a6d73c550ee517c2d0d7f5b508e6ca12d3580000000000000000000000000000000000000000000000000000000000ffff7fb99ad36400000020ce9de6e0e014e8a99d90c3acdb92eff1833dd6e0e95c5b30556e0fdde17418664e2c5de484c3477fba8dc90ce5d50bd537578dd665b94e172ae9a1aa4ff7dd3f02000000200101010000000000030000000c4347c48408493a7778708bb267b5a4ca0bc0d36a6033a8ee489732e7df082b48a5430a02f7abf2a041b6f095c50251b1a9cd0d0823b8fb4f019513032708ab2d3a00325e46f115cf5cd854235180814e5c0bdbee65f968338c13a35436605412";
            let input_bytes = hex::decode(input_hex).unwrap();
            use scale::Decode;
            let full_proof: FullTransactionProof = Decode::decode(&mut &input_bytes[..]).unwrap();

            let mut contract = Brc21 {
                locked: 200,
                ticker: "INTR".to_owned(),
            };

            // call the inner function - this means we skip the inclusion check
            contract.do_mint(alice(), full_proof);

            let emitted_events = ink::env::test::recorded_events().collect::<Vec<_>>();

            assert_mint_event(&emitted_events[0], DEFAULT_TICKER, 102, alice());

            assert_eq!(contract.locked, 302); // starts at 200, mint 102
        }

        #[test]
        fn redeem_works() {
            // a proof of an inscription with the following content:
            // {
            //     "p": "brc-21",
            //     "op": "redeem",
            //     "tick": "INTR",
            //     "amt": "50",
            //     "dest": "INTERLAY",
            //     "acc": "d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d"
            // }
            // where acc is the Alice account

            let input_hex = "01000000040083511c58b63e49a5043e33430d16a8860166c23ed2598fb10f32138c985c7ac30000000000fdffffff0c0101b1936c1a4fb178cef726c8408a569dd2372031dc4a83d7660b0f88de54be1346560d51d8a9eb7e64f4af245026075f7c3ecc108b834389efd16f20b2e09671d2d503205ea6c870f8b68109a17f15e4219b8bd466d7a43a24082f18c389abc023b2b783ac0063036f72640101106170706c69636174696f6e2f6a736f6e004cb67b0a202020202270223a20226272632d3231222c0a20202020226f70223a202272656465656d222c0a20202020227469636b223a2022494e5452222c0a2020202022616d74223a20223530222c0a202020202264657374223a2022494e5445524c4159222c0a2020202022616363223a202264343335393363373135666464333163363131343161626430346139396664363832326338353538383534636364653339613536383465376135366461323764220a7d0a6884c05ea6c870f8b68109a17f15e4219b8bd466d7a43a24082f18c389abc023b2b783041027000000000000885120343917c817799b69162ba765a2ed311711ca167a552645b805a9e146ff24a9500100000000ba0100008f207ad3026916f8e76fac3421bc01d639c1d5b4ccad455d69cc88b48819b0550000000000000000000000000000000000000000000000000000000000ffff7fcd9ad36400000020e19d7898eca1ee98fbaa79303201893c655a4c37f4271a327b995f58f9cdb9688f338340c560a04f1635a10800feb2e132ef5b7cdd7f26318228d29af01639120300000020010001010000000003000000089dcb4fc367e38c21334e337a7fc9c808e068ebc5a0166d202fcf93fff6f032282f8108552a3e0af701a40f154037dcebfbf4d02336be284de852cc6839ca22b102000000040101f30500000400ffffffff0480000000000000000000000000000000000000000000000000000000000000000008cb824a0000000000885120f519acb8178c569861300691593a04fd6ede802812cbbb5bbca84748846e57570000000000000000986a24aa21a9ed83283d48beef37541d874d8638df44be1389199eebf72dc9b6dd5b44241a41810100000000b50000008f207ad3026916f8e76fac3421bc01d639c1d5b4ccad455d69cc88b48819b0550000000000000000000000000000000000000000000000000000000000ffff7fcd9ad36400000020e19d7898eca1ee98fbaa79303201893c655a4c37f4271a327b995f58f9cdb9688f338340c560a04f1635a10800feb2e132ef5b7cdd7f26318228d29af016391203000000200101010000000000030000000cca2e0f7d98990102eee51e63364b2de57cab1b44a507cb16db97f5ec23fa895483511c58b63e49a5043e33430d16a8860166c23ed2598fb10f32138c985c7ac3df11510fef13b96d681f58433ac02f99fd917eeacbdd833855718722a7aad9c1";
            let input_bytes = hex::decode(input_hex).unwrap();
            use scale::Decode;
            let full_proof: FullTransactionProof = Decode::decode(&mut &input_bytes[..]).unwrap();

            let mut contract = Brc21 {
                locked: 200,
                ticker: "INTR".to_owned(),
            };

            // call the inner function - this means we skip the inclusion check
            contract.do_redeem(full_proof.clone());
            let emitted_events = ink::env::test::recorded_events().collect::<Vec<_>>();

            let txid = full_proof.user_tx_proof.transaction.tx_id_bounded(u32::MAX).unwrap();
            assert_redeem_event(&emitted_events[0], DEFAULT_TICKER, 50, alice(), txid);

            assert_eq!(contract.locked, 150); // starts at 200, redeemed 50
        }

        /// Test if the default constructor does its job.
        #[ink::test]
        fn new_works() {
            let brc21 = Brc21::new(DEFAULT_TICKER.to_string());
            assert_eq!(brc21.get_ticker(), DEFAULT_TICKER);
            assert_eq!(brc21.get_locked(), 0);
        }
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
