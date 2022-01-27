mod common;

use common::{
    broadcast_certificate, broadcast_notification, committee, delete_storage, keys, notification,
    spawn_witnesses, votes,
};
use crypto::Digest;
use function_name::named;
use futures::future::try_join_all;
use messages::error::WitnessError;
use messages::publish::{
    Proof, PublishCertificate, PublishNotification, PublishVote, Root, SequenceNumber,
};
use messages::sync::State;
use messages::WitnessToIdPMessage;

#[tokio::test]
#[named]
async fn correct_notification() {
    let base_port = 7_000;
    let committee = committee(base_port);
    let test_id = function_name!();

    // Spawn 4 witnesses.
    spawn_witnesses(&test_id, &committee);
    tokio::task::yield_now().await;

    // Broadcast a publish notification.
    let notification = notification();
    let handles = broadcast_notification(notification, &committee).await;

    // Wait for the witnesses' replies.
    let mut replies: Vec<_> = try_join_all(handles)
        .await
        .unwrap()
        .iter()
        .map(|reply| match bincode::deserialize(&reply).unwrap() {
            WitnessToIdPMessage::PublishVote(Ok(vote)) => vote,
            _ => panic!("Unexpected protocol message"),
        })
        .collect();
    replies.sort_by_key(|x| x.author);

    // Ensure the received votes are as expected.
    let mut expected_votes = votes();
    expected_votes.sort_by_key(|x| x.author);
    assert_eq!(replies, expected_votes);

    // Delete the storage.
    delete_storage(&test_id);
}

#[tokio::test]
#[named]
async fn unexpected_sequence_number() {
    let base_port = 7_100;
    let committee = committee(base_port);
    let test_id = function_name!();

    // Spawn 4 witnesses.
    spawn_witnesses(&test_id, &committee);
    tokio::task::yield_now().await;

    // Make a publish notification with a bad sequence number.
    let bad_sequence_number = SequenceNumber::default() + 1;
    let (_, identity_provider) = keys().pop().unwrap();
    let notification = PublishNotification::new(
        /* root */ Root::default(),
        /* proof */ Proof::default(),
        /* sequence_number */ bad_sequence_number,
        /* keypair */ &identity_provider,
    );

    // Broadcast the notification.
    let handles = broadcast_notification(notification, &committee).await;

    // Ensure the witnesses' replies are as expected.
    for reply in try_join_all(handles).await.unwrap() {
        match bincode::deserialize(&reply).unwrap() {
            WitnessToIdPMessage::PublishVote(Err(WitnessError::UnexpectedSequenceNumber {
                expected,
                got,
            })) => {
                assert_eq!(expected, SequenceNumber::default());
                assert_eq!(got, bad_sequence_number);
            }
            _ => panic!("Unexpected protocol message"),
        }
    }

    // Delete the storage.
    delete_storage(&test_id);
}

#[tokio::test]
#[named]
async fn conflicting_notification() {
    let base_port = 7_200;
    let committee = committee(base_port);
    let test_id = function_name!();

    // Spawn 4 witnesses.
    spawn_witnesses(&test_id, &committee);
    tokio::task::yield_now().await;

    // Broadcast a first notification.
    let notification = notification();
    let notification_root = notification.root.clone();
    let handles = broadcast_notification(notification, &committee).await;
    let _ = try_join_all(handles).await.unwrap();

    // Broadcast a conflicting notification.
    let (_, identity_provider) = keys().pop().unwrap();
    let conflict = PublishNotification::new(
        /* root */ Digest([1; 32]),
        /* proof */ Proof::default(),
        /* sequence_number */ SequenceNumber::default(),
        /* keypair */ &identity_provider,
    );
    let conflict_root = conflict.root.clone();
    let handles = broadcast_notification(conflict, &committee).await;

    // Ensure the witnesses' replies are as expected.
    for reply in try_join_all(handles).await.unwrap() {
        match bincode::deserialize(&reply).unwrap() {
            WitnessToIdPMessage::PublishVote(Err(WitnessError::ConflictingNotification {
                lock,
                received,
            })) => {
                assert_eq!(lock, notification_root);
                assert_eq!(received, conflict_root);
            }
            _ => panic!("Unexpected protocol message"),
        }
    }

    // Delete the storage.
    delete_storage(&test_id);
}

#[tokio::test]
#[named]
async fn expected_certificate() {
    let base_port = 7_300;
    let committee = committee(base_port);
    let test_id = function_name!();

    // Spawn 4 witnesses.
    spawn_witnesses(&test_id, &committee);
    tokio::task::yield_now().await;

    // Broadcast a certificate.
    let notification = notification();
    let certificate = PublishCertificate {
        root: notification.root.clone(),
        sequence_number: notification.sequence_number,
        votes: votes()
            .into_iter()
            .map(|x| (x.author, x.signature))
            .collect(),
    };
    let handles = broadcast_certificate(certificate, &committee).await;

    // Make the expected state.
    let expected = State {
        root: Root::default(),
        sequence_number: SequenceNumber::default() + 1,
        lock: None,
    };

    // Ensure the witnesses' replies are as expected.
    for reply in try_join_all(handles).await.unwrap() {
        match bincode::deserialize(&reply).unwrap() {
            WitnessToIdPMessage::State(Ok(state)) => assert_eq!(state, expected),
            _ => panic!("Unexpected protocol message"),
        }
    }

    // Delete the storage.
    delete_storage(&test_id);
}

#[tokio::test]
#[named]
async fn unexpected_certificate() {
    let base_port = 7_400;
    let committee = committee(base_port);
    let test_id = function_name!();

    // Spawn 4 witnesses.
    spawn_witnesses(&test_id, &committee);
    tokio::task::yield_now().await;

    // Make a publish certificate for a future sequence number.
    let future_sequence_number = SequenceNumber::default() + 1;
    let (_, identity_provider) = keys().pop().unwrap();

    let notification = PublishNotification::new(
        /* root */ Root::default(),
        /* proof */ Proof::default(),
        /* sequence_number */ future_sequence_number,
        /* keypair */ &identity_provider,
    );

    let votes: Vec<_> = keys()
        .iter()
        .map(|(_, keypair)| PublishVote::new(&notification, keypair))
        .collect();

    let certificate = PublishCertificate {
        root: notification.root.clone(),
        sequence_number: notification.sequence_number,
        votes: votes.into_iter().map(|x| (x.author, x.signature)).collect(),
    };

    // Broadcast the certificate.
    let handles = broadcast_certificate(certificate, &committee).await;

    // Ensure the witnesses' replies are as expected.
    for reply in try_join_all(handles).await.unwrap() {
        match bincode::deserialize(&reply).unwrap() {
            WitnessToIdPMessage::State(Err(WitnessError::MissingEarlierCertificates(seq))) => {
                assert_eq!(seq, SequenceNumber::default());
            }
            _ => panic!("Unexpected protocol message"),
        }
    }

    // Delete the storage.
    delete_storage(&test_id);
}
