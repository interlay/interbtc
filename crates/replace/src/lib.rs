#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]
#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(test)]
extern crate mocktopus;

#[cfg(test)]
use mocktopus::macros::mockable;

use bitcoin::types::H256Le;
use codec::{Decode, Encode};
use frame_support::traits::Currency;
/// # PolkaBTC Replace implementation
/// The Replace module according to the specification at
/// https://interlay.gitlab.io/polkabtc-spec/spec/issue.html
// Substrate
use frame_support::{decl_event, decl_module, decl_storage, dispatch::DispatchResult, ensure};
use primitive_types::H256;
use sha2::{Digest, Sha256};
use sp_core::H160;
use sp_runtime::ModuleId;
use system::ensure_signed;
use x_core::Error;

type DOT<T> = <<T as collateral::Trait>::DOT as Currency<<T as system::Trait>::AccountId>>::Balance;
type PolkaBTC<T> =
    <<T as treasury::Trait>::PolkaBTC as Currency<<T as system::Trait>::AccountId>>::Balance;

/// The issue module id, used for deriving its sovereign account ID.
const _MODULE_ID: ModuleId = ModuleId(*b"issuemod");

/// The pallet's configuration trait.
pub trait Trait:
    system::Trait
    + vault_registry::Trait
    + collateral::Trait
    + btc_relay::Trait
    + treasury::Trait
    + exchange_rate_oracle::Trait
{
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Replace<AccountId, BlockNumber, PolkaBTC, DOT> {
    old_vault: AccountId,
    open_time: BlockNumber,
    amount: PolkaBTC,
    griefing_collateral: DOT,
    new_vault: Option<AccountId>,
    collateral: DOT,
    accept_time: Option<BlockNumber>,
    btc_address: H160,
}

impl<AccountId, BlockNumber, PolkaBTC, DOT> Replace<AccountId, BlockNumber, PolkaBTC, DOT> {
    fn add_new_vault(
        &mut self,
        new_vault_id: AccountId,
        accept_time: BlockNumber,
        collateral: DOT,
        btc_address: H160,
    ) {
        self.new_vault = Some(new_vault_id);
        self.accept_time = Some(accept_time);
        self.collateral = collateral;
        self.btc_address = btc_address;
    }
}
use rand::SeedableRng;

#[derive(Encode, Debug, Decode, Clone)]
pub struct ReplaceRngSeed(pub Vec<u8>);
#[derive(Encode, Debug, Decode, Clone)]
pub struct ReplaceRng(ReplaceRngSeed);

impl Default for ReplaceRngSeed {
    fn default() -> ReplaceRngSeed {
        ReplaceRngSeed([1u8; 64].to_vec())
    }
}

impl AsMut<[u8]> for ReplaceRngSeed {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

impl SeedableRng for ReplaceRng {
    type Seed = ReplaceRngSeed;

    fn from_seed(seed: ReplaceRngSeed) -> ReplaceRng {
        ReplaceRng(seed)
    }
}

#[derive(Encode, Decode, Default, Clone)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct ReplaceKey {
    seed: ReplaceRngSeed,
    nonce: u64,
    btc_address: H160,
}

fn replace_key(seed: ReplaceRngSeed, nonce: u64, btc_address: H160) -> H256 {
    let key = ReplaceKey {
        seed,
        nonce,
        btc_address,
    };
    let mut hasher = Sha256::default();
    hasher.input(key.encode());

    let mut result = [0; 32];
    result.copy_from_slice(&hasher.result()[..]);
    H256(result)
}

// The pallet's storage items.
decl_storage! {
    trait Store for Module<T: Trait> as Replace {
        ReplaceGriefingCollateral: DOT<T>;
        ReplacePeriod: T::BlockNumber;
        ReplaceRequests: map hasher(blake2_128_concat) H256 => Option<Replace<T::AccountId, T::BlockNumber, PolkaBTC<T>, DOT<T>>>;
    }
}

// The pallet's events.
decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
        PolkaBTC = PolkaBTC<T>,
        DOT = DOT<T>,
        BlockNumber = <T as system::Trait>::BlockNumber,
    {
        RequestReplace(AccountId, PolkaBTC, BlockNumber, H256),
        WithdrawReplace(AccountId, H256),
        AcceptReplace(AccountId, H256, DOT),
        ExecuteReplace(H256, AccountId, AccountId),
        AuctionReplace(AccountId, H256, DOT),
        CancelReplace(AccountId, AccountId, H256),
    }
);

