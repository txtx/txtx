use std::{collections::HashMap, fs::File, thread::sleep, time::Duration};

use bollard::{
    container::{Config, CreateContainerOptions, RemoveContainerOptions},
    image::CommitContainerOptions,
    network::CreateNetworkOptions,
    secret::{
        ContainerCreateResponse, ContainerStateStatusEnum, HostConfig, Mount, MountTypeEnum,
        PortBinding,
    },
    volume::CreateVolumeOptions,
    Docker,
};
use serde_json::Value as JsonValue;
use txtx_addon_kit::{
    indexmap::IndexMap,
    reqwest::Url,
    types::{
        diagnostics::Diagnostic,
        types::{ObjectType, Value},
    },
};

use crate::constants::{
    DEFAULT_OP_NODE_RPC_KIND, DEFAULT_TAG, OP_BATCHER_IMAGE, OP_GETH_IMAGE, OP_NODE_IMAGE,
    OP_PROPOSER_IMAGE,
};

use super::rollup_config::RollupConfig;

pub const OP_GETH_CONTAINER_NAME: &str = "op-geth";
pub const OP_NODE_CONTAINER_NAME: &str = "op-node";
pub const OP_BATCHER_CONTAINER_NAME: &str = "op-batcher";
pub const OP_PROPOSER_CONTAINER_NAME: &str = "op-proposer";
pub const OP_NODE_RPC_PORT: &str = "9545";
pub const OP_GETH_AUTH_RPC_PORT: &str = "8551";
pub const OP_GETH_RPC_PORT: &str = "8545";
pub const OP_GETH_WS_PORT: &str = "8546";
pub const OP_BATCHER_RPC_PORT: &str = "8548";
pub const OP_PROPOSER_RPC_PORT: &str = "8560";

#[derive(Debug, Clone)]
pub struct RollupDeployer {
    pub network_name: String,
    pub l1_rpc_api_url: Url,
    pub l1_rpc_kind: String,
    pub working_dir: String,
    pub l1_deployment_addresses: IndexMap<String, Value>,
    rollup_config: RollupConfig,
    sequencer_secret_key: String,
    batcher_secret_key: String,
    proposer_secret_key: String,
    jwt: String,
    docker: Docker,
    network_id: Option<String>,
    op_geth_container_id: Option<String>,
    op_node_container_id: Option<String>,
    op_batcher_container_id: Option<String>,
    op_proposer_container_id: Option<String>,
    datadir_mount: Option<Mount>,
    conf_mount: Option<Mount>,
}

impl RollupDeployer {
    pub fn new(
        network_name: &str,
        working_dir: &str,
        l1_rpc_api_url: &str,
        l1_rpc_kind: Option<&str>,
        rollup_config: &RollupConfig,
        l1_deployment_addresses: &IndexMap<String, Value>,
        sequencer_secret_key: &str,
        batcher_secret_key: &str,
        proposer_secret_key: &str,
        jwt: &str,
    ) -> Result<Self, Diagnostic> {
        let docker = Docker::connect_with_socket_defaults()
            .map_err(|e| diagnosed_error!("Failed to connect to Docker: {}", e))?;

        let url = Url::parse(l1_rpc_api_url)
            .map_err(|e| diagnosed_error!("invalid L1 RPC API URL: {}", e))?;
        Ok(Self {
            network_name: network_name.to_string(),
            l1_rpc_api_url: url,
            l1_rpc_kind: l1_rpc_kind
                .map(|s| s.to_string())
                .unwrap_or(DEFAULT_OP_NODE_RPC_KIND.to_string()),
            sequencer_secret_key: sequencer_secret_key.to_string(),
            batcher_secret_key: batcher_secret_key.to_string(),
            proposer_secret_key: proposer_secret_key.to_string(),
            jwt: jwt.to_string(),
            working_dir: working_dir.to_string(),
            rollup_config: rollup_config.clone(),
            l1_deployment_addresses: l1_deployment_addresses.clone(),
            docker,
            network_id: None,
            op_geth_container_id: None,
            op_node_container_id: None,
            op_batcher_container_id: None,
            op_proposer_container_id: None,
            datadir_mount: None,
            conf_mount: None,
        })
    }

    pub fn get_container_ids(&self) -> Value {
        ObjectType::from(vec![
            ("op_geth_container_id", Value::string(self.op_geth_container_id.clone().unwrap())),
            ("op_node_container_id", Value::string(self.op_node_container_id.clone().unwrap())),
            (
                "op_batcher_container_id",
                Value::string(self.op_batcher_container_id.clone().unwrap()),
            ),
            (
                "op_proposer_container_id",
                Value::string(self.op_proposer_container_id.clone().unwrap()),
            ),
        ])
        .to_value()
    }

