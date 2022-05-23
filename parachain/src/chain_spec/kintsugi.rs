use super::*;

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

pub fn kintsugi_mainnet_config() -> KintsugiChainSpec {
    let id: ParaId = 2092.into();
    KintsugiChainSpec::from_genesis(
        "Kintsugi",
        "kintsugi",
        ChainType::Live,
        move || {
            kintsugi_mainnet_genesis(
                vec![
                    // 5DyzufhT1Ynxk9uxrWHjrVuap8oB4Zz7uYdquZHxFxvYBovd (//authority/0)
                    hex!["54e1a41c9ba60ca45e911e8798ba9d81c22b04435b04816490ebddffe4dffc5c"].unchecked_into(),
                    // 5EvgAvVBQXtFFbcN74rYR2HE8RsWsEJHqPHhrGX427cnbvY2 (//authority/1)
                    hex!["7e951061df4d5b61b31a69d62233a5a3a2abdc3195902dd22bc062fadbf42e17"].unchecked_into(),
                    // 5Hp2yfUMoA5uJM6DQpDJAuCHdzvhzn57gurH1Cxp4cUTzciB (//authority/2)
                    hex!["fe3915da55703833883c8e0dc9a81bc5ab5e3b4099b23d810cd5d78c6598395b"].unchecked_into(),
                    // 5FQzZEbc5CtF7gR1De449GtvDwpyVwWPZMqyq9yjJmxXKmgU (//authority/3)
                    hex!["942dd2ded2896fa236c0f0df58dff88a04d7cf661a4676059d79dc54a271234a"].unchecked_into(),
                    // 5EqmSYibeeyypp2YGtJdkZxiNjLKpQLCMpW5J3hNgWBfT9Gw (//authority/4)
                    hex!["7ad693485d4d67a2112881347a553009f0c1de3b26e662aa3863085f536d0537"].unchecked_into(),
                    // 5E1WeDF5L8xXLmMnLmJUCXo5xqLD6zzPP14T9vESydQmUA29 (//authority/5)
                    hex!["5608fa7874491c640d0420f5f44650a0b5b8b67411b2670b68440bb97e74ee1c"].unchecked_into(),
                    // 5D7eFVnyAhcbEJAPAVENqoCr44zTbztsiragiYjz1ExDePja (//authority/6)
                    hex!["2e79d45517532bc4b6b3359be9ea2aa8b711a0a5362880cfb6651bcb87fe1b05"].unchecked_into(),
                    // 5FkCciu8zasoDoViTbAYpcHgitQgB5GHN64HWdXyy8kykXFK (//authority/7)
                    hex!["a2d4159da7f458f8140899f443b480199c65e75ffb755ea9e097aa5b18352001"].unchecked_into(),
                    // 5H3E3GF1LUeyowgRx47n8AJsRCyzA4f2YNuTo4qEQy7fbbBo (//authority/8)
                    hex!["dc0c47c6f8fd81190d4fcee4ab2074db5d83eaf301f2cd795ec9b39b8e753f66"].unchecked_into(),
                    // 5ERqgB3mYvotBFu6vVf7fdnTgxHJvVidBpQL8W4yrpFL25mo (//authority/9)
                    hex!["6896f1128f9a92c68f14713f0cbeb67a402621d7c80257ea3b246fcca5aede17"].unchecked_into(),
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
    initial_authorities: Vec<AuraId>,
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
        aura: kintsugi_runtime::AuraConfig {
            authorities: initial_authorities,
        },
        aura_ext: Default::default(),
        security: kintsugi_runtime::SecurityConfig {
            initial_status: kintsugi_runtime::StatusCode::Shutdown,
        },
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
            issue_period: kintsugi_runtime::DAYS,
            issue_btc_dust_value: DEFAULT_DUST_VALUE,
        },
        redeem: kintsugi_runtime::RedeemConfig {
            redeem_transaction_size: expected_transaction_size(),
            redeem_period: kintsugi_runtime::DAYS,
            redeem_btc_dust_value: DEFAULT_DUST_VALUE,
        },
        replace: kintsugi_runtime::ReplaceConfig {
            replace_period: kintsugi_runtime::DAYS,
            replace_btc_dust_value: DEFAULT_DUST_VALUE,
        },
        vault_registry: kintsugi_runtime::VaultRegistryConfig {
            minimum_collateral_vault: vec![(Token(KSM), 3 * KSM.one())],
            punishment_delay: kintsugi_runtime::DAYS,
            system_collateral_ceiling: vec![(default_pair_kintsugi(Token(KSM)), 317 * KSM.one())], /* 317 ksm, about
                                                                                                    * 100k
                                                                                                    * USD at
                                                                                                    * time of writing */
            secure_collateral_threshold: vec![(
                default_pair_kintsugi(Token(KSM)),
                FixedU128::checked_from_rational(260, 100).unwrap(),
            )], /* 260% */
            premium_redeem_threshold: vec![(
                default_pair_kintsugi(Token(KSM)),
                FixedU128::checked_from_rational(200, 100).unwrap(),
            )], /* 200% */
            liquidation_collateral_threshold: vec![(
                default_pair_kintsugi(Token(KSM)),
                FixedU128::checked_from_rational(150, 100).unwrap(),
            )], /* 150% */
        },
        fee: kintsugi_runtime::FeeConfig {
            issue_fee: FixedU128::checked_from_rational(15, 10000).unwrap(), // 0.15%
            issue_griefing_collateral: FixedU128::checked_from_rational(5, 100000).unwrap(), // 0.005%
            refund_fee: FixedU128::checked_from_rational(5, 1000).unwrap(),  // 0.5%
            redeem_fee: FixedU128::checked_from_rational(5, 1000).unwrap(),  // 0.5%
            premium_redeem_fee: FixedU128::checked_from_rational(5, 100).unwrap(), // 5%
            punishment_fee: FixedU128::checked_from_rational(1, 10).unwrap(), // 10%
            replace_griefing_collateral: FixedU128::checked_from_rational(1, 10).unwrap(), // 10%
            theft_fee: FixedU128::checked_from_rational(5, 100).unwrap(),    // 5%
            theft_fee_max: 10000000,                                         // 0.1 BTC
        },
        refund: kintsugi_runtime::RefundConfig {
            refund_btc_dust_value: DEFAULT_DUST_VALUE,
            refund_transaction_size: expected_transaction_size(),
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
    }
}
