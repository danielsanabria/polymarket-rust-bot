use crate::api::PolymarketApi;
use crate::config::Config;
use crate::discovery::MarketDiscovery;
use crate::models::*;
use crate::strategy::state::*;
use crate::strategy::risk::RiskManager;
use crate::oracle::BinanceOracle;
use crate::hedger::HyperliquidHedger;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use log::{info, warn, error, debug};

pub struct MarketProcessor {
    api: Arc<PolymarketApi>,
    config: Config,
    oracle: Arc<BinanceOracle>,
    hedger: Arc<HyperliquidHedger>,
    discovery: MarketDiscovery,
    risk: RiskManager,
}

impl MarketProcessor {
    pub fn new(api: Arc<PolymarketApi>, config: Config, oracle: Arc<BinanceOracle>, hedger: Arc<HyperliquidHedger>) -> Self {
        let discovery = MarketDiscovery::new(api.clone());
        let risk = RiskManager::new(api.clone(), config.clone(), oracle.clone(), hedger.clone());
        Self { api, config, oracle, hedger, discovery, risk }
    }

    pub async fn process_asset(
        &self, 
        asset: &str, 
        current_period_et: i64,
        states: Arc<Mutex<HashMap<String, PreLimitOrderState>>>,
        trades: Arc<Mutex<HashMap<String, CycleTrade>>>,
        total_profit: Arc<Mutex<f64>>,
    ) -> Result<()> {
        let mut states_guard = states.lock().await;
        let state = states_guard.get(asset).cloned();
        
        let current_time_et = crate::strategy::get_current_time_et();
        let next_period_start = current_period_et + MARKET_DURATION_SECS;
        let time_until_next = next_period_start - current_time_et;

        let needs_danger_handling = state.as_ref().map_or(false, |s| {
            !s.merged && !s.risk_sold && s.status != CycleStatus::WaitingForNextCycle &&
            ((s.up_matched && !s.down_matched) || (s.down_matched && !s.up_matched))
        });

        let seconds_elapsed = current_time_et - current_period_et; // Bug 1 fix: use direct diff
        if time_until_next <= (self.config.strategy.place_order_before_mins * 60) as i64 && seconds_elapsed >= 60 {
            let is_next_market_prepared = state.as_ref().map_or(false, |s| s.expiry == next_period_start + MARKET_DURATION_SECS);
            
            if !is_next_market_prepared && !needs_danger_handling {
                let signal = self.risk.get_place_signal(asset, current_period_et).await;
                if signal == crate::signals::MarketSignal::Good {
                    if let Some(next_market) = self.discover_next_market(asset, next_period_start).await? {
                        info!("PREPARING: {} market (starts in {}s)", asset, time_until_next);
                        let (up_token_id, down_token_id) = self.discovery.get_market_tokens(&next_market.condition_id).await?;

                        let up_shares_current = states_guard.values().filter(|s| s.asset == asset).map(|s| s.up_shares).sum::<f64>();
                        let down_shares_current = states_guard.values().filter(|s| s.asset == asset).map(|s| s.down_shares).sum::<f64>();
                        let delta = self.risk.calculate_inventory_penalty(up_shares_current, down_shares_current);
                        
                        let base_price = self.config.strategy.price_limit;
                        let up_price = self.round_price(base_price - delta);
                        let down_price = self.round_price(base_price + delta);

                        // Rule 2: Filtro de Coste Máximo (Spread Cap)
                        if up_price + down_price > 0.94 {
                            info!("{} | PROPOSED STRADDLE TOO EXPENSIVE: ${:.2} + ${:.2} = ${:.2} (> 0.94). Skipping quote.", 
                                asset, up_price, down_price, up_price + down_price);
                            return Ok(());
                        }

                        // Calculate Kelly Size
                        let p_up = 0.52; // Temporary bias for test
                        let p_down = 1.0 - p_up;
                        
                        let up_size_shares = self.risk.calculate_kelly_size(p_up, up_price).await;
                        let down_size_shares = self.risk.calculate_kelly_size(p_down, down_price).await;
                        
                        let shares_to_use = up_size_shares.max(down_size_shares).min(self.config.strategy.shares * 5.0).max(1.0);

                        debug!("{} | Skewing limits: Up=${:.2}, Down=${:.2} (delta={:.3}, q={:.1}) | Kelly Size: {:.1} shares", 
                            asset, up_price, down_price, delta, up_shares_current - down_shares_current, shares_to_use);

                        let up_order = self.place_limit_order(&up_token_id, "BUY", up_price, shares_to_use).await?;
                        let down_order = self.place_limit_order(&down_token_id, "BUY", down_price, shares_to_use).await?;
                        
                        let binance_price_at_placement = self.oracle.get_price(asset).await;

                        let new_state = PreLimitOrderState {
                            asset: asset.to_string(),
                            condition_id: next_market.condition_id,
                            up_token_id,
                            down_token_id,
                            up_order_id: up_order.order_id,
                            down_order_id: down_order.order_id,
                            up_order_price: up_price,
                            down_order_price: down_price,
                            up_matched: false,
                            down_matched: false,
                            merged: false,
                            expiry: next_period_start + MARKET_DURATION_SECS,
                            risk_sold: false,
                            order_placed_at: current_time_et,
                            market_period_start: next_period_start,
                            one_side_matched_at: None,
                            binance_price_at_placement,
                            up_order_shares: shares_to_use,
                            down_order_shares: shares_to_use,
                            up_shares: 0.0,
                            down_shares: 0.0,
                            up_hedged: false,
                            down_hedged: false,
                            both_hedged: false,
                            status: CycleStatus::AcceptingOrders,
                            winner_entry_price: None,
                        };
                        states_guard.insert(asset.to_string(), new_state);
                        return Ok(());
                    }
                } else if signal == crate::signals::MarketSignal::Bad {
                    debug!("{} | Bad signal for current market — skipping pre-orders for next 15m", asset);
                }
            }
        }

        if let Some(mut s) = state {
            let seconds_elapsed = current_time_et - current_period_et; // Bug 1 fix: direct diff

            // Rule 1: Sincronización Inicial y Límite de Entrada
            if seconds_elapsed > 720 && s.status == CycleStatus::AcceptingOrders && !s.up_matched && !s.down_matched {
                info!("{} | Min 12 reached without entry. Transitioning to WaitingForNextCycle.", asset);
                s.status = CycleStatus::WaitingForNextCycle;
                if let Some(id) = &s.up_order_id { let _ = self.api.cancel_order(id).await; }
                if let Some(id) = &s.down_order_id { let _ = self.api.cancel_order(id).await; }
            }

            // Bug 2 fix: Timeout medido desde que se colocaron las órdenes, no desde inicio del período
            let secs_since_placement = current_time_et - s.order_placed_at;
            if secs_since_placement > 180 && s.status == CycleStatus::AcceptingOrders && (!s.up_matched || !s.down_matched) {
                warn!("{} | Straddle not formed within 180s of placement. Canceling and transitioning to WaitingForNextCycle.", asset);
                s.status = CycleStatus::WaitingForNextCycle;
                if let Some(id) = &s.up_order_id { let _ = self.api.cancel_order(id).await; }
                if let Some(id) = &s.down_order_id { let _ = self.api.cancel_order(id).await; }
            }

            self.risk.check_order_matches(&mut s).await?;

            // Update status if straddle formed
            if s.up_matched && s.down_matched && s.status == CycleStatus::AcceptingOrders {
                s.status = CycleStatus::StraddleFormed;
            }
            
            // Phase 3.2: Correlation Check (only for ALTs)
            if asset != "BTC" && !s.up_matched && !s.down_matched && s.status == CycleStatus::AcceptingOrders {
                self.check_btc_correlation_trigger(asset, &mut s).await?;
            }

            if s.up_matched && s.down_matched && !s.merged {
                self.handle_both_matched(asset, &mut s, total_profit.clone(), trades.clone()).await?;
            }

            self.handle_one_side_risk(asset, &mut s, total_profit.clone()).await?;

            let current_time_et = crate::strategy::get_current_time_et();
            if current_time_et > s.expiry {
                if s.up_matched && s.down_matched && !s.risk_sold && !s.merged {
                    let trade = self.cycle_trade_holding_both(&s);
                    let mut t = trades.lock().await;
                    t.insert(s.condition_id.clone(), trade);
                }
                
                // Rule 6: Auditoría de EXPIRED (Log PnL Real 0 - entry_cost)
                let loss = s.winner_entry_price.map(|p| -p * s.up_order_shares).unwrap_or(0.0);
                info!("Market expired for {}. Clearing state. Loss: ${:.2}", asset, loss);
                self.risk.log_trade(asset, "EXPIRED", 0.0, 0.0, loss).await;
                states_guard.remove(asset);
            } else {
                states_guard.insert(asset.to_string(), s);
            }
        } else if time_until_next > (self.config.strategy.place_order_before_mins * 60) as i64
            && self.config.strategy.signal.mid_market_enabled
        {
            self.handle_mid_market_entry(asset, current_period_et, &mut states_guard).await?;
        }

        Ok(())
    }

