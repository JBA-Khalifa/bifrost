// Copyright 2019-2020 Liebi Technologies.
// This file is part of Bifrost.

// Bifrost is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Bifrost is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Bifrost.  If not, see <http://www.gnu.org/licenses/>.

#![warn(unused_extern_crates)]

//! Service implementation. Specialized wrapper over substrate service.

pub mod chain_spec;
pub mod collator;
mod client;

use sc_finality_grandpa::{self as grandpa, FinalityProofProvider as GrandpaFinalityProofProvider};
use {
	std::time::Duration,
	std::sync::Arc,
	sp_trie::PrefixedMemoryDB,
	sc_client_api::ExecutorProvider,
	tracing::info,
};
use sc_executor::native_executor_instance;

pub use self::client::{AbstractClient, Client, ClientHandle, ExecuteWithClient, RuntimeApiCollection};
pub use polkadot_primitives::v1::{Block, BlockId, CollatorId, Hash, Id as ParaId};
pub use sc_client_api::{Backend, ExecutionStrategy, CallExecutor};
pub use sc_consensus::LongestChain;
pub use sc_executor::NativeExecutionDispatch;
pub use sc_service::{
	Role, PruningMode, TransactionPoolOptions, Error as ServiceError, RuntimeGenesis,
	TFullClient, TLightClient, TFullBackend, TLightBackend, TFullCallExecutor, TLightCallExecutor,
	Configuration, ChainSpec, TaskManager,
};
pub use sp_api::{ApiRef, Core as CoreApi, ConstructRuntimeApi, ProvideRuntimeApi, StateBackend};
pub use sp_runtime::traits::{DigestFor, HashFor, NumberFor, Block as BlockT, self as runtime_traits, BlakeTwo256};

pub use asgard_runtime;
pub use bifrost_runtime;
pub use rococo_runtime;

native_executor_instance!(
	pub AsgardExecutor,
	asgard_runtime::api::dispatch,
	asgard_runtime::native_version,
	frame_benchmarking::benchmarking::HostFunctions,
);

native_executor_instance!(
	pub BifrostExecutor,
	bifrost_runtime::api::dispatch,
	bifrost_runtime::native_version,
	frame_benchmarking::benchmarking::HostFunctions,
);

native_executor_instance!(
	pub RococoExecutor,
	rococo_runtime::api::dispatch,
	rococo_runtime::native_version,
	frame_benchmarking::benchmarking::HostFunctions,
);

/// Can be called for a `Configuration` to check if it is a configuration for the `Bifrost` network.
pub trait IdentifyVariant {
	/// Returns if this is a configuration for the `Asgard` network.
	fn is_asgard(&self) -> bool;

	/// Returns if this is a configuration for the `Bifrost` network.
	fn is_bifrost(&self) -> bool;

	/// Returns if this is a configuration for the `Rococo` network.
	fn is_rococo(&self) -> bool;
}

impl IdentifyVariant for Box<dyn ChainSpec> {
	fn is_asgard(&self) -> bool {
		self.id().starts_with("asgard") || self.id().starts_with("asg")
	}
	fn is_bifrost(&self) -> bool {
		self.id().starts_with("bifrost") || self.id().starts_with("bnc")
	}
	fn is_rococo(&self) -> bool {
		self.id().starts_with("bifrost") || self.id().starts_with("roc")
	}
}

pub type FullBackend = sc_service::TFullBackend<Block>;
type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;
pub type FullClient<RuntimeApi, Executor> = sc_service::TFullClient<Block, RuntimeApi, Executor>;
type FullGrandpaBlockImport<RuntimeApi, Executor> = grandpa::GrandpaBlockImport<
	FullBackend, Block, FullClient<RuntimeApi, Executor>, FullSelectChain
>;

type LightBackend = sc_service::TLightBackendWithHash<Block, sp_runtime::traits::BlakeTwo256>;
type LightClient<RuntimeApi, Executor> =
	sc_service::TLightClientWithBackend<Block, RuntimeApi, Executor, LightBackend>;

