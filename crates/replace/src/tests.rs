use crate::mock::*;
use frame_support::assert_noop;
use primitive_types::H256;
/// Tests for Replace
use x_core::Error;

fn request_replace(
    origin: AccountId,
    amount: Balance,
    vault: AccountId,
    collateral: Balance,
) -> Result<H256, Error> {
    Replace::_request_replace(origin, amount, vault, collateral)
}

#[test]
fn request_replace_invalid_amount() {
    run_test(|| {
        <system::Module<Test>>::set_block_number(0);
        assert_noop!(request_replace(ALICE, 0, 0, BOB), Error::InvalidAmount);
    })
}
