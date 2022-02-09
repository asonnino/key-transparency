mod aggregator;
pub mod batcher;
pub mod prover;
pub mod publisher;
pub mod synchronizer;

use akd::storage::types::{AkdLabel, AkdValue};

/// A client request in a format understandable by `akd`.
type Request = (AkdLabel, AkdValue);

/// A batch of requests.
pub type Batch = Vec<Request>;

/// Storage address of the sequence number.
pub const STORE_LAST_NOTIFICATION_ADDR: [u8; 32] = [255; 32];
