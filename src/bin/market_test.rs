use crate::api::PolymarketApi;
use crate::config::Config;
use anyhow::Result;
use std::sync::Arc;

#[path = "../api.rs"] mod api;
#[path = "../config.rs"] mod config;
#[path = "../models.rs"] mod models;

#[tokio::main]
async fn main() -> Result<()> {
    let config = config::Config::load(&std::path::PathBuf::from("config.json"))?;
    let api = PolymarketApi::new(
        config.polymarket.gamma_api_url.clone(),
        config.polymarket.clob_api_url.clone(),
        config.polymarket.api_key.clone(),
        config.polymarket.api_secret.clone(),
        config.polymarket.api_passphrase.clone(),
        config.polymarket.private_key.clone(),
        config.polymarket.proxy_wallet_address.clone(),
        config.polymarket.signature_type,
    );

    println!("Searching for active 15m markets...");
    // We can't easily list ALL markets filterable by slug without a large query,
    // but we can try to find them by searching for "15m" in the Gamma API.
    // Actually, let's just try to fetch a few potential slugs.
    
    let current_period = 1774512900; // 08:15 UTC
    let next_period = current_period + 900; // 08:30 UTC
    
    let assets = vec!["bitcoin", "ethereum", "solana", "xrp"];
    for asset in assets {
        let slug = format!("{}-updown-15m-{}", asset, current_period);
        match api.get_market_by_slug(&slug).await {
            Ok(m) => println!("Found Market: {} | Active: {} | Closed: {}", m.question, m.active, m.closed),
            Err(e) => println!("Slug {} not found: {}", slug, e),
        }
        
        let slug_next = format!("{}-updown-15m-{}", asset, next_period);
        match api.get_market_by_slug(&slug_next).await {
            Ok(m) => println!("Found Market: {} | Active: {} | Closed: {}", m.question, m.active, m.closed),
            Err(_) => (),
        }
    }

    Ok(())
}
