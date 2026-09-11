# statarbrust

A Rust statistical-arbitrage pairs-trading engine for the energy sector. Fetches historical prices, finds cointegrated pairs, generates mean-reversion trading signals, backtests them, and produces an HTML dashboard of the results.

## Pipeline

1. **Universe** — 15 energy tickers (`src/universe/energy_list.rs`), fetched from Yahoo Finance.
2. **Cointegration scan** — every pair is tested with Engle-Granger + ADF (MacKinnon critical values); only cointegrated pairs move forward.
3. **Spread & z-score** — OLS hedge ratio, spread, and a rolling z-score (10-day vs 40-day window) per pair.
4. **Signals** — mean-reversion entries at **1.5 SD** and **2.0 SD** (each backtested independently, so aggressive vs. conservative entries can be compared), exiting at 0.5 SD.
5. **Backtest** — per-bar PnL net of transaction costs (2 bps/leg per position change), producing an equity curve, annualized Sharpe ratio, and max drawdown for every pair/threshold combination.
6. **Ranking** — all pairs are ranked best-to-trade by Sharpe ratio.
7. **Forecast** — a short ARMA(1,2)-based 5-day-ahead spread forecast with projected trade direction.

## Output (`output/`)

- `dashboard.html` — ranking table + price/signal charts (both thresholds) and equity curves for every pair.
- `backtests_energy.csv` — rank, pair, threshold, gross/net PnL, Sharpe, max drawdown, trade count.
- Per-pair PNGs: `<A>_<B>_{1_5sd,2sd}_{signals,prices}.png`, `<A>_<B>_equity.png`.

## Running

```bash
cargo run --release
```

Requires network access to fetch price data from Yahoo Finance. Output is written to `output/`.

## Layout

```
src/
  data/      price fetching (Yahoo Finance)
  model/     OLS, ADF/cointegration, spread & z-score, forecasting
  engine/    signal generation, backtesting
  universe/  ticker universe, correlation/cointegration scan, plotting, HTML export
```
