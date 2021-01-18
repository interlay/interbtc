mod mock;

use mock::*;
use primitive_types::H256;
use vault_registry::Vault;

type StakedRelayersCall = staked_relayers::Call<Runtime>;
type StakedRelayersModule = staked_relayers::Module<Runtime>;

#[test]
fn integration_test_report_vault_theft() {
    ExtBuilder::build().execute_with(|| {
        let user = ALICE;
        let vault = BOB;
        let amount = 100;
        let collateral_vault = 1000000;

        let vault_btc_address = BtcAddress::P2SH(H160([
            215, 255, 109, 96, 235, 244, 10, 155, 24, 134, 172, 206, 6, 101, 59, 162, 34, 77, 143,
            234,
        ]));
        let other_btc_address = BtcAddress::P2SH(H160([1; 20]));

        SystemModule::set_block_number(1);

        assert_ok!(ExchangeRateOracleModule::_set_exchange_rate(
            FixedU128::one()
        ));
        VaultRegistryModule::insert_vault(&account_of(LIQUIDATION_VAULT), Vault::default());
        // assert_ok!(CollateralModule::lock_collateral(&account_of(vault), collateral_vault));
        assert_ok!(Call::VaultRegistry(VaultRegistryCall::register_vault(
            collateral_vault,
            vault_btc_address.clone()
        ))
        .dispatch(origin_of(account_of(vault))));

        // register as staked relayer
        assert_ok!(
            Call::StakedRelayers(StakedRelayersCall::register_staked_relayer(100))
                .dispatch(origin_of(account_of(user)))
        );

        SystemModule::set_block_number(StakedRelayersModule::get_maturity_period() + 100);

        // manually activate
        assert_ok!(StakedRelayersModule::activate_staked_relayer(&account_of(
            user
        )));

        let (tx_id, _height, proof, raw_tx) = generate_transaction_and_mine_with_script_sig(
            other_btc_address,
            amount,
            H256::zero(),
            &[
                0, 71, 48, 68, 2, 32, 91, 128, 41, 150, 96, 53, 187, 63, 230, 129, 53, 234, 210,
                186, 21, 187, 98, 38, 255, 112, 30, 27, 228, 29, 132, 140, 155, 62, 123, 216, 232,
                168, 2, 32, 72, 126, 179, 207, 142, 8, 99, 8, 32, 78, 244, 166, 106, 160, 207, 227,
                61, 210, 172, 234, 234, 93, 59, 159, 79, 12, 194, 240, 212, 3, 120, 50, 1, 71, 81,
                33, 3, 113, 209, 131, 177, 9, 29, 242, 229, 15, 217, 247, 165, 78, 111, 80, 79, 50,
                200, 117, 80, 30, 233, 210, 167, 133, 175, 62, 253, 134, 127, 212, 51, 33, 2, 128,
                200, 184, 235, 148, 25, 43, 34, 28, 173, 55, 54, 189, 164, 187, 243, 243, 152, 7,
                84, 210, 85, 156, 238, 77, 97, 188, 240, 162, 197, 105, 62, 82, 174,
            ],
        );

        SystemModule::set_block_number(1000);

        assert_ok!(Call::StakedRelayers(StakedRelayersCall::report_vault_theft(
            account_of(vault),
            tx_id,
            proof,
            raw_tx
        ))
        .dispatch(origin_of(account_of(user))));
    });
}
