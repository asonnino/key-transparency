use crypto::Digest;
use messages::error::MessageError;
use messages::update::SequenceNumber;
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

pub type WitnessResult<T> = Result<T, WitnessError>;

#[derive(Debug, Error)]
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
