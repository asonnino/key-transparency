use messages::publish::{Proof, PublishNotification, Root, SequenceNumber};
use test_utils::{committee, keys};

#[test]
fn verify_notification() {
    let (_, identity_provider) = keys().pop().unwrap();
    let notification = PublishNotification::new(
        /* root */ Root::default(),
        /* proof */ Proof::default(),
        /* sequence_number */ SequenceNumber::default(),
        /* keypair */ &identity_provider,
    );
    assert!(notification.verify(&committee(0), &Root::default()).is_ok());
}