    async fn handle_both_matched(
        &self,
        asset: &str,
        s: &mut PreLimitOrderState,
        total_profit: Arc<Mutex<f64>>,
        trades: Arc<Mutex<HashMap<String, CycleTrade>>>,
    ) -> Result<()> {
        let threshold = self.config.strategy.sell_opposite_above;
        let (up_price, down_price) = (
            self.api.get_price(&s.up_token_id, "SELL").await.ok()
                .and_then(|p| p.to_string().parse::<f64>().ok()).unwrap_or(0.0),
            self.api.get_price(&s.down_token_id, "SELL").await.ok()
                .and_then(|p| p.to_string().parse::<f64>().ok()).unwrap_or(0.0),
        );

        let current_time_et = crate::strategy::get_current_time_et();
        // Bug 4 fix: use time_remaining-based trigger instead of hardcoded 720-780s window
        let time_remaining_mins = (s.expiry - current_time_et) / 60;
        
        let sell_opposite = if up_price >= threshold {
            Some(("Up", "Down", &s.down_token_id, s.down_order_price))
        } else if down_price >= threshold {
            Some(("Down", "Up", &s.up_token_id, s.up_order_price))
        } else {
            None
        };

        if let Some((winner, loser, token_to_sell, purchase_price)) = sell_opposite {
            // Rule 5: Vender pierna perdedora cuando quedan <= sell_opposite_time_remaining mins
            // Bug B fix: guard with != ClosingLoser to avoid double-execution on consecutive ticks
            if time_remaining_mins <= self.config.strategy.sell_opposite_time_remaining as i64 
                && s.status != CycleStatus::ClosingLoser {
                info!("{}: Both filled, {} price ${:.2} >= {:.2} AND {}m remaining (trigger: {}m) — selling {} to reduce loss", 
                    asset, winner, if winner == "Up" { up_price } else { down_price }, threshold, 
                    time_remaining_mins, self.config.strategy.sell_opposite_time_remaining, loser);
                s.status = CycleStatus::ClosingLoser;
                s.winner_entry_price = if winner == "Up" { Some(s.up_order_price) } else { Some(s.down_order_price) };
                
                let sell_price = self.api.get_price(token_to_sell, "SELL").await.ok()
                    .and_then(|p| p.to_string().parse::<f64>().ok()).unwrap_or(0.0);

                let shares_to_sell = if loser == "Down" { s.down_order_shares } else { s.up_order_shares };
                if self.config.strategy.simulation_mode {
                    let loss = (purchase_price - sell_price) * shares_to_sell;
                    let mut total = total_profit.lock().await;
                    *total -= loss;
                    info!("SIMULATION: Would sell {} {} shares at ${:.4} (purchased at ${:.2}). Loss: ${:.2}", 
                        shares_to_sell, loser, sell_price, purchase_price, loss);
                } else {
                    if let Err(e) = self.api.place_market_order(token_to_sell, shares_to_sell, "SELL", None).await {
                        error!("Failed to sell {} token for {}: {}", loser, asset, e);
                    } else {
                        let loss = (purchase_price - sell_price) * shares_to_sell;
                        let mut total = total_profit.lock().await;
                        *total -= loss;
                        info!("   Sold {} {} shares at ${:.2}. Loss: ${:.2}", shares_to_sell, loser, sell_price, loss);
                        self.risk.log_trade(asset, &format!("CLOSE_{}", winner), shares_to_sell, sell_price, -loss).await;
                    }
                }
                s.merged = true;
                let trade = self.cycle_trade_holding_winner(&s, winner);
                let mut t = trades.lock().await;
                t.insert(s.condition_id.clone(), trade);
            }
        }
        Ok(())
    }

