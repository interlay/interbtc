use crate::*;
use dex_stable::traits::{StablePair, StableSwapMode};
use sp_runtime::{FixedPointNumber, FixedPointOperand, FixedU128};
use sp_std::vec;

const MAX_HOPS: u32 = 4;

#[derive(Clone, Debug, PartialEq)]
pub struct Amount<Balance, CurrencyId> {
    value: Balance,
    currency: CurrencyId,
}

impl<Balance, CurrencyId> Amount<Balance, CurrencyId> {
    pub fn new(value: Balance, currency: CurrencyId) -> Self {
        Self { value, currency }
    }
}

pub trait DexQuery<Balance, CurrencyId> {
    type Path;
    fn path_of(&self, input: &CurrencyId) -> Self::Path;
    fn contains(&self, currency: &CurrencyId) -> bool;
    fn get_output_amount(&self, amount_in: &Amount<Balance, CurrencyId>) -> Option<Amount<Balance, CurrencyId>>;
}

#[derive(Clone, Debug)]
pub(crate) enum DexTradingPair<T: Config> {
    Stable(StablePair<T::StablePoolId, T::CurrencyId>),
    Normal(T::CurrencyId, T::CurrencyId),
}

impl<T: Config> DexQuery<T::Balance, T::CurrencyId> for DexTradingPair<T> {
    type Path = Route<T::StablePoolId, T::CurrencyId>;

    fn path_of(&self, input: &T::CurrencyId) -> Self::Path {
        match self {
            Self::Stable(stable_pair) => Self::Path::Stable(stable_pair.clone().path_of(input.clone())),
            Self::Normal(token0, token1) => Self::Path::Normal(vec![
                input.clone(),
                if input == token0 { token1 } else { token0 }.clone(),
            ]),
        }
    }

    fn contains(&self, currency: &T::CurrencyId) -> bool {
        match self {
            Self::Stable(stable_pair) => &stable_pair.token0 == currency || &stable_pair.token1 == currency,
            Self::Normal(token0, token1) => token0 == currency || token1 == currency,
        }
    }

    fn get_output_amount(
        &self,
        amount_in: &Amount<T::Balance, T::CurrencyId>,
    ) -> Option<Amount<T::Balance, T::CurrencyId>> {
        Some(match self {
            Self::Stable(stable_pair) => {
                let stable_path = stable_pair.clone().path_of(amount_in.currency);
                let amount_out = match stable_path.mode {
                    StableSwapMode::Single => T::StableAMM::stable_amm_calculate_swap_amount(
                        stable_path.pool_id,
                        T::StableAMM::currency_index(stable_path.pool_id, stable_path.from_currency)?,
                        T::StableAMM::currency_index(stable_path.pool_id, stable_path.to_currency)?,
                        amount_in.value,
                    )?,
                    StableSwapMode::FromBase => T::StableAMM::stable_amm_calculate_swap_amount_from_base(
                        stable_path.pool_id,
                        stable_path.base_pool_id,
                        T::StableAMM::currency_index(stable_path.base_pool_id, stable_path.from_currency)?,
                        T::StableAMM::currency_index(stable_path.pool_id, stable_path.to_currency)?,
                        amount_in.value,
                    )
                    .ok()??,
                    StableSwapMode::ToBase => T::StableAMM::stable_amm_calculate_swap_amount_from_base(
                        stable_path.pool_id,
                        stable_path.base_pool_id,
                        T::StableAMM::currency_index(stable_path.pool_id, stable_path.from_currency)?,
                        T::StableAMM::currency_index(stable_path.base_pool_id, stable_path.to_currency)?,
                        amount_in.value,
                    )
                    .ok()??,
                };
                Amount {
                    value: amount_out,
                    currency: stable_path.to_currency,
                }
            }
            Self::Normal(token0, token1) => {
                let output_currency = if amount_in.currency == *token0 { token1 } else { token0 };
                let amounts = T::NormalAmm::get_amount_out_by_path(
                    amount_in.value.into(),
                    &vec![amount_in.currency, output_currency.clone()],
                )
                .ok()?;
                let amount_out = T::Balance::from(*amounts.last()?);
                Amount {
                    value: amount_out,
                    currency: output_currency.clone(),
                }
            }
        })
    }
}

#[derive(Debug, PartialEq)]
struct Trade<Balance, CurrencyId, TradingPath> {
    input_amount: Amount<Balance, CurrencyId>,
    output_amount: Amount<Balance, CurrencyId>,
    execution_price: FixedU128,
    path: Vec<TradingPath>,
}

