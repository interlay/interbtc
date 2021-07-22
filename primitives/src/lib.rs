#![cfg_attr(not(feature = "std"), no_std)]

use bitcoin::{Address as BtcAddress, PublicKey as BtcPublicKey};
use bstringify::bstringify;
use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};
pub use sp_core::H256;
pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;
use sp_runtime::{
    generic,
    traits::{BlakeTwo256, IdentifyAccount, Verify},
    FixedI128, FixedU128, MultiSignature, RuntimeDebug,
};
use sp_std::{
    convert::{Into, TryFrom},
    prelude::*,
};

pub use bitcoin::types::H256Le;

pub mod issue {
    use super::*;

    #[derive(Encode, Decode, Clone, PartialEq)]
    #[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize))]
    pub enum IssueRequestStatus {
        Pending,
        /// optional refund ID
        Completed(Option<H256>),
        Cancelled,
    }

    impl Default for IssueRequestStatus {
        fn default() -> Self {
            IssueRequestStatus::Pending
        }
    }

    // Due to a known bug in serde we need to specify how u128 is (de)serialized.
    // See https://github.com/paritytech/substrate/issues/4641
    #[derive(Encode, Decode, Default, Clone, PartialEq)]
    #[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize))]
    pub struct IssueRequest<AccountId, BlockNumber, Wrapped, Collateral> {
        pub vault: AccountId,
        pub opentime: BlockNumber,
        pub period: BlockNumber,
        #[cfg_attr(feature = "std", serde(bound(deserialize = "Collateral: std::str::FromStr")))]
        #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
        #[cfg_attr(feature = "std", serde(bound(serialize = "Collateral: std::fmt::Display")))]
        #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
        pub griefing_collateral: Collateral,
        #[cfg_attr(feature = "std", serde(bound(deserialize = "Wrapped: std::str::FromStr")))]
        #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
        #[cfg_attr(feature = "std", serde(bound(serialize = "Wrapped: std::fmt::Display")))]
        #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
        /// the number of tokens that will be transfered to the user (as such, this does not include the fee)
        pub amount: Wrapped,
        #[cfg_attr(feature = "std", serde(bound(deserialize = "Wrapped: std::str::FromStr")))]
        #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
        #[cfg_attr(feature = "std", serde(bound(serialize = "Wrapped: std::fmt::Display")))]
        #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
        /// the number of tokens that will be tranferred to the fee pool
        pub fee: Wrapped,
        pub requester: AccountId,
        pub btc_address: BtcAddress,
        pub btc_public_key: BtcPublicKey,
        pub btc_height: u32,
        pub status: IssueRequestStatus,
    }
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

pub mod redeem {
    use super::*;

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
}

pub mod refund {
    use super::*;

