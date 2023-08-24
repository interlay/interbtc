use fp_evm::Log;
use frame_support::assert_ok;
use hex_literal::hex;
use sp_core::{H160, H256, U256};

struct MockPrecompileHandle {
    context: fp_evm::Context,
    logs: Vec<Log>,
}

impl fp_evm::PrecompileHandle for MockPrecompileHandle {
    fn call(
        &mut self,
        _: H160,
        _: Option<fp_evm::Transfer>,
        _: Vec<u8>,
        _: Option<u64>,
        _: bool,
        _: &fp_evm::Context,
    ) -> (fp_evm::ExitReason, Vec<u8>) {
        unimplemented!()
    }

    fn record_cost(&mut self, _: u64) -> Result<(), fp_evm::ExitError> {
        unimplemented!()
    }

    fn remaining_gas(&self) -> u64 {
        unimplemented!()
    }

    fn log(&mut self, address: H160, topics: Vec<H256>, data: Vec<u8>) -> Result<(), fp_evm::ExitError> {
        self.logs.push(Log { address, topics, data });
        Ok(())
    }

    fn code_address(&self) -> H160 {
        unimplemented!()
    }

    fn input(&self) -> &[u8] {
        unimplemented!()
    }

    fn context(&self) -> &fp_evm::Context {
        &self.context
    }

    fn is_static(&self) -> bool {
        true
    }

    fn gas_limit(&self) -> Option<u64> {
        unimplemented!()
    }

    fn record_external_cost(
        &mut self,
        _ref_time: Option<u64>,
        _proof_size: Option<u64>,
    ) -> Result<(), fp_evm::ExitError> {
        Ok(())
    }

    fn refund_external_cost(&mut self, _ref_time: Option<u64>, _proof_size: Option<u64>) {}
}

#[test]
fn generate_call() {
    #[derive(Debug, evm_macro::EvmCall)]
    enum Call {
        #[selector = "totalSupply()"]
        TotalSupply,
        #[selector = "balanceOf(address)"]
        BalanceOf { account: H160 },
        #[selector = "transfer(address,uint256)"]
        Transfer { recipient: H160, amount: U256 },
        #[selector = "allowance(address,address)"]
        Allowance { owner: H160, spender: H160 },
        #[selector = "approve(address,uint256)"]
        Approve { spender: H160, amount: U256 },
        #[selector = "transferFrom(address,address,uint256)"]
        TransferFrom {
            sender: H160,
            recipient: H160,
            amount: U256,
        },
    }

    assert!(matches!(Call::new(&hex!("18160ddd")).unwrap(), Call::TotalSupply));

    assert!(matches!(
        Call::new(&hex!(
            "70a082310000000000000000000000005b38da6a701c568545dcfcb03fcb875f56beddc4"
        ))
        .unwrap(),
        Call::BalanceOf {
            account: H160(hex!["5b38da6a701c568545dcfcb03fcb875f56beddc4"])
        }
    ));

    assert!(matches!(Call::new(&hex!(
        "a9059cbb000000000000000000000000ab8483f64d9c6d1ecf9b849ae677dd3315835cb20000000000000000000000000000000000000000000000000000000000000064"
    )).unwrap(), Call::Transfer {
        recipient: H160(hex!["Ab8483F64d9C6d1EcF9b849Ae677dD3315835cb2"]),
        amount,
    } if amount == U256::from(100)));

    assert!(matches!(Call::new(&hex!(
        "dd62ed3e0000000000000000000000005b38da6a701c568545dcfcb03fcb875f56beddc4000000000000000000000000ab8483f64d9c6d1ecf9b849ae677dd3315835cb2"
    )).unwrap(), Call::Allowance {
        owner: H160(hex!["5b38da6a701c568545dcfcb03fcb875f56beddc4"]),
        spender: H160(hex!["Ab8483F64d9C6d1EcF9b849Ae677dD3315835cb2"]),
    }));

    assert!(matches!(Call::new(&hex!(
        "095ea7b3000000000000000000000000ab8483f64d9c6d1ecf9b849ae677dd3315835cb20000000000000000000000000000000000000000000000000000000000000064"
    )).unwrap(), Call::Approve {
        spender: H160(hex!["Ab8483F64d9C6d1EcF9b849Ae677dD3315835cb2"]),
        amount
    } if amount == U256::from(100)));

    assert!(matches!(Call::new(&hex!(
        "23b872dd0000000000000000000000005b38da6a701c568545dcfcb03fcb875f56beddc4000000000000000000000000ab8483f64d9c6d1ecf9b849ae677dd3315835cb20000000000000000000000000000000000000000000000000000000000000064"
    )).unwrap(), Call::TransferFrom {
        sender: H160(hex!["5b38da6a701c568545dcfcb03fcb875f56beddc4"]),
        recipient: H160(hex!["Ab8483F64d9C6d1EcF9b849Ae677dD3315835cb2"]),
        amount
    } if amount == U256::from(100)));
}

#[test]
fn generate_event() {
    #[derive(evm_macro::EvmEvent)]
    enum Event {
        #[selector = "Transfer(address,address,uint256)"]
        Transfer {
            #[indexed]
            from: H160,
            #[indexed]
            to: H160,
            value: U256,
        },
    }

    let mut precompile_handle = MockPrecompileHandle {
        context: fp_evm::Context {
            address: H160([1; 20]),
            caller: Default::default(),
            apparent_value: Default::default(),
        },
        logs: Default::default(),
    };

    assert_ok!(Event::Transfer {
        from: H160([2; 20]),
        to: H160([3; 20]),
        value: U256::one(),
    }
    .log(&mut precompile_handle));

    let mut data = [0u8; 32];
    U256::one().to_big_endian(&mut data);

    assert_eq!(
        precompile_handle.logs,
        vec![Log {
            address: H160([1; 20]),
            topics: vec![
                H256(hex!["ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef"]),
                H256(hex!["0000000000000000000000000202020202020202020202020202020202020202"]),
                H256(hex!["0000000000000000000000000303030303030303030303030303030303030303"]),
            ],
            data: data.to_vec(),
        }]
    )
}
