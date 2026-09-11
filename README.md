# statarbrust

A Rust statistical-arbitrage pairs-trading engine for the energy sector. Fetches historical prices, finds cointegrated pairs, generates mean-reversion trading signals, backtests them, and produces an HTML dashboard of the results.

## Pipeline

1. **Universe** — 15 energy tickers (`src/universe/energy_list.rs`), fetched from Yahoo Finance.
2. **Cointegration scan** — every pair is tested with Engle-Granger + ADF (MacKinnon critical values); only cointegrated pairs move forward.
3. **Spread & z-score** — OLS hedge ratio, spread, and a rolling z-score (10-day vs 40-day window) per pair.
4. **Signals** — mean-reversion entries at **1.5 SD** and **2.0 SD** (each backtested independently, so aggressive vs. conservative entries can be compared), exiting at 0.5 SD.
5. **Backtest** — per-bar PnL net of transaction costs (2 bps/leg per position change), producing an equity curve, annualized volatility, a risk-free-adjusted Sharpe ratio, and max drawdown for every pair/threshold combination.
6. **Ranking** — all pairs are ranked best-to-trade by Sharpe ratio.
7. **Forecast** — a short ARMA(1,2)-based 5-day-ahead spread forecast with projected trade direction.

## Output (`output/`)

- `dashboard.html` — ranking table + price/signal charts (both thresholds) and equity curves for every pair.
- `backtests_energy.csv` — rank, pair, threshold, gross/net PnL, risk-free rate, volatility, Sharpe, max drawdown, trade count.
- Per-pair PNGs: `<A>_<B>_{1_5sd,2sd}_{signals,prices}.png`, `<A>_<B>_equity.png`.

## Running

```bash
cargo run --release
```

Requires network access to fetch price data from Yahoo Finance. Output is written to `output/`.

## Results (snapshot)

Yahoo Finance data is live, so results shift run to run — this table is from one run on 2026-09-11, included for reference. Full ranking in `output/backtests_energy.csv`.

| Rank | Pair | Entry SD | Risk-Free Rate | Volatility | Sharpe | Max Drawdown | Net PnL | Trades |
|---|---|---|---|---|---|---|---|---|
| 1 | SLB / APA | 1.5 | 4.00% | 1.6044 | 0.312 | 6.2414 | 7.50 | 54 |
| 2 | DVN / FANG | 1.5 | 4.00% | 3.0500 | 0.258 | 10.1834 | 11.48 | 47 |
| 3 | COP / FANG | 1.5 | 4.00% | 2.3203 | 0.181 | 4.2961 | 6.37 | 54 |
| 4 | EOG / FANG | 1.5 | 4.00% | 2.6668 | 0.178 | 7.4005 | 7.12 | 48 |
| 5 | HAL / APA | 1.5 | 4.00% | 1.2287 | 0.091 | 2.9023 | 2.10 | 59 |
| 6 | XOM / FANG | 1.5 | 4.00% | 1.5371 | 0.088 | 4.1831 | 2.43 | 36 |
| 7–17 | (all 2.0 SD pairs) | 2.0 | 4.00% | 0.0000 | 0.000 | 0.0000 | 0.00 | 0 |
| 18 | CVX / FANG | 1.5 | 4.00% | 4.1806 | -0.048 | 18.8277 | -2.20 | 59 |
| 19 | CVX / MPC | 1.5 | 4.00% | 4.3307 | -0.119 | 19.8911 | -6.58 | 48 |
| 20 | MPC / PSX | 1.5 | 4.00% | 2.0801 | -0.177 | 8.8871 | -4.55 | 47 |
| 21 | PSX / VLO | 1.5 | 4.00% | 2.5979 | -0.243 | 11.9833 | -8.21 | 46 |
| 22 | SLB / HAL | 1.5 | 4.00% | 1.0698 | -0.610 | 9.4672 | -8.49 | 58 |

Every 2.0 SD pair shows zero trades: with this z-score's 10/40-day window, the spread rarely swings past 2 SD, so the conservative threshold never fires. It's a real (if underwhelming) finding, not a bug — see Limitations below.

### Top 3 pairs

**#1 SLB / APA** (Sharpe 0.312)

![SLB/APA price + signals](output/SLB_APA_1_5sd_prices.png)
![SLB/APA equity curve](output/SLB_APA_equity.png)

**#2 DVN / FANG** (Sharpe 0.258)

![DVN/FANG price + signals](output/DVN_FANG_1_5sd_prices.png)
![DVN/FANG equity curve](output/DVN_FANG_equity.png)

**#3 COP / FANG** (Sharpe 0.181)

![COP/FANG price + signals](output/COP_FANG_1_5sd_prices.png)
![COP/FANG equity curve](output/COP_FANG_equity.png)

## Layout

```
src/
  data/      price fetching (Yahoo Finance)
  model/     OLS, ADF/cointegration, spread & z-score, forecasting
  engine/    signal generation, backtesting
  universe/  ticker universe, correlation/cointegration scan, plotting, HTML export
```

## Assumptions & limitations

- **Sizing**: backtests use unit notional (1.0) per pair, so PnL/Sharpe/volatility figures are in normalized units, not dollars — useful for ranking pairs against each other, not as a real P&L.
- **Costs**: 2 bps per leg charged on every position change (entry, exit, or a direct long↔short flip); no market impact or partial fills modeled.
- **Risk-free rate**: fixed at 4%/year (`RISK_FREE_RATE_ANNUAL` in `src/engine/backtest.rs`), applied per-bar against unit-notional returns — an approximation, not a real cash rate.
- **Hedge ratio look-ahead bias**: the OLS hedge ratio (`hedge_ratio_ols`) is fit once on the *entire* price series per pair, so early trades in the backtest technically use a hedge ratio informed by the full history, including data that hadn't happened yet. A walk-forward alternative (`walk_forward_hedge_ratio` in `src/model/spread.rs`) already exists in the codebase and fixes this, but main.rs doesn't use it yet — worth wiring in before treating results as realistic.
- **Not investment advice**: this is a research/education tool; do not trade on its output without independent validation.
