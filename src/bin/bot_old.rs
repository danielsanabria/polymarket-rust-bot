#[path = "../api.rs"] mod api;
#[path = "../config.rs"] mod config;
#[path = "../models.rs"] mod models;
#[path = "../discovery.rs"] mod discovery;
#[path = "../signals.rs"] mod signals;
#[path = "../strategy_old.rs"] mod strategy_old;

use anyhow::Result;
use clap::Parser;
use config::{Args, Config};
use std::io::Write;
use std::sync::Arc;
use api::PolymarketApi;
use strategy_old::PreLimitStrategy;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .format(|buf, record| {
            writeln!(buf, "{}", record.args())
        })
        .init();

    let args = Args::parse();
    let config = Config::load(&args.config)?;
    
    let api = Arc::new(PolymarketApi::new(
        config.polymarket.gamma_api_url.clone(),
        config.polymarket.clob_api_url.clone(),
        config.polymarket.api_key.clone(),
        config.polymarket.api_secret.clone(),
        config.polymarket.api_passphrase.clone(),
        config.polymarket.private_key.clone(),
        config.polymarket.proxy_wallet_address.clone(),
        config.polymarket.signature_type,
    ));

    eprintln!("🚀 Starting OLD Polymarket Pre-Limit Order Bot");
    let strategy = Arc::new(PreLimitStrategy::new(api, config));
    strategy.run().await
}
