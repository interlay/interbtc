mod mock;

use mock::*;
use primitive_types::H256;
use sp_runtime::traits::CheckedMul;
use vault_registry::Vault;

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

        SecurityModule::set_active_block_number(1);

        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(FixedU128::one()));
        VaultRegistryModule::insert_vault(&account_of(LIQUIDATION_VAULT), Vault::default());
        // assert_ok!(CollateralModule::lock_collateral(&account_of(vault), collateral_vault));
        assert_ok!(
            Call::VaultRegistry(VaultRegistryCall::register_vault(collateral_vault, dummy_public_key()))
                .dispatch(origin_of(account_of(vault)))
        );
        assert_ok!(VaultRegistryModule::insert_vault_deposit_address(
            &account_of(vault),
            vault_btc_address
        ));

        // register as staked relayer
        assert_ok!(Call::StakedRelayers(StakedRelayersCall::register_staked_relayer(100))
            .dispatch(origin_of(account_of(user))));

        SecurityModule::set_active_block_number(StakedRelayersModule::get_maturity_period() + 100);

        // manually activate
        assert_ok!(StakedRelayersModule::activate_staked_relayer(&account_of(user)));

        let initial_sla = SlaModule::relayer_sla(account_of(ALICE));

        let (tx_id, _height, proof, raw_tx) = TransactionGenerator::new()
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
                .checked_mul(&SlaModule::relayer_block_submission())
                .unwrap();
        assert_eq!(SlaModule::relayer_sla(account_of(ALICE)), expected_sla);

        SecurityModule::set_active_block_number(1000);

        if submit_by_relayer {
            assert_ok!(Call::StakedRelayers(StakedRelayersCall::report_vault_theft(
                account_of(vault),
                tx_id,
                proof,
                raw_tx
            ))
            .dispatch(origin_of(account_of(user))));

            // check sla increase for the theft report
            expected_sla = expected_sla + SlaModule::relayer_correct_theft_report();
            assert_eq!(SlaModule::relayer_sla(account_of(ALICE)), expected_sla);
        } else {
            assert_ok!(Call::StakedRelayers(StakedRelayersCall::report_vault_theft(
                account_of(vault),
                tx_id,
                proof,
                raw_tx
            ))
            .dispatch(origin_of(account_of(CAROL))));
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
