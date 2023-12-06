#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::{vec, vec::Vec};
use bitcoin::{Address as BtcAddress, PublicKey as BtcPublicKey};
use bstringify::bstringify;
use codec::{Decode, Encode, MaxEncodedLen};
use core::convert::TryFrom;
#[cfg(any(feature = "runtime-benchmarks", feature = "substrate-compat"))]
use core::convert::TryInto;
use primitive_types::H256;
#[cfg(feature = "std")]
use scale_decode::DecodeAsType;
#[cfg(feature = "std")]
use scale_encode::EncodeAsType;
use scale_info::TypeInfo;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub use bitcoin::types::H256Le;

pub const BITCOIN_TESTNET: &str = "bitcoin-testnet";
pub const BITCOIN_MAINNET: &str = "bitcoin-mainnet";
pub const BITCOIN_REGTEST: &str = "bitcoin-regtest";

#[cfg(feature = "substrate-compat")]
pub use arithmetic::*;

#[cfg(feature = "substrate-compat")]
mod arithmetic {
    use super::*;
    use sp_runtime::{FixedI128, FixedPointNumber, FixedU128};

    /// The signed fixed point type.
    pub type SignedFixedPoint = FixedI128;

    /// The `Inner` type of the `SignedFixedPoint`.
    pub type SignedInner = <FixedI128 as FixedPointNumber>::Inner;

    /// The unsigned fixed point type.
    pub type UnsignedFixedPoint = FixedU128;

    /// The `Inner` type of the `UnsignedFixedPoint`.
    pub type UnsignedInner = <FixedU128 as FixedPointNumber>::Inner;

    pub trait BalanceToFixedPoint<FixedPoint> {
        fn to_fixed(self) -> Option<FixedPoint>;
    }

    impl BalanceToFixedPoint<SignedFixedPoint> for Balance {
        fn to_fixed(self) -> Option<SignedFixedPoint> {
            SignedFixedPoint::checked_from_integer(
                TryInto::<<SignedFixedPoint as FixedPointNumber>::Inner>::try_into(self).ok()?,
            )
        }
    }

    pub trait TruncateFixedPointToInt: FixedPointNumber {
        /// take a fixed point number and turns it into the truncated inner representation,
        /// e.g. FixedU128(1.23) -> 1u128
        fn truncate_to_inner(&self) -> Option<<Self as FixedPointNumber>::Inner>;
    }

    impl TruncateFixedPointToInt for SignedFixedPoint {
        fn truncate_to_inner(&self) -> Option<Self::Inner> {
            self.into_inner().checked_div(SignedFixedPoint::accuracy())
        }
    }

    impl TruncateFixedPointToInt for UnsignedFixedPoint {
        fn truncate_to_inner(&self) -> Option<<Self as FixedPointNumber>::Inner> {
            self.into_inner().checked_div(UnsignedFixedPoint::accuracy())
        }
    }
}

#[derive(
    Serialize, Deserialize, Encode, Decode, Clone, PartialEq, Eq, Debug, PartialOrd, Ord, TypeInfo, MaxEncodedLen,
)]
#[cfg_attr(feature = "std", derive(std::hash::Hash))]
pub struct VaultCurrencyPair<CurrencyId: Copy> {
    pub collateral: CurrencyId,
    pub wrapped: CurrencyId,
}

#[derive(
    Serialize, Deserialize, Encode, Decode, Clone, PartialEq, Eq, Debug, PartialOrd, Ord, TypeInfo, MaxEncodedLen,
)]
#[cfg_attr(feature = "std", derive(std::hash::Hash))]
pub struct VaultId<AccountId, CurrencyId: Copy> {
    pub account_id: AccountId,
    pub currencies: VaultCurrencyPair<CurrencyId>,
}

impl<AccountId, CurrencyId: Copy> VaultId<AccountId, CurrencyId> {
    pub fn new(account_id: AccountId, collateral_currency: CurrencyId, wrapped_currency: CurrencyId) -> Self {
        Self {
            account_id,
            currencies: VaultCurrencyPair::<CurrencyId> {
                collateral: collateral_currency,
                wrapped: wrapped_currency,
            },
        }
    }

