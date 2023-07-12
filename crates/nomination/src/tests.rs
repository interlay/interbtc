use crate::{ext, mock::*};
use currency::Amount;
use frame_support::{assert_err, assert_ok};
use mocktopus::mocking::*;
use sp_arithmetic::FixedI128;

#[test]
fn should_not_deposit_against_invalid_vault() {
    run_test(|| {
        assert_err!(
            Nomination::_deposit_collateral(&ALICE, &BOB.account_id, 100),
            TestError::VaultNotOptedInToNomination
        );
    })
}

fn collateral(amount: u128) -> Amount<Test> {
    Amount::new(amount, DEFAULT_COLLATERAL_CURRENCY)
}

#[test]
fn should_deposit_against_valid_vault() {
    run_test(|| {
        ext::vault_registry::vault_exists::<Test>.mock_safe(|_| MockResult::Return(true));
        ext::vault_registry::get_backing_collateral::<Test>.mock_safe(|_| MockResult::Return(Ok(collateral(10000))));
        ext::vault_registry::compute_collateral::<Test>.mock_safe(|_| MockResult::Return(Ok(collateral(10000))));
        ext::vault_registry::pool_manager::deposit_collateral::<Test>.mock_safe(|_, _, _| MockResult::Return(Ok(())));
        assert_ok!(Nomination::_opt_in_to_nomination(&ALICE));
        assert_ok!(Nomination::set_nomination_limit(
            RuntimeOrigin::signed(ALICE.account_id),
            ALICE.currencies,
            100
        ));
        assert_ok!(Nomination::_deposit_collateral(&ALICE, &BOB.account_id, 100));
    })
}

#[test]
fn should_not_withdraw_collateral() {
    use orml_traits::MultiCurrency;

    run_test(|| {
        assert_ok!(Tokens::deposit(
            ALICE.currencies.collateral,
            &ALICE.account_id,
            24_609_778_406_619_232
        ));
        VaultRegistry::_set_system_collateral_ceiling(ALICE.currencies, u128::MAX);
        assert_ok!(VaultRegistry::register_public_key(
            RuntimeOrigin::signed(ALICE.account_id),
            vault_registry::BtcPublicKey::dummy()
        ));
        assert_ok!(VaultRegistry::register_vault(
            RuntimeOrigin::signed(ALICE.account_id),
            ALICE.currencies,
            24_609_778_406_619_232
        ));

        staking::SlashPerToken::<Test>::insert(0, ALICE, FixedI128::from_inner(25_210_223_519_649_666));
        staking::SlashTally::<Test>::insert(
            0,
            (ALICE, ALICE.account_id),
            FixedI128::from_inner(100_834_580_684_768_029_667_333_677_168),
        );
        staking::Stake::<Test>::insert(
            0,
            (ALICE, ALICE.account_id),
            FixedI128::from_inner(3_999_749_570_096_999_994_120_799_432_121),
        );
        staking::TotalCurrentStake::<Test>::insert(
            0,
            ALICE,
            FixedI128::from_inner(3_999_749_570_097_000_000_000_000_000_000),
        );
        staking::TotalStake::<Test>::insert(
            0,
            ALICE,
            FixedI128::from_inner(3_999_749_570_096_999_994_120_799_432_121),
        );

        // should not withdraw all
        assert_err!(
            Nomination::_withdraw_collateral(&ALICE, &ALICE.account_id, Some(3999749570097), 0),
            staking::Error::<Test>::InsufficientFunds
        );

        // should withdraw all
        assert_ok!(Nomination::_withdraw_collateral(&ALICE, &ALICE.account_id, None, 0));

        // stake is now zero
        assert_ok!(ext::staking::compute_stake::<Test>(&ALICE, &ALICE.account_id), 0);
        assert_ok!(
            VaultRegistry::get_backing_collateral(&ALICE),
            Amount::new(0, ALICE.collateral_currency())
        );
    });
}
