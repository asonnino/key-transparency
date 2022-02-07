use anyhow::{Context, Result};
use clap::{arg, crate_name, crate_version, App, AppSettings};
use config::{Committee, Import, PrivateConfig};
use env_logger::Env;
use storage::Storage;
use witness::spawn_witness;

#[tokio::main]
async fn main() -> Result<()> {
    // Read the cli parameters.
    let matches = App::new(crate_name!())
        .version(crate_version!())
        .about("Benchmark client for Key Transparency witnesses.")
        .args(&[
            arg!(-v... "Sets the level of verbosity"),
            arg!(--committee <FILE> "The path to the committee file"),
            arg!(--keypair <FILE> "The path to the witness keypair"),
            arg!(--secure_storage <FILE> "The directory to hold the secure storage"),
            arg!(--audit_storage <FILE> "The directory to hold the audit storage"),
        ])
        .setting(AppSettings::ArgRequiredElseHelp)
        .get_matches();

    // Configure the logger.
    let log_level = match matches.occurrences_of("v") {
        0 => "error",
        1 => "warn",
        2 => "info",
        3 => "debug",
        _ => "trace",
    };
    env_logger::Builder::from_env(Env::default().default_filter_or(log_level))
        .format_timestamp_millis()
        .init();

    // Parse the input parameters.
    let committee_file = matches.value_of("committee").unwrap();
    let committee = Committee::import(committee_file).context("Failed to load committee")?;

    let keypair_file = matches.value_of("keypair").unwrap();
    let keypair = PrivateConfig::import(keypair_file).context("Failed to load keypair")?;

    let secure_storage_file = matches.value_of("secure_storage").unwrap();
    let secure_storage =
        Storage::new(secure_storage_file).context("Failed to create secure storage")?;

    let audit_storage_file = matches.value_of("audit_storage").unwrap();
    let audit_storage =
        Storage::new(audit_storage_file).context("Failed to create audit storage")?;

    // Spawn a witness.
    spawn_witness(keypair.secret, committee, secure_storage, audit_storage);

    // TODO: better way to prevent the program from exiting....
    loop {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }
}
