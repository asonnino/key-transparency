use crate::Replier;
use messages::sync::PublishCertificateRequest;
use messages::WitnessToIdPMessage;
use storage::Storage;
use tokio::sync::mpsc::Receiver;

/// Task dedicated to help other witnesses to sync up by replying to certificate requests.
pub struct SyncHelper {
    /// The persistent storage.
    storage: Storage,
    /// Receive the publish certificates requests.
    rx_certificate_request: Receiver<(PublishCertificateRequest, Replier)>,
}

impl SyncHelper {
    /// Spawn a new sync helper task.
    pub fn spawn(
        storage: Storage,
        rx_certificate_request: Receiver<(PublishCertificateRequest, Replier)>,
    ) {
        tokio::spawn(async move {
            Self {
                storage,
                rx_certificate_request,
            }
            .run()
            .await
        });
    }

    /// Main loop answering certificate requests.
    async fn run(&mut self) {
        while let Some((request, replier)) = self.rx_certificate_request.recv().await {
            // Check whether the requested certificate is in storage.
            let key = request.sequence_number.to_le_bytes();
            if let Some(serialized_certificate) = self
                .storage
                .read(&key)
                .expect("Failed to load certificate from storage")
            {
                // Reply with the certificate (if we have it).
                let reply = WitnessToIdPMessage::PublishCertificateResponse(serialized_certificate);
                replier
                    .send(reply)
                    .expect("Failed to reply to certificate sync request");
            }
        }
    }
}
