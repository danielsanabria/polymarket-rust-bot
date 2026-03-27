use crate::api::PolymarketApi;
use crate::config::Config;
use crate::models::*;
use crate::signals::{self, MarketSignal};
use crate::discovery::MarketDiscovery;
use crate::strategy::state::*;
use crate::oracle::BinanceOracle;
use crate::hedger::HyperliquidHedger;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
use log::{info, debug, warn, error};

pub struct RiskManager {
    api: Arc<PolymarketApi>,
    config: Config,
    oracle: Arc<BinanceOracle>,
    hedger: Arc<HyperliquidHedger>,
    bankroll: Arc<Mutex<f64>>,
}

impl RiskManager {
    pub fn new(api: Arc<PolymarketApi>, config: Config, oracle: Arc<BinanceOracle>, hedger: Arc<HyperliquidHedger>) -> Self {
        let initial_bankroll = config.strategy.bankroll_usdc;
        Self { 
            api, 
            config, 
            oracle, 
            hedger, 
            bankroll: Arc::new(Mutex::new(initial_bankroll)) 
        }
    }

    pub async fn check_order_matches(&self, state: &mut PreLimitOrderState) -> Result<()> {
        let current_time_et = crate::strategy::get_current_time_et();
        
        if current_time_et < state.market_period_start {
            debug!("Market {} for {} hasn't started yet (current: {}, start: {})", 
                state.market_period_start, state.asset, current_time_et, state.market_period_start);
            return Ok(());
        }

        if !self.config.strategy.simulation_mode {
            if let (Some(up_id), Some(down_id)) = (&state.up_order_id, &state.down_order_id) {
                if !up_id.starts_with("SIM-") && !down_id.starts_with("SIM-") {
                    match self.api.are_both_orders_filled(up_id, down_id).await {
                        Ok((up_filled, down_filled)) => {
                            if up_filled && !state.up_matched {
                                info!("✅ Up order filled for {} (verified via API)", state.asset);
                                state.up_matched = true;
                                state.up_shares = state.up_order_shares;
                            }
                            if down_filled && !state.down_matched {
                                info!("✅ Down order filled for {} (verified via API)", state.asset);
                                state.down_matched = true;
                                state.down_shares = state.down_order_shares;
                            }
                            return Ok(());
                        }
                        Err(e) => {
                            debug!("{}: API fill check failed ({}), falling back to price inference", state.asset, e);
                        }
                    }
                }
            }
        }

        let up_price_result = self.api.get_price(&state.up_token_id, "SELL").await;
        let down_price_result = self.api.get_price(&state.down_token_id, "SELL").await;
        
        if let Ok(up_price) = up_price_result {
            let up_price_f64: f64 = up_price.to_string().parse().unwrap_or(0.0);
            let limit = state.up_order_price;
            if (up_price_f64 <= limit || (up_price_f64 - limit).abs() < 0.001) && !state.up_matched {
                if self.config.strategy.simulation_mode {
                    info!("SIMULATION: Up order matched for {} (hit ${:.4} <= ${:.2})", 
                        state.asset, up_price_f64, limit);
                } else {
                    info!("MATCHED: Up order for {} (hit ${:.4} <= ${:.2})", 
                        state.asset, up_price_f64, limit);
                }
                state.up_matched = true;
                state.up_shares = state.up_order_shares;
                self.log_trade(&state.asset, "BUY_UP", state.up_shares, up_price_f64, 0.0).await;
            }
        }
        
        if let Ok(down_price) = down_price_result {
            let down_price_f64: f64 = down_price.to_string().parse().unwrap_or(0.0);
            let limit = state.down_order_price;
            let price_matches = down_price_f64 <= limit || (down_price_f64 - limit).abs() < 0.001;
            if price_matches && !state.down_matched {
                if self.config.strategy.simulation_mode {
                    info!("SIMULATION: Down order matched for {} (hit ${:.2} <= ${:.2})", 
                        state.asset, down_price_f64, limit);
                } else {
                    info!("MATCHED: Down order for {} (hit ${:.2} <= ${:.2})", 
                        state.asset, down_price_f64, limit);
                }
                state.down_matched = true;
                state.down_shares = state.down_order_shares;
                self.log_trade(&state.asset, "BUY_DOWN", state.down_shares, down_price_f64, 0.0).await;
            }
        }

        // Phase 2.2: Oracle Safety Check (Kill Switch)
        self.check_oracle_safety(state).await?;

        Ok(())
    }

