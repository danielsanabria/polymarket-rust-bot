# Roadmap: Polymarket Arbitrage Bot v2

## Milestone 1: Core Modularization & Parallelization
**Goal:** Transition from a monolithic polling loop to a high-performance event-driven core.

- **Phase 1.1: Structure Refactor**
  - Decomposition of `strategy.rs` into `mod strategy/` (scanner, execution, risk, pnl).
  - Move asset list to dynamic configuration.
- **Phase 1.2: Threading & Concurrency**
  - Implement per-asset `tokio::spawn` tasks to eliminate sequential processing lag.
  - Implement shared cross-thread price cache.

## Milestone 2: High-Speed Oracle & Real-time Risk
**Goal:** Use external exchange data to protect against platform latency.

- **Phase 2.1: Binance WebSocket Integration**
  - Implementation of `oracle/binance.rs` using `tokio-tungstenite`.
  - Real-time stream processing for `aggTrade` events.
- **Phase 2.2: Oracle-Driven Order Management**
  - Logic to "Kill" orders (Immediate cancel) if Binance price moves against the position before Polymarket fill.

## Milestone 3: Predictive Alpha (Lead-Lag)
**Goal:** Front-run Polymarket price adjustments using BTC as a lead indicator.

- **Phase 3.1: BTC Volatility Sensor**
  - Implementation of a sliding-window volatility tracker for BTCUSDT.
- **Phase 3.2: Correlation Trigger**
  - Automated "Up"/"Down" buys for ALT markets (SOL, XRP) when BTC breaks out/down.

## Milestone 4: External Hedging (Risk Floor)
**Goal:** Eliminate the "one-side filled" catastrophe risk.

- **Phase 4.1: Hyperliquid Integration**
  - Implementation of `hedger/hyperliquid.rs`.
  - Secure API key management and order signing.
- **Phase 4.2: Hedge Orchestration**
  - Automated short/long opening on Hyperliquid when Polymarket fills are imbalanced.
  - Automated position closing upon market resolution.

## Milestone 5: The "Flash" Module
**Goal:** Capture late-period liquidity panic in the final 120 seconds.

- **Phase 5.1: EV Calculation Engine**
  - Implementation of real-time $EV$ calculation based on $Prob = f(Spot, MidPrice)$.
- **Phase 5.2: Flash Execution**
  - High-priority taker-mode module for the final 2 minutes.

## Milestone 6: Verification & Backtesting
**Goal:** Prove profitability and stability before live deployment.

- **Phase 6.1: V2 Simulation Suite**
  - Extension of simulation mode to include Oracle and Hedge latency modeling.
- **Phase 6.2: Historical Audit**
  - Tooling to run the bot against 24h of historical Binance/Polymarket data.

## Milestone 7: Advanced Risk & Adaptive Sizing
**Goal:** Implement mathematical frameworks for inventory neutrality and optimal capital allocation.

- **Phase 7.1: Dynamic Inventory Management (Avellaneda-Stoikov)**
  - Implement inventory-aware limit pricing.
  - Skew Up/Down limits based on current net exposure.
- **Phase 7.2: Mathematical Sizing (Kelly Criterion)**
  - Implementation of `KellyEngine` for dynamic bet sizing.
  - Integration with Binance Oracle for real-time edge calculation.

## Milestone 8: Execution Optimization & Hardening
**Goal:** Enhance execution reliability, risk management, and audit trailing based on v5 performance logs.

- **Phase 8.1: Deterministic State Machine & Cycle Sync**
  - Implement `CycleState` enum for robust flow control.
  - Deterministic `cycle_window` calculation: `floor(unix_timestamp / 900) * 900`.
  - 12-minute entry cut-off and 3-minute straddle completion timeout.
- **Phase 8.2: Hard Risk Guards & Legging Protection**
  - Hard cap on total straddle cost ($0.94).
  - Danger Sell Protection: Shift from market to limit orders with $0.05 floor.
  - 15-second timeout for imbalanced legs (immediate hedge/exit).
- **Phase 8.3: Auditing & Dynamic Kelly Criterion**
  - Persistence of `winner_entry_price` for accurate redemption P&L calculation.
  - Update `EXPIRED` event logging to reflect real session capital growth.
  - Integrate updated current capital into `KellyCriterion::calculate()`.