pub fn new_partial<RuntimeApi, Executor>(
	config: &mut Configuration
) -> Result<
		sc_service::PartialComponents<
		FullClient<RuntimeApi, Executor>, FullBackend, FullSelectChain,
		sp_consensus::DefaultImportQueue<Block, FullClient<RuntimeApi, Executor>>,
		sc_transaction_pool::FullPool<Block, FullClient<RuntimeApi, Executor>>,
		(
			impl Fn(
				node_rpc::DenyUnsafe,
				sc_rpc::SubscriptionTaskExecutor,
			) -> node_rpc::RpcExtension,
			(
				sc_consensus_babe::BabeBlockImport<
					Block, FullClient<RuntimeApi, Executor>, FullGrandpaBlockImport<RuntimeApi, Executor>
				>,
				grandpa::LinkHalf<Block, FullClient<RuntimeApi, Executor>, FullSelectChain>,
				sc_consensus_babe::BabeLink<Block>
			),
			grandpa::SharedVoterState,
		)
	>,
	ServiceError
>
	where
		RuntimeApi: ConstructRuntimeApi<Block, FullClient<RuntimeApi, Executor>> + Send + Sync + 'static,
		RuntimeApi::RuntimeApi:
		RuntimeApiCollection<StateBackend = sc_client_api::StateBackendFor<FullBackend, Block>>,
		Executor: NativeExecutionDispatch + 'static,
{
	let (client, backend, keystore_container, task_manager) =
		sc_service::new_full_parts::<Block, RuntimeApi, Executor>(&config)?;
	let client = Arc::new(client);

	let select_chain = sc_consensus::LongestChain::new(backend.clone());

	let transaction_pool = sc_transaction_pool::BasicPool::new_full(
		config.transaction_pool.clone(),
		config.prometheus_registry(),
		task_manager.spawn_handle(),
		client.clone(),
	);

	let (grandpa_block_import, grandpa_link) = grandpa::block_import(
		client.clone(), &(client.clone() as Arc<_>), select_chain.clone(),
	)?;
	let justification_import = grandpa_block_import.clone();

	let (block_import, babe_link) = sc_consensus_babe::block_import(
		sc_consensus_babe::Config::get_or_compute(&*client)?,
		grandpa_block_import,
		client.clone(),
	)?;

	let inherent_data_providers = sp_inherents::InherentDataProviders::new();

	let import_queue = sc_consensus_babe::import_queue(
		babe_link.clone(),
		block_import.clone(),
		Some(Box::new(justification_import)),
		client.clone(),
		select_chain.clone(),
		inherent_data_providers.clone(),
		&task_manager.spawn_handle(),
		config.prometheus_registry(),
		sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone()),
	)?;

	let justification_stream = grandpa_link.justification_stream();
	let shared_authority_set = grandpa_link.shared_authority_set().clone();
	let shared_voter_state = grandpa::SharedVoterState::empty();
	let finality_proof_provider =
		GrandpaFinalityProofProvider::new_for_service(backend.clone(), client.clone());

	let import_setup = (block_import.clone(), grandpa_link, babe_link.clone());
	let rpc_setup = shared_voter_state.clone();

	let babe_config = babe_link.config().clone();
	let shared_epoch_changes = babe_link.epoch_changes().clone();

	let rpc_extensions_builder = {
		let client = client.clone();
		let keystore = keystore_container.sync_keystore();
		let transaction_pool = transaction_pool.clone();
		let select_chain = select_chain.clone();
		let chain_spec = config.chain_spec.cloned_box();

		move |deny_unsafe, subscription_executor| -> node_rpc::RpcExtension {
			let deps = node_rpc::FullDeps {
				client: client.clone(),
				pool: transaction_pool.clone(),
				select_chain: select_chain.clone(),
				chain_spec: chain_spec.cloned_box(),
				deny_unsafe,
				babe: node_rpc::BabeDeps {
					babe_config: babe_config.clone(),
					shared_epoch_changes: shared_epoch_changes.clone(),
					keystore: keystore.clone(),
				},
				grandpa: node_rpc::GrandpaDeps {
					shared_voter_state: shared_voter_state.clone(),
					shared_authority_set: shared_authority_set.clone(),
					justification_stream: justification_stream.clone(),
					subscription_executor,
					finality_provider: finality_proof_provider.clone(),
				},
			};

			node_rpc::create_full(deps)
		}
	};

	Ok(sc_service::PartialComponents {
		client,
		backend,
		task_manager,
		keystore_container,
		select_chain,
		import_queue,
		transaction_pool,
		inherent_data_providers,
		other: (rpc_extensions_builder, import_setup, rpc_setup)
	})
}