    pub fn from_pair(account_id: AccountId, currencies: VaultCurrencyPair<CurrencyId>) -> Self {
        Self { account_id, currencies }
    }

    pub fn collateral_currency(&self) -> CurrencyId {
        self.currencies.collateral
    }

    pub fn wrapped_currency(&self) -> CurrencyId {
        self.currencies.wrapped
    }
}

impl<AccountId, CurrencyId: Copy> From<(AccountId, VaultCurrencyPair<CurrencyId>)> for VaultId<AccountId, CurrencyId> {
    fn from((account_id, currencies): (AccountId, VaultCurrencyPair<CurrencyId>)) -> Self {
        VaultId::new(account_id, currencies.collateral, currencies.wrapped)
    }
}

pub mod issue {
    use super::*;

    #[derive(Serialize, Deserialize, Encode, Decode, Clone, PartialEq, TypeInfo, MaxEncodedLen)]
    #[cfg_attr(feature = "std", derive(Debug))]
    #[serde(rename_all = "camelCase")]
    pub enum IssueRequestStatus {
        /// opened, but not yet executed or cancelled
        Pending,
        /// payment was received
        Completed,
        /// payment was not received, vault may receive griefing collateral
        Cancelled,
    }

    impl Default for IssueRequestStatus {
        fn default() -> Self {
            IssueRequestStatus::Pending
        }
    }

    // Due to a known bug in serde we need to specify how u128 is (de)serialized.
    // See https://github.com/paritytech/substrate/issues/4641
    #[derive(Serialize, Deserialize, Encode, Decode, Clone, PartialEq, TypeInfo, MaxEncodedLen)]
    #[cfg_attr(feature = "std", derive(Debug))]
    pub struct IssueRequest<AccountId, BlockNumber, Balance, CurrencyId: Copy> {
        /// the vault associated with this issue request
        pub vault: VaultId<AccountId, CurrencyId>,
        /// the *active* block height when this request was opened
        pub opentime: BlockNumber,
        /// the issue period when this request was opened
        pub period: BlockNumber,
        #[cfg_attr(feature = "std", serde(bound(deserialize = "Balance: std::str::FromStr")))]
        #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
        #[cfg_attr(feature = "std", serde(bound(serialize = "Balance: std::fmt::Display")))]
        #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
        /// the collateral held for spam prevention
        pub griefing_collateral: Balance,
        /// The currency used for the griefing collateral
        pub griefing_currency: CurrencyId,
        #[cfg_attr(feature = "std", serde(bound(deserialize = "Balance: std::str::FromStr")))]
        #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
        #[cfg_attr(feature = "std", serde(bound(serialize = "Balance: std::fmt::Display")))]
        #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
        /// the number of tokens that will be transferred to the user (as such, this does not include the fee)
        pub amount: Balance,
        #[cfg_attr(feature = "std", serde(bound(deserialize = "Balance: std::str::FromStr")))]
        #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
        #[cfg_attr(feature = "std", serde(bound(serialize = "Balance: std::fmt::Display")))]
        #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
        /// the number of tokens that will be transferred to the fee pool
        pub fee: Balance,
        /// the account issuing tokens
        pub requester: AccountId,
        /// the vault's Bitcoin deposit address
        pub btc_address: BtcAddress,
        /// the vault's Bitcoin public key (when this request was made)
        pub btc_public_key: BtcPublicKey,
        /// the highest recorded height in the BTC-Relay (at time of opening)
        pub btc_height: u32,
        /// the status of this issue request
        pub status: IssueRequestStatus,
    }
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Encode, Decode, Default, TypeInfo)]
#[serde(rename_all = "camelCase")]
/// a wrapper around a balance, used in RPC to workaround a bug where using u128
/// in runtime-apis fails. See <https://github.com/paritytech/substrate/issues/4641>
pub struct BalanceWrapper<T> {
    #[cfg_attr(feature = "std", serde(bound(serialize = "T: std::fmt::Display")))]
    #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
    #[cfg_attr(feature = "std", serde(bound(deserialize = "T: std::str::FromStr")))]
    #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
    pub amount: T,
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

