#![feature(const_trait_impl)]

use std::path::PathBuf;
use anyhow::{Context, Result};
use structopt::StructOpt;
use tracing::level_filters::LevelFilter;
use crate::config::Config;

use crate::database::Database;

mod config;
mod database;
mod ldap;
mod radius;

#[derive(Debug, StructOpt)]
#[structopt(name = "concess", about = "A super simple concession provider")]
pub struct Opt {
    #[structopt(short, long, parse(from_occurrences))]
    pub verbose: u32,

    #[structopt(short, long, default_value("concess.yaml"))]
    pub config: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    let opt = Opt::from_args();

    tracing_subscriber::fmt()
        .with_max_level(match opt.verbose {
            0 => LevelFilter::WARN,
            1 => LevelFilter::INFO,
            2 => LevelFilter::DEBUG,
            _ => LevelFilter::TRACE,
        })
        .init();

    let config = Config::load(&opt.config).await
        .with_context(|| format!("Failed to load config: {:?}", &opt.config))?;

    let database = Database::load(&config.data).await
        .with_context(|| format!("Failed to load database: {:?}", config.data))?;

    let ldap = ldap::serve(config.ldap, database.clone(), tokio::signal::ctrl_c());
    
    let radius = radius::serve(config.radius, database.clone(), tokio::signal::ctrl_c());

    tokio::try_join!(ldap, radius)?;

    return Ok(());
}