// The pallet's dispatchable functions.
decl_module! {
    /// The module declaration.
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        // Initializing events
        // this is needed only if you are using events in your pallet
        fn deposit_event() = default;

        /// Request the replacement of a new vault ownership
        ///
        /// # Arguments
        ///
        /// * `origin` - sender of the transaction
        /// * `amount` - amount of PolkaBTC
        /// * `vault` - address of the vault
        /// * `griefing_collateral` - amount of DOT
        fn request_replace(origin, old_vault: T::AccountId, amount: PolkaBTC<T>, timeout: T::BlockNumber, griefing_collateral: DOT<T>)
            -> DispatchResult
        {
            let requester = ensure_signed(origin)?;
            Self::_request_replace(requester, old_vault, amount, timeout, griefing_collateral)?;
            Ok(())
        }

        /// Withdraw a request of vault replacement
        ///
        /// # Arguments
        ///
        /// * `origin` - sender of the transaction
        /// * `vault_id` - the of the vault to cancel the request
        /// * `replace_id` - the unique identifier for the specific request
        fn withdraw_replace_request(origin, vault_id: T::AccountId, replace_id: H256)
            -> DispatchResult
        {
            let _ = ensure_signed(origin)?;
            Self::_withdraw_replace_request(vault_id, replace_id)?;
            Ok(())
        }

        /// Accept request of vault replacement
        ///
        /// # Arguments
        ///
        /// * `origin` - sender of the transaction
        /// * `vault_id` - the of the vault to cancel the request
        /// * `replace_id` - the unique identifier for the specific request
        fn accept_replace(origin, new_vault_id: T::AccountId, replace_id: H256, collateral: DOT<T>)
            -> DispatchResult
        {
            let _ = ensure_signed(origin)?;
            Self::_accept_replace(new_vault_id, replace_id, collateral)?;
            Ok(())
        }

        /// Accept request of vault replacement
        ///
        /// # Arguments
        ///
        /// * `origin` - sender of the transaction
        /// * `vault_id` - the of the vault to cancel the request
        /// * `replace_id` - the unique identifier for the specific request
        fn auction_replace(origin, old_vault_id: T::AccountId, new_vault_id: T::AccountId, btc_amount: PolkaBTC<T>, collateral: DOT<T>)
            -> DispatchResult
        {
            let _ = ensure_signed(origin)?;
            Self::_auction_replace(old_vault_id, new_vault_id, btc_amount, collateral)?;
            Ok(())
        }

        fn execute_replace(origin, new_vault_id: T::AccountId, replace_id: H256, tx_id: H256Le, tx_block_height: u32, tx_index: H256, merkle_proof: Vec<u8>, raw_tx: Vec<u8>) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            Self::_execute_replace(new_vault_id, replace_id, tx_id, tx_block_height, tx_index, merkle_proof, raw_tx)?;
            Ok(())
        }

        fn cancel_replace(origin, new_vault_id: T::AccountId, replace_id: H256) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            Self::_cancel_replace(new_vault_id, replace_id)?;
            Ok(())
        }
    }
}

// "Internal" functions, callable by code.
#[cfg_attr(test, mockable)]
impl<T: Trait> Module<T> {
    fn _request_replace(
        requester: T::AccountId,
        vault_id: T::AccountId,
        mut amount: PolkaBTC<T>,
        timeout: T::BlockNumber,
        griefing_collateral: DOT<T>,
    ) -> Result<H256, Error> {
        // check preconditions
        // check amount is non zero
        let zero: PolkaBTC<T> = 0u32.into();
        if amount == zero {
            return Err(Error::InvalidAmount);
        }
        // check timeout
        let zero: T::BlockNumber = 0.into();
        if timeout == zero {
            return Err(Error::InvalidTimeout);
        }
        // check vault exists
        let vault = <vault_registry::Module<T>>::_get_vault_from_id(&vault_id)?;
        // check vault is not banned
        let height = Self::current_height();
        //TODO(jaupe) implement this function ensure_not_banned
        vault.ensure_not_banned(height)?;
        // check amount is available for replacing
        if amount > vault.issued_tokens {
            amount = vault.issued_tokens;
        }
        // check that the remaining dot collateral is valid if it's a partial replace
        if amount != vault.issued_tokens {
            // TODO(jaupe) verify that the coin rate is BTCDOT
            //let amount: u128 = amount.into();
            let rate = <exchange_rate_oracle::Module<T>>::get_exchange_rate()? as u32; // assume its BTCDOT until confirmed
            let btcdot_rate: PolkaBTC<T> = rate.into();
            let _remaining_collateral = btcdot_rate * amount;
            let remaining_collateral = 0; //TODO(jaupe) convert PokaBTC to fixed precision type
                                          //TODO(jaupe) convert DOT into u32 for now; but forward, go to fixed precision types
            let minimum_collateral: u32 = 1_000_000; //<vault_registry::Module<T>>::minimum_collateral().into();
            if remaining_collateral < minimum_collateral {
                return Err(Error::InsufficientCollateral);
            }
        }
        // check sufficient griefing amount
        ensure!(
            griefing_collateral >= <ReplaceGriefingCollateral<T>>::get(),
            Error::InsufficientCollateral
        );

        // lock griefing collateral
        <collateral::Module<T>>::lock_collateral(&requester, griefing_collateral)?;

        let _btc_address =
            <vault_registry::Module<T>>::_increase_to_be_issued_tokens(&vault_id, amount.clone())?;

        let replace = Replace {
            old_vault: vault_id.clone(),
            open_time: height,
            amount,
            griefing_collateral,
            new_vault: None,
            collateral: vault.collateral,
            accept_time: None,
            btc_address: vault.btc_address,
        };

        //TODO(jaupe) populate nonce
        let nonce = 0;
        let key = replace_key(ReplaceRngSeed::default(), nonce, vault.btc_address);
        Self::insert_replace_request(key, replace);

        Self::deposit_event(<Event<T>>::RequestReplace(vault_id, amount, timeout, key));
        Ok(key)
    }

