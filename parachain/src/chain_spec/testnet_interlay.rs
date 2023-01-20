use primitives::Rate;
use testnet_interlay_runtime::LoansConfig;

use super::*;

fn testnet_properties(bitcoin_network: &str) -> Map<String, Value> {
    let mut properties = Map::new();
    let mut token_symbol: Vec<String> = vec![];
    let mut token_decimals: Vec<u32> = vec![];
    [INTR, IBTC, DOT, KINT, KBTC, KSM].iter().for_each(|token| {
        token_symbol.push(token.symbol().to_string());
        token_decimals.push(token.decimals() as u32);
    });
    properties.insert("tokenSymbol".into(), token_symbol.into());
    properties.insert("tokenDecimals".into(), token_decimals.into());
    properties.insert("ss58Format".into(), testnet_interlay_runtime::SS58Prefix::get().into());
    properties.insert("bitcoinNetwork".into(), bitcoin_network.into());
    properties
}

fn default_pair_testnet(currency_id: CurrencyId) -> VaultCurrencyPair<CurrencyId> {
    VaultCurrencyPair {
        collateral: currency_id,
        wrapped: testnet_interlay_runtime::GetWrappedCurrencyId::get(),
    }
}

pub fn development_config(id: ParaId) -> InterlayTestnetChainSpec {
    InterlayTestnetChainSpec::from_genesis(
        "interBTC",
        "dev_testnet",
        ChainType::Development,
        move || {
            testnet_genesis(
                get_account_id_from_seed::<sr25519::Public>("Alice"),
                vec![get_authority_keys_from_seed("Alice")],
                vec![
                    get_account_id_from_seed::<sr25519::Public>("Alice"),
                    get_account_id_from_seed::<sr25519::Public>("Bob"),
                    get_account_id_from_seed::<sr25519::Public>("Charlie"),
                    get_account_id_from_seed::<sr25519::Public>("Dave"),
                    get_account_id_from_seed::<sr25519::Public>("Eve"),
                    get_account_id_from_seed::<sr25519::Public>("Ferdie"),
                    get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
                    get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
                    get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
                    get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
                    get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
                    get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
                ],
                vec![
                    (
                        get_account_id_from_seed::<sr25519::Public>("Alice"),
                        "Alice".as_bytes().to_vec(),
                    ),
                    (
                        get_account_id_from_seed::<sr25519::Public>("Bob"),
                        "Bob".as_bytes().to_vec(),
                    ),
                    (
                        get_account_id_from_seed::<sr25519::Public>("Charlie"),
                        "Charlie".as_bytes().to_vec(),
                    ),
                ],
                id,
                DEFAULT_BITCOIN_CONFIRMATIONS,
            )
        },
        Vec::new(),
        None,
        None,
        None,
        Some(testnet_properties(BITCOIN_REGTEST)),
        Extensions {
            relay_chain: "dev".into(),
            para_id: id.into(),
        },
    )
}

