use crate::relaychain::polkadot_test_net::*;
use codec::Encode;
use frame_support::{
    assert_noop, assert_ok,
    weights::{Weight as FrameWeight, WeightToFee},
};
use orml_traits::{location::Reserve, MultiCurrency};
use orml_xtokens::{AbsoluteReserveProviderMigrationPhase, MigrationPhase};
use primitives::{
    CurrencyId::{ForeignAsset, Token},
    CustomMetadata, TokenSymbol,
};
use xcm::latest::{prelude::*, Weight};
use xcm_builder::ParentIsPreset;
use xcm_emulator::{TestExt, XcmExecutor};
use xcm_executor::traits::{Convert, WeightBounds};

mod hrmp {
    use super::*;

    use polkadot_runtime_parachains::hrmp;
    fn construct_xcm(call: hrmp::Call<polkadot_runtime::Runtime>, xcm_fee: u128, transact_weight: Weight) -> Xcm<()> {
        Xcm(vec![
            WithdrawAsset((Here, xcm_fee).into()),
            BuyExecution {
                fees: (Here, xcm_fee).into(),
                weight_limit: Unlimited, /* Let polkadot weigh the message. Weight will include the
                                          * `transact.require_weight_at_most` */
            },
            Transact {
                require_weight_at_most: transact_weight,
                origin_kind: OriginKind::Native,
                call: polkadot_runtime::RuntimeCall::Hrmp(call).encode().into(),
            },
            RefundSurplus,
            DepositAsset {
                assets: All.into(),
                beneficiary: Junction::AccountId32 { id: BOB, network: None }.into(),
            },
        ])
    }

    fn has_open_channel_requested_event(sender: u32, recipient: u32) -> bool {
        PolkadotNet::execute_with(|| {
            polkadot_runtime::System::events().iter().any(|r| {
                matches!(
                    r.event,
                    polkadot_runtime::RuntimeEvent::Hrmp(hrmp::Event::OpenChannelRequested(
                        actual_sender,
                        actual_recipient,
                        1000,
                        102400
                    )) if actual_sender == sender.into() && actual_recipient == recipient.into()
                )
            })
        })
    }

    fn has_open_channel_accepted_event(sender: u32, recipient: u32) -> bool {
        PolkadotNet::execute_with(|| {
            polkadot_runtime::System::events().iter().any(|r| {
                matches!(
                    r.event,
                    polkadot_runtime::RuntimeEvent::Hrmp(hrmp::Event::OpenChannelAccepted(
                        actual_sender,
                        actual_recipient
                    )) if actual_sender == sender.into() && actual_recipient == recipient.into()
                )
            })
        })
    }

    fn init_open_channel<T>(sender: u32, recipient: u32, xcm_fee: u128, transact_weight: Weight)
    where
        T: TestExt,
    {
        // do hrmp_init_open_channel
        assert!(!has_open_channel_requested_event(sender, recipient)); // just a sanity check
        T::execute_with(|| {
            let message = construct_xcm(
                hrmp::Call::<polkadot_runtime::Runtime>::hrmp_init_open_channel {
                    recipient: recipient.into(),
                    proposed_max_capacity: 1000,
                    proposed_max_message_size: 102400,
                },
                xcm_fee,
                transact_weight,
            );
            assert_ok!(pallet_xcm::Pallet::<interlay_runtime_parachain::Runtime>::send_xcm(
                Here, Parent, message
            ));
        });
        assert!(has_open_channel_requested_event(sender, recipient));
    }

    fn accept_open_channel<T>(sender: u32, recipient: u32, xcm_fee: u128, transact_weight: Weight)
    where
        T: TestExt,
    {
        // do hrmp_accept_open_channel
        assert!(!has_open_channel_accepted_event(sender, recipient)); // just a sanity check
        T::execute_with(|| {
            let message = construct_xcm(
                hrmp::Call::<polkadot_runtime::Runtime>::hrmp_accept_open_channel { sender: sender.into() },
                xcm_fee,
                transact_weight,
            );
            assert_ok!(pallet_xcm::Pallet::<interlay_runtime_parachain::Runtime>::send_xcm(
                Here, Parent, message
            ));
        });
        assert!(has_open_channel_accepted_event(sender, recipient));
    }

    #[test]
    fn open_hrmp_channel_cheaply() {
        // check that 0.25 DOT is enough
        let xcm_fee = DOT.one() / 4;
        let transact_weight = Weight::from_parts(10_000_000_000, 100_000_000);
        let deposit = 2 * (10 * DOT.one() + xcm_fee) + polkadot_runtime::ExistentialDeposit::get();
        open_hrmp_channel(deposit, xcm_fee, transact_weight);
    }

    #[test]
    fn test_required_transact_weight() {
        // actual minimum transact weight at time of writing is < 700_000_000. Use
        // 800_000_000 so tests don't break every polkadot upgrade
        let xcm_fee = DOT.one() / 5;
        let transact_weight = Weight::from_parts(800_000_000, 50_000_000);
        let deposit = 2 * (10 * DOT.one() + xcm_fee) + polkadot_runtime::ExistentialDeposit::get();
        open_hrmp_channel(deposit, xcm_fee, transact_weight);
    }

    #[test]
    fn open_hrmp_channel_with_buffer() {
        // the actual values used in production: about twice the minimum amounts
        let xcm_fee = DOT.one() / 2;
        let transact_weight = Weight::from_parts(10_000_000_000, 100_000_000);
        let deposit = 2 * (10 * DOT.one() + xcm_fee) + polkadot_runtime::ExistentialDeposit::get();
        open_hrmp_channel(deposit, xcm_fee, transact_weight);
    }

