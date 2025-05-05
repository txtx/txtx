use serde_json::Value as JsonValue;
use std::{fmt::Debug, future::Future, pin::Pin, sync::Arc};

#[derive(Debug, Clone)]
pub struct CloudServiceContext {
    pub authenticated_cloud_service_router: Option<Arc<dyn AuthenticatedCloudServiceRouter>>,
}

impl CloudServiceContext {
    pub fn new(
        authenticated_cloud_service_router: Option<Arc<dyn AuthenticatedCloudServiceRouter>>,
    ) -> Self {
        Self { authenticated_cloud_service_router }
    }
    pub fn empty() -> Self {
        Self { authenticated_cloud_service_router: None }
    }
}

pub trait AuthenticatedCloudServiceRouter: Send + Sync + Debug {
    fn route<'a>(
        &'a self,
        service: CloudService,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>>;
}

#[derive(Debug, Clone)]
pub enum CloudService {
    Registry,
    Id,
    Svm(SvmService),
    Evm,
}

impl CloudService {
    pub fn svm_subgraph(url: &str, params: JsonValue, do_include_token: bool) -> Self {
        Self::Svm(SvmService::DeploySubgraph(DeploySubgraphCommand {
            url: url.to_string(),
            params,
            do_include_token,
        }))
    }
    pub fn token_required(&self) -> bool {
        match self {
            CloudService::Registry => false,
            CloudService::Id => false,
            CloudService::Svm(SvmService::DeploySubgraph(cmd)) => cmd.do_include_token,
            CloudService::Evm => false,
        }
    }
}

#[derive(Debug, Clone)]
pub enum SvmService {
    DeploySubgraph(DeploySubgraphCommand),
}

#[derive(Debug, Clone)]
pub struct DeploySubgraphCommand {
    pub url: String,
    pub params: JsonValue,
    pub do_include_token: bool,
}
