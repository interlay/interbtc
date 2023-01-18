use super::*;

pub const PARA_ID: u32 = 2092;

fn kintsugi_properties() -> Map<String, Value> {
    let mut properties = Map::new();
    let mut token_symbol: Vec<String> = vec![];
    let mut token_decimals: Vec<u32> = vec![];
    [KINT, KBTC, KSM, INTR, IBTC, DOT].iter().for_each(|token| {
        token_symbol.push(token.symbol().to_string());
        token_decimals.push(token.decimals() as u32);
    });
    properties.insert("tokenSymbol".into(), token_symbol.into());
    properties.insert("tokenDecimals".into(), token_decimals.into());
    properties.insert("ss58Format".into(), kintsugi_runtime::SS58Prefix::get().into());
    properties.insert("bitcoinNetwork".into(), BITCOIN_MAINNET.into());
    properties
}

fn default_pair_kintsugi(currency_id: CurrencyId) -> VaultCurrencyPair<CurrencyId> {
    VaultCurrencyPair {
        collateral: currency_id,
        wrapped: kintsugi_runtime::GetWrappedCurrencyId::get(),
    }
}

pub fn kintsugi_dev_config() -> KintsugiChainSpec {
    let id: ParaId = PARA_ID.into();
    KintsugiChainSpec::from_genesis(
        "Kintsugi",
        "kintsugi",
        ChainType::Live,
        move || {
            kintsugi_mainnet_genesis(
                vec![get_authority_keys_from_seed("Alice")],
                vec![(
                    get_account_id_from_seed::<sr25519::Public>("Bob"),
                    "Bob".as_bytes().to_vec(),
                )],
                id,
                1,
            )
        },
        Vec::new(),
        None,
        None,
        None,
        Some(kintsugi_properties()),
        Extensions {
            relay_chain: "kusama".into(),
            para_id: id.into(),
        },
    )
}

pub fn kintsugi_mainnet_config() -> KintsugiChainSpec {
    let id: ParaId = PARA_ID.into();
    KintsugiChainSpec::from_genesis(
        "Kintsugi",
        "kintsugi",
        ChainType::Live,
        move || {
            kintsugi_mainnet_genesis(
                vec![
                    // 5DyzufhT1Ynxk9uxrWHjrVuap8oB4Zz7uYdquZHxFxvYBovd (//authority/0)
                    get_authority_keys_from_public_key(hex![
                        "54e1a41c9ba60ca45e911e8798ba9d81c22b04435b04816490ebddffe4dffc5c"
                    ]),
                    // 5EvgAvVBQXtFFbcN74rYR2HE8RsWsEJHqPHhrGX427cnbvY2 (//authority/1)
                    get_authority_keys_from_public_key(hex![
                        "7e951061df4d5b61b31a69d62233a5a3a2abdc3195902dd22bc062fadbf42e17"
                    ]),
                    // 5Hp2yfUMoA5uJM6DQpDJAuCHdzvhzn57gurH1Cxp4cUTzciB (//authority/2)
                    get_authority_keys_from_public_key(hex![
                        "fe3915da55703833883c8e0dc9a81bc5ab5e3b4099b23d810cd5d78c6598395b"
                    ]),
                    // 5FQzZEbc5CtF7gR1De449GtvDwpyVwWPZMqyq9yjJmxXKmgU (//authority/3)
                    get_authority_keys_from_public_key(hex![
                        "942dd2ded2896fa236c0f0df58dff88a04d7cf661a4676059d79dc54a271234a"
                    ]),
                    // 5EqmSYibeeyypp2YGtJdkZxiNjLKpQLCMpW5J3hNgWBfT9Gw (//authority/4)
                    get_authority_keys_from_public_key(hex![
                        "7ad693485d4d67a2112881347a553009f0c1de3b26e662aa3863085f536d0537"
                    ]),
                    // 5E1WeDF5L8xXLmMnLmJUCXo5xqLD6zzPP14T9vESydQmUA29 (//authority/5)
                    get_authority_keys_from_public_key(hex![
                        "5608fa7874491c640d0420f5f44650a0b5b8b67411b2670b68440bb97e74ee1c"
                    ]),
                    // 5D7eFVnyAhcbEJAPAVENqoCr44zTbztsiragiYjz1ExDePja (//authority/6)
                    get_authority_keys_from_public_key(hex![
                        "2e79d45517532bc4b6b3359be9ea2aa8b711a0a5362880cfb6651bcb87fe1b05"
                    ]),
                    // 5FkCciu8zasoDoViTbAYpcHgitQgB5GHN64HWdXyy8kykXFK (//authority/7)
                    get_authority_keys_from_public_key(hex![
                        "a2d4159da7f458f8140899f443b480199c65e75ffb755ea9e097aa5b18352001"
                    ]),
                    // 5H3E3GF1LUeyowgRx47n8AJsRCyzA4f2YNuTo4qEQy7fbbBo (//authority/8)
                    get_authority_keys_from_public_key(hex![
                        "dc0c47c6f8fd81190d4fcee4ab2074db5d83eaf301f2cd795ec9b39b8e753f66"
                    ]),
                    // 5ERqgB3mYvotBFu6vVf7fdnTgxHJvVidBpQL8W4yrpFL25mo (//authority/9)
                    get_authority_keys_from_public_key(hex![
                        "6896f1128f9a92c68f14713f0cbeb67a402621d7c80257ea3b246fcca5aede17"
                    ]),
                ],
                vec![(
                    get_account_id_from_string("5DcrZv97CipkXni4aXcg98Nz9doT6nfs6t3THn7hhnRXTd6D"),
                    "Interlay".as_bytes().to_vec(),
                )],
                id,
                SECURE_BITCOIN_CONFIRMATIONS,
            )
        },
        Vec::new(),
        None,
        None,
        None,
        Some(kintsugi_properties()),
        Extensions {
            relay_chain: "kusama".into(),
            para_id: id.into(),
        },
    )
}

