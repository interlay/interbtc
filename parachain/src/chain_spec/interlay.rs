use super::*;

pub const PARA_ID: u32 = 2032;

fn interlay_properties() -> Map<String, Value> {
    let mut properties = Map::new();
    let mut token_symbol: Vec<String> = vec![];
    let mut token_decimals: Vec<u32> = vec![];
    [INTR, IBTC, DOT, KINT, KBTC, KSM].iter().for_each(|token| {
        token_symbol.push(token.symbol().to_string());
        token_decimals.push(token.decimals() as u32);
    });
    properties.insert("tokenSymbol".into(), token_symbol.into());
    properties.insert("tokenDecimals".into(), token_decimals.into());
    properties.insert("ss58Format".into(), interlay_runtime::SS58Prefix::get().into());
    properties.insert("bitcoinNetwork".into(), BITCOIN_MAINNET.into());
    properties
}

fn default_pair_interlay(currency_id: CurrencyId) -> VaultCurrencyPair<CurrencyId> {
    VaultCurrencyPair {
        collateral: currency_id,
        wrapped: interlay_runtime::GetWrappedCurrencyId::get(),
    }
}

pub fn interlay_dev_config() -> InterlayChainSpec {
    let id: ParaId = PARA_ID.into();
    InterlayChainSpec::from_genesis(
        "Interlay",
        "interlay",
        ChainType::Live,
        move || {
            interlay_mainnet_genesis(
                vec![get_authority_keys_from_seed("Alice")],
                vec![(
                    get_account_id_from_seed::<sr25519::Public>("Bob"),
                    BoundedVec::truncate_from("Bob".as_bytes().to_vec()),
                )],
                vec![get_account_id_from_seed::<sr25519::Public>("Alice")],
                id,
                1,
            )
        },
        Vec::new(),
        None,
        None,
        None,
        Some(interlay_properties()),
        Extensions {
            relay_chain: "polkadot".into(),
            para_id: id.into(),
        },
    )
}

pub fn interlay_mainnet_config() -> InterlayChainSpec {
    let id: ParaId = PARA_ID.into();
    InterlayChainSpec::from_genesis(
        "Interlay",
        "interlay",
        ChainType::Live,
        move || {
            interlay_mainnet_genesis(
                vec![
                    // 5CDEceADNMhAgHBCDnb7Ls1YZKgwe2z3qmcwNHTeAFr5dGrW (//authority/1)
                    get_authority_keys_from_public_key(hex![
                        "068181205488a5517460dd305c9ec781ddf6e68627609ec88cbb60d0b7647d0f"
                    ]),
                    // 5G6AgvRRkzFvs69SXY2Ah6PmjySswGFqHTgriqLohNMzfEsc (//authority/2)
                    get_authority_keys_from_public_key(hex![
                        "b20e80ecc31ce2ccb3487e7cc4447098417813cf7553f1f459662f782bbfd12a"
                    ]),
                    // 5EXCEev51P1KFkMQQdjT25KzMWMLG5EXw51uhaCQbDziPe8t (//authority/3)
                    get_authority_keys_from_public_key(hex![
                        "6cac613f09264c7397fa27dfc131d0c77a4dc8d5b5e22a22e3e1a6ac8e00d211"
                    ]),
                    // 5GH6mdEu56ku6ez26udZkaL9F5unbV7sUeJHnYbkLx4LTgiN (//authority/4)
                    get_authority_keys_from_public_key(hex![
                        "ba6502c812d5ece87390df7f955d50f1fc55adff99e4bc68fa7b58494bd0dc1e"
                    ]),
                    // 5H3X7DPUsnUUBqtRxCnSbrPX38jwsxg5pXcNyMabCf9QaU6i (//authority/5)
                    get_authority_keys_from_public_key(hex![
                        "dc45bc9ddeaacb1ffd04bfaf1366033f54640380a51a255448a639aa670d680c"
                    ]),
                    // 5Fy933qEzYeiN22fbWEU4RgJkvhVwXurPPZsrXstkoZFNcBS (//authority/6)
                    get_authority_keys_from_public_key(hex![
                        "acb238ad11721c943d8e43232efde998276179d7994aa2500b45d3adbe4ab90c"
                    ]),
                    // 5Ew8SA3y8jg4kfYAAatJ541EdZAmpyG8yCaZESJnE2nhsAE5 (//authority/7)
                    get_authority_keys_from_public_key(hex![
                        "7eed78d2af8350ddc6da7bafaeeac9df86f71ae0efcfd04e99a423b72003c007"
                    ]),
                    // 5EpntRydKc1AbGwPk7xt4aLnDoisQQ8dqY6zCYGFCxH9ex7M (//authority/8)
                    get_authority_keys_from_public_key(hex![
                        "7a1832d12c6ab761b9fbc7747d6a26601c42a68e2e3086cee64c7e84178d306d"
                    ]),
                    // 5Fjk4u3j4buQtf5YMU7Pj6AtSrvFaH5eGyKeUdQvyc41ipgY (//authority/9)
                    get_authority_keys_from_public_key(hex![
                        "a27ab6a94eb0d61f9e95adb45e68b5c71fd668070e664238bcbd51ca7515e168"
                    ]),
                ],
                vec![(
                    get_account_id_from_string("5FyE5kCDSVtM1KmscBBa2Api8ZsF2DBT81QHf9RuS2NntUPw"),
                    BoundedVec::truncate_from("Interlay".as_bytes().to_vec()),
                )],
                vec![], // no endowed accounts
                id,
                SECURE_BITCOIN_CONFIRMATIONS,
            )
        },
        Vec::new(),
        None,
        None,
        None,
        Some(interlay_properties()),
        Extensions {
            relay_chain: "polkadot".into(),
            para_id: id.into(),
        },
    )
}

