# External Integrations

## 1. Polymarket Gamma API
- **Purpose:** Market discovery and metadata.
- **Usage:** Fetching market slugs, question details, and active status.
- **Endpoint:** Configurable, usually `https://gamma-api.polymarket.com`.

## 2. Polymarket CLOB API
- **Purpose:** Order placement, orderbook data, and settlement.
- **Usage:** Placing limit orders, cancelling orders, checking fill status, fetching current prices.
- **Authentication:** HMAC signatures for headers and EIP-712 signatures for order payloads.

## 3. Polymarket Conditional Tokens (Smart Contract)
- **Purpose:** Redemptions.
- **Usage:** Direct interaction with the `ConditionalTokens` contract on Polygon via `alloy`.
- **Method:** `redeemPositions`.

## 4. EVM Signers
- **Purpose:** Secure transaction and order signing.
- **Support:** Local private keys, Proxy Wallets, and Gnosis Safes.