- **Phase 8.4: Correlation Alpha (Binance BTC Trigger)**
  - Background WebSocket task for Binance BTC futures volatility tracking.
  - Independent lead-lag trigger for ALT markets based on BTC breakouts.

## Milestone 9: Audit-Driven Stabilization & Logic Consolidation
**Goal:** Resolve identified bugs and technical debt from the v5 audit.

- **Phase 9.1: Critical Bug Fixing & Timezone Sync**
  - Fix `side_enum` moves and `final_price` re-signing logic in `api.rs`.
  - Unify `seconds_elapsed` source to ET across `processor.rs`.
  - Fix `redeem_tokens` hardcoding and `winner_entry_price` logic.
- **Phase 9.2: Technical Debt & Authentication Refactor**
  - Consolidate duplicated `CLOB` authentication blocks.
  - Implement dynamic asset list from configuration.
  - Merge duplicated `log_trade` implementations.
- **Phase 9.3: Flash Module & Hedge Implementation**
  - Implement Milestone 5 Flash Module (EV Engine).
  - Transition HyperliquidHedger from stub to functional.

## Milestone 10: Strategy Stabilization & Edge Restoration
**Goal:** Restore positive expected value by eliminating negative-edge entries, tightening loss
management, and isolating the pure arbitrage core from directional noise.

**Context:** Live simulation data (26–27 Mar 2026, ~21h of operation) revealed that 100% of
recorded exits are losses. Root cause analysis identified three compounding failure modes:
(1) straddle entry costs frequently at or above $1.00 (no mathematical edge);
(2) the loser leg closes at $0.05–$0.20 because the winner trigger ($0.95) fires too late;
(3) BTC correlation directional buys create single-leg exposures at inflated prices ($0.59–$0.61),
bypassing all straddle guards.

Phases are ordered by impact-to-risk ratio — highest damage sources first.

---

### Phase 10.1: Strategic De-risking — Neutralize Directional Noise
**Priority: Critical. Must ship before any other phase.**

**Problem:** `check_btc_correlation_trigger` in `processor.rs` places single-leg market orders
(BUY_UP or BUY_DOWN) when BTC moves >0.25% in <1s. These entries bypass the straddle cost cap,
buy at market price ($0.55–$0.65), and leave an unhedged directional position. In the simulation
data, these are the highest individual losses per trade.

**Changes:**

- `src/strategy/processor.rs` — `check_btc_correlation_trigger()`:
  - Add config flag `btc_correlation_enabled: bool` (default: `false`).
  - Gate the entire function body behind this flag.
  - When disabled, return `Ok(())` immediately — zero side effects.

- `src/config.rs` — `SignalConfig`:
  - Add `btc_correlation_enabled: bool` with `#[serde(default)]` → `false`.
  - Add `btc_correlation_threshold: f64` with default `0.003` (raise from 0.0025 to reduce noise).
  - Add `btc_correlation_min_straddle_cost: f64` — if re-enabled, only trigger when the
    implied straddle (entry price × 2) stays below $0.94.

- `src/strategy/processor.rs` — mid-market entry (`handle_mid_market_entry()`):
  - Add a separate config flag `mid_market_enabled` (already exists) but add a secondary guard:
    straddle cost at time of entry must be < $0.90 (tighter than pre-period entries) because
    mid-period prices are already directional.
  - If the market has moved more than 15% from 0.50/0.50 baseline (e.g. Up > $0.65 already),
    skip entirely — no edge remains.

**Verification:** After this phase, `trades.csv` should contain zero `ALPHA_BUY_UP` or
`ALPHA_BUY_DOWN` entries. All BUY entries should be paired (BUY_UP + BUY_DOWN same asset
within same cycle).

---

### Phase 10.2: Hard Entry Guards — Enforce Straddle Arbitrage Cap
**Priority: High.**

**Problem:** The $0.97 cap in `processor.rs` is too loose. With `up + down = $0.97`, the
theoretical max profit is $0.03/share. After Polymarket's fee (~1%), effective edge is near zero
or negative. The cap must be $0.94 to guarantee a meaningful edge buffer ($0.06/share minimum).