    pub async fn init(&mut self) -> Result<(), Diagnostic> {
        self.initialize_docker_network().await?;
        self.prep_host_fs()?;
        self.create_l2_config_files().await.map_err(|e| {
            diagnosed_error!(
                "failed to create L2 config files for rollup deployment: {}",
                e.message
            )
        })?;
        self.initialize_op_geth().await.map_err(|e| {
            diagnosed_error!("failed to initialize op-geth for rollup deployment: {}", e.message)
        })?;
        self.prep_volumes().await.map_err(|e| {
            diagnosed_error!("failed to prep volumes for rollup deployment: {}", e.message)
        })?;
        Ok(())
    }

    pub async fn start(&mut self) -> Result<(), Diagnostic> {
        self.start_op_geth().await.map_err(|e| {
            diagnosed_error!("failed to start op-geth for rollup deployment: {}", e.message)
        })?;
        self.start_op_node().await.map_err(|e| {
            diagnosed_error!("failed to start op-node for rollup deployment: {}", e.message)
        })?;
        self.start_batcher().await.map_err(|e| {
            diagnosed_error!("failed to start op-batcher for rollup deployment: {}", e.message)
        })?;
        self.start_proposer().await.map_err(|e| {
            diagnosed_error!("failed to start op-proposer for rollup deployment: {}", e.message)
        })?;
        Ok(())
    }

    pub async fn check_ready_state(&self) -> Result<bool, Diagnostic> {
        todo!()
    }

    async fn initialize_docker_network(&mut self) -> Result<(), Diagnostic> {
        let mut labels = HashMap::new();
        labels.insert("project", self.network_name.as_str());

        let network_id = self
            .docker
            .create_network::<&str>(CreateNetworkOptions {
                name: &self.network_name,
                driver: "bridge",
                labels,
                // options,
                ..Default::default()
            })
            .await
            .map_err(|e| {
                diagnosed_error!(
                    "Unable to create a Docker network. Is Docker running locally? (error: {})",
                    e
                )
            })?
            .id;

        self.network_id = network_id;

        self.datadir_mount = Some(self.create_volume("datadir").await?);
        self.conf_mount = Some(self.create_volume("conf").await?);

        Ok(())
    }

    fn prep_host_fs(&self) -> Result<(), Diagnostic> {
        std::fs::create_dir_all(&self.working_dir).map_err(|e| {
            diagnosed_error!("Failed to create working directory for RollupDeployer: {}", e)
        })?;
        std::fs::File::create(format!("{}/genesis.json", self.working_dir)).map_err(|e| {
            diagnosed_error!("Failed to create genesis.json file for RollupDeployer: {}", e)
        })?;
        std::fs::File::create(format!("{}/rollup.json", self.working_dir)).map_err(|e| {
            diagnosed_error!("Failed to create rollup.json file for RollupDeployer: {}", e)
        })?;
        std::fs::write(format!("{}/jwt.txt", self.working_dir), self.jwt.clone())
            .map_err(|e| diagnosed_error!("Failed to write jwt to jwt.txt: {}", e))?;
        std::fs::create_dir_all(format!("{}/datadir", self.working_dir)).map_err(|e| {
            diagnosed_error!("Failed to create datadir directory for RollupDeployer: {}", e)
        })?;
        Ok(())
    }

    async fn create_l2_config_files(&self) -> Result<(), Diagnostic> {
        if self.network_id.is_none() {
            return Err(diagnosed_error!("Network not initialized"));
        }

        let deploy_config_data = serde_json::to_string(&self.rollup_config)
            .map_err(|e| diagnosed_error!("failed to serialize deployment config: {}", e))?;

        let deploy_config_mount = self
            .create_tmp_file("deploy-config.json", "/conf", &deploy_config_data)
            .map_err(|e| diagnosed_error!("failed to create deploy-config.json: {}", e))?;

        let deployments_data = serde_json::to_string(
            &self
                .l1_deployment_addresses
                .iter()
                .map(|(k, v)| (k, v.to_json(None)))
                .collect::<IndexMap<&String, JsonValue>>(),
        )
        .map_err(|e| diagnosed_error!("failed to serialize L1 deployment addresses: {}", e))?;

        let deployments_mount = self
            .create_tmp_file("deployments.json", "/conf", &deployments_data)
            .map_err(|e| diagnosed_error!("failed to create deployments.json: {}", e))?;

        let options =
            CreateContainerOptions { platform: Some("linux/amd64"), ..Default::default() };

        let l1_rpc_api_url = self.get_l1_rpc_api_url();

        let config = Config {
            image: Some(format!("{}:{}", OP_NODE_IMAGE, DEFAULT_TAG)),
            cmd: Some(vec![
                "./usr/local/bin/op-node".into(),
                "genesis".into(),
                "l2".into(),
                "--deploy-config".into(),
                "/conf/deploy-config.json".into(),
                "--l1-deployments".into(),
                "/conf/deployments.json".into(),
                "--outfile.l2".into(),
                "/host/genesis.json".into(),
                "--outfile.rollup".into(),
                "/host/rollup.json".into(),
                "--l1-rpc".into(),
                l1_rpc_api_url,
            ]),

            host_config: Some(HostConfig {
                auto_remove: Some(false),
                network_mode: Some(self.network_name.clone()),
                extra_hosts: Some(vec!["host.docker.internal:host-gateway".into()]),
                mounts: Some(vec![
                    deploy_config_mount,
                    deployments_mount,
                    Mount {
                        target: Some(format!("/host")),
                        source: Some(self.working_dir.clone()),
                        typ: Some(MountTypeEnum::BIND),
                        ..Default::default()
                    },
                ]),
                ..Default::default()
            }),
            ..Default::default()
        };

        let ContainerCreateResponse { id: container_id, .. } =
            self.docker.create_container(Some(options), config).await.map_err(|e| {
                diagnosed_error!(
                    "Failed to create a container for generating L2 config files: {}",
                    e
                )
            })?;

        self.docker.start_container::<String>(&container_id, None).await.map_err(|e| {
            diagnosed_error!("Failed to start the container for generating L2 config files: {}", e)
        })?;

        self.wait_for_container_exit(&container_id).await.map_err(|e| {
            diagnosed_error!("Failed to wait for create l2 config files container to exit: {}", e)
        })?;

        self.docker
            .remove_container(
                &container_id,
                Some(RemoveContainerOptions { v: false, force: false, ..Default::default() }),
            )
            .await
            .map_err(|e| {
                diagnosed_error!(
                    "Failed to remove the container for generating L2 config files: {}",
                    e
                )
            })?;

        Ok(())
    }

