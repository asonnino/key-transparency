#![allow(dead_code)]

use akd::directory::Directory;
use akd::storage::memory::AsyncInMemoryDatabase;
use akd::storage::types::{AkdLabel, AkdValue};
use bytes::Bytes;
use config::Committee;
use crypto::KeyPair;
use messages::publish::{Proof, PublishCertificate, PublishNotification, PublishVote};
use messages::{Blake3, IdPToWitnessMessage, Root};

/// Create a publish proof from a tree with the specified number of key-value pairs and an in-memory storage.
pub async fn proof(entries: usize) -> (Root, Root, Proof) {
    let db = AsyncInMemoryDatabase::new();
    proof_with_storage(entries, db).await
}

/// Create a publish proof from a tree with the specified number of key-value pairs and storage.
pub async fn proof_with_storage<AkdStorage>(entries: usize, db: AkdStorage) -> (Root, Root, Proof)
where
    AkdStorage: akd::storage::Storage + Sync + Send + 'static,
{
    // Create the list of 64-bytes key-value pairs (in memory).
    let items: Vec<_> = (0..entries)
        .map(|i| {
            let key = format!("key-{:>27}", i);
            let value = format!("value-{:>25}", i);
            (AkdLabel(key), AkdValue(value))
        })
        .collect();

    // Create a test tree with the specified number of key-values.
    let mut akd = Directory::<_>::new::<Blake3>(&db).await.unwrap();
    akd.publish::<Blake3>(items, false).await.unwrap();

    // Compute the start root (at sequence 0) and end root (at sequence 1).
    let current_azks = akd.retrieve_current_azks().await.unwrap();
    let start = akd
        .get_root_hash_at_epoch::<Blake3>(&current_azks, 0)
        .await
        .unwrap();
    let end = akd
        .get_root_hash_at_epoch::<Blake3>(&current_azks, 1)
        .await
        .unwrap();

    // Generate the audit proof.
    let proof = akd.audit::<Blake3>(0, 1).await.unwrap();
    (start, end, proof)
}

/// Make dumb (but valid) publish notifications.
pub struct NotificationGenerator<'a> {
    /// The keypair of the IdP to generate the notification.
    keypair: &'a KeyPair,
    /// The end state root (to verify the proof).
    root: Root,
    /// A state proof to re-use in every notification.
    proof: Proof,
}

impl<'a> NotificationGenerator<'a> {
    pub async fn new(keypair: &'a KeyPair, proof_entries: usize) -> NotificationGenerator<'a> {
        let (_, root, proof) = proof(proof_entries).await;
        Self {
            keypair,
            root,
            proof,
        }
    }

    /// Make a dummy (but valid) publish notification.
    pub fn make_notification(&self, sequence_number: u64) -> Bytes {
        let notification =
            PublishNotification::new(self.root, self.proof.clone(), sequence_number, self.keypair);
        let message = IdPToWitnessMessage::PublishNotification(notification);
        let serialized = bincode::serialize(&message).unwrap();
        Bytes::from(serialized)
    }
}

/// Make dumb (but valid) publish certificates.
pub struct CertificateGenerator {
    /// The committee information.
    pub committee: Committee,
    votes: Vec<PublishVote>,
}

impl CertificateGenerator {
    pub fn new(committee: Committee) -> Self {
        Self {
            committee,
            votes: Vec::new(),
        }
    }

    /// Reset the certificate generator.
    pub fn clear(&mut self) {
        self.votes.clear();
    }

    /// Try to assemble a certificate from votes.
    pub fn try_make_certificate(&mut self, vote: PublishVote) -> Option<Bytes> {
        self.votes.push(vote);
        (self.votes.len() >= self.committee.quorum_threshold() as usize).then(|| {
            let certificate = PublishCertificate {
                root: self.votes[0].root,
                sequence_number: self.votes[0].sequence_number,
                votes: self
                    .votes
                    .drain(..)
                    .map(|v| (v.author, v.signature))
                    .collect(),
            };
            let message = IdPToWitnessMessage::PublishCertificate(certificate);
            let serialized = bincode::serialize(&message).unwrap();
            Bytes::from(serialized)
        })
    }
}
