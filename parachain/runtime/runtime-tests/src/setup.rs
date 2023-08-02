pub use crate::utils::*;
use bitcoin::merkle::PartialTransactionProof;
pub use codec::Encode;
use frame_support::traits::GenesisBuild;
pub use frame_support::{assert_noop, assert_ok, traits::Currency, BoundedVec};
pub use frame_system::RawOrigin;
pub use orml_traits::{location::RelativeLocations, Change, GetByKey, MultiCurrency};
pub use pretty_assertions::assert_eq;
pub use sp_core::H160;
pub use sp_runtime::{
    traits::{AccountIdConversion, BadOrigin, BlakeTwo256, Convert, Dispatchable, Hash, One, Zero},
    AccountId32, DispatchError, DispatchResult, FixedPointNumber, MultiAddress, Perbill, Permill,
};
pub use xcm::latest::prelude::*;
pub use xcm_emulator::XcmExecutor;

#[cfg(not(feature = "with-interlay-runtime"))]
pub use kintsugi_imports::*;
#[cfg(not(feature = "with-interlay-runtime"))]
mod kintsugi_imports {
    pub use frame_support::{parameter_types, weights::Weight};
    pub use kintsugi_runtime_parachain::{xcm_config::*, *};
    pub use sp_runtime::{traits::AccountIdConversion, FixedPointNumber};

    pub const DEFAULT_COLLATERAL_CURRENCY: CurrencyId = Token(KSM);
    pub const DEFAULT_WRAPPED_CURRENCY: CurrencyId = Token(KBTC);
    pub const DEFAULT_NATIVE_CURRENCY: CurrencyId = Token(KINT);
    pub const DEFAULT_GRIEFING_CURRENCY: CurrencyId = DEFAULT_NATIVE_CURRENCY;
}

#[cfg(feature = "with-interlay-runtime")]
pub use interlay_imports::*;
#[cfg(feature = "with-interlay-runtime")]
mod interlay_imports {
    pub use frame_support::{parameter_types, weights::Weight};
    pub use interlay_runtime_parachain::{xcm_config::*, *};
    pub use sp_runtime::{traits::AccountIdConversion, FixedPointNumber};

    pub const DEFAULT_COLLATERAL_CURRENCY: CurrencyId = Token(DOT);
    pub const DEFAULT_WRAPPED_CURRENCY: CurrencyId = Token(IBTC);
    pub const DEFAULT_NATIVE_CURRENCY: CurrencyId = Token(INTR);
    pub const DEFAULT_GRIEFING_CURRENCY: CurrencyId = DEFAULT_NATIVE_CURRENCY;
}

pub fn dummy_tx() -> FullTransactionProof {
    FullTransactionProof {
        coinbase_proof: PartialTransactionProof {
            merkle_proof: Default::default(),
            transaction: Default::default(),
            tx_encoded_len: u32::MAX,
        },
        user_tx_proof: PartialTransactionProof {
            merkle_proof: Default::default(),
            transaction: Default::default(),
            tx_encoded_len: u32::MAX,
        },
    }
}

pub struct ExtBuilder {
    test_externalities: sp_io::TestExternalities,
}

