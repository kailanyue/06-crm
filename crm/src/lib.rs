mod abi;
mod config;

pub mod pb;

pub use config::AppConfig;
use crm_metadata::pb::metadata_client::MetadataClient;
use crm_send::pb::notification_client::NotificationClient;
use pb::{
    crm_server::{Crm, CrmServer},
    RecallRequest, RecallResponse, RemindRequest, RemindResponse, WelcomeRequest, WelcomeResponse,
};
use tonic::{async_trait, transport::Channel, Request, Response, Status};
use user_stat::pb::user_stats_client::UserStatsClient;

pub struct CrmService {
    config: AppConfig,
    user_stats: UserStatsClient<Channel>,
    notification: NotificationClient<Channel>,
    metadata: MetadataClient<Channel>,
}

#[async_trait]
impl Crm for CrmService {
    async fn welcome(
        &self,
        request: Request<WelcomeRequest>,
    ) -> std::result::Result<Response<WelcomeResponse>, Status> {
        self.welcome(request.into_inner()).await
    }

    /// last watched in X days, given them something to watch
    async fn recall(
        &self,
        request: Request<RecallRequest>,
    ) -> std::result::Result<Response<RecallResponse>, Status> {
        self.recall(request.into_inner()).await
    }

    /// last watched in X days, and user still have unfinished contents
    async fn remind(
        &self,
        request: Request<RemindRequest>,
    ) -> std::result::Result<Response<RemindResponse>, Status> {
        self.remind(request.into_inner()).await
    }
}

impl CrmService {
    pub async fn try_new(config: AppConfig) -> Result<Self, tonic::transport::Error> {
        let user_stats = UserStatsClient::connect(config.server.user_stats.clone()).await?;
        let notification = NotificationClient::connect(config.server.notification.clone()).await?;
        let metadata = MetadataClient::connect(config.server.metadata.clone()).await?;
        Ok(Self {
            config,
            user_stats,
            notification,
            metadata,
        })
    }

    pub fn into_server(self) -> CrmServer<Self> {
        CrmServer::new(self)
    }
}
