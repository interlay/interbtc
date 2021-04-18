#![allow(dead_code)]
extern crate hex;

pub use bitcoin::{
    formatter::{Formattable, TryFormattable},
    types::*,
};
pub use btc_parachain_runtime::{AccountId, Call, Event, Runtime};
pub use btc_relay::{BtcAddress, BtcPublicKey};
pub use frame_support::{assert_err, assert_noop, assert_ok, dispatch::DispatchResultWithPostInfo};
pub use mocktopus::mocking::*;
pub use primitive_types::{H256, U256};
pub use security::{ErrorCode, StatusCode};
pub use sp_arithmetic::{FixedI128, FixedPointNumber, FixedU128};
pub use sp_core::H160;
pub use sp_runtime::traits::Dispatchable;
pub use sp_std::convert::TryInto;
pub use vault_registry::CurrencySource;

pub use issue::IssueRequest;
pub use redeem::RedeemRequest;
pub use refund::RefundRequest;
pub use replace::ReplaceRequest;
pub use sp_runtime::AccountId32;
pub use std::convert::TryFrom;

pub mod issue_testing_utils;
pub mod redeem_testing_utils;

pub const ALICE: [u8; 32] = [0u8; 32];
pub const BOB: [u8; 32] = [1u8; 32];
pub const CAROL: [u8; 32] = [2u8; 32];
pub const DAVE: [u8; 32] = [10u8; 32];
pub const EVE: [u8; 32] = [11u8; 32];
pub const FRANK: [u8; 32] = [12u8; 32];
pub const GRACE: [u8; 32] = [13u8; 32];

pub const LIQUIDATION_VAULT: [u8; 32] = [3u8; 32];
pub const FEE_POOL: [u8; 32] = [4u8; 32];
pub const MAINTAINER: [u8; 32] = [5u8; 32];

pub const FAUCET: [u8; 32] = [128u8; 32];
pub const DUMMY: [u8; 32] = [255u8; 32];

pub const INITIAL_BALANCE: u128 = 1_000_000_000_000;
pub const INITIAL_LIQUIDATION_VAULT_BALANCE: u128 = 1_000;

pub const DEFAULT_USER_FREE_BALANCE: u128 = 1_000_000;
pub const DEFAULT_USER_LOCKED_BALANCE: u128 = 100_000;
pub const DEFAULT_USER_FREE_TOKENS: u128 = 10_000_000;
pub const DEFAULT_USER_LOCKED_TOKENS: u128 = 1000;

pub const DEFAULT_VAULT_TO_BE_ISSUED: u128 = 10_000;
pub const DEFAULT_VAULT_ISSUED: u128 = 100_000;
pub const DEFAULT_VAULT_TO_BE_REDEEMED: u128 = 20_000;
pub const DEFAULT_VAULT_BACKING_COLLATERAL: u128 = 1_000_000;
pub const DEFAULT_VAULT_GRIEFING_COLLATERAL: u128 = 30_000;
pub const DEFAULT_VAULT_FREE_BALANCE: u128 = 200_000;
pub const DEFAULT_VAULT_FREE_TOKENS: u128 = 0;
pub const DEFAULT_VAULT_REPLACE_COLLATERAL: u128 = 20_000;
pub const DEFAULT_VAULT_TO_BE_REPLACED: u128 = 40_000;

pub const CONFIRMATIONS: u32 = 6;

pub type BTCRelayCall = btc_relay::Call<Runtime>;
pub type BTCRelayPallet = btc_relay::Pallet<Runtime>;
pub type BTCRelayError = btc_relay::Error<Runtime>;
pub type BTCRelayEvent = btc_relay::Event<Runtime>;

pub type CollateralError = collateral::Error<Runtime>;
pub type CollateralPallet = collateral::Module<Runtime>;

pub type ExchangeRateOracleCall = exchange_rate_oracle::Call<Runtime>;
pub type ExchangeRateOraclePallet = exchange_rate_oracle::Module<Runtime>;

pub type FeeCall = fee::Call<Runtime>;
pub type FeeError = fee::Error<Runtime>;
pub type FeePallet = fee::Pallet<Runtime>;

pub type IssueCall = issue::Call<Runtime>;
pub type IssuePallet = issue::Pallet<Runtime>;
pub type IssueEvent = issue::Event<Runtime>;
pub type IssueError = issue::Error<Runtime>;

