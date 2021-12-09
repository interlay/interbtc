mod mock;

use frame_support::traits::Currency;
use mock::{assert_eq, *};
use orml_vesting::VestingSchedule;
use primitives::VaultCurrencyPair;
use sp_core::{Encode, H256};

type VestingCall = orml_vesting::Call<Runtime>;
type UtilityCall = pallet_utility::Call<Runtime>;

#[test]
fn integration_test_multisig_transfer() {
    ExtBuilder::build().execute_with(|| {
        // step 0: clear eve's balance for easier testing
        assert_ok!(Call::Tokens(TokensCall::set_balance {
            who: account_of(EVE),
            currency_id: Token(DOT),
            new_free: 0,
            new_reserved: 0,
        })
        .dispatch(root()));

        // step 1: deposit funds to a shared account
        let multisig_account = MultiSigPallet::multi_account_id(&vec![account_of(ALICE), account_of(BOB)], 2);
        assert_ok!(Call::Tokens(TokensCall::set_balance {
            who: multisig_account.clone(),
            currency_id: Token(DOT),
            new_free: 20_000_000_000_001,
            new_reserved: 0,
        })
        .dispatch(root()));

        // step 2: submit a call, to be executed from the shared account
        let call = Call::Tokens(TokensCall::transfer {
            dest: account_of(EVE),
            currency_id: Token(DOT),
            amount: 20_000_000_000_001,
        })
        .encode();
        assert_ok!(Call::MultiSig(MultiSigCall::as_multi {
            threshold: 2,
            other_signatories: vec![account_of(BOB)],
            maybe_timepoint: None,
            call: call.clone(),
            store_call: true,
            max_weight: 1000000000000,
        })
        .dispatch(origin_of(account_of(ALICE))));

        // step 2a: balance should not have changed yet - the call is not executed yet
        assert_eq!(CollateralCurrency::total_balance(&account_of(EVE)), 0);

        // step 3: get the timepoint at which the call was made. In production, you would get this
        // from the event metadata, or from storage
        let timepoint = MultiSigPallet::timepoint();

        // step 4: let the second account approve
        assert_ok!(Call::MultiSig(MultiSigCall::approve_as_multi {
            threshold: 2,
            other_signatories: vec![account_of(ALICE)],
            maybe_timepoint: Some(timepoint),
            call_hash: sp_core::blake2_256(&call),
            max_weight: 1000000000000,
        })
        .dispatch(origin_of(account_of(BOB))));
        // step 4a: check that the call is now executed
        assert_eq!(CollateralCurrency::total_balance(&account_of(EVE)), 20_000_000_000_001);
    });
}

#[test]
fn integration_test_multisig_vesting() {
    ExtBuilder::build().execute_with(|| {
        let vesting_amount = 30_000_000;
        let multisig_account = MultiSigPallet::multi_account_id(&vec![account_of(ALICE), account_of(BOB)], 2);

        // vested transfer takes free balance of caller
        assert_ok!(Call::Tokens(TokensCall::set_balance {
            who: multisig_account.clone(),
            currency_id: Token(INTR),
            new_free: vesting_amount,
            new_reserved: 0,
        })
        .dispatch(root()));

        // gradually release amount over 100 periods
        let call = Call::Vesting(VestingCall::vested_transfer {
            dest: account_of(EVE),
            schedule: VestingSchedule {
                start: 0,
                period: 10,
                period_count: 100,
                per_period: vesting_amount / 100,
            },
        })
        .encode();

        assert_ok!(Call::MultiSig(MultiSigCall::as_multi {
            threshold: 2,
            other_signatories: vec![account_of(BOB)],
            maybe_timepoint: None,
            call: call.clone(),
            store_call: true,
            max_weight: 1000000000000,
        })
        .dispatch(origin_of(account_of(ALICE))));

        assert_ok!(Call::MultiSig(MultiSigCall::approve_as_multi {
            threshold: 2,
            other_signatories: vec![account_of(ALICE)],
            maybe_timepoint: Some(MultiSigPallet::timepoint()),
            call_hash: sp_core::blake2_256(&call),
            max_weight: 1000000000000,
        })
        .dispatch(origin_of(account_of(BOB))));

        // max amount should be locked in vesting
        assert_eq!(
            TokensPallet::locks(&account_of(EVE), Token(INTR))
                .iter()
                .map(|balance_lock| balance_lock.amount)
                .max()
                .unwrap_or_default(),
            vesting_amount
        );
    });
}

#[test]
fn integration_test_batched_multisig_vesting() {
    ExtBuilder::build().execute_with(|| {
        // authorize and execute a batch of 1000 vesting schedules

        let accounts: Vec<_> = (0u32..1000)
            .map(|x| {
                let mut byte_vec = x.to_be_bytes().to_vec();
                byte_vec.extend(&[0; 28]);
                let arr: [u8; 32] = byte_vec.try_into().unwrap();
                AccountId::from(arr)
            })
            .collect();

        // arbitrary amount for each account
        let vesting_amounts: Vec<_> = (0u128..1000).map(|x| x * 100 + 100).collect();

        let multisig_account = MultiSigPallet::multi_account_id(&vec![account_of(ALICE), account_of(BOB)], 2);

        // vested transfer takes free balance of caller
        assert_ok!(Call::Tokens(TokensCall::set_balance {
            who: multisig_account.clone(),
            currency_id: Token(INTR),
            new_free: vesting_amounts.iter().sum(),
            new_reserved: 0,
        })
        .dispatch(root()));

        // gradually release amount over 100 periods
        let calls: Vec<_> = accounts
            .iter()
            .zip(vesting_amounts.iter())
            .map(|(account, vesting_amount)| {
                Call::Vesting(VestingCall::vested_transfer {
                    dest: account.clone(),
                    schedule: VestingSchedule {
                        start: 0,
                        period: 10,
                        period_count: 100,
                        per_period: vesting_amount / 100,
                    },
                })
            })
            .collect();

        let batch = Call::Utility(UtilityCall::batch { calls }).encode();

        assert_ok!(Call::MultiSig(MultiSigCall::as_multi {
            threshold: 2,
            other_signatories: vec![account_of(BOB)],
            maybe_timepoint: None,
            call: batch.clone(),
            store_call: true,
            max_weight: 1000000000000,
        })
        .dispatch(origin_of(account_of(ALICE))));

        assert_ok!(Call::MultiSig(MultiSigCall::approve_as_multi {
            threshold: 2,
            other_signatories: vec![account_of(ALICE)],
            maybe_timepoint: Some(MultiSigPallet::timepoint()),
            call_hash: sp_core::blake2_256(&batch),
            max_weight: 1000000000000,
        })
        .dispatch(origin_of(account_of(BOB))));

        // max amount should be locked in vesting
        for (account, vesting_amount) in accounts.iter().zip(vesting_amounts) {
            assert_eq!(
                TokensPallet::locks(&account, Token(INTR))
                    .iter()
                    .map(|balance_lock| balance_lock.amount)
                    .max()
                    .unwrap_or_default(),
                vesting_amount
            );
        }
    });
}
