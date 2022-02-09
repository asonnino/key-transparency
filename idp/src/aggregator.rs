use config::Committee;
use messages::publish::{PublishCertificate, PublishVote};

pub struct Aggregator {
    _committee: Committee,
    votes: Vec<PublishVote>,
}

impl Aggregator {
    pub fn new(committee: Committee) -> Self {
        Self {
            _committee: committee,
            votes: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.votes.clear();
    }

    pub fn append(&mut self, _vote: PublishVote) -> Option<PublishCertificate> {
        None
    }
}
