use crate::{Batch, Request};
use akd::storage::types::{AkdLabel, AkdValue};
use bytes::Bytes;
use log::{debug, warn};
use messages::error::{IdpError, IdpResult};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::task::JoinHandle;
use tokio::time::{sleep, Duration, Instant};

/// Assemble clients requests into batches.
pub struct Batcher {
    /// The preferred batch size (in bytes).
    batch_size: usize,
    /// The maximum delay after which to seal the batch (in ms).
    max_batch_delay: u64,
    /// Channel to receive requests from the network.
    tx_request: Receiver<Bytes>,
    /// Output channel to deliver sealed batches to the `NotificationMaker`.
    tx_batch: Sender<Batch>,
    /// Holds the current batch.
    current_batch: Batch,
    /// Holds the size of the current batch (in bytes).
    current_batch_size: usize,
}

impl Batcher {
    /// Spawn a new `Batcher` task.
    pub fn spawn(
        batch_size: usize,
        max_batch_delay: u64,
        tx_request: Receiver<Bytes>,
        tx_batch: Sender<Batch>,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            Self {
                batch_size,
                max_batch_delay,
                tx_request,
                tx_batch,
                current_batch: Vec::with_capacity(2 * batch_size),
                current_batch_size: 0,
            }
            .run()
            .await
        })
    }

    /// Deserialize client requests into a format understandable by `akd`.
    fn deserialize(bytes: &[u8]) -> IdpResult<Request> {
        if bytes.len() < 2 {
            return Err(IdpError::InvalidRequest);
        }
        let mut iter = bytes.chunks(2);
        let key = String::from_utf8_lossy(iter.next().unwrap()).to_string();
        let value = String::from_utf8_lossy(iter.next().unwrap()).to_string();
        Ok((AkdLabel(key), AkdValue(value)))
    }

    /// Main loop receiving incoming requests and creating batches.
    async fn run(&mut self) {
        let timer = sleep(Duration::from_millis(self.max_batch_delay));
        tokio::pin!(timer);

        loop {
            tokio::select! {
                // Assemble client requests into batches of preset size.
                Some(bytes) = self.tx_request.recv() => {
                    let update = match Self::deserialize(&bytes) {
                        Ok(x) => x,
                        Err(e) => {
                            warn!("{}", e);
                            continue;
                        }
                    };

                    self.current_batch_size += 1;
                    self.current_batch.push(update);
                    if self.current_batch_size >= self.batch_size {
                        self.seal().await;
                        timer.as_mut().reset(Instant::now() + Duration::from_millis(self.max_batch_delay));
                    }
                },

                // If the timer triggers, seal the batch even if it contains few transactions.
                () = &mut timer => {
                    if !self.current_batch.is_empty() {
                        debug!("Timer triggered, sealing batch now");
                        self.seal().await;
                    }
                    timer.as_mut().reset(Instant::now() + Duration::from_millis(self.max_batch_delay));
                }
            }

            // Give the change to schedule other tasks.
            tokio::task::yield_now().await;
        }
    }

    /// Seal the current batch.
    async fn seal(&mut self) {
        self.current_batch_size = 0;
        let batch: Batch = self.current_batch.drain(..).collect();
        self.tx_batch
            .send(batch)
            .await
            .expect("Failed to deliver batch");
    }
}
