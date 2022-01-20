pub mod error;
pub mod publish;
pub mod sync;

use error::WitnessResult;
use publish::{PublishCertificate, PublishNotification, PublishVote};
use serde::{Deserialize, Serialize};

/// Messages sent by the IdP to the witnesses.
#[derive(Serialize, Deserialize)]
pub enum IdPtoWitnessMessage {
    PublishNotification(PublishNotification),
    PublishCertificate(PublishCertificate),
}

/// Replies sent by the witnesses to the IdP.
#[derive(Serialize, Deserialize)]
pub enum WitnessToIdPMessage {
    PublishVote(WitnessResult<PublishVote>),
}