    #[derive(Serialize, Deserialize, Encode, Decode, Clone, Eq, PartialEq, TypeInfo, MaxEncodedLen)]
    #[cfg_attr(feature = "std", derive(Debug))]
    #[serde(rename_all = "camelCase")]
    pub enum RedeemRequestStatus {
        /// opened, but not yet executed or cancelled
        Pending,
        /// successfully executed with a valid payment from the vault
        Completed,
        /// bool=true indicates that the vault minted tokens for the amount that the redeemer burned
        Reimbursed(bool),
        /// user received compensation, but is retrying the redeem with another vault
        Retried,
    }

    impl Default for RedeemRequestStatus {
        fn default() -> Self {
            RedeemRequestStatus::Pending
        }
    }

    // Due to a known bug in serde we need to specify how u128 is (de)serialized.
    // See https://github.com/paritytech/substrate/issues/4641
    #[derive(Serialize, Deserialize, Encode, Decode, Clone, PartialEq, TypeInfo, MaxEncodedLen)]
    #[cfg_attr(feature = "std", derive(Debug))]
    pub struct RedeemRequest<AccountId, BlockNumber, Balance, CurrencyId: Copy> {
        /// the vault associated with this redeem request
        pub vault: VaultId<AccountId, CurrencyId>,
        /// the *active* block height when this request was opened
        pub opentime: BlockNumber,
        /// the redeem period when this request was opened
        pub period: BlockNumber,
        #[cfg_attr(feature = "std", serde(bound(deserialize = "Balance: std::str::FromStr")))]
        #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
        #[cfg_attr(feature = "std", serde(bound(serialize = "Balance: std::fmt::Display")))]
        #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
        /// total redeem fees - taken from request amount
        pub fee: Balance,
        #[cfg_attr(feature = "std", serde(bound(deserialize = "Balance: std::str::FromStr")))]
        #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
        #[cfg_attr(feature = "std", serde(bound(serialize = "Balance: std::fmt::Display")))]
        #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
        /// amount the vault should spend on the bitcoin inclusion fee - taken from request amount
        pub transfer_fee_btc: Balance,
        #[cfg_attr(feature = "std", serde(bound(deserialize = "Balance: std::str::FromStr")))]
        #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
        #[cfg_attr(feature = "std", serde(bound(serialize = "Balance: std::fmt::Display")))]
        #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
        /// total amount of BTC for the vault to send
        pub amount_btc: Balance,
        #[cfg_attr(feature = "std", serde(bound(deserialize = "Balance: std::str::FromStr")))]
        #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
        #[cfg_attr(feature = "std", serde(bound(serialize = "Balance: std::fmt::Display")))]
        #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
        /// premium redeem amount in collateral
        pub premium: Balance,
        /// the account redeeming tokens (for BTC)
        pub redeemer: AccountId,
        /// the user's Bitcoin address for payment verification
        pub btc_address: BtcAddress,
        /// the highest recorded height in the BTC-Relay (at time of opening)
        pub btc_height: u32,
        /// the status of this redeem request
        pub status: RedeemRequestStatus,
    }
}

pub mod replace {
    use super::*;

    #[derive(Serialize, Deserialize, Encode, Decode, Clone, PartialEq, TypeInfo, MaxEncodedLen)]
    #[cfg_attr(feature = "std", derive(Debug, Eq))]
    #[serde(rename_all = "camelCase")]
    pub enum ReplaceRequestStatus {
        /// accepted, but not yet executed or cancelled
        Pending,
        /// successfully executed with a valid payment from the old vault
        Completed,
        /// payment was not received, new vault may receive griefing collateral
        Cancelled,
    }

    impl Default for ReplaceRequestStatus {
        fn default() -> Self {
            ReplaceRequestStatus::Pending
        }
    }

