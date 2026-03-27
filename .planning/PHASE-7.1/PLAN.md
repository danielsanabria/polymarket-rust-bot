# Implementation Plan: Dynamic Inventory Management (Avellaneda-Stoikov)

## Overview
Implement the Avellaneda-Stoikov market-making model to adjust limit prices based on current inventory.

## Proposed Changes

### [MOD] `src/strategy/state.rs`
- Add `inventory` field to track net shares per asset.
- Implement inventory serialization/deserialization for persistence.

### [MOD] `src/strategy/processor.rs`
- Update `process_asset` to retrieve the current inventory for calculations.
- Pass inventory to `RiskManager`.

### [MOD] `src/strategy/risk.rs`
- Implement `calculate_inventory_delta(asset: &str, inventory: f64) -> f64`
  - $\delta = \gamma \cdot q$
- Implement `calculate_kelly_size(bankroll: f64, p: f64, c: f64, k: f64) -> f64`
  - $b = (1.0 - c) / c$
  - $f^* = (p * (b + 1.0) - 1.0) / b$
  - return $f^* * k * bankroll$

### [MOD] `src/config.rs`
- Add `risk_aversion_gamma` to `StrategyConfig`.
- Add `kelly_fraction_k` to `StrategyConfig`.
- Add `bankroll_usdc` to `StrategyConfig`.

## Step-by-Step Implementation

1. **Inventory Tracker**:
   - Track `net_shares = up_shares - down_shares`.
   - Update `net_shares` whenever an order matches (in `RiskManager`).
2. **Mathematical Framework**:
   - Implement `reservation_price(mid_price, inventory, time_to_expiry, gamma, sigma)`.
   - Skew the base `price_limit`:
     - If `inventory > 0` (long bias), decrease Up limit, increase Down limit.
     - If `inventory < 0` (short bias), increase Up limit, decrease Down limit.
3. **Integration**:
   - Replace static `price_limit` usage in `processor.rs` with calls to `calculate_skewed_limit`.
4. **Verification**:
   - Run simulation and manually inspect if limits skew as expected based on simulated matches.