    async fn handle_one_side_risk(
        &self,
        asset: &str,
        s: &mut PreLimitOrderState,
        total_profit: Arc<Mutex<f64>>,
    ) -> Result<()> {
        let current_time_et = crate::strategy::get_current_time_et();
        let only_one_matched = (s.up_matched && !s.down_matched) || (s.down_matched && !s.up_matched);
        
        if only_one_matched && s.one_side_matched_at.is_none() {
            s.one_side_matched_at = Some(current_time_et);
        }

        let mode = self.config.strategy.signal.one_side_buy_risk_management.to_lowercase();
        let mut should_sell_early = if !only_one_matched {
            false
        } else if mode.contains("price") {
            let price = if s.up_matched {
                self.api.get_price(&s.up_token_id, "SELL").await
            } else {
                self.api.get_price(&s.down_token_id, "SELL").await
            }.ok().and_then(|p| p.to_string().parse::<f64>().ok()).unwrap_or(0.0);
            
            self.risk.is_danger_signal(price)
        } else if mode.contains("time") {
            let danger_mins = self.config.strategy.signal.danger_time_passed as i64;
            s.one_side_matched_at.map_or(false, |t| current_time_et - t >= danger_mins * 60)
        } else {
            // Bug 3 fix: "none" or unknown mode = do not trigger early sell
            false
        };

        if !self.config.strategy.simulation_mode && should_sell_early {
            if let (Some(up_id), Some(down_id)) = (&s.up_order_id, &s.down_order_id) {
                if let Ok((true, true)) = self.api.are_both_orders_filled(up_id, down_id).await {
                    info!("{}: Danger signal but both orders filled — skipping sell", asset);
                    s.up_matched = true;
                    s.down_matched = true;
                    should_sell_early = false;
                }
            }
        }

        if !s.merged && !s.risk_sold && should_sell_early {
            self.execute_danger_sell(asset, s, total_profit).await?;
        }
        Ok(())
    }

