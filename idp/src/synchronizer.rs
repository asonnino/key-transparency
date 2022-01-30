use bytes::Bytes;
use config::{Committee, VotingPower};
use crypto::PublicKey;
use futures::stream::futures_unordered::FuturesUnordered;
use futures::stream::StreamExt;
use messages::IdPToWitnessMessage;
use network::reliable_sender::{CancelHandler, ReliableSender};
use std::net::SocketAddr;

pub struct Synchronizer {
    committee: Committee,
    names: Vec<PublicKey>,
    addresses: Vec<SocketAddr>,
    network: ReliableSender,
}

impl Synchronizer {
    pub fn new(committee: Committee) -> Self {
        let (names, addresses): (Vec<_>, _) =
            committee.witnesses_addresses().iter().cloned().unzip();
        Self {
            committee,
            names,
            addresses,
            network: ReliableSender::new(),
        }
    }
    pub async fn synchronize_witnesses() {
        // Query the current state of the witnesses.
        // Gather the missing certificates.
    }

    /// Helper function. It waits for a future to complete and then delivers a value.
    async fn waiter(wait_for: CancelHandler, deliver: VotingPower) -> VotingPower {
        let _ = wait_for.await;
        deliver
    }

    async fn query_state(&mut self) {
        // Broadcast the state query.
        let message = IdPToWitnessMessage::StateQuery;
        let serialized = bincode::serialize(&message).expect("Failed to serialize state query");
        let bytes = Bytes::from(serialized);
        let handles = self.network.broadcast(self.addresses.clone(), bytes).await;

        // Collect the handlers.
        let mut wait_for_quorum: FuturesUnordered<_> = self
            .names
            .iter()
            .zip(handles.into_iter())
            .into_iter()
            .map(|(name, handler)| {
                let stake = self.committee.voting_power(name);
                Self::waiter(handler, stake)
            })
            .collect();

        // Wait for the first 2f nodes to send back an Ack. Then we consider the batch delivered and we
        // send its digest to the primary (that will include it into the dag). This should reduce the
        // amount of synching.
        let mut total_voting_power = VotingPower::default();
        while let Some(voting_power) = wait_for_quorum.next().await {
            total_voting_power += voting_power;
            if total_voting_power >= self.committee.quorum_threshold() {
                // TODO
                break;
            }
        }

        // TODO: Wait for a bit longer to give time to slow authorities.
    }
}
