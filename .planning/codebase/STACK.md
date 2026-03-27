# Tech Stack

## Core
- **Language:** Rust (Edition 2021)
- **Runtime:** `tokio` (Async/Await)
- **Error Handling:** `anyhow`

## Networking & API
- **HTTP Client:** `reqwest`
- **WebSocket:** `tokio-tungstenite`
- **Polymarket Interaction:** `polymarket-client-sdk` (v0.4.2)
- **EVM Interaction:** `alloy` (v1.3)

## Data Processing
- **Serialization:** `serde`, `serde_json`, `toml`
- **Math:** `rust_decimal` (Precise decimal math for finance)
- **Time:** `chrono`, `chrono-tz` (Timezone support for ET/UTC conversions)

## Security & Crypto
- **Hashing:** `sha2`, `hmac`
- **Encoding:** `hex`, `base64`

## Utilities
- **CLI:** `clap`
- **Logging:** `log`, `env_logger`
- **FS:** `walkdir`
