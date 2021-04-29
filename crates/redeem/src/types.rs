use btc_relay::BtcAddress;
use codec::{Decode, Encode};
use frame_support::traits::Currency;
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
    /// ActiveBlockNumber, btc_height
    V3,
}

pub(crate) type Backing<T> = <<T as currency::Config<currency::Instance1>>::Currency as Currency<
    <T as frame_system::Config>::AccountId,
>>::Balance;

pub(crate) type Issuing<T> = <<T as currency::Config<currency::Instance2>>::Currency as Currency<
    <T as frame_system::Config>::AccountId,
>>::Balance;

#[derive(Encode, Decode, Clone, PartialEq)]
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
pub struct RedeemRequest<AccountId, BlockNumber, Issuing, Backing> {
    pub vault: AccountId,
    pub opentime: BlockNumber,
    pub period: BlockNumber,
    #[cfg_attr(feature = "std", serde(bound(deserialize = "Issuing: std::str::FromStr")))]
    #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
    #[cfg_attr(feature = "std", serde(bound(serialize = "Issuing: std::fmt::Display")))]
    #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
    /// Total redeem fees in issuance - taken from request amount
    pub fee: Issuing,
    #[cfg_attr(feature = "std", serde(bound(deserialize = "Issuing: std::str::FromStr")))]
    #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
    #[cfg_attr(feature = "std", serde(bound(serialize = "Issuing: std::fmt::Display")))]
    #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
    /// Total amount of BTC for the vault to send
    pub amount_btc: Issuing,
    #[cfg_attr(feature = "std", serde(bound(deserialize = "Backing: std::str::FromStr")))]
    #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
    #[cfg_attr(feature = "std", serde(bound(serialize = "Backing: std::fmt::Display")))]
    #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
    /// Premium redeem amount in collateral
    pub premium_dot: Backing,
    pub redeemer: AccountId,
    pub btc_address: BtcAddress,
    /// The latest Bitcoin height as reported by the BTC-Relay at time of opening.
    pub btc_height: u32,
    pub status: RedeemRequestStatus,
}

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize))]
pub struct RedeemRequestV2<AccountId, BlockNumber, Issuing, Backing> {
    pub vault: AccountId,
    pub opentime: BlockNumber,
    #[cfg_attr(feature = "std", serde(bound(deserialize = "Issuing: std::str::FromStr")))]
    #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
    #[cfg_attr(feature = "std", serde(bound(serialize = "Issuing: std::fmt::Display")))]
    #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
    /// Total redeem fees in issuance - taken from request amount
    pub fee: Issuing,
    #[cfg_attr(feature = "std", serde(bound(deserialize = "Issuing: std::str::FromStr")))]
    #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
    #[cfg_attr(feature = "std", serde(bound(serialize = "Issuing: std::fmt::Display")))]
    #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
    /// Total amount of BTC for the vault to send
    pub amount_btc: Issuing,
    #[cfg_attr(feature = "std", serde(bound(deserialize = "Backing: std::str::FromStr")))]
    #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
    #[cfg_attr(feature = "std", serde(bound(serialize = "Backing: std::fmt::Display")))]
    #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
    /// Premium redeem amount in collateral
    pub premium_dot: Backing,
    pub redeemer: AccountId,
    pub btc_address: BtcAddress,
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
