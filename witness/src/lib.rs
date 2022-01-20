mod publish_handler;
mod sync_helper;

use crate::publish_handler::PublishHandler;
use crate::sync_helper::SyncHelper;
use async_trait::async_trait;
use bytes::Bytes;
use config::Committee;
use crypto::KeyPair;
use futures::sink::SinkExt;
use log::info;
use messages::publish::{PublishCertificate, PublishNotification};
use messages::sync::PublishCertificateRequest;
use messages::{IdPtoWitnessMessage, WitnessToIdPMessage};
use network::receiver::{MessageHandler, Receiver as NetworkReceiver, Writer};
use std::error::Error;
use storage::Storage;
use tokio::sync::mpsc::{channel, Sender};
use tokio::sync::oneshot;

pub(crate) const DEFAULT_CHANNEL_SIZE: usize = 1_000;

/// One-shot channel to reply to the IdP.
pub(crate) type Replier = oneshot::Sender<WitnessToIdPMessage>;

/// Spawn a new witness.
pub fn spawn_witness(
    // The public and secret keypair of this witness.
    keypair: KeyPair,
    // The committee information.
    committee: Committee,
    // The storage for safety-critical information.
    secure_storage: Storage,
    // The storage for certificates and other self-authenticated information.
    audit_storage: Storage,
) {
    let name = keypair.public();

    let (tx_notification, rx_notification) = channel(DEFAULT_CHANNEL_SIZE);
    let (tx_certificate, rx_certificate) = channel(DEFAULT_CHANNEL_SIZE);
    let (tx_state_query, rx_state_query) = channel(DEFAULT_CHANNEL_SIZE);
    let (tx_certificate_request, rx_certificate_request) = channel(DEFAULT_CHANNEL_SIZE);

    // Spawn the publish handler. This task handles all publish-related messages.
    PublishHandler::spawn(
        keypair,
        committee.clone(),
        secure_storage,
        rx_notification,
        rx_certificate,
        rx_state_query,
    );

    // Spawn the sync helper. This task replies to sync request helping other witness to get up to speed.
    SyncHelper::spawn(audit_storage, rx_certificate_request);

    // Spawn a network receiver.
    let address = committee
        .witness_address(&name)
        .expect("Our public key is not in the committee");
    let handler = WitnessHandler {
        tx_notification,
        tx_certificate,
        tx_state_query,
        tx_certificate_request,
    };
    NetworkReceiver::spawn(address, handler);

    info!("Witness {} successfully booted on {}", name, address.ip());
}

/// Defines how the network receiver handles incoming messages.
#[derive(Clone)]
struct WitnessHandler {
    tx_notification: Sender<(PublishNotification, Replier)>,
    tx_certificate: Sender<(PublishCertificate, Replier)>,
    tx_state_query: Sender<Replier>,
    tx_certificate_request: Sender<(PublishCertificateRequest, Replier)>,
}

#[async_trait]
impl MessageHandler for WitnessHandler {
    async fn dispatch(&self, writer: &mut Writer, serialized: Bytes) -> Result<(), Box<dyn Error>> {
        let (sender, receiver) = oneshot::channel();

        // Deserialize and parse the message.
        match bincode::deserialize(&serialized)? {
            IdPtoWitnessMessage::PublishNotification(notification) => self
                .tx_notification
                .send((notification, sender))
                .await
                .expect("Failed to send publish notification to publish handler"),
            IdPtoWitnessMessage::PublishCertificate(certificate) => self
                .tx_certificate
                .send((certificate, sender))
                .await
                .expect("Failed to send publish certificate to publish handler"),
            IdPtoWitnessMessage::StateQuery => self
                .tx_state_query
                .send(sender)
                .await
                .expect("Failed to send state query to publish handler"),
            IdPtoWitnessMessage::PublishCertificateQuery(request) => self
                .tx_certificate_request
                .send((request, sender))
                .await
                .expect("Failed to certificate query query to sync helper"),
        }

        // Reply to the IdP.
        let reply = receiver.await.expect("Failed to receive message reply");
        let bytes = bincode::serialize(&reply).expect("Failed to serialize reply");
        writer.send(Bytes::from(bytes)).await?;
        Ok(())
    }
}