    // Due to a known bug in serde we need to specify how u128 is (de)serialized.
    // See https://github.com/paritytech/substrate/issues/4641
    #[derive(Serialize, Deserialize, Encode, Decode, Clone, PartialEq, TypeInfo, MaxEncodedLen)]
    #[cfg_attr(feature = "std", derive(Eq, Debug))]
    pub struct ReplaceRequest<AccountId, BlockNumber, Balance, CurrencyId: Copy> {
        /// the vault which has requested to be replaced
        pub old_vault: VaultId<AccountId, CurrencyId>,
        /// the vault which is replacing the old vault
        pub new_vault: VaultId<AccountId, CurrencyId>,
        #[cfg_attr(feature = "std", serde(bound(deserialize = "Balance: std::str::FromStr")))]
        #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
        #[cfg_attr(feature = "std", serde(bound(serialize = "Balance: std::fmt::Display")))]
        #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
        /// the amount of tokens to be replaced
        pub amount: Balance,
        #[cfg_attr(feature = "std", serde(bound(deserialize = "Balance: std::str::FromStr")))]
        #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
        #[cfg_attr(feature = "std", serde(bound(serialize = "Balance: std::fmt::Display")))]
        #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
        /// the collateral held for spam prevention
        pub griefing_collateral: Balance,
        #[cfg_attr(feature = "std", serde(bound(deserialize = "Balance: std::str::FromStr")))]
        #[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
        #[cfg_attr(feature = "std", serde(bound(serialize = "Balance: std::fmt::Display")))]
        #[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
        /// additional collateral to cover replacement
        pub collateral: Balance,
        /// the *active* block height when this request was opened
        pub accept_time: BlockNumber,
        /// the replace period when this request was opened
        pub period: BlockNumber,
        /// the Bitcoin address of the new vault
        pub btc_address: BtcAddress,
        /// the highest recorded height in the BTC-Relay (at time of opening)
        pub btc_height: u32,
        /// the status of this replace request
        pub status: ReplaceRequestStatus,
    }
}

pub mod oracle {
    use super::*;

    #[derive(Serialize, Deserialize, Encode, Decode, Clone, Eq, PartialEq, Debug, TypeInfo, MaxEncodedLen)]
    #[serde(rename_all = "camelCase")]
    pub enum Key {
        ExchangeRate(CurrencyId),
        FeeEstimation,
    }
}

#[cfg(feature = "substrate-compat")]
pub use runtime::*;

#[cfg(feature = "substrate-compat")]
mod runtime {
    use super::*;
    use sp_runtime::{
        generic,
        traits::{BlakeTwo256, IdentifyAccount, Verify},
        MultiSignature, OpaqueExtrinsic,
    };

    /// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
    pub type Signature = MultiSignature;

    /// Some way of identifying an account on the chain. We intentionally make it equivalent
    /// to the public key of our transaction signing scheme.
    pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

    /// Opaque block header type.
    pub type Header = generic::Header<BlockNumber, BlakeTwo256>;

    /// Opaque block type.
    pub type Block = generic::Block<Header, OpaqueExtrinsic>;
}

/// An index to a block.
pub type BlockNumber = u32;

/// Index of a transaction in the chain. 32-bit should be plenty.
pub type Nonce = u32;

/// Balance of an account.
pub type Balance = u128;

/// Signed version of Balance
pub type SignedBalance = i128;

/// Index of a transaction in the chain.
pub type Index = u32;

/// A hash of some data used by the chain.
pub type Hash = H256;

/// An instant or duration in time.
pub type Moment = u64;

/// Loans pallet types
#[cfg(feature = "substrate-compat")]
pub use loans::*;

#[cfg(feature = "substrate-compat")]
mod loans {
    use super::*;
    use sp_runtime::{FixedU128, Permill};

    pub type Price = FixedU128;
    pub type Timestamp = Moment;
    pub type PriceDetail = (Price, Timestamp);
    pub type Rate = FixedU128;
    pub type Ratio = Permill;
    pub type Shortfall = FixedU128;
    pub type Liquidity = FixedU128;
    pub const SECONDS_PER_YEAR: Timestamp = 365 * 24 * 60 * 60;
}

pub trait CurrencyInfo {
    fn name(&self) -> &str;
    fn symbol(&self) -> &str;
    fn decimals(&self) -> u8;
}

