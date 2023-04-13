mod mock;
use mock::{assert_eq, *};

mod client_releases {
    use clients_info::ClientRelease;
    use frame_support::BoundedVec;

    use super::{assert_eq, *};

    #[test]
    fn integration_test_vault_registry_set_current_client_release_works() {
        ExtBuilder::build().execute_with(|| {
            // Set the vault release
            let vault_key = BoundedVec::try_from(b"vault".to_vec()).unwrap();
            let vault_release = ClientRelease {
                uri: BoundedVec::try_from(b"https://github.com/interlay/interbtc-clients/releases/download/1.16.0/vault-parachain-metadata-kintsugi-testnet".to_vec()).unwrap(),
                checksum: H256::default(),
            };
            assert_ok!(RuntimeCall::ClientsInfo(ServicesCall::set_current_client_release {
                client_name: vault_key.clone(),
                release: vault_release.clone()
            })
            .dispatch(root()));
            assert_eq!(
                ServicesPallet::current_client_release(vault_key),
                Some(vault_release)
            );

            // Set the oracle release
            let oracle_client_name = BoundedVec::try_from(b"oracle".to_vec()).unwrap();
            let oracle_release = ClientRelease {
                uri: BoundedVec::try_from(b"https://github.com/interlay/interbtc-clients/releases/download/1.16.0/oracle-parachain-metadata-kintsugi-testnet".to_vec()).unwrap(),
                checksum: H256::default(),
            };
            assert_ok!(RuntimeCall::ClientsInfo(ServicesCall::set_current_client_release {
                client_name: oracle_client_name.clone(),
                release: oracle_release.clone()
            })
            .dispatch(root()));
            assert_eq!(
                ServicesPallet::current_client_release(oracle_client_name),
                Some(oracle_release)
            );
        });
    }

    #[test]
    fn integration_test_vault_registry_set_pending_client_release_works() {
        ExtBuilder::build().execute_with(|| {
            // Set the vault release
            let vault_key = BoundedVec::try_from(b"vault".to_vec()).unwrap();
            let vault_release = ClientRelease {
                uri: BoundedVec::try_from(b"https://github.com/interlay/interbtc-clients/releases/download/1.16.0/vault-parachain-metadata-kintsugi-testnet".to_vec()).unwrap(),
                checksum: H256::default(),
            };
            assert_ok!(RuntimeCall::ClientsInfo(ServicesCall::set_pending_client_release {
                client_name: vault_key.clone(),
                release: vault_release.clone()
            })
            .dispatch(root()));

            // Set the oracle release
            let oracle_key = BoundedVec::try_from(b"oracle".to_vec()).unwrap();
            let oracle_release = ClientRelease {
                uri: BoundedVec::try_from(b"https://github.com/interlay/interbtc-clients/releases/download/1.16.0/oracle-parachain-metadata-kintsugi-testnet".to_vec()).unwrap(),
                checksum: H256::default(),
            };
            assert_ok!(RuntimeCall::ClientsInfo(ServicesCall::set_pending_client_release {
                client_name: oracle_key.clone(),
                release: oracle_release.clone()
            })
            .dispatch(root()));

            assert_eq!(
                ServicesPallet::pending_client_release(vault_key),
                Some(vault_release)
            );
            assert_eq!(
                ServicesPallet::pending_client_release(oracle_key),
                Some(oracle_release)
            );
        });
    }
}
