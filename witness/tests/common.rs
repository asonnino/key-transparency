use bytes::Bytes;
use config::{Committee, Witness};
use crypto::{KeyPair, PublicKey};
use messages::publish::{
    Proof, PublishCertificate, PublishNotification, PublishVote, Root, SequenceNumber,
};
use messages::IdPToWitnessMessage;
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
        identity_provider,
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

// Test publish notification.
pub fn notification() -> PublishNotification {
    let (_, identity_provider) = keys().pop().unwrap();
    PublishNotification::new(
        /* root */ Root::default(),
        /* proof */ Proof::default(),
        /* sequence_number */ SequenceNumber::default(),
        /* keypair */ &identity_provider,
    )
}

// The witnesses' votes over a test notification.
pub fn votes() -> Vec<PublishVote> {
    let notification = notification();
    keys()
        .iter()
        .map(|(_, keypair)| PublishVote::new(&notification, keypair))
        .collect()
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