macro_rules! create_currency_id {
    ($(#[$meta:meta])*
	$vis:vis enum TokenSymbol {
        $($(#[$vmeta:meta])* $symbol:ident($name:expr, $deci:literal) = $val:literal,)*
    }) => {
		$(#[$meta])*
		$vis enum TokenSymbol {
			$($(#[$vmeta])* $symbol = $val,)*
		}

        $(pub const $symbol: TokenSymbol = TokenSymbol::$symbol;)*

        impl TryFrom<u8> for TokenSymbol {
			type Error = ();

			fn try_from(v: u8) -> Result<Self, Self::Error> {
				match v {
					$($val => Ok(TokenSymbol::$symbol),)*
					_ => Err(()),
				}
			}
		}

		impl Into<u8> for TokenSymbol {
			fn into(self) -> u8 {
				match self {
					$(TokenSymbol::$symbol => ($val),)*
				}
			}
		}

        impl TokenSymbol {
			pub fn get_info() -> Vec<(&'static str, u32)> {
				vec![
					$((stringify!($symbol), $deci),)*
				]
			}

            pub const fn one(&self) -> Balance {
                10u128.pow(self.decimals() as u32)
            }

            const fn decimals(&self) -> u8 {
				match self {
					$(TokenSymbol::$symbol => $deci,)*
				}
			}
		}

		impl CurrencyInfo for TokenSymbol {
			fn name(&self) -> &str {
				match self {
					$(TokenSymbol::$symbol => $name,)*
				}
			}
			fn symbol(&self) -> &str {
				match self {
					$(TokenSymbol::$symbol => stringify!($symbol),)*
				}
			}
			fn decimals(&self) -> u8 {
				self.decimals()
			}
		}

		impl TryFrom<Vec<u8>> for TokenSymbol {
			type Error = ();
			fn try_from(v: Vec<u8>) -> Result<TokenSymbol, ()> {
				match v.as_slice() {
					$(bstringify!($symbol) => Ok(TokenSymbol::$symbol),)*
					_ => Err(()),
				}
			}
		}
    }
}

create_currency_id! {
    #[derive(Serialize, Deserialize,Encode, Decode, Eq, Hash, PartialEq, Copy, Clone, Debug, PartialOrd, Ord, TypeInfo, MaxEncodedLen)]
    #[cfg_attr(feature = "std", derive(EncodeAsType,DecodeAsType))]
    #[repr(u8)]
    pub enum TokenSymbol {
        DOT("Polkadot", 10) = 0,
        IBTC("interBTC", 8) = 1,
        INTR("Interlay", 10) = 2,

        KSM("Kusama", 12) = 10,
        KBTC("kBTC", 8) = 11,
        KINT("Kintsugi", 12) = 12,
    }
}

#[derive(
    Serialize,
    Deserialize,
    Encode,
    Decode,
    Eq,
    Hash,
    PartialEq,
    Copy,
    Clone,
    Debug,
    PartialOrd,
    Ord,
    TypeInfo,
    MaxEncodedLen,
)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "std", derive(EncodeAsType, DecodeAsType))]
pub enum LpToken {
    Token(TokenSymbol),
    ForeignAsset(ForeignAssetId),
    StableLpToken(StablePoolId),
}

#[derive(
    Serialize,
    Deserialize,
    Encode,
    Decode,
    Eq,
    Hash,
    PartialEq,
    Copy,
    Clone,
    Debug,
    PartialOrd,
    Ord,
    TypeInfo,
    MaxEncodedLen,
)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "std", derive(EncodeAsType, DecodeAsType))]
pub enum CurrencyId {
    Token(TokenSymbol),
    ForeignAsset(ForeignAssetId),
    LendToken(LendTokenId),
    LpToken(LpToken, LpToken),
    StableLpToken(StablePoolId),
}

pub type ForeignAssetId = u32;
pub type LendTokenId = u32;
pub type StablePoolId = u32;

#[derive(scale_info::TypeInfo, Encode, Decode, Clone, Eq, PartialEq, Debug)]
pub struct CustomMetadata {
    pub fee_per_second: u128,
    pub coingecko_id: Vec<u8>,
}

impl CurrencyId {
    pub fn sort(&mut self) {
        match *self {
            CurrencyId::LpToken(x, y) => {
                if x > y {
                    *self = CurrencyId::LpToken(y, x)
                }
            }
            _ => {}
        }
    }

