use frame_support::dispatch::DispatchError;

pub trait CurrencyConversion<Amount, CurrencyId> {
    fn convert(amount: &Amount, to: CurrencyId) -> Result<Amount, DispatchError>;
}
