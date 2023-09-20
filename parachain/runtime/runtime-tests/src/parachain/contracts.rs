use crate::setup::{assert_eq, *};
use pallet_contracts::{CollectEvents, DebugInfo, Determinism};
use pallet_contracts_primitives::Code;
use sp_runtime::traits::Hash;

pub const GAS_LIMIT: Weight = Weight::from_parts(100_000_000_000_000, 10000 * 1024 * 1024);

type ContractsCall = pallet_contracts::Call<Runtime>;
type ContractsError = pallet_contracts::Error<Runtime>;
type ContractsEvent = pallet_contracts::Event<Runtime>;
type ContractsPallet = pallet_contracts::Pallet<Runtime>;

mod relay {
    use currency::Amount;

    use super::*;

    pub fn transfer_sats(address: BtcAddress, amount: u64) -> FullTransactionProof {
        let amount = Amount::<Runtime>::new(amount as u128, Token(KBTC));

        let (_tx_id, _height, proof) = TransactionGenerator::new().with_outputs(vec![(address, amount)]).mine();

        SecurityPallet::set_active_block_number(SecurityPallet::active_block_number() + CONFIRMATIONS);

        proof
    }
}

#[test]
fn test_basic_contract() {
    // not sure this case would ever be used, best we have a test for it anyway..
    ExtBuilder::build().execute_with(|| {
        let key = kintsugi_runtime_parachain::contracts::EnableContracts::key();
        let hex = hex::encode(key);
        println!("key = {hex}");
        // note: current working directory is diffent when you run this test, vs when you debug it.
        // However, the `PWD` env variable is (surprisingly) set to the workspace root in both cases.
        // So, we use a path relative to PWD
        let contract_path =
            std::env::var("PWD").expect("pwd not set") + "/contracts/hello_world/target/ink/hello_world.wasm";

        let blob = std::fs::read(contract_path).unwrap();
        let blob_hash = <Runtime as frame_system::Config>::Hashing::hash(&blob);

        let value = 0; // a value of 100 doesn't seem to work.. need to look into this

        // you can either do:
        // - upload_code + bare_instantiate(Code::Existing), or,
        // - bare_instantiate(Code::Upload)
        assert_ok!(RuntimeCall::Contracts(ContractsCall::upload_code {
            code: blob,
            determinism: Determinism::Enforced,
            storage_deposit_limit: None
        })
        .dispatch(origin_of(account_of(ALICE))));

        // This needs to match the `selector` of one of the constructors. You can check the selector
        // in the generated metadata: contracts/hello_world/target/ink/hello_world.json (compile the contract first)
        // Note: if the constructor takes any input arguments, it needs to be appended here in scale encoding
        let input = vec![0x61, 0xef, 0x7e, 0x3e]; // new_default
        let ret = ContractsPallet::bare_instantiate(
            account_of(ALICE),
            value,
            GAS_LIMIT,
            None,
            Code::Existing(blob_hash),
            input,
            vec![],
            DebugInfo::Skip,
            CollectEvents::Skip,
        );
        let result = ret.result.unwrap();

        // non-zero indicated the REVERT flag was set, meaning something went wrong in the execution
        assert_eq!(result.result.flags.bits(), 0);

        // The address that the contract was deployed to
        let addr = result.account_id;

        // Alternative to upload_code + bare_instantiate(Code::Existing):
        //     let q = ContractsPallet::bare_instantiate(
        //         account_of(ALICE),
        //         0,
        //         GAS_LIMIT,
        //         None,
        //         Code::Upload(blob),
        //         vec![],
        //         vec![],
        //         true,
        //     );

        // see comment above regarding selector
        let do_something_on_runtime_selector = vec![0xdb, 0xf4, 0x28, 0x29];
        let result = ContractsPallet::bare_call(
            account_of(ALICE),
            addr.clone(),
            0,
            GAS_LIMIT,
            None,
            do_something_on_runtime_selector,
            DebugInfo::Skip,
            CollectEvents::Skip,
            Determinism::Enforced,
        );
        assert_ok!(result.result);
    })
}