    pub fn is_lend_token(&self) -> bool {
        matches!(self, CurrencyId::LendToken(_))
    }

    pub fn join_lp_token(currency_id_0: Self, currency_id_1: Self) -> Option<Self> {
        let lp_token_0 = match currency_id_0 {
            CurrencyId::Token(symbol) => LpToken::Token(symbol),
            CurrencyId::ForeignAsset(foreign_asset_id) => LpToken::ForeignAsset(foreign_asset_id),
            CurrencyId::StableLpToken(stable_pool_id) => LpToken::StableLpToken(stable_pool_id),
            _ => return None,
        };
        let lp_token_1 = match currency_id_1 {
            CurrencyId::Token(symbol) => LpToken::Token(symbol),
            CurrencyId::ForeignAsset(foreign_asset_id) => LpToken::ForeignAsset(foreign_asset_id),
            CurrencyId::StableLpToken(stable_pool_id) => LpToken::StableLpToken(stable_pool_id),
            _ => return None,
        };
        Some(CurrencyId::LpToken(lp_token_0, lp_token_1))
    }

    pub fn is_lp_token(&self) -> bool {
        match self {
            Self::Token(_) | Self::ForeignAsset(_) | Self::StableLpToken(_) => true,
            _ => false,
        }
    }
}

impl Into<CurrencyId> for LpToken {
    fn into(self) -> CurrencyId {
        match self {
            LpToken::Token(token) => CurrencyId::Token(token),
            LpToken::ForeignAsset(foreign_asset_id) => CurrencyId::ForeignAsset(foreign_asset_id),
            LpToken::StableLpToken(stable_pool_id) => CurrencyId::StableLpToken(stable_pool_id),
        }
    }
}

#[cfg(feature = "runtime-benchmarks")]
impl From<u32> for CurrencyId {
    fn from(value: u32) -> Self {
        if value < 1000 {
            // Inner value must fit inside `u8`
            CurrencyId::ForeignAsset((value % 256).try_into().unwrap())
        } else {
            CurrencyId::StableLpToken((value % 256).try_into().unwrap())
        }
    }
}

pub mod xcm {
    use codec::{Compact, Encode};
    use sp_io::hashing::blake2_256;
    use sp_std::{borrow::Borrow, marker::PhantomData, vec::Vec};
    use xcm::prelude::{
        AccountId32, AccountKey20, Here, MultiLocation, PalletInstance, Parachain, X1,
    };
    use xcm_executor::traits::Convert;

    /// NOTE: Copied from <https://github.com/moonbeam-foundation/polkadot/blob/d83bb6cc7d7c93ead2fd3cafce0e268fd3f6b9bc/xcm/xcm-builder/src/location_conversion.rs#L25C1-L68C2>
    ///
    /// temporary struct that mimics the behavior of the upstream type that we
    /// will move to once we update this repository to Polkadot 0.9.43+.
    pub struct HashedDescriptionDescribeFamilyAllTerminal<AccountId>(PhantomData<AccountId>);
    impl<AccountId: From<[u8; 32]> + Clone> HashedDescriptionDescribeFamilyAllTerminal<AccountId> {
        fn describe_location_suffix(l: &MultiLocation) -> Result<Vec<u8>, ()> {
            match (l.parents, &l.interior) {
                (0, Here) => Ok(Vec::new()),
                (0, X1(PalletInstance(i))) => {
                    Ok((b"Pallet", Compact::<u32>::from(*i as u32)).encode())
                }
                (0, X1(AccountId32 { id, .. })) => Ok((b"AccountId32", id).encode()),
                (0, X1(AccountKey20 { key, .. })) => Ok((b"AccountKey20", key).encode()),
                _ => Err(()),
            }
        }
    }