    fn open_hrmp_channel(initial_balance: u128, xcm_fee: u128, transact_weight: Weight) {
        let existential_deposit = DOT.one();

        // setup sovereign account balances
        PolkadotNet::execute_with(|| {
            assert_ok!(polkadot_runtime::Balances::transfer(
                polkadot_runtime::RuntimeOrigin::signed(ALICE.into()),
                sp_runtime::MultiAddress::Id(interlay_sovereign_account_on_polkadot()),
                initial_balance
            ));
            assert_ok!(polkadot_runtime::Balances::transfer(
                polkadot_runtime::RuntimeOrigin::signed(ALICE.into()),
                sp_runtime::MultiAddress::Id(sibling_sovereign_account_on_polkadot()),
                initial_balance
            ));
            assert_ok!(polkadot_runtime::Balances::transfer(
                polkadot_runtime::RuntimeOrigin::signed(ALICE.into()),
                sp_runtime::MultiAddress::Id(BOB.into()),
                existential_deposit
            ));
        });

        // open channel interlay -> sibling
        init_open_channel::<Interlay>(INTERLAY_PARA_ID, SIBLING_PARA_ID, xcm_fee, transact_weight);
        accept_open_channel::<Sibling>(INTERLAY_PARA_ID, SIBLING_PARA_ID, xcm_fee, transact_weight);

        // open channel sibling -> interlay
        init_open_channel::<Sibling>(SIBLING_PARA_ID, INTERLAY_PARA_ID, xcm_fee, transact_weight);
        accept_open_channel::<Interlay>(SIBLING_PARA_ID, INTERLAY_PARA_ID, xcm_fee, transact_weight);

        // check that Bob received left-over funds (from both Interlay and Sibling).
        PolkadotNet::execute_with(|| {
            let free_balance = polkadot_runtime::Balances::free_balance(&AccountId::from(BOB));
            assert!(free_balance > existential_deposit);
        });
    }
}

#[test]
fn test_transact_barrier() {
    let call = orml_tokens::Call::<interlay_runtime_parachain::Runtime>::transfer_all {
        dest: ALICE.into(),
        currency_id: Token(DOT),
        keep_alive: false,
    };
    let message = Xcm(vec![
        WithdrawAsset((Here, 410000000000u128).into()),
        BuyExecution {
            fees: (Here, 400000000000u128).into(),
            weight_limit: Unlimited,
        },
        Transact {
            require_weight_at_most: Weight::from_parts(10000000000, 0u64),
            origin_kind: OriginKind::Native,
            call: interlay_runtime_parachain::RuntimeCall::Tokens(call).encode().into(),
        },
        RefundSurplus,
        DepositAsset {
            assets: All.into(),
            beneficiary: Junction::AccountId32 { id: BOB, network: None }.into(),
        },
    ]);

    PolkadotNet::execute_with(|| {
        assert_ok!(pallet_xcm::Pallet::<polkadot_runtime::Runtime>::send_xcm(
            Here,
            X1(Parachain(2032)),
            message
        ));
    });

    Interlay::execute_with(|| {
        assert!(System::events().iter().any(|r| matches!(
            r.event,
            RuntimeEvent::DmpQueue(cumulus_pallet_dmp_queue::Event::ExecutedDownward {
                outcome: Outcome::Error(XcmError::Barrier),
                ..
            })
        )));
    });
}

#[test]
fn transfer_from_relay_chain() {
    PolkadotNet::execute_with(|| {
        assert_ok!(polkadot_runtime::XcmPallet::reserve_transfer_assets(
            polkadot_runtime::RuntimeOrigin::signed(ALICE.into()),
            Box::new(Parachain(INTERLAY_PARA_ID).into_versioned()),
            Box::new(Junction::AccountId32 { id: BOB, network: None }.into_versioned()),
            Box::new((Here, DOT.one()).into()),
            0
        ));
    });

    Interlay::execute_with(|| {
        // use an upperbound rather than an exact value so this check doesn't break at each minor update
        let xcm_fee_over_estimation = 20_000_000;

        assert!(Tokens::free_balance(Token(DOT), &AccountId::from(BOB)) > DOT.one() - xcm_fee_over_estimation);
        // rest should go to treasury:
        assert_eq!(
            Tokens::free_balance(Token(DOT), &TreasuryAccount::get()),
            DOT.one() - Tokens::free_balance(Token(DOT), &AccountId::from(BOB))
        );
    });
}

#[test]
fn transfer_to_relay_chain() {
    PolkadotNet::execute_with(|| {
        assert_ok!(polkadot_runtime::Balances::transfer(
            polkadot_runtime::RuntimeOrigin::signed(ALICE.into()),
            sp_runtime::MultiAddress::Id(interlay_sovereign_account_on_polkadot()),
            2 * DOT.one()
        ));
    });

    let used_weight = FrameWeight::from_parts(4_000_000_000 as u64, 0u64); // The value used in UI - very conservative: actually used at time of writing = 298_368_000

    Interlay::execute_with(|| {
        assert_ok!(XTokens::transfer(
            RuntimeOrigin::signed(ALICE.into()),
            Token(DOT),
            2 * DOT.one(),
            Box::new(MultiLocation::new(1, X1(Junction::AccountId32 { id: BOB, network: None })).into()),
            WeightLimit::Unlimited
        ));
    });

    PolkadotNet::execute_with(|| {
        let fee =
            <polkadot_runtime::Runtime as pallet_transaction_payment::Config>::WeightToFee::weight_to_fee(&used_weight);
        assert_eq!(
            polkadot_runtime::Balances::free_balance(&AccountId::from(BOB)),
            2 * DOT.one() - fee
        );

        // UI uses 482771107 - make sure that that's an overestimation
        assert!(fee < 482771107);
    });
}