fn interlay_mainnet_genesis(
    invulnerables: Vec<(AccountId, AuraId)>,
    authorized_oracles: Vec<(AccountId, interlay_runtime::OracleName)>,
    endowed_accounts: Vec<AccountId>,
    id: ParaId,
    bitcoin_confirmations: u32,
) -> interlay_runtime::GenesisConfig {
    interlay_runtime::GenesisConfig {
        system: interlay_runtime::SystemConfig {
            code: interlay_runtime::WASM_BINARY
                .expect("WASM binary was not build, please build it!")
                .to_vec(),
        },
        parachain_system: Default::default(),
        parachain_info: interlay_runtime::ParachainInfoConfig { parachain_id: id },
        collator_selection: interlay_runtime::CollatorSelectionConfig {
            invulnerables: invulnerables.iter().cloned().map(|(acc, _)| acc).collect(),
            candidacy_bond: Zero::zero(),
            ..Default::default()
        },
        session: interlay_runtime::SessionConfig {
            keys: invulnerables
                .iter()
                .cloned()
                .map(|(acc, aura)| {
                    (
                        acc.clone(),                            // account id
                        acc.clone(),                            // validator id
                        interlay_runtime::SessionKeys { aura }, // session keys
                    )
                })
                .collect(),
        },
        // no need to pass anything to aura, in fact it will panic if we do.
        // Session will take care of this.
        aura: Default::default(),
        aura_ext: Default::default(),
        security: interlay_runtime::SecurityConfig {
            initial_status: interlay_runtime::StatusCode::Error,
        },
        asset_registry: Default::default(),
        tokens: interlay_runtime::TokensConfig {
            balances: endowed_accounts
                .iter()
                .flat_map(|k| vec![(k.clone(), Token(INTR), 1 << 60)])
                .collect(),
        },
        vesting: Default::default(),
        oracle: interlay_runtime::OracleConfig {
            authorized_oracles,
            max_delay: DEFAULT_MAX_DELAY_MS,
        },
        btc_relay: interlay_runtime::BTCRelayConfig {
            bitcoin_confirmations,
            parachain_confirmations: bitcoin_confirmations.saturating_mul(interlay_runtime::BITCOIN_BLOCK_SPACING),
            disable_difficulty_check: false,
            disable_inclusion_check: false,
        },
        issue: interlay_runtime::IssueConfig {
            issue_period: interlay_runtime::DAYS,
            issue_btc_dust_value: DEFAULT_DUST_VALUE,
        },
        redeem: interlay_runtime::RedeemConfig {
            redeem_transaction_size: expected_transaction_size(),
            redeem_period: interlay_runtime::DAYS * 2,
            redeem_btc_dust_value: DEFAULT_DUST_VALUE,
        },
        replace: interlay_runtime::ReplaceConfig {
            replace_period: interlay_runtime::DAYS * 2,
            replace_btc_dust_value: DEFAULT_DUST_VALUE,
        },
        vault_registry: interlay_runtime::VaultRegistryConfig {
            minimum_collateral_vault: vec![(Token(DOT), 30 * DOT.one())],
            punishment_delay: interlay_runtime::DAYS,
            system_collateral_ceiling: vec![(default_pair_interlay(Token(DOT)), 2_450_000 * DOT.one())],
            secure_collateral_threshold: vec![(
                default_pair_interlay(Token(DOT)),
                /* 260% */
                FixedU128::checked_from_rational(260, 100).unwrap(),
            )],
            premium_redeem_threshold: vec![(
                default_pair_interlay(Token(DOT)),
                /* 200% */
                FixedU128::checked_from_rational(200, 100).unwrap(),
            )],
            liquidation_collateral_threshold: vec![(
                default_pair_interlay(Token(DOT)),
                /* 150% */
                FixedU128::checked_from_rational(150, 100).unwrap(),
            )],
        },
        fee: interlay_runtime::FeeConfig {
            issue_fee: FixedU128::checked_from_rational(15, 10000).unwrap(), // 0.15%
            issue_griefing_collateral: FixedU128::checked_from_rational(5, 1000).unwrap(), // 0.5%
            redeem_fee: FixedU128::checked_from_rational(5, 1000).unwrap(),  // 0.5%
            premium_redeem_fee: FixedU128::checked_from_rational(5, 100).unwrap(), // 5%
            punishment_fee: FixedU128::checked_from_rational(1, 10).unwrap(), // 10%
            replace_griefing_collateral: FixedU128::checked_from_rational(1, 10).unwrap(), // 10%
        },
        nomination: interlay_runtime::NominationConfig {
            is_nomination_enabled: false,
        },
        technical_committee: Default::default(),
        technical_membership: Default::default(),
        democracy: Default::default(),
        supply: interlay_runtime::SupplyConfig {
            initial_supply: interlay_runtime::token_distribution::INITIAL_ALLOCATION,
            // start of year 5
            start_height: interlay_runtime::YEARS * 4,
            inflation: FixedU128::checked_from_rational(2, 100).unwrap(), // 2%
        },
        polkadot_xcm: interlay_runtime::PolkadotXcmConfig {
            safe_xcm_version: Some(3),
        },
        sudo: Default::default(),
    }
}
