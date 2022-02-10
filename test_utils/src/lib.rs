use akd::directory::Directory;
use akd::storage::memory::AsyncInMemoryDatabase;
use akd::storage::types::{AkdLabel, AkdValue};
use bytes::Bytes;
use config::{Committee, Idp, Witness};
use crypto::{KeyPair, PublicKey};
use messages::publish::{Proof, PublishCertificate, PublishNotification, PublishVote};
use messages::IdPToWitnessMessage;
use messages::{Blake3, Root};
use network::reliable_sender::{CancelHandler, ReliableSender};
use rand::rngs::StdRng;
use rand::SeedableRng;
use storage::Storage;
use witness::spawn_witness;

// Test cryptographic keys.
pub fn keys() -> Vec<(PublicKey, KeyPair)> {
    let mut rng = StdRng::from_seed([0; 32]);
    (0..4)
        .map(|_| KeyPair::generate_keypair(&mut rng))
        .collect()
}

// Test committee.
pub fn committee(base_port: u16) -> Committee {
    let (identity_provider, _) = keys().pop().unwrap();
    Committee {
        idp: Idp {
            name: identity_provider,
            address: format!("127.0.0.1:{}", base_port + 100).parse().unwrap(),
        },
        witnesses: keys()
            .into_iter()
            .enumerate()
            .map(|(i, (name, _))| {
                (
                    name,
                    Witness {
                        voting_power: 1,
                        address: format!("127.0.0.1:{}", base_port + i as u16)
                            .parse()
                            .unwrap(),
                    },
                )
            })
            .collect(),
    }
}

// Test proof and root hashes.
pub async fn proof() -> (Root, Root, Proof) {
    // Create a test tree with dumb key-values.
    let db = AsyncInMemoryDatabase::new();
    let mut akd = Directory::<_>::new::<Blake3>(&db).await.unwrap();
    akd.publish::<Blake3>(
        vec![
            (AkdLabel("A".to_string()), AkdValue("B".to_string())),
            (AkdLabel("C".to_string()), AkdValue("D".to_string())),
        ],
        false,
    )
    .await
    .unwrap();

    // Compute the start root (at sequence 0) and end root (at sequence 1).
    let current_azks = akd.retrieve_current_azks().await.unwrap();
    let start_root = akd
        .get_root_hash_at_epoch::<Blake3>(&current_azks, /* sequence number */ 0)
        .await
        .unwrap();
    let end_root = akd
        .get_root_hash_at_epoch::<Blake3>(&current_azks, /* sequence number */ 1)
        .await
        .unwrap();

    // Generate the audit proof.
    let proof = akd.audit::<Blake3>(0, 1).await.unwrap();

    // Return the start root, end root, and the audit proof.
    (start_root, end_root, proof)
}

// Test publish notification.
pub async fn notification() -> PublishNotification {
    let (_, identity_provider) = keys().pop().unwrap();
    let (_, root, proof) = proof().await;
    PublishNotification::new(
        root,
        proof,
        /* sequence_number */ 1,
        /* keypair */ &identity_provider,
    )
}

// The witnesses' votes over a test notification.
pub async fn votes() -> Vec<PublishVote> {
    let notification = notification().await;
    keys()
        .iter()
        .map(|(_, keypair)| PublishVote::new(&notification, keypair))
        .collect()
}

// A test certificate.
pub async fn certificate() -> PublishCertificate {
    let notification = notification().await;
    PublishCertificate {
        root: notification.root,
        sequence_number: notification.sequence_number,
        votes: votes()
            .await
            .into_iter()
            .map(|x| (x.author, x.signature))
            .collect(),
    }
}

// Spawn test witnesses.
pub fn spawn_witnesses(test_id: &str, committee: &Committee) {
    delete_storage(test_id);
    for (i, (_, keypair)) in keys().into_iter().enumerate() {
        let secure_storage_path = format!(".test_secure_storage_{}_{}", test_id, i);
        let secure_storage = Storage::new(&secure_storage_path).unwrap();

        let audit_storage_path = format!(".test_audit_storage_{}_{}", test_id, i);
        let audit_storage = Storage::new(&audit_storage_path).unwrap();

        spawn_witness(keypair, committee.clone(), secure_storage, audit_storage);
    }
}

// Helper function deleting a test storage.
pub fn delete_storage(test_id: &str) {
    for i in 0..keys().len() {
        let secure_storage_path = format!(".test_secure_storage_{}_{}", test_id, i);
        let _ = std::fs::remove_dir_all(&secure_storage_path);
        let audit_storage_path = format!(".test_audit_storage_{}_{}", test_id, i);
        let _ = std::fs::remove_dir_all(&audit_storage_path);
    }
}

// Broadcast a publish notification to the witnesses.
pub async fn broadcast_notification(
    notification: PublishNotification,
    committee: &Committee,
) -> Vec<CancelHandler> {
    let addresses = committee
        .witnesses_addresses()
        .into_iter()
        .map(|(_, address)| address)
        .collect();
    let message = IdPToWitnessMessage::PublishNotification(notification);
    let serialized = bincode::serialize(&message).unwrap();
    let bytes = Bytes::from(serialized);
    let mut sender = ReliableSender::new();
    sender.broadcast(addresses, bytes).await
}

// Broadcast a publish certificate to the witnesses.
pub async fn broadcast_certificate(
    certificate: PublishCertificate,
    committee: &Committee,
) -> Vec<CancelHandler> {
    let addresses = committee
        .witnesses_addresses()
        .into_iter()
        .map(|(_, address)| address)
        .collect();
    let message = IdPToWitnessMessage::PublishCertificate(certificate);
    let serialized = bincode::serialize(&message).unwrap();
    let bytes = Bytes::from(serialized);
    let mut sender = ReliableSender::new();
    sender.broadcast(addresses, bytes).await
}