pub struct NewFull<C> {
	pub task_manager: TaskManager,
	pub client: C,
	pub network: Arc<sc_network::NetworkService<Block, <Block as BlockT>::Hash>>,
	pub network_status_sinks: sc_service::NetworkStatusSinks<Block>,
	pub rpc_handlers: sc_service::RpcHandlers,
	pub backend: Arc<FullBackend>,
}

impl<C> NewFull<C> {
	/// Convert the client type using the given `func`.
	pub fn with_client<NC>(self, func: impl FnOnce(C) -> NC) -> NewFull<NC> {
		NewFull {
			client: func(self.client),
			task_manager: self.task_manager,
			network: self.network,
			network_status_sinks: self.network_status_sinks,
			rpc_handlers: self.rpc_handlers,
			backend: self.backend,
		}
	}
}

/// Create a new full node of arbitrary runtime and executor.
///
/// This is an advanced feature and not recommended for general use. Generally, `build_full` is
/// a better choice.
pub fn new_full<RuntimeApi, Executor>(
	mut config: Configuration,
	grandpa_pause: Option<(u32, u32)>,
) -> Result<NewFull<Arc<FullClient<RuntimeApi, Executor>>>, ServiceError>
	where
		RuntimeApi: ConstructRuntimeApi<Block, FullClient<RuntimeApi, Executor>> + Send + Sync + 'static,
		RuntimeApi::RuntimeApi:
		RuntimeApiCollection<StateBackend = sc_client_api::StateBackendFor<FullBackend, Block>>,
		Executor: NativeExecutionDispatch + 'static,
{
	let role = config.role.clone();
	let force_authoring = config.force_authoring;
	let backoff_authoring_blocks =
		Some(sc_consensus_slots::BackoffAuthoringOnFinalizedHeadLagging::default());
	let disable_grandpa = config.disable_grandpa;
	let name = config.network.node_name.clone();

	let sc_service::PartialComponents {
		client,
		backend,
		mut task_manager,
		keystore_container,
		select_chain,
		import_queue,
		transaction_pool,
		inherent_data_providers,
		other: (rpc_extensions_builder, import_setup, rpc_setup)
	} = new_partial::<RuntimeApi, Executor>(&mut config)?;

	let prometheus_registry = config.prometheus_registry().cloned();

	let shared_voter_state = rpc_setup;

	// Note: GrandPa is pushed before the Polkadot-specific protocols. This doesn't change
	// anything in terms of behaviour, but makes the logs more consistent with the other
	// Substrate nodes.
	config.network.extra_sets.push(grandpa::grandpa_peers_set_config());

	let (network, network_status_sinks, system_rpc_tx, network_starter) =
		sc_service::build_network(sc_service::BuildNetworkParams {
			config: &config,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			spawn_handle: task_manager.spawn_handle(),
			import_queue,
			on_demand: None,
			block_announce_validator_builder: None,
		})?;

	if config.offchain_worker.enabled {
		let _ = sc_service::build_offchain_workers(
			&config, backend.clone(), task_manager.spawn_handle(), client.clone(), network.clone(),
		);
	}

	let telemetry_connection_sinks = sc_service::TelemetryConnectionSinks::default();

	let rpc_handlers = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
		config,
		backend: backend.clone(),
		client: client.clone(),
		keystore: keystore_container.sync_keystore(),
		network: network.clone(),
		rpc_extensions_builder: Box::new(rpc_extensions_builder),
		transaction_pool: transaction_pool.clone(),
		task_manager: &mut task_manager,
		on_demand: None,
		remote_blockchain: None,
		telemetry_connection_sinks: telemetry_connection_sinks.clone(),
		network_status_sinks: network_status_sinks.clone(),
		system_rpc_tx,
	})?;

	let (block_import, link_half, babe_link) = import_setup;

	// we'd say let overseer_handler = authority_discovery_service.map(|authority_discovery_service|, ...),
	// but in that case we couldn't use ? to propagate errors

	if role.is_authority() {
		let can_author_with =
			sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone());

		let proposer = sc_basic_authorship::ProposerFactory::new(
			task_manager.spawn_handle(),
			client.clone(),
			transaction_pool,
			prometheus_registry.as_ref(),
		);

		let babe_config = sc_consensus_babe::BabeParams {
			keystore: keystore_container.sync_keystore(),
			client: client.clone(),
			select_chain,
			block_import,
			env: proposer,
			sync_oracle: network.clone(),
			inherent_data_providers: inherent_data_providers.clone(),
			force_authoring,
			backoff_authoring_blocks,
			babe_link,
			can_author_with,
		};

		let babe = sc_consensus_babe::start_babe(babe_config)?;
		task_manager.spawn_essential_handle().spawn_blocking("babe", babe);
	}

	// if the node isn't actively participating in consensus then it doesn't
	// need a keystore, regardless of which protocol we use below.
	let keystore_opt = if role.is_authority() {
		Some(keystore_container.sync_keystore())
	} else {
		None
	};

	let config = grandpa::Config {
		// FIXME substrate#1578 make this available through chainspec
		gossip_duration: Duration::from_millis(1000),
		justification_period: 512,
		name: Some(name),
		observer_enabled: false,
		keystore: keystore_opt,
		is_authority: role.is_network_authority(),
	};

	let enable_grandpa = !disable_grandpa;
	if enable_grandpa {
		// start the full GRANDPA voter
		// NOTE: unlike in substrate we are currently running the full
		// GRANDPA voter protocol for all full nodes (regardless of whether
		// they're validators or not). at this point the full voter should
		// provide better guarantees of block and vote data availability than
		// the observer.

		// add a custom voting rule to temporarily stop voting for new blocks
		// after the given pause block is finalized and restarting after the
		// given delay.
		let voting_rule = match grandpa_pause {
			Some((block, delay)) => {
				info!(
					block_number = %block,
					delay = %delay,
					"GRANDPA scheduled voting pause set for block #{} with a duration of {} blocks.",
					block,
					delay,
				);

				grandpa::VotingRulesBuilder::default()
					// .add(grandpa_support::PauseAfterBlockFor(block, delay))
					.build()
			}
			None => grandpa::VotingRulesBuilder::default().build(),
		};

		let grandpa_config = grandpa::GrandpaParams {
			config,
			link: link_half,
			network: network.clone(),
			telemetry_on_connect: Some(telemetry_connection_sinks.on_connect_stream()),
			voting_rule,
			prometheus_registry: prometheus_registry.clone(),
			shared_voter_state,
		};

		task_manager.spawn_essential_handle().spawn_blocking(
			"grandpa-voter",
			grandpa::run_grandpa_voter(grandpa_config)?
		);
	}

	network_starter.start_network();

	Ok(NewFull {
		task_manager,
		client,
		network,
		network_status_sinks,
		rpc_handlers,
		backend,
	})
}

