use anyhow::Result;
use clap::Parser;
use simple_logging::log_to_file;
use std::path::{Path, PathBuf};
use time::Duration;
mod responder;
mod rest;
mod watcher;

// A command line argument parser for the TTP Watcher
#[derive(clap::Parser, Debug)]
#[command(name="Trusted Traveler Program Monitoring",author, version, about, long_about = None)]
struct Args {
    /// The location to monitor. Must be a valid string contained in a location's name
    #[arg(short, long)]
    location: String,

    /// Set to use email alerting
    #[arg(short, long)]
    email: Option<String>,

    /// The polling period in seconds
    #[arg(value_parser = |arg: &str| -> Result<Duration> {Ok(Duration::seconds(arg.parse()?))})]
    #[arg(long, default_value = "30")]
    poll_period: Duration,

    /// The path to the sendgrid secret
    #[arg(long, default_value=Path::new("sendgrid.secret").to_path_buf().into_os_string())]
    sendgrid_config_path: PathBuf,

    /// The path to the location cache. This should be a json file containing a list of locations returned from the TTP API.
    #[arg(long)]
    location_cache_path: Option<PathBuf>,

    /// The path to the log file
    #[arg(long, default_value=Path::new("debug.log").to_path_buf().into_os_string())]
    log_path: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Set up watcher. If a location cache is provided, load it. Otherwise, fetch from the API.
    let poll_period = args.poll_period;
    let watcher = {
        if let Some(location_cache) = args.location_cache_path {
            let locations = watcher::load_locations_from_file(location_cache)?;
            watcher::Watcher::new(poll_period, locations)?
        } else {
            watcher::Watcher::new_no_cache(poll_period).await?
        }
    };

    // Setup async tasks
    let mut set = tokio::task::JoinSet::new();

    // Register file logging backend
    {
        log_to_file(args.log_path, log::LevelFilter::Info)?;
        let mut logging_responder = responder::logging::Logging::new(watcher.get_receiver())?;
        set.spawn(async move { logging_responder.run().await });
    }

    // Register email alerting backend
    if let Some(email) = args.email {
        let email_config = responder::email::load_email_data(args.sendgrid_config_path)?;

        let mut responder = responder::email::Email::new(watcher.get_receiver(), email_config)?;
        set.spawn(async move { responder.alert_on_availability(&email).await });
    }

    let location_string = args.location;
    set.spawn(async move { watcher.watch(location_string.as_str()).await });

    set.join_next().await;

    Ok(())
}
