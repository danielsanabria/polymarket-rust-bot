# Requirements: Polymarket Arbitrage Bot v2

## 1. High-Speed Oracle Integration
- **Feature:** Real-time price tracking from top-tier exchanges.
- **Goal:** Use external prices as the "Ground Truth" to anticipate Polymarket price adjustments.
- **Must Have:**
  - WebSocket connection to Binance Spots (BTC, ETH, SOL, XRP).
  - Background price cache with sub-10ms update latency.
  - Automatic order cancellation if Binance price deviates by $> 0.5\%$ from the Polymarket limit price.

## 2. Lead-Lag Correlation Sensor
- **Feature:** Predictive buy/sell signals based on BTC dominance.
- **Goal:** Front-run ALT price adjustments on Polymarket.
- **Must Have:**
  - BTC price volatility tracker (e.g., if BTC moves $> 0.3\%$ in $< 500$ ms).
  - Configurable "Aggression" parameter for ALT buy-ins during BTC breakouts.
  - Automated limit adjustment for pending ALT orders based on BTC direction.

## 3. External Hedging (Hyperliquid)
- **Feature:** Risk mitigation for one-sided fills.
- **Goal:** Protected exposure when only the "Up" or "Down" side is matched.
- **Must Have:**
  - Integration with Hyperliquid SDK for Perpetual Futures.
  - Automated "Short" opening if "Up" token is filled but "Down" is not matched within 10 seconds.
  - Position sizing parity (hedge size = Polymarket share count * price).
  - Automated hedge closure when the market expires or the counter-side fills later.

## 4. Flash Liquidity Module (Last 120s)
- **Feature:** Opportunistic endgame trading.
- **Goal:** Capitalize on late-period panic and liquidity squeezes.
- **Must Have:**
  - Countdown timer for 15m period end.
  - $EV$ (Expected Value) calculator: $EV = (Prob \times G) - ((1 - Prob) \times L)$.
  - Probability inference based on Binance Spot vs Polymarket Mid-price.
  - High-priority execution task in the final 120 seconds.

## 5. Performance & Architecture
- **Must Have:**
  - Parallel processing for assets (BTC, ETH, SOL, XRP in separate threads).
  - Move from polling to internal event-driven architecture.
  - Refactor `strategy.rs` into specialized sub-modules (Oracle, Hedger, Strategy, Scanner).

## 6. Testing & Validation
- **Must Have:**
  - Historical backtesting module using Binance CSV/JSON data.
  - Mock API responses for Hyperliquid/Polymarket testing.
  - Paper trading support for all new V2 features.