    async fn initialize_op_geth(&self) -> Result<(), Diagnostic> {
        if self.network_id.is_none() {
            return Err(diagnosed_error!("Network not initialized"));
        }

        let options =
            CreateContainerOptions { platform: Some("linux/amd64"), ..Default::default() };

        let config = Config {
            image: Some(format!("{}:{}", OP_GETH_IMAGE, DEFAULT_TAG)),
            cmd: Some(vec![
                "init".into(),
                "--datadir".into(),
                "/host/datadir".into(),
                "--state.scheme".into(),
                "hash".into(),
                "/host/genesis.json".into(),
            ]),

            host_config: Some(HostConfig {
                auto_remove: Some(false),
                network_mode: Some(self.network_name.clone()),
                extra_hosts: Some(vec!["host.docker.internal:host-gateway".into()]),
                mounts: Some(vec![Mount {
                    target: Some(format!("/host")),
                    source: Some(self.working_dir.clone()),
                    typ: Some(MountTypeEnum::BIND),
                    ..Default::default()
                }]),
                ..Default::default()
            }),
            ..Default::default()
        };

        let ContainerCreateResponse { id: container_id, .. } =
            self.docker.create_container(Some(options), config).await.map_err(|e| {
                diagnosed_error!(
                    "Failed to create a container for initializing op-geth files: {}",
                    e
                )
            })?;

        self.docker.start_container::<String>(&container_id, None).await.map_err(|e| {
            diagnosed_error!("Failed to start the container for initializing op-geth: {}", e)
        })?;

        self.wait_for_container_exit(&container_id).await.map_err(|e| {
            diagnosed_error!(
                "Failed to wait for the container for initializing op-geth to exit: {}",
                e
            )
        })?;

        self.docker
            .remove_container(
                &container_id,
                Some(RemoveContainerOptions { v: false, force: false, ..Default::default() }),
            )
            .await
            .map_err(|e| {
                diagnosed_error!("Failed to remove the container for initializing op-geth: {}", e)
            })?;

        Ok(())
    }

    async fn prep_volumes(&self) -> Result<(), Diagnostic> {
        if self.network_id.is_none() {
            return Err(diagnosed_error!("Network not initialized"));
        }
        let Some(datadir_mount) = self.datadir_mount.clone() else {
            return Err(diagnosed_error!("datadir mount not created"));
        };
        let Some(conf_mount) = self.conf_mount.clone() else {
            return Err(diagnosed_error!("conf mount not created"));
        };

        let config = Config {
            image: Some("alpine:latest"),
            host_config: Some(HostConfig {
                mounts: Some(vec![
                    datadir_mount,
                    conf_mount,
                    Mount {
                        target: Some(format!("/host")),
                        source: Some(self.working_dir.clone()),
                        typ: Some(MountTypeEnum::BIND),
                        ..Default::default()
                    },
                ]),
                ..Default::default()
            }),
            cmd: Some(vec![
                "sh",
                "-c",
                "cp -Rv /host/datadir/* /datadir && \
                cp -v /host/rollup.json /conf/rollup.json && \
                cp -v /host/jwt.txt /conf/jwt.txt",
            ]),
            ..Default::default()
        };

        let ContainerCreateResponse { id: container_id, .. } = self
            .docker
            .create_container::<&str, &str>(None, config)
            .await
            .map_err(|e| diagnosed_error!("Failed to create container to prep volumes: {e}"))?;

        self.docker
            .start_container::<String>(&container_id, None)
            .await
            .map_err(|e| diagnosed_error!("Failed to start container to prep volumes: {e}"))?;

        self.wait_for_container_exit(&container_id).await.map_err(|e| {
            diagnosed_error!("Failed to wait for the container for prepping volumes to exit: {}", e)
        })?;

        self.docker
            .remove_container(
                &container_id,
                Some(RemoveContainerOptions { v: false, force: false, ..Default::default() }),
            )
            .await
            .map_err(|e| {
                diagnosed_error!("Failed to remove the container for prepping volumes: {}", e)
            })?;
        Ok(())
    }

