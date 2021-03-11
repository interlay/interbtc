mod mock;
use mock::issue_testing_utils::{ExecuteIssueBuilder, RequestIssueBuilder};
use mock::*;

const PROOF_SUBMITTER: [u8; 32] = BOB;
const VAULT1: [u8; 32] = CAROL;
const VAULT2: [u8; 32] = DAVE;
const ISSUE_RELAYER: [u8; 32] = EVE;
const RELAYER_1: [u8; 32] = FRANK;
const RELAYER_2: [u8; 32] = GRACE;

fn setup_relayer(relayer: [u8; 32], sla: u32, stake: u128) {
    // register as staked relayer
    assert_ok!(
        Call::StakedRelayers(StakedRelayersCall::register_staked_relayer(stake))
            .dispatch(origin_of(account_of(relayer)))
    );
    for _ in 0..sla {
        SlaModule::event_update_relayer_sla(
            account_of(relayer),
            sla::types::RelayerEvent::BlockSubmission,
        )
        .unwrap();
    }
}

fn set_issued_and_backing(vault: [u8; 32], amount_issued: u128, backing: u128) {
    let (issue_id, _) = RequestIssueBuilder::new(100 * amount_issued)
        .with_vault(vault)
        .request();

    ExecuteIssueBuilder::new(issue_id)
        .with_submitter(PROOF_SUBMITTER)
        .with_relayer(ISSUE_RELAYER)
        .execute();

    CoreVaultData::force_to(
        vault,
        CoreVaultData {
            backing_collateral: 100 * backing,
            ..CoreVaultData::vault(vault)
        },
    );
    VaultRegistryModule::slash_collateral(
        CurrencySource::Backing(account_of(PROOF_SUBMITTER)),
        CurrencySource::FreeBalance(account_of(FAUCET)),
        CurrencySource::<Runtime>::Backing(account_of(PROOF_SUBMITTER))
            .current_balance()
            .unwrap(),
    )
    .unwrap();
}
#[test]
fn test_vault_fee_pool_withdrawal() {
    ExtBuilder::build().execute_with(|| {
        set_issued_and_backing(VAULT1, 200, 800);
        set_issued_and_backing(VAULT2, 800, 200);

        let epoch_rewards = FeeModule::epoch_rewards_polka_btc();
        let vault_rewards = (epoch_rewards * 90) / 100; // set at 90% in tests

        // simulate that we entered a new epoch
        assert_ok!(FeeModule::update_rewards_for_epoch());

        // First vault gets 26% of the vault pool (20% of the 90% awarded by issued,
        // and 80% of the 10% awarded by collateral
        assert_eq!(
            FeeModule::get_polka_btc_rewards(&account_of(VAULT1)),
            (vault_rewards * 26) / 100
        );
        // second vault gets the other 74%
        assert_eq!(
            FeeModule::get_polka_btc_rewards(&account_of(VAULT2)),
            (vault_rewards * 74) / 100
        );
    })
}

#[test]
fn test_vault_fee_pool_withdrawal_with_liquidated_vaults() {
    ExtBuilder::build().execute_with(|| {
        set_issued_and_backing(VAULT1, 200, 800);
        set_issued_and_backing(VAULT2, 800, 200);

        drop_exchange_rate_and_liquidate(VAULT2);

        let epoch_rewards = FeeModule::epoch_rewards_polka_btc();
        let vault_rewards = (epoch_rewards * 90) / 100; // set at 90% in tests

        // simulate that we entered a new epoch
        assert_ok!(FeeModule::update_rewards_for_epoch());

        // First vault gets 100% of the vault pool
        assert_eq!(
            FeeModule::get_polka_btc_rewards(&account_of(VAULT1)),
            vault_rewards
        );
        // second vault gets nothing
        assert_eq!(FeeModule::get_polka_btc_rewards(&account_of(VAULT2)), 0);
    })
}

#[test]
fn test_vault_fee_pool_withdrawal_over_multiple_epochs() {
    ExtBuilder::build().execute_with(|| {
        set_issued_and_backing(VAULT1, 200, 800);

        let epoch_1_rewards = FeeModule::epoch_rewards_polka_btc();
        let vault_epoch_1_rewards = (epoch_1_rewards * 90) / 100; // set at 90% in tests

        // simulate that we entered a new epoch
        assert_ok!(FeeModule::update_rewards_for_epoch());

        // First vault gets 100% of the vault pool
        assert_eq!(
            FeeModule::get_polka_btc_rewards(&account_of(VAULT1)),
            vault_epoch_1_rewards
        );

        set_issued_and_backing(VAULT2, 800, 200);

        let epoch_2_rewards = FeeModule::epoch_rewards_polka_btc();
        let vault_epoch_2_rewards = (epoch_2_rewards * 90) / 100; // set at 90% in tests

        // simulate that we entered a new epoch
        assert_ok!(FeeModule::update_rewards_for_epoch());

        // First vault gets 26% of the vault_epoch_2_rewards (20% of the 90% awarded by issued,
        // and 80% of the 10% awarded by collateral
        assert_eq!(
            FeeModule::get_polka_btc_rewards(&account_of(VAULT1)),
            vault_epoch_1_rewards + (vault_epoch_2_rewards * 26) / 100 - 1 // - 1 due to rounding difference
        );
        // second vault gets the other 74%
        assert_eq!(
            FeeModule::get_polka_btc_rewards(&account_of(VAULT2)),
            (vault_epoch_2_rewards * 74) / 100
        );
    })
}

#[test]
fn test_relayer_fee_pool_withdrawal() {
    ExtBuilder::build().execute_with(|| {
        set_issued_and_backing(VAULT1, 1000, 1000);

        // make the used relayer irrelevant in fee calculations
        SlaModule::event_update_relayer_sla(
            account_of(ISSUE_RELAYER),
            sla::types::RelayerEvent::FalseInvalidVoteOrReport,
        )
        .unwrap();

        setup_relayer(RELAYER_1, 20, 100);
        setup_relayer(RELAYER_2, 40, 200);

        let epoch_rewards = FeeModule::epoch_rewards_polka_btc();
        let relayer_rewards = (epoch_rewards * 10) / 100; // set at 10% in tests

        // simulate that we entered a new epoch
        assert_ok!(FeeModule::update_rewards_for_epoch());

        // First vault gets 20% of the vault pool
        assert_eq!(
            FeeModule::get_polka_btc_rewards(&account_of(RELAYER_1)),
            (relayer_rewards * 20) / 100
        );
        // second vault gets the other 80%
        assert_eq!(
            FeeModule::get_polka_btc_rewards(&account_of(RELAYER_2)),
            (relayer_rewards * 80) / 100
        );
    })
}