impl ExtBuilder {
    pub fn build() -> Self {
        let mut storage = frame_system::GenesisConfig::default()
            .build_storage::<Runtime>()
            .unwrap();

        let balances = vec![
            (account_of(ALICE), INITIAL_BALANCE),
            (account_of(BOB), INITIAL_BALANCE),
            (account_of(CAROL), INITIAL_BALANCE),
            (account_of(DAVE), INITIAL_BALANCE),
            (account_of(EVE), INITIAL_BALANCE),
            (account_of(FRANK), INITIAL_BALANCE),
            (account_of(GRACE), INITIAL_BALANCE),
            (account_of(FAUCET), 1 << 60),
        ];

        let balances = balances
            .into_iter()
            .flat_map(|(account, balance)| {
                iter_collateral_currencies()
                    .filter(|c| !c.is_lend_token())
                    .chain(iter_native_currencies())
                    .unique()
                    .map(move |currency| (account.clone(), currency, balance))
            })
            .chain(iter_wrapped_currencies().map(move |currency| (account_of(FAUCET), currency, 1 << 60)))
            .collect();

        orml_tokens::GenesisConfig::<Runtime> { balances }
            .assimilate_storage(&mut storage)
            .unwrap();

        oracle::GenesisConfig::<Runtime> {
            authorized_oracles: vec![(account_of(BOB), BoundedVec::truncate_from(BOB.to_vec()))],
            max_delay: 3600000, // one hour
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        pallet_sudo::GenesisConfig::<Runtime> {
            key: Some(account_of(ALICE)),
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        btc_relay::GenesisConfig::<Runtime> {
            bitcoin_confirmations: CONFIRMATIONS,
            parachain_confirmations: CONFIRMATIONS,
            disable_difficulty_check: true,
            disable_inclusion_check: false,
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        vault_registry::GenesisConfig::<Runtime> {
            minimum_collateral_vault: vec![
                (Token(DOT), 0),
                (Token(KSM), 0),
                (ForeignAsset(1), 0),
                (Token(INTR), 0),
                (Token(KINT), 0),
                (LendToken(1), 0),
            ],
            punishment_delay: 8,
            system_collateral_ceiling: iter_currency_pairs().map(|pair| (pair, FUND_LIMIT_CEILING)).collect(),
            secure_collateral_threshold: iter_currency_pairs()
                .map(|pair| (pair, FixedU128::checked_from_rational(150, 100).unwrap()))
                .collect(),
            premium_redeem_threshold: iter_currency_pairs()
                .map(|pair| (pair, FixedU128::checked_from_rational(150, 100).unwrap()))
                .collect(),
            liquidation_collateral_threshold: iter_currency_pairs()
                .map(|pair| (pair, FixedU128::checked_from_rational(110, 100).unwrap()))
                .collect(),
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        issue::GenesisConfig::<Runtime> {
            issue_period: 10,
            issue_btc_dust_value: 2,
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        redeem::GenesisConfig::<Runtime> {
            redeem_transaction_size: 400,
            redeem_period: 10,
            redeem_btc_dust_value: 1,
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        replace::GenesisConfig::<Runtime> {
            replace_period: 10,
            replace_btc_dust_value: 2,
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        fee::GenesisConfig::<Runtime> {
            issue_fee: FixedU128::checked_from_rational(15, 10000).unwrap(), // 0.15%
            issue_griefing_collateral: FixedU128::checked_from_rational(5, 100000).unwrap(), // 0.005%
            redeem_fee: FixedU128::checked_from_rational(5, 1000).unwrap(),  // 0.5%
            premium_redeem_fee: FixedU128::checked_from_rational(5, 100).unwrap(), // 5%
            punishment_fee: FixedU128::checked_from_rational(1, 10).unwrap(), // 10%
            replace_griefing_collateral: FixedU128::checked_from_rational(1, 10).unwrap(), // 10%
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        pallet_collective::GenesisConfig::<Runtime, TechnicalCommitteeInstance> {
            members: vec![account_of(ALICE)],
            phantom: Default::default(),
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        supply::GenesisConfig::<Runtime> {
            initial_supply: token_distribution::INITIAL_ALLOCATION,
            start_height: YEARS * 5,
            inflation: FixedU128::checked_from_rational(2, 100).unwrap(), // 2%
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        GenesisBuild::<Runtime>::assimilate_storage(
            &loans::GenesisConfig {
                max_exchange_rate: Rate::from_inner(DEFAULT_MAX_EXCHANGE_RATE),
                min_exchange_rate: Rate::from_inner(DEFAULT_MIN_EXCHANGE_RATE),
            },
            &mut storage,
        )
        .unwrap();

        <pallet_xcm::GenesisConfig as GenesisBuild<Runtime>>::assimilate_storage(
            &pallet_xcm::GenesisConfig {
                safe_xcm_version: Some(2),
            },
            &mut storage,
        )
        .unwrap();

        Self {
            test_externalities: sp_io::TestExternalities::from(storage),
        }
    }

    /// do setup common to all integration tests, then execute the callback
    pub fn execute_with<R>(self, execute: impl FnOnce() -> R) -> R {
        self.execute_without_relay_init(|| {
            // initialize btc relay
            let _ = TransactionGenerator::new().with_confirmations(7).mine();

            assert_ok!(RuntimeCall::Oracle(OracleCall::insert_authorized_oracle {
                account_id: account_of(ALICE),
                name: BoundedVec::truncate_from(vec![])
            })
            .dispatch(root()));
            assert_ok!(RuntimeCall::Oracle(OracleCall::feed_values {
                values: vec![
                    (OracleKey::ExchangeRate(DEFAULT_COLLATERAL_CURRENCY), FixedU128::from(1)),
                    (OracleKey::ExchangeRate(DEFAULT_GRIEFING_CURRENCY), FixedU128::from(1)),
                    (OracleKey::FeeEstimation, FixedU128::from(3)),
                ]
            })
            .dispatch(origin_of(account_of(ALICE))));
            OraclePallet::begin_block(0);

            let ret = execute();
            VaultRegistryPallet::total_user_vault_collateral_integrity_check();
            VaultRegistryPallet::collateral_integrity_check();
            ret
        })
    }

    /// used for btc-relay test
    pub fn execute_without_relay_init<R>(mut self, execute: impl FnOnce() -> R) -> R {
        self.test_externalities.execute_with(|| {
            SystemPallet::set_block_number(1); // required to be able to dispatch functions
            SecurityPallet::set_active_block_number(1);

            assert_ok!(OraclePallet::_set_exchange_rate(
                DEFAULT_COLLATERAL_CURRENCY,
                FixedU128::one()
            ));
            set_default_thresholds();

            let ret = execute();
            VaultRegistryPallet::total_user_vault_collateral_integrity_check();
            ret
        })
    }
}