**Additionally:** When both sides are bought at slightly different times (Up first, then Down
30–60s later), the second leg often prices in the initial move, raising total straddle cost
above cap retroactively.

**Changes:**

- `src/strategy/processor.rs` — `process_asset()` pre-order block:
  ```
  // Current
  if straddle_cost > 0.97 { ... skip ... }

  // Replace with
  if straddle_cost >= 0.94 { ... skip ... }
  ```

- `src/strategy/processor.rs` — add **second-leg cost check** in `check_order_matches()`:
  - When the first leg fills (e.g. Up matched), fetch current ask for the Down leg.
  - Compute `implied_straddle = up_fill_price + down_current_ask`.
  - If `implied_straddle >= 0.94`, cancel the pending Down order and mark state as
    `WaitingForNextCycle`. Log as `SKIP_SECOND_LEG_EXPENSIVE`.
  - This prevents the "legging risk" where the bot is stuck holding one expensive leg.

- `src/config.rs` — `StrategyConfig`:
  - Add `straddle_hard_cap: f64` with default `0.94`.
  - Add `straddle_second_leg_cap: f64` with default `0.94` (can be tuned independently).

**Verification:** Average straddle cost in `trades.csv` should drop below $0.92. No individual
straddle should appear above $0.94.

---

### Phase 10.3: Adaptive Loss Management — Rewrite Loser Sell Logic
**Priority: High.**

**Problem:** `sell_opposite_above = 0.95` means the bot waits until the winner is nearly certain
($0.95) before selling the loser. By that point the loser is at $0.03–$0.10, generating maximum
loss on that leg. The math: buy Up @ $0.45, buy Down @ $0.45, total $0.90. If Up → $0.95,
Down → $0.05. Selling Down at $0.05 = loss of $0.40/share on Down leg, net straddle P&L = $0.00.

The target is to sell the loser when it still retains meaningful value.

**Changes:**

- `src/config.rs` — `StrategyConfig`:
  - Change `sell_opposite_above` default from `0.95` to `0.70`.
  - Add `sell_opposite_time_remaining_mins: u64` with default `10` (down from 15).
    At 10 minutes remaining the loser still has ~$0.25–$0.35 of value if the winner is at $0.70.
  - Add `loser_stop_loss_price: f64` with default `0.25`. If either leg falls below this
    regardless of the winner's price, sell it immediately (time-independent stop).

- `src/strategy/processor.rs` — `handle_both_matched()`:
  - Add **loser stop-loss** check: runs every tick, independent of `time_remaining_mins`.
    ```rust
    // New: price-based stop loss on loser leg (runs always, not gated on time)
    let loser_price = if up_price >= down_price { down_price } else { up_price };
    if loser_price <= self.config.strategy.loser_stop_loss_price && !s.merged {
        // sell loser immediately at limit (bid * 0.95).max(0.05)
    }
    ```
  - Lower the time-gated trigger: `sell_opposite_above` → `0.70` in config.
  - Keep the existing time-remaining gate but lower it to 10 minutes.

- **Rationale for $0.70 / 10min combination:**
  When the winner hits $0.70, the loser is typically at $0.28–$0.32. Selling at that point:
  - Entry cost: $0.90 (avg straddle)
  - Winner value at expiry: $1.00
  - Loser sell: $0.30
  - Net: ($1.00 + $0.30) − $0.90 = **+$0.40/share profit**
  vs current ($1.00 + $0.05) − $0.90 = **+$0.15/share** (best case) or $0.00 (worst case).

**Verification:** After this phase, CLOSE trades in `trades.csv` should show exit prices of
$0.25–$0.35 on loser legs instead of $0.05–$0.20.

---

### Phase 10.4: Early Danger Exit — Precision Emergency Execution
**Priority: Medium. Reduces tail-loss severity.**

**Problem:** `danger_price = 0.15` triggers too late. Markets move from $0.45 to $0.01 in under
90 seconds on Polymarket. By the time the bot reads the price, executes the check, and places an
order, the actual fill is at $0.003–$0.01 (as seen in `trades.csv`).

