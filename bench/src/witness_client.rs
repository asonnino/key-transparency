mod payload_generator;

use anyhow::{anyhow, Context, Result};
use clap::{arg, crate_name, crate_version, App, AppSettings, Arg};
use config::{Committee, Import, PrivateConfig};
use crypto::KeyPair;
use futures::future::join_all;
use futures::stream::futures_unordered::FuturesUnordered;
use futures::stream::StreamExt;
use log::{debug, info, warn};
use messages::WitnessToIdPMessage;
use network::reliable_sender::ReliableSender;
use payload_generator::{CertificateGenerator, NotificationGenerator};
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio::time::{interval, sleep, Duration, Instant};

#[tokio::main]
async fn main() -> Result<()> {
    // Read the cli parameters.
    let matches = App::new(crate_name!())
        .version(crate_version!())
        .about("Benchmark client for Key Transparency witnesses.")
        .arg(Arg::new("verbose").multiple_occurrences(true).short('v'))
        .args(&[
            arg!(--idp <FILE> "The keypair of the IdP"),
            arg!(--committee <FILE> "The path to the committee file"),
            arg!(--rate <INT> "The rate (txs/s) at which to send the transactions"),
            arg!(--proof_entries <INT> "The number of key updates per proof"),
        ])
        .setting(AppSettings::ArgRequiredElseHelp)
        .get_matches();

    // Configure the logger.
    let log_level = match matches.occurrences_of("verbose") {
        0 => log::LevelFilter::Error,
        1 => log::LevelFilter::Warn,
        2 => log::LevelFilter::Info,
        3 => log::LevelFilter::Debug,
        _ => log::LevelFilter::Trace,
    };
    env_logger::Builder::new()
        .format_timestamp_millis()
        .filter_module("witness", log_level)
        .filter_module("network", log_level)
        .init();

    // Parse the input parameters.
    let idp_file = matches.value_of("idp").unwrap();
    let idp = PrivateConfig::import(idp_file).context("Failed to load IdP key file")?;

    let committee_file = matches.value_of("committee").unwrap();
    let committee = Committee::import(committee_file).context("Failed to load committee")?;

    let rate = matches
        .value_of("rate")
        .unwrap()
        .parse::<u64>()
        .context("The rate of transactions must be a non-negative integer")?;

    let proof_entries = matches
        .value_of("proof_entries")
        .unwrap()
        .parse::<usize>()
        .context("The number of key updates per proof must be a non-negative integer")?;

    // Make a benchmark client.
    let client = BenchmarkClient::new(idp.secret, committee, rate, proof_entries);
    client.print_parameters();

    // Wait for all nodes to be online and synchronized.
    client.wait().await;

    // Start the benchmark.
    client
        .benchmark()
        .await
        .context("Failed to submit transactions")
}

/// A client only useful to benchmark the witnesses.
pub struct BenchmarkClient {
    /// The key pair of the IdP.
    idp: KeyPair,
    /// The committee information.
    committee: Committee,
    /// The number of requests per seconds that this client submits.
    rate: u64,
    /// The number of key updates per proof.
    proof_entries: usize,
    /// The network address of the witnesses.
    targets: Vec<SocketAddr>,
}

impl BenchmarkClient {
    /// Creates a new benchmark client.
    pub fn new(idp: KeyPair, committee: Committee, rate: u64, proof_entries: usize) -> Self {
        let targets: Vec<_> = committee
            .witnesses_addresses()
            .into_iter()
            .map(|(_, x)| x)
            .collect();

        Self {
            idp,
            committee,
            rate,
            proof_entries,
            targets,
        }
    }

    /// Log the benchmark parameters required to compute performance.
    pub fn print_parameters(&self) {
        // NOTE: These log entries are used to compute performance.
        info!("Transactions rate: {} tx/s", self.rate);
        for target in &self.targets {
            info!("Target witness address: {}", target);
        }
    }

    /// Wait for all authorities to be online.
    pub async fn wait(&self) {
        info!("Waiting for all witnesses to be online...");
        join_all(
            self.committee
                .witnesses_addresses()
                .into_iter()
                .map(|(_, address)| {
                    tokio::spawn(async move {
                        while TcpStream::connect(address).await.is_err() {
                            sleep(Duration::from_millis(10)).await;
                        }
                    })
                }),
        )
        .await;
    }

    /// Run a benchmark with the provided parameters.
    pub async fn benchmark(&self) -> Result<()> {
        const PRECISION: u64 = 1; // Timing burst precision.
        const BURST_DURATION: u64 = 1000 / PRECISION;
        let burst = self.rate / PRECISION;
        let mut counter = 0; // Identifies sample transactions.

        // Connect to the witnesses.
        let mut network = ReliableSender::new();

        // Initiate the generator of dumb requests.
        let notification_generator =
            NotificationGenerator::new(&self.idp, self.proof_entries).await;
        let mut certificate_generator = CertificateGenerator::new(self.committee.clone());

        // Gather certificates handles to sink their response.
        let mut certificate_responses = FuturesUnordered::new();

        // Submit all transactions.
        let interval = interval(Duration::from_millis(BURST_DURATION));
        tokio::pin!(interval);

        // NOTE: This log entry is used to compute performance.
        info!("Start sending transactions");
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    let now = Instant::now();
                    for x in 1..=burst {
                        let id = counter * burst + x;
                        let bytes = notification_generator.make_notification(id);

                        // NOTE: This log entry is used to compute performance.
                        info!("Sending sample transaction {}", id);

                        let mut wait_for_quorum: FuturesUnordered<_> = network
                            .broadcast(self.targets.clone(), bytes)
                            .await
                            .into_iter()
                            .collect();

                        while let Some(bytes) = wait_for_quorum.next().await {
                            let result = match bincode::deserialize(&bytes?)? {
                                WitnessToIdPMessage::PublishVote(result) => result,
                                _ => return Err(anyhow!("Unexpected protocol message"))
                            };
                            let vote = result.context("Witness returned error")?;
                            debug!("{:?}", vote);
                            if let Some(certificate) = certificate_generator.try_make_certificate(vote)
                            {
                                // NOTE: This log entry is used to compute performance.
                                info!("Assembled certificate {}", id);

                                network
                                    .broadcast(self.targets.clone(), certificate)
                                    .await
                                    .into_iter()
                                    .for_each(|handle| certificate_responses.push(handle));

                                certificate_generator.clear();
                                break;
                            }
                        }
                    }
                    counter += 1;

                    if now.elapsed().as_millis() > BURST_DURATION as u128 {
                        // NOTE: This log entry is used to compute performance.
                        warn!("Transaction rate too high for this client");
                    }
                },
                Some(_) = certificate_responses.next() => {
                    // Sink certificates' responses
                },
                else => break
            }
        }
        Ok(())
    }
}