pub fn staging_testnet_config(id: ParaId) -> InterlayTestnetChainSpec {
    InterlayTestnetChainSpec::from_genesis(
        "interBTC",
        "staging_testnet",
        ChainType::Live,
        move || {
            testnet_genesis(
                // 5Ec37KSdjSbGKoQN4evLXrZskjc7jxXYrowPHEtH2MzRC7mv (//sudo/1)
                get_account_id_from_string("5Ec37KSdjSbGKoQN4evLXrZskjc7jxXYrowPHEtH2MzRC7mv"),
                vec![
                    // 5EqCiRZGFZ88JCK9FNmak2SkRHSohWpEFpx28vwo5c1m98Xe (//authority/1)
                    get_authority_keys_from_public_key(hex![
                        "7a6868acf544dc5c3f2f9f6f9a5952017bbefb51da41819307fc21cf3efb554d"
                    ]),
                    // 5DbwRgYTAtjJ8Mys8ta8RXxWPcSmiyx4dPRsvU1k4TYyk4jq (//authority/2)
                    get_authority_keys_from_public_key(hex![
                        "440e84dd3604be606f3110c21f93a0e981fb93b28288270dcdce8a43c68ff36e"
                    ]),
                    // 5GVtSRJmnFxVcFz7jejbCrY2SREhZJZUHuJkm2KS75bTqRF2 (//authority/3)
                    get_authority_keys_from_public_key(hex![
                        "c425b0d9fed64d3bd5be0a6d06053d2bfb72f4983146788f5684aec9f5eb0c7f"
                    ]),
                ],
                vec![
                    // 5Ec37KSdjSbGKoQN4evLXrZskjc7jxXYrowPHEtH2MzRC7mv (//sudo/1)
                    get_account_id_from_string("5Ec37KSdjSbGKoQN4evLXrZskjc7jxXYrowPHEtH2MzRC7mv"),
                    // 5ECj4iBBi3h8kYzhqLFmzVLafC64UpsXvK7H4ZZyXoVQJdJq (//oracle/1)
                    get_account_id_from_string("5ECj4iBBi3h8kYzhqLFmzVLafC64UpsXvK7H4ZZyXoVQJdJq"),
                    // 5FgWDuxgS8VasP6KtvESHUuuDn6L8BTCqbYyFW9mDwAaLtbY (//account/1)
                    get_account_id_from_string("5FgWDuxgS8VasP6KtvESHUuuDn6L8BTCqbYyFW9mDwAaLtbY"),
                    // 5H3n25VshwPeMzKhn4gnVEjCEndFsjt85ydW2Vvo8ysy7CnZ (//account/2)
                    get_account_id_from_string("5H3n25VshwPeMzKhn4gnVEjCEndFsjt85ydW2Vvo8ysy7CnZ"),
                    // 5GKciEHZWSGxtAihqGjXC6XpXSGNoudDxACuDLbYF1ipygZj (//account/3)
                    get_account_id_from_string("5GKciEHZWSGxtAihqGjXC6XpXSGNoudDxACuDLbYF1ipygZj"),
                    // 5GjJ26ffHApgUFLgxKWpWL5T5ppxWjSRJe42PjPNATLvjcJK (//account/4)
                    get_account_id_from_string("5GjJ26ffHApgUFLgxKWpWL5T5ppxWjSRJe42PjPNATLvjcJK"),
                    // 5DqzGaydetDXGya818gyuHA7GAjEWRsQN6UWNKpvfgq2KyM7 (//account/5)
                    get_account_id_from_string("5DqzGaydetDXGya818gyuHA7GAjEWRsQN6UWNKpvfgq2KyM7"),
                ],
                vec![(
                    // 5ECj4iBBi3h8kYzhqLFmzVLafC64UpsXvK7H4ZZyXoVQJdJq (//oracle/1)
                    get_account_id_from_string("5ECj4iBBi3h8kYzhqLFmzVLafC64UpsXvK7H4ZZyXoVQJdJq"),
                    "Interlay".as_bytes().to_vec(),
                )],
                id,
                DEFAULT_BITCOIN_CONFIRMATIONS,
            )
        },
        Vec::new(),
        None,
        None,
        None,
        Some(testnet_properties(BITCOIN_TESTNET)),
        Extensions {
            relay_chain: "staging".into(),
            para_id: id.into(),
        },
    )
}

pub fn rococo_local_testnet_config(id: ParaId) -> InterlayTestnetChainSpec {
    development_config(id)
}

