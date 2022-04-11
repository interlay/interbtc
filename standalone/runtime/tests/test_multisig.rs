mod mock;

use frame_support::traits::WrapperKeepOpaque;
use mock::{assert_eq, *};
use orml_tokens::AccountData;
use orml_vesting::VestingSchedule;
use sp_core::{crypto::Ss58Codec, Encode, H256};
use sp_std::str::FromStr;

type VestingCall = orml_vesting::Call<Runtime>;

fn set_balance(who: AccountId, currency_id: CurrencyId, new_free: Balance) {
    assert_ok!(Call::Tokens(TokensCall::set_balance {
        who,
        currency_id,
        new_free,
        new_reserved: 0,
    })
    .dispatch(root()));
}

#[test]
fn integration_test_transfer_from_multisig_to_vested() {
    ExtBuilder::build().execute_with(|| {
        // step 0: clear eve's balance for easier testing
        assert_ok!(Call::Tokens(TokensCall::set_balance {
            who: account_of(EVE),
            currency_id: Token(INTR),
            new_free: 0,
            new_reserved: 0,
        })
        .dispatch(root()));

        // step 1: deposit funds to a shared account
        let multisig_account = MultisigPallet::multi_account_id(&vec![account_of(ALICE), account_of(BOB)], 2);
        set_balance(multisig_account.clone(), Token(INTR), 20_000_000_000_001);
        set_balance(account_of(ALICE), Token(INTR), 1 << 60);

        // step 2: submit a call, to be executed from the shared account
        let call = Call::Tokens(TokensCall::transfer {
            dest: account_of(EVE),
            currency_id: Token(INTR),
            amount: 20_000_000_000_001,
        })
        .encode();
        assert_ok!(Call::Multisig(MultisigCall::as_multi {
            threshold: 2,
            other_signatories: vec![account_of(BOB)],
            maybe_timepoint: None,
            call: WrapperKeepOpaque::from_encoded(call.clone()),
            store_call: true,
            max_weight: 1000000000000,
        })
        .dispatch(origin_of(account_of(ALICE))));

        // step 2a: balance should not have changed yet - the call is not executed yet
        assert_eq!(
            TokensPallet::accounts(account_of(EVE), Token(INTR)),
            AccountData {
                free: 0,
                reserved: 0,
                frozen: 0,
            }
        );

        // step 3: get the timepoint at which the call was made. In production, you would get this
        // from the event metadata, or from storage
        let timepoint = MultisigPallet::timepoint();

        // step 4: let the second account approve
        assert_ok!(Call::Multisig(MultisigCall::approve_as_multi {
            threshold: 2,
            other_signatories: vec![account_of(ALICE)],
            maybe_timepoint: Some(timepoint),
            call_hash: sp_core::blake2_256(&call),
            max_weight: 1000000000000,
        })
        .dispatch(origin_of(account_of(BOB))));
        // step 4a: check that the call is now executed
        assert_eq!(
            TokensPallet::accounts(account_of(EVE), Token(INTR)),
            AccountData {
                free: 20_000_000_000_001,
                reserved: 0,
                frozen: 0,
            }
        );
    });
}