**Changes:**

- `src/config.rs` — `SignalConfig`:
  - Raise `danger_price` default from `0.15` to `0.28`.
  - This gives the bot ~2–3 more check cycles before price reaches zero.

- `src/strategy/processor.rs` — `execute_danger_sell()`:
  - Already uses `(bid * 0.80).max(0.05)` limit — keep this.
  - Add **pre-flight check**: before placing the limit order, fetch current mid price.
    If mid < $0.05, skip the sell entirely and just cancel the order (nothing to recover).
    Selling at $0.05 with Polymarket fees may return $0.00 net — not worth the API call.
  - Add `danger_time_passed` reduction: lower from 30 minutes to `15` minutes.
    If one leg hasn't filled in 15 minutes, the market has directionally resolved.

- `src/strategy/risk.rs` — `check_oracle_safety()`:
  - Tighten the Binance toxic liquidity threshold from `0.005` (0.5%) to `0.003` (0.3%).
    Orders should be killed faster when Binance diverges from entry price.

**Verification:** `DANGER_SELL` exit prices in `trades.csv` should average $0.20–$0.30 instead
of $0.05–$0.10.

---

### Phase 10.5: Kelly Criterion Recalibration for Straddle P&L
**Priority: Low. Sizing optimization — only meaningful once edge is restored.**

**Problem:** `calculate_kelly_size()` uses `p = 0.52` (directional win probability), which is
the wrong model for a straddle. In a straddle, the relevant probability is `p_fill` — the
probability that both legs fill before the period ends. With `p = 0.52`, Kelly oversizes
positions in low-liquidity periods.

**Changes:**

- `src/strategy/risk.rs` — `calculate_kelly_size()`:
  - Replace the hardcoded `p_up = 0.52` in `processor.rs` with a dynamic estimate.
  - Add `fill_probability_estimate(asset, time_until_period, spread) -> f64`:
    - Starts at 0.85 (based on historical fill rates for liquid pairs like BTC).
    - Decays linearly as `time_until_period` shrinks below 60s.
    - Scales down for wider spreads (e.g. XRP, SOL have lower liquidity).
  - The Kelly formula for a straddle with guaranteed $1.00 payout:
    - `b = (1.00 - straddle_cost) / straddle_cost` (net odds)
    - `p = fill_probability_estimate(...)` (both legs fill)
    - `f* = (p*(b+1) - 1) / b`
    - `f_final = f* * kelly_fraction_k`

- `src/config.rs` — `StrategyConfig`:
  - Add `fill_probability_btc: f64` default `0.88`.
  - Add `fill_probability_eth: f64` default `0.85`.
  - Add `fill_probability_sol: f64` default `0.78`.
  - Add `fill_probability_xrp: f64` default `0.75`.
  These can be tuned based on observed fill rates from `trades.csv`.

---

## Implementation Order & Config Changes Summary

Phases must be implemented in order: **10.1 → 10.2 → 10.3 → 10.4 → 10.5**.
Each phase is independently verifiable via `trades.csv` before proceeding.

### Config values to update immediately (before any code changes):

```json
{
  "strategy": {
    "straddle_hard_cap": 0.94,
    "sell_opposite_above": 0.70,
    "sell_opposite_time_remaining": 10,
    "loser_stop_loss_price": 0.25,
    "signal": {
      "mid_market_enabled": false,
      "btc_correlation_enabled": false,
      "danger_price": 0.28,
      "danger_time_passed": 15
    }
  }
}
```

These config changes alone (no code changes required) implement the intent of phases 10.1–10.4
partially. The code changes then enforce the caps at the execution layer so they cannot be
bypassed by edge cases.

### Expected outcome after full Milestone 10:
- Average straddle cost: $0.88–$0.92 (currently $0.96–$1.02)
- Loser leg exit price: $0.25–$0.35 (currently $0.05–$0.20)
- Net straddle P&L per winner cycle: +$0.30–$0.45/share (currently $0.00 to -$0.15)
- Danger sell exit price: $0.20–$0.30 (currently $0.003–$0.10)
- BTC correlation directional entries: 0 (currently ~15% of all entries)