/// Send INTR to sibling. On the sibling, it will be registered as a foreign asset.
/// By also transferring it back, we test that the asset-registry has been properly
/// integrated.
#[test]
fn transfer_to_sibling_and_back() {
    fn sibling_sovereign_account() -> AccountId {
        use sp_runtime::traits::AccountIdConversion;
        polkadot_parachain::primitives::Sibling::from(SIBLING_PARA_ID).into_account_truncating()
    }

    Sibling::execute_with(|| {
        register_intr_as_foreign_asset();
    });

    Interlay::execute_with(|| {
        assert_ok!(Tokens::deposit(
            Token(INTR),
            &AccountId::from(ALICE),
            100_000_000_000_000
        ));
    });

    Interlay::execute_with(|| {
        assert_ok!(XTokens::transfer(
            RuntimeOrigin::signed(ALICE.into()),
            Token(INTR),
            10_000_000_000_000,
            Box::new(
                MultiLocation::new(
                    1,
                    X2(
                        Parachain(SIBLING_PARA_ID),
                        Junction::AccountId32 {
                            network: None,
                            id: BOB.into(),
                        }
                    )
                )
                .into()
            ),
            WeightLimit::Unlimited,
        ));

        assert_eq!(
            Tokens::free_balance(Token(INTR), &AccountId::from(ALICE)),
            90_000_000_000_000
        );

        assert_eq!(
            Tokens::free_balance(Token(INTR), &sibling_sovereign_account()),
            10_000_000_000_000
        );
    });

    Sibling::execute_with(|| {
        let xcm_fee = 800_000_000;

        // check reception
        assert_eq!(
            Tokens::free_balance(ForeignAsset(1), &AccountId::from(BOB)),
            10_000_000_000_000 - xcm_fee
        );

        // return some back to interlay
        assert_ok!(XTokens::transfer(
            RuntimeOrigin::signed(BOB.into()),
            ForeignAsset(1),
            5_000_000_000_000,
            Box::new(
                MultiLocation::new(
                    1,
                    X2(
                        Parachain(INTERLAY_PARA_ID),
                        Junction::AccountId32 {
                            network: None,
                            id: ALICE.into(),
                        }
                    )
                )
                .into()
            ),
            WeightLimit::Unlimited,
        ));
    });

    // check reception
    Interlay::execute_with(|| {
        let used_weight = 800_000_000; // empirically determined in test - weight is decreased in AllowTopLevelPaidExecutionFrom
        let intr_per_second = interlay_runtime_parachain::xcm_config::CanonicalizedIntrPerSecond::get().1;
        let xcm_fee = (intr_per_second * used_weight) / WEIGHT_REF_TIME_PER_SECOND as u128;

        assert_eq!(
            Tokens::free_balance(Token(INTR), &AccountId::from(ALICE)),
            95_000_000_000_000 - xcm_fee
        );

        assert_eq!(Tokens::free_balance(Token(INTR), &TreasuryAccount::get()), xcm_fee);

        assert_eq!(
            Tokens::free_balance(Token(INTR), &sibling_sovereign_account()),
            5_000_000_000_000
        );
    });
}

#[test]
fn xcm_transfer_execution_barrier_trader_works() {
    fn construct_xcm<T>(amount: u128, limit: WeightLimit) -> Xcm<T> {
        Xcm(vec![
            ReserveAssetDeposited((Parent, amount).into()),
            BuyExecution {
                fees: (Parent, amount).into(),
                weight_limit: limit,
            },
            DepositAsset {
                assets: All.into(),
                beneficiary: Here.into(),
            },
        ])
    }

    let expect_weight_limit = <interlay_runtime_parachain::xcm_config::XcmConfig as interlay_runtime_parachain::xcm_config::xcm_executor::Config>::Weigher::weight(
        &mut construct_xcm(100, Unlimited)).unwrap();
    let weight_limit_too_low = Weight::from_parts(500_000_000, 0u64);
    let unit_instruction_weight = Weight::from_parts(200_000_000, 0u64);
    let minimum_fee = (interlay_runtime_parachain::xcm_config::DotPerSecond::get().1
        * expect_weight_limit.ref_time() as u128)
        / WEIGHT_REF_TIME_PER_SECOND as u128;

    // relay-chain use normal account to send xcm, destination parachain can't pass Barrier check
    let message = construct_xcm(100, Unlimited);
    PolkadotNet::execute_with(|| {
        // Polkadot effectively disabled the `send` extrinsic in 0.9.19, so use send_xcm
        assert_ok!(pallet_xcm::Pallet::<polkadot_runtime::Runtime>::send_xcm(
            X1(Junction::AccountId32 {
                network: None,
                id: ALICE.into(),
            }),
            Parachain(INTERLAY_PARA_ID),
            message
        ));
    });
    Interlay::execute_with(|| {
        assert!(System::events().iter().any(|r| matches!(
            r.event,
            RuntimeEvent::DmpQueue(cumulus_pallet_dmp_queue::Event::ExecutedDownward {
                outcome: Outcome::Error(XcmError::Barrier),
                ..
            })
        )));
    });

    // AllowTopLevelPaidExecutionFrom barrier test case:
    // para-chain use XcmExecutor `execute_xcm()` method to execute xcm.
    // if `weight_limit` in BuyExecution is less than `xcm_weight(max_weight)`, then Barrier can't pass.
    // other situation when `weight_limit` is `Unlimited` or large than `xcm_weight`, then it's ok.
    let message = construct_xcm(100, Limited(weight_limit_too_low));
    Interlay::execute_with(|| {
        let hash = message.using_encoded(sp_io::hashing::blake2_256);
        let r = XcmExecutor::<XcmConfig>::execute_xcm(Parent, message, hash, expect_weight_limit);
        assert_eq!(r, Outcome::Error(XcmError::Barrier));
    });

    // trader inside BuyExecution have TooExpensive error if payment less than calculated weight amount.
    // the minimum of calculated weight amount(`FixedRateOfFungible<KsmPerSecond>`) is 96_000_000

    let message = construct_xcm(minimum_fee - 1, Limited(expect_weight_limit));
    Interlay::execute_with(|| {
        let hash = message.using_encoded(sp_io::hashing::blake2_256);
        let r = XcmExecutor::<XcmConfig>::execute_xcm(Parent, message, hash, expect_weight_limit);
        assert_eq!(
            r,
            Outcome::Incomplete(expect_weight_limit - unit_instruction_weight, XcmError::TooExpensive)
        );
    });

    // all situation fulfilled, execute success
    let message = construct_xcm(minimum_fee, Limited(expect_weight_limit));
    Interlay::execute_with(|| {
        let hash = message.using_encoded(sp_io::hashing::blake2_256);
        let r = XcmExecutor::<XcmConfig>::execute_xcm(Parent, message, hash, expect_weight_limit);
        assert_eq!(r, Outcome::Complete(expect_weight_limit));
    });
}

