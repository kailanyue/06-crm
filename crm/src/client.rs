use anyhow::Result;
use crm::pb::{crm_client::CrmClient, WelcomeRequestBuilder};
use tracing::info;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = CrmClient::connect("http://[::1]:50000").await?;

    let req = WelcomeRequestBuilder::default()
        .id(Uuid::new_v4().to_string())
        .interval(93u32)
        .content_ids([1u32, 2, 3])
        .build()?;

    let response = client.welcome(req).await?;

    info!("Response: {:?}", response);

    Ok(())
}