    pub async fn check_oracle_safety(&self, state: &mut PreLimitOrderState) -> Result<()> {
        if state.risk_sold || state.merged {
            let total_shares = state.up_shares + state.down_shares;
            if total_shares > 0.0 {
                self.hedger.close_hedge_order(&state.asset, total_shares, "BOTH").await?;
            }
            return Ok(());
        }

        let only_one_matched = (state.up_matched && !state.down_matched) || (state.down_matched && !state.up_matched);

        if let Some(binance_price) = self.oracle.get_price(&state.asset).await {
            if let Some(price_at_placement) = state.binance_price_at_placement {
                let delta_pct = (binance_price - price_at_placement) / price_at_placement;
                let threshold = 0.005; 

                if !state.up_matched && delta_pct < -threshold {
                    if let Some(order_id) = &state.up_order_id {
                        warn!("🚨 TOXIC LIQUIDITY DETECTED ({}): Binance price dropped {:.2}%. Killing 'Up' order.", 
                            state.asset, delta_pct * 100.0);
                        if !self.config.strategy.simulation_mode {
                            let _ = self.api.cancel_order(order_id).await;
                        }
                        state.up_order_id = None; 
                    }
                }

                if !state.down_matched && delta_pct > threshold {
                    if let Some(order_id) = &state.down_order_id {
                        warn!("🚨 TOXIC LIQUIDITY DETECTED ({}): Binance price rose {:.2}%. Killing 'Down' order.", 
                            state.asset, delta_pct * 100.0);
                        if !self.config.strategy.simulation_mode {
                            let _ = self.api.cancel_order(order_id).await;
                        }
                        state.down_order_id = None;
                    }
                }
            }
        }

        if state.up_matched && !state.down_matched && state.down_order_id.is_some() {
            if !state.up_hedged {
                self.hedger.place_hedge_order(&state.asset, state.up_shares, "SHORT").await?;
                state.up_hedged = true;
            }
        } else if state.down_matched && !state.up_matched && state.up_order_id.is_some() {
            if !state.down_hedged {
                self.hedger.place_hedge_order(&state.asset, state.down_shares, "LONG").await?;
                state.down_hedged = true;
            }
        } else if state.up_matched && state.down_matched {
            if !state.both_hedged {
                self.hedger.close_hedge_order(&state.asset, state.up_shares + state.down_shares, "BOTH").await?;
                state.both_hedged = true;
            }
        }

        Ok(())
    }

    pub fn calculate_inventory_penalty(&self, up_shares: f64, down_shares: f64) -> f64 {
        let q = up_shares - down_shares;
        let delta = self.config.strategy.risk_aversion_gamma * q;
        delta
    }

    pub async fn calculate_kelly_size(&self, p: f64, c: f64) -> f64 {
        if c <= 0.0 || c >= 1.0 { return 0.0; }
        let b = (1.0 - c) / c;
        let f_star = (p * (b + 1.0) - 1.0) / b;
        
        if f_star <= 0.0 {
            return 0.0;
        }

        let f_final = f_star * self.config.strategy.kelly_fraction_k;
        let bankroll = *self.bankroll.lock().await;
        let order_size_usdc = bankroll * f_final;
        
        debug!("🎯 Kelly Check: p={:.2}, c={:.2}, b={:.2} | f*={:.4}, f_final={:.4} | Bankroll: ${:.2} | Order: ${:.2}", 
            p, c, b, f_star, f_final, bankroll, order_size_usdc);
            
        order_size_usdc / c
    }

    pub async fn get_market_snapshot(&self, asset: &str, period_start: i64) -> Option<(f64, f64, i64)> {
        let slug = MarketDiscovery::build_15m_slug(asset, period_start);
        let market = self.api.get_market_by_slug(&slug).await.ok()?;
        if !market.active || market.closed {
            return None;
        }
        
        let discovery = MarketDiscovery::new(self.api.clone());
        let (up_token_id, down_token_id) = discovery.get_market_tokens(&market.condition_id).await.ok()?;
        
        let (up_res, down_res) = tokio::join!(
            self.api.get_price(&up_token_id, "SELL"),
            self.api.get_price(&down_token_id, "SELL")
        );
        let up_price = up_res.ok()?.to_string().parse::<f64>().ok()?;
        let down_price = down_res.ok()?.to_string().parse::<f64>().ok()?;
        let current_time_et = crate::strategy::get_current_time_et();
        let market_end = period_start + MARKET_DURATION_SECS;
        let time_remaining = market_end - current_time_et;
        Some((up_price, down_price, time_remaining.max(0)))
    }

    pub async fn get_place_signal(&self, asset: &str, period_start: i64) -> MarketSignal {
        let Some((up_price, down_price, time_remaining)) = self.get_market_snapshot(asset, period_start).await else {
            return MarketSignal::Unknown;
        };
        signals::evaluate_place_signal(
            &self.config.strategy.signal,
            up_price,
            down_price,
            time_remaining,
        )
    }

    pub fn is_danger_signal(&self, price: f64) -> bool {
        signals::is_danger_signal(&self.config.strategy.signal, price)
    }

    pub async fn log_trade(&self, asset: &str, action: &str, shares: f64, price: f64, pnl: f64) {
        // Update Dynamic Bankroll
        {
            let mut b = self.bankroll.lock().await;
            *b += pnl;
            if pnl != 0.0 {
                info!("💰 BANKROLL UPDATE: PnL ${:.2} | New Total: ${:.2}", pnl, *b);
            }
        }

        use std::fs::OpenOptions;
        use std::io::Write;
        let file_exists = std::path::Path::new("trades.csv").exists();
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("trades.csv") {
            if !file_exists {
                let _ = writeln!(file, "timestamp,asset,action,shares,price,pnl");
            }
            let ts = chrono::Utc::now().to_rfc3339();
            let _ = writeln!(file, "{},{},{},{:.4},{:.4},{:.4}", ts, asset, action, shares, price, pnl);
        }
    }
}