#[test]
fn subscribe_version_notify_works() {
    // relay chain subscribe version notify of para chain
    PolkadotNet::execute_with(|| {
        let r = pallet_xcm::Pallet::<polkadot_runtime::Runtime>::force_subscribe_version_notify(
            polkadot_runtime::RuntimeOrigin::root(),
            Box::new(Parachain(INTERLAY_PARA_ID).into_versioned()),
        );
        assert_ok!(r);
    });
    PolkadotNet::execute_with(|| {
        polkadot_runtime::System::assert_has_event(polkadot_runtime::RuntimeEvent::XcmPallet(
            pallet_xcm::Event::SupportedVersionChanged(
                MultiLocation {
                    parents: 0,
                    interior: X1(Parachain(INTERLAY_PARA_ID)),
                },
                3,
            ),
        ));
    });

    // para chain subscribe version notify of relay chain
    Interlay::execute_with(|| {
        let r = pallet_xcm::Pallet::<interlay_runtime_parachain::Runtime>::force_subscribe_version_notify(
            RuntimeOrigin::root(),
            Box::new(Parent.into()),
        );
        assert_ok!(r);
    });
    Interlay::execute_with(|| {
        System::assert_has_event(interlay_runtime_parachain::RuntimeEvent::PolkadotXcm(
            pallet_xcm::Event::SupportedVersionChanged(
                MultiLocation {
                    parents: 1,
                    interior: Here,
                },
                3,
            ),
        ));
    });

    // para chain subscribe version notify of sibling chain
    Interlay::execute_with(|| {
        let r = pallet_xcm::Pallet::<interlay_runtime_parachain::Runtime>::force_subscribe_version_notify(
            RuntimeOrigin::root(),
            Box::new((Parent, Parachain(SIBLING_PARA_ID)).into()),
        );
        assert_ok!(r);
    });
    Interlay::execute_with(|| {
        assert!(interlay_runtime_parachain::System::events().iter().any(|r| matches!(
            r.event,
            interlay_runtime_parachain::RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::XcmpMessageSent {
                message_hash: Some(_)
            })
        )));
    });
    Sibling::execute_with(|| {
        assert!(interlay_runtime_parachain::System::events().iter().any(|r| matches!(
            r.event,
            interlay_runtime_parachain::RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::XcmpMessageSent {
                message_hash: Some(_)
            }) | interlay_runtime_parachain::RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::Success {
                message_hash: Some(_),
                weight: _
            })
        )));
    });
}

fn weigh_xcm(mut message: Xcm<RuntimeCall>, fee_per_second: u128) -> u128 {
    let trapped_xcm_message_weight = <interlay_runtime_parachain::xcm_config::XcmConfig as interlay_runtime_parachain::xcm_config::xcm_executor::Config>::Weigher::weight(
        &mut message).unwrap();
    (fee_per_second * trapped_xcm_message_weight.ref_time() as u128) / WEIGHT_REF_TIME_PER_SECOND as u128
}

fn general_key_of(token_symbol: TokenSymbol) -> Junction {
    let id = Token(token_symbol);
    let encoded = id.encode();
    let mut data = [0u8; 32];
    if encoded.len() > 32 {
        // we are not returning result, so panic is inevitable. Let's make it explicit.
        panic!("Currency ID was too long to be encoded");
    }
    data[..encoded.len()].copy_from_slice(&encoded[..]);
    GeneralKey {
        length: encoded.len() as u8,
        data,
    }
}

#[test]
fn trap_assets_works() {
    let mut intr_treasury_amount = 0;
    let (ksm_asset_amount, intr_asset_amount) = (10 * DOT.one(), 10 * INTR.one());

    let parent_account: AccountId = ParentIsPreset::<AccountId>::convert(Parent.into()).unwrap();

    Interlay::execute_with(|| {
        assert_ok!(Tokens::deposit(Token(DOT), &parent_account, 100 * DOT.one()));
        assert_ok!(Tokens::deposit(Token(INTR), &parent_account, 100 * INTR.one()));

        intr_treasury_amount = Tokens::free_balance(Token(INTR), &TreasuryAccount::get());
    });

    let assets: MultiAsset = (Parent, ksm_asset_amount).into();

    fn construct_xcm<T>(assets: MultiAsset, intr_asset_amount: Balance) -> Xcm<T> {
        Xcm(vec![
            WithdrawAsset(assets.clone().into()),
            BuyExecution {
                fees: assets,
                weight_limit: Limited(Weight::from_parts(DOT.one() as u64, 0u64)),
            },
            WithdrawAsset(
                (
                    (Parent, X2(Parachain(INTERLAY_PARA_ID), general_key_of(INTR))),
                    intr_asset_amount,
                )
                    .into(),
            ),
        ])
    }

    let trapped_xcm_message_fee = weigh_xcm(
        construct_xcm(assets.clone(), intr_asset_amount),
        interlay_runtime_parachain::xcm_config::DotPerSecond::get().1,
    );

    // Withdraw intr and ksm on interlay but don't deposit it
    PolkadotNet::execute_with(|| {
        assert_ok!(pallet_xcm::Pallet::<polkadot_runtime::Runtime>::send_xcm(
            Here,
            Parachain(INTERLAY_PARA_ID),
            construct_xcm(assets.clone(), intr_asset_amount),
        ));
    });

    let mut trapped_assets: Option<MultiAssets> = None;
    // verify that the assets got trapped (i.e. didn't get burned)
    Interlay::execute_with(|| {
        assert!(System::events().iter().any(|r| matches!(
            r.event,
            RuntimeEvent::PolkadotXcm(pallet_xcm::Event::AssetsTrapped(_, _, _))
        )));

        let event = System::events()
            .iter()
            .find(|r| {
                matches!(
                    r.event,
                    RuntimeEvent::PolkadotXcm(pallet_xcm::Event::AssetsTrapped(_, _, _))
                )
            })
            .cloned()
            .unwrap();

        use std::convert::TryFrom;
        use xcm::VersionedMultiAssets;
        trapped_assets = match event.event {
            RuntimeEvent::PolkadotXcm(pallet_xcm::Event::AssetsTrapped(_, _, ticket)) => {
                Some(TryFrom::<VersionedMultiAssets>::try_from(ticket).unwrap())
            }
            _ => panic!("event not found"),
        };

        // unchanged treasury amounts
        assert_eq!(
            trapped_xcm_message_fee,
            Tokens::free_balance(Token(DOT), &TreasuryAccount::get())
        );
        assert_eq!(
            intr_treasury_amount,
            Tokens::free_balance(Token(INTR), &TreasuryAccount::get())
        );
    });

    let trapped_intr_amount = trapped_assets
        .clone()
        .unwrap()
        .into_inner()
        .into_iter()
        .find_map(|x| match x {
            MultiAsset {
                id: AssetId::Concrete(location),
                fun: Fungibility::Fungible(amount),
            } if location == (Parent, X2(Parachain(INTERLAY_PARA_ID), general_key_of(INTR))).into() => Some(amount),
            _ => None,
        })
        .unwrap();

    let trapped_dot_amount = trapped_assets
        .clone()
        .unwrap()
        .into_inner()
        .into_iter()
        .find_map(|x| match x {
            MultiAsset {
                id: AssetId::Concrete(location),
                fun: Fungibility::Fungible(amount),
            } if location == Parent.into() => Some(amount),
            _ => None,
        })
        .unwrap();

    fn construct_reclaiming_xcm<T>(trapped_assets: Option<MultiAssets>, intr_asset_amount: Balance) -> Xcm<T> {
        Xcm(vec![
            ClaimAsset {
                assets: trapped_assets.unwrap(),
                ticket: Here.into(),
            },
            BuyExecution {
                fees: (
                    (Parent, X2(Parachain(INTERLAY_PARA_ID), general_key_of(INTR))),
                    intr_asset_amount / 4,
                )
                    .into(),
                weight_limit: Limited(Weight::from_parts(4_000_000_000_000, 0u64)),
            },
            DepositAsset {
                assets: All.into(),
                beneficiary: Junction::AccountId32 { id: BOB, network: None }.into(),
            },
        ])
    }

    // Now reclaim trapped assets
    PolkadotNet::execute_with(|| {
        assert_ok!(pallet_xcm::Pallet::<polkadot_runtime::Runtime>::send_xcm(
            Here,
            Parachain(INTERLAY_PARA_ID),
            construct_reclaiming_xcm(trapped_assets.clone(), intr_asset_amount),
        ));
    });

    // verify that assets were claimed successfully (deposited into Bob's account)
    Interlay::execute_with(|| {
        let reclaim_xcm_fee = weigh_xcm(
            construct_reclaiming_xcm(trapped_assets, intr_asset_amount),
            interlay_runtime_parachain::xcm_config::IntrPerSecond::get().1,
        );
        assert_eq!(
            Tokens::free_balance(Token(INTR), &AccountId::from(BOB)),
            trapped_intr_amount - reclaim_xcm_fee
        );
        assert!(trapped_dot_amount > 0);
        assert_eq!(
            Tokens::free_balance(Token(DOT), &AccountId::from(BOB)),
            trapped_dot_amount
        );
    });
}

