mod mock;

use mock::*;
use primitive_types::H256;
use sp_runtime::traits::CheckedMul;

pub const RELAYER: [u8; 32] = ALICE;
pub const VAULT: [u8; 32] = BOB;

fn test_vault_theft(submit_by_relayer: bool) {
    ExtBuilder::build().execute_with(|| {
        let user = ALICE;
        let vault = BOB;
        let amount = 100;
        let collateral_vault = 1000000;

        let vault_btc_address = BtcAddress::P2SH(H160([
            215, 255, 109, 96, 235, 244, 10, 155, 24, 134, 172, 206, 6, 101, 59, 162, 34, 77, 143, 234,
        ]));
        let other_btc_address = BtcAddress::P2SH(H160([1; 20]));

        SecurityPallet::set_active_block_number(1);

        assert_ok!(ExchangeRateOraclePallet::_set_exchange_rate(FixedU128::one()));
        assert_ok!(
            Call::VaultRegistry(VaultRegistryCall::register_vault(collateral_vault, dummy_public_key()))
                .dispatch(origin_of(account_of(vault)))
        );
        assert_ok!(VaultRegistryPallet::insert_vault_deposit_address(
            &account_of(vault),
            vault_btc_address
        ));

        // register as staked relayer
        assert_ok!(Call::StakedRelayers(StakedRelayersCall::register_staked_relayer(100))
            .dispatch(origin_of(account_of(user))));

        let initial_sla = SlaPallet::relayer_sla(account_of(ALICE));

        let (_tx_id, _height, proof, raw_tx, _) = TransactionGenerator::new()
            .with_address(other_btc_address)
            .with_amount(amount)
            .with_confirmations(7)
            .with_relayer(Some(ALICE))
            .mine();

        // check sla increase for the block submission. The call above will have submitted 7 blocks
        // (the actual transaction, plus 6 confirmations)
        let mut expected_sla = initial_sla
            + FixedI128::checked_from_integer(7)
                .unwrap()
                .checked_mul(&SlaPallet::relayer_block_submission())
                .unwrap();
        assert_eq!(SlaPallet::relayer_sla(account_of(ALICE)), expected_sla);

        SecurityPallet::set_active_block_number(1000);

        if submit_by_relayer {
            assert_ok!(
                Call::StakedRelayers(StakedRelayersCall::report_vault_theft(account_of(vault), proof, raw_tx))
                    .dispatch(origin_of(account_of(user)))
            );

            // check sla increase for the theft report
            expected_sla = expected_sla + SlaPallet::relayer_correct_theft_report();
            assert_eq!(SlaPallet::relayer_sla(account_of(ALICE)), expected_sla);
        } else {
            assert_ok!(
                Call::StakedRelayers(StakedRelayersCall::report_vault_theft(account_of(vault), proof, raw_tx))
                    .dispatch(origin_of(account_of(CAROL)))
            );
        }
    });
}

#[test]
fn integration_test_report_vault_theft_by_relayer() {
    test_vault_theft(true);
}

#[test]
fn integration_test_report_vault_theft_by_non_relayer() {
    test_vault_theft(false);
}

#[test]
fn test_staked_relayer_parachain_status_check_fails() {
    ExtBuilder::build().execute_with(|| {
        SecurityPallet::set_status(StatusCode::Shutdown);

        assert_noop!(
            Call::StakedRelayers(StakedRelayersCall::initialize(Default::default(), 0))
                .dispatch(origin_of(account_of(ALICE))),
            SecurityError::ParachainShutdown
        );
        assert_noop!(
            Call::StakedRelayers(StakedRelayersCall::register_staked_relayer(0)).dispatch(origin_of(account_of(ALICE))),
            SecurityError::ParachainShutdown
        );
        assert_noop!(
            Call::StakedRelayers(StakedRelayersCall::deregister_staked_relayer())
                .dispatch(origin_of(account_of(ALICE))),
            SecurityError::ParachainShutdown
        );
        assert_noop!(
            Call::StakedRelayers(StakedRelayersCall::store_block_header(Default::default()))
                .dispatch(origin_of(account_of(ALICE))),
            SecurityError::ParachainShutdown
        );
        assert_noop!(
            Call::StakedRelayers(StakedRelayersCall::report_vault_theft(
                Default::default(),
                Default::default(),
                Default::default()
            ))
            .dispatch(origin_of(account_of(ALICE))),
            SecurityError::ParachainShutdown
        );
    })
}
