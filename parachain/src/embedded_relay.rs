use crate::service::{FullBackend, FullClient, RuntimeApiCollection};
use async_trait::async_trait;
use bitcoin_client::RandomDelay;
use bitcoin_client::{
    relay::Issuing, //Error as DirectRelayError,
    BitcoinCoreApi,
};
use cumulus_client_cli::CollatorOptions;
use cumulus_client_consensus_aura::{AuraConsensus, BuildAuraConsensusParams, SlotProportion};
use cumulus_client_consensus_common::{ParachainBlockImport as TParachainBlockImport, ParachainConsensus};
use cumulus_client_network::BlockAnnounceValidator;
use cumulus_client_service::{
    prepare_node_config, start_collator, start_full_node, StartCollatorParams, StartFullNodeParams,
};
use cumulus_primitives_core::ParaId;
use cumulus_primitives_parachain_inherent::{MockValidationDataInherentDataProvider, MockXcmConfig};
use cumulus_relay_chain_inprocess_interface::build_inprocess_relay_chain;
use cumulus_relay_chain_interface::{RelayChainInterface, RelayChainResult};
use cumulus_relay_chain_minimal_node::build_minimal_relay_chain_node;
use frame_support::{storage::storage_prefix, Blake2_128Concat, StorageHasher};
use futures::StreamExt;
use polkadot_primitives::BlockId;
use polkadot_service::{Client, CollatorPair};
use primitives::*;
use sc_client_api::{BlockBackend, HeaderBackend, StateBackendFor};
use sc_consensus::{ImportQueue, LongestChain};
use sc_executor::{HeapAllocStrategy, NativeElseWasmExecutor, WasmExecutor, DEFAULT_HEAP_ALLOC_STRATEGY};
use sc_network::NetworkBlock;
use sc_network_sync::SyncingService;
use sc_service::{Configuration, PartialComponents, RpcHandlers, TFullBackend, TFullClient, TaskManager};
use sc_telemetry::{Telemetry, TelemetryHandle, TelemetryWorker, TelemetryWorkerHandle};
use sc_transaction_pool::{ChainApi, Pool};
use sp_api::{ConstructRuntimeApi, Encode, StateBackend};
use sp_consensus_aura::{
    sr25519::{AuthorityId as AuraId, AuthorityPair as AuraPair},
    SlotDuration,
};
use sp_core::{Pair, H256};
use sp_keyring::Sr25519Keyring;
use sp_keystore::KeystorePtr;
use sp_runtime::{
    traits::{BlakeTwo256, Block as BlockT},
    OpaqueExtrinsic, SaturatedConversion,
};
use std::{sync::Arc, time::Duration};
use substrate_prometheus_endpoint::Registry;
use tokio::sync::Mutex;

pub fn create_extrinsic<Executor, RuntimeApi>(
    client: Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
    nonce: u32,
    call: kintsugi_runtime::RuntimeCall,
) -> OpaqueExtrinsic
where
    Executor: sc_executor::NativeExecutionDispatch + 'static,
{
    let genesis_hash = client.block_hash(0).ok().flatten().expect("Genesis block exists; qed");
    let best_hash = client.chain_info().best_hash;
    let best_block = client.chain_info().best_number;

    let period = kintsugi_runtime::BlockHashCount::get()
        .checked_next_power_of_two()
        .map(|c| c / 2)
        .unwrap_or(2) as u64;

    let extra: kintsugi_runtime::SignedExtra = (
        frame_system::CheckSpecVersion::<kintsugi_runtime::Runtime>::new(),
        frame_system::CheckTxVersion::<kintsugi_runtime::Runtime>::new(),
        frame_system::CheckGenesis::<kintsugi_runtime::Runtime>::new(),
        frame_system::CheckEra::<kintsugi_runtime::Runtime>::from(sp_runtime::generic::Era::mortal(
            period,
            best_block.saturated_into(),
        )),
        frame_system::CheckNonce::<kintsugi_runtime::Runtime>::from(nonce),
        frame_system::CheckWeight::<kintsugi_runtime::Runtime>::new(),
        // frame_system::CheckNonZeroSender::<runtime::Runtime>::new(),
        pallet_transaction_payment::ChargeTransactionPayment::<kintsugi_runtime::Runtime>::from(0),
    );

    /// The payload being signed
    type SignedPayload =
        sp_runtime::generic::SignedPayload<kintsugi_runtime::RuntimeCall, kintsugi_runtime::SignedExtra>;

    let raw_payload = SignedPayload::from_raw(
        call.clone(),
        extra.clone(),
        (
            kintsugi_runtime::VERSION.spec_version,
            kintsugi_runtime::VERSION.transaction_version,
            genesis_hash,
            best_hash,
            (),
            (),
            (),
        ),
    );
    let sender = Sr25519Keyring::Alice.pair();
    let signature = raw_payload.using_encoded(|e| sender.sign(e));

    kintsugi_runtime::UncheckedExtrinsic::new_signed(
        call.clone(),
        sp_runtime::AccountId32::from(sender.public()).into(),
        kintsugi_runtime::Signature::Sr25519(signature.clone()),
        extra.clone(),
    )
    .into()
}