    async fn start_op_geth(&mut self) -> Result<(), Diagnostic> {
        if self.network_id.is_none() {
            return Err(diagnosed_error!("Network not initialized"));
        }
        let Some(datadir_mount) = self.datadir_mount.clone() else {
            return Err(diagnosed_error!("datadir mount not created"));
        };
        let Some(conf_mount) = self.conf_mount.clone() else {
            return Err(diagnosed_error!("conf mount not created"));
        };

        let options = CreateContainerOptions {
            platform: Some("linux/amd64"),
            name: OP_GETH_CONTAINER_NAME,
            ..Default::default()
        };

        let exposed_ports = RollupDeployer::generate_exposed_ports(&vec![
            OP_GETH_WS_PORT,
            OP_GETH_AUTH_RPC_PORT,
            OP_GETH_RPC_PORT,
        ]);
        let port_bindings = RollupDeployer::generate_port_bindings(&vec![OP_GETH_RPC_PORT]);

        let l2_chain_id = self.rollup_config.l2_chain_id;

        let config = Config {
            image: Some(format!("{}:{}", OP_GETH_IMAGE, DEFAULT_TAG)),
            cmd: Some(vec![
                "--datadir".into(),
                "/datadir".into(),
                "--http".into(),
                "--http.corsdomain".into(),
                "*".into(),
                "--http.vhosts".into(),
                "*".into(),
                "--http.addr".into(),
                "0.0.0.0".into(),
                "--http.port".into(),
                OP_GETH_RPC_PORT.into(),
                "--http.api".into(),
                "web3,debug,eth,txpool,net,engine".into(),
                "--ws".into(),
                "--ws.addr".into(),
                "0.0.0.0".into(),
                "--ws.port".into(),
                OP_GETH_WS_PORT.into(),
                "--ws.origins".into(),
                "*".into(),
                "--ws.api".into(),
                "debug,eth,txpool,net,engine".into(),
                "--syncmode".into(),
                "full".into(),
                "--gcmode".into(),
                "archive".into(),
                "--nodiscover".into(),
                "--maxpeers".into(),
                "0".into(),
                "--networkid".into(),
                format!("{}", l2_chain_id),
                "--authrpc.vhosts".into(),
                "*".into(),
                "--authrpc.addr".into(),
                "0.0.0.0".into(),
                "--authrpc.port".into(),
                OP_GETH_AUTH_RPC_PORT.into(),
                "--authrpc.jwtsecret".into(),
                "/conf/jwt.txt".into(),
                "--rollup.disabletxpoolgossip".into(),
                "--state.scheme".into(),
                "hash".into(),
            ]),
            exposed_ports: Some(exposed_ports),
            host_config: Some(HostConfig {
                auto_remove: Some(false),
                network_mode: Some(self.network_name.clone()),
                port_bindings: Some(port_bindings),
                extra_hosts: Some(vec!["host.docker.internal:host-gateway".into()]),
                mounts: Some(vec![datadir_mount, conf_mount]),
                ..Default::default()
            }),
            ..Default::default()
        };

        let ContainerCreateResponse { id: container_id, .. } =
            self.docker.create_container(Some(options), config).await.map_err(|e| {
                diagnosed_error!("Failed to create a container for starting op-geth: {}", e)
            })?;

        self.docker.start_container::<String>(&container_id, None).await.map_err(|e| {
            diagnosed_error!("Failed to start the container for starting op-geth: {}", e)
        })?;

        self.op_geth_container_id = Some(container_id);

        Ok(())
    }

