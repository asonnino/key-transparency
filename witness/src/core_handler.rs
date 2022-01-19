use config::Committee;
use crypto::KeyPair;
use log::{debug, warn};
use messages::ensure;
use messages::error::{WitnessError, WitnessResult};
use messages::publish::{
    PublishCertificate, PublishMessage, PublishNotification, PublishVote, SequenceNumber,
};
use tokio::sync::mpsc::Receiver;

/// Core logic handing publish notifications and certificates.
pub struct PublishHandler {
    /// The keypair of this authority.
    keypair: KeyPair,
    /// The committee information.
    committee: Committee,
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
        rx_notification: Receiver<PublishNotification>,
        rx_certificate: Receiver<PublishCertificate>,
    ) {
        tokio::spawn(async move {
            // TODO: Read the sequence number and lock from storage.
            let sequence_number = 0;
            let lock = None;

            // Run an instance of the handler.
            Self {
                keypair,
                committee,
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

                            // Register the lock.
                            self.lock = Some(vote);
                            // TODO: persist the lock.

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

                                // Publish the witness state.
                                self.sequence_number += 1;
                                self.lock = None;
                                // TODO: persist the state.
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
