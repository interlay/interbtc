use crate::{relaychain::kusama_test_net::*, setup::*};
use frame_support::assert_ok;
use orml_traits::MultiCurrency;
use primitives::CurrencyId::Token;
use xcm_builder::ParentIsPreset;
use xcm_emulator::TestExt;
use xcm_executor::traits::Convert;

mod hrmp {
    use super::*;

    use polkadot_runtime_parachains::hrmp;
    fn construct_xcm(call: hrmp::Call<kusama_runtime::Runtime>) -> Xcm<()> {
        Xcm(vec![
            WithdrawAsset((Here, 410000000000).into()),
            BuyExecution {
                fees: (Here, 400000000000).into(),
                weight_limit: Unlimited,
            },
            Transact {
                require_weight_at_most: 10000000000,
                origin_type: OriginKind::Native,
                call: kusama_runtime::Call::Hrmp(call).encode().into(),
            },
            RefundSurplus,
            DepositAsset {
                assets: All.into(),
                max_assets: 1,
                beneficiary: Junction::AccountId32 {
                    id: BOB,
                    network: NetworkId::Any,
                }
                .into(),
            },
        ])
    }

    fn has_open_channel_requested_event(sender: u32, recipient: u32) -> bool {
        KusamaNet::execute_with(|| {
            kusama_runtime::System::events().iter().any(|r| {
                matches!(
                    r.event,
                    kusama_runtime::Event::Hrmp(hrmp::Event::OpenChannelRequested(
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
        KusamaNet::execute_with(|| {
            kusama_runtime::System::events().iter().any(|r| {
                matches!(
                    r.event,
                    kusama_runtime::Event::Hrmp(hrmp::Event::OpenChannelAccepted(
                        actual_sender,
                        actual_recipient
                    )) if actual_sender == sender.into() && actual_recipient == recipient.into()
                )
            })
        })
    }

    fn init_open_channel<T>(sender: u32, recipient: u32)
    where
        T: TestExt,
    {
        // do hrmp_init_open_channel
        assert!(!has_open_channel_requested_event(sender, recipient)); // just a sanity check
        T::execute_with(|| {
            let message = construct_xcm(hrmp::Call::<kusama_runtime::Runtime>::hrmp_init_open_channel {
                recipient: recipient.into(),
                proposed_max_capacity: 1000,
                proposed_max_message_size: 102400,
            });
            assert_ok!(pallet_xcm::Pallet::<kintsugi_runtime_parachain::Runtime>::send_xcm(
                Here, Parent, message
            ));
        });
        assert!(has_open_channel_requested_event(sender, recipient));
    }

    fn accept_open_channel<T>(sender: u32, recipient: u32)
    where
        T: TestExt,
    {
        // do hrmp_accept_open_channel
        assert!(!has_open_channel_accepted_event(sender, recipient)); // just a sanity check
        T::execute_with(|| {
            let message = construct_xcm(hrmp::Call::<kusama_runtime::Runtime>::hrmp_accept_open_channel {
                sender: sender.into(),
            });
            assert_ok!(pallet_xcm::Pallet::<kintsugi_runtime_parachain::Runtime>::send_xcm(
                Here, Parent, message
            ));
        });
        assert!(has_open_channel_accepted_event(sender, recipient));
    }
    #[test]
    fn open_hrmp_channel() {
        // setup sovereign account balances
        KusamaNet::execute_with(|| {
            assert_ok!(kusama_runtime::Balances::transfer(
                kusama_runtime::Origin::signed(ALICE.into()),
                sp_runtime::MultiAddress::Id(kintsugi_sovereign_account_on_kusama()),
                10_820_000_000_000
            ));
            assert_ok!(kusama_runtime::Balances::transfer(
                kusama_runtime::Origin::signed(ALICE.into()),
                sp_runtime::MultiAddress::Id(sibling_sovereign_account_on_kusama()),
                10_820_000_000_000
            ));
        });

        // open channel kintsugi -> sibling
        init_open_channel::<Kintsugi>(KINTSUGI_PARA_ID, SIBLING_PARA_ID);
        accept_open_channel::<Sibling>(KINTSUGI_PARA_ID, SIBLING_PARA_ID);

        // open channel sibling -> kintsugi
        init_open_channel::<Sibling>(SIBLING_PARA_ID, KINTSUGI_PARA_ID);
        accept_open_channel::<Kintsugi>(SIBLING_PARA_ID, KINTSUGI_PARA_ID);

        // check that Bob received left-over funds (from both Kintsugi and Sibling).
        // We expect slightly less than 4 * 0.41 KSM
        KusamaNet::execute_with(|| {
            assert_eq!(
                kusama_runtime::Balances::free_balance(&AccountId::from(BOB)),
                1_637_510_889_920
            );
        });
    }
}

#[test]
fn transfer_from_relay_chain() {
    KusamaNet::execute_with(|| {
        assert_ok!(kusama_runtime::XcmPallet::reserve_transfer_assets(
            kusama_runtime::Origin::signed(ALICE.into()),
            Box::new(Parachain(KINTSUGI_PARA_ID).into().into()),
            Box::new(
                Junction::AccountId32 {
                    id: BOB,
                    network: NetworkId::Any
                }
                .into()
                .into()
            ),
            Box::new((Here, KSM.one()).into()),
            0
        ));
    });

    Kintsugi::execute_with(|| {
        let xcm_fee = 128_000_000;
        assert_eq!(
            Tokens::free_balance(Token(KSM), &AccountId::from(BOB)),
            KSM.one() - xcm_fee
        );
        assert_eq!(Tokens::free_balance(Token(KSM), &TreasuryAccount::get()), xcm_fee);
    });
}

#[test]
fn transfer_to_relay_chain() {
    KusamaNet::execute_with(|| {
        assert_ok!(kusama_runtime::Balances::transfer(
            kusama_runtime::Origin::signed(ALICE.into()),
            sp_runtime::MultiAddress::Id(kintsugi_sovereign_account_on_kusama()),
            2 * KSM.one()
        ));
    });

    Kintsugi::execute_with(|| {
        assert_ok!(XTokens::transfer(
            Origin::signed(ALICE.into()),
            Token(KSM),
            KSM.one(),
            Box::new(
                MultiLocation::new(
                    1,
                    X1(Junction::AccountId32 {
                        id: BOB,
                        network: NetworkId::Any,
                    })
                )
                .into()
            ),
            4_000_000_000
        ));
    });

    KusamaNet::execute_with(|| {
        // xcm fee depends on the ExtrinsicBaseWeight. It's calculated as follows
        // ExtrinsicBaseWeight = 80_350 * WEIGHT_PER_NANOS = 80_350_000
        // PricePerBaseWeight = 1/10 cent = 10^12 / 30_000 / 10
        // fee = (weight/ExtrinsicBaseWeight) * PricePerBaseWeight = (4_000_000_000 / 80_350_000) * (10^12 / 300_00 /
        // 10) = 165940676.208 (theoretical)
        // .. But due to rounding is actually 165940672
        let xcm_fee = 165_940_672;
        assert_eq!(
            kusama_runtime::Balances::free_balance(&AccountId::from(BOB)),
            KSM.one() - xcm_fee
        );
    });
}

#[test]
fn transfer_to_sibling() {
    fn sibling_sovereign_account() -> AccountId {
        use sp_runtime::traits::AccountIdConversion;
        polkadot_parachain::primitives::Sibling::from(SIBLING_PARA_ID).into_account()
    }

    Kintsugi::execute_with(|| {
        assert_ok!(Tokens::deposit(
            Token(KINT),
            &AccountId::from(ALICE),
            100_000_000_000_000
        ));
    });

    Kintsugi::execute_with(|| {
        assert_ok!(XTokens::transfer(
            Origin::signed(ALICE.into()),
            Token(KINT),
            10_000_000_000_000,
            Box::new(
                MultiLocation::new(
                    1,
                    X2(
                        Parachain(SIBLING_PARA_ID),
                        Junction::AccountId32 {
                            network: NetworkId::Any,
                            id: BOB.into(),
                        }
                    )
                )
                .into()
            ),
            1_000_000_000,
        ));

        assert_eq!(
            Tokens::free_balance(Token(KINT), &AccountId::from(ALICE)),
            90_000_000_000_000
        );

        assert_eq!(
            Tokens::free_balance(Token(KINT), &sibling_sovereign_account()),
            10_000_000_000_000
        );
    });

    Sibling::execute_with(|| {
        assert_ok!(XTokens::transfer_multiasset(
            Origin::signed(BOB.into()),
            Box::new(
                MultiAsset {
                    id: Concrete(
                        MultiLocation::new(1, X2(Parachain(KINTSUGI_PARA_ID), GeneralKey(Token(KINT).encode()))).into()
                    ),
                    fun: Fungibility::Fungible(5_000_000_000_000),
                }
                .into()
            ),
            Box::new(
                MultiLocation::new(
                    1,
                    X2(
                        Parachain(KINTSUGI_PARA_ID),
                        Junction::AccountId32 {
                            network: NetworkId::Any,
                            id: ALICE.into(),
                        }
                    )
                )
                .into()
            ),
            1_000_000_000,
        ));
    });

    Kintsugi::execute_with(|| {
        let xcm_fee = 170666666;
        assert_eq!(
            Tokens::free_balance(Token(KINT), &AccountId::from(ALICE)),
            95_000_000_000_000 - xcm_fee
        );

        assert_eq!(Tokens::free_balance(Token(KINT), &TreasuryAccount::get()), xcm_fee);

        assert_eq!(
            Tokens::free_balance(Token(KINT), &sibling_sovereign_account()),
            5_000_000_000_000
        );
    });
}

#[test]
fn xcm_transfer_execution_barrier_trader_works() {
    let expect_weight_limit = 600_000_000;
    let weight_limit_too_low = 500_000_000;
    let unit_instruction_weight = 200_000_000;

    // relay-chain use normal account to send xcm, destination parachain can't pass Barrier check
    let message = Xcm(vec![
        ReserveAssetDeposited((Parent, 100).into()),
        BuyExecution {
            fees: (Parent, 100).into(),
            weight_limit: Unlimited,
        },
        DepositAsset {
            assets: All.into(),
            max_assets: 1,
            beneficiary: Here.into(),
        },
    ]);
    KusamaNet::execute_with(|| {
        // Kusama effectively disabled the `send` extrinsic in 0.9.19, so use send_xcm
        assert_ok!(pallet_xcm::Pallet::<kusama_runtime::Runtime>::send_xcm(
            X1(Junction::AccountId32 {
                network: NetworkId::Any,
                id: ALICE.into(),
            }),
            Parachain(KINTSUGI_PARA_ID).into(),
            message
        ));
    });
    Kintsugi::execute_with(|| {
        assert!(System::events().iter().any(|r| matches!(
            r.event,
            Event::DmpQueue(cumulus_pallet_dmp_queue::Event::ExecutedDownward(
                _,
                Outcome::Error(XcmError::Barrier)
            ))
        )));
    });

    // AllowTopLevelPaidExecutionFrom barrier test case:
    // para-chain use XcmExecutor `execute_xcm()` method to execute xcm.
    // if `weight_limit` in BuyExecution is less than `xcm_weight(max_weight)`, then Barrier can't pass.
    // other situation when `weight_limit` is `Unlimited` or large than `xcm_weight`, then it's ok.
    let message = Xcm::<kintsugi_runtime_parachain::Call>(vec![
        ReserveAssetDeposited((Parent, 100).into()),
        BuyExecution {
            fees: (Parent, 100).into(),
            weight_limit: Limited(weight_limit_too_low),
        },
        DepositAsset {
            assets: All.into(),
            max_assets: 1,
            beneficiary: Here.into(),
        },
    ]);
    Kintsugi::execute_with(|| {
        let r = XcmExecutor::<XcmConfig>::execute_xcm(Parent, message, expect_weight_limit);
        assert_eq!(r, Outcome::Error(XcmError::Barrier));
    });

    // trader inside BuyExecution have TooExpensive error if payment less than calculated weight amount.
    // the minimum of calculated weight amount(`FixedRateOfFungible<KsmPerSecond>`) is 96_000_000
    let message = Xcm::<kintsugi_runtime_parachain::Call>(vec![
        ReserveAssetDeposited((Parent, 95_999_999).into()),
        BuyExecution {
            fees: (Parent, 95_999_999).into(),
            weight_limit: Limited(expect_weight_limit),
        },
        DepositAsset {
            assets: All.into(),
            max_assets: 1,
            beneficiary: Here.into(),
        },
    ]);
    Kintsugi::execute_with(|| {
        let r = XcmExecutor::<XcmConfig>::execute_xcm(Parent, message, expect_weight_limit);
        assert_eq!(
            r,
            Outcome::Incomplete(expect_weight_limit - unit_instruction_weight, XcmError::TooExpensive)
        );
    });

    // all situation fulfilled, execute success
    let message = Xcm::<kintsugi_runtime_parachain::Call>(vec![
        ReserveAssetDeposited((Parent, 96_000_000).into()),
        BuyExecution {
            fees: (Parent, 96_000_000).into(),
            weight_limit: Limited(expect_weight_limit),
        },
        DepositAsset {
            assets: All.into(),
            max_assets: 1,
            beneficiary: Here.into(),
        },
    ]);
    Kintsugi::execute_with(|| {
        let r = XcmExecutor::<XcmConfig>::execute_xcm(Parent, message, expect_weight_limit);
        assert_eq!(r, Outcome::Complete(expect_weight_limit));
    });
}

#[test]
fn subscribe_version_notify_works() {
    // relay chain subscribe version notify of para chain
    KusamaNet::execute_with(|| {
        let r = pallet_xcm::Pallet::<kusama_runtime::Runtime>::force_subscribe_version_notify(
            kusama_runtime::Origin::root(),
            Box::new(Parachain(KINTSUGI_PARA_ID).into().into()),
        );
        assert_ok!(r);
    });
    KusamaNet::execute_with(|| {
        kusama_runtime::System::assert_has_event(kusama_runtime::Event::XcmPallet(
            pallet_xcm::Event::SupportedVersionChanged(
                MultiLocation {
                    parents: 0,
                    interior: X1(Parachain(KINTSUGI_PARA_ID)),
                },
                2,
            ),
        ));
    });

    // para chain subscribe version notify of relay chain
    Kintsugi::execute_with(|| {
        let r = pallet_xcm::Pallet::<kintsugi_runtime_parachain::Runtime>::force_subscribe_version_notify(
            Origin::root(),
            Box::new(Parent.into()),
        );
        assert_ok!(r);
    });
    Kintsugi::execute_with(|| {
        System::assert_has_event(kintsugi_runtime_parachain::Event::PolkadotXcm(
            pallet_xcm::Event::SupportedVersionChanged(
                MultiLocation {
                    parents: 1,
                    interior: Here,
                },
                2,
            ),
        ));
    });

    // para chain subscribe version notify of sibling chain
    Kintsugi::execute_with(|| {
        let r = pallet_xcm::Pallet::<kintsugi_runtime_parachain::Runtime>::force_subscribe_version_notify(
            Origin::root(),
            Box::new((Parent, Parachain(SIBLING_PARA_ID)).into()),
        );
        assert_ok!(r);
    });
    Kintsugi::execute_with(|| {
        assert!(kintsugi_runtime_parachain::System::events().iter().any(|r| matches!(
            r.event,
            kintsugi_runtime_parachain::Event::XcmpQueue(cumulus_pallet_xcmp_queue::Event::XcmpMessageSent(Some(_)))
        )));
    });
    Sibling::execute_with(|| {
        assert!(testnet_runtime_parachain::System::events().iter().any(|r| matches!(
            r.event,
            testnet_runtime_parachain::Event::XcmpQueue(cumulus_pallet_xcmp_queue::Event::XcmpMessageSent(Some(_)))
                | testnet_runtime_parachain::Event::XcmpQueue(cumulus_pallet_xcmp_queue::Event::Success(Some(_)))
        )));
    });
}

#[test]
fn trap_assets_works() {
    let mut kint_treasury_amount = 0;
    let (ksm_asset_amount, kint_asset_amount) = (KSM.one(), KINT.one());
    let trader_weight_to_treasury: u128 = 96_000_000;

    let parent_account: AccountId = ParentIsPreset::<AccountId>::convert(Parent.into()).unwrap();

    Kintsugi::execute_with(|| {
        assert_ok!(Tokens::deposit(Token(KSM), &parent_account, 100 * KSM.one()));
        assert_ok!(Tokens::deposit(Token(KINT), &parent_account, 100 * KINT.one()));

        kint_treasury_amount = Tokens::free_balance(Token(KINT), &TreasuryAccount::get());
    });

    let assets: MultiAsset = (Parent, ksm_asset_amount).into();
    // Withdraw kint and ksm on kintsugi but don't deposit it
    KusamaNet::execute_with(|| {
        let xcm = vec![
            WithdrawAsset(assets.clone().into()),
            BuyExecution {
                fees: assets,
                weight_limit: Limited(KSM.one() as u64),
            },
            WithdrawAsset(
                (
                    (
                        Parent,
                        X2(Parachain(KINTSUGI_PARA_ID), GeneralKey(Token(KINT).encode())),
                    ),
                    kint_asset_amount,
                )
                    .into(),
            ),
        ];
        assert_ok!(pallet_xcm::Pallet::<kusama_runtime::Runtime>::send_xcm(
            Here,
            Parachain(KINTSUGI_PARA_ID).into(),
            Xcm(xcm),
        ));
    });

    let mut trapped_assets: Option<MultiAssets> = None;
    // verify that the assets got trapped (i.e. didn't get burned)
    Kintsugi::execute_with(|| {
        assert!(System::events()
            .iter()
            .any(|r| matches!(r.event, Event::PolkadotXcm(pallet_xcm::Event::AssetsTrapped(_, _, _)))));

        let event = System::events()
            .iter()
            .find(|r| matches!(r.event, Event::PolkadotXcm(pallet_xcm::Event::AssetsTrapped(_, _, _))))
            .cloned()
            .unwrap();

        use std::convert::TryFrom;
        use xcm::VersionedMultiAssets;
        trapped_assets = match event.event {
            Event::PolkadotXcm(pallet_xcm::Event::AssetsTrapped(_, _, ticket)) => {
                Some(TryFrom::<VersionedMultiAssets>::try_from(ticket).unwrap())
            }
            _ => panic!("event not found"),
        };

        // unchanged treasury amounts
        assert_eq!(
            trader_weight_to_treasury,
            Tokens::free_balance(Token(KSM), &TreasuryAccount::get())
        );
        assert_eq!(
            kint_treasury_amount,
            Tokens::free_balance(Token(KINT), &TreasuryAccount::get())
        );
    });

    // Now reclaim trapped assets
    KusamaNet::execute_with(|| {
        let xcm = vec![
            ClaimAsset {
                assets: trapped_assets.unwrap(),
                ticket: Here.into(),
            },
            BuyExecution {
                fees: (
                    (
                        Parent,
                        X2(Parachain(KINTSUGI_PARA_ID), GeneralKey(Token(KINT).encode())),
                    ),
                    kint_asset_amount / 2,
                )
                    .into(),
                weight_limit: Limited(KSM.one() as u64),
            },
            DepositAsset {
                assets: All.into(),
                max_assets: 2,
                beneficiary: Junction::AccountId32 {
                    id: BOB,
                    network: NetworkId::Any,
                }
                .into(),
            },
        ];
        assert_ok!(pallet_xcm::Pallet::<kusama_runtime::Runtime>::send_xcm(
            Here,
            Parachain(KINTSUGI_PARA_ID).into(),
            Xcm(xcm),
        ));
    });

    // verify that assets were claimed successfully (deposited into Bob's account)
    Kintsugi::execute_with(|| {
        let kint_xcm_fee = 127_999_999;
        let ksm_xcm_fee = 96_000_000;
        assert_eq!(
            Tokens::free_balance(Token(KINT), &AccountId::from(BOB)),
            kint_asset_amount - kint_xcm_fee
        );
        assert_eq!(
            Tokens::free_balance(Token(KSM), &AccountId::from(BOB)),
            ksm_asset_amount - ksm_xcm_fee
        );
    });
}