    impl<AccountId: From<[u8; 32]> + Clone> Convert<MultiLocation, AccountId>
    for HashedDescriptionDescribeFamilyAllTerminal<AccountId>
    {
        fn convert_ref(location: impl Borrow<MultiLocation>) -> Result<AccountId, ()> {
            let l = location.borrow();
            let to_hash = match (l.parents, l.interior.first()) {
                (0, Some(Parachain(index))) => {
                    let tail = l.interior.split_first().0;
                    let interior = Self::describe_location_suffix(&tail.into())?;
                    (b"ChildChain", Compact::<u32>::from(*index), interior).encode()
                }
                (1, Some(Parachain(index))) => {
                    let tail = l.interior.split_first().0;
                    let interior = Self::describe_location_suffix(&tail.into())?;
                    (b"SiblingChain", Compact::<u32>::from(*index), interior).encode()
                }
                (1, _) => {
                    let tail = l.interior.into();
                    let interior = Self::describe_location_suffix(&tail)?;
                    (b"ParentChain", interior).encode()
                }
                _ => return Err(()),
            };
            Ok(blake2_256(&to_hash).into())
        }

        fn reverse_ref(_: impl Borrow<AccountId>) -> Result<MultiLocation, ()> {
            Err(())
        }
    }

    #[test]
    fn test_hashed_family_all_terminal_converter() {
        use xcm::prelude::X2;

        type Converter<AccountId> = HashedDescriptionDescribeFamilyAllTerminal<AccountId>;

        assert_eq!(
            [
                129, 211, 14, 6, 146, 54, 225, 200, 135, 103, 248, 244, 125, 112, 53, 133, 91, 42,
                215, 236, 154, 199, 191, 208, 110, 148, 223, 55, 92, 216, 250, 34
            ],
            Converter::<[u8; 32]>::convert(MultiLocation {
                parents: 0,
                interior: X2(
                    Parachain(1),
                    AccountId32 {
                        network: None,
                        id: [0u8; 32]
                    }
                ),
            })
                .unwrap()
        );
        assert_eq!(
            [
                17, 142, 105, 253, 199, 34, 43, 136, 155, 48, 12, 137, 155, 219, 155, 110, 93, 181,
                93, 252, 124, 60, 250, 195, 229, 86, 31, 220, 121, 111, 254, 252
            ],
            Converter::<[u8; 32]>::convert(MultiLocation {
                parents: 1,
                interior: X2(
                    Parachain(1),
                    AccountId32 {
                        network: None,
                        id: [0u8; 32]
                    }
                ),
            })
                .unwrap()
        );
        assert_eq!(
            [
                237, 65, 190, 49, 53, 182, 196, 183, 151, 24, 214, 23, 72, 244, 235, 87, 187, 67,
                52, 122, 195, 192, 10, 58, 253, 49, 0, 112, 175, 224, 125, 66
            ],
            Converter::<[u8; 32]>::convert(MultiLocation {
                parents: 0,
                interior: X2(
                    Parachain(1),
                    AccountKey20 {
                        network: None,
                        key: [0u8; 20]
                    }
                ),
            })
                .unwrap()
        );
        assert_eq!(
            [
                226, 225, 225, 162, 254, 156, 113, 95, 68, 155, 160, 118, 126, 18, 166, 132, 144,
                19, 8, 204, 228, 112, 164, 189, 179, 124, 249, 1, 168, 110, 151, 50
            ],
            Converter::<[u8; 32]>::convert(MultiLocation {
                parents: 1,
                interior: X2(
                    Parachain(1),
                    AccountKey20 {
                        network: None,
                        key: [0u8; 20]
                    }
                ),
            })
                .unwrap()
        );
        assert_eq!(
            [
                254, 186, 179, 229, 13, 24, 84, 36, 84, 35, 64, 95, 114, 136, 62, 69, 247, 74, 215,
                104, 121, 114, 53, 6, 124, 46, 42, 245, 121, 197, 12, 208
            ],
            Converter::<[u8; 32]>::convert(MultiLocation {
                parents: 1,
                interior: X2(Parachain(2), PalletInstance(3)),
            })
                .unwrap()
        );
        assert_eq!(
            [
                217, 56, 0, 36, 228, 154, 250, 26, 200, 156, 1, 39, 254, 162, 16, 187, 107, 67, 27,
                16, 218, 254, 250, 184, 6, 27, 216, 138, 194, 93, 23, 165
            ],
            Converter::<[u8; 32]>::convert(MultiLocation {
                parents: 1,
                interior: Here,
            })
                .unwrap()
        );
    }
}
