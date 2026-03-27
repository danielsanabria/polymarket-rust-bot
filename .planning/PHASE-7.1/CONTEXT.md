# Phase 7.1: Dynamic Inventory Management (Avellaneda-Stoikov)

## Objective
Implement a risk-aware pricing engine that adjusts limit orders based on current inventory levels to maintain delta-neutrality and reduce exposure to one-sided fills.

## Context
The bot currently uses a static `price_limit` (e.g., 0.45) for both Up and Down sides. If the bot accumulates a large position in one direction without balancing it, it assumes significant directional risk. The Avellaneda-Stoikov model provides a mathematical framework to "skew" prices to incentivize the market to fill the side that balances the inventory.

## Requirements
1. **Inventory Tracking**: Real-time tracking of net shares per asset across current and pending markets.
2. **Inventory Penalty Calculation**:
   - $q = Q_{up} - Q_{down}$
   - $\delta = \gamma \cdot q$
   - $P_{up}^* = P_{base} - \delta$
   - $P_{down}^* = P_{base} + \delta$
   - Implementation of clamping (0.01 to 0.99) and real-time tracking from fills.

3. **Kelly Criterion Sizing (Phase 7.2 Integration)**:
   - $b = (1.0 - c) / c$
   - $f^* = (p \cdot (b + 1.0) - 1.0) / b$
   - $f_{final} = f^* \cdot K$
   - `order_size_usdc = bankroll_usdc * f_final`
4. **Simulation Support**: Update simulation mode to reflect inventory-based pricing.

## Success Criteria
- Bot skewed Up/Down limits proportionally to inventory during simulation.
- Net inventory stays closer to zero on average compared to static pricing.
- Bot successfully "buys back" neutral position by over-pricing the missing side.
