use crate::command::IdentifyChain;
use primitives::Block;
use sc_cli::{ChainSpec, Result};
use sc_executor::NativeElseWasmExecutor;
use sp_api::ConstructRuntimeApi;
use sp_core::{Encode, Pair};
use sp_runtime::OpaqueExtrinsic;
use std::sync::Arc;

macro_rules! with_runtime {
    ($selected_runtime:ident, $codeblock:block ) => {{
        match $selected_runtime {
            SelectedRuntime::Kintsugi => {
                use kintsugi_runtime as runtime;
                $codeblock
            }
            SelectedRuntime::Interlay => {
                use interlay_runtime as runtime;
                $codeblock
            }
        }
    }};
}

pub struct RemarkBuilder<RuntimeApi, Executor>
where
    Executor: sc_executor::NativeExecutionDispatch + 'static,
{
    pub(crate) client: Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
    pub(crate) selected_runtime: SelectedRuntime,
}

impl<RuntimeApi, Executor> frame_benchmarking_cli::ExtrinsicBuilder for RemarkBuilder<RuntimeApi, Executor>
where
    RuntimeApi: ConstructRuntimeApi<Block, TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>
        + Send
        + Sync
        + 'static,
    Executor: sc_executor::NativeExecutionDispatch + 'static,
{
    fn pallet(&self) -> &str {
        "system"
    }

    fn extrinsic(&self) -> &str {
        "remark"
    }

    fn build(&self, nonce: u32) -> std::result::Result<OpaqueExtrinsic, &'static str> {
        Ok(create_extrinsic(self.client.clone(), nonce, self.selected_runtime))
    }
}

use sc_service::TFullClient;

fn create_extrinsic<Executor, RuntimeApi>(
    client: Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
    nonce: u32,
    selected_runtime: SelectedRuntime,
) -> OpaqueExtrinsic
where
    Executor: sc_executor::NativeExecutionDispatch + 'static,
{
    use sc_client_api::BlockBackend;
    use sp_keyring::Sr25519Keyring;
    use sp_runtime::SaturatedConversion;

    with_runtime!(selected_runtime, {
        let call = runtime::RuntimeCall::System(frame_system::Call::remark {
            remark: Default::default(),
        });

        let genesis_hash = client.block_hash(0).ok().flatten().expect("Genesis block exists; qed");
        let best_hash = client.chain_info().best_hash;
        let best_block = client.chain_info().best_number;

        let period = runtime::BlockHashCount::get()
            .checked_next_power_of_two()
            .map(|c| c / 2)
            .unwrap_or(2) as u64;

        let extra: runtime::SignedExtra = (
            frame_system::CheckSpecVersion::<runtime::Runtime>::new(),
            frame_system::CheckTxVersion::<runtime::Runtime>::new(),
            frame_system::CheckGenesis::<runtime::Runtime>::new(),
            frame_system::CheckEra::<runtime::Runtime>::from(sp_runtime::generic::Era::mortal(
                period,
                best_block.saturated_into(),
            )),
            frame_system::CheckNonce::<runtime::Runtime>::from(nonce),
            frame_system::CheckWeight::<runtime::Runtime>::new(),
            // frame_system::CheckNonZeroSender::<runtime::Runtime>::new(),
            pallet_transaction_payment::ChargeTransactionPayment::<runtime::Runtime>::from(0),
        );

        /// The payload being signed
        type SignedPayload = sp_runtime::generic::SignedPayload<runtime::RuntimeCall, runtime::SignedExtra>;

        let raw_payload = SignedPayload::from_raw(
            call.clone(),
            extra.clone(),
            (
                runtime::VERSION.spec_version,
                runtime::VERSION.transaction_version,
                genesis_hash,
                best_hash,
                (),
                (),
                (),
            ),
        );
        let sender = Sr25519Keyring::Alice.pair();
        let signature = raw_payload.using_encoded(|e| sender.sign(e));

        runtime::UncheckedExtrinsic::new_signed(
            call.clone(),
            sp_runtime::AccountId32::from(sender.public()).into(),
            runtime::Signature::Sr25519(signature.clone()),
            extra.clone(),
        )
        .into()
    })
}

pub fn para_benchmark_inherent_data() -> std::result::Result<sp_inherents::InherentData, sp_inherents::Error> {
    use cumulus_primitives_core::PersistedValidationData;
    use cumulus_primitives_parachain_inherent::{ParachainInherentData, INHERENT_IDENTIFIER};
    use cumulus_test_relay_sproof_builder::RelayStateSproofBuilder;
    use sp_inherents::InherentDataProvider;

    let mut inherent_data = sp_inherents::InherentData::new();

    // Assume that all runtimes have the `timestamp` pallet.
    let d = std::time::Duration::from_millis(0);
    let timestamp = sp_timestamp::InherentDataProvider::new(d.into());
    futures::executor::block_on(timestamp.provide_inherent_data(&mut inherent_data))?;

    let sproof_builder = RelayStateSproofBuilder::default();
    let (relay_parent_storage_root, relay_chain_state) = sproof_builder.into_state_root_and_proof();
    // `relay_parent_number` should be bigger than 0 for benchmarking.
    // It is mocked value, any number except 0 is valid.
    let validation_data = PersistedValidationData {
        relay_parent_number: 1,
        relay_parent_storage_root,
        ..Default::default()
    };

    // Parachain blocks needs to include ParachainInherentData, otherwise block is invalid.
    let para_data = ParachainInherentData {
        validation_data,
        relay_chain_state,
        downward_messages: Default::default(),
        horizontal_messages: Default::default(),
    };

    inherent_data.put_data(INHERENT_IDENTIFIER, &para_data)?;

    Ok(inherent_data)
}

#[derive(Clone, Copy)]
pub(crate) enum SelectedRuntime {
    Kintsugi,
    Interlay,
}

impl SelectedRuntime {
    pub(crate) fn from_chain_spec(chain_spec: &Box<dyn ChainSpec>) -> Result<Self> {
        if chain_spec.is_interlay() {
            Ok(Self::Interlay)
        } else if chain_spec.is_kintsugi() {
            Ok(Self::Kintsugi)
        } else {
            Err("Chain doesn't support benchmarking".into())
        }
    }
}
