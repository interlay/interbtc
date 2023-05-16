use crate::relaychain::kusama_test_net::*;
use codec::Encode;
use frame_support::assert_ok;
use orml_traits::MultiCurrency;
use primitives::{
    CurrencyId::{ForeignAsset, Token},
    CustomMetadata, TokenSymbol,
};
use sp_runtime::{FixedPointNumber, FixedU128};
use xcm::latest::{prelude::*, Weight};
use xcm_builder::ParentIsPreset;
use xcm_emulator::{TestExt, XcmExecutor};
use xcm_executor::traits::Convert;

mod fees {
    use super::*;

    // N * unit_weight * (weight/10^12) * token_per_second
    fn weight_calculation(instruction_count: u32, unit_weight: Weight, per_second: u128) -> u128 {
        let weight = unit_weight.saturating_mul(instruction_count as u64);
        let weight_ratio =
            FixedU128::saturating_from_rational(weight.ref_time() as u128, WEIGHT_REF_TIME_PER_SECOND as u128);
        weight_ratio.saturating_mul_int(per_second)
    }

    fn native_unit_cost(instruction_count: u32, per_second: u128) -> u128 {
        let unit_weight: Weight = kintsugi_runtime_parachain::xcm_config::UnitWeightCost::get();
        assert_eq!(unit_weight.ref_time(), 200_000_000);
        assert_eq!(unit_weight.proof_size(), 0);

        weight_calculation(instruction_count, unit_weight, per_second)
    }

    pub fn ksm_per_second_as_fee(instruction_count: u32) -> u128 {
        let ksm_per_second = kintsugi_runtime_parachain::xcm_config::ksm_per_second();

        // check ksm per second. It's by no means essential - it's just useful to be forced to check the
        // change after polkadot updates
        assert!(ksm_per_second < 210000000000);

        native_unit_cost(instruction_count, ksm_per_second)
    }

    pub fn kint_per_second_as_fee(instruction_count: u32) -> u128 {
        let kint_per_second = kintsugi_runtime_parachain::xcm_config::kint_per_second();

        native_unit_cost(instruction_count, kint_per_second)
    }
}

mod hrmp {
    use super::*;

    use polkadot_runtime_parachains::hrmp;
    fn construct_xcm(call: hrmp::Call<kusama_runtime::Runtime>) -> Xcm<()> {
        Xcm(vec![
            WithdrawAsset((Here, 410000000000u128).into()),
            BuyExecution {
                fees: (Here, 400000000000u128).into(),
                weight_limit: Unlimited,
            },
            Transact {
                require_weight_at_most: Weight::from_ref_time(10000000000),
                origin_kind: OriginKind::Native,
                call: kusama_runtime::RuntimeCall::Hrmp(call).encode().into(),
            },
            RefundSurplus,
            DepositAsset {
                assets: All.into(),
                beneficiary: Junction::AccountId32 { id: BOB, network: None }.into(),
            },
        ])
    }

