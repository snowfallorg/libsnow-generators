use std::fs;
use std::io::Write;

use anyhow::{Context, Result};
use aws_config::meta::region::RegionProviderChain;
use aws_config::BehaviorVersion;
use clap::{Parser, Subcommand};
use libsnow_generators::ddb::batch_put::batch_store_put;
use libsnow_generators::{
    ddb::nix::get_store,
    revisions::{get_revisions, update_markers},
    s3::db::create_db,
};
use log::*;

#[derive(Subcommand, Debug)]
enum Commands {
    S3 {
        #[arg(short, long)]
        /// Upload to S3
        upload: bool,
        #[arg(short, long, default_value = "libsnow")]
        /// S3 Bucket to upload to
        bucket: String,
    },
    Ddb {
        #[arg(short, long)]
        /// DynamoDB table to upload to
        table: String,
    }
}

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long, default_value = "./processed")]
    /// Directory where processed markers are stored
    processed: String,
    #[arg(short, long)]
    /// Verbose logging
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    if args.verbose {
        std::env::set_var("RUST_LOG", "libsnow_generator=debug");
    }

    pretty_env_logger::init();

    info!("Args: {:#?}", args);

    let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");
    let config = aws_config::defaults(BehaviorVersion::latest())
        .region(region_provider)
        .retry_config(aws_config::retry::RetryConfig::adaptive())
        .load()
        .await;

    let revision = get_revisions(&args.processed)
        .await
        .expect("Failed to get revisions");

    info!("Got revisions: {:#?}", revision);

    match args.command {
        Commands::S3 { upload, bucket } => {
            let client = aws_sdk_s3::Client::new(&config);
            for (i, (channel, revs)) in revision.iter().enumerate() {
                for (j, r) in revs.iter().enumerate() {
                    info!(
                        "Revision: {} ({}/{}) ({}/{})",
                        r,
                        j + 1,
                        revs.len(),
                        i + 1,
                        revision.len()
                    );
                    create_db(&client, channel, r, upload, &bucket).await?;
                }
            }
        }
        Commands::Ddb { table }=> {
            let client = aws_sdk_dynamodb::Client::new(&config);
            for (i, (channel, revs)) in revision.iter().enumerate() {
                for (j, r) in revs.iter().enumerate() {
                    info!(
                        "Revision: {} ({}/{}) ({}/{})",
                        r,
                        j + 1,
                        revs.len(),
                        i + 1,
                        revision.len()
                    );
                    let storeset = get_store(r.split('.').last().context("Failed to get revision")?).await;

                    // Read processed
                    let prevpaths =
                        fs::read_to_string(format!("{}/{}/store-paths", args.processed, channel))
                            .unwrap_or_default();
                    let paths = prevpaths.split("\n").collect::<Vec<&str>>();

                    // Write to processed
                    let mut file =
                        fs::File::create(format!("{}/{}/store-paths", args.processed, channel))?;

                    let new_storeset = storeset
                        .iter()
                        .filter(|(k, _v)| !paths.contains(&k.as_str()))
                        .map(|(k, v)| (k.to_string(), v.clone()))
                        .collect::<std::collections::HashMap<String, _>>();

                    debug!(
                        "Total paths: {}. New paths: {}",
                        paths.len(),
                        new_storeset.len()
                    );

                    batch_store_put(&client, &new_storeset, &table).await?;

                    file.write_all(
                        storeset
                            .keys()
                            .map(String::to_string)
                            .collect::<Vec<_>>()
                            .join("\n")
                            .as_bytes(),
                    )
                    .expect("Failed to write to store-paths file");
                }
            }
        }
    }

    let _ = update_markers(&args.processed, revision)?;

    Ok(())
}
