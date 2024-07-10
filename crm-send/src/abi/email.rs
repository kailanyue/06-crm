use tonic::Status;
use tracing::warn;

use crate::pb::{send_request::Msg, EmailMessage, SendRequest, SendResponse};

use super::{to_ts, Sender};

impl Sender for EmailMessage {
    async fn send(
        self,
        svc: crate::NotificationService,
    ) -> Result<crate::pb::SendResponse, tonic::Status> {
        let message_id = self.message_id.clone();
        svc.sender.send(Msg::Email(self)).await.map_err(|e| {
            warn!("Failed to send message: {:?}", e);
            Status::internal("Failed to send message")
        })?;
        Ok(SendResponse {
            message_id,
            timestamp: Some(to_ts()),
        })
    }
}

impl From<EmailMessage> for Msg {
    fn from(value: EmailMessage) -> Self {
        Msg::Email(value)
    }
}

impl From<EmailMessage> for SendRequest {
    fn from(value: EmailMessage) -> Self {
        let msg: Msg = value.into();
        SendRequest { msg: Some(msg) }
    }
}

#[cfg(feature = "test_utils")]
impl EmailMessage {
    pub fn fake() -> Self {
        use fake::faker::internet::en::SafeEmail;
        use fake::Fake;
        use uuid::Uuid;
        EmailMessage {
            message_id: Uuid::new_v4().to_string(),
            sender: SafeEmail().fake(),
            recipients: vec![SafeEmail().fake()],
            subject: "Hello".to_string(),
            body: "Hello, world!".to_string(),
        }
    }
}