    fn has_open_channel_requested_event(sender: u32, recipient: u32) -> bool {
        KusamaNet::execute_with(|| {
            kusama_runtime::System::events().iter().any(|r| {
                matches!(
                    r.event,
                    kusama_runtime::RuntimeEvent::Hrmp(hrmp::Event::OpenChannelRequested(
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
                    kusama_runtime::RuntimeEvent::Hrmp(hrmp::Event::OpenChannelAccepted(
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
                kusama_runtime::RuntimeOrigin::signed(ALICE.into()),
                sp_runtime::MultiAddress::Id(kintsugi_sovereign_account_on_kusama()),
                10_820_000_000_000
            ));
            assert_ok!(kusama_runtime::Balances::transfer(
                kusama_runtime::RuntimeOrigin::signed(ALICE.into()),
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
            let free_balance = kusama_runtime::Balances::free_balance(&AccountId::from(BOB));
            assert!(free_balance > 1_600_000_000_000 && free_balance < 1_640_000_000_000);
        });
    }
}

#[test]
fn test_transact_barrier() {
    let call = orml_tokens::Call::<kintsugi_runtime_parachain::Runtime>::transfer_all {
        dest: ALICE.into(),
        currency_id: Token(KSM),
        keep_alive: false,
    };
    let message = Xcm(vec![
        WithdrawAsset((Here, 410000000000u128).into()),
        BuyExecution {
            fees: (Here, 400000000000u128).into(),
            weight_limit: Unlimited,
        },
        Transact {
            require_weight_at_most: Weight::from_ref_time(10000000000),
            origin_kind: OriginKind::Native,
            call: kintsugi_runtime_parachain::RuntimeCall::Tokens(call).encode().into(),
        },
        RefundSurplus,
        DepositAsset {
            assets: All.into(),
            beneficiary: Junction::AccountId32 { id: BOB, network: None }.into(),
        },
    ]);

    KusamaNet::execute_with(|| {
        assert_ok!(pallet_xcm::Pallet::<kusama_runtime::Runtime>::send_xcm(
            Here,
            X1(Parachain(2092)),
            message
        ));
    });

    Kintsugi::execute_with(|| {
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
    KusamaNet::execute_with(|| {
        assert_ok!(kusama_runtime::XcmPallet::reserve_transfer_assets(
            kusama_runtime::RuntimeOrigin::signed(ALICE.into()),
            Box::new(Parachain(KINTSUGI_PARA_ID).into_versioned()),
            Box::new(Junction::AccountId32 { id: BOB, network: None }.into_versioned()),
            Box::new((Here, KSM.one()).into()),
            0
        ));
    });

    Kintsugi::execute_with(|| {
        let xcm_fee = KSM.one() - Tokens::free_balance(Token(KSM), &AccountId::from(BOB));

        assert!(xcm_fee < 1000000000); // fees are set to 1000000000 in ui - make sure it's enough
        assert!(xcm_fee > 0); // check that some fees are taken

        // check that fees go to treasury
        assert_eq!(Tokens::free_balance(Token(KSM), &TreasuryAccount::get()), xcm_fee);
    });
}

#[test]
fn transfer_to_relay_chain() {
    KusamaNet::execute_with(|| {
        assert_ok!(kusama_runtime::Balances::transfer(
            kusama_runtime::RuntimeOrigin::signed(ALICE.into()),
            sp_runtime::MultiAddress::Id(kintsugi_sovereign_account_on_kusama()),
            2 * KSM.one()
        ));
    });

    Kintsugi::execute_with(|| {
        assert_ok!(XTokens::transfer(
            RuntimeOrigin::signed(ALICE.into()),
            Token(KSM),
            KSM.one(),
            Box::new(MultiLocation::new(1, X1(Junction::AccountId32 { id: BOB, network: None })).into()),
            WeightLimit::Unlimited
        ));
    });

    KusamaNet::execute_with(|| {
        let fee = KSM.one() - kusama_runtime::Balances::free_balance(&AccountId::from(BOB));

        // UI uses 165940672 - make sure that that's an overestimation
        assert!(fee < 165940672);
    });
}

/// Send KINT to sibling. On the sibling, it will be registered as a foreign asset.
/// By also transferring it back, we test that the asset-registry has been properly
/// integrated.
#[test]
fn transfer_to_sibling_and_back() {
    fn sibling_sovereign_account() -> AccountId {
        use sp_runtime::traits::AccountIdConversion;
        polkadot_parachain::primitives::Sibling::from(SIBLING_PARA_ID).into_account_truncating()
    }

    Sibling::execute_with(|| {
        register_kint_as_foreign_asset();
    });

    Kintsugi::execute_with(|| {
        assert_ok!(Tokens::deposit(
            Token(KINT),
            &AccountId::from(ALICE),
            100_000_000_000_000
        ));
    });

    Kintsugi::execute_with(|| {
        assert_ok!(XTokens::transfer(
            RuntimeOrigin::signed(ALICE.into()),
            Token(KINT),
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
            Tokens::free_balance(Token(KINT), &AccountId::from(ALICE)),
            90_000_000_000_000
        );

        assert_eq!(
            Tokens::free_balance(Token(KINT), &sibling_sovereign_account()),
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

        // return some back to kintsugi
        assert_ok!(XTokens::transfer(
            RuntimeOrigin::signed(BOB.into()),
            ForeignAsset(1),
            5_000_000_000_000,
            Box::new(
                MultiLocation::new(
                    1,
                    X2(
                        Parachain(KINTSUGI_PARA_ID),
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
    Kintsugi::execute_with(|| {
        let xcm_fee = fees::kint_per_second_as_fee(4);
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
    let unit_instruction_weight: Weight = kintsugi_runtime_parachain::xcm_config::UnitWeightCost::get();
    let message_weight = unit_instruction_weight.saturating_mul(3);
    let xcm_fee = fees::ksm_per_second_as_fee(3);

    // relay-chain use normal account to send xcm, destination parachain can't pass Barrier check
    let message = Xcm(vec![
        ReserveAssetDeposited((Parent, 100).into()),
        BuyExecution {
            fees: (Parent, 100).into(),
            weight_limit: Unlimited,
        },
        DepositAsset {
            assets: All.into(),
            beneficiary: Here.into(),
        },
    ]);
    KusamaNet::execute_with(|| {
        // Kusama effectively disabled the `send` extrinsic in 0.9.19, so use send_xcm
        assert_ok!(pallet_xcm::Pallet::<kusama_runtime::Runtime>::send_xcm(
            X1(Junction::AccountId32 {
                network: None,
                id: ALICE.into(),
            }),
            Parachain(KINTSUGI_PARA_ID),
            message
        ));
    });
    Kintsugi::execute_with(|| {
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
    let message = Xcm::<kintsugi_runtime_parachain::RuntimeCall>(vec![
        ReserveAssetDeposited((Parent, 100).into()),
        BuyExecution {
            fees: (Parent, 100).into(),
            weight_limit: Limited(message_weight - Weight::from_ref_time(1)),
        },
        DepositAsset {
            assets: All.into(),
            beneficiary: Here.into(),
        },
    ]);
    Kintsugi::execute_with(|| {
        let hash = message.using_encoded(sp_io::hashing::blake2_256);
        let r = XcmExecutor::<XcmConfig>::execute_xcm(Parent, message, hash, message_weight);
        assert_eq!(r, Outcome::Error(XcmError::Barrier));
    });

    // trader inside BuyExecution have TooExpensive error if payment less than calculated weight amount.
    // the minimum of calculated weight amount(`FixedRateOfFungible<KsmPerSecond>`).
    let message = Xcm::<kintsugi_runtime_parachain::RuntimeCall>(vec![
        ReserveAssetDeposited((Parent, xcm_fee - 1).into()),
        BuyExecution {
            fees: (Parent, xcm_fee - 1).into(),
            weight_limit: Limited(message_weight),
        },
        DepositAsset {
            assets: All.into(),
            beneficiary: Here.into(),
        },
    ]);
    Kintsugi::execute_with(|| {
        let hash = message.using_encoded(sp_io::hashing::blake2_256);
        let r = XcmExecutor::<XcmConfig>::execute_xcm(Parent, message, hash, message_weight);
        assert_eq!(
            r,
            Outcome::Incomplete(message_weight - unit_instruction_weight, XcmError::TooExpensive)
        );
    });

    // all situation fulfilled, execute success
    let message = Xcm::<kintsugi_runtime_parachain::RuntimeCall>(vec![
        ReserveAssetDeposited((Parent, xcm_fee).into()),
        BuyExecution {
            fees: (Parent, xcm_fee).into(),
            weight_limit: Limited(message_weight),
        },
        DepositAsset {
            assets: All.into(),
            beneficiary: Here.into(),
        },
    ]);
    Kintsugi::execute_with(|| {
        let hash = message.using_encoded(sp_io::hashing::blake2_256);
        let r = XcmExecutor::<XcmConfig>::execute_xcm(Parent, message, hash, message_weight);
        assert_eq!(r, Outcome::Complete(message_weight));
    });
}

#[test]
fn subscribe_version_notify_works() {
    // relay chain subscribe version notify of para chain
    KusamaNet::execute_with(|| {
        let r = pallet_xcm::Pallet::<kusama_runtime::Runtime>::force_subscribe_version_notify(
            kusama_runtime::RuntimeOrigin::root(),
            Box::new(Parachain(KINTSUGI_PARA_ID).into_versioned()),
        );
        assert_ok!(r);
    });
    KusamaNet::execute_with(|| {
        kusama_runtime::System::assert_has_event(kusama_runtime::RuntimeEvent::XcmPallet(
            pallet_xcm::Event::SupportedVersionChanged(
                MultiLocation {
                    parents: 0,
                    interior: X1(Parachain(KINTSUGI_PARA_ID)),
                },
                3,
            ),
        ));
    });

    // para chain subscribe version notify of relay chain
    Kintsugi::execute_with(|| {
        let r = pallet_xcm::Pallet::<kintsugi_runtime_parachain::Runtime>::force_subscribe_version_notify(
            RuntimeOrigin::root(),
            Box::new(Parent.into()),
        );
        assert_ok!(r);
    });
    Kintsugi::execute_with(|| {
        System::assert_has_event(kintsugi_runtime_parachain::RuntimeEvent::PolkadotXcm(
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
    Kintsugi::execute_with(|| {
        let r = pallet_xcm::Pallet::<kintsugi_runtime_parachain::Runtime>::force_subscribe_version_notify(
            RuntimeOrigin::root(),
            Box::new((Parent, Parachain(SIBLING_PARA_ID)).into()),
        );
        assert_ok!(r);
    });
    Kintsugi::execute_with(|| {
        assert!(kintsugi_runtime_parachain::System::events().iter().any(|r| matches!(
            r.event,
            kintsugi_runtime_parachain::RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::XcmpMessageSent {
                message_hash: Some(_)
            })
        )));
    });
    Sibling::execute_with(|| {
        assert!(testnet_kintsugi_runtime_parachain::System::events()
            .iter()
            .any(|r| matches!(
                r.event,
                testnet_kintsugi_runtime_parachain::RuntimeEvent::XcmpQueue(
                    cumulus_pallet_xcmp_queue::Event::XcmpMessageSent { message_hash: Some(_) }
                ) | testnet_kintsugi_runtime_parachain::RuntimeEvent::XcmpQueue(
                    cumulus_pallet_xcmp_queue::Event::Success {
                        message_hash: Some(_),
                        weight: _,
                    }
                )
            )));
    });
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
    let mut kint_treasury_amount = 0;
    let mut ksm_treasury_amount = 0;
    let (ksm_asset_amount, kint_asset_amount) = (KSM.one(), KINT.one());
    let trader_weight_to_treasury = fees::ksm_per_second_as_fee(3);

    let parent_account: AccountId = ParentIsPreset::<AccountId>::convert(Parent.into()).unwrap();

    Kintsugi::execute_with(|| {
        assert_ok!(Tokens::deposit(Token(KSM), &parent_account, 100 * KSM.one()));
        assert_ok!(Tokens::deposit(Token(KINT), &parent_account, 100 * KINT.one()));

        kint_treasury_amount = Tokens::free_balance(Token(KINT), &TreasuryAccount::get());
        ksm_treasury_amount = Tokens::free_balance(Token(KSM), &TreasuryAccount::get());
    });

    let assets: MultiAsset = (Parent, ksm_asset_amount).into();
    // Withdraw kint and ksm on kintsugi but don't deposit it
    KusamaNet::execute_with(|| {
        let xcm = vec![
            WithdrawAsset(assets.clone().into()),
            BuyExecution {
                fees: assets,
                weight_limit: Limited(Weight::from_ref_time(KSM.one() as u64)),
            },
            WithdrawAsset(
                (
                    (Parent, X2(Parachain(KINTSUGI_PARA_ID), general_key_of(KINT))),
                    kint_asset_amount,
                )
                    .into(),
            ),
        ];
        assert_ok!(pallet_xcm::Pallet::<kusama_runtime::Runtime>::send_xcm(
            Here,
            Parachain(KINTSUGI_PARA_ID),
            Xcm(xcm),
        ));
    });

    let mut trapped_assets: Option<MultiAssets> = None;
    // verify that the assets got trapped (i.e. didn't get burned)
    Kintsugi::execute_with(|| {
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
                    (Parent, X2(Parachain(KINTSUGI_PARA_ID), general_key_of(KINT))),
                    kint_asset_amount / 2,
                )
                    .into(),
                weight_limit: Limited(Weight::from_ref_time(KSM.one() as u64)),
            },
            DepositAsset {
                assets: All.into(),
                beneficiary: Junction::AccountId32 { id: BOB, network: None }.into(),
            },
        ];
        assert_ok!(pallet_xcm::Pallet::<kusama_runtime::Runtime>::send_xcm(
            Here,
            Parachain(KINTSUGI_PARA_ID),
            Xcm(xcm),
        ));
    });

    // verify that assets were claimed successfully (deposited into Bob's account)
    Kintsugi::execute_with(|| {
        let kint_xcm_fee = fees::kint_per_second_as_fee(3);
        let ksm_xcm_fee = fees::ksm_per_second_as_fee(3);
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

fn register_kint_as_foreign_asset() {
    let metadata = AssetMetadata {
        decimals: 12,
        name: "Kintsugi native".as_bytes().to_vec(),
        symbol: "extKINT".as_bytes().to_vec(),
        existential_deposit: 0,
        location: Some(MultiLocation::new(1, X2(Parachain(KINTSUGI_PARA_ID), general_key_of(KINT))).into()),
        additional: CustomMetadata {
            fee_per_second: 1_000_000_000_000,
            coingecko_id: "kint-sugi".as_bytes().to_vec(),
        },
    };
    AssetRegistry::register_asset(RuntimeOrigin::root(), metadata, None).unwrap();
}
