use anyhow::Result;
use crm::pb::{crm_client::CrmClient, RemindRequestBuilder};
use tonic::{
    transport::{Certificate, Channel, ClientTlsConfig},
    Request,
};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pem = include_str!("../../fixtures/rootCA.pem");

    let tls = ClientTlsConfig::new()
        .ca_certificate(Certificate::from_pem(pem))
        .domain_name("localhost");

    let channel = Channel::from_static("https://[::1]:50000")
        .tls_config(tls)?
        .connect()
        .await?;

    let mut client = CrmClient::new(channel);

    // wellcome request
    // let welcome_request = WelcomeRequestBuilder::default()
    //     .id(Uuid::new_v4().to_string())
    //     .interval(110u32)
    //     .content_ids([1u32, 2, 3])
    //     .build()?;

    // let response_welcome = client
    //     .welcome(Request::new(welcome_request))
    //     .await?
    //     .into_inner();
    // println!("Wellcome Response: {:?}", response_welcome);

    // // recall request
    // let recall_request = RecallRequestBuilder::default()
    //     .id(Uuid::new_v4().to_string())
    //     .last_visit_interval(30u32)
    //     .content_ids([1u32, 2, 3])
    //     .build()?;

    // let response_recall = client
    //     .recall(Request::new(recall_request))
    //     .await?
    //     .into_inner();
    // println!("Recall Response: {:?}", response_recall);

    // remind request
    let remind_request = RemindRequestBuilder::default()
        .id(Uuid::new_v4().to_string())
        .last_visit_interval(20u32)
        .build()?;

    let response_remind = client
        .remind(Request::new(remind_request))
        .await?
        .into_inner();
    println!("Remind Response: {:?}", response_remind);
    Ok(())
}
