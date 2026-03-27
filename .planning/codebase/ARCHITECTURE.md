# Architecture Overview

The Polymarket Arbitrage Bot is designed as a high-frequency trading (HFT) system optimized for 15-minute periodic markets. It follows a modular async architecture based on Rust's `tokio` runtime.

## Core Components

### 1. Strategy Engine (`PreLimitStrategy`)
The heart of the bot. It manages the lifecycle of a trade:
- **Discovery:** Identifying the next 15m market slug.
- **Entry:** Placing dual-side limit orders (Up/Down) at a calculated price.
- **Monitoring:** Tracking fill status and market price movements.
- **Exit:** Implementing risk management (danger signals, time-passed exits) and profit-taking (selling the "loser" near expiry if the "winner" is in profit).
- **Resolution:** Redeeming winning tokens and calculating PnL.

### 2. API Layer (`PolymarketApi`)
A robust wrapper around Polymarket's Gamma and CLOB APIs.
- Incorporates the official `polymarket-client-sdk`.
- Handles complex EIP-712 signing for limit orders.
- Supports Proxy Wallets and Gnosis Safe via `alloy`.
- Manages authentication and HMAC-based signed headers.

### 3. Market Discovery (`MarketDiscovery`)
Handles the periodic nature of the target markets.
- Generates time-based slugs (e.g., `btc-updown-15m-1767726000`).
- Translates asset names to Polymarket-compatible slugs.
- Identifies the Up/Down token IDs for a given market.

### 4. Signal Processing (`signals.rs`)
Internal logic for evaluating entry and exit conditions.
- `evaluate_place_signal`: Determines if current market conditions are favorable for mid-period entries.
- `is_danger_signal`: Detects price collapses that trigger emergency exits.

## Data Flow
1. `main.rs` initializes the `PolymarketApi` and `PreLimitStrategy`.
2. `PreLimitStrategy` runs a continuous loop:
   - Polls for new market discovery.
   - Updates state for active positions.
   - Refreshes PnL from the resolution of previous markets.
3. API calls are asynchronous and leveraged via `tokio` for concurrency across multiple assets (BTC, ETH, SOL, XRP).