    // Due to a known bug in serde we need to specify how u128 is (de)serialized.
    // See https://github.com/paritytech/substrate/issues/4641
    #[derive(Encode, Decode, Default, Clone, PartialEq)]
    #[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize))]
    pub struct RefundRequest<AccountId, Wrapped> {
        pub vault: AccountId,
        #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
        #[cfg_attr(feature = "std", serde(bound(serialize = "Wrapped: std::fmt::Display")))]
        #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
        pub amount_wrapped: Wrapped,
        #[cfg_attr(feature = "std", serde(bound(deserialize = "Wrapped: std::str::FromStr")))]
        #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
        #[cfg_attr(feature = "std", serde(bound(serialize = "Wrapped: std::fmt::Display")))]
        #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
        pub fee: Wrapped,
        #[cfg_attr(feature = "std", serde(bound(deserialize = "Wrapped: std::str::FromStr")))]
        #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
        #[cfg_attr(feature = "std", serde(bound(serialize = "Wrapped: std::fmt::Display")))]
        #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
        pub amount_btc: Wrapped,
        pub issuer: AccountId,
        pub btc_address: BtcAddress,
        pub issue_id: H256,
        pub completed: bool,
    }
}

pub mod replace {
    use super::*;

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
    pub struct ReplaceRequest<AccountId, BlockNumber, Wrapped, Collateral> {
        pub old_vault: AccountId,
        pub new_vault: AccountId,
        #[cfg_attr(feature = "std", serde(bound(deserialize = "Wrapped: std::str::FromStr")))]
        #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
        #[cfg_attr(feature = "std", serde(bound(serialize = "Wrapped: std::fmt::Display")))]
        #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
        pub amount: Wrapped,
        #[cfg_attr(feature = "std", serde(bound(deserialize = "Collateral: std::str::FromStr")))]
        #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
        #[cfg_attr(feature = "std", serde(bound(serialize = "Collateral: std::fmt::Display")))]
        #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
        pub griefing_collateral: Collateral,
        #[cfg_attr(feature = "std", serde(bound(deserialize = "Collateral: std::str::FromStr")))]
        #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
        #[cfg_attr(feature = "std", serde(bound(serialize = "Collateral: std::fmt::Display")))]
        #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
        pub collateral: Collateral,
        pub accept_time: BlockNumber,
        pub period: BlockNumber,
        pub btc_address: BtcAddress,
        pub btc_height: u32,
        pub status: ReplaceRequestStatus,
    }
}

pub mod oracle {
    use super::*;

    #[derive(Encode, Decode, Clone, Eq, PartialEq, Debug)]
    pub enum BitcoinInclusionTime {
        Fast,
        Half,
        Hour,
    }

    #[derive(Encode, Decode, Clone, Eq, PartialEq, Debug)]
    pub enum Key {
        // needs to be something that other pallets can pass. However, how can rust know
        // how T is related to the other pallets?
        ExchangeRate(CurrencyId),
        FeeEstimation(BitcoinInclusionTime),
    }
}

/// An index to a block.
pub type BlockNumber = u32;

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// The type for looking up accounts. We don't expect more than 4 billion of them, but you
/// never know...
pub type AccountIndex = u32;

/// Index of a transaction in the chain. 32-bit should be plenty.
pub type Nonce = u32;

/// Balance of an account.
pub type Balance = u128;

/// Signed version of Balance
pub type Amount = i128;

/// Index of a transaction in the chain.
pub type Index = u32;

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

/// An instant or duration in time.
pub type Moment = u64;

/// Digest item type.
pub type DigestItem = generic::DigestItem<Hash>;

/// Opaque block header type.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;

/// Opaque block type.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;

/// Opaque block identifier type.
pub type BlockId = generic::BlockId<Block>;

/// The signed fixed point type.
pub type SignedFixedPoint = FixedI128;

/// The `Inner` type of the `SignedFixedPoint`.
pub type SignedInner = i128;

/// The unsigned fixed point type.
pub type UnsignedFixedPoint = FixedU128;

/// The `Inner` type of the `UnsignedFixedPoint`.
pub type UnsignedInner = u128;

macro_rules! create_currency_id {
    ($(#[$meta:meta])*
	$vis:vis enum CurrencyId {
        $($(#[$vmeta:meta])* $symbol:ident($name:expr, $deci:literal),)*
    }) => {
		$(#[$meta])*
		$vis enum CurrencyId {
			$($(#[$vmeta])* $symbol,)*
		}

        $(pub const $symbol: CurrencyId = CurrencyId::$symbol;)*

		impl TryFrom<Vec<u8>> for CurrencyId {
			type Error = ();
			fn try_from(v: Vec<u8>) -> Result<CurrencyId, ()> {
				match v.as_slice() {
					$(bstringify!($symbol) => Ok(CurrencyId::$symbol),)*
					_ => Err(()),
				}
			}
		}

    }
}

create_currency_id! {
    #[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord)]
    #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
    pub enum CurrencyId {
        DOT("Polkadot", 10),
        KSM("Kusama", 12),
        INTERBTC("interBTC", 8),
        INTR("Interlay", 10),
    }
}
