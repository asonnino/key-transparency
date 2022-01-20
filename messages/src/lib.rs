pub mod error;
pub mod publish;
pub mod sync;

use error::WitnessResult;
use publish::{PublishCertificate, PublishNotification, PublishVote};
use serde::{Deserialize, Serialize};
use sync::{PublishCertificateRequest, State};

/// Messages sent by the IdP to the witnesses.
#[derive(Serialize, Deserialize, Debug)]
pub enum IdPtoWitnessMessage {
    PublishNotification(PublishNotification),
    PublishCertificate(PublishCertificate),
    StateQuery,
    PublishCertificateQuery(PublishCertificateRequest),
}

/// Replies sent by the witnesses to the IdP.
#[derive(Serialize, Deserialize, Debug)]
pub enum WitnessToIdPMessage {
    PublishVote(WitnessResult<PublishVote>),
    State(WitnessResult<State>),
    PublishCertificateResponse(Vec<u8>),
}
