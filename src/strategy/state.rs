use serde::{Deserialize, Serialize};

/// 15-minute market duration in seconds
pub const MARKET_DURATION_SECS: i64 = 900;
pub const MARKET_DURATION_SECS_U64: u64 = 900;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycleTrade {
    pub condition_id: String,
    pub period_timestamp: u64,
    pub market_duration_secs: u64,
    pub up_token_id: Option<String>,
    pub down_token_id: Option<String>,
    pub up_shares: f64,
    pub down_shares: f64,
    pub up_avg_price: f64,
    pub down_avg_price: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CycleStatus {
    WaitingForNextCycle, // Last 3 minutes of the current cycle, or idle
    AcceptingOrders,    // First 3 minutes, seeking straddle
    StraddleFormed,     // Both legs matched
    ClosingLoser,       // Min 12-13, exit perdedora
    Expired             // Post-payout/Waiting for cleanup
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreLimitOrderState {
    pub asset: String,
    pub condition_id: String,
    pub up_token_id: String,
    pub down_token_id: String,
    pub up_order_id: Option<String>,
    pub down_order_id: Option<String>,
    pub up_order_price: f64,
    pub down_order_price: f64,
    pub up_matched: bool,
    pub down_matched: bool,
    pub merged: bool,
    pub expiry: i64,
    pub risk_sold: bool,
    pub order_placed_at: i64,
    pub market_period_start: i64,
    /// Timestamp when we first had only one side matched (for sell_after_danger_time_passed)
    pub one_side_matched_at: Option<i64>,
    /// Binance price at the time the limit orders were placed (for Toxic Liquidity check)
    pub binance_price_at_placement: Option<f64>,
    pub up_order_shares: f64,
    pub down_order_shares: f64,
    pub up_shares: f64,
    pub down_shares: f64,
    pub up_hedged: bool,
    pub down_hedged: bool,
    pub both_hedged: bool,
    pub status: CycleStatus,
    pub winner_entry_price: Option<f64>,
}
