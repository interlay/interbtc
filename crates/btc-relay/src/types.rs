use crate::{Error, ACCEPTED_MAX_TRANSACTION_OUTPUTS};
use bitcoin::types::{BlockHeader, H256Le, Transaction, Value};
pub use bitcoin::Address as BtcAddress;
use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{dispatch::DispatchError, ensure};
use scale_info::TypeInfo;
use sp_core::H256;
use sp_std::{convert::TryFrom, vec::Vec};

/// Bitcoin Enriched Block Headers
#[derive(Encode, Decode, Default, Clone, Copy, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen)]
pub struct RichBlockHeader<BlockNumber> {
    pub block_header: BlockHeader,
    /// height of the block in the bitcoin chain
    pub block_height: u32,
    /// id if the chain that this block belongs to
    pub chain_id: u32,
    /// active_block_number of the parachain at the time this block was submitted
    pub para_height: BlockNumber,
}

impl<BlockNumber> RichBlockHeader<BlockNumber> {
    /// Creates a new RichBlockHeader
    ///
    /// # Arguments
    ///
    /// * `block_header` - Bitcoin block header
    /// * `chain_id` - chain reference
    /// * `block_height` - chain height
    /// * `account_id` - submitter
    /// * `para_height` - height of the parachain at submission
    pub fn new(block_header: BlockHeader, chain_id: u32, block_height: u32, para_height: BlockNumber) -> Self {
        RichBlockHeader {
            block_header,
            block_height,
            chain_id,
            para_height,
        }
    }

    pub fn block_hash(&self) -> H256Le {
        self.block_header.hash
    }
}

#[cfg_attr(feature = "std", derive(Debug, PartialEq))]
pub struct OpReturnPaymentData<T: frame_system::Config> {
    pub op_return: H256,
    // vec of (amount, address)
    payments: Vec<(Value, BtcAddress)>,
    _marker: sp_std::marker::PhantomData<T>,
}

impl<T: crate::Config> TryFrom<Transaction> for OpReturnPaymentData<T> {
    type Error = DispatchError;

    fn try_from(transaction: Transaction) -> Result<Self, Self::Error> {
        // check the number of outputs - this check is redundant due to the checks below, but
        // this serves to put an upperbound to the number of iterations
        ensure!(
            transaction.outputs.len() <= ACCEPTED_MAX_TRANSACTION_OUTPUTS,
            Error::<T>::InvalidOpReturnTransaction
        );

        let mut payments = Vec::new();
        let mut op_returns = Vec::new();
        for tx in transaction.outputs {
            if let Ok(address) = tx.extract_address() {
                payments.push((tx.value, address));
            } else if let Ok(data) = tx.script.extract_op_return_data() {
                // make sure the amount is zero
                ensure!(tx.value == 0, Error::<T>::InvalidOpReturnTransaction);
                // make sure that the op_return is exactly 32 bytes
                ensure!(data.len() == 32, Error::<T>::InvalidOpReturnTransaction);
                op_returns.push(H256::from_slice(&data));
            } else {
                return Err(Error::<T>::InvalidOpReturnTransaction.into());
            }
        }

        // check we have exactly 1 op-return
        ensure!(op_returns.len() == 1, Error::<T>::InvalidOpReturnTransaction);

        // Check that we have either 1 payment, or 2 payments to different addresses. Enforcing the
        // payments to be unique helps to prevent the vault from paying more than is allowed
        match payments.len() {
            1 => (),
            2 => {
                // ensure that the addresses are not identical
                ensure!(payments[0].1 != payments[1].1, Error::<T>::InvalidOpReturnTransaction);
            }
            _ => return Err(Error::<T>::InvalidOpReturnTransaction.into()),
        }

        Ok(Self {
            op_return: op_returns.remove(0),
            payments,
            _marker: Default::default(),
        })
    }
}

impl<T: crate::Config> OpReturnPaymentData<T> {
    // ensures this is a valid payment. If it is, it returns the return-to-self address
    pub fn ensure_valid_payment_to(
        &self,
        expected_amount: Value,
        recipient: BtcAddress,
        op_return: Option<H256>,
    ) -> Result<Option<BtcAddress>, DispatchError> {
        // make sure the op_return matches
        if let Some(op_return) = op_return {
            ensure!(op_return == self.op_return, Error::<T>::InvalidPayment);
        }

        // ensure we have a correct payment to the recipient
        let paid_amount = self
            .payments
            .iter()
            .find_map(|&(amount, address)| if address == recipient { Some(amount) } else { None })
            .ok_or(Error::<T>::InvalidPayment)?;

        ensure!(paid_amount == expected_amount, Error::<T>::InvalidPaymentAmount);

        // return the return-to-self if it exists, otherwise None
        Ok(self
            .payments
            .iter()
            .find_map(|&(_, address)| if address != recipient { Some(address) } else { None }))
    }
}
