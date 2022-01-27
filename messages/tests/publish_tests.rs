use messages::publish::Root;
use test_utils::{certificate, committee, notification, votes};

#[test]
fn verify_notification() {
    let notification = notification();
    assert!(notification.verify(&committee(0), &Root::default()).is_ok());
}

#[test]
fn verify_vote() {
    let vote = votes().pop().unwrap();
    assert!(vote.verify(&committee(0)).is_ok());
}

#[test]
fn verify_certificate() {
    let certificate = certificate();
    assert!(certificate.verify(&committee(0)).is_ok());
}
