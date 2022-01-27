pub mod error;
pub mod publish;
pub mod sync;

use error::WitnessResult;
use publish::{PublishCertificate, PublishNotification, PublishVote};
use serde::{Deserialize, Serialize};
use sync::{PublishCertificateQuery, State};

/// Alias for serialized publish certificates.
pub type SerializedPublishCertificateMessage = Vec<u8>;

/// Messages sent by the IdP to the witnesses.
#[derive(Serialize, Deserialize, Debug)]
pub enum IdPToWitnessMessage {
    PublishNotification(PublishNotification),
    PublishCertificate(PublishCertificate),
    StateQuery,
    PublishCertificateQuery(PublishCertificateQuery),
}

/// Replies sent by the witnesses to the IdP.
#[derive(Serialize, Deserialize, Debug)]
pub enum WitnessToIdPMessage {
    PublishVote(WitnessResult<PublishVote>),
    State(WitnessResult<State>),
    PublishCertificateResponse(SerializedPublishCertificateMessage),
}
