use clap::Parser;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(short, long, default_value = "config.json")]
    pub config: PathBuf,

    #[arg(long)]
    pub redeem: bool,

    #[arg(long, requires = "redeem")]
    pub condition_id: Option<String>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub polymarket: PolymarketConfig,
    pub strategy: StrategyConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyConfig {
    pub price_limit: f64,
    pub shares: f64,
    pub place_order_before_mins: u64,
    pub check_interval_ms: u64,
    #[serde(default)]
    pub simulation_mode: bool,
    #[serde(default)]
    pub signal: SignalConfig,
    #[serde(default = "default_sell_opposite_above")]
    pub sell_opposite_above: f64,
    #[serde(default = "default_sell_opposite_time_remaining")]
    pub sell_opposite_time_remaining: u64,
    #[serde(default = "default_market_closure_check_interval_seconds")]
    pub market_closure_check_interval_seconds: u64,
    #[serde(default = "default_risk_aversion_gamma")]
    pub risk_aversion_gamma: f64,
    #[serde(default = "default_kelly_fraction_k")]
    pub kelly_fraction_k: f64,
    #[serde(default = "default_bankroll_usdc")]
    pub bankroll_usdc: f64,
    #[serde(default = "default_assets")]
    pub assets: Vec<String>,
    // Phase 10.2: Hard entry caps
    #[serde(default = "default_straddle_hard_cap")]
    pub straddle_hard_cap: f64,
    #[serde(default = "default_straddle_hard_cap")]
    pub straddle_second_leg_cap: f64,
    // Phase 10.3: Loser stop-loss
    #[serde(default = "default_loser_stop_loss_price")]
    pub loser_stop_loss_price: f64,
    // Phase 10.5: Per-asset fill probability estimates
    #[serde(default = "default_fill_probability_btc")]
    pub fill_probability_btc: f64,
    #[serde(default = "default_fill_probability_eth")]
    pub fill_probability_eth: f64,
    #[serde(default = "default_fill_probability_sol")]
    pub fill_probability_sol: f64,
    #[serde(default = "default_fill_probability_xrp")]
    pub fill_probability_xrp: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SignalConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_stable_min")]
    pub stable_min: f64,
    #[serde(default = "default_stable_max")]
    pub stable_max: f64,
    #[serde(default = "default_clear_threshold")]
    pub clear_threshold: f64,
    #[serde(default = "default_clear_remaining_mins")]
    pub clear_remaining_mins: u64,
    #[serde(default = "default_danger_price")]
    pub danger_price: f64,
    #[serde(default = "default_danger_time_passed")]
    pub danger_time_passed: u64,
    #[serde(default = "default_one_side_buy_risk_management")]
    pub one_side_buy_risk_management: String,
    // Phase 10.1: mid-market re-entry (disabled by default — too expensive mid-period)
    #[serde(default)]
    pub mid_market_enabled: bool,
    // Phase 10.1: BTC correlation directional trigger (disabled by default)
    #[serde(default)]
    pub btc_correlation_enabled: bool,
    #[serde(default = "default_btc_correlation_threshold")]
    pub btc_correlation_threshold: f64,
    #[serde(default = "default_btc_correlation_min_straddle_cost")]
    pub btc_correlation_min_straddle_cost: f64,
}

fn default_true() -> bool { true }
fn default_stable_min() -> f64 { 0.20 }
fn default_stable_max() -> f64 { 0.80 }
fn default_clear_threshold() -> f64 { 0.99 }
fn default_clear_remaining_mins() -> u64 { 15 }
// Phase 10.4: raised from 0.15 to 0.28 — react before full collapse
fn default_danger_price() -> f64 { 0.28 }
// Phase 10.4: lowered from 30 to 15 — one-side timeout faster
fn default_danger_time_passed() -> u64 { 15 }
fn default_one_side_buy_risk_management() -> String { "price".to_string() }
// Phase 10.3: lowered from 0.95 to 0.70 — sell loser when winner has direction
fn default_sell_opposite_above() -> f64 { 0.70 }
// Phase 10.3: 10 minutes remaining (down from 15)
fn default_sell_opposite_time_remaining() -> u64 { 10 }
fn default_market_closure_check_interval_seconds() -> u64 { 120 }
fn default_risk_aversion_gamma() -> f64 { 0.001 }
fn default_kelly_fraction_k() -> f64 { 0.25 }
fn default_bankroll_usdc() -> f64 { 500.0 }
fn default_assets() -> Vec<String> { 
    vec!["BTC".to_string(), "ETH".to_string(), "SOL".to_string(), "XRP".to_string()] 
}
// Phase 10.1: BTC correlation defaults
fn default_btc_correlation_threshold() -> f64 { 0.003 }
fn default_btc_correlation_min_straddle_cost() -> f64 { 0.94 }
// Phase 10.2: straddle hard caps
fn default_straddle_hard_cap() -> f64 { 0.94 }
// Phase 10.3: loser leg stop-loss price
fn default_loser_stop_loss_price() -> f64 { 0.25 }
// Phase 10.5: per-asset fill probability estimates
fn default_fill_probability_btc() -> f64 { 0.88 }
fn default_fill_probability_eth() -> f64 { 0.85 }
fn default_fill_probability_sol() -> f64 { 0.78 }
fn default_fill_probability_xrp() -> f64 { 0.75 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolymarketConfig {
    pub gamma_api_url: String,
    pub clob_api_url: String,
    pub api_key: Option<String>,
    pub api_secret: Option<String>,
    pub api_passphrase: Option<String>,
    pub private_key: Option<String>,
    pub proxy_wallet_address: Option<String>,
    pub signature_type: Option<u8>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            polymarket: PolymarketConfig {
                gamma_api_url: "https://gamma-api.polymarket.com".to_string(),
                clob_api_url: "https://clob.polymarket.com".to_string(),
                api_key: None,
                api_secret: None,
                api_passphrase: None,
                private_key: None,
                proxy_wallet_address: None,
                signature_type: None,
            },
            strategy: StrategyConfig {
                price_limit: 0.45,
                shares: 5.0,
                place_order_before_mins: 3,
                check_interval_ms: 2000,
                simulation_mode: false,
                signal: SignalConfig::default(),
                sell_opposite_above: 0.70,
                sell_opposite_time_remaining: 10,
                market_closure_check_interval_seconds: 120,
                risk_aversion_gamma: 0.001,
                kelly_fraction_k: 0.25,
                bankroll_usdc: 500.0,
                assets: vec!["BTC".to_string(), "ETH".to_string(), "SOL".to_string(), "XRP".to_string()],
                straddle_hard_cap: 0.94,
                straddle_second_leg_cap: 0.94,
                loser_stop_loss_price: 0.25,
                fill_probability_btc: 0.88,
                fill_probability_eth: 0.85,
                fill_probability_sol: 0.78,
                fill_probability_xrp: 0.75,
            },
        }
    }
}

impl Config {
    pub fn load(path: &PathBuf) -> anyhow::Result<Self> {
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            Ok(serde_json::from_str(&content)?)
        } else {
            let config = Config::default();
            let content = serde_json::to_string_pretty(&config)?;
            std::fs::write(path, content)?;
            Ok(config)
        }
    }
}
