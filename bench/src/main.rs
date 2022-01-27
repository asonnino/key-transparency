use config::Committee;
use crypto::KeyPair;
use messages::publish::{
    Proof, PublishCertificate, PublishNotification, PublishVote, Root, SequenceNumber,
};
use statistical::{mean, standard_deviation};
use std::time::Instant;
use test_utils::{certificate, committee, keys, notification, votes};

/// The number of runs used to compute statistics.
const RUNS: usize = 100;

/// The number measures to constitute a run (to smooth bootstrapping).
const PRECISION: usize = 100;

/// Run micro-benchmarks for every CPU-intensive operation.
fn main() {
    println!("Starting micro-benchmarks:");

    // Run all micro-benchmarks.
    create_notification();
    verify_notification();
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
        "  {:.2} +/- {:.2} ms .......... {}",
        mean(&data),
        standard_deviation(&data, None),
        id
    );
}

/// Benchmark the creation of a publish notification.
fn create_notification() {
    let setup = || {
        let (_, identity_provider) = keys().pop().unwrap();
        identity_provider
    };

    let run = |identity_provider: &KeyPair| {
        PublishNotification::new(
            /* root */ Root::default(),
            /* proof */ Proof::default(),
            /* sequence_number */ SequenceNumber::default(),
            /* keypair */ identity_provider,
        )
    };

    bench("create notification", setup, run);
}

/// Benchmark the verification of a publish notification.
fn verify_notification() {
    struct Data(PublishNotification, Committee, Root);

    let setup = || Data(notification(), committee(0), Root::default());

    let run = |data: &Data| {
        let notification = &data.0;
        let committee = &data.1;
        let previous_root = &data.2;
        notification.verify(committee, previous_root)
    };

    bench("verify notification", setup, run);
}

/// Benchmark the creation of a publish vote.
fn create_vote() {
    struct Data(PublishNotification, KeyPair);

    let setup = || {
        let (_, keypair) = keys().pop().unwrap();
        Data(notification(), keypair)
    };

    let run = |data: &Data| {
        let notification = &data.0;
        let keypair = &data.1;
        PublishVote::new(notification, keypair)
    };

    bench("create vote", setup, run);
}

/// Benchmark the verification of a publish vote.
fn verify_vote() {
    struct Data(PublishVote, Committee);

    let setup = || {
        let vote = votes().pop().unwrap();
        Data(vote, committee(0))
    };

    let run = |data: &Data| {
        let vote = &data.0;
        let committee = &data.1;
        vote.verify(committee)
    };

    bench("verify vote", setup, run);
}

/// Benchmark the aggregation of a quorum of votes into a certificate.
fn aggregate_certificate() {
    struct Data(PublishNotification, Vec<PublishVote>);

    let setup = || {
        let threshold = committee(0).quorum_threshold() as usize;
        let mut votes = votes();
        votes.truncate(threshold);
        Data(notification(), votes)
    };

    let run = |data: &Data| {
        let notification = &data.0;
        let votes = &data.1;
        PublishCertificate {
            root: notification.root.clone(),
            sequence_number: notification.sequence_number,
            votes: votes
                .iter()
                .map(|x| (x.author, x.signature.clone()))
                .collect(),
        }
    };

    bench("verify certificate", setup, run);
}

/// Benchmark the verification of a certificate.
fn verify_certificate() {
    struct Data(PublishCertificate, Committee);

    let setup = || {
        let threshold = committee(0).quorum_threshold() as usize;
        let mut certificate = certificate();
        certificate.votes.truncate(threshold);
        Data(certificate, committee(0))
    };

    let run = |data: &Data| {
        let certificate = &data.0;
        let committee = &data.1;
        certificate.verify(committee)
    };

    bench("verify certificate", setup, run);
}
