pub mod state;
pub mod risk;
pub mod processor;

use crate::api::PolymarketApi;
use crate::config::Config;
use crate::discovery::MarketDiscovery;
use crate::strategy::state::*;
use crate::strategy::processor::MarketProcessor;
use crate::strategy::risk::RiskManager;
use crate::oracle::BinanceOracle;
use crate::hedger::HyperliquidHedger;
use crate::ai::{SharedAiState, SharedAiContext};
use anyhow::Result;
use chrono::Utc;
use chrono_tz::America::New_York;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use log::{info, warn, error};

pub struct PreLimitStrategy {
    api: Arc<PolymarketApi>,
    config: Config,
    oracle: Arc<BinanceOracle>,
    hedger: Arc<HyperliquidHedger>,
    processor: MarketProcessor,
    states: Arc<Mutex<HashMap<String, PreLimitOrderState>>>,
    last_status_display: Arc<Mutex<std::time::Instant>>,
    total_profit: Arc<Mutex<f64>>,
    trades: Arc<Mutex<HashMap<String, CycleTrade>>>,
    closure_checked: Arc<Mutex<HashMap<String, bool>>>,
    period_profit: Arc<Mutex<f64>>,
    ai_state: SharedAiState,
    ai_context: SharedAiContext,
}

impl PreLimitStrategy {
    pub fn new(api: Arc<PolymarketApi>, config: Config, oracle: Arc<BinanceOracle>, hedger: Arc<HyperliquidHedger>, ai_state: SharedAiState, ai_context: SharedAiContext) -> Self {
        let processor = MarketProcessor::new(api.clone(), config.clone(), oracle.clone(), hedger.clone(), ai_state.clone(), ai_context.clone());
        Self {
            api,
            config,
            oracle,
            hedger,
            processor,
            states: Arc::new(Mutex::new(HashMap::new())),
            last_status_display: Arc::new(Mutex::new(std::time::Instant::now())),
            total_profit: Arc::new(Mutex::new(0.0)),
            trades: Arc::new(Mutex::new(HashMap::new())),
            closure_checked: Arc::new(Mutex::new(HashMap::new())),
            period_profit: Arc::new(Mutex::new(0.0)),
            ai_state,
            ai_context,
        }
    }

    pub async fn get_total_profit(&self) -> f64 {
        *self.total_profit.lock().await
    }

    pub async fn get_period_profit(&self) -> f64 {
        *self.period_profit.lock().await
    }

    pub async fn run(&self) -> Result<()> {
        let _ = self.display_market_status().await;
        loop {
            let should_display = {
                let mut last = self.last_status_display.lock().await;
                if last.elapsed().as_secs() >= 10 {
                    *last = std::time::Instant::now();
                    true
                } else {
                    false
                }
            };
            
            if should_display {
                let _ = self.display_market_status().await;
            }
            
            if let Err(e) = self.process_markets().await {
                error!("Error processing markets: {}", e);
            }
            sleep(Duration::from_millis(self.config.strategy.check_interval_ms)).await;
        }
    }

    async fn process_markets(&self) -> Result<()> {
        let assets = self.config.strategy.assets.clone();
        let current_period_et = get_current_15m_period_et();
        for asset in assets {
            self.processor.process_asset(
                &asset, 
                current_period_et, 
                self.states.clone(), 
                self.trades.clone(), 
                self.total_profit.clone()
            ).await?;
        }
        Ok(())
    }

    pub async fn check_market_closure(&self) -> Result<()> {
        let trades: Vec<(String, CycleTrade)> = {
            let t = self.trades.lock().await;
            t.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        };
        if trades.is_empty() { return Ok(()); }
        
        let current_time = Utc::now().timestamp() as u64;

        for (market_key, trade) in trades {
            if current_time < trade.period_timestamp + trade.market_duration_secs { continue; }

            {
                let checked = self.closure_checked.lock().await;
                if checked.get(&trade.condition_id).copied().unwrap_or(false) { continue; }
            }

            let market = match self.api.get_market(&trade.condition_id).await {
                Ok(m) => m,
                Err(_) => continue,
            };
            if !market.closed { continue; }

            let up_wins = trade.up_token_id.as_ref().map(|id| market.tokens.iter().any(|t| t.token_id == *id && t.winner)).unwrap_or(false);
            let down_wins = trade.down_token_id.as_ref().map(|id| market.tokens.iter().any(|t| t.token_id == *id && t.winner)).unwrap_or(false);

            let total_cost = (trade.up_shares * trade.up_avg_price) + (trade.down_shares * trade.down_avg_price);
            let payout = if up_wins { trade.up_shares } else if down_wins { trade.down_shares } else { 0.0 };
            let pnl = payout - total_cost;

            let winner = if up_wins { "Up" } else if down_wins { "Down" } else { "Unknown" };
            let sim_prefix = if self.config.strategy.simulation_mode { "SIMULATION: " } else { "" };
            info!("------------------------------------------------");
            info!("{sim_prefix}Market closed | Winner: {winner} | PnL ${pnl:.2}");

            if !self.config.strategy.simulation_mode && (up_wins || down_wins) {
                let token_id = if up_wins { trade.up_token_id.as_deref().unwrap_or("") } else { trade.down_token_id.as_deref().unwrap_or("") };
                let winner_side = if up_wins { "Up" } else { "Down" };
                let _ = self.api.redeem_tokens(&trade.condition_id, token_id, winner_side).await;
            }

            *self.total_profit.lock().await += pnl;
            *self.period_profit.lock().await += pnl;
            
            self.closure_checked.lock().await.insert(trade.condition_id.clone(), true);
            self.trades.lock().await.remove(&market_key);
        }
        Ok(())
    }

