pub mod publish_handler;
pub mod sync_helper;

use messages::WitnessToIdPMessage;
use tokio::sync::oneshot;

/// One-shot channel to reply to the IdP.
pub type Replier = oneshot::Sender<WitnessToIdPMessage>;