fn register_intr_as_foreign_asset() {
    let metadata = AssetMetadata {
        decimals: 12,
        name: "Interlay native".as_bytes().to_vec(),
        symbol: "extINTR".as_bytes().to_vec(),
        existential_deposit: 0,
        location: Some(MultiLocation::new(1, X2(Parachain(INTERLAY_PARA_ID), general_key_of(INTR))).into()),
        additional: CustomMetadata {
            fee_per_second: 1_000_000_000_000,
            coingecko_id: "interlay".as_bytes().to_vec(),
        },
    };
    AssetRegistry::register_asset(RuntimeOrigin::root(), metadata, None).unwrap();
}

// ** ASSET HUB MIGRATION TESTS**
fn concrete_fungible(id: MultiLocation) -> MultiAsset {
    (id, 1).into()
}
fn register_sibling_asset_as_foreign_asset() {
    let metadata = AssetMetadata {
        decimals: 8,
        name: "kBTC".as_bytes().to_vec(),
        symbol: "KBTC".as_bytes().to_vec(),
        existential_deposit: 0,
        location: Some(MultiLocation::new(1, X1(Parachain(SIBLING_PARA_ID))).into()),
        additional: CustomMetadata {
            fee_per_second: 1_000_000_000_000,
            coingecko_id: "kbtc".as_bytes().to_vec(),
        },
    };
    AssetRegistry::register_asset(RuntimeOrigin::root(), metadata, None).unwrap();
}

fn sovereign_account_on_sibling(para_id: u32) -> AccountId {
    use sp_runtime::traits::AccountIdConversion;
    polkadot_parachain::primitives::Sibling::from(para_id).into_account_truncating()
}

