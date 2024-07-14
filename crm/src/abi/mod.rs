use crate::{
    pb::{
        RecallRequest, RecallResponse, RemindRequest, RemindResponse, WelcomeRequest,
        WelcomeResponse,
    },
    CrmService,
};
use chrono::{Duration, Utc};
use crm_metadata::pb::{Content, MaterializeRequest};
use crm_send::pb::SendRequest;
use futures::StreamExt;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Response, Status};
use tracing::warn;
use user_stat::pb::QueryRequest;

const CHANNEL_SIZE: usize = 1024;

impl CrmService {
    pub async fn welcome(
        &self,
        request: WelcomeRequest,
    ) -> Result<Response<WelcomeResponse>, Status> {
        let request_id = request.id;

        let d1 = Utc::now() - Duration::days(request.interval as i64);
        let d2 = Utc::now() + Duration::days(1);
        // 构造查询请求
        let query = QueryRequest::new_with_dt("created_at", d1, d2);
        // 查询用户统计信息
        let mut res_user_stats = self.user_stats.clone().query(query).await?.into_inner();

        // 获取内容元数据
        let contents = self
            .metadata
            .clone()
            .materialize(MaterializeRequest::new_with_ids(&request.content_ids))
            .await?
            .into_inner();

        // 过滤内容元数据
        let contents: Vec<Content> = contents
            .filter_map(|v| async move { v.ok() })
            .collect()
            .await;

        // 将内容元数据转换为Arc<Vec<Content>>
        let contents = Arc::new(contents);
        // 创建发送者和接收者channel
        let (tx, rx) = mpsc::channel(CHANNEL_SIZE);

        // 获取发送者邮箱
        let sender = self.config.server.sender_email.clone();
        // 在单独的task中处理用户统计信息
        tokio::spawn(async move {
            while let Some(Ok(user)) = res_user_stats.next().await {
                println!("User: {:?}", user);
                let contents = contents.clone();
                let sender = sender.clone();
                let tx = tx.clone();

                // 构造发送请求
                let req = SendRequest::new("Welcome".to_string(), sender, &[user.email], &contents);
                // 发送请求
                if let Err(e) = tx.send(req).await {
                    warn!("Failed to send message: {:?}", e);
                }
            }
        });
        // 将发送请求转换为ReceiverStream
        let reqs = ReceiverStream::new(rx);

        self.notification.clone().send(reqs).await?;

        Ok(Response::new(WelcomeResponse { id: request_id }))
    }

    pub async fn recall(&self, request: RecallRequest) -> Result<Response<RecallResponse>, Status> {
        // 获取请求id
        let request_id = request.id;
        let d1 = Utc::now() - Duration::days(request.last_visit_interval as i64);
        let d2 = Utc::now() + Duration::days(1);

        let query = QueryRequest::new_with_dt("last_visited_at", d1, d2);

        let mut res_user_stats = self.user_stats.clone().query(query).await?.into_inner();

        let contents = self
            .metadata
            .clone()
            .materialize(MaterializeRequest::new_with_ids(&request.content_ids))
            .await?
            .into_inner();

        let contents: Vec<Content> = contents
            .filter_map(|v| async move { v.ok() })
            .collect()
            .await;

        let contents = Arc::new(contents);
        let (tx, rx) = mpsc::channel(CHANNEL_SIZE);

        let sender = self.config.server.sender_email.clone();

        tokio::spawn(async move {
            while let Some(Ok(user)) = res_user_stats.next().await {
                let contents = contents.clone();
                let sender = sender.clone();
                let tx = tx.clone();

                let req = SendRequest::new("Recall".to_string(), sender, &[user.email], &contents);
                if let Err(e) = tx.send(req).await {
                    warn!("Failed to send message: {:?}", e);
                }
            }
        });

        let reqs = ReceiverStream::new(rx);

        self.notification.clone().send(reqs).await?;
        Ok(Response::new(RecallResponse { id: request_id }))
    }

    pub async fn remind(&self, request: RemindRequest) -> Result<Response<RemindResponse>, Status> {
        let request_id = request.id;
        let d1 = Utc::now() - Duration::days(request.last_visit_interval as i64);
        let d2 = Utc::now() + Duration::days(1);

        let query = QueryRequest::new_with_dt("last_watched_at", d1, d2);

        // 查询用户统计信息
        let mut res_user_stats = self.user_stats.clone().query(query).await?.into_inner();

        let (tx, rx) = mpsc::channel(CHANNEL_SIZE);

        let sender = self.config.server.sender_email.clone();

        tokio::spawn(async move {
            while let Some(Ok(user)) = res_user_stats.next().await {
                let sender = sender.clone();
                let tx = tx.clone();

                println!("Remind: {:?}", user.name);
                let req = SendRequest::new_remind(
                    "Recall".to_string(),
                    sender,
                    &[user.email],
                    user.viewed_but_not_started,
                    user.started_but_not_finished,
                );

                if let Err(e) = tx.send(req).await {
                    warn!("Failed to send message: {:?}", e);
                }
            }
        });

        let reqs = ReceiverStream::new(rx);

        self.notification.clone().send(reqs).await?;
        Ok(Response::new(RemindResponse { id: request_id }))
    }
}
