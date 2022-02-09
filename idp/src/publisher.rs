use crate::aggregator::Aggregator;
use crate::{SerializedPublishNotification, STORE_LAST_NOTIFICATION_ADDR};
use bytes::Bytes;
use config::Committee;
use crypto::PublicKey;
use futures::stream::futures_unordered::FuturesUnordered;
use futures::stream::StreamExt;
use log::{debug, info, warn};
use messages::error::{IdpError, IdpResult, MessageError, WitnessError, WitnessResult};
use messages::publish::PublishVote;
use messages::WitnessToIdPMessage;
use network::reliable_sender::{CancelHandler, ReliableSender};
use std::net::SocketAddr;
use storage::Storage;
use tokio::sync::mpsc::Receiver;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

/// Broadcast publish notifications to the witnesses, gather votes and broadcast certificates.
pub struct Publisher {
    /// The persistent storage.
    storage: Storage,
    /// Receive serialized publish notifications to broadcast.
    rx_notification: Receiver<SerializedPublishNotification>,
    /// A reliable network sender.
    network: ReliableSender,
    names: Vec<PublicKey>,
    /// The network addresses of the witnesses.
    addresses: Vec<SocketAddr>,
    /// A votes aggregator to assemble a quorum of votes into a certificate.
    aggregator: Aggregator,
}

impl Publisher {
    /// Spawn a new broadcaster.
    pub fn spawn(
        committee: Committee,
        storage: Storage,
        rx_notification: Receiver<SerializedPublishNotification>,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            let (names, addresses) = committee.witnesses_addresses().into_iter().unzip();
            Self {
                storage,
                rx_notification,
                network: ReliableSender::new(),
                names,
                addresses,
                aggregator: Aggregator::new(committee),
            }
            .run()
            .await;
        })
    }

    async fn sync(&mut self, serialized: Bytes) -> CancelHandler {
        let (sender, receiver) = oneshot::channel();
        receiver
    }

    /// Handle the witness reply to a IdP publish notification.
    async fn handle_notification_reply(
        &mut self,
        notification: &Bytes,
        reply: &Bytes,
        futures: &mut FuturesUnordered<CancelHandler>,
    ) -> IdpResult<Option<PublishVote>> {
        // Check if we got an expected reply.
        let result = match bincode::deserialize(&reply).map_err(MessageError::from)? {
            WitnessToIdPMessage::PublishVote(result) => result,
            _ => return Err(IdpError::UnexpectedProtocolMessage),
        };

        // Check if the witness replied with a vote or if it is outdated.
        match result {
            Ok(vote) => Ok(Some(vote)),
            Err(e) => {
                if let WitnessError::UnexpectedSequenceNumber { expected, got } = e {
                    if expected < got {
                        debug!("{}", e);
                        let handle = self.sync(notification.clone()).await;
                        futures.push(handle);
                        return Ok(None);
                    }
                }
                Err(e).map_err(IdpError::from)
            }
        }
    }

    /*
    /// Helper function. It waits for a future to complete and then delivers a value.
    async fn waiter(author: PublicKey, wait_for: CancelHandler) -> (PublicKey, Bytes) {
        let reply = wait_for
            .await
            .expect("Failed to receive response from network");
        (author, reply)
    }
    */

    async fn run(&mut self) {
        // Gather certificates handles to receive state ack.
        let mut state_responses = FuturesUnordered::new();

        //
        loop {
            tokio::select! {
                // Receive serialized publish notifications.
                Some(serialized) = self.rx_notification.recv() => {
                    // Persist the last notification to storage.
                    self.storage
                        .write(&STORE_LAST_NOTIFICATION_ADDR, &serialized)
                        .expect("Failed to persist state");

                    // Broadcast the publish notification to the witnesses.
                    let bytes = Bytes::from(serialized);
                    let addresses = self.addresses.clone();
                    let mut wait_for_quorum: FuturesUnordered<_> = self
                        .network
                        .broadcast(addresses, bytes.clone())
                        .await
                        .into_iter()
                        .collect();

                    // Collect the votes and assemble a certificate.
                    while let Some(result) = wait_for_quorum.next().await {
                        let reply = result.expect("Failed to receive response from network");
                        let vote = match self.handle_notification_reply(&bytes, &reply, &mut wait_for_quorum).await {
                            Ok(Some(vote)) => vote,
                            Ok(None) => continue,
                            Err(e) => {
                                warn!("{:?}", e);
                                continue
                            }
                        };

                        if let Some(certificate) = self.aggregator.append(vote) {
                            info!("Processed {:?}", certificate);

                            // Broadcast the certificate to the witnesses.
                            let serialized = bincode::serialize(&certificate)
                                .expect("Failed to serialize certificate");
                            let bytes = Bytes::from(serialized);
                            self.network
                                .broadcast(self.addresses.clone(), bytes)
                                .await
                                .into_iter()
                                .for_each(|handle| state_responses.push(handle));

                            // Clear the aggregator and stop waiting for votes.
                            self.aggregator.clear();
                            break;
                        }
                    }
                },

                // Receive state ack from the witnesses.
                Some(_reply) = state_responses.next() => {
                    // Sync
                },
            }
        }
    }
}