fn kintsugi_mainnet_genesis(
    invulnerables: Vec<(AccountId, AuraId)>,
    authorized_oracles: Vec<(AccountId, Vec<u8>)>,
    id: ParaId,
    bitcoin_confirmations: u32,
) -> kintsugi_runtime::GenesisConfig {
    kintsugi_runtime::GenesisConfig {
        system: kintsugi_runtime::SystemConfig {
            code: kintsugi_runtime::WASM_BINARY
                .expect("WASM binary was not build, please build it!")
                .to_vec(),
        },
        parachain_system: Default::default(),
        parachain_info: kintsugi_runtime::ParachainInfoConfig { parachain_id: id },
        collator_selection: kintsugi_runtime::CollatorSelectionConfig {
            invulnerables: invulnerables.iter().cloned().map(|(acc, _)| acc).collect(),
            candidacy_bond: Zero::zero(),
            ..Default::default()
        },
        session: kintsugi_runtime::SessionConfig {
            keys: invulnerables
                .iter()
                .cloned()
                .map(|(acc, aura)| {
                    (
                        acc.clone(),                            // account id
                        acc.clone(),                            // validator id
                        kintsugi_runtime::SessionKeys { aura }, // session keys
                    )
                })
                .collect(),
        },
        // no need to pass anything to aura, in fact it will panic if we do.
        // Session will take care of this.
        aura: Default::default(),
        aura_ext: Default::default(),
        security: kintsugi_runtime::SecurityConfig {
            initial_status: kintsugi_runtime::StatusCode::Error,
        },
        asset_registry: Default::default(),
        tokens: Default::default(),
        vesting: Default::default(),
        oracle: kintsugi_runtime::OracleConfig {
            authorized_oracles,
            max_delay: DEFAULT_MAX_DELAY_MS,
        },
        btc_relay: kintsugi_runtime::BTCRelayConfig {
            bitcoin_confirmations,
            parachain_confirmations: bitcoin_confirmations.saturating_mul(kintsugi_runtime::BITCOIN_BLOCK_SPACING),
            disable_difficulty_check: false,
            disable_inclusion_check: false,
        },
        issue: kintsugi_runtime::IssueConfig {
            issue_period: kintsugi_runtime::DAYS * 2,
            issue_btc_dust_value: DEFAULT_DUST_VALUE,
        },
        redeem: kintsugi_runtime::RedeemConfig {
            redeem_transaction_size: expected_transaction_size(),
            redeem_period: kintsugi_runtime::DAYS * 2,
            redeem_btc_dust_value: DEFAULT_DUST_VALUE,
        },
        replace: kintsugi_runtime::ReplaceConfig {
            replace_period: kintsugi_runtime::DAYS * 2,
            replace_btc_dust_value: DEFAULT_DUST_VALUE,
        },
        vault_registry: kintsugi_runtime::VaultRegistryConfig {
            minimum_collateral_vault: vec![(Token(KINT), 55 * KINT.one()), (Token(KSM), 3 * KSM.one())],
            punishment_delay: kintsugi_runtime::DAYS,
            system_collateral_ceiling: vec![
                (default_pair_kintsugi(Token(KINT)), 26_200 * KINT.one()),
                (default_pair_kintsugi(Token(KSM)), 60_000 * KSM.one()),
            ],
            secure_collateral_threshold: vec![
                (
                    default_pair_kintsugi(Token(KINT)),
                    /* 900% */
                    FixedU128::checked_from_rational(900, 100).unwrap(),
                ),
                (
                    default_pair_kintsugi(Token(KSM)),
                    /* 260% */
                    FixedU128::checked_from_rational(260, 100).unwrap(),
                ),
            ],
            premium_redeem_threshold: vec![
                (
                    default_pair_kintsugi(Token(KINT)),
                    /* 650% */
                    FixedU128::checked_from_rational(650, 100).unwrap(),
                ),
                (
                    default_pair_kintsugi(Token(KSM)),
                    /* 200% */
                    FixedU128::checked_from_rational(200, 100).unwrap(),
                ),
            ],
            liquidation_collateral_threshold: vec![
                (
                    default_pair_kintsugi(Token(KINT)),
                    /* 500% */
                    FixedU128::checked_from_rational(500, 100).unwrap(),
                ),
                (
                    default_pair_kintsugi(Token(KSM)),
                    /* 150% */
                    FixedU128::checked_from_rational(150, 100).unwrap(),
                ),
            ],
        },
        fee: kintsugi_runtime::FeeConfig {
            issue_fee: FixedU128::checked_from_rational(15, 10000).unwrap(), // 0.15%
            issue_griefing_collateral: FixedU128::checked_from_rational(5, 1000).unwrap(), // 0.5%
            redeem_fee: FixedU128::checked_from_rational(5, 1000).unwrap(),  // 0.5%
            premium_redeem_fee: FixedU128::checked_from_rational(5, 100).unwrap(), // 5%
            punishment_fee: FixedU128::checked_from_rational(1, 10).unwrap(), // 10%
            replace_griefing_collateral: FixedU128::checked_from_rational(1, 10).unwrap(), // 10%
        },
        nomination: kintsugi_runtime::NominationConfig {
            is_nomination_enabled: false,
        },
        technical_committee: Default::default(),
        technical_membership: Default::default(),
        treasury: Default::default(),
        democracy: Default::default(),
        supply: kintsugi_runtime::SupplyConfig {
            initial_supply: kintsugi_runtime::token_distribution::INITIAL_ALLOCATION,
            // start of year 5
            start_height: kintsugi_runtime::YEARS * 4,
            inflation: FixedU128::checked_from_rational(2, 100).unwrap(), // 2%
        },
        polkadot_xcm: kintsugi_runtime::PolkadotXcmConfig {
            safe_xcm_version: Some(2),
        },
        sudo: Default::default(),
    }
}
