pub mod error;
pub mod publish;
pub mod sync;

use error::WitnessResult;
use publish::{PublishCertificate, PublishNotification, PublishVote};
use serde::{Deserialize, Serialize};
use sync::{PublishCertificateQuery, State};
use winter_crypto::hashers::Blake3_256;
use winter_crypto::{Digest as _, Hasher};
use winter_math::fields::f128::BaseElement;
use winter_utils::{Deserializable, SliceReader};

/// The sequence number of consistent (or reliable) broadcast.
pub type SequenceNumber = u64;

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

// The hasher for the state tree.
pub type Blake3 = Blake3_256<BaseElement>;

/// Represents a state root.
pub type Root = <Blake3 as Hasher>::Digest;

/// A serde serializer for the type `winter_crypto::Digest`
pub fn serialize_root<S>(x: &Root, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    x.as_bytes().serialize(s)
}

/// A serde deserializer for the type `winter_crypto::Digest`
pub fn deserialize_root<'de, D>(deserializer: D) -> Result<Root, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let buf = <[u8; 32]>::deserialize(deserializer)?;
    Root::read_from(&mut SliceReader::new(&buf)).map_err(serde::de::Error::custom)
}