    async fn execute_danger_sell(&self, asset: &str, s: &mut PreLimitOrderState, total_profit: Arc<Mutex<f64>>) -> Result<()> {
        let (token_to_sell, other_order_id, purchase_price, side_name) = if s.up_matched {
            (&s.up_token_id, &s.down_order_id, s.up_order_price, "Up")
        } else {
            (&s.down_token_id, &s.up_order_id, s.down_order_price, "Down")
        };

        warn!("{}: Danger/15s Timeout triggered — only {} token matched. Selling and canceling other order", asset, side_name);
        
        // Rule 3: Protección contra Danger Sell a precio cero (Límite max(bid * 0.8, 0.05))
        let bid = self.api.get_price(token_to_sell, "SELL").await.ok()
            .and_then(|p| p.to_string().parse::<f64>().ok()).unwrap_or(0.0);
        
        let limit_price = if bid > 0.0 { (bid * 0.80).max(0.05) } else { 0.05 };
        let shares_to_sell = if side_name == "Up" { s.up_order_shares } else { s.down_order_shares };

        if self.config.strategy.simulation_mode {
            let loss = (purchase_price - limit_price) * shares_to_sell;
            let mut total = total_profit.lock().await;
            *total -= loss;
            warn!("SIMULATION: Would sell {} {} shares at LIMIT ${:.4} (Bid: ${:.3}). Loss: ${:.2}", 
                shares_to_sell, side_name, limit_price, bid, loss);
        } else {
            if let Err(e) = self.place_limit_order(token_to_sell, "SELL", limit_price, shares_to_sell).await {
                error!("Failed to place LIMIT danger sell for {} {}: {}", asset, side_name, e);
            } else {
                if let Some(id) = other_order_id {
                    let _ = self.api.cancel_order(id).await;
                }
                let loss = (purchase_price - limit_price) * shares_to_sell;
                let mut total = total_profit.lock().await;
                *total -= loss;
                warn!("   Placed LIMIT sell {} {} shares at ${:.2}. Est. Loss: ${:.2}", shares_to_sell, side_name, limit_price, loss);
            }
        }
        s.risk_sold = true;
        s.merged = true;
        self.risk.log_trade(asset, &format!("DANGER_SELL_{}", side_name), shares_to_sell, limit_price, -(purchase_price - limit_price) * shares_to_sell).await;
        Ok(())
    }