pub type RefundCall = refund::Call<Runtime>;
pub type RefundPallet = refund::Pallet<Runtime>;
pub type RefundEvent = refund::Event<Runtime>;

pub type RedeemCall = redeem::Call<Runtime>;
pub type RedeemPallet = redeem::Pallet<Runtime>;
pub type RedeemError = redeem::Error<Runtime>;
pub type RedeemEvent = redeem::Event<Runtime>;

pub type ReplaceCall = replace::Call<Runtime>;
pub type ReplaceEvent = replace::Event<Runtime>;
pub type ReplacePallet = replace::Pallet<Runtime>;

pub type SecurityError = security::Error<Runtime>;
pub type SecurityPallet = security::Pallet<Runtime>;

pub type SlaPallet = sla::Pallet<Runtime>;

pub type StakedRelayersCall = staked_relayers::Call<Runtime>;
pub type StakedRelayersPallet = staked_relayers::Pallet<Runtime>;

pub type SystemModule = frame_system::Pallet<Runtime>;

pub type TreasuryPallet = treasury::Pallet<Runtime>;

pub type VaultRegistryCall = vault_registry::Call<Runtime>;
pub type VaultRegistryError = vault_registry::Error<Runtime>;
pub type VaultRegistryPallet = vault_registry::Pallet<Runtime>;

pub fn default_user_state() -> UserData {
    UserData {
        free_balance: DEFAULT_USER_FREE_BALANCE,
        locked_balance: DEFAULT_USER_LOCKED_BALANCE,
        locked_tokens: DEFAULT_USER_LOCKED_TOKENS,
        free_tokens: DEFAULT_USER_FREE_TOKENS,
    }
}

pub fn default_vault_state() -> CoreVaultData {
    CoreVaultData {
        to_be_issued: DEFAULT_VAULT_TO_BE_ISSUED,
        issued: DEFAULT_VAULT_ISSUED,
        to_be_redeemed: DEFAULT_VAULT_TO_BE_REDEEMED,
        backing_collateral: DEFAULT_VAULT_BACKING_COLLATERAL,
        griefing_collateral: DEFAULT_VAULT_GRIEFING_COLLATERAL,
        free_balance: DEFAULT_VAULT_FREE_BALANCE,
        free_tokens: 0,
        replace_collateral: DEFAULT_VAULT_REPLACE_COLLATERAL,
        to_be_replaced: DEFAULT_VAULT_TO_BE_REPLACED,
    }
}

pub fn root() -> <Runtime as frame_system::Config>::Origin {
    <Runtime as frame_system::Config>::Origin::root()
}

pub fn origin_of(account_id: AccountId) -> <Runtime as frame_system::Config>::Origin {
    <Runtime as frame_system::Config>::Origin::signed(account_id)
}

pub fn account_of(address: [u8; 32]) -> AccountId {
    AccountId::from(address)
}

#[derive(Debug, PartialEq, Default, Clone)]
pub struct UserData {
    pub free_balance: u128,
    pub locked_balance: u128,
    pub locked_tokens: u128,
    pub free_tokens: u128,
}

impl UserData {
    #[allow(dead_code)]
    pub fn get(id: [u8; 32]) -> Self {
        let account_id = account_of(id);
        Self {
            free_balance: CollateralPallet::get_balance_from_account(&account_id),
            locked_balance: CollateralPallet::get_collateral_from_account(&account_id),
            locked_tokens: TreasuryPallet::get_locked_balance_from_account(account_id.clone()),
            free_tokens: TreasuryPallet::get_balance_from_account(account_id.clone()),
        }
    }
    #[allow(dead_code)]
    pub fn force_to(id: [u8; 32], new: Self) -> Self {
        let old = Self::get(id);
        let account_id = account_of(id);

        // set tokens to 0
        TreasuryPallet::lock(account_id.clone(), old.free_tokens).unwrap();
        TreasuryPallet::burn(account_id.clone(), old.free_tokens + old.locked_tokens).unwrap();

        // set free balance:
        CollateralPallet::transfer(account_id.clone(), account_of(FAUCET), old.free_balance).unwrap();
        CollateralPallet::transfer(account_of(FAUCET), account_id.clone(), new.free_balance).unwrap();

        // set locked balance:
        CollateralPallet::slash_collateral(account_id.clone(), account_of(FAUCET), old.locked_balance).unwrap();
        CollateralPallet::transfer(account_of(FAUCET), account_id.clone(), new.locked_balance).unwrap();
        CollateralPallet::lock_collateral(&account_id, new.locked_balance).unwrap();

        // set free_tokens
        TreasuryPallet::mint(account_id.clone(), new.free_tokens);

        // set locked_tokens
        TreasuryPallet::mint(account_id.clone(), new.locked_tokens);
        TreasuryPallet::lock(account_id, new.locked_tokens).unwrap();

        // sanity check:
        assert_eq!(Self::get(id), new);

        new
    }
}