// Test that xtokens transfers are disabled during the AHM, and that the DOT reserve is correct
// both before and after the migration. The patch was introduced here:
// https://github.com/r0gue-io/open-runtime-module-library/blob/master/xtokens/src/lib.rs#L556.
// so, it's called by every call on xtokens, so it's enough with testing this out just with
// the transfer method.
#[test]
fn ahm_transfer_of_dot_via_xtokens_to_sibling_parachain() {
    let interlay_sovereign_account_on_asset_hub = sovereign_account_on_sibling(INTERLAY_PARA_ID);
    let sibling_sovereign_account_on_asset_hub = sovereign_account_on_sibling(SIBLING_PARA_ID);

    TestNet::reset();
    // Fund sovereign accounts
    PolkadotNet::execute_with(|| {
        assert_ok!(polkadot_runtime::Balances::transfer(
            polkadot_runtime::RuntimeOrigin::signed(ALICE.into()),
            sp_runtime::MultiAddress::Id(interlay_sovereign_account_on_polkadot()),
            10 * DOT.one()
        ));
    });
    AssetHub::execute_with(|| {
        assert_ok!(polkadot_asset_hub_runtime::Balances::force_set_balance(
            polkadot_asset_hub_runtime::RuntimeOrigin::root(),
            sp_runtime::MultiAddress::Id(interlay_sovereign_account_on_asset_hub.clone()),
            10 * DOT.one()
        ));
    });

    // Before the migration => Everything goes as always, transfers are possible and the reserve is
    // Polkadot
    Interlay::execute_with(|| {
        assert_eq!(
            AbsoluteReserveProviderMigrationPhase::<Runtime>::reserve(&concrete_fungible(MultiLocation::parent())),
            Some(MultiLocation::parent())
        );

        assert_ok!(XTokens::transfer(
            RuntimeOrigin::signed(ALICE.into()),
            Token(DOT),
            DOT.one(),
            Box::new(
                MultiLocation::new(
                    1,
                    X2(
                        Junction::Parachain(SIBLING_PARA_ID),
                        Junction::AccountId32 { id: BOB, network: None }
                    )
                )
                .into()
            ),
            WeightLimit::Unlimited
        ));
    });

    Sibling::execute_with(|| {
        let bob_balance = Tokens::free_balance(Token(DOT), &AccountId::from(BOB));

        // A little bit less to pay fees
        assert!(bob_balance > 0);
        assert!(bob_balance < DOT.one());
    });

    // During the migration the transfer is deactivated
    Interlay::execute_with(|| {
        assert_ok!(XTokens::set_migration_phase(
            RuntimeOrigin::root(),
            MigrationPhase::InProgress
        ));

        assert_noop!(
            XTokens::transfer(
                RuntimeOrigin::signed(ALICE.into()),
                Token(DOT),
                DOT.one(),
                Box::new(
                    MultiLocation::new(
                        1,
                        X2(
                            Junction::Parachain(SIBLING_PARA_ID),
                            Junction::AccountId32 { id: BOB, network: None }
                        )
                    )
                    .into()
                ),
                WeightLimit::Unlimited
            ),
            orml_xtokens::Error::<Runtime>::AssetHasNoReserve
        );
    });

    // After the migration, the transfer is available again, but the reserve is now Polkadot AH
    Interlay::execute_with(|| {
        assert_ok!(XTokens::set_migration_phase(
            RuntimeOrigin::root(),
            MigrationPhase::Completed
        ));
    });

    // Before executing the transfer, we need to set the sibling's migration status to Completed.
    // It's runtime is Interlay's runtime anyway, so it should be able to recognize AH as the
    // reserve.
    Sibling::execute_with(|| {
        assert_ok!(XTokens::set_migration_phase(
            RuntimeOrigin::root(),
            MigrationPhase::Completed
        ));
    });

    Interlay::execute_with(|| {
        assert_eq!(
            AbsoluteReserveProviderMigrationPhase::<Runtime>::reserve(&concrete_fungible(MultiLocation::parent())),
            Some(MultiLocation::new(1, Junction::Parachain(POLKADOT_ASSET_HUB_PARA_ID)))
        );

        assert_ok!(XTokens::transfer(
            RuntimeOrigin::signed(ALICE.into()),
            Token(DOT),
            DOT.one(),
            Box::new(
                MultiLocation::new(
                    1,
                    X2(
                        Junction::Parachain(SIBLING_PARA_ID),
                        Junction::AccountId32 { id: BOB, network: None }
                    )
                )
                .into()
            ),
            WeightLimit::Unlimited
        ));
    });

    AssetHub::execute_with(|| {
        let interlay_balance =
            polkadot_asset_hub_runtime::Balances::free_balance(interlay_sovereign_account_on_asset_hub);
        let sibling_balance =
            polkadot_asset_hub_runtime::Balances::free_balance(sibling_sovereign_account_on_asset_hub);
        assert!(interlay_balance < 10 * DOT.one());
        assert!(sibling_balance > 0);
    });

    Sibling::execute_with(|| {
        let bob_balance = Tokens::free_balance(Token(DOT), &AccountId::from(BOB));

        // A little bit less to pay fees
        assert!(bob_balance > DOT.one());
        assert!(bob_balance < 2 * DOT.one());
    });
}

#[test]
fn ahm_transfer_dot_from_polkadot_to_interlay() {
    TestNet::reset();

    // Pre migration -> Everything's ok
    PolkadotNet::execute_with(|| {
        assert_ok!(polkadot_runtime::XcmPallet::reserve_transfer_assets(
            polkadot_runtime::RuntimeOrigin::signed(ALICE.into()),
            Box::new(Parachain(INTERLAY_PARA_ID).into_versioned()),
            Box::new(Junction::AccountId32 { id: BOB, network: None }.into_versioned()),
            Box::new((Here, DOT.one()).into()),
            0
        ));
    });

    Interlay::execute_with(|| {
        let bob = Tokens::free_balance(Token(DOT), &AccountId::from(BOB));
        assert!(bob > 0);
        assert!(bob < DOT.one());

        // Change the migration status
        assert_ok!(XTokens::set_migration_phase(
            RuntimeOrigin::root(),
            MigrationPhase::InProgress
        ));
    });

    // During the migration, the extrinsic's ok in Polkadot but the balance doesn't get to BOB.
    // NOTE: This is Ok here because this Polkadot version's not aware of the migration. In
    // production, this extrinsic will be directly blocked on Polkadot. But this proves that
    // Interlay's reserve is deactivated.
    PolkadotNet::execute_with(|| {
        assert_ok!(polkadot_runtime::XcmPallet::reserve_transfer_assets(
            polkadot_runtime::RuntimeOrigin::signed(ALICE.into()),
            Box::new(Parachain(INTERLAY_PARA_ID).into_versioned()),
            Box::new(Junction::AccountId32 { id: BOB, network: None }.into_versioned()),
            Box::new((Here, DOT.one()).into()),
            0
        ));
    });

    Interlay::execute_with(|| {
        let bob = Tokens::free_balance(Token(DOT), &AccountId::from(BOB));
        assert!(bob > 0);
        assert!(bob < DOT.one());

        // Coomplete the migration
        assert_ok!(XTokens::set_migration_phase(
            RuntimeOrigin::root(),
            MigrationPhase::Completed
        ));
    });

    // After the migration, reserve transfer from Polkadot doesn't work as the reserve is now AH.
    PolkadotNet::execute_with(|| {
        assert_ok!(polkadot_runtime::XcmPallet::reserve_transfer_assets(
            polkadot_runtime::RuntimeOrigin::signed(ALICE.into()),
            Box::new(Parachain(INTERLAY_PARA_ID).into_versioned()),
            Box::new(Junction::AccountId32 { id: BOB, network: None }.into_versioned()),
            Box::new((Here, DOT.one()).into()),
            0
        ));
    });

    Interlay::execute_with(|| {
        let bob = Tokens::free_balance(Token(DOT), &AccountId::from(BOB));
        assert!(bob > 0);
        assert!(bob < DOT.one());
    })
}

