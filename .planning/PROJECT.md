# Project: Polymarket Arbitrage Bot v2

Efficiently capitalize on 15-minute periodic markets on Polymarket using high-speed oracles, correlation strategies, and external hedging.

## Core Value
Maximize risk-adjusted returns in periodic binary markets by exploiting platform latency and market panic.

## Vision
To be the fastest and most robust market participant in Polymarket's 15m periodics, leveraging real-time external data to stay ahead of platform price adjustments.

## Requirements

### Validated
- ✓ Automated discovery of 15-minute periodic markets (BTC, ETH, SOL, XRP)
- ✓ Dual-side limit order placement (Straddle strategy)
- [x] Analyze strategy processing logic in `src/strategy/processor.rs` <!-- id: 0 -->
- [x] Analyze oracle integration in `src/oracle/binance.rs` <!-- id: 1 -->
- [x] Analyze risk management in `src/strategy/risk.rs` <!-- id: 2 -->
- [x] Synthesize explanation for the user <!-- id: 3 -->
- ✓ Real-time PnL tracking and session reporting
- ✓ Automated token redemption on market resolution
- ✓ Simulation/Paper trading mode

### Active
- [ ] **Deterministic State Machine:** Implement `CycleState` and cycle-sync for robust execution.
- [ ] **Hard Risk Guards:** $0.94 straddle cap and limit-based emergency exits ($0.05 floor).
- [ ] **Correlation Alpha:** Real-time Binance BTC trigger for lead-lag ALT trading.
- [ ] **Legging Risk Management:** 15s timeout for imbalanced positions.
- [ ] **Redemption Auditing:** Persistence of `winner_entry_price` for accurate PnL tracking.
- [ ] **Adaptive Kelly Sizing:** Dynamic capital-aware position sizing.
- [ ] **Audit Resolution (Phase 9.1):** Critical fixes for `side_enum`, `final_price`, `redeem_tokens`, and ET/UTC synchronization.
- [ ] **Tech Debt (Phase 9.2):** CLOB auth consolidation, dynamic asset list, and logger merge.
- [ ] **Feature Completion (Phase 9.3):** Flash Module EV Engine and Hyperliquid integration.

### Out of Scope
- [ ] Cross-platform arbitrage (other than hedging) — focus remains on Polymarket.
- [ ] Long-term prediction markets — focus is strictly on the 15m periodic cycles.

## Tech Stack
- **Language:** Rust (Tokio async)
- **APIs:** Polymarket CLOB & Gamma, Binance/Coinbase WS, Hyperliquid/dYdX REST/WS
- **Key Libraries:** `alloy` (evm), `reqwest`, `tokio-tungstenite`, `polymarket-client-sdk`, `rust_decimal`

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Fixed Limit Entry | Ensures a guaranteed profit margin if both sides fill. | Validated |
| 15m Periodics only | High volume and predictable expiration cycles. | Validated |
| High-Speed WS Oracles | Polymarket prices lag; external exchanges define the "true" price. | Pending |

---
*Last updated: 2026-03-27 during v5 hardening initialization*