    fn _withdraw_replace_request(vault_id: T::AccountId, request_id: H256) -> Result<(), Error> {
        // check vault exists
        let vault = <vault_registry::Module<T>>::_get_vault_from_id(&vault_id)?;
        let req = Self::get_replace_request(request_id)?;
        if req.old_vault != vault_id {
            return Err(Error::InvalidVaultID); // TODO(jaupe) is this the correct error code? should it call ensure! macro?
        }
        //let threshold = <vault_registry::Module<T>>::auction_collateral_threshold();
        // check collateral is below the threshold
        // TODO(jaupe) investigate if availabe tokens need to be checked too

        // TODO(jaupe) get collateral from collateral module
        /*
        let threshold: DOT<T> = 1_000_000u32.into(); // TODO(jaupe) convert this correctly
        if vault.collateral < threshold {
            return Err(Error::InsufficientCollateral);
        }
        */
        // check that a new vault owner hasn't already comitted to replacing
        if req.new_vault.is_some() {
            return Err(Error::CancelAcceptedRequest);
        }
        <collateral::Module<T>>::release_collateral(
            &req.old_vault,
            req.griefing_collateral.clone(),
        )?;
        <vault_registry::Module<T>>::_decrease_to_be_issued_tokens(
            &req.old_vault,
            req.amount.clone(),
        )?;
        Self::remove_replace_request(request_id);
        Self::deposit_event(<Event<T>>::WithdrawReplace(vault_id, request_id));
        Ok(())
    }

    fn _accept_replace(
        new_vault_id: T::AccountId,
        replace_id: H256,
        collateral: DOT<T>,
    ) -> Result<(), Error> {
        // check new vault exists
        let vault = <vault_registry::Module<T>>::_get_vault_from_id(&new_vault_id)?;
        let mut req = Self::get_replace_request(replace_id)?;
        // ensure vault is not banned
        let height = <system::Module<T>>::block_number();
        vault.ensure_not_banned(height)?;
        // ensure sufficient collateral
        vault.ensure_collateral(collateral)?;
        // lock collateral
        <collateral::Module<T>>::lock_collateral(&new_vault_id, collateral)?;
        // update request data
        req.add_new_vault(new_vault_id.clone(), height, collateral, vault.btc_address);
        Self::insert_replace_request(replace_id, req);
        // emit event
        Self::deposit_event(<Event<T>>::AcceptReplace(
            new_vault_id,
            replace_id,
            collateral,
        ));
        Ok(())
    }

