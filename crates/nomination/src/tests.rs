use crate::mock::*;
use frame_support::assert_err;
use vault_registry::{BtcPublicKey, VaultStatus, Wallet};

fn dummy_public_key() -> BtcPublicKey {
    BtcPublicKey([
        2, 205, 114, 218, 156, 16, 235, 172, 106, 37, 18, 153, 202, 140, 176, 91, 207, 51, 187, 55, 18, 45, 222, 180,
        119, 54, 243, 97, 173, 150, 161, 169, 230,
    ])
}

fn register_vault(id: u64) {
    <vault_registry::Pallet<Test>>::insert_vault(
        &id,
        vault_registry::Vault {
            id,
            to_be_replaced_tokens: 0,
            to_be_issued_tokens: 0,
            issued_tokens: 10,
            to_be_redeemed_tokens: 0,
            replace_collateral: 0,
            backing_collateral: 100,
            wallet: Wallet::new(dummy_public_key()),
            banned_until: None,
            status: VaultStatus::Active(true),
            ..Default::default()
        },
    );
}

#[test]
fn test_non_vaults_cannot_become_operators() {
    run_test(|| {
        assert_err!(
            Nomination::opt_in_to_nomination(Origin::signed(BOB)),
            TestError::NotAVault
        );
    })
}

#[test]
fn test_regular_vaults_cannot_opt_out() {
    run_test(|| {
        register_vault(BOB);
        assert_eq!(Nomination::is_operator(&BOB).unwrap(), false);
        assert_err!(
            Nomination::opt_out_of_nomination(Origin::signed(BOB)),
            TestError::VaultNotOptedInToNomination
        );
    });
}