    async fn start_op_node(&mut self) -> Result<(), Diagnostic> {
        if self.network_id.is_none() {
            return Err(diagnosed_error!("Network not initialized"));
        }
        if self.op_geth_container_id.is_none() {
            return Err(diagnosed_error!("op-geth container not started"));
        };
        let Some(conf_mount) = self.conf_mount.clone() else {
            return Err(diagnosed_error!("conf mount not created"));
        };

        let options = CreateContainerOptions {
            platform: Some("linux/amd64"),
            name: &OP_NODE_CONTAINER_NAME,
            ..Default::default()
        };

        let exposed_ports = RollupDeployer::generate_exposed_ports(&vec![OP_NODE_RPC_PORT]);

        let config = Config {
            image: Some(format!("{}:{}", OP_NODE_IMAGE, DEFAULT_TAG)),
            cmd: Some(vec![
                "usr/local/bin/op-node".into(),
                "--l2".into(),
                RollupDeployer::op_geth_auth_rpc_url(),
                "--rpc.port".into(),
                OP_NODE_RPC_PORT.into(),
                "--l2.jwt-secret".into(),
                "/conf/jwt.txt".into(),
                "--sequencer.enabled".into(),
                "--sequencer.l1-confs".into(),
                "5".into(),
                "--verifier.l1-confs".into(),
                "4".into(),
                "--rollup.config".into(),
                "/conf/rollup.json".into(),
                "--rpc.addr".into(),
                "0.0.0.0".into(),
                "--p2p.disable".into(),
                "--rpc.enable-admin".into(),
                "--p2p.sequencer.key".into(),
                format!("{}", self.sequencer_secret_key),
                "--l1".into(),
                self.get_l1_rpc_api_url(),
                "--l1.rpckind".into(),
                format!("{}", self.l1_rpc_kind),
                "--l1.trustrpc".into(),
            ]),
            exposed_ports: Some(exposed_ports),
            host_config: Some(HostConfig {
                auto_remove: Some(false),
                network_mode: Some(self.network_name.clone()),
                extra_hosts: Some(vec!["host.docker.internal:host-gateway".into()]),
                mounts: Some(vec![conf_mount]),
                ..Default::default()
            }),
            ..Default::default()
        };

        let ContainerCreateResponse { id: container_id, .. } =
            self.docker.create_container(Some(options), config).await.map_err(|e| {
                diagnosed_error!("Failed to create a container for starting op-node: {}", e)
            })?;

        self.docker.start_container::<String>(&container_id, None).await.map_err(|e| {
            diagnosed_error!("Failed to start the container for starting op-node: {}", e)
        })?;

        self.op_node_container_id = Some(container_id);

        Ok(())
    }

    async fn start_batcher(&mut self) -> Result<(), Diagnostic> {
        if self.network_id.is_none() {
            return Err(diagnosed_error!("Network not initialized"));
        }
        if self.op_geth_container_id.is_none() {
            return Err(diagnosed_error!("op-geth container not started"));
        };
        if self.op_node_container_id.is_none() {
            return Err(diagnosed_error!("op-node container not started"));
        };

        let options = CreateContainerOptions {
            platform: Some("linux/amd64"),
            name: &OP_BATCHER_CONTAINER_NAME,
            ..Default::default()
        };

        let exposed_ports = RollupDeployer::generate_exposed_ports(&vec![OP_BATCHER_RPC_PORT]);

        let config = Config {
            image: Some(format!("{}:{}", OP_BATCHER_IMAGE, DEFAULT_TAG)),
            cmd: Some(vec![
                "/usr/local/bin/op-batcher".into(),
                "--l2-eth-rpc".into(),
                RollupDeployer::op_geth_rpc_url(),
                "--rollup-rpc".into(),
                RollupDeployer::op_node_rpc_url(),
                "--poll-interval".into(),
                "1s".into(),
                "--sub-safety-margin".into(),
                "6".into(),
                "--num-confirmations".into(),
                "1".into(),
                "--safe-abort-nonce-too-low-count".into(),
                "3".into(),
                "--resubmission-timeout".into(),
                "30s".into(),
                "--rpc.addr".into(),
                "0.0.0.0".into(),
                "--rpc.port".into(),
                OP_BATCHER_RPC_PORT.into(),
                "--rpc.enable-admin".into(),
                "--max-channel-duration".into(),
                "25".into(),
                "--l1-eth-rpc".into(),
                self.get_l1_rpc_api_url(),
                "--private-key".into(),
                self.batcher_secret_key.clone(),
            ]),
            exposed_ports: Some(exposed_ports),
            host_config: Some(HostConfig {
                auto_remove: Some(false),
                network_mode: Some(self.network_name.clone()),
                extra_hosts: Some(vec!["host.docker.internal:host-gateway".into()]),
                mounts: Some(vec![]),
                ..Default::default()
            }),
            ..Default::default()
        };

        let ContainerCreateResponse { id: container_id, .. } =
            self.docker.create_container(Some(options), config).await.map_err(|e| {
                diagnosed_error!("Failed to create a container for starting op-batcher: {}", e)
            })?;

        self.docker.start_container::<String>(&container_id, None).await.map_err(|e| {
            diagnosed_error!("Failed to start the container for starting op-batcher: {}", e)
        })?;

        self.op_batcher_container_id = Some(container_id);

        Ok(())
    }

