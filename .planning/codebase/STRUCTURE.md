# Project Structure

```text
src/
├── main.rs          # Entry point, initialization, and redemption-only mode
├── api.rs           # Core Polymarket API implementation (Gamma/CLOB/Redemption)
├── strategy.rs      # Main trading strategy and market processing loop
├── discovery.rs     # Logic for periodic market discovery and slug generation
├── signals.rs       # Entry/Exit signal evaluation
├── models.rs        # Shared data structures and API models
└── config.rs        # Configuration loading (JSON/Env/Args)
```

## Module Definitions

- **main.rs:** Handles CLI arguments, logger setup, and starts the strategy loop or one-off tasks like `redeem`.
- **api.rs:** Thick wrapper for Polymarket. Contains the `PolymarketApi` struct and its complex signing logic for orders.
- **strategy.rs:** Contains `PreLimitStrategy`. Manages the state machine for each asset of interest.
- **discovery.rs:** Specialized in calculating timestamps for 15-minute and 1-hour periods in New York time.
- **signals.rs:** Pure logic for determining whether to buy or sell based on price thresholds.
- **models.rs:** Contains `Market`, `OrderBook`, `OrderResponse`, and internal state structs like `PreLimitOrderState`.
- **config.rs:** Syncs `config.json` with the application's runtime parameters.
