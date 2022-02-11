mod utils;

use config::Committee;
use crypto::KeyPair;
use futures::executor::block_on;
use messages::publish::{PublishCertificate, PublishNotification, PublishVote};
use messages::Root;
use statistical::{mean, standard_deviation};
use std::time::Instant;
use test_utils::{certificate, committee, keys, notification, votes};
use utils::{proof, proof_with_storage};

/// The number of runs used to compute statistics.
const RUNS: usize = 10;

/// The number measures to constitute a run (to smooth bootstrapping).
const PRECISION: usize = 100;

/// The number of key-values pair in the state tree.
const DEFAULT_TREE_ENTRIES: usize = 1_000;

/// Run micro-benchmarks for every CPU-intensive operation.
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let tree_entries = match args.len() {
        x if x > 1 => args[1].parse().unwrap_or(DEFAULT_TREE_ENTRIES),
        _ => DEFAULT_TREE_ENTRIES,
    };
    println!("Starting micro-benchmarks:");

    // Run all micro-benchmarks.
    create_notification(tree_entries);
    verify_notification(tree_entries);
    create_vote();
    verify_vote();
    aggregate_certificate();
    verify_certificate();
}

/// Run a single micro-benchmark.
/// The `setup` function is executed before starting the timer and produces all the parameters needed for the
/// benchmark. The `run` function is executed multiple times using the setup data (as references).
fn bench<Setup, Run, Data, Result>(id: &str, setup: Setup, run: Run)
where
    Setup: FnOnce() -> Data,
    Run: Fn(&Data) -> Result,
{
    // Get the setup parameters.
    let inputs = setup();

    // Run the function to benchmark a number of times.
    let mut data = Vec::new();
    for _ in 0..RUNS {
        let now = Instant::now();
        for _ in 0..PRECISION {
            let _result = run(&inputs);
        }
        let elapsed = now.elapsed().as_millis() as f64;
        data.push(elapsed / PRECISION as f64);
    }

    // Display the results to stdout.
    println!(
        "  {:>7.2} +/- {:<5.2} ms {:.>30}",
        mean(&data),
        standard_deviation(&data, None),
        id
    );
}

/// Benchmark the creation of a publish notification.
fn create_notification(tree_entries: usize) {
    struct Data(KeyPair);

    let setup = || {
        let (_, keypair) = keys().pop().unwrap();
        Data(keypair)
    };

    let run = |data: &Data| {
        let Data(keypair) = data;

        // TODO: Use a persistent storage rather than the in-memory storage below.
        let db = akd::storage::memory::AsyncInMemoryDatabase::new();

        let (_, root, proof) = block_on(proof_with_storage(tree_entries, db));
        PublishNotification::new(root, proof, 1, keypair)
    };

    bench("create notification", setup, run);
}

/// Benchmark the verification of a publish notification.
fn verify_notification(tree_entries: usize) {
    struct Data(PublishNotification, Committee, Root);

    let setup = || {
        let (_, keypair) = keys().pop().unwrap();
        let (_, root, proof) = block_on(proof(tree_entries));
        let notification = PublishNotification::new(root, proof, 1, &keypair);
        Data(notification, committee(0), Root::default())
    };

    let run = |data: &Data| {
        let Data(notification, committee, previous_root) = data;
        block_on(notification.verify(committee, previous_root))
    };

    bench("verify notification", setup, run);
}

/// Benchmark the creation of a publish vote.
fn create_vote() {
    struct Data(PublishNotification, KeyPair);

    let setup = || {
        let (_, keypair) = keys().pop().unwrap();
        Data(block_on(notification()), keypair)
    };

    let run = |data: &Data| {
        let Data(notification, keypair) = data;
        PublishVote::new(notification, keypair)
    };

    bench("create vote", setup, run);
}

/// Benchmark the verification of a publish vote.
fn verify_vote() {
    struct Data(PublishVote, Committee);

    let setup = || {
        let vote = block_on(votes()).pop().unwrap();
        Data(vote, committee(0))
    };

    let run = |data: &Data| {
        let Data(vote, committee) = data;
        vote.verify(committee)
    };

    bench("verify vote", setup, run);
}

/// Benchmark the aggregation of a quorum of votes into a certificate.
fn aggregate_certificate() {
    struct Data(PublishNotification, Vec<PublishVote>);

    let setup = || {
        let threshold = committee(0).quorum_threshold() as usize;
        let mut votes = block_on(votes());
        votes.truncate(threshold);
        Data(block_on(notification()), votes)
    };

    let run = |data: &Data| {
        let Data(notification, votes) = data;
        PublishCertificate {
            root: notification.root,
            sequence_number: notification.sequence_number,
            votes: votes
                .iter()
                .map(|x| (x.author, x.signature.clone()))
                .collect(),
        }
    };

    bench("aggregate certificate", setup, run);
}

/// Benchmark the verification of a certificate.
fn verify_certificate() {
    struct Data(PublishCertificate, Committee);

    let setup = || {
        let threshold = committee(0).quorum_threshold() as usize;
        let mut certificate = block_on(certificate());
        certificate.votes.truncate(threshold);
        Data(certificate, committee(0))
    };

    let run = |data: &Data| {
        let Data(certificate, committee) = data;
        certificate.verify(committee)
    };

    bench("verify certificate", setup, run);
}