#[derive(Debug, PartialEq, Default, Clone)]
pub struct FeePool {
    pub balance: u128,
    pub tokens: u128,
}

impl FeePool {
    pub fn get() -> Self {
        Self {
            balance: FeePallet::epoch_rewards_dot(),
            tokens: FeePallet::epoch_rewards_polka_btc(),
        }
    }
}

#[derive(Debug, PartialEq, Default, Clone)]
pub struct CoreVaultData {
    pub to_be_issued: u128,
    pub issued: u128,
    pub to_be_redeemed: u128,
    pub backing_collateral: u128,
    pub griefing_collateral: u128,
    pub free_balance: u128,
    pub free_tokens: u128,
    pub to_be_replaced: u128,
    pub replace_collateral: u128,
}

impl CoreVaultData {
    #[allow(dead_code)]
    pub fn vault(vault: [u8; 32]) -> Self {
        let account_id = account_of(vault);
        let vault = VaultRegistryPallet::get_vault_from_id(&account_id).unwrap();
        Self {
            to_be_issued: vault.to_be_issued_tokens,
            issued: vault.issued_tokens,
            to_be_redeemed: vault.to_be_redeemed_tokens,
            backing_collateral: CurrencySource::<Runtime>::Backing(account_id.clone())
                .current_balance()
                .unwrap(),
            griefing_collateral: CurrencySource::<Runtime>::Griefing(account_id.clone())
                .current_balance()
                .unwrap(),
            free_balance: CollateralPallet::get_balance_from_account(&account_id),
            free_tokens: TreasuryPallet::get_balance_from_account(account_id.clone()),
            to_be_replaced: vault.to_be_replaced_tokens,
            replace_collateral: vault.replace_collateral,
        }
    }
    #[allow(dead_code)]
    pub fn liquidation_vault() -> Self {
        let account_id = account_of(LIQUIDATION_VAULT);
        let vault = VaultRegistryPallet::get_liquidation_vault();
        Self {
            to_be_issued: vault.to_be_issued_tokens,
            issued: vault.issued_tokens,
            to_be_redeemed: vault.to_be_redeemed_tokens,
            backing_collateral: CurrencySource::<Runtime>::LiquidationVault.current_balance().unwrap(),
            griefing_collateral: 0,
            free_balance: CollateralPallet::get_balance_from_account(&account_id),
            free_tokens: TreasuryPallet::get_balance_from_account(account_id.clone()),
            to_be_replaced: 0,
            replace_collateral: 0,
        }
    }