    async fn check_btc_correlation_trigger(&self, asset: &str, s: &mut PreLimitOrderState) -> Result<()> {
        if let Some(vol) = self.oracle.get_btc_volatility(1000).await {
            let threshold = 0.0025; 
            if vol > threshold && !s.up_matched {
                info!("🚀 BTC BREAKOUT DETECTED ({:.2}%). Front-running {} 'Up' move.", vol * 100.0, asset);
                
                let entry_price = self.api.get_price(&s.up_token_id, "BUY").await.ok()
                    .and_then(|p| p.to_string().parse::<f64>().ok()).unwrap_or(s.up_order_price);

                if self.config.strategy.simulation_mode {
                    info!("SIMULATION: Market order placed for {} Up at ${:.4}", asset, entry_price);
                    s.up_matched = true;
                    s.up_shares = s.up_order_shares;
                    s.winner_entry_price = Some(entry_price);
                    self.risk.log_trade(asset, "ALPHA_BUY_UP", s.up_shares, entry_price, 0.0).await;
                } else {
                    if let Err(e) = self.api.place_market_order(&s.up_token_id, s.up_order_shares, "BUY", None).await {
                        error!("Failed to front-run BTC breakout for {}: {}", asset, e);
                    } else {
                        s.up_matched = true;
                        s.up_shares = s.up_order_shares;
                        s.winner_entry_price = Some(entry_price);
                        if let Some(down_id) = &s.down_order_id {
                            let _ = self.api.cancel_order(down_id).await;
                        }
                        self.risk.log_trade(asset, "ALPHA_BUY_UP", s.up_shares, entry_price, 0.0).await;
                    }
                }
            } else if vol < -threshold && !s.down_matched {
                info!("📉 BTC BREAKDOWN DETECTED ({:.2}%). Front-running {} 'Down' move.", vol * 100.0, asset);

                let entry_price = self.api.get_price(&s.down_token_id, "BUY").await.ok()
                    .and_then(|p| p.to_string().parse::<f64>().ok()).unwrap_or(s.down_order_price);

                if self.config.strategy.simulation_mode {
                    info!("SIMULATION: Market order placed for {} Down at ${:.4}", asset, entry_price);
                    s.down_matched = true;
                    s.down_shares = s.down_order_shares;
                    s.winner_entry_price = Some(entry_price);
                    self.risk.log_trade(asset, "ALPHA_BUY_DOWN", s.down_shares, entry_price, 0.0).await;
                } else {
                    if let Err(e) = self.api.place_market_order(&s.down_token_id, s.down_order_shares, "BUY", None).await {
                        error!("Failed to front-run BTC breakdown for {}: {}", asset, e);
                    } else {
                        s.down_matched = true;
                        s.down_shares = s.down_order_shares;
                        s.winner_entry_price = Some(entry_price);
                        if let Some(up_id) = &s.up_order_id {
                            let _ = self.api.cancel_order(up_id).await;
                        }
                        self.risk.log_trade(asset, "ALPHA_BUY_DOWN", s.down_shares, entry_price, 0.0).await;
                    }
                }
            }
        }
        Ok(())
    }

    async fn handle_mid_market_entry(&self, asset: &str, current_period_et: i64, states: &mut HashMap<String, PreLimitOrderState>) -> Result<()> {
        let current_time_et = crate::strategy::get_current_time_et();
        let time_remaining = (current_period_et + MARKET_DURATION_SECS) - current_time_et;
        let min_remaining = (self.config.strategy.signal.danger_time_passed * 60) as i64;
        
        if time_remaining < min_remaining {
            return Ok(());
        }

        if self.risk.get_place_signal(asset, current_period_et).await == crate::signals::MarketSignal::Good {
            if let Some(current_market) = self.discover_next_market(asset, current_period_et).await? {
                if let Some((up_price, down_price, _)) = self.risk.get_market_snapshot(asset, current_period_et).await {
                    let (up_order_price, down_order_price) = if up_price <= down_price {
                        (self.round_price(up_price), self.round_price(0.98 - up_price))
                    } else {
                        (self.round_price(0.98 - down_price), self.round_price(down_price))
                    };

                    // Rule 2: Filtro de Coste Máximo (Spread Cap) en Mid-market entry
                    if up_order_price + down_order_price > 0.94 {
                        debug!("{} | MID-ENTRY TOO EXPENSIVE: ${:.2} + ${:.2} = ${:.2} (> 0.94). Skipping.", 
                            asset, up_order_price, down_order_price, up_order_price + down_order_price);
                        return Ok(());
                    }

                    let up_size_shares = self.risk.calculate_kelly_size(0.5, up_order_price).await;
                    let down_size_shares = self.risk.calculate_kelly_size(0.5, down_order_price).await;
                    let shares_to_use = up_size_shares.max(down_size_shares).min(self.config.strategy.shares * 5.0).max(1.0);

                    info!("MID-ENTRY: {} | U=${:.2} D:${:.2} | Kelly: {:.1} shares", asset, up_order_price, down_order_price, shares_to_use);
                    let (up_token_id, down_token_id) = self.discovery.get_market_tokens(&current_market.condition_id).await?;
                    let up_order = self.place_limit_order(&up_token_id, "BUY", up_order_price, shares_to_use).await?;
                    let down_order = self.place_limit_order(&down_token_id, "BUY", down_order_price, shares_to_use).await?;

                    let binance_price_at_placement = self.oracle.get_price(asset).await;

                    let new_state = PreLimitOrderState {
                        asset: asset.to_string(),
                        condition_id: current_market.condition_id,
                        up_token_id,
                        down_token_id,
                        up_order_id: up_order.order_id,
                        down_order_id: down_order.order_id,
                        up_order_price,
                        down_order_price,
                        up_matched: false,
                        down_matched: false,
                        merged: false,
                        expiry: current_period_et + MARKET_DURATION_SECS,
                        risk_sold: false,
                        order_placed_at: current_time_et,
                        market_period_start: current_period_et,
                        one_side_matched_at: None,
                        binance_price_at_placement,
                        up_order_shares: shares_to_use,
                        down_order_shares: shares_to_use,
                        up_shares: 0.0,
                        down_shares: 0.0,
                        up_hedged: false,
                        down_hedged: false,
                        both_hedged: false,
                        status: CycleStatus::AcceptingOrders,
                        winner_entry_price: None,
                    };
                    states.insert(asset.to_string(), new_state);
                }
            }
        }
        Ok(())
    }

