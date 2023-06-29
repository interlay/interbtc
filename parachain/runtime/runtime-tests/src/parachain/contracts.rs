use crate::setup::{assert_eq, *};
use pallet_contracts::Determinism;
use pallet_contracts_primitives::Code;
use sp_runtime::traits::Hash;

pub const GAS_LIMIT: Weight = Weight::from_parts(100_000_000_000_000, 10000 * 1024 * 1024);

type ContractsCall = pallet_contracts::Call<Runtime>;
type ContractsError = pallet_contracts::Error<Runtime>;
type ContractsEvent = pallet_contracts::Event<Runtime>;
type ContractsPallet = pallet_contracts::Pallet<Runtime>;

#[test]
fn test_contract() {
    // not sure this case would ever be used, best we have a test for it anyway..
    ExtBuilder::build().execute_with(|| {
        // note: current working directory is diffent when you run this test, vs when you debug it.
        // As a temporary workaround, I'm using an absolute path which is correct (only) on my machine
        let contract_path =
            "/home/sander/workspace/interlay/btc-parachain/contracts/hello_world/target/ink/hello_world.wasm";

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
            true,
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
            false,
            Determinism::Enforced,
        );
        assert_ok!(result.result);
    })
}