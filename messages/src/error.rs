use crypto::{CryptoError, Digest, PublicKey};
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

pub type MessageResult<T> = Result<T, MessageError>;

#[derive(Debug, Error)]
pub enum MessageError {
    #[error("Malformed notification id {0}")]
    MalformedNotificationId(Digest),

    #[error("Invalid signature")]
    InvalidSignature(#[from] CryptoError),

    #[error("Received message from unknown witness {0}")]
    UnknownWitness(PublicKey),

    #[error("Witness {0} appears in quorum more than once")]
    WitnessReuse(PublicKey),

    #[error("Received certificate without a quorum")]
    CertificateRequiresQuorum,
}