/// Builds a new service for a light client.
fn new_light<Runtime, Dispatch>(config: Configuration) -> Result<(TaskManager, sc_service::RpcHandlers), ServiceError>
	where
		Runtime: 'static + Send + Sync + ConstructRuntimeApi<Block, LightClient<Runtime, Dispatch>>,
		<Runtime as ConstructRuntimeApi<Block, LightClient<Runtime, Dispatch>>>::RuntimeApi:
		RuntimeApiCollection<StateBackend = sc_client_api::StateBackendFor<LightBackend, Block>>,
		Dispatch: NativeExecutionDispatch + 'static,
{
	use sc_client_api::backend::RemoteBackend;

	let (client, backend, keystore_container, mut task_manager, on_demand) =
		sc_service::new_light_parts::<Block, Runtime, Dispatch>(&config)?;

	let select_chain = sc_consensus::LongestChain::new(backend.clone());

	let transaction_pool = Arc::new(sc_transaction_pool::BasicPool::new_light(
		config.transaction_pool.clone(),
		config.prometheus_registry(),
		task_manager.spawn_handle(),
		client.clone(),
		on_demand.clone(),
	));

	let (grandpa_block_import, _) = grandpa::block_import(
		client.clone(),
		&(client.clone() as Arc<_>),
		select_chain.clone(),
	)?;
	let justification_import = grandpa_block_import.clone();

	let (babe_block_import, babe_link) = sc_consensus_babe::block_import(
		sc_consensus_babe::Config::get_or_compute(&*client)?,
		grandpa_block_import,
		client.clone(),
	)?;

	let inherent_data_providers = sp_inherents::InherentDataProviders::new();

	let import_queue = sc_consensus_babe::import_queue(
		babe_link,
		babe_block_import,
		Some(Box::new(justification_import)),
		client.clone(),
		select_chain.clone(),
		inherent_data_providers.clone(),
		&task_manager.spawn_handle(),
		config.prometheus_registry(),
		sp_consensus::NeverCanAuthor,
	)?;

	let (network, network_status_sinks, system_rpc_tx, network_starter) =
		sc_service::build_network(sc_service::BuildNetworkParams {
			config: &config,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			spawn_handle: task_manager.spawn_handle(),
			import_queue,
			on_demand: Some(on_demand.clone()),
			block_announce_validator_builder: None,
		})?;

	if config.offchain_worker.enabled {
		let _ = sc_service::build_offchain_workers(
			&config,
			backend.clone(),
			task_manager.spawn_handle(),
			client.clone(),
			network.clone(),
		);
	}

	let light_deps = node_rpc::LightDeps {
		remote_blockchain: backend.remote_blockchain(),
		fetcher: on_demand.clone(),
		client: client.clone(),
		pool: transaction_pool.clone(),
	};

	let rpc_extensions = node_rpc::create_light(light_deps);

	let rpc_handlers = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
		on_demand: Some(on_demand),
		remote_blockchain: Some(backend.remote_blockchain()),
		rpc_extensions_builder: Box::new(sc_service::NoopRpcExtensionBuilder(rpc_extensions)),
		task_manager: &mut task_manager,
		telemetry_connection_sinks: sc_service::TelemetryConnectionSinks::default(),
		config,
		keystore: keystore_container.sync_keystore(),
		backend,
		transaction_pool,
		client,
		network,
		network_status_sinks,
		system_rpc_tx,
	})?;

	network_starter.start_network();

	Ok((task_manager, rpc_handlers))
}

