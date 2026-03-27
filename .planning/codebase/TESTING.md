# Testing & Verification

## Current State
- **Automated Tests:** Minimal unit tests in the current codebase. The focus has been on runtime stability and integration.
- **Simulation Mode:** A comprehensive `simulation_mode` in `strategy.rs` allows for "paper trading".
  - Fills are inferred from price hits rather than real CLOB fills.
  - PnL is tracked in-memory.
  - No real USDC or assets are used.

## Recommended Verification Plan
1. **Mock API Tests:** Implement `MockPolymarketApi` to test strategy transitions without network calls.
2. **Order Signing Validation:** Unit tests for EIP-712 signature generation to prevent regression in authentication logic.
3. **Market Period Logic:** Unit tests for `discovery.rs` to ensure edge cases (daylight savings, midnight rollovers) are handled correctly.
