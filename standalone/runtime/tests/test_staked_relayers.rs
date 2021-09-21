mod mock;

use currency::Amount;
use mock::{assert_eq, *};
use sp_core::H256;

pub const RELAYER: [u8; 32] = ALICE;
pub const VAULT: [u8; 32] = BOB;

fn test_with<R>(execute: impl Fn(CurrencyId) -> R) {
    let test_with = |currency_id| {
        ExtBuilder::build().execute_with(|| {
            assert_ok!(OraclePallet::_set_exchange_rate(currency_id, FixedU128::one()));
            execute(currency_id)
        })
    };
    test_with(CurrencyId::DOT);
    test_with(CurrencyId::KSM);
}

#[test]
fn integration_test_report_vault_theft() {
    test_with(|currency_id| {
        let user = ALICE;
        let vault = BOB;
        let theft_amount = wrapped(100);
        let collateral_vault = 1000000;
        let issued_tokens = wrapped(100);
        let vault_id = vault_id_of(vault, currency_id);

        let vault_btc_address = BtcAddress::P2SH(H160([
            215, 255, 109, 96, 235, 244, 10, 155, 24, 134, 172, 206, 6, 101, 59, 162, 34, 77, 143, 234,
        ]));
        let other_btc_address = BtcAddress::P2SH(H160([1; 20]));

        SecurityPallet::set_active_block_number(1);

        assert_ok!(Call::VaultRegistry(VaultRegistryCall::register_vault(
            currency_id,
            DEFAULT_WRAPPED_CURRENCY,
            collateral_vault,
            dummy_public_key(),
        ))
        .dispatch(origin_of(account_of(vault))));
        assert_ok!(VaultRegistryPallet::insert_vault_deposit_address(
            vault_id.clone(),
            vault_btc_address
        ));

        assert_ok!(VaultRegistryPallet::try_increase_to_be_issued_tokens(
            &vault_id,
            &issued_tokens,
        ));
        assert_ok!(VaultRegistryPallet::issue_tokens(&vault_id, &issued_tokens));

        let (_tx_id, _height, proof, raw_tx, _) = TransactionGenerator::new()
            .with_address(other_btc_address)
            .with_amount(theft_amount)
            .with_confirmations(7)
            .with_relayer(Some(ALICE))
            .mine();

        SecurityPallet::set_active_block_number(1000);

        let pre_liquidation_state = ParachainState::get(currency_id);
        let theft_fee = FeePallet::get_theft_fee(&Amount::new(collateral_vault, currency_id)).unwrap();

        assert_ok!(
            Call::Relay(RelayCall::report_vault_theft(vault_id, proof, raw_tx)).dispatch(origin_of(account_of(user)))
        );

        let confiscated_collateral = Amount::new(150, currency_id);
        assert_eq!(
            ParachainState::get(currency_id),
            pre_liquidation_state.with_changes(|user, vault, liquidation_vault, _fee_pool| {
                let liquidation_vault = liquidation_vault.with_currency(&currency_id);

                (*user.balances.get_mut(&currency_id).unwrap()).free += theft_fee;

                vault.issued -= issued_tokens;
                vault.backing_collateral -= confiscated_collateral;
                vault.backing_collateral -= theft_fee;

                liquidation_vault.issued += issued_tokens;
                liquidation_vault.collateral += confiscated_collateral;
            })
        );
    });
}

#[test]
fn integration_test_relay_parachain_status_check_fails() {
    ExtBuilder::build().execute_with(|| {
        SecurityPallet::set_status(StatusCode::Shutdown);

        assert_noop!(
            Call::Relay(RelayCall::initialize(Default::default(), 0)).dispatch(origin_of(account_of(ALICE))),
            SecurityError::ParachainShutdown
        );
        assert_noop!(
            Call::Relay(RelayCall::store_block_header(Default::default())).dispatch(origin_of(account_of(ALICE))),
            SecurityError::ParachainShutdown
        );
        assert_noop!(
            Call::Relay(RelayCall::report_vault_theft(
                vault_id_of(ALICE, DOT),
                Default::default(),
                Default::default()
            ))
            .dispatch(origin_of(account_of(ALICE))),
            SecurityError::ParachainShutdown
        );
    })
}
