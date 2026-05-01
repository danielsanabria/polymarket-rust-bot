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
**Goal:** Restore positive edge through strict arbitrage caps and adaptive loss cutting.

- **Phase 10.1: Hard Entry Guards & Mid-Market Restriction**
  - Enforce $0.94 straddle cap in `processor.rs`.
  - Disable or strictly cap mid-market entries.
- **Phase 10.2: Adaptive Loss Management (Loser Sell Logic)**
  - Sell loser leg when winner hits $0.70 ($sell\_opposite\_above$).
  - Implement price-based stop loss ($0.25) for loser leg.
- **Phase 10.3: Early Danger Exit & Precision Execution**
  - Raise `danger_price` to $0.25 and use limit orders with $0.05 floor.
- [ ] **Strategic De-risking (BTC & Kelly):**
  - Disable non-neutral directional buys from BTC correlation.
  - Recalculate Kelly inputs for fill-probability neutral straddles.

## Recurring: Knowledge Lifecycle & Quality Control
**Goal:** Maintain the memory and mathematical integrity of the bot across all future versions.

- **Post-Milestone Retrospective (PMR):**
  - Update `.planning/lessons.md` after every milestone completion or major strategy pivot.
  - Analyze performance data from `trades.csv` and bot logs to identify new anti-patterns.
- **Guardrail Audit:**
  - Verify every new strategy against the "Project Guardrails" in `PROJECT.md` before starting implementation.
  - Audit price feed integrity (Gamma vs CLOB) during the Research phase of every new arbitrage module.

### Phase 1: Integrar IA local (Ollama + Gemma 2B) para estrategia en tiempo real

**Goal:** [To be planned]
**Requirements**: TBD
**Depends on:** Phase 0
**Plans:** 0 plans

Plans:
- [ ] TBD (run /gsd-plan-phase 1 to break down)
