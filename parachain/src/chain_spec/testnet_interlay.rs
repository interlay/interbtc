use super::*;
use crate::chain_spec::interlay::interlay_mainnet_genesis;

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
    properties.insert("ss58Format".into(), interlay_runtime::SS58Prefix::get().into());
    properties.insert("bitcoinNetwork".into(), bitcoin_network.into());
    properties
}

pub fn staging_mainnet_config(benchmarking: bool) -> InterlayChainSpec {
    InterlayChainSpec::from_genesis(
        "interBTC",
        "staging_testnet",
        ChainType::Live,
        move || {
            let mut genesis = interlay_mainnet_genesis(
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
                vec![(
                    // 5ECj4iBBi3h8kYzhqLFmzVLafC64UpsXvK7H4ZZyXoVQJdJq (//oracle/1)
                    get_account_id_from_string("5ECj4iBBi3h8kYzhqLFmzVLafC64UpsXvK7H4ZZyXoVQJdJq"),
                    BoundedVec::truncate_from("Interlay".as_bytes().to_vec()),
                )],
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
                ]
                .into_iter()
                .chain(if benchmarking {
                    vec![get_account_id_from_seed::<sr25519::Public>("Alice")]
                } else {
                    vec![]
                })
                .collect(),
                crate::chain_spec::interlay::PARA_ID.into(),
                DEFAULT_BITCOIN_CONFIRMATIONS,
            );

            // 5Ec37KSdjSbGKoQN4evLXrZskjc7jxXYrowPHEtH2MzRC7mv (//sudo/1)
            genesis.sudo.key = Some(get_account_id_from_string(
                "5Ec37KSdjSbGKoQN4evLXrZskjc7jxXYrowPHEtH2MzRC7mv",
            ));
            genesis.btc_relay.bitcoin_confirmations = DEFAULT_BITCOIN_CONFIRMATIONS;
            genesis.btc_relay.parachain_confirmations =
                DEFAULT_BITCOIN_CONFIRMATIONS.saturating_mul(interlay_runtime::BITCOIN_BLOCK_SPACING);
            genesis.btc_relay.disable_difficulty_check = true;

            genesis
        },
        Vec::new(),
        None,
        None,
        None,
        Some(testnet_properties(BITCOIN_TESTNET)),
        Extensions {
            relay_chain: "staging".into(),
            para_id: crate::chain_spec::interlay::PARA_ID.into(),
        },
    )
}