    fn _auction_replace(
        old_vault_id: T::AccountId,
        new_vault_id: T::AccountId,
        btc_amount: PolkaBTC<T>,
        collateral: DOT<T>,
    ) -> Result<(), Error> {
        let new_vault = <vault_registry::Module<T>>::_get_vault_from_id(&new_vault_id)?;
        let old_vault = <vault_registry::Module<T>>::_get_vault_from_id(&old_vault_id)?;
        // check collateral is below the minimin auction threshold
        // TODO(jaupe) investigate if availabe tokens need to be checked too
        //let threshold = <vault_registry::Module<T>>::auction_collateral_threshold();
        let threshold: DOT<T> = 1_000_000u32.into(); // TODO(jaupe) convert this correctly
                                                     // TODO(jaupe) get colllateral
                                                     //if old_vault.collateral < threshold {
                                                     //    return Err(Error::InsufficientCollateral);
                                                     //}
                                                     // TODO(jaupe) check collateral exceeds secure threshold
                                                     /*
                                                     let threshold = <vault_registry::Module<T>>::secure_collateral_threshold();

                                                     if (old_vault.collateral_u64() as u128) < threshold {
                                                         return Err(Error::InsufficientCollateral);
                                                     }
                                                     */
        // lock collateral
        <collateral::Module<T>>::lock_collateral(&new_vault_id, collateral)?;
        // increase to be redeemed tokens
        <vault_registry::Module<T>>::_increase_to_be_redeemed_tokens(&old_vault_id, btc_amount)?;
        // generate request
        let height = <system::Module<T>>::block_number();
        //TODO(jaupe) populate nonce
        let nonce = 0;
        let replace_id = replace_key(ReplaceRngSeed::default(), nonce, new_vault.btc_address);
        Self::insert_replace_request(
            replace_id,
            Replace {
                new_vault: Some(new_vault_id.clone()),
                old_vault: old_vault_id.clone(),
                open_time: height,
                accept_time: Some(height),
                amount: btc_amount,
                griefing_collateral: 0.into(),
                btc_address: new_vault.btc_address,
                collateral: collateral,
            },
        );
        // emit event
        Self::deposit_event(<Event<T>>::AuctionReplace(
            new_vault_id,
            replace_id,
            collateral,
        ));
        Ok(())
    }

    //TODO(jaupe) work out what tx index is for
    fn _execute_replace(
        new_vault_id: T::AccountId,
        replace_id: H256,
        tx_id: H256Le,
        tx_block_height: u32,
        _tx_index: H256,
        merkle_proof: Vec<u8>,
        raw_tx: Vec<u8>,
    ) -> Result<(), Error> {
        let req = Self::get_replace_request(replace_id)?;
        let replace_period = Self::replace_period();
        let height = Self::current_height();
        if req.open_time >= height - replace_period {
            return Err(Error::ReplacePeriodExpired);
        }
        let new_vault = <vault_registry::Module<T>>::_get_vault_from_id(&new_vault_id)?;
        // verify transaction is included
        // TODO(jaupe) work out what confirmations and insecure should be
        let confirmations = 0;
        let insecure = false;
        <btc_relay::Module<T>>::_verify_transaction_inclusion(
            tx_id,
            tx_block_height,
            merkle_proof,
            confirmations,
            insecure,
        )?;
        let amount = 0i64; //TODO(jaupe) work out how to convert substrate currency type to i64
        <btc_relay::Module<T>>::_validate_transaction(
            raw_tx,
            amount,
            new_vault.btc_address.as_bytes().to_vec(),
            replace_id.as_bytes().to_vec(),
        )?;
        // TODO(jaupe) discuss with dan if i need to implement replace_tokens or not as it's missing
        unimplemented!();
    }

    fn _cancel_replace(new_vault_id: T::AccountId, replace_id: H256) -> Result<(), Error> {
        let req = Self::get_replace_request(replace_id)?;
        let _vault = <vault_registry::Module<T>>::_get_vault_from_id(&new_vault_id)?;
        let _current_height = Self::current_height();
        let _replace_period = Self::replace_period();
        //TODO(jaupe) ensure timeout period has not expired
        <collateral::Module<T>>::slash_collateral(
            req.old_vault.clone(),
            new_vault_id.clone(),
            req.griefing_collateral,
        )?;
        //TODO(jaupe) call decreaseToBeRedeemedTokens once available
        Self::remove_replace_request(replace_id.clone());
        Self::deposit_event(<Event<T>>::CancelReplace(
            new_vault_id,
            req.old_vault,
            replace_id,
        ));
        Ok(())
    }

    fn get_replace_request(
        id: H256,
    ) -> Result<Replace<T::AccountId, T::BlockNumber, PolkaBTC<T>, DOT<T>>, Error> {
        <ReplaceRequests<T>>::get(id).ok_or(Error::InvalidReplaceID)
    }

    fn insert_replace_request(
        key: H256,
        value: Replace<T::AccountId, T::BlockNumber, PolkaBTC<T>, DOT<T>>,
    ) {
        <ReplaceRequests<T>>::insert(key, value)
    }

    fn replace_period() -> T::BlockNumber {
        <ReplacePeriod<T>>::get()
    }

    fn remove_replace_request(key: H256) {
        <ReplaceRequests<T>>::remove(key)
    }

    #[allow(dead_code)]
    fn set_issue_griefing_collateral(amount: DOT<T>) {
        <ReplaceGriefingCollateral<T>>::set(amount);
    }

    #[allow(dead_code)]
    fn set_replace_period(value: T::BlockNumber) {
        <ReplacePeriod<T>>::set(value);
    }

    fn current_height() -> T::BlockNumber {
        <system::Module<T>>::block_number()
    }
}