impl<Balance, CurrencyId, TradingPath> Trade<Balance, CurrencyId, TradingPath>
where
    Balance: FixedPointOperand,
    CurrencyId: PartialEq,
{
    fn new(
        input_amount: Amount<Balance, CurrencyId>,
        output_amount: Amount<Balance, CurrencyId>,
        path: Vec<TradingPath>,
    ) -> Self {
        let execution_price =
            FixedU128::checked_from_rational(output_amount.value, input_amount.value).unwrap_or_default();
        Self {
            input_amount,
            output_amount,
            execution_price,
            path,
        }
    }

    // https://github.com/zenlinkpro/dex-sdk/blob/bba0310df15893913f31c999da9aca71f0bf152c/packages/sdk-router/src/SmartRouterV2.ts#L11
    fn is_better(&self, maybe_other: &Option<Self>) -> bool {
        if let Some(other) = maybe_other {
            if !self.input_amount.currency.eq(&other.input_amount.currency)
                || !self.output_amount.currency.eq(&other.output_amount.currency)
            {
                // TODO: should we return an error here?
                false
            } else {
                // TODO: compare price impact?
                self.execution_price.gt(&other.execution_price)
            }
        } else {
            true
        }
    }
}

struct TradeFinder<Balance, CurrencyId, TradingPair> {
    input_amount: Amount<Balance, CurrencyId>,
    output_currency: CurrencyId,
    pairs: Vec<TradingPair>,
}

impl<Balance, CurrencyId, TradingPair> TradeFinder<Balance, CurrencyId, TradingPair>
where
    Balance: Clone + FixedPointOperand,
    CurrencyId: Clone + PartialEq,
    TradingPair: Clone + DexQuery<Balance, CurrencyId>,
    <TradingPair as rpc::DexQuery<Balance, CurrencyId>>::Path: Clone,
{
    pub fn new(
        input_amount: Amount<Balance, CurrencyId>,
        output_currency: CurrencyId,
        pairs: Vec<TradingPair>,
    ) -> Self {
        Self {
            input_amount,
            output_currency,
            pairs,
        }
    }

    pub fn find_best_trade(
        self,
    ) -> Option<Trade<Balance, CurrencyId, <TradingPair as DexQuery<Balance, CurrencyId>>::Path>> {
        // TODO: support routing by exact out
        self.find_best_trade_exact_in(self.input_amount.clone(), self.pairs.clone(), vec![], MAX_HOPS)
    }

    fn find_best_trade_exact_in(
        &self,
        input_amount: Amount<Balance, CurrencyId>,
        pairs: Vec<TradingPair>,
        path: Vec<<TradingPair as DexQuery<Balance, CurrencyId>>::Path>,
        hop_limit: u32,
    ) -> Option<Trade<Balance, CurrencyId, <TradingPair as DexQuery<Balance, CurrencyId>>::Path>> {
        if hop_limit == 0 {
            return None;
        }
        let mut best_trade = None;

        for (i, pair) in pairs.iter().enumerate() {
            if !pair.contains(&input_amount.currency) {
                continue;
            }
            let output_amount = if let Some(output_amount) = pair.get_output_amount(&input_amount) {
                output_amount
            } else {
                continue;
            };
            if output_amount.value.is_zero() {
                continue;
            }

            let current_path = [&path[..], &[pair.path_of(&input_amount.currency)]].concat();
            if self.output_currency == output_amount.currency {
                let trade = Trade::new(self.input_amount.clone(), output_amount, current_path);
                if trade.is_better(&best_trade) {
                    best_trade = Some(trade);
                }
            } else {
                // pairs excluding this pair
                let mut pairs = self.pairs.clone();
                pairs.remove(i);

                if let Some(trade) = self.find_best_trade_exact_in(output_amount, pairs, current_path, hop_limit - 1) {
                    if trade.is_better(&best_trade) {
                        best_trade = Some(trade);
                    }
                }
            }
        }

        best_trade
    }
}

impl<T: Config> Pallet<T> {
    pub(crate) fn get_all_trading_pairs() -> Vec<DexTradingPair<T>> {
        T::StableAMM::get_all_trading_pairs()
            .into_iter()
            .map(DexTradingPair::Stable)
            .chain(
                T::NormalAmm::get_all_trading_pairs()
                    .into_iter()
                    .map(|(token0, token1)| DexTradingPair::Normal(token0, token1)),
            )
            .collect()
    }