/// This is adapted from the substrate code
pub fn storage_map_final_key<H: StorageHasher>(
    pallet_prefix: &str,
    map_name: &str,
    key: impl codec::Encode,
) -> Vec<u8> {
    let key_hashed = H::hash(&key.encode());
    let pallet_prefix_hashed = frame_support::Twox128::hash(pallet_prefix.as_bytes());
    let storage_prefix_hashed = frame_support::Twox128::hash(map_name.as_bytes());

    let mut final_key =
        Vec::with_capacity(pallet_prefix_hashed.len() + storage_prefix_hashed.len() + key_hashed.as_ref().len());

    final_key.extend_from_slice(&pallet_prefix_hashed[..]);
    final_key.extend_from_slice(&storage_prefix_hashed[..]);
    final_key.extend_from_slice(key_hashed.as_ref());

    final_key
}

/// This is adapted from the substrate code
pub fn storage_double_map_final_key<H1: StorageHasher, H2: StorageHasher>(
    pallet_prefix: &str,
    map_name: &str,
    key1: impl codec::Encode,
    key2: impl codec::Encode,
) -> Vec<u8> {
    let key1_hashed = H1::hash(&key1.encode());
    let key2_hashed = H2::hash(&key2.encode());
    let pallet_prefix_hashed = frame_support::Twox128::hash(pallet_prefix.as_bytes());
    let storage_prefix_hashed = frame_support::Twox128::hash(map_name.as_bytes());

    let mut final_key = Vec::with_capacity(
        pallet_prefix_hashed.len()
            + storage_prefix_hashed.len()
            + key1_hashed.as_ref().len()
            + key2_hashed.as_ref().len(),
    );

    final_key.extend_from_slice(&pallet_prefix_hashed[..]);
    final_key.extend_from_slice(&storage_prefix_hashed[..]);
    final_key.extend_from_slice(key1_hashed.as_ref());
    final_key.extend_from_slice(key2_hashed.as_ref());

    final_key
}

pub struct DirectIssuer<Executor, RuntimeApi, PoolApi>
where
    PoolApi: ChainApi<Block = Block> + 'static,
    Executor: sc_executor::NativeExecutionDispatch + 'static,
    RuntimeApi: ConstructRuntimeApi<Block, FullClient<RuntimeApi, Executor>> + Send + Sync + 'static,
    RuntimeApi::RuntimeApi: RuntimeApiCollection<StateBackend = sc_client_api::StateBackendFor<FullBackend, Block>>,
    RuntimeApi::RuntimeApi: sp_consensus_aura::AuraApi<Block, AuraId>,
{
    pool: Arc<Pool<PoolApi>>,
    client: Arc<FullClient<RuntimeApi, Executor>>,
}

impl<Executor, RuntimeApi, PoolApi> DirectIssuer<Executor, RuntimeApi, PoolApi>
where
    PoolApi: ChainApi<Block = Block> + 'static,
    Executor: sc_executor::NativeExecutionDispatch + 'static,
    RuntimeApi: ConstructRuntimeApi<Block, FullClient<RuntimeApi, Executor>> + Send + Sync + 'static,
    RuntimeApi::RuntimeApi: RuntimeApiCollection<StateBackend = sc_client_api::StateBackendFor<FullBackend, Block>>,
    RuntimeApi::RuntimeApi: sp_consensus_aura::AuraApi<Block, AuraId>,
{
    pub async fn new(pool: Arc<Pool<PoolApi>>, client: Arc<FullClient<RuntimeApi, Executor>>) -> Self {
        //  system.account(alice)

        Self { pool, client }
    }

    fn read_storage<T: codec::Decode>(&self, address: &[u8]) -> Option<T> {
        let info = self.client.chain_info();
        let state = self.client.state_at(info.best_hash).unwrap().storage(address).unwrap();
        state.map(|x| codec::Decode::decode(&mut &x[..]).unwrap())
    }

    async fn get_nonce(&self) -> u32 {
        let alice = Sr25519Keyring::Alice.to_account_id();
        let address = storage_map_final_key::<Blake2_128Concat>("System", "Account", alice);

        self.read_storage(&address).unwrap_or_default()
    }
    async fn call(&self, call: kintsugi_runtime::RuntimeCall) {
        let info = self.client.chain_info();

        let nonce = self.get_nonce().await;

        let tx = create_extrinsic(self.client.clone(), nonce, call);

        // todo: wait until inclusion: use submit_and_watch
        let watcher = self
            .pool
            .submit_and_watch(
                &BlockId::Hash(info.best_hash),
                sp_runtime::transaction_validity::TransactionSource::Local,
                tx,
            )
            .await
            .unwrap();
        let mut stream = watcher.into_stream();
        while let Some(x) = stream.next().await {
            log::error!("Status: {:?}", x);
            if matches!(x, sc_transaction_pool_api::TransactionStatus::InBlock(_)) {
                return;
            }
        }
        panic!("tx failure");
    }
}
#[derive(Clone, Debug)]
pub struct ZeroDelay;

