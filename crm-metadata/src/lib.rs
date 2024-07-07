pub mod pb;

mod abi;
mod config;

use futures::Stream;
use pb::{
    metadata_server::{Metadata, MetadataServer},
    Content, MaterializeRequest,
};
use std::pin::Pin;
use tonic::{async_trait, Request, Response, Status, Streaming};

pub use config::AppConfig;

#[allow(unused)]
pub struct MetadataService {
    config: AppConfig,
}

type ServiceResult<T> = Result<Response<T>, Status>;
type ResponseStream = Pin<Box<dyn Stream<Item = Result<Content, Status>> + Send>>;

#[async_trait]
impl Metadata for MetadataService {
    // 定义MaterializeStream的类型为ResponseStream
    type MaterializeStream = ResponseStream;

    // 实现materialize方法，接收一个Request<Streaming<MaterializeRequest>>类型的参数，返回一个ServiceResult<Self::MaterializeStream>类型的结果
    async fn materialize(
        &self,
        request: Request<Streaming<MaterializeRequest>>,
    ) -> ServiceResult<Self::MaterializeStream> {
        let query = request.into_inner();
        self.materialize(query).await
    }
}

impl MetadataService {
    pub fn new(config: AppConfig) -> Self {
        Self { config }
    }

    pub fn into_server(self) -> MetadataServer<Self> {
        MetadataServer::new(self)
    }
}
