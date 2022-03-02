use crate::STORE_LAST_NOTIFICATION_ADDR;
use akd::directory::Directory;
use akd::primitives::akd_vrf::HardCodedAkdVRF;
use crypto::KeyPair;
use futures::executor::block_on;
use messages::publish::{Proof, PublishNotification};
use messages::update::Batch;
use messages::{Blake3, Root, SequenceNumber};
use storage::Storage;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::task::JoinHandle;

/// Create publish notifications from client requests.
pub struct Prover<AkdStorage> {
    /// The private key material of the IdP.
    keypair: KeyPair,
    /// Receive batches of clients' requests.
    rx_batch: Receiver<Batch>,
    /// Outputs handles waiting to receive witnesses' votes.
    tx_notification: Sender<PublishNotification>,
    /// The sequence number of the last notification created by the IdP.
    sequence_number: SequenceNumber,
    /// The `akd` key directory.
    akd: Directory<AkdStorage, HardCodedAkdVRF>,
}

impl<AkdStorage> Prover<AkdStorage>
where
    AkdStorage: akd::storage::Storage + Sync + Send + 'static,
{
    /// Spawn a new `Prover`.
    pub fn spawn(
        keypair: KeyPair,
        secure_storage: &Storage,
        akd_storage: AkdStorage,
        rx_batch: Receiver<Batch>,
        tx_notification: Sender<PublishNotification>,
    ) -> JoinHandle<()> {
        // Load the last sequence number and perform initialization steps.
        let sequence_number = block_on(Self::initialize(secure_storage, &tx_notification));

        // Run the prover in a new task.
        tokio::spawn(async move {
            // Make or load the akd directory.
            let db = akd_storage;
            let vrf = HardCodedAkdVRF {};
            let akd = Directory::new::<Blake3>(&db, &vrf, false)
                .await
                .expect("Failed to create akd");

            // Run a new `NotificationMaker`.
            Self {
                keypair,
                rx_batch,
                tx_notification,
                sequence_number,
                akd,
            }
            .run()
            .await;
        })
    }

    /// Load the last sequence number from storage and perform initialization steps.
    async fn initialize(
        storage: &Storage,
        tx_notification: &Sender<PublishNotification>,
    ) -> SequenceNumber {
        match storage
            .read(&STORE_LAST_NOTIFICATION_ADDR)
            .expect("Failed to load last notification from storage")
        {
            Some(serialized) => {
                // Deserialize the notification and extract its sequence number.
                let notification: PublishNotification =
                    bincode::deserialize(&serialized).expect("Failed to deserialize notification");
                let sequence_number = notification.sequence_number;

                // Try to re-broadcast it. This is useful in case the IdP crashes after updating its
                // last notification but before successfully broadcasting it. Otherwise it will have
                // no effect (witnesses are idempotent).
                tx_notification
                    .send(notification)
                    .await
                    .expect("Failed to deliver serialized notification");

                sequence_number
            }
            None => SequenceNumber::default(),
        }
    }

    /// Compute an audit proof from a batch of requests.
    async fn make_proof(&mut self, batch: Batch) -> (Root, Proof) {
        let current = self.sequence_number;
        let next = current + 1;

        // Persist the batch.
        self.akd
            .publish::<Blake3>(batch, /* use_transaction */ false)
            .await
            .expect("Failed to persist publish request");

        // Extract the latest root.
        let current_azks = self.akd.retrieve_current_azks().await.unwrap();
        let root = self
            .akd
            .get_root_hash_at_epoch::<Blake3>(&current_azks, next)
            .await
            .unwrap();

        // Generate the audit proof.
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

            // Increment the sequence number.
            self.sequence_number += 1;

            // Make a new publish notification.
            let notification =
                PublishNotification::new(root, proof, self.sequence_number, &self.keypair);

            // Send the notification to the broadcaster.
            self.tx_notification
                .send(notification)
                .await
                .expect("Failed to deliver serialized notification");
        }
    }
}
