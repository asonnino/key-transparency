use crate::{Batch, SerializedPublishNotification, STORE_LAST_NOTIFICATION_ADDR};
use akd::directory::Directory;
use akd::storage::memory::AsyncInMemoryDatabase;
use crypto::KeyPair;
use messages::publish::{Proof, PublishNotification};
use messages::{Blake3, IdPToWitnessMessage, Root, SequenceNumber};
use storage::Storage;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::task::JoinHandle;

/// Create publish notifications from client requests.
pub struct Prover {
    /// The private key material of the IdP.
    keypair: KeyPair,
    /// Receive batches of clients' requests.
    rx_batch: Receiver<Batch>,
    /// Outputs handles waiting to receive witnesses' votes.
    tx_notification: Sender<SerializedPublishNotification>,
    /// The last notification created by the IdP.
    last_notification: Option<PublishNotification>,
    /// The `akd` key directory.
    akd: Directory<AsyncInMemoryDatabase>,
}

impl Prover {
    /// Spawn a new `Prover`.
    pub fn spawn(
        keypair: KeyPair,
        storage: Storage,
        rx_batch: Receiver<Batch>,
        tx_notification: Sender<SerializedPublishNotification>,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            // Try to load the last notification from storage.
            let last_notification = match storage
                .read(&STORE_LAST_NOTIFICATION_ADDR)
                .expect("Failed to load last notification from storage")
            {
                Some(serialized) => {
                    // Try to re-broadcast it. This is useful in case the IdP crashes after updating its
                    // last notification but before successfully broadcasting it. Otherwise it will have
                    // no effect (witnesses are idempotent).
                    tx_notification
                        .send(serialized.clone())
                        .await
                        .expect("Failed to deliver serialized notification");

                    // Deserialize the notification (we will need to extract its sequence number).
                    bincode::deserialize(&serialized).expect("Failed to deserialize notification")
                }
                None => None,
            };

            // Make or load the akd.
            let db = AsyncInMemoryDatabase::new();
            let akd = Directory::<_>::new::<Blake3>(&db)
                .await
                .expect("Failed to create akd");

            // Run a new `NotificationMaker`.
            Self {
                keypair,
                rx_batch,
                tx_notification,
                last_notification,
                akd,
            }
            .run()
            .await;
        })
    }

    /// Get the latest sequence number of the IdP.
    fn sequence_number(&self) -> SequenceNumber {
        match &self.last_notification {
            Some(x) => x.sequence_number,
            None => 0,
        }
    }

    /// Compute an audit proof from a batch of requests.
    async fn make_proof(&mut self, batch: Batch) -> (Root, Proof) {
        // Persist the batch.
        self.akd
            .publish::<Blake3>(batch, /* use_transaction */ false)
            .await
            .expect("Failed to persist publish request");

        // Extract the latest root.
        let current_azks = self.akd.retrieve_current_azks().await.unwrap();
        let root = self
            .akd
            .get_root_hash_at_epoch::<Blake3>(&current_azks, 1)
            .await
            .unwrap();

        // Generate the audit proof.
        let current = self.sequence_number();
        let next = current + 1;

        let proof = self
            .akd
            .audit::<Blake3>(current, next)
            .await
            .expect("Failed to create audit proof");

        // Output the latest root hash and the audit proof.
        (root, proof)
    }

    /// Main loop receiving batches of client requests.
    async fn run(&mut self) {
        while let Some(batch) = self.rx_batch.recv().await {
            // Compute the audit proof (CPU-intensive).
            let (root, proof) = self.make_proof(batch).await;

            // Assemble and serialize a new publish notification.
            let next = self.sequence_number() + 1;
            let notification = PublishNotification::new(root, proof, next, &self.keypair);
            let message = IdPToWitnessMessage::PublishNotification(notification.clone());
            let serialized =
                bincode::serialize(&message).expect("Failed to serialize notification");

            // Update the latest notification. The `Broadcaster` persists the last notification.
            self.last_notification = Some(notification);

            // Send the notification to the broadcaster.
            self.tx_notification
                .send(serialized)
                .await
                .expect("Failed to deliver serialized notification");
        }
    }
}
