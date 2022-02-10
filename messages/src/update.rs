use crate::error::{MessageError, MessageResult};
use akd::storage::types::{AkdLabel, AkdValue};

/// A client request in a format understandable by `akd`.
pub type Request = (AkdLabel, AkdValue);

/// Deserialize client requests into a format understandable by `akd`.
pub fn deserialize_request(bytes: &[u8]) -> MessageResult<Request> {
    if bytes.len() < 2 {
        return Err(MessageError::UpdateRequestTooShort);
    }
    let mut iter = bytes.chunks(2);
    let key = String::from_utf8_lossy(iter.next().unwrap()).to_string();
    let value = String::from_utf8_lossy(iter.next().unwrap()).to_string();
    Ok((AkdLabel(key), AkdValue(value)))
}

/// A batch of requests.
pub type Batch = Vec<Request>;
