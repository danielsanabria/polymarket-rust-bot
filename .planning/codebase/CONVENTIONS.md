# Coding Conventions

- **Async Everywhere:** The system is fully asynchronous using `tokio`, including file loading and API polling.
- **Defensive API Calls:** Heavy use of `Result` and `anyhow` for clear error context. All API responses are validated for structure and success flags.
- **Shared State:** Use of `Arc<Mutex<T>>` for the `PolymarketApi` and global strategy state to allow safe concurrent access from background tasks.
- **Precise Financials:** `rust_decimal` is used for all prices and sizes to avoid binary floating-point errors. Converstion to `f64` is only done for logging or high-level comparisons when precision is less critical.
- **Logging:** Structured logging using the `log` crate with `Info`, `Warn`, and `Error` levels. `eprintln!` is used for critical UI-like confirmation messages in the console.
- **Timezone Awareness:** All market cycle calculations are normalized to New York (Eastern) time as per Polymarket's periodic market schedule.
