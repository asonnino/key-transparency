use bytes::Bytes;
use config::{Committee, Witness};
use crypto::{KeyPair, PublicKey};
use futures::future::try_join_all;
use messages::publish::{Proof, PublishNotification, PublishVote, Root, SequenceNumber};
use messages::{IdPToWitnessMessage, WitnessToIdPMessage};
use network::reliable_sender::ReliableSender;
use rand::rngs::StdRng;
use rand::SeedableRng;
use storage::Storage;
use witness::spawn_witness;

pub fn keys() -> Vec<(PublicKey, KeyPair)> {
    let mut rng = StdRng::from_seed([0; 32]);
    (0..4)
        .map(|_| KeyPair::generate_keypair(&mut rng))
        .collect()
}

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

#[tokio::test]
async fn send_notification() {
    let base_port = 7_000;
    let committee = committee(base_port);

    // Make a publish notification.
    let (_, identity_provider) = keys().pop().unwrap();
    let notification = PublishNotification::new(
        /* root */ Root::default(),
        /* proof */ Proof::default(),
        /* sequence_number */ SequenceNumber::default(),
        /* keypair */ &identity_provider,
    );

    // Spawn 4 witnesses.
    let mut expected_votes = Vec::new();
    for (i, (_, keypair)) in keys().into_iter().enumerate() {
        // Compute the expected vote.
        let expected_vote = PublishVote::new(&notification, &keypair);
        expected_votes.push(expected_vote);

        // Spawn the witness.
        let secure_storage_path = format!(".test_secure_storage_send_notification_{}", i);
        let _ = std::fs::remove_dir_all(&secure_storage_path);
        let secure_storage = Storage::new(&secure_storage_path).unwrap();

        let audit_storage_path = format!(".test_audit_storage_send_notification_{}", i);
        let _ = std::fs::remove_dir_all(&audit_storage_path);
        let audit_storage = Storage::new(&audit_storage_path).unwrap();

        spawn_witness(keypair, committee.clone(), secure_storage, audit_storage);
    }
    tokio::task::yield_now().await;

    // Broadcast the request to the witnesses.
    let addresses = committee
        .witnesses_addresses()
        .into_iter()
        .map(|(_, address)| address)
        .collect();
    let message = IdPToWitnessMessage::PublishNotification(notification);
    let serialized_notification = bincode::serialize(&message).unwrap();
    let handles = ReliableSender::new()
        .broadcast(addresses, Bytes::from(serialized_notification))
        .await;

    // Wait for the witnesses reply and ensure they are as expected.
    let mut votes: Vec<_> = try_join_all(handles)
        .await
        .unwrap()
        .iter()
        .map(|reply| match bincode::deserialize(&reply).unwrap() {
            WitnessToIdPMessage::PublishVote(Ok(vote)) => vote,
            _ => panic!("Unexpected protocol message"),
        })
        .collect();

    // Ensure the received votes are as expected.
    expected_votes.sort_by_key(|x| x.author);
    votes.sort_by_key(|x| x.author);
    assert_eq!(votes, expected_votes);
}