#[test]
fn ahm_transfer_dot_from_interlay_to_polkadot() {
    TestNet::reset();
    PolkadotNet::execute_with(|| {
        assert_ok!(polkadot_runtime::Balances::force_set_balance(
            polkadot_runtime::RuntimeOrigin::root(),
            sp_runtime::MultiAddress::Id(interlay_sovereign_account_on_polkadot()),
            10 * DOT.one()
        ));
    });
    AssetHub::execute_with(|| {
        assert_ok!(polkadot_asset_hub_runtime::Balances::force_set_balance(
            polkadot_asset_hub_runtime::RuntimeOrigin::root(),
            sp_runtime::MultiAddress::Id(sovereign_account_on_sibling(INTERLAY_PARA_ID)),
            10 * DOT.one()
        ));
    });

    // Before the migration -> Everything ok
    Interlay::execute_with(|| {
        assert_ok!(XTokens::transfer(
            RuntimeOrigin::signed(ALICE.into()),
            Token(DOT),
            2 * DOT.one(),
            Box::new(MultiLocation::new(1, X1(Junction::AccountId32 { id: BOB, network: None })).into()),
            WeightLimit::Unlimited
        ));
    });

    PolkadotNet::execute_with(|| {
        let bob = polkadot_runtime::Balances::free_balance(AccountId::from(BOB));
        assert!(bob > 0);
        assert!(bob < 2 * DOT.one());
    });

    // During the migration => Extrinsic blocked as expected
    Interlay::execute_with(|| {
        assert_ok!(XTokens::set_migration_phase(
            RuntimeOrigin::root(),
            MigrationPhase::InProgress
        ));

        assert_noop!(
            XTokens::transfer(
                RuntimeOrigin::signed(ALICE.into()),
                Token(DOT),
                DOT.one(),
                Box::new(
                    MultiLocation::new(
                        1,
                        X2(
                            Junction::Parachain(SIBLING_PARA_ID),
                            Junction::AccountId32 { id: BOB, network: None }
                        )
                    )
                    .into()
                ),
                WeightLimit::Unlimited
            ),
            orml_xtokens::Error::<Runtime>::AssetHasNoReserve
        );
    });

    // After the migration => Extrinsic succeed
    Interlay::execute_with(|| {
        assert_ok!(XTokens::set_migration_phase(
            RuntimeOrigin::root(),
            MigrationPhase::Completed
        ));

        assert_ok!(XTokens::transfer(
            RuntimeOrigin::signed(ALICE.into()),
            Token(DOT),
            2 * DOT.one(),
            Box::new(MultiLocation::new(1, X1(Junction::AccountId32 { id: BOB, network: None })).into()),
            WeightLimit::Unlimited
        ));
    });

    // **BUT** Polkadot doesn't receive the tokens perse. This is because Polkadot is teleporter
    // of DOT, so the reserve (AH) doesn't forward the reserve transfer message. Tokens can be found in AH tho
    PolkadotNet::execute_with(|| {
        let bob = polkadot_runtime::Balances::free_balance(AccountId::from(BOB));
        assert!(bob > 0);
        assert!(bob < 2 * DOT.one());
    });
    AssetHub::execute_with(|| {
        let bob = polkadot_asset_hub_runtime::Balances::free_balance(AccountId::from(BOB));
        assert!(bob > 0);
        assert!(bob < 2 * DOT.one());
    });
}

#[test]
fn ahm_transfer_dot_from_asset_hub_to_interlay() {
    TestNet::reset();

    // Pre migration -> The DOT reserve is Interlay, so AH isn't trusted
    AssetHub::execute_with(|| {
        assert_ok!(polkadot_asset_hub_runtime::PolkadotXcm::reserve_transfer_assets(
            polkadot_asset_hub_runtime::RuntimeOrigin::signed(ALICE.into()),
            Box::new(MultiLocation::new(1, Parachain(INTERLAY_PARA_ID)).into()),
            Box::new(Junction::AccountId32 { id: BOB, network: None }.into_versioned()),
            Box::new((MultiLocation::parent(), DOT.one()).into()),
            0
        ));
    });

    Interlay::execute_with(|| {
        let bob = Tokens::free_balance(Token(DOT), &AccountId::from(BOB));
        assert!(bob == 0);

        // Change the migration status
        assert_ok!(XTokens::set_migration_phase(
            RuntimeOrigin::root(),
            MigrationPhase::InProgress
        ));
    });

    // During the migration, the extrinsic's ok in PolkadotAssetHub but the balance doesn't get to BOB.
    // NOTE: This is Ok here because this Polkadot version's not aware of the migration. In
    // production, this extrinsic will be directly blocked on Polkadot. But this proves that
    // Interlay's reserve is deactivated.
    AssetHub::execute_with(|| {
        assert_ok!(polkadot_asset_hub_runtime::PolkadotXcm::reserve_transfer_assets(
            polkadot_asset_hub_runtime::RuntimeOrigin::signed(ALICE.into()),
            Box::new(MultiLocation::new(1, Parachain(INTERLAY_PARA_ID)).into()),
            Box::new(Junction::AccountId32 { id: BOB, network: None }.into_versioned()),
            Box::new((MultiLocation::parent(), DOT.one()).into()),
            0
        ));
    });

    Interlay::execute_with(|| {
        let bob = Tokens::free_balance(Token(DOT), &AccountId::from(BOB));
        assert!(bob == 0);

        // Coomplete the migration
        assert_ok!(XTokens::set_migration_phase(
            RuntimeOrigin::root(),
            MigrationPhase::Completed
        ));
    });

    // After the migration, reserve transfer works because AH is the right reserve.
    // **NOTE**: This shouldn't be called on production, AH will disallow this call, so it'll
    // result in paying fees for nothing. Use `transfer_assets_using_type_and_the` instead, we
    // didn't use it here cause it's not available on this AH version
    AssetHub::execute_with(|| {
        assert_ok!(polkadot_asset_hub_runtime::PolkadotXcm::reserve_transfer_assets(
            polkadot_asset_hub_runtime::RuntimeOrigin::signed(ALICE.into()),
            Box::new(MultiLocation::new(1, Parachain(INTERLAY_PARA_ID)).into()),
            Box::new(Junction::AccountId32 { id: BOB, network: None }.into_versioned()),
            Box::new((MultiLocation::parent(), DOT.one()).into()),
            0
        ));
    });

    Interlay::execute_with(|| {
        let bob = Tokens::free_balance(Token(DOT), &AccountId::from(BOB));
        assert!(bob > 0);
        assert!(bob < DOT.one());
    })
}