    async fn start_proposer(&mut self) -> Result<(), Diagnostic> {
        if self.network_id.is_none() {
            return Err(diagnosed_error!("Network not initialized"));
        }
        if self.op_geth_container_id.is_none() {
            return Err(diagnosed_error!("op-geth container not started"));
        };
        if self.op_node_container_id.is_none() {
            return Err(diagnosed_error!("op-node container not started"));
        };
        if self.op_batcher_container_id.is_none() {
            return Err(diagnosed_error!("op-batcher container not started"));
        };

        let options = CreateContainerOptions {
            platform: Some("linux/amd64"),
            name: &OP_PROPOSER_CONTAINER_NAME,
            ..Default::default()
        };

        let l2_output_oracle_proxy = self
            .l1_deployment_addresses
            .get("L2OutputOracleProxy")
            .and_then(|v| v.as_string())
            .ok_or(diagnosed_error!("L2OutputOracleProxy not found in L1 Deployments"))?;

        let exposed_ports = RollupDeployer::generate_exposed_ports(&vec![OP_PROPOSER_RPC_PORT]);

        let config = Config {
            image: Some(format!("{}:{}", OP_PROPOSER_IMAGE, DEFAULT_TAG)),
            cmd: Some(vec![
                "/usr/local/bin/op-proposer".into(),
                "--poll-interval".into(),
                "12s".into(),
                "--rpc.port".into(),
                OP_PROPOSER_RPC_PORT.into(),
                "--rollup-rpc".into(),
                RollupDeployer::op_node_rpc_url(),
                "--l2oo-address".into(),
                l2_output_oracle_proxy.to_string(),
                "--private-key".into(),
                self.proposer_secret_key.clone(),
                "--l1-eth-rpc".into(),
                format!("{}", self.get_l1_rpc_api_url()),
            ]),
            exposed_ports: Some(exposed_ports),
            host_config: Some(HostConfig {
                auto_remove: Some(false),
                network_mode: Some(self.network_name.clone()),
                extra_hosts: Some(vec!["host.docker.internal:host-gateway".into()]),
                mounts: Some(vec![]),
                ..Default::default()
            }),
            ..Default::default()
        };

        let ContainerCreateResponse { id: container_id, .. } =
            self.docker.create_container(Some(options), config).await.map_err(|e| {
                diagnosed_error!("Failed to create a container for starting op-proposer: {}", e)
            })?;

        self.docker.start_container::<String>(&container_id, None).await.map_err(|e| {
            diagnosed_error!("Failed to start the container for starting op-proposer: {}", e)
        })?;

        self.op_proposer_container_id = Some(container_id);
        Ok(())
    }

    fn create_tmp_file(
        &self,
        file_name: &str,
        internal_file_loc: &str,
        data: &str,
    ) -> Result<Mount, Diagnostic> {
        let tmp_file_path = std::env::temp_dir().join(file_name);
        std::fs::write(&tmp_file_path, data)
            .map_err(|e| diagnosed_error!("failed to write to tmp file: {}", e))?;

        let mount = Mount {
            target: Some(format!("{}/{}", internal_file_loc, file_name)),
            source: Some(tmp_file_path.to_str().unwrap().to_string()),
            typ: Some(MountTypeEnum::BIND),
            ..Default::default()
        };
        Ok(mount)
    }

    async fn wait_for_container_exit(
        &self,
        container_id: &str,
    ) -> Result<(), bollard::errors::Error> {
        loop {
            let container_info = self.docker.inspect_container(container_id, None).await?;

            if let Some(state) = container_info.state {
                if Some(ContainerStateStatusEnum::EXITED) == state.status {
                    return Ok(());
                }
            }

            // Wait a bit before checking again
            sleep(Duration::from_secs(2));
        }
    }

    async fn create_volume(&self, volume_name: &str) -> Result<Mount, Diagnostic> {
        let volume_opts =
            CreateVolumeOptions::<String> { name: volume_name.into(), ..Default::default() };

        self.docker
            .create_volume(volume_opts)
            .await
            .map_err(|e| diagnosed_error!("failed to create a docker volume: {}", e))?;

        let mount = Mount {
            target: Some(format!("/{volume_name}")),
            source: Some(volume_name.into()),
            typ: Some(MountTypeEnum::VOLUME),
            read_only: Some(false),
            ..Default::default()
        };
        Ok(mount)
    }

    fn get_l1_rpc_api_url(&self) -> String {
        let l1_rpc_api_url = match (url_is_local(&self.l1_rpc_api_url), self.l1_rpc_api_url.port())
        {
            (true, Some(port)) => {
                format!("http://host.docker.internal:{}", port)
            }
            _ => self.l1_rpc_api_url.to_string(),
        };
        l1_rpc_api_url
    }

    fn op_geth_auth_rpc_url() -> String {
        format!("http://{}:{}", OP_GETH_CONTAINER_NAME, OP_GETH_AUTH_RPC_PORT)
    }
    fn op_geth_rpc_url() -> String {
        format!("http://{}:{}", OP_GETH_CONTAINER_NAME, OP_GETH_RPC_PORT)
    }
    fn op_node_rpc_url() -> String {
        format!("http://{}:{}", OP_NODE_CONTAINER_NAME, OP_NODE_RPC_PORT)
    }

    fn generate_port_bindings(ports: &Vec<&str>) -> HashMap<String, Option<Vec<PortBinding>>> {
        ports
            .iter()
            .map(|port| {
                (
                    format!("{}/tcp", port),
                    Some(vec![PortBinding { host_ip: None, host_port: Some(port.to_string()) }]),
                )
            })
            .collect::<HashMap<String, Option<Vec<PortBinding>>>>()
    }
    fn generate_exposed_ports(ports: &Vec<&str>) -> HashMap<String, HashMap<(), ()>> {
        ports
            .iter()
            .map(|port| (format!("{}/tcp", port), HashMap::new()))
            .collect::<HashMap<String, HashMap<(), ()>>>()
    }
}

