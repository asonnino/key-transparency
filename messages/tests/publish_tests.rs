use messages::Root;
use test_utils::{certificate, committee, notification, votes};

#[tokio::test]
async fn verify_notification() {
    let notification = notification().await;
    assert!(notification
        .verify(&committee(0), &Root::default())
        .await
        .is_ok());
}

#[tokio::test]
async fn verify_vote() {
    let vote = votes().await.pop().unwrap();
    assert!(vote.verify(&committee(0)).is_ok());
}

#[tokio::test]
async fn verify_certificate() {
    let certificate = certificate().await;
    assert!(certificate.verify(&committee(0)).is_ok());
}
