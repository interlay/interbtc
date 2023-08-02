use crate::setup::{assert_eq, *};
use fp_evm::{ExecutionInfoV2, FeeCalculator};
use fp_rpc::ConvertTransaction;
use hex_literal::hex;
use pallet_evm::{AddressMapping, ExitReason, ExitSucceed, GasWeightMapping, Runner};

// pragma solidity ^0.8.9;
// contract flipper {
//   bool private value;
//   constructor(bool initvalue) {
//     value = initvalue;
//   }
//
//   function flip() public {
//     value = !value;
//   }
//
//   function get() public view returns (bool) {
//     return value;
//   }
// }
pub const CONTRACT_TX: &str = "02f9024682084980849502f90085010c388d0083020a778080b901ea608060405234801561001057600080fd5b506040516101ca3803806101ca8339818101604052810190610032919061008e565b806000806101000a81548160ff021916908315150217905550506100bb565b600080fd5b60008115159050919050565b61006b81610056565b811461007657600080fd5b50565b60008151905061008881610062565b92915050565b6000602082840312156100a4576100a3610051565b5b60006100b284828501610079565b91505092915050565b610100806100ca6000396000f3fe6080604052348015600f57600080fd5b506004361060325760003560e01c80636d4ce63c146037578063cde4efa9146051575b600080fd5b603d6059565b6040516048919060b1565b60405180910390f35b6057606f565b005b60008060009054906101000a900460ff16905090565b60008054906101000a900460ff16156000806101000a81548160ff021916908315150217905550565b60008115159050919050565b60ab816098565b82525050565b600060208201905060c4600083018460a4565b9291505056fea2646970667358221220da54b58a2aebbc9317cfd38d215d97502ce00c8af859f5dd42453d3a47e35be164736f6c634300081100330000000000000000000000000000000000000000000000000000000000000000c080a0a7e73235820df76570958958146ff8ea0e8bbc989ea8e9f77ab849e51b763852a01afd2d44c171f03b9de17c8731f6543cae18d8a0b95103ef33cb8a5ae923498c";
pub const TRANSFER_TX: &str = "02f87382084980843b9aca00843b9aca008252089470997970c51812dc3a010c7d01b50e0d17dc79c8872386f26fc1000080c001a01b14ef6be71ca69615ad5818f74a3a8bf9d50dbac28dfa64326260caa5e50089a0468d7023ece460138491ff25c5fb6df2ab0c706f592fef18a640d1cb4434c180";

pub fn unchecked_eth_tx(raw_hex_tx: &str) -> UncheckedExtrinsic {
    let converter = TransactionConverter;
    converter.convert_transaction(ethereum_transaction(raw_hex_tx))
}

pub fn ethereum_transaction(raw_hex_tx: &str) -> pallet_ethereum::Transaction {
    let bytes = hex::decode(raw_hex_tx).expect("Transaction bytes");
    ethereum::EnvelopedDecodable::decode(&bytes[..]).expect("Transaction is valid")
}

#[test]
fn test_transfer() {
    ExtBuilder::build().execute_with(|| {
        let from_address = H160(hex!["f39fd6e51aad88f6f4ce6ab8827279cfffb92266"]);
        let from_account_id = <Runtime as pallet_evm::Config>::AddressMapping::into_account_id(from_address.clone());
        let to_address = H160(hex!["70997970c51812dc3a010c7d01b50e0d17dc79c8"]);
        let to_account_id = <Runtime as pallet_evm::Config>::AddressMapping::into_account_id(to_address);

        set_balance(from_account_id.clone(), NATIVE_CURRENCY_ID, 1 << 60);
        assert_eq!(Tokens::free_balance(NATIVE_CURRENCY_ID, &to_account_id), 0);

        pallet_evm_chain_id::ChainId::<Runtime>::put(2121);
        let uxt = unchecked_eth_tx(TRANSFER_TX);
        // NOTE: we can also apply using ValidatedTransaction
        // but this allows us to check signature recovery
        assert_ok!(Executive::apply_extrinsic(uxt));

        assert_eq!(
            Tokens::free_balance(NATIVE_CURRENCY_ID, &to_account_id),
            10000000000000000
        );

        let tx_hash = H256(hex!["b237d25ba8c7b1fe45c795da35ec6cb20f643c15dc97d9c6eabfdb9e3d37f3ea"]);
        assert!(System::events().iter().any(|a| {
            match a.event {
                RuntimeEvent::Ethereum(pallet_ethereum::Event::Executed {
                    from,
                    to,
                    transaction_hash,
                    exit_reason: ExitReason::Succeed(ExitSucceed::Stopped),
                    ..
                }) if from == from_address && to == to_address && transaction_hash == tx_hash => true,
                _ => false,
            }
        }));
    })
}

fn call_contract(from: H160, to: H160, input: Vec<u8>) -> ExecutionInfoV2<Vec<u8>> {
    let gas_limit: u64 = 1_000_000;
    let weight_limit = <Runtime as pallet_evm::Config>::GasWeightMapping::gas_to_weight(gas_limit, true);
    let min_gas_price = <BaseFee as FeeCalculator>::min_gas_price().0;
    <Runtime as pallet_evm::Config>::Runner::call(
        from,
        to,
        input,
        U256::zero(), // value
        gas_limit,
        Some(min_gas_price),
        None,       // max_priority_fee_per_gas
        None,       // nonce
        Vec::new(), // access_list
        true,       // is_transactional
        true,       // validate
        Some(weight_limit),
        Some(0),
        &<Runtime as pallet_evm::Config>::config().clone(),
    )
    .unwrap()
}

#[test]
fn test_contract() {
    ExtBuilder::build().execute_with(|| {
        let user_address = H160(hex!["f39fd6e51aad88f6f4ce6ab8827279cfffb92266"]);
        let user_account_id = <Runtime as pallet_evm::Config>::AddressMapping::into_account_id(user_address.clone());
        set_balance(user_account_id.clone(), NATIVE_CURRENCY_ID, 1 << 60);

        evm::EnableCreate::set(&true);
        pallet_evm_chain_id::ChainId::<Runtime>::put(2121);
        let uxt = unchecked_eth_tx(CONTRACT_TX);
        assert_ok!(Executive::apply_extrinsic(uxt));
        let contract_address = H160(hex!["5FbDB2315678afecb367f032d93F642f64180aa3"]);

        assert_eq!(
            call_contract(
                user_address,
                contract_address,
                hex!["6d4ce63c"].to_vec(), // get()
            )
            .value,
            [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        );

        call_contract(
            user_address,
            contract_address,
            hex!["cde4efa9"].to_vec(), // flip()
        );

        assert_eq!(
            call_contract(
                user_address,
                contract_address,
                hex!["6d4ce63c"].to_vec(), // get()
            )
            .value,
            [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]
        );
    })
}