    pub fn find_best_trade_exact_in(
        input_amount: T::Balance,
        input_currency: T::CurrencyId,
        output_currency: T::CurrencyId,
    ) -> Option<(T::Balance, Vec<Route<T::StablePoolId, T::CurrencyId>>)> {
        let trade = TradeFinder::new(
            Amount::new(input_amount, input_currency),
            output_currency,
            Self::get_all_trading_pairs(),
        )
        .find_best_trade()?;
        Some((trade.output_amount.value, trade.path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mock::{CurrencyId, CurrencyId::Token};

    #[derive(Clone, Debug)]
    struct UniswapPair {
        token0: CurrencyId,
        token1: CurrencyId,
        reserve0: u128,
        reserve1: u128,
    }

    impl UniswapPair {
        fn new(token0: CurrencyId, token1: CurrencyId, reserve_in: u128, amount_in: u128, amount_out: u128) -> Self {
            Self {
                token0,
                token1,
                reserve0: reserve_in,
                // (amountIn * reserveOut) /  (reserveIn + amountIn) = amountOut
                // (amountIn * reserveOut) = amountOut * (reserveIn + amountIn)
                // reserveOut = (amountOut * (reserveIn + amountIn)) / amountIn
                reserve1: (amount_out * (reserve_in + amount_in)) / amount_in,
            }
        }

        fn output_of(&self, currency_id: &CurrencyId) -> CurrencyId {
            if currency_id == &self.token0 {
                self.token1
            } else {
                self.token0
            }
        }

        fn reserve_of(&self, currency_id: &CurrencyId) -> u128 {
            if currency_id == &self.token0 {
                self.reserve0
            } else {
                self.reserve1
            }
        }
    }

    impl DexQuery<u128, CurrencyId> for UniswapPair {
        type Path = (CurrencyId, CurrencyId);

        fn path_of(&self, input: &CurrencyId) -> Self::Path {
            (input.clone(), self.output_of(input))
        }

        fn contains(&self, currency: &CurrencyId) -> bool {
            &self.token0 == currency || &self.token1 == currency
        }

        fn get_output_amount(&self, amount_in: &Amount<u128, CurrencyId>) -> Option<Amount<u128, CurrencyId>> {
            let input_reserve = self.reserve_of(&amount_in.currency);
            let output_reserve = self.reserve_of(&self.output_of(&amount_in.currency));

            let numerator = amount_in.value * output_reserve;
            let denominator = input_reserve + amount_in.value;
            let amount_out = numerator / denominator;

            if amount_out > output_reserve {
                None
            } else {
                Some(Amount {
                    value: numerator / denominator,
                    currency: self.output_of(&amount_in.currency),
                })
            }
        }
    }

    const KBTC: CurrencyId = Token(0);
    const USDT: CurrencyId = Token(1);
    const KSM: CurrencyId = Token(2);

    #[test]
    fn test_set_price() {
        assert_eq!(
            UniswapPair::new(KBTC, USDT, 100_000_000_000, 100_000_000, 24_804_129_249).get_output_amount(&Amount {
                value: 100_000_000,
                currency: KBTC,
            }),
            Some(Amount {
                value: 24_804_129_249,
                currency: USDT,
            })
        );
    }

    #[test]
    fn test_find_best_trade() {
        // KBTC/USDT = 24732.363767
        let kbtc_usdt = UniswapPair {
            token0: KBTC,
            token1: USDT,
            reserve0: 2_703_163_325,
            reserve1: 695_300_254_200,
        };

        // KBTC/KSM = 279.792625403109
        let kbtc_ksm = UniswapPair {
            token0: KBTC,
            token1: KSM,
            reserve0: 2_448_716_486,
            reserve1: 7_151_736_602_191_629,
        };

        assert_eq!(
            kbtc_ksm.get_output_amount(&Amount {
                value: 1000000000000,
                currency: KSM,
            }),
            Some(Amount {
                value: 342346,
                currency: KBTC,
            })
        );

        // USDT/KSM = 0.027655481836
        let usdt_ksm = UniswapPair {
            token0: USDT,
            token1: KSM,
            reserve0: 50_000_000_000,
            reserve1: 1_386_962_552_011_095,
        };

        // KSM/USDT * USDT/KBTC = 36.159196 * 0.00003887 = 0.0014055
        // KSM/KBTC = 0.00342346
        assert_eq!(
            TradeFinder::new(
                Amount::new(1000000000000, KSM),
                KBTC,
                vec![kbtc_ksm.clone(), kbtc_usdt.clone(), usdt_ksm]
            )
            .find_best_trade()
            .unwrap()
            .output_amount,
            Amount {
                value: 342346,
                currency: KBTC,
            }
        );

        let ksm_usdt = UniswapPair::new(KSM, USDT, 100_000_000_000_000, 1_000_000_000_000, 100_000_000);

        // KSM/USDT * USDT/KBTC = 100 * 0.00003887 = 0.003887
        // KSM/KBTC = 0.00342346
        assert_eq!(
            TradeFinder::new(
                Amount::new(1000000000000, KSM),
                KBTC,
                vec![kbtc_ksm, kbtc_usdt, ksm_usdt]
            )
            .find_best_trade()
            .unwrap()
            .output_amount,
            Amount {
                value: 388720,
                currency: KBTC,
            }
        );
    }

    #[test]
    fn test_find_best_trade_insufficient_liquidity() {
        assert_eq!(
            TradeFinder::new(
                Amount::new(100, Token(0)),
                Token(1),
                vec![UniswapPair {
                    token0: Token(0),
                    token1: Token(1),
                    reserve0: 0,
                    reserve1: 0,
                }]
            )
            .find_best_trade(),
            None
        );
    }
}
