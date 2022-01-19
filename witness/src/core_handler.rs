use crate::ensure;
use crate::error::{WitnessError, WitnessResult};
use config::Committee;
use crypto::KeyPair;
use log::{debug, warn};
use messages::update::{
    Digestible, SequenceNumber, UpdateCertificate, UpdateNotification, UpdateVote,
};
use tokio::sync::mpsc::Receiver;

/// Core logic handing update notifications and certificates.
pub struct UpdateHandler {
    /// The keypair of this authority.
    keypair: KeyPair,
    /// The committee information.
    committee: Committee,
    /// Receive update notifications from the IdP.
    rx_notification: Receiver<UpdateNotification>,
    /// Receive update certificates from the IdP.
    rx_certificate: Receiver<UpdateCertificate>,
    /// The current sequence number.
    sequence_number: SequenceNumber,
    /// The notification on which this witness is locked.
    lock: Option<UpdateVote>,
}

impl UpdateHandler {
    /// Spawn a new update handler task.
    pub fn spawn(
        keypair: KeyPair,
        committee: Committee,
        rx_notification: Receiver<UpdateNotification>,
        rx_certificate: Receiver<UpdateCertificate>,
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

    /// Try to vote for an update notification.
    fn make_vote(&self, notification: &UpdateNotification) -> WitnessResult<UpdateVote> {
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
            None => Ok(UpdateVote::new(notification, &self.keypair)),
        }
    }

    /// Process an update certificate.
    fn process_certificate(&self, certificate: &UpdateCertificate) -> WitnessResult<()> {
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
                // Receive update notifications.
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

                // Receive update certificates.
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
