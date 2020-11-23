use btc_relay::BtcPayload;
use codec::{Decode, Encode};
use frame_support::traits::Currency;

#[cfg(feature = "std")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub(crate) type DOT<T> =
    <<T as collateral::Trait>::DOT as Currency<<T as frame_system::Trait>::AccountId>>::Balance;

pub(crate) type PolkaBTC<T> =
    <<T as treasury::Trait>::PolkaBTC as Currency<<T as frame_system::Trait>::AccountId>>::Balance;

// Due to a known bug in serde we need to specify how u128 is (de)serialized.
// See https://github.com/paritytech/substrate/issues/4641
#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize))]
pub struct RedeemRequest<AccountId, BlockNumber, PolkaBTC, DOT> {
    pub vault: AccountId,
    pub opentime: BlockNumber,
    #[cfg_attr(
        feature = "std",
        serde(bound(deserialize = "PolkaBTC: std::str::FromStr"))
    )]
    #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
    #[cfg_attr(
        feature = "std",
        serde(bound(serialize = "PolkaBTC: std::fmt::Display"))
    )]
    #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
    pub amount_polka_btc: PolkaBTC,
    #[cfg_attr(
        feature = "std",
        serde(bound(deserialize = "PolkaBTC: std::str::FromStr"))
    )]
    #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
    #[cfg_attr(
        feature = "std",
        serde(bound(serialize = "PolkaBTC: std::fmt::Display"))
    )]
    #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
    pub amount_btc: PolkaBTC,
    #[cfg_attr(feature = "std", serde(bound(deserialize = "DOT: std::str::FromStr")))]
    #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
    #[cfg_attr(feature = "std", serde(bound(serialize = "DOT: std::fmt::Display")))]
    #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
    pub amount_dot: DOT,
    #[cfg_attr(feature = "std", serde(bound(deserialize = "DOT: std::str::FromStr")))]
    #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
    #[cfg_attr(feature = "std", serde(bound(serialize = "DOT: std::fmt::Display")))]
    #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
    pub premium_dot: DOT,
    pub redeemer: AccountId,
    pub btc_address: BtcPayload,
    pub completed: bool,
}

#[cfg(feature = "std")]
fn serialize_as_string<S: Serializer, T: std::fmt::Display>(
    t: &T,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&t.to_string())
}

#[cfg(feature = "std")]
fn deserialize_from_string<'de, D: Deserializer<'de>, T: std::str::FromStr>(
    deserializer: D,
) -> Result<T, D::Error> {
    let s = String::deserialize(deserializer)?;
    s.parse::<T>()
        .map_err(|_| serde::de::Error::custom("Parse from string failed"))
}
