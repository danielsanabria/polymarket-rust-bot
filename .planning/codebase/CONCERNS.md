# Codebase Concerns & Debt

- **Monolithic Strategy:** `strategy.rs` is over 900 lines and handles discovery, state management, risk, and PnL. It should be decomposed into smaller modules.
- **Hardcoded Assets:** The list of assets (BTC, ETH, SOL, XRP) is currently hardcoded in `process_markets`. It should be moved to dynamic configuration.
- **Polling Frequency:** The bot relies on polling for price updates. Moving to WebSockets for internal price tracking (even if just from Polymarket) would reduce latency.
- **Single Threaded Event Loop:** While async, the main `process_markets` loop processes assets sequentially. It should spawn parallel tasks per asset.
- **Lack of Hedging:** Currently, a one-sided fill results in high "danger" risk with no protection.
- **Lagging Oracle:** Dependency on internal Polymarket prices means the bot is a "taker" of price movements rather than a "predicter".
