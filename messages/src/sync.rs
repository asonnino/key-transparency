use crate::publish::{PublishMessage, PublishVote, Root, SequenceNumber};
use serde::{Deserialize, Serialize};

/// The safety-critical state of a witness.
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct State {
    /// The latest root commitment.
    pub root: Root,
    /// The current sequence number.
    pub sequence_number: SequenceNumber,
    /// The notification on which this entity is locked.
    pub lock: Option<PublishVote>,
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

// Useful for tests.
impl PartialEq for State {
    fn eq(&self, other: &Self) -> bool {
        self.root == other.root
            && self.sequence_number == other.sequence_number
            && self.lock == other.lock
    }
}

/// Request of a publish certificate request.
#[derive(Serialize, Deserialize)]
pub struct PublishCertificateQuery {
    /// The sequence number of the requested certificate.
    pub sequence_number: SequenceNumber,
}

impl std::fmt::Debug for PublishCertificateQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "CertRequest({})", self.sequence_number)
    }
}
