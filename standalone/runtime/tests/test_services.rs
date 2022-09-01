mod mock;
use mock::{assert_eq, *};

mod client_releases {
    use clients_info::ClientRelease;

    use super::{assert_eq, *};

    #[test]
    fn integration_test_vault_registry_set_current_client_release_works() {
        ExtBuilder::build().execute_with(|| {
            // Set the vault release
            let vault_key = b"vault".to_vec();
            let vault_release = ClientRelease {
                uri: b"https://github.com/interlay/interbtc-clients/releases/download/1.16.0/vault-parachain-metadata-kintsugi-testnet"
                    .to_vec(),
                checksum: H256::default(),
            };
            assert_ok!(Call::ClientsInfo(ServicesCall::set_current_client_release {
                client_name: vault_key.clone(),
                release: vault_release.clone()
            })
            .dispatch(root()));
            assert_eq!(
                ServicesPallet::current_client_release(vault_key),
                Some(vault_release)
            );

            // Set the oracle release
            let oracle_client_name = b"oracle".to_vec();
            let oracle_release = ClientRelease {
                uri: b"https://github.com/interlay/interbtc-clients/releases/download/1.16.0/oracle-parachain-metadata-kintsugi-testnet"
                    .to_vec(),
                checksum: H256::default(),
            };
            assert_ok!(Call::ClientsInfo(ServicesCall::set_current_client_release {
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
            let vault_key = b"vault".to_vec();
            let vault_release = ClientRelease {
                uri: b"https://github.com/interlay/interbtc-clients/releases/download/1.16.0/vault-parachain-metadata-kintsugi-testnet"
                    .to_vec(),
                checksum: H256::default(),
            };
            assert_ok!(Call::ClientsInfo(ServicesCall::set_pending_client_release {
                client_name: vault_key.clone(),
                release: vault_release.clone()
            })
            .dispatch(root()));

            // Set the oracle release
            let oracle_key = b"oracle".to_vec();
            let oracle_release = ClientRelease {
                uri: b"https://github.com/interlay/interbtc-clients/releases/download/1.16.0/oracle-parachain-metadata-kintsugi-testnet"
                    .to_vec(),
                checksum: H256::default(),
            };
            assert_ok!(Call::ClientsInfo(ServicesCall::set_pending_client_release {
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