    async fn display_market_status(&self) -> Result<()> {
        let assets = self.config.strategy.assets.clone();
        let current_time_et = get_current_time_et();
        
        let total_profit = {
            let total = self.total_profit.lock().await;
            *total
        };
        
        info!("--- MARKET STATUS | Profit: ${:.2} ---", total_profit);
        
        let mut states = self.states.lock().await;
        let mut states_to_check: Vec<String> = Vec::new();
        
        for asset in &assets {
            if let Some(state) = states.get_mut(asset) {
                let market_period = state.market_period_start;
                
                match self.processor.discover_next_market(asset, market_period).await {
                    Ok(Some(market)) => {
                        let up_price_result = self.api.get_price(&state.up_token_id, "SELL").await;
                        let down_price_result = self.api.get_price(&state.down_token_id, "SELL").await;
                        
                        let market_end = market_period + MARKET_DURATION_SECS;
                        let time_remaining = market_end - current_time_et;
                        let minutes = if time_remaining > 0 { time_remaining / 60 } else { 0 };
                        let seconds = if time_remaining > 0 { time_remaining % 60 } else { 0 };

                        let up_price_str = match up_price_result {
                            Ok(p) => format!("${:.2}", p),
                            Err(_) => "N/A".to_string(),
                        };
                        let down_price_str = match down_price_result {
                            Ok(p) => format!("${:.2}", p),
                            Err(_) => "N/A".to_string(),
                        };
                        
                        let order_status = format!("Up:{} Down:{}", 
                            if state.up_matched { "OK" } else { "WAIT" },
                            if state.down_matched { "OK" } else { "WAIT" });
                        
                        info!("{} | U:{} D:{} | {}m{}s | {} | M:{}", 
                            asset, up_price_str, down_price_str, minutes, seconds, order_status, market_period);
                    }
                    _ => {
                        info!("{} | Inactive | Up:{} Down:{}", 
                            asset,
                            if state.up_matched { "OK" } else { "WAIT" },
                            if state.down_matched { "OK" } else { "WAIT" });
                    }
                }
            } else {
                let current_period_et = get_current_15m_period_et();
                let next_period_start = current_period_et + MARKET_DURATION_SECS;
                let next_period_end = next_period_start + MARKET_DURATION_SECS;
                let secs_elapsed = current_time_et - current_period_et;
                let secs_until_next = next_period_start - current_time_et;
                let order_window_secs = (self.config.strategy.place_order_before_mins * 60) as i64;
                let secs_until_order_window = secs_until_next - order_window_secs;
                
                if secs_until_order_window > 0 {
                    info!("{} | Idle | Period: {}m{}s elapsed | Order window in: {}m{}s",
                        asset,
                        secs_elapsed / 60, secs_elapsed % 60,
                        secs_until_order_window / 60, secs_until_order_window % 60);
                } else if secs_until_next > 0 {
                    info!("{} | 🕒 IN ORDER WINDOW | {}s until next period starts", asset, secs_until_next);
                } else {
                    info!("{} | Waiting for next period ({}) | {}s ago", asset, next_period_start, -secs_until_next);
                }
            }
        }
        
        drop(states);
        info!("------------------------------------------------------------");
        Ok(())
    }
}

pub fn get_current_15m_period_et() -> i64 {
    MarketDiscovery::current_15m_period_start_et()
}

pub fn get_current_time_et() -> i64 {
    Utc::now().with_timezone(&New_York).timestamp()
}