#[async_trait]
impl RandomDelay for ZeroDelay {
    type Error = DirectRelayError;
    async fn delay(&self, _seed_data: &[u8; 32]) -> Result<(), DirectRelayError> {
        Ok(())
    }
}

#[derive(Debug)]
pub enum DirectRelayError {
    // we could return actual errors but since we don't really expect any and it's
    // just for development, we stick with unwrapping for now
}

#[async_trait]
impl<Executor, RuntimeApi, PoolApi> Issuing for DirectIssuer<Executor, RuntimeApi, PoolApi>
where
    PoolApi: ChainApi<Block = Block> + 'static,
    Executor: sc_executor::NativeExecutionDispatch + 'static,
    RuntimeApi: ConstructRuntimeApi<Block, FullClient<RuntimeApi, Executor>> + Send + Sync + 'static,
    RuntimeApi::RuntimeApi: RuntimeApiCollection<StateBackend = sc_client_api::StateBackendFor<FullBackend, Block>>,
    RuntimeApi::RuntimeApi: sp_consensus_aura::AuraApi<Block, AuraId>,
{
    type Error = DirectRelayError;
    async fn is_initialized(&self) -> Result<bool, DirectRelayError> {
        let address = storage_prefix(b"BTCRelay", b"StartBlockHeight");
        Ok(self.read_storage::<u32>(&address).is_some())
    }

    async fn initialize(&self, header: Vec<u8>, height: u32) -> Result<(), DirectRelayError> {
        let block_header = bitcoin::parser::parse_block_header(&header).unwrap();

        let call = kintsugi_runtime::RuntimeCall::BTCRelay(kintsugi_runtime::btc_relay::Call::initialize {
            block_header,
            block_height: height,
        });

        log::error!("pre-call");
        self.call(call).await;
        log::error!("post-call");

        Ok(())
    }

    async fn submit_block_header(
        &self,
        header: Vec<u8>,
        _random_delay: Arc<Box<dyn RandomDelay<Error = Self::Error> + Send + Sync>>,
    ) -> Result<(), DirectRelayError> {
        let block_header = bitcoin::parser::parse_block_header(&header).unwrap();

        let call = kintsugi_runtime::RuntimeCall::BTCRelay(kintsugi_runtime::btc_relay::Call::store_block_header {
            block_header,
            fork_bound: 10,
        });

        self.call(call).await;

        Ok(())
    }

    async fn submit_block_header_batch(&self, headers: Vec<Vec<u8>>) -> Result<(), DirectRelayError> {
        for header in headers {
            self.submit_block_header(header, Arc::new(Box::new(ZeroDelay)))
                .await
                .unwrap();
        }
        Ok(())
    }

    async fn get_best_height(&self) -> Result<u32, DirectRelayError> {
        let address = storage_prefix(b"BTCRelay", b"BestBlockHeight");
        Ok(self.read_storage::<u32>(&address).unwrap_or_default())
    }

    async fn get_block_hash(&self, height: u32) -> Result<Vec<u8>, DirectRelayError> {
        let address = storage_double_map_final_key::<Blake2_128Concat, Blake2_128Concat>(
            "BTCRelay",
            "ChainsHashes",
            0u32,
            height,
        );

        let hash = self.read_storage::<H256Le>(&address).unwrap();
        Ok(hex::decode(hash.to_hex_le()).unwrap())
    }

    async fn is_block_stored(&self, hash_le: Vec<u8>) -> Result<bool, DirectRelayError> {
        let hash = H256Le::from_bytes_le(&hash_le);
        let key = storage_map_final_key::<Blake2_128Concat>("BTCRelay", "BlockHeaders", hash);

        // don't bother decoding, just check if it exists
        let state = self
            .client
            .state_at(self.client.chain_info().best_hash)
            .unwrap()
            .storage(&key)
            .unwrap();
        Ok(state.is_some())
    }
}
