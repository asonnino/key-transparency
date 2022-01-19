use crate::publish::SequenceNumber;
use crypto::{CryptoError, Digest, PublicKey};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[macro_export]
macro_rules! bail {
    ($e:expr) => {
        return Err($e);
    };
}

#[macro_export(local_inner_macros)]
macro_rules! ensure {
    ($cond:expr, $e:expr) => {
        if !($cond) {
            bail!($e);
        }
    };
}

/// Convenient result wrappers.
pub type MessageResult<T> = Result<T, MessageError>;
pub type WitnessResult<T> = Result<T, WitnessError>;

/// Errors triggered when parsing and verifying protocol messages.
#[derive(Debug, Error, Serialize, Deserialize)]
pub enum MessageError {
    #[error("Malformed notification id {0}")]
    MalformedNotificationId(Digest),

    #[error("Invalid signature: {0}")]
    InvalidSignature(String),

    #[error("Received message from unknown witness {0}")]
    UnknownWitness(PublicKey),

    #[error("Witness {0} appears in quorum more than once")]
    WitnessReuse(PublicKey),

    #[error("Received certificate without a quorum")]
    CertificateRequiresQuorum,
}

impl From<CryptoError> for MessageError {
    fn from(error: CryptoError) -> Self {
        MessageError::InvalidSignature(error.to_string())
    }
}

/// Errors triggered by the witness when processing IdP's messages.
#[derive(Debug, Error, Serialize, Deserialize)]
pub enum WitnessError {
    #[error(transparent)]
    MessageError(#[from] MessageError),

    #[error("Received unexpected sequence number, received {expected} but got {got}")]
    UnexpectedSequenceNumber {
        expected: SequenceNumber,
        got: SequenceNumber,
    },

    #[error("Received conflicting notifications for the same sequence number: {0} != {1}")]
    ConflictingNotification(Digest, Digest),

    #[error("Missing earlier certificates, current sequence number at {0}")]
    MissingEarlierCertificates(SequenceNumber),
}
