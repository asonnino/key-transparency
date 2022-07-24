#![allow(dead_code)]

use akd::{
    directory::Directory,
    ecvrf::HardCodedAkdVRF,
    storage::{
        memory::AsyncInMemoryDatabase,
        types::{AkdLabel, AkdValue},
    },
};
use bytes::{BufMut, Bytes, BytesMut};
use config::Committee;
use crypto::KeyPair;
use messages::{
    publish::{Proof, PublishCertificate, PublishNotification, PublishVote},
    Blake3, IdPToWitnessMessage, Root,
};

// The size of the AkdLabel and AkdValue
const LABEL_VALUE_SIZE_BYTES: usize = 32;

/// Create a publish proof from a tree with the specified number of key-value pairs and an in-memory storage.
pub async fn proof(entries: u64) -> (Root, Root, Proof) {
    let db = AsyncInMemoryDatabase::new();
    proof_with_storage(entries, db).await
}

/// Create a publish proof from a tree with the specified number of key-value pairs and storage.
pub async fn proof_with_storage<AkdStorage>(num_entries: u64, db: AkdStorage) -> (Root, Root, Proof)
where
    AkdStorage: akd::storage::Storage + Sync + Send + 'static,
{
    // Create the list of key-value pairs (in memory).
    let key_entries = generate_key_entries(num_entries);

    // Create a test tree with the specified number of key-values.
    let vrf = HardCodedAkdVRF {};
    let akd = Directory::new::<Blake3>(&db, &vrf, false).await.unwrap();

    // Compute the start root (at sequence 0) and end root (at sequence 1).
    let current_azks = akd.retrieve_current_azks().await.unwrap();
    let start = akd
        .get_root_hash_at_epoch::<Blake3>(&current_azks, 0)
        .await
        .unwrap();

    akd.publish::<Blake3>(key_entries).await.unwrap();

    let current_azks = akd.retrieve_current_azks().await.unwrap();
    let end = akd
        .get_root_hash_at_epoch::<Blake3>(&current_azks, 1)
        .await
        .unwrap();

    // Generate the audit proof.
    let proof = akd.audit::<Blake3>(0, 1).await.unwrap();
    (start, end, proof)
}

/// Performs a publish with number of key-value pairs. Pair creation must be done outside
/// of this function to *only* measure the publish time.
/// Note that the measurements WILL include directory creation times which should not
/// affect performance too much.
pub async fn publish_with_storage<AkdStorage>(
    key_entries: Vec<(AkdLabel, AkdValue)>,
    db: AkdStorage,
) where
    AkdStorage: akd::storage::Storage + Sync + Send,
{
    let vrf = HardCodedAkdVRF {};
    let akd = Directory::new::<Blake3>(&db, &vrf, false).await.unwrap();

    akd.publish::<Blake3>(key_entries).await.unwrap();
}

/// Performs a publish operation with given number of key entries and prints the
/// storage stats. It is not meant to be used in benches.
pub async fn publish_with_storage_stats<AkdStorage>(num_key_entries: u64, db: AkdStorage)
where
    AkdStorage: akd::storage::Storage + Sync + Send,
{
    // Setup
    let vrf = HardCodedAkdVRF {};
    let akd = Directory::new::<Blake3>(&db, &vrf, false).await.unwrap();

    // Generate keys and publish.
    // It is okay to include key generation here since this function
    // is not used in benches per-se but used for obtaining storage stats.
    let key_entries = generate_key_entries(num_key_entries);
    akd.publish::<Blake3>(key_entries).await.unwrap();

    // Display storage stats.
    println!("***********************************************************");
    println!("Number of key entries: {}", num_key_entries);
    db.log_metrics(log::Level::Debug).await;
    println!("***********************************************************");
}

pub fn generate_key_entries(num_entries: u64) -> Vec<(AkdLabel, AkdValue)> {
    let mut label = BytesMut::with_capacity(LABEL_VALUE_SIZE_BYTES);
    let mut value = BytesMut::with_capacity(LABEL_VALUE_SIZE_BYTES);

    (0..num_entries)
        .map(|i| {
            label.put_u64(i);
            let l = label.split().freeze();

            value.put_u64(i);
            let v = value.split().freeze();

            (AkdLabel(l.to_vec()), AkdValue(v.to_vec()))
        })
        .collect()
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
    pub async fn new(keypair: &'a KeyPair, proof_entries: u64) -> NotificationGenerator<'a> {
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
