use crate::publish::{PublishMessage, PublishVote, SequenceNumber};
use crypto::Digest;
use serde::{Deserialize, Serialize};

/// The safety-critical state of a witness.
#[derive(Serialize, Deserialize)]
pub struct State {
    /// The latest root commitment.
    root: Digest,
    /// The current sequence number.
    sequence_number: SequenceNumber,
    /// The notification on which this entity is locked.
    lock: Option<PublishVote>,
}

impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(
            f,
            "State{}({}, {:?})",
            self.sequence_number,
            self.root,
            self.lock.as_ref().map(|vote| vote.digest())
        )
    }
}
