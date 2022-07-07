use akd::storage::types::{AkdLabel, AkdValue};

/// A client request in a format understandable by `akd`.
pub type UpdateRequest = (AkdLabel, AkdValue);

/// A batch of requests.
pub type Batch = Vec<UpdateRequest>;