#[test]
fn test_btc_swap_contract() {
    // not sure this case would ever be used, best we have a test for it anyway..
    ExtBuilder::build().execute_with(|| {
        // note: current working directory is diffent when you run this test, vs when you debug it.
        // However, the `PWD` env variable is (surprisingly) set to the workspace root in both cases.
        // So, we use a path relative to PWD
        let contract_path = std::env::var("PWD").expect("pwd not set") + "/contracts/btc_swap/target/ink/btc_swap.wasm";

        let blob = std::fs::read(contract_path).unwrap();
        let blob_hash = <Runtime as frame_system::Config>::Hashing::hash(&blob);

        let value = 0; // a value of 100 doesn't seem to work.. need to look into this

        // initialize contract..
        assert_ok!(RuntimeCall::Contracts(ContractsCall::upload_code {
            code: blob,
            determinism: Determinism::Enforced,
            storage_deposit_limit: None
        })
        .dispatch(origin_of(account_of(ALICE))));

        // This needs to match the `selector` of one of the constructors. You can check the selector
        // in the generated metadata: contracts/hello_world/target/ink/hello_world.json (compile the contract first)
        // Note: if the constructor takes any input arguments, it needs to be appended here in scale encoding
        let input = vec![0x9b, 0xae, 0x9d, 0x5e]; // new
        let ret = ContractsPallet::bare_instantiate(
            account_of(ALICE),
            value,
            GAS_LIMIT,
            None,
            Code::Existing(blob_hash),
            input,
            vec![],
            DebugInfo::Skip,
            CollectEvents::Skip,
        );
        let result = ret.result.unwrap();

        // non-zero indicated the REVERT flag was set, meaning something went wrong in the execution
        assert_eq!(result.result.flags.bits(), 0);
        // The address that the contract was deployed to
        let addr = result.account_id;

        // call create_trade
        let mut create_trade = vec![0xf7, 0x3c, 0xab, 0x55];
        let address: BtcAddress = Default::default();
        let min_satoshis: u64 = 1000000;
        address.encode_to(&mut create_trade);
        min_satoshis.encode_to(&mut create_trade);
        let result = ContractsPallet::bare_call(
            account_of(ALICE),
            addr.clone(),
            0,
            GAS_LIMIT,
            None,
            create_trade,
            DebugInfo::Skip,
            CollectEvents::Skip,
            Determinism::Enforced,
        );
        assert_ok!(result.result);

        // test case 1: transfer of sufficient tokens has been made.
        dry_run(|| {
            let proof = relay::transfer_sats(address, min_satoshis);

            // see comment above regarding selector
            let mut execute_trade = vec![0x6b, 0xf4, 0x21, 0xce];
            account_of(ALICE).encode_to(&mut execute_trade);
            proof.encode_to(&mut execute_trade);
            let result = ContractsPallet::bare_call(
                account_of(ALICE),
                addr.clone(),
                0,
                GAS_LIMIT,
                None,
                execute_trade,
                DebugInfo::Skip,
                CollectEvents::Skip,
                Determinism::Enforced,
            );
            assert_eq!(result.result.unwrap().flags.bits(), 0); // checks that result is ok, and no error flags are set
        });

        // test case 2: payment of insufficient value: execution fails
        let proof = relay::transfer_sats(address, min_satoshis / 2);
        let mut execute_trade = vec![0x6b, 0xf4, 0x21, 0xce];
        account_of(ALICE).encode_to(&mut execute_trade);
        proof.encode_to(&mut execute_trade);
        let result = ContractsPallet::bare_call(
            account_of(ALICE),
            addr.clone(),
            0,
            GAS_LIMIT,
            None,
            execute_trade,
            DebugInfo::Skip,
            CollectEvents::Skip,
            Determinism::Enforced,
        );
        assert_err!(result.result, ContractsError::ContractTrapped);
    })
}
