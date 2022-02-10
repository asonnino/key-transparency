use anyhow::{Context, Result};
use clap::{arg, crate_name, crate_version, App, AppSettings, Arg};
use config::{Committee, Import, PrivateConfig};
use idp::spawn_idp;
use storage::Storage;

/// The default maximum delay before sealing a batch (in ms).
const DEFAULT_MAX_BATCH_DELAY: u64 = 200;

#[tokio::main]
async fn main() -> Result<()> {
    // Read the cli parameters.
    let matches = App::new(crate_name!())
        .version(crate_version!())
        .about("The Key Transparency IdP.")
        .arg(Arg::new("verbose").multiple_occurrences(true).short('v'))
        .args(&[
            arg!(--keypair <FILE> "The path to the witness keypair"),
            arg!(--committee <FILE> "The path to the committee file"),
            arg!(--secure_storage <FILE> "The directory to hold the secure storage"),
            arg!(--sync_storage <FILE> "The directory to hold the sync storage"),
            arg!(--batch_size <INT> "The number of client update requests to batch into a proof"),
            arg!(--max_batch_delay [INT] "The maximum delay (ms) before sealing a batch"),
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
        .filter_module("idp", log_level)
        .filter_module("network", log_level)
        .init();

    // Parse the parameters.
    let private_config_file = matches.value_of("keypair").unwrap();
    let private_config =
        PrivateConfig::import(private_config_file).context("Failed to load keypair")?;

    let committee_file = matches.value_of("committee").unwrap();
    let committee = Committee::import(committee_file).context("Failed to load committee")?;

    let secure_storage_file = matches.value_of("secure_storage").unwrap();
    let secure_storage =
        Storage::new(secure_storage_file).context("Failed to create secure storage")?;

    let sync_storage_file = matches.value_of("sync_storage").unwrap();
    let sync_storage = Storage::new(sync_storage_file).context("Failed to create sync storage")?;

    let batch_size = matches
        .value_of("batch_size")
        .unwrap()
        .parse::<usize>()
        .context("The batch size must be a non-negative integer")?;

    let max_batch_delay = match matches.value_of("max_batch_delay") {
        Some(x) => x
            .parse::<u64>()
            .context("The maximum batch delay must be a non-negative integer")?,
        None => DEFAULT_MAX_BATCH_DELAY,
    };

    // Spawn the IdP.
    spawn_idp(
        /* keypair */ private_config.secret,
        committee,
        secure_storage,
        sync_storage,
        batch_size,
        max_batch_delay,
    )
    .await;

    // If the following statement is reached, all IdP tasks go out of scope.
    unreachable!();
}