fn testnet_genesis(
    root_key: AccountId,
    invulnerables: Vec<(AccountId, AuraId)>,
    endowed_accounts: Vec<AccountId>,
    authorized_oracles: Vec<(AccountId, Vec<u8>)>,
    id: ParaId,
    bitcoin_confirmations: u32,
) -> testnet_interlay_runtime::GenesisConfig {
    testnet_interlay_runtime::GenesisConfig {
        system: testnet_interlay_runtime::SystemConfig {
            code: testnet_interlay_runtime::WASM_BINARY
                .expect("WASM binary was not build, please build it!")
                .to_vec(),
        },
        parachain_system: Default::default(),
        parachain_info: testnet_interlay_runtime::ParachainInfoConfig { parachain_id: id },
        collator_selection: testnet_interlay_runtime::CollatorSelectionConfig {
            invulnerables: invulnerables.iter().cloned().map(|(acc, _)| acc).collect(),
            candidacy_bond: Zero::zero(),
            ..Default::default()
        },
        session: testnet_interlay_runtime::SessionConfig {
            keys: invulnerables
                .iter()
                .cloned()
                .map(|(acc, aura)| {
                    (
                        acc.clone(),                                    // account id
                        acc.clone(),                                    // validator id
                        testnet_interlay_runtime::SessionKeys { aura }, // session keys
                    )
                })
                .collect(),
        },
        // no need to pass anything to aura, in fact it will panic if we do.
        // Session will take care of this.
        aura: Default::default(),
        aura_ext: Default::default(),
        security: testnet_interlay_runtime::SecurityConfig {
            initial_status: testnet_interlay_runtime::StatusCode::Error,
        },
        sudo: testnet_interlay_runtime::SudoConfig {
            // Assign network admin rights.
            key: Some(root_key.clone()),
        },
        asset_registry: Default::default(),
        tokens: testnet_interlay_runtime::TokensConfig {
            balances: endowed_accounts
                .iter()
                .flat_map(|k| vec![(k.clone(), Token(DOT), 1 << 60), (k.clone(), Token(INTR), 1 << 60)])
                .collect(),
        },
        vesting: Default::default(),
        oracle: testnet_interlay_runtime::OracleConfig {
            authorized_oracles,
            max_delay: DEFAULT_MAX_DELAY_MS,
        },
        btc_relay: testnet_interlay_runtime::BTCRelayConfig {
            bitcoin_confirmations,
            parachain_confirmations: bitcoin_confirmations
                .saturating_mul(testnet_interlay_runtime::BITCOIN_BLOCK_SPACING),
            disable_difficulty_check: true,
            disable_inclusion_check: false,
        },
        issue: testnet_interlay_runtime::IssueConfig {
            issue_period: testnet_interlay_runtime::DAYS,
            issue_btc_dust_value: DEFAULT_DUST_VALUE,
        },
        redeem: testnet_interlay_runtime::RedeemConfig {
            redeem_transaction_size: expected_transaction_size(),
            redeem_period: testnet_interlay_runtime::DAYS * 2,
            redeem_btc_dust_value: DEFAULT_DUST_VALUE,
        },
        replace: testnet_interlay_runtime::ReplaceConfig {
            replace_period: testnet_interlay_runtime::DAYS * 2,
            replace_btc_dust_value: DEFAULT_DUST_VALUE,
        },
        vault_registry: testnet_interlay_runtime::VaultRegistryConfig {
            minimum_collateral_vault: vec![(Token(DOT), 30 * DOT.one())],
            punishment_delay: interlay_runtime::DAYS,
            system_collateral_ceiling: vec![(default_pair_testnet(Token(DOT)), 2_450_000 * DOT.one())],
            secure_collateral_threshold: vec![(
                default_pair_testnet(Token(DOT)),
                /* 260% */
                FixedU128::checked_from_rational(260, 100).unwrap(),
            )],
            premium_redeem_threshold: vec![(
                default_pair_testnet(Token(DOT)),
                /* 200% */
                FixedU128::checked_from_rational(200, 100).unwrap(),
            )],
            liquidation_collateral_threshold: vec![(
                default_pair_testnet(Token(DOT)),
                /* 150% */
                FixedU128::checked_from_rational(150, 100).unwrap(),
            )],
        },
        fee: testnet_interlay_runtime::FeeConfig {
            issue_fee: FixedU128::checked_from_rational(15, 10000).unwrap(), // 0.15%
            issue_griefing_collateral: FixedU128::checked_from_rational(5, 1000).unwrap(), // 0.5%
            redeem_fee: FixedU128::checked_from_rational(5, 1000).unwrap(),  // 0.5%
            premium_redeem_fee: FixedU128::checked_from_rational(5, 100).unwrap(), // 5%
            punishment_fee: FixedU128::checked_from_rational(1, 10).unwrap(), // 10%
            replace_griefing_collateral: FixedU128::checked_from_rational(1, 10).unwrap(), // 10%
        },
        nomination: testnet_interlay_runtime::NominationConfig {
            is_nomination_enabled: false,
        },
        technical_committee: Default::default(),
        technical_membership: Default::default(),
        treasury: Default::default(),
        democracy: Default::default(),
        supply: testnet_interlay_runtime::SupplyConfig {
            initial_supply: testnet_interlay_runtime::token_distribution::INITIAL_ALLOCATION,
            // start of year 5
            start_height: testnet_interlay_runtime::YEARS * 4,
            inflation: FixedU128::checked_from_rational(2, 100).unwrap(), // 2%
        },
        polkadot_xcm: testnet_interlay_runtime::PolkadotXcmConfig {
            safe_xcm_version: Some(2),
        },
        loans: LoansConfig {
            max_exchange_rate: Rate::from_inner(loans::DEFAULT_MAX_EXCHANGE_RATE),
            min_exchange_rate: Rate::from_inner(loans::DEFAULT_MIN_EXCHANGE_RATE),
        },
    }
}
