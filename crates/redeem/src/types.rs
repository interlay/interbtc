use btc_relay::BtcAddress;
use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Storage version.
#[derive(Encode, Decode, Eq, PartialEq)]
pub enum Version {
    /// Initial version.
    V0,
    /// BtcAddress type with script format.
    V1,
    /// RedeemRequestStatus, removed amount_dot and amount_polka_btc
    V2,
    /// ActiveBlockNumber, btc_height, transfer_fee_btc
    V3,
}

pub(crate) type BalanceOf<T> = <T as vault_registry::Config>::Balance;

pub(crate) type Collateral<T> = BalanceOf<T>;

pub(crate) type Wrapped<T> = BalanceOf<T>;

#[derive(Encode, Decode, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize))]
pub enum RedeemRequestStatus {
    Pending,
    Completed,
    /// bool=true indicates that the vault minted tokens for the amount that the redeemer burned
    Reimbursed(bool),
    Retried,
}

impl Default for RedeemRequestStatus {
    fn default() -> Self {
        RedeemRequestStatus::Pending
    }
}

// Due to a known bug in serde we need to specify how u128 is (de)serialized.
// See https://github.com/paritytech/substrate/issues/4641
#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize))]
pub struct RedeemRequest<AccountId, BlockNumber, Wrapped, Collateral> {
    pub vault: AccountId,
    pub opentime: BlockNumber,
    pub period: BlockNumber,

    #[cfg_attr(feature = "std", serde(bound(deserialize = "Wrapped: std::str::FromStr")))]
    #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
    #[cfg_attr(feature = "std", serde(bound(serialize = "Wrapped: std::fmt::Display")))]
    #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
    /// Total redeem fees in issuance - taken from request amount
    pub fee: Wrapped,

    #[cfg_attr(feature = "std", serde(bound(deserialize = "Wrapped: std::str::FromStr")))]
    #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
    #[cfg_attr(feature = "std", serde(bound(serialize = "Wrapped: std::fmt::Display")))]
    #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
    /// Amount the vault should spend on the bitcoin inclusion fee - taken from request amount
    pub transfer_fee_btc: Wrapped,

    #[cfg_attr(feature = "std", serde(bound(deserialize = "Wrapped: std::str::FromStr")))]
    #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
    #[cfg_attr(feature = "std", serde(bound(serialize = "Wrapped: std::fmt::Display")))]
    #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
    /// Total amount of BTC for the vault to send
    pub amount_btc: Wrapped,

    #[cfg_attr(feature = "std", serde(bound(deserialize = "Collateral: std::str::FromStr")))]
    #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
    #[cfg_attr(feature = "std", serde(bound(serialize = "Collateral: std::fmt::Display")))]
    #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
    /// Premium redeem amount in collateral
    pub premium: Collateral,

    pub redeemer: AccountId,
    pub btc_address: BtcAddress,
    /// The latest Bitcoin height as reported by the BTC-Relay at time of opening.
    pub btc_height: u32,
    pub status: RedeemRequestStatus,
}

#[cfg(feature = "std")]
fn serialize_as_string<S: Serializer, T: std::fmt::Display>(t: &T, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&t.to_string())
}

#[cfg(feature = "std")]
fn deserialize_from_string<'de, D: Deserializer<'de>, T: std::str::FromStr>(deserializer: D) -> Result<T, D::Error> {
    let s = String::deserialize(deserializer)?;
    s.parse::<T>()
        .map_err(|_| serde::de::Error::custom("Parse from string failed"))
}