#[test]
fn integration_test_transfer_from_multisig_to_unvested() {
    ExtBuilder::build().execute_with(|| {
        let vesting_amount = 30_000_000;
        let multisig_account = MultisigPallet::multi_account_id(&vec![account_of(ALICE), account_of(BOB)], 2);

        // vested transfer takes free balance of caller
        set_balance(multisig_account.clone(), Token(INTR), vesting_amount);
        set_balance(account_of(ALICE), Token(INTR), 1 << 60);
        // clear eve's balance
        set_balance(account_of(EVE), Token(INTR), 0);

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

        assert_ok!(Call::Multisig(MultisigCall::as_multi {
            threshold: 2,
            other_signatories: vec![account_of(BOB)],
            maybe_timepoint: None,
            call: WrapperKeepOpaque::from_encoded(call.clone()),
            store_call: true,
            max_weight: 1000000000000,
        })
        .dispatch(origin_of(account_of(ALICE))));

        assert_ok!(Call::Multisig(MultisigCall::approve_as_multi {
            threshold: 2,
            other_signatories: vec![account_of(ALICE)],
            maybe_timepoint: Some(MultisigPallet::timepoint()),
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
        assert_eq!(
            TokensPallet::accounts(account_of(EVE), Token(INTR)),
            AccountData {
                free: vesting_amount,
                reserved: 0,
                frozen: vesting_amount,
            }
        );
    });
}

#[test]
fn integration_test_transfer_to_vested_multisig() {
    ExtBuilder::build().execute_with(|| {
        // step 0: setup eve's balance
        assert_ok!(Call::Tokens(TokensCall::set_balance {
            who: account_of(EVE),
            currency_id: Token(INTR),
            new_free: 20_000_000_000_001,
            new_reserved: 0,
        })
        .dispatch(root()));

        // calculate accountid for the multisig
        let multisig_account = MultisigPallet::multi_account_id(&vec![account_of(ALICE), account_of(BOB)], 2);

        // transfer to the multisig
        assert_ok!(Call::Tokens(TokensCall::transfer {
            dest: multisig_account.clone(),
            currency_id: Token(INTR),
            amount: 20_000_000_000_001,
        })
        .dispatch(origin_of(account_of(EVE))));

        assert_eq!(
            TokensPallet::accounts(multisig_account, Token(INTR)),
            AccountData {
                free: 20_000_000_000_001,
                reserved: 0,
                frozen: 0,
            }
        );
    });
}

#[test]
fn integration_test_transfer_to_unvested_multisig() {
    // not sure this case would ever be used, best we have a test for it anyway..
    ExtBuilder::build().execute_with(|| {
        let vesting_amount = 30_000_000;
        // step 0: setup eve's balance
        assert_ok!(Call::Tokens(TokensCall::set_balance {
            who: account_of(EVE),
            currency_id: Token(INTR),
            new_free: vesting_amount * 2,
            new_reserved: 0,
        })
        .dispatch(root()));

        // calculate accountid for the multisig
        let multisig_account = MultisigPallet::multi_account_id(&vec![account_of(ALICE), account_of(BOB)], 2);

        // transfer to the multisig
        assert_ok!(Call::Vesting(VestingCall::vested_transfer {
            dest: multisig_account.clone(),
            schedule: VestingSchedule {
                start: 0,
                period: 10,
                period_count: 100,
                per_period: vesting_amount / 100,
            },
        })
        .dispatch(origin_of(account_of(EVE))));

        assert_eq!(
            TokensPallet::accounts(multisig_account, Token(INTR)),
            AccountData {
                free: vesting_amount,
                reserved: 0,
                frozen: vesting_amount,
            }
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

        let multisig_account = MultisigPallet::multi_account_id(&vec![account_of(ALICE), account_of(BOB)], 2);

        // vested transfer takes free balance of caller
        set_balance(multisig_account.clone(), Token(INTR), vesting_amounts.iter().sum());
        set_balance(account_of(ALICE), Token(INTR), 1 << 60);

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

        assert_ok!(Call::Multisig(MultisigCall::as_multi {
            threshold: 2,
            other_signatories: vec![account_of(BOB)],
            maybe_timepoint: None,
            call: WrapperKeepOpaque::from_encoded(batch.clone()),
            store_call: true,
            max_weight: 1000000000000,
        })
        .dispatch(origin_of(account_of(ALICE))));

        assert_ok!(Call::Multisig(MultisigCall::approve_as_multi {
            threshold: 2,
            other_signatories: vec![account_of(ALICE)],
            maybe_timepoint: Some(MultisigPallet::timepoint()),
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

// multisig may produce an invalid address if inputs are not sorted
fn sort_addresses(entries: Vec<AccountId>) -> Vec<AccountId> {
    let mut signatories = entries.clone();
    signatories.sort_by(|left, right| left.cmp(right));
    signatories
}

#[test]
fn should_calculate_sorted_multisig_address() {
    ExtBuilder::build().execute_with(|| {
        // 0xb42637741a394e89426e8026536090c23647fdc0cccd1156785d84ff87ed2eb0
        let multisig_account = MultisigPallet::multi_account_id(
            &sort_addresses(vec![
                AccountId::from_str("5Gn1vqSHnzz61gfXK1wRBcbKtPcSPmxKrpihApnTuvA7NJnj").unwrap(),
                AccountId::from_str("5CyPQSfoHdb626qGyH16D1DJKjxQtZxbF4pbKzTRRGyCchEx").unwrap(),
                AccountId::from_str("5EEj6K6FFDBuMwfS1DtxMDdumWGjqNq34nnbUsFH4vQRfjQi").unwrap(),
                AccountId::from_str("5D2xxiX1ACobFxxD4gvD7pmQRg7q2yi94n4JjtGRHsnh3gns").unwrap(),
            ]),
            2,
        );
        assert_eq!(
            "5DnWh2e4Fi2iDeEjNYMr5iU1RJ4cnG2X7tZcwcFgxJkBrBXX",
            multisig_account.to_ss58check()
        );
    })
}
