use function_name::named;
use futures::future::try_join_all;
use network::reliable_sender::ReliableSender;
use test_utils::{committee, keys, listener, serialized_update, spawn_test_idp};

#[tokio::test]
#[named]
async fn correct_update() {
    let base_port = 9_000;
    let committee = committee(base_port);
    let address = committee.idp.address;
    let test_id = function_name!();

    // Spawn the IdP.
    spawn_test_idp(&test_id, committee.clone());
    tokio::task::yield_now().await;

    // Spawn the listeners acting as witnesses.
    let received: Vec<_> = keys()
        .into_iter()
        .map(|(name, key)| {
            let address = committee.witness_address(&name).unwrap();
            listener(address, key)
        })
        .collect();

    // Send a enough correct updates to create a batch.
    let mut network = ReliableSender::new();
    for _ in 0..2 {
        let update = serialized_update();
        let handle = network.send(address, update).await;
        handle.await.unwrap();
    }

    //let (notifications, certificates) = try_join_all(received).await.into_iter().unzip();
    for x in try_join_all(received).await {
        println!("{:?}", x);
    }
}
