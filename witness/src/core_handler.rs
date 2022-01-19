use config::Committee;
use crypto::KeyPair;
use log::{debug, warn};
use messages::ensure;
use messages::error::{WitnessError, WitnessResult};
use messages::publish::{
    PublishCertificate, PublishMessage, PublishNotification, PublishVote, SequenceNumber,
};
use std::convert::TryInto;
use storage::Storage;
use tokio::sync::mpsc::Receiver;

/// Storage address of the sequence number.
pub const STORE_SEQ_ADDR: [u8; 32] = [0; 32];
/// Storage address of the witness' lock.
pub const STORE_LOCK_ADDR: [u8; 32] = [1; 32];

/// Core logic handing publish notifications and certificates.
pub struct PublishHandler {
    /// The keypair of this authority.
    keypair: KeyPair,
    /// The committee information.
    committee: Committee,
    /// The persistent storage.
    storage: Storage,
    /// Receive publish notifications from the IdP.
    rx_notification: Receiver<PublishNotification>,
    /// Receive publish certificates from the IdP.
    rx_certificate: Receiver<PublishCertificate>,
    /// The current sequence number.
    sequence_number: SequenceNumber,
    /// The notification on which this witness is locked.
    lock: Option<PublishVote>,
}

impl PublishHandler {
    /// Spawn a new publish handler task.
    pub fn spawn(
        keypair: KeyPair,
        committee: Committee,
        storage: Storage,
        rx_notification: Receiver<PublishNotification>,
        rx_certificate: Receiver<PublishCertificate>,
    ) {
        tokio::spawn(async move {
            // Read the sequence number and lock from storage.
            let sequence_number = storage
                .read(&STORE_SEQ_ADDR)
                .expect("Failed to load sequence number from storage")
                .map(|bytes| {
                    let x = bytes.try_into().expect("Sequence number should be 8 bytes");
                    SequenceNumber::from_le_bytes(x)
                })
                .unwrap_or_default();
            let lock = storage
                .read(&STORE_LOCK_ADDR)
                .expect("Failed to load lock from storage")
                .map(|bytes| bincode::deserialize(&bytes).expect("Failed to deserialize vote"));

            // Run an instance of the handler.
            Self {
                keypair,
                committee,
                storage,
                rx_notification,
                rx_certificate,
                sequence_number,
                lock,
            }
            .run()
            .await
        });
    }

    /// Try to vote for a publish notification.
    fn make_vote(&self, notification: &PublishNotification) -> WitnessResult<PublishVote> {
        // Verify the notification.
        notification.verify(&self.committee)?;

        // Check the sequence number.
        ensure!(
            self.sequence_number == notification.sequence_number(),
            WitnessError::UnexpectedSequenceNumber {
                expected: self.sequence_number,
                got: notification.sequence_number()
            }
        );

        // Ensure there are no locks.
        match self.lock.as_ref() {
            Some(vote) => {
                ensure!(
                    vote.root() == notification.root(),
                    WitnessError::ConflictingNotification(
                        vote.root().clone(),
                        notification.root().clone()
                    )
                );
                Ok(vote.clone())
            }
            None => Ok(PublishVote::new(notification, &self.keypair)),
        }
    }

    /// Process a publish certificate.
    fn process_certificate(&self, certificate: &PublishCertificate) -> WitnessResult<()> {
        // Verify the certificate's validity.
        certificate.verify(&self.committee)?;

        // Ensure the witness is not missing previous certificates.
        ensure!(
            self.sequence_number >= certificate.sequence_number(),
            WitnessError::MissingEarlierCertificates(self.sequence_number)
        );
        Ok(())
    }

    /// Main loop listening to verified IdP's notification messages.
    async fn run(&mut self) {
        loop {
            tokio::select! {
                // Receive publish notifications.
                Some(notification) = self.rx_notification.recv() => {
                    debug!("Received {:?}", notification);
                    match self.make_vote(&notification) {
                        Err(e) => {
                            warn!("{}", e);

                            // Reply with an error message.
                            unimplemented!();
                        },
                        Ok(vote) => {
                            debug!("Create {:?}", vote);
                            let serialized_vote = bincode::serialize(&vote)
                                .expect("Failed to serialize vote");

                            // Register the lock.
                            self.lock = Some(vote);
                            self.storage.write(&STORE_LOCK_ADDR, &serialized_vote)
                                .expect("Failed to persist lock");

                            // Reply with a vote.
                            unimplemented!();
                        }
                    }
                },

                // Receive publish certificates.
                Some(certificate) = self.rx_certificate.recv() => {
                    debug!("Received {:?}", certificate);
                    match self.process_certificate(&certificate) {
                        Err(e) => {
                            warn!("{}", e);

                            // Reply with an error message.
                            unimplemented!();
                        },
                        Ok(()) => {
                            if self.sequence_number == certificate.sequence_number() {
                                debug!("Processing {:?}", certificate);

                                // Update the witness state.
                                self.sequence_number += 1;
                                self.storage.write(&STORE_SEQ_ADDR, &self.sequence_number.to_le_bytes())
                                    .expect("Failed to persist sequence number");

                                self.lock = None;
                                self.storage.write(&STORE_LOCK_ADDR, &Vec::default())
                                    .expect("Failed to persist lock");

                            } else {
                                debug!("Already processed {:?}", certificate);
                            }

                            // Reply with an acknowledgement.
                            unimplemented!();
                        }
                    }
                }
            }
        }
    }
}
