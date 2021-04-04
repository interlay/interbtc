use btc_relay::BtcAddress;
use codec::{Decode, Encode};
use frame_support::traits::Currency;
#[cfg(feature = "std")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sp_core::H160;

/// Storage version.
#[derive(Encode, Decode, Eq, PartialEq)]
pub enum Version {
    /// Initial version.
    V0,
    /// BtcAddress type with script format.
    V1,
    /// Status, make all fields non-optional, remove open_time
    V2,
    /// active block number, open_bitcoin_height
    V3,
}

pub(crate) type DOT<T> = <<T as collateral::Config>::DOT as Currency<<T as frame_system::Config>::AccountId>>::Balance;
pub(crate) type PolkaBTC<T> =
    <<T as treasury::Config>::PolkaBTC as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[derive(Encode, Decode, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize))]
pub enum ReplaceRequestStatus {
    Pending,
    Completed,
    Cancelled,
}

impl Default for ReplaceRequestStatus {
    fn default() -> Self {
        ReplaceRequestStatus::Pending
    }
}
// Due to a known bug in serde we need to specify how u128 is (de)serialized.
// See https://github.com/paritytech/substrate/issues/4641
#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize))]
pub struct ReplaceRequest<AccountId, BlockNumber, PolkaBTC, DOT> {
    pub old_vault: AccountId,
    pub new_vault: AccountId,
    #[cfg_attr(feature = "std", serde(bound(deserialize = "PolkaBTC: std::str::FromStr")))]
    #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
    #[cfg_attr(feature = "std", serde(bound(serialize = "PolkaBTC: std::fmt::Display")))]
    #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
    pub amount: PolkaBTC,
    #[cfg_attr(feature = "std", serde(bound(deserialize = "DOT: std::str::FromStr")))]
    #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
    #[cfg_attr(feature = "std", serde(bound(serialize = "DOT: std::fmt::Display")))]
    #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
    pub griefing_collateral: DOT,
    #[cfg_attr(feature = "std", serde(bound(deserialize = "DOT: std::str::FromStr")))]
    #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
    #[cfg_attr(feature = "std", serde(bound(serialize = "DOT: std::fmt::Display")))]
    #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
    pub collateral: DOT,
    pub accept_time: BlockNumber,
    pub period: BlockNumber,
    pub btc_address: BtcAddress,
    pub open_bitcoin_height: u32,
    pub status: ReplaceRequestStatus,
}

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize))]
pub struct ReplaceRequestV2<AccountId, BlockNumber, PolkaBTC, DOT> {
    pub old_vault: AccountId,
    pub new_vault: AccountId,
    #[cfg_attr(feature = "std", serde(bound(deserialize = "PolkaBTC: std::str::FromStr")))]
    #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
    #[cfg_attr(feature = "std", serde(bound(serialize = "PolkaBTC: std::fmt::Display")))]
    #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
    pub amount: PolkaBTC,
    #[cfg_attr(feature = "std", serde(bound(deserialize = "DOT: std::str::FromStr")))]
    #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
    #[cfg_attr(feature = "std", serde(bound(serialize = "DOT: std::fmt::Display")))]
    #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
    pub griefing_collateral: DOT,
    #[cfg_attr(feature = "std", serde(bound(deserialize = "DOT: std::str::FromStr")))]
    #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
    #[cfg_attr(feature = "std", serde(bound(serialize = "DOT: std::fmt::Display")))]
    #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
    pub collateral: DOT,
    pub accept_time: BlockNumber,
    pub btc_address: BtcAddress,
    pub status: ReplaceRequestStatus,
}

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize))]
pub struct ReplaceRequestV1<AccountId, BlockNumber, PolkaBTC, DOT> {
    pub old_vault: AccountId,
    pub open_time: BlockNumber,
    #[cfg_attr(feature = "std", serde(bound(deserialize = "PolkaBTC: std::str::FromStr")))]
    #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
    #[cfg_attr(feature = "std", serde(bound(serialize = "PolkaBTC: std::fmt::Display")))]
    #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
    pub amount: PolkaBTC,
    #[cfg_attr(feature = "std", serde(bound(deserialize = "DOT: std::str::FromStr")))]
    #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
    #[cfg_attr(feature = "std", serde(bound(serialize = "DOT: std::fmt::Display")))]
    #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
    pub griefing_collateral: DOT,
    pub new_vault: Option<AccountId>,
    #[cfg_attr(feature = "std", serde(bound(deserialize = "DOT: std::str::FromStr")))]
    #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
    #[cfg_attr(feature = "std", serde(bound(serialize = "DOT: std::fmt::Display")))]
    #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
    pub collateral: DOT,
    pub accept_time: Option<BlockNumber>,
    pub btc_address: Option<BtcAddress>,
    pub completed: bool,
    pub cancelled: bool,
}

// todo: serialize_as_string deserialize_from_string are defined multiple times
// throughout the code; Maybe this should be merged.. Although it should only
// be only a temporary workaround)

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

#[derive(Encode, Decode, Default, Clone, PartialEq)]
pub(crate) struct ReplaceRequestV0<AccountId, BlockNumber, PolkaBTC, DOT> {
    pub old_vault: AccountId,
    pub open_time: BlockNumber,
    pub amount: PolkaBTC,
    pub griefing_collateral: DOT,
    pub new_vault: Option<AccountId>,
    pub collateral: DOT,
    pub accept_time: Option<BlockNumber>,
    pub btc_address: H160,
    pub completed: bool,
}