    pub(crate) async fn discover_next_market(&self, asset_name: &str, next_timestamp: i64) -> Result<Option<Market>> {
        let slug = MarketDiscovery::build_15m_slug(asset_name, next_timestamp);
        match self.api.get_market_by_slug(&slug).await {
            Ok(m) => Ok(if m.active && !m.closed { Some(m) } else { None }),
            Err(e) => {
                debug!("Failed to find market with slug {}: {}", slug, e);
                Ok(None)
            }
        }
    }

    async fn place_limit_order(&self, token_id: &str, side: &str, price: f64, shares: f64) -> Result<OrderResponse> {
        let price = self.round_price(price);
        if self.config.strategy.simulation_mode {
            info!("SIMULATION: Would place {} order for token {}: {:.2} shares @ ${:.2}", 
                side, token_id, shares, price);
            Ok(OrderResponse {
                order_id: Some(format!("SIM-{}-{}", side, chrono::Utc::now().timestamp())),
                status: "SIMULATED".to_string(),
                message: None,
            })
        } else {
            let order = OrderRequest {
                token_id: token_id.to_string(),
                side: side.to_string(),
                size: shares.to_string(),
                price: price.to_string(),
                order_type: "LIMIT".to_string(),
            };
            self.api.place_order(&order).await
        }
    }

    fn round_price(&self, price: f64) -> f64 {
        let rounded = (price * 100.0).round() / 100.0;
        rounded.clamp(0.01, 0.99)
    }

    fn cycle_trade_holding_winner(&self, s: &PreLimitOrderState, winner: &str) -> CycleTrade {
        let (up_shares, down_shares, up_avg, down_avg) = if winner == "Up" {
            (s.up_order_shares, 0.0, s.up_order_price, 0.0)
        } else {
            (0.0, s.down_order_shares, 0.0, s.down_order_price)
        };
        CycleTrade {
            condition_id: s.condition_id.clone(),
            period_timestamp: s.market_period_start as u64,
            market_duration_secs: MARKET_DURATION_SECS_U64,
            up_token_id: Some(s.up_token_id.clone()),
            down_token_id: Some(s.down_token_id.clone()),
            up_shares,
            down_shares,
            up_avg_price: up_avg,
            down_avg_price: down_avg,
        }
    }

    fn cycle_trade_holding_both(&self, s: &PreLimitOrderState) -> CycleTrade {
        CycleTrade {
            condition_id: s.condition_id.clone(),
            period_timestamp: s.market_period_start as u64,
            market_duration_secs: MARKET_DURATION_SECS_U64,
            up_token_id: Some(s.up_token_id.clone()),
            down_token_id: Some(s.down_token_id.clone()),
            up_shares: s.up_order_shares,
            down_shares: s.down_order_shares,
            up_avg_price: s.up_order_price,
            down_avg_price: s.down_order_price,
        }
    }

}