#[derive(Debug, Clone)]
pub struct RollupPackager {
    working_dir: String,
    docker: Docker,
    op_geth_container_id: String,
    op_node_container_id: String,
    op_batcher_container_id: String,
    op_proposer_container_id: String,
}

impl RollupPackager {
    pub fn new(
        working_dir: &str,
        rollup_container_ids: IndexMap<String, Value>,
    ) -> Result<Self, Diagnostic> {
        let op_geth_container_id = rollup_container_ids
            .get("op_geth_container_id")
            .and_then(|v| v.as_string())
            .ok_or(diagnosed_error!("op_geth_container_id not found in container ids"))?;
        let op_node_container_id = rollup_container_ids
            .get("op_node_container_id")
            .and_then(|v| v.as_string())
            .ok_or(diagnosed_error!("op_node_container_id not found in container ids"))?;
        let op_batcher_container_id = rollup_container_ids
            .get("op_batcher_container_id")
            .and_then(|v| v.as_string())
            .ok_or(diagnosed_error!("op_batcher_container_id not found in container ids"))?;
        let op_proposer_container_id = rollup_container_ids
            .get("op_proposer_container_id")
            .and_then(|v| v.as_string())
            .ok_or(diagnosed_error!("op_proposer_container_id not found in container ids"))?;

        Ok(Self {
            working_dir: working_dir.to_string(),
            op_geth_container_id: op_geth_container_id.to_string(),
            op_node_container_id: op_node_container_id.to_string(),
            op_batcher_container_id: op_batcher_container_id.to_string(),
            op_proposer_container_id: op_proposer_container_id.to_string(),
            docker: Docker::connect_with_socket_defaults()
                .map_err(|e| diagnosed_error!("Failed to connect to Docker: {}", e))?,
        })
    }

    pub async fn package_rollup(&self) -> Result<(), Diagnostic> {
        self.pause_containers().await?;
        self.commit_containers().await?;
        self.tar_volumes().await?;
        let docker_compose_builder = DockerComposeBuilder::new(
            &self.working_dir,
            "ovm_network",
            "latest",
            "latest",
            "latest",
            "latest",
        );
        docker_compose_builder.build()?;
        self.remove_containers().await?;
        Ok(())
    }