#[test]
fn ahm_transfer_dot_from_interlay_to_asset_hub() {
    TestNet::reset();
    PolkadotNet::execute_with(|| {
        assert_ok!(polkadot_runtime::Balances::force_set_balance(
            polkadot_runtime::RuntimeOrigin::root(),
            sp_runtime::MultiAddress::Id(interlay_sovereign_account_on_polkadot()),
            10 * DOT.one()
        ));
    });
    AssetHub::execute_with(|| {
        assert_ok!(polkadot_asset_hub_runtime::Balances::force_set_balance(
            polkadot_asset_hub_runtime::RuntimeOrigin::root(),
            sp_runtime::MultiAddress::Id(sovereign_account_on_sibling(INTERLAY_PARA_ID)),
            10 * DOT.one()
        ));
    });

    // Before the migration -> Everything ok
    Interlay::execute_with(|| {
        assert_ok!(XTokens::transfer(
            RuntimeOrigin::signed(ALICE.into()),
            Token(DOT),
            2 * DOT.one(),
            Box::new(
                MultiLocation::new(
                    1,
                    X2(
                        Junction::Parachain(POLKADOT_ASSET_HUB_PARA_ID),
                        Junction::AccountId32 { id: BOB, network: None }
                    )
                )
                .into()
            ),
            WeightLimit::Unlimited
        ));
    });

    // **BUT** AH doesn't receive the tokens perse. This is because Polkadot forwards the message
    // BUT isn't reserve of DOT for AH, so funds remains locked on AH child account. Note this is
    // different from the case when we reserve transfer DOT from Interlay to Polkadot with AH being
    // the reserve.
    AssetHub::execute_with(|| {
        let bob = polkadot_asset_hub_runtime::Balances::free_balance(AccountId::from(BOB));
        assert!(bob == 0);
    });

    PolkadotNet::execute_with(|| {
        let ah_sovereign_account = polkadot_runtime::Balances::free_balance(ah_sovereign_account_on_polkadot());
        assert!(ah_sovereign_account > 0);
        assert!(ah_sovereign_account < 2 * DOT.one());
    });

    // During the migration => Extrinsic blocked as expected
    Interlay::execute_with(|| {
        assert_ok!(XTokens::set_migration_phase(
            RuntimeOrigin::root(),
            MigrationPhase::InProgress
        ));

        assert_noop!(
            XTokens::transfer(
                RuntimeOrigin::signed(ALICE.into()),
                Token(DOT),
                DOT.one(),
                Box::new(
                    MultiLocation::new(
                        1,
                        X2(
                            Junction::Parachain(SIBLING_PARA_ID),
                            Junction::AccountId32 { id: BOB, network: None }
                        )
                    )
                    .into()
                ),
                WeightLimit::Unlimited
            ),
            orml_xtokens::Error::<Runtime>::AssetHasNoReserve
        );
    });

    // After the migration => OK
    Interlay::execute_with(|| {
        assert_ok!(XTokens::set_migration_phase(
            RuntimeOrigin::root(),
            MigrationPhase::Completed
        ));

        assert_ok!(XTokens::transfer(
            RuntimeOrigin::signed(ALICE.into()),
            Token(DOT),
            2 * DOT.one(),
            Box::new(MultiLocation::new(1, X1(Junction::AccountId32 { id: BOB, network: None })).into()),
            WeightLimit::Unlimited
        ));
    });

    AssetHub::execute_with(|| {
        let bob = polkadot_asset_hub_runtime::Balances::free_balance(AccountId::from(BOB));
        assert!(bob > 0);
        assert!(bob < 2 * DOT.one());
    });
}

// Other tokens aren't affected by the migration
#[test]
fn ahm_transfer_with_xtokens_not_dot() {
    TestNet::reset();
    Interlay::execute_with(|| {
        register_sibling_asset_as_foreign_asset();

        assert_ok!(Tokens::deposit(
            ForeignAsset(1),
            &AccountId::from(ALICE),
            100_000_000_000_000
        ));
    });

    Interlay::execute_with(|| {
        // Before the migration => transfer works. The reserve is the sibling
        assert_ok!(XTokens::transfer(
            RuntimeOrigin::signed(ALICE.into()),
            ForeignAsset(1),
            10_000_000_000_000,
            Box::new(
                MultiLocation::new(
                    1,
                    X2(
                        Junction::Parachain(SIBLING_PARA_ID),
                        Junction::AccountId32 {
                            network: None,
                            id: BOB.into(),
                        }
                    )
                )
                .into()
            ),
            WeightLimit::Unlimited,
        ));

        assert_eq!(
            AbsoluteReserveProviderMigrationPhase::<Runtime>::reserve(&concrete_fungible(MultiLocation::new(
                1,
                X1(Junction::Parachain(SIBLING_PARA_ID))
            ))),
            Some(MultiLocation::new(1, Junction::Parachain(SIBLING_PARA_ID)))
        );

        // During the migration => transfer works. The reserve is still the sibling
        assert_ok!(XTokens::set_migration_phase(
            RuntimeOrigin::root(),
            MigrationPhase::InProgress
        ));

        assert_eq!(
            AbsoluteReserveProviderMigrationPhase::<Runtime>::reserve(&concrete_fungible(MultiLocation::new(
                1,
                Junction::Parachain(SIBLING_PARA_ID)
            ))),
            Some(MultiLocation::new(1, Junction::Parachain(SIBLING_PARA_ID)))
        );

        assert_ok!(XTokens::transfer(
            RuntimeOrigin::signed(ALICE.into()),
            ForeignAsset(1),
            10_000_000_000_000,
            Box::new(
                MultiLocation::new(
                    1,
                    X2(
                        Junction::Parachain(SIBLING_PARA_ID),
                        Junction::AccountId32 {
                            network: None,
                            id: BOB.into(),
                        }
                    )
                )
                .into()
            ),
            WeightLimit::Unlimited,
        ));

        // After the migration => transfer works. The reserve is still the sibling
        assert_ok!(XTokens::set_migration_phase(
            RuntimeOrigin::root(),
            MigrationPhase::Completed
        ));

        assert_eq!(
            AbsoluteReserveProviderMigrationPhase::<Runtime>::reserve(&concrete_fungible(MultiLocation::new(
                1,
                Junction::Parachain(SIBLING_PARA_ID)
            ))),
            Some(MultiLocation::new(1, Junction::Parachain(SIBLING_PARA_ID)))
        );

        assert_ok!(XTokens::transfer(
            RuntimeOrigin::signed(ALICE.into()),
            ForeignAsset(1),
            10_000_000_000_000,
            Box::new(
                MultiLocation::new(
                    1,
                    X2(
                        Junction::Parachain(SIBLING_PARA_ID),
                        Junction::AccountId32 {
                            network: None,
                            id: BOB.into(),
                        }
                    )
                )
                .into()
            ),
            WeightLimit::Unlimited,
        ));
    });
}