/// Builds a new object suitable for chain operations.
pub fn new_chain_ops(mut config: &mut Configuration) -> Result<
	(
		Arc<Client>,
		Arc<FullBackend>,
		sp_consensus::import_queue::BasicQueue<Block, PrefixedMemoryDB<BlakeTwo256>>,
		TaskManager,
	),
	ServiceError
>
{
	config.keystore = sc_service::config::KeystoreConfig::InMemory;
	if config.chain_spec.is_asgard() {
		let sc_service::PartialComponents { client, backend, import_queue, task_manager, .. }
			= new_partial::<asgard_runtime::RuntimeApi, AsgardExecutor>(config)?;
		Ok((Arc::new(Client::Asgard(client)), backend, import_queue, task_manager))
	} else if config.chain_spec.is_bifrost() {
		let sc_service::PartialComponents { client, backend, import_queue, task_manager, .. }
			= new_partial::<bifrost_runtime::RuntimeApi, BifrostExecutor>(config)?;
		Ok((Arc::new(Client::Bifrost(client)), backend, import_queue, task_manager))
	} else {
		let sc_service::PartialComponents { client, backend, import_queue, task_manager, .. }
			= new_partial::<bifrost_runtime::RuntimeApi, BifrostExecutor>(config)?;
		Ok((Arc::new(Client::Bifrost(client)), backend, import_queue, task_manager))
	}
}

/// Build a new light node.
pub fn build_light(config: Configuration) -> Result<(TaskManager, sc_service::RpcHandlers), ServiceError> {
	if config.chain_spec.is_asgard() {
		new_light::<asgard_runtime::RuntimeApi, AsgardExecutor>(config)
	} else if config.chain_spec.is_bifrost() {
		new_light::<bifrost_runtime::RuntimeApi, BifrostExecutor>(config)
	} else {
		new_light::<bifrost_runtime::RuntimeApi, BifrostExecutor>(config)
	}
}

pub fn build_full(
	config: Configuration,
	grandpa_pause: Option<(u32, u32)>,
) -> Result<TaskManager, ServiceError> {
	if config.chain_spec.is_asgard() {
		new_full::<asgard_runtime::RuntimeApi, AsgardExecutor>(
			config,
			grandpa_pause,
		).map(|full| full.task_manager)
	} else {
		new_full::<bifrost_runtime::RuntimeApi, BifrostExecutor>(
			config,
			grandpa_pause,
		).map(|full| full.task_manager)
	}
}