    async fn pause_containers(&self) -> Result<(), Diagnostic> {
        let container_ids = vec![
            &self.op_geth_container_id,
            &self.op_node_container_id,
            &self.op_batcher_container_id,
            &self.op_proposer_container_id,
        ];

        for container_id in container_ids {
            match self.docker.pause_container(container_id).await {
                Ok(_) => {}
                Err(e) => {
                    if e.to_string().eq(&format!(
                        "Docker responded with status code 409: Container {} is already paused",
                        container_id
                    )) {
                        continue;
                    } else {
                        return Err(diagnosed_error!(
                            "Failed to pause container {}: {}",
                            container_id,
                            e
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    async fn remove_containers(&self) -> Result<(), Diagnostic> {
        let container_ids = vec![
            &self.op_geth_container_id,
            &self.op_node_container_id,
            &self.op_batcher_container_id,
            &self.op_proposer_container_id,
        ];

        for container_id in container_ids {
            self.docker.stop_container(&container_id, None).await.map_err(|e| {
                diagnosed_error!("Failed to stop container {}: {}", container_id, e)
            })?;

            self.docker
                .remove_container(
                    &container_id,
                    Some(RemoveContainerOptions { v: false, force: false, ..Default::default() }),
                )
                .await
                .map_err(|e| {
                    diagnosed_error!("Failed to remove container {}: {}", container_id, e)
                })?;
        }

        Ok(())
    }

    async fn tar_volumes(&self) -> Result<(), Diagnostic> {
        let config = Config {
            image: Some("alpine:latest"),
            host_config: Some(HostConfig {
                mounts: Some(vec![
                    Mount {
                        target: Some(format!("/datadir")),
                        source: Some("datadir".into()),
                        typ: Some(MountTypeEnum::VOLUME),
                        read_only: Some(false),
                        ..Default::default()
                    },
                    Mount {
                        target: Some(format!("/conf")),
                        source: Some("conf".into()),
                        typ: Some(MountTypeEnum::VOLUME),
                        read_only: Some(false),
                        ..Default::default()
                    },
                    Mount {
                        target: Some(format!("/host")),
                        source: Some(self.working_dir.clone()),
                        typ: Some(MountTypeEnum::BIND),
                        ..Default::default()
                    },
                ]),
                ..Default::default()
            }),
            cmd: Some(vec![
                "sh",
                "-c",
                "tar czf /host/datadir.tar.gz /datadir && \
                tar czf /host/conf.tar.gz /conf",
            ]),
            ..Default::default()
        };
        let ContainerCreateResponse { id: container_id, .. } =
            self.docker.create_container::<&str, &str>(None, config).await.map_err(|e| {
                diagnosed_error!("Failed to create container to archive volumes: {e}")
            })?;

        self.docker
            .start_container::<String>(&container_id, None)
            .await
            .map_err(|e| diagnosed_error!("Failed to start container to archive volumes: {e}"))?;

        self.wait_for_container_exit(&container_id).await.map_err(|e| {
            diagnosed_error!(
                "Failed to wait for the container for archiving volumes to exit: {}",
                e
            )
        })?;

        self.docker
            .remove_container(
                &container_id,
                Some(RemoveContainerOptions { v: false, force: false, ..Default::default() }),
            )
            .await
            .map_err(|e| {
                diagnosed_error!("Failed to remove the container for archiving volumes: {}", e)
            })?;

        Ok(())
    }

    pub async fn commit_containers(&self) -> Result<(), Diagnostic> {
        self.commit_container(&self.op_proposer_container_id, "txtx-op-proposer", "latest").await?;
        self.commit_container(&self.op_batcher_container_id, "txtx-op-batcher", "latest").await?;
        self.commit_container(&self.op_node_container_id, "txtx-op-node", "latest").await?;
        self.commit_container(&self.op_geth_container_id, "txtx-op-geth", "latest").await?;

        Ok(())
    }

    async fn wait_for_container_exit(
        &self,
        container_id: &str,
    ) -> Result<(), bollard::errors::Error> {
        loop {
            let container_info = self.docker.inspect_container(container_id, None).await?;

            if let Some(state) = container_info.state {
                if Some(ContainerStateStatusEnum::EXITED) == state.status {
                    return Ok(());
                }
            }

            // Wait a bit before checking again
            sleep(Duration::from_secs(2));
        }
    }

    async fn commit_container(
        &self,
        container_id: &str,
        name: &str,
        tag: &str,
    ) -> Result<(), Diagnostic> {
        let opts = CommitContainerOptions {
            container: container_id.to_string(),
            pause: true,
            repo: name.to_string(),
            tag: tag.to_string(),
            ..Default::default()
        };
        let config = Config::<String> { ..Default::default() };
        self.docker.commit_container(opts, config).await.map_err(|e| {
            diagnosed_error!("Failed to commit container {}({}): {}", name, container_id, e)
        })?;
        Ok(())
    }

    // fn archive_working_dir(&self) -> Result<(), Diagnostic> {
    //     let tar_file_path = format!("{}/rollup-state.tar", self.working_dir);
    //     let tar_file = File::create(&tar_file_path)
    //         .map_err(|e| diagnosed_error!("Invalid working dir: {}", e))?;
    //     let mut tar_builder = tar::Builder::new(tar_file);
    //     tar_builder.append_dir_all(".", &self.working_dir).map_err(|e| {
    //         diagnosed_error!(
    //             "Failed to package working directory for to archive rollup state: {}",
    //             e
    //         )
    //     })?;
    //     tar_builder
    //         .finish()
    //         .map_err(|e| diagnosed_error!("Failed to archive rollup state: {e}"))?;
    //     Ok(())
    // }
}

pub struct DockerComposeBuilder {
    working_dir: String,
    network_name: String,
    op_geth_tag: String,
    op_node_tag: String,
    op_batcher_tag: String,
    op_proposer_tag: String,
}
impl DockerComposeBuilder {
    pub fn new(
        working_dir: &str,
        network_name: &str,
        op_geth_tag: &str,
        op_node_tag: &str,
        op_batcher_tag: &str,
        op_proposer_tag: &str,
    ) -> Self {
        Self {
            working_dir: working_dir.to_string(),
            network_name: network_name.to_string(),
            op_geth_tag: op_geth_tag.to_string(),
            op_node_tag: op_node_tag.to_string(),
            op_batcher_tag: op_batcher_tag.to_string(),
            op_proposer_tag: op_proposer_tag.to_string(),
        }
    }

    pub fn build(&self) -> Result<(), Diagnostic> {
        let template = mustache::compile_str(include_str!("./templates/docker-compose.yml.mst"))
            .expect("Failed to compile template");

        let builder = mustache::MapBuilder::new()
            .insert("double_open", &"{{")
            .expect("failed to encode open braces")
            .insert("double_close", &"}}")
            .expect("failed to encode close braces")
            .insert_str("working_dir", &self.working_dir)
            .insert_str("network_name", &self.network_name)
            .insert_str("op_geth_tag", &self.op_geth_tag)
            .insert_str("op_node_tag", &self.op_node_tag)
            .insert_str("op_batcher_tag", &self.op_batcher_tag)
            .insert_str("op_proposer_tag", &self.op_proposer_tag)
            .build();

        let mut output_file = File::create(format!("{}/docker-compose.yml", self.working_dir))
            .map_err(|e| diagnosed_error!("Failed to create docker-compose.yml file: {}", e))?;

        template
            .render_data(&mut output_file, &builder)
            .map_err(|e| diagnosed_error!("Failed to render docker-compose.yml template: {}", e))?;

        Ok(())
    }
}

fn url_is_local(url: &Url) -> bool {
    url.host_str() == Some("localhost") || url.host_str() == Some("127.0.0.1")
}