    #[allow(dead_code)]
    pub fn force_to(vault: [u8; 32], state: CoreVaultData) {
        // replace collateral is part of griefing collateral, so it needs to smaller or equal
        assert!(state.griefing_collateral >= state.replace_collateral);
        assert!(state.to_be_replaced + state.to_be_redeemed <= state.issued);

        // register vault if not yet registered
        try_register_vault(100, vault);

        // temporarily give vault a lot of backing collateral so we can set issued & to-be-issued to whatever we want
        VaultRegistryPallet::slash_collateral(
            CurrencySource::FreeBalance(account_of(FAUCET)),
            CurrencySource::Backing(account_of(vault)),
            CollateralPallet::get_balance_from_account(&account_of(FAUCET)),
        )
        .unwrap();

        let current = CoreVaultData::vault(vault);

        // set all token types to 0
        assert_ok!(VaultRegistryPallet::decrease_to_be_issued_tokens(
            &account_of(vault),
            current.to_be_issued
        ));
        assert_ok!(VaultRegistryPallet::decrease_to_be_redeemed_tokens(
            &account_of(vault),
            current.to_be_redeemed
        ));
        assert_ok!(VaultRegistryPallet::try_increase_to_be_redeemed_tokens(
            &account_of(vault),
            current.issued
        ));
        assert_ok!(VaultRegistryPallet::decrease_tokens(
            &account_of(vault),
            &account_of(DUMMY),
            current.issued,
        ));
        assert_ok!(VaultRegistryPallet::decrease_to_be_replaced_tokens(
            &account_of(vault),
            current.to_be_replaced,
        ));
        assert_ok!(TreasuryPallet::lock(account_of(vault), current.free_tokens));
        assert_ok!(TreasuryPallet::burn(account_of(vault), current.free_tokens));

        // set to-be-issued
        assert_ok!(VaultRegistryPallet::try_increase_to_be_issued_tokens(
            &account_of(vault),
            state.to_be_issued
        ));
        // set issued (2 steps)
        assert_ok!(VaultRegistryPallet::try_increase_to_be_issued_tokens(
            &account_of(vault),
            state.issued
        ));
        assert_ok!(VaultRegistryPallet::issue_tokens(&account_of(vault), state.issued));
        // set to-be-redeemed
        assert_ok!(VaultRegistryPallet::try_increase_to_be_redeemed_tokens(
            &account_of(vault),
            state.to_be_redeemed
        ));
        // set to-be-replaced:
        assert_ok!(VaultRegistryPallet::try_increase_to_be_replaced_tokens(
            &account_of(vault),
            state.to_be_replaced,
            state.replace_collateral
        ));

        // set free tokens:
        TreasuryPallet::mint(account_of(vault), state.free_tokens);

        // clear all balances
        VaultRegistryPallet::slash_collateral(
            CurrencySource::Backing(account_of(vault)),
            CurrencySource::FreeBalance(account_of(FAUCET)),
            CurrencySource::<Runtime>::Backing(account_of(vault))
                .current_balance()
                .unwrap(),
        )
        .unwrap();
        VaultRegistryPallet::slash_collateral(
            CurrencySource::Griefing(account_of(vault)),
            CurrencySource::FreeBalance(account_of(FAUCET)),
            CurrencySource::<Runtime>::Griefing(account_of(vault))
                .current_balance()
                .unwrap(),
        )
        .unwrap();
        VaultRegistryPallet::slash_collateral(
            CurrencySource::FreeBalance(account_of(vault)),
            CurrencySource::FreeBalance(account_of(FAUCET)),
            CurrencySource::<Runtime>::FreeBalance(account_of(vault))
                .current_balance()
                .unwrap(),
        )
        .unwrap();

        // now set balances to desired values
        VaultRegistryPallet::slash_collateral(
            CurrencySource::FreeBalance(account_of(FAUCET)),
            CurrencySource::Backing(account_of(vault)),
            state.backing_collateral,
        )
        .unwrap();
        VaultRegistryPallet::slash_collateral(
            CurrencySource::FreeBalance(account_of(FAUCET)),
            CurrencySource::Griefing(account_of(vault)),
            state.griefing_collateral,
        )
        .unwrap();
        VaultRegistryPallet::slash_collateral(
            CurrencySource::FreeBalance(account_of(FAUCET)),
            CurrencySource::FreeBalance(account_of(vault)),
            state.free_balance,
        )
        .unwrap();

        // check that we achieved the desired state
        assert_eq!(CoreVaultData::vault(vault), state);
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ParachainState {
    user: UserData,
    vault: CoreVaultData,
    liquidation_vault: CoreVaultData,
    fee_pool: FeePool,
}

impl Default for ParachainState {
    fn default() -> Self {
        Self {
            user: default_user_state(),
            vault: default_vault_state(),
            liquidation_vault: CoreVaultData {
                free_balance: INITIAL_LIQUIDATION_VAULT_BALANCE,
                ..Default::default()
            },
            fee_pool: Default::default(),
        }
    }
}

impl ParachainState {
    pub fn get() -> Self {
        Self {
            user: UserData::get(ALICE),
            vault: CoreVaultData::vault(BOB),
            liquidation_vault: CoreVaultData::liquidation_vault(),
            fee_pool: FeePool::get(),
        }
    }

    pub fn with_changes(
        &self,
        f: impl FnOnce(&mut UserData, &mut CoreVaultData, &mut CoreVaultData, &mut FeePool),
    ) -> Self {
        let mut state = self.clone();
        f(
            &mut state.user,
            &mut state.vault,
            &mut state.liquidation_vault,
            &mut state.fee_pool,
        );
        state
    }
}

// todo: merge with ParachainState
#[derive(Debug, PartialEq, Clone)]
pub struct ParachainTwoVaultState {
    vault1: CoreVaultData,
    vault2: CoreVaultData,
    liquidation_vault: CoreVaultData,
}

impl Default for ParachainTwoVaultState {
    fn default() -> Self {
        Self {
            vault1: default_vault_state(),
            vault2: default_vault_state(),
            liquidation_vault: CoreVaultData {
                free_balance: INITIAL_LIQUIDATION_VAULT_BALANCE,
                ..Default::default()
            },
        }
    }
}

impl ParachainTwoVaultState {
    pub fn get() -> Self {
        Self {
            vault1: CoreVaultData::vault(BOB),
            vault2: CoreVaultData::vault(CAROL),
            liquidation_vault: CoreVaultData::liquidation_vault(),
        }
    }

    pub fn with_changes(&self, f: impl FnOnce(&mut CoreVaultData, &mut CoreVaultData, &mut CoreVaultData)) -> Self {
        let mut state = self.clone();
        f(&mut state.vault1, &mut state.vault2, &mut state.liquidation_vault);
        state
    }
}
#[allow(dead_code)]
pub fn drop_exchange_rate_and_liquidate(vault: [u8; 32]) {
    assert_ok!(ExchangeRateOraclePallet::_set_exchange_rate(
        FixedU128::checked_from_integer(10_000_000_000).unwrap()
    ));
    assert_ok!(VaultRegistryPallet::liquidate_vault(&account_of(vault)));
}

#[allow(dead_code)]
pub fn set_default_thresholds() {
    let secure = FixedU128::checked_from_rational(150, 100).unwrap();
    let auction = FixedU128::checked_from_rational(120, 100).unwrap();
    let premium = FixedU128::checked_from_rational(135, 100).unwrap();
    let liquidation = FixedU128::checked_from_rational(110, 100).unwrap();

    VaultRegistryPallet::set_secure_collateral_threshold(secure);
    VaultRegistryPallet::set_auction_collateral_threshold(auction);
    VaultRegistryPallet::set_premium_redeem_threshold(premium);
    VaultRegistryPallet::set_liquidation_collateral_threshold(liquidation);
}

pub fn dummy_public_key() -> BtcPublicKey {
    BtcPublicKey([
        2, 205, 114, 218, 156, 16, 235, 172, 106, 37, 18, 153, 202, 140, 176, 91, 207, 51, 187, 55, 18, 45, 222, 180,
        119, 54, 243, 97, 173, 150, 161, 169, 230,
    ])
}

#[allow(dead_code)]
pub fn try_register_vault(collateral: u128, vault: [u8; 32]) {
    if VaultRegistryPallet::get_vault_from_id(&account_of(vault)).is_err() {
        assert_ok!(
            Call::VaultRegistry(VaultRegistryCall::register_vault(collateral, dummy_public_key()))
                .dispatch(origin_of(account_of(vault)))
        );
    };
}

#[allow(dead_code)]
pub fn force_issue_tokens(user: [u8; 32], vault: [u8; 32], collateral: u128, tokens: u128) {
    // register the vault
    assert_ok!(
        Call::VaultRegistry(VaultRegistryCall::register_vault(collateral, dummy_public_key()))
            .dispatch(origin_of(account_of(vault)))
    );

    // increase to be issued tokens
    assert_ok!(VaultRegistryPallet::try_increase_to_be_issued_tokens(
        &account_of(vault),
        tokens
    ));

    // issue tokens
    assert_ok!(VaultRegistryPallet::issue_tokens(&account_of(vault), tokens));

    // mint tokens to the user
    treasury::Pallet::<Runtime>::mint(user.into(), tokens);
}

#[allow(dead_code)]
pub fn required_collateral_for_issue(issue_btc: u128) -> u128 {
    let fee_amount_btc = FeePallet::get_issue_fee(issue_btc).unwrap();
    let total_amount_btc = issue_btc + fee_amount_btc;
    VaultRegistryPallet::get_required_collateral_for_polkabtc(total_amount_btc).unwrap()
}

pub fn assert_store_main_chain_header_event(height: u32, hash: H256Le, relayer: AccountId) {
    let store_event = Event::btc_relay(BTCRelayEvent::StoreMainChainHeader(height, hash, relayer));
    let events = SystemModule::events();

    // store only main chain header
    assert!(events.iter().any(|a| a.event == store_event));
}

#[derive(Default, Clone, Debug)]
pub struct TransactionGenerator {
    address: BtcAddress,
    amount: u128,
    return_data: Option<H256>,
    script: Vec<u8>,
    confirmations: u32,
    relayer: Option<[u8; 32]>,
}

impl TransactionGenerator {
    pub fn new() -> Self {
        Self {
            relayer: None,
            confirmations: 7,
            amount: 100,
            script: vec![
                0, 71, 48, 68, 2, 32, 91, 128, 41, 150, 96, 53, 187, 63, 230, 129, 53, 234, 210, 186, 21, 187, 98, 38,
                255, 112, 30, 27, 228, 29, 132, 140, 155, 62, 123, 216, 232, 168, 2, 32, 72, 126, 179, 207, 142, 8, 99,
                8, 32, 78, 244, 166, 106, 160, 207, 227, 61, 210, 172, 234, 234, 93, 59, 159, 79, 12, 194, 240, 212, 3,
                120, 50, 1, 71, 81, 33, 3, 113, 209, 131, 177, 9, 29, 242, 229, 15, 217, 247, 165, 78, 111, 80, 79, 50,
                200, 117, 80, 30, 233, 210, 167, 133, 175, 62, 253, 134, 127, 212, 51, 33, 2, 128, 200, 184, 235, 148,
                25, 43, 34, 28, 173, 55, 54, 189, 164, 187, 243, 243, 152, 7, 84, 210, 85, 156, 238, 77, 97, 188, 240,
                162, 197, 105, 62, 82, 174,
            ],
            return_data: Some(H256::zero()),
            ..Default::default()
        }
    }
    pub fn with_address(&mut self, address: BtcAddress) -> &mut Self {
        self.address = address;
        self
    }

    pub fn with_amount(&mut self, amount: u128) -> &mut Self {
        self.amount = amount;
        self
    }

    pub fn with_op_return(&mut self, op_return: Option<H256>) -> &mut Self {
        self.return_data = op_return;
        self
    }
    pub fn with_script(&mut self, script: &[u8]) -> &mut Self {
        self.script = script.to_vec();
        self
    }
    pub fn with_confirmations(&mut self, confirmations: u32) -> &mut Self {
        self.confirmations = confirmations;
        self
    }
    pub fn with_relayer(&mut self, relayer: Option<[u8; 32]>) -> &mut Self {
        self.relayer = relayer;
        self
    }
    pub fn mine(&self) -> (H256Le, u32, Vec<u8>, Vec<u8>, Transaction) {
        let mut height = 1;
        let extra_confirmations = self.confirmations - 1;

        // initialize BTC Relay with one block
        let init_block = BlockBuilder::new()
            .with_version(2)
            .with_coinbase(&self.address, 50, 3)
            .with_timestamp(1588813835)
            .mine(U256::from(2).pow(254.into()))
            .unwrap();

        let raw_init_block_header = RawBlockHeader::from_bytes(&init_block.header.try_format().unwrap())
            .expect("could not serialize block header");

        match BTCRelayPallet::initialize(account_of(ALICE), raw_init_block_header, height) {
            Ok(_) => {}
            Err(e) if e == BTCRelayError::AlreadyInitialized.into() => {}
            _ => panic!("Failed to initialize btc relay"),
        }

        height = BTCRelayPallet::get_best_block_height() + 1;

        let value = self.amount as i64;
        let mut transaction_builder = TransactionBuilder::new();
        transaction_builder.with_version(2);
        transaction_builder.add_input(
            TransactionInputBuilder::new()
                .with_coinbase(false)
                .with_script(&self.script)
                .with_previous_hash(init_block.transactions[0].hash())
                .build(),
        );

        transaction_builder.add_output(TransactionOutput::payment(value, &self.address));
        if let Some(op_return_data) = self.return_data {
            transaction_builder.add_output(TransactionOutput::op_return(0, op_return_data.as_bytes()));
        }

        let transaction = transaction_builder.build();

        let prev_hash = BTCRelayPallet::get_best_block();
        let block = BlockBuilder::new()
            .with_previous_hash(prev_hash)
            .with_version(2)
            .with_coinbase(&self.address, 50, 3)
            .with_timestamp(1588814835)
            .add_transaction(transaction.clone())
            .mine(U256::from(2).pow(254.into()))
            .unwrap();

        let raw_block_header =
            RawBlockHeader::from_bytes(&block.header.try_format().unwrap()).expect("could not serialize block header");

        let tx_id = transaction.tx_id();
        let tx_block_height = height;
        let proof = block.merkle_proof(&[tx_id]).unwrap();
        let bytes_proof = proof.try_format().unwrap();
        let raw_tx = transaction.format_with(true);

        // let _ = Call::StakedRelayers(StakedRelayersCall::register_staked_relayer(
        //     100,
        // ))
        // .dispatch(origin_of(account_of(self.relayer)));
        self.relay(height, &block, raw_block_header);

        // Mine six new blocks to get over required confirmations
        let mut prev_block_hash = block.header.hash().unwrap();
        let mut timestamp = 1588814835;
        for _ in 0..extra_confirmations {
            height += 1;
            timestamp += 1000;
            let conf_block = BlockBuilder::new()
                .with_previous_hash(prev_block_hash)
                .with_version(2)
                .with_coinbase(&self.address, 50, 3)
                .with_timestamp(timestamp)
                .mine(U256::from(2).pow(254.into()))
                .unwrap();

            let raw_conf_block_header = RawBlockHeader::from_bytes(&conf_block.header.try_format().unwrap())
                .expect("could not serialize block header");
            self.relay(height, &conf_block, raw_conf_block_header);

            prev_block_hash = conf_block.header.hash().unwrap();
        }

        (tx_id, tx_block_height, bytes_proof, raw_tx, transaction)
    }

    fn relay(&self, height: u32, block: &Block, raw_block_header: RawBlockHeader) {
        if let Some(relayer) = self.relayer {
            let _ = Call::StakedRelayers(StakedRelayersCall::register_staked_relayer(100))
                .dispatch(origin_of(account_of(relayer)));

            assert_ok!(
                Call::StakedRelayers(StakedRelayersCall::store_block_header(raw_block_header))
                    .dispatch(origin_of(account_of(relayer)))
            );
            assert_store_main_chain_header_event(height, block.header.hash().unwrap(), account_of(relayer));
        } else {
            // bypass staked relayer module
            assert_ok!(BTCRelayPallet::store_block_header(&account_of(ALICE), raw_block_header));
            assert_store_main_chain_header_event(height, block.header.hash().unwrap(), account_of(ALICE));
        }
    }
}

#[allow(dead_code)]
pub fn generate_transaction_and_mine(
    address: BtcAddress,
    amount: u128,
    return_data: Option<H256>,
) -> (H256Le, u32, Vec<u8>, Vec<u8>) {
    let (tx_id, height, proof, raw_tx, _) = TransactionGenerator::new()
        .with_address(address)
        .with_amount(amount)
        .with_op_return(return_data)
        .mine();
    (tx_id, height, proof, raw_tx)
}

pub struct ExtBuilder {
    test_externalities: sp_io::TestExternalities,
}

impl ExtBuilder {
    pub fn build() -> Self {
        let mut storage = frame_system::GenesisConfig::default()
            .build_storage::<Runtime>()
            .unwrap();

        pallet_balances::GenesisConfig::<Runtime, pallet_balances::Instance1> {
            balances: vec![
                (account_of(ALICE), INITIAL_BALANCE),
                (account_of(BOB), INITIAL_BALANCE),
                (account_of(CAROL), INITIAL_BALANCE),
                (account_of(DAVE), INITIAL_BALANCE),
                (account_of(EVE), INITIAL_BALANCE),
                (account_of(FRANK), INITIAL_BALANCE),
                (account_of(GRACE), INITIAL_BALANCE),
                (account_of(FAUCET), 1 << 60),
                // create accounts for vault & fee pool; this needs a minimum amount because
                // the parachain refuses to create accounts with a balance below `ExistentialDeposit`
                (account_of(LIQUIDATION_VAULT), INITIAL_LIQUIDATION_VAULT_BALANCE),
                (account_of(FEE_POOL), 1000),
            ],
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        pallet_balances::GenesisConfig::<Runtime, pallet_balances::Instance2> { balances: vec![] }
            .assimilate_storage(&mut storage)
            .unwrap();

        exchange_rate_oracle::GenesisConfig::<Runtime> {
            authorized_oracles: vec![(account_of(BOB), BOB.to_vec())],
            max_delay: 3600000, // one hour
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        btc_relay::GenesisConfig::<Runtime> {
            bitcoin_confirmations: CONFIRMATIONS,
            parachain_confirmations: CONFIRMATIONS,
            disable_difficulty_check: false,
            disable_inclusion_check: false,
            disable_op_return_check: false,
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        vault_registry::GenesisConfig::<Runtime> {
            minimum_collateral_vault: 0,
            punishment_delay: 8,
            secure_collateral_threshold: FixedU128::checked_from_rational(150, 100).unwrap(),
            auction_collateral_threshold: FixedU128::checked_from_rational(120, 100).unwrap(),
            premium_redeem_threshold: FixedU128::checked_from_rational(135, 100).unwrap(),
            liquidation_collateral_threshold: FixedU128::checked_from_rational(110, 100).unwrap(),
            liquidation_vault_account_id: account_of(LIQUIDATION_VAULT),
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        issue::GenesisConfig::<Runtime> { issue_period: 10 }
            .assimilate_storage(&mut storage)
            .unwrap();

        redeem::GenesisConfig::<Runtime> {
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
            issue_fee: FixedU128::checked_from_rational(5, 1000).unwrap(), // 0.5%
            issue_griefing_collateral: FixedU128::checked_from_rational(5, 100000).unwrap(), // 0.005%
            refund_fee: FixedU128::checked_from_rational(5, 1000).unwrap(), // 0.5%
            redeem_fee: FixedU128::checked_from_rational(5, 1000).unwrap(), // 0.5%
            premium_redeem_fee: FixedU128::checked_from_rational(5, 100).unwrap(), // 5%
            auction_redeem_fee: FixedU128::checked_from_rational(5, 100).unwrap(), // 5%
            punishment_fee: FixedU128::checked_from_rational(1, 10).unwrap(), // 10%
            replace_griefing_collateral: FixedU128::checked_from_rational(1, 10).unwrap(), // 10%
            fee_pool_account_id: account_of(FEE_POOL),
            maintainer_account_id: account_of(MAINTAINER),
            epoch_period: 5,
            vault_rewards_issued: FixedU128::checked_from_rational(90, 100).unwrap(), // 90%
            vault_rewards_locked: FixedU128::checked_from_rational(10, 100).unwrap(), // 10%
            vault_rewards: FixedU128::checked_from_rational(70, 100).unwrap(),        // 70%
            relayer_rewards: FixedU128::checked_from_rational(20, 100).unwrap(),      // 20%
            maintainer_rewards: FixedU128::checked_from_rational(10, 100).unwrap(),   // 10%
            collator_rewards: FixedU128::checked_from_rational(0, 100).unwrap(),      // 0%
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        sla::GenesisConfig::<Runtime> {
            vault_target_sla: FixedI128::from(100),
            vault_redeem_failure_sla_change: FixedI128::from(-100),
            vault_executed_issue_max_sla_change: FixedI128::from(4),
            vault_submitted_issue_proof: FixedI128::from(1),
            vault_refunded: FixedI128::from(1),
            relayer_target_sla: FixedI128::from(100),
            relayer_block_submission: FixedI128::from(1),
            relayer_duplicate_block_submission: FixedI128::from(1),
            relayer_correct_no_data_vote_or_report: FixedI128::from(1),
            relayer_correct_invalid_vote_or_report: FixedI128::from(10),
            relayer_correct_theft_report: FixedI128::from(1),
            relayer_false_no_data_vote_or_report: FixedI128::from(-10),
            relayer_false_invalid_vote_or_report: FixedI128::from(-100),
            relayer_ignored_vote: FixedI128::from(-10),
        }
        .assimilate_storage(&mut storage)
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

            execute()
        })
    }

    /// used for btc-relay test
    pub fn execute_without_relay_init<R>(mut self, execute: impl FnOnce() -> R) -> R {
        self.test_externalities.execute_with(|| {
            SystemModule::set_block_number(1); // required to be able to dispatch functions
            SecurityPallet::set_active_block_number(1);

            assert_ok!(ExchangeRateOraclePallet::_set_exchange_rate(FixedU128::one()));
            set_default_thresholds();

            execute()
        })
    }
}
