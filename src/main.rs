mod data;
mod model;
mod engine;
mod universe;

use anyhow::Result;
use universe::energy_list::energy_universe;
use std::io::Write;
use universe::fetch_all::fetch_universe;
use universe::correlation::correlation_matrix;
use universe::cointegration_scan::scan_cointegration;
use model::spread::{hedge_ratio_ols, spread, zscore, rolling_zscore, walk_forward_hedge_ratio, half_life};
use engine::backtest::{backtest_pair, BacktestResult};
use engine::align::align_series;
use universe::export::export_backtests;
use universe::export_json::{export_equity_curves, EquityCurveEntry};
use universe::export_html::export_full_dashboard;
use universe::plots::{plot_series_svg, plot_prices_with_signals_svg, plot_signals_with_forecast_svg, plot_equity_curves_svg};
use crate::engine::forecast::{normalize_with_last_window};
use crate::engine::signals::{generate_signals, trade_direction};
use chrono::NaiveDate;
use crate::model::forecast::forecast_arma12;
use crate::engine::signals::{per_stock_signals, StockSignal};

// ---------------------------------------------------------------------
// WALK-FORWARD HEDGE RATIO CONFIGURATION
// ---------------------------------------------------------------------
// These two numbers control how the hedge ratio (and hence the spread and
// every entry/exit signal built on top of it) is estimated for EVERY pair
// below. Read model::spread::walk_forward_hedge_ratio's doc comment first
// if the terms "formation period" / "lookback" are unfamiliar.
//
// WALK_FORWARD_LOOKBACK: how many trailing bars (trading days) are used
// each time the hedge ratio is refit. Set to one trading year (~252 days),
// matching the 12-month "formation period" convention from Gatev,
// Goetzmann & Rouwenhorst (2006), "Pairs Trading: Performance of a
// Relative-Value Arbitrage Rule," Review of Financial Studies 19(3) --
// the paper that established the modern empirical pairs-trading
// methodology this project follows loosely. A full year of daily data
// gives the OLS fit enough observations to be statistically stable while
// still being short enough to track a relationship that can genuinely
// drift over a 10+ year sample.
//
// WALK_FORWARD_REESTIMATE_EVERY: how often (in bars) the hedge ratio is
// refit using that trailing window. Refitting every single bar would be
// needlessly expensive and would let the ratio chase short-term noise;
// refitting too rarely risks trading on a stale relationship. Once a
// month (~21 trading days) is a common practical middle ground in
// industry pairs-trading writeups (e.g. Chan, "Algorithmic Trading,"
// 2013) and keeps the number of OLS fits per pair small (roughly
// sample_length / 21, a few hundred at most -- negligible compute cost).
const WALK_FORWARD_LOOKBACK: usize = 252;
const WALK_FORWARD_REESTIMATE_EVERY: usize = 21;

#[tokio::main]
async fn main() -> Result<()> {

    // Charts grouped per pair (pair label -> [(chart title, path), ...]) so
    // the dashboard can lay out each pair's charts side by side instead of
    // interleaving every pair into one flat grid.
    let mut chart_groups = Vec::<(String, Vec<(String, String)>)>::new();

    // Full bar-by-bar equity curves (dates + values), one entry per pair
    // per threshold -- exported to JSON at the end for anything that wants
    // to draw a REAL interactive chart (zoom, hover, live redraw) instead
    // of the static PNG images plotted above. See universe/export_json.rs.
    let mut equity_curve_entries = Vec::<EquityCurveEntry>::new();

    // Create output folder if it doesn't exist
    std::fs::create_dir_all("output")?;

    // 1. Universe
    let tickers = energy_universe();

    // 2. Fetch
    let mut data = fetch_universe(&tickers).await?;

    // 3. Align
    data = align_series(&data);
    let timestamps = &data[0].2;   // timestamps for the first ticker

    let hist_dates: Vec<NaiveDate> = timestamps
    .iter()
    .map(|ts| ts.date())
    .collect();

    // 4. Correlation
    let corrs = correlation_matrix(&data);
    println!("Top correlated pairs:");
    for (a, b, c) in corrs.iter().take(5) {
        println!("{}/{} corr={:.3}", a, b, c);
    }

    // 5. Cointegration -- see universe/cointegration_scan.rs for exactly
    // what "cointegrated" means here and why it tests both regression
    // directions before deciding.
    let coint = scan_cointegration(&data);
    let coint_pairs: Vec<_> = coint.into_iter()
        .filter(|(_, _, _, is_c)| *is_c)
        .collect();

    println!("Cointegrated pairs:");
    for (a, b, adf, _) in &coint_pairs {
        println!("{}/{} ADF={:.3}", a, b, adf);
    }

    // 6. Demo if empty -- illustrative-only fallback so the program still
    // produces *something* visual when no pair in the universe clears the
    // cointegration bar. This intentionally uses the SIMPLER one-shot,
    // full-sample, raw-price hedge ratio (not the walk-forward/log-price
    // one used for real trading below) since it exists purely to draw two
    // example charts, not to generate a tradeable signal.
    if coint_pairs.is_empty() {
        println!("No cointegrated pairs found — generating demo plots.");

        let (a, b, _) = &corrs[0];

        let xa = data.iter().find(|(n, _, _)| n == a).unwrap().1.clone();
        let yb = data.iter().find(|(n, _, _)| n == b).unwrap().1.clone();
        let n = xa.len().min(yb.len());
        let (x, y) = (&xa[..n], &yb[..n]);

        let (alpha, beta) = hedge_ratio_ols(x, y)?;
        let spr = spread(alpha, beta, x, y);
        let z = zscore(&spr);

        plot_series_svg("demo_spread.svg", &format!("Spread {} / {}", a, b), &spr)?;
        plot_series_svg("demo_zscore.svg", &format!("Z-score {} / {}", a, b), &z)?;

        return Ok(());
    }

    // 7. Backtest + charts, one pair at a time.
    let mut results = Vec::<BacktestResult>::new();

    for (a, b, _, _) in &coint_pairs {
        // Raw closing prices for both legs of the pair, trimmed to the
        // shorter of the two (should already be equal length after
        // align_series above -- this is just a defensive re-check).
        let xa_raw = data.iter().find(|(n, _, _)| n == a).unwrap().1.clone();
        let yb_raw = data.iter().find(|(n, _, _)| n == b).unwrap().1.clone();
        let n = xa_raw.len().min(yb_raw.len());
        let xa_raw = xa_raw[..n].to_vec();
        let yb_raw = yb_raw[..n].to_vec();
        let hist_dates_raw = &hist_dates[..n];
        let timestamps_raw = &timestamps[..n];

        if n <= WALK_FORWARD_LOOKBACK + 60 {
            // Not enough history for a full formation period PLUS a
            // meaningfully long trading window afterward -- skip rather
            // than trade on a near-empty sample.
            println!(
                "Skipping {}/{}: only {} bars of aligned history, need > {} (a {}-bar formation period plus room to trade).",
                a, b, n, WALK_FORWARD_LOOKBACK + 60, WALK_FORWARD_LOOKBACK
            );
            continue;
        }

        // ---- Cointegration-consistent, walk-forward, log-price spread ----
        //
        // WHY LOG PRICES: universe/cointegration_scan.rs tests
        // `ln(price_a) = alpha + beta * ln(price_b)`, not raw prices --
        // using log prices is the standard convention for pairs trading
        // (Vidyamurthy, "Pairs Trading: Quantitative Methods and
        // Analysis," 2004, ch. 3) because it makes `beta` interpretable as
        // a relative-return elasticity and keeps the spread's scale
        // stable even as both stocks' price LEVELS drift apart over a
        // 10+ year sample (a raw-price spread between a $40 stock and a
        // $150 stock means something very different in 2012 than in
        // 2026). Trading a DIFFERENT spread definition than the one
        // actually tested for cointegration would mean the statistical
        // guarantee from the screening step doesn't really apply to what
        // gets traded -- an inconsistency an earlier version of this
        // pipeline had (raw-price hedge ratio used for trading, log-price
        // hedge ratio used only for screening). Fixed here by using log
        // prices in both places.
        //
        // WHY WALK-FORWARD: see the extensive comment on
        // model::spread::walk_forward_hedge_ratio. In one sentence: fitting
        // a single hedge ratio on the ENTIRE price history (as this
        // pipeline originally did) means even the EARLIEST simulated
        // trades use a beta that implicitly "knows" about price moves
        // that, at that point in simulated time, hadn't happened yet --
        // textbook look-ahead bias that makes backtests look better than
        // any real trader could have actually achieved. walk_forward_
        // hedge_ratio instead only ever fits on a trailing window of bars
        // that were already in the past at each point in time.
        let xa_log: Vec<f64> = xa_raw.iter().map(|p| p.ln()).collect();
        let yb_log: Vec<f64> = yb_raw.iter().map(|p| p.ln()).collect();
        let wf = walk_forward_hedge_ratio(&xa_log, &yb_log, WALK_FORWARD_LOOKBACK, WALK_FORWARD_REESTIMATE_EVERY);

        // Bars before WALK_FORWARD_LOOKBACK have no fitted hedge ratio yet
        // (walk_forward_hedge_ratio leaves them as NaN -- this is the
        // "formation period": there simply isn't a full lookback window of
        // real history behind them yet). Real, out-of-sample trading can
        // only start once that first full window exists, so every series
        // below is sliced to begin exactly there.
        let start = WALK_FORWARD_LOOKBACK;
        let spr: Vec<f64> = wf.spread[start..].to_vec();       // log-space spread, walk-forward, no look-ahead
        let hist_dates: Vec<NaiveDate> = hist_dates_raw[start..].to_vec();
        let timestamps: Vec<_> = timestamps_raw[start..].to_vec();
        let xa: Vec<f64> = xa_raw[start..].to_vec();            // RAW prices (post-formation) -- used for dollar PnL and price charts
        let yb: Vec<f64> = yb_raw[start..].to_vec();
        let (x, y) = (xa.as_slice(), yb.as_slice());

        // 10-day-vs-40-day rolling z-score on the walk-forward log spread
        // -- see README.md "Pipeline" step 3 for exactly what "1 SD" means
        // here (it's the spread's own trailing 40-day standard deviation,
        // recomputed at every bar).
        let rz = rolling_zscore(&spr, 10, 40);

        // ---- Half-life diagnostic and tradability gate ---------------
        // A pair can pass the cointegration test on borderline statistical
        // grounds while showing essentially no usable mean reversion in
        // practice -- cointegration asks "does this spread eventually come
        // back?", half-life asks "how fast, concretely?" A spread with no
        // measurable pull back toward its mean isn't a coherent
        // mean-reversion trade no matter how good its p-value looked, so
        // such pairs are skipped here rather than traded anyway. See
        // model::spread::half_life's doc comment for the full derivation.
        let hl = half_life(&spr);
        match hl {
            Some(h) => println!("{}/{}: half-life ≈ {:.1} bars", a, b, h),
            None => {
                println!(
                    "Skipping {}/{}: cointegrated by the ADF test, but the walk-forward spread shows no measurable mean reversion (half-life undefined) -- not a coherent mean-reversion trade regardless of the cointegration p-value.",
                    a, b
                );
                continue;
            }
        }

        let forecast_a = forecast_arma12(&xa, 5)?;
        let forecast_b = forecast_arma12(&yb, 5)?;

        // ---- 5-DAY FORECAST ----
        // Forecasts the log spread itself (same series the live z-score
        // and signals are built from, per the log-price discussion
        // above), then re-expresses that forecast in z-score units using
        // only the last 20 bars of REALIZED spread as the normalization
        // window (normalize_with_last_window) -- i.e. the forecast is
        // scored against "how unusual is this relative to what just
        // happened," not against the whole multi-year history.
        let forecast_spread = forecast_arma12(&spr, 5)?;
        let forecast_z = normalize_with_last_window(&spr, &forecast_spread);
        let forecast_sigs = generate_signals(&forecast_z, 0.2, 0.1);

        // --- Build forecast dates (5 days ahead) ---
        let last_date = hist_dates.last().unwrap();
        let forecast_dates: Vec<NaiveDate> = (1..=5)
            .map(|i| *last_date + chrono::Duration::days(i))
            .collect();

        // Convert to BUY/SELL instructions
        let forecast_trades: Vec<_> = forecast_sigs
            .iter()
            .map(|s| trade_direction(a, b, *s))
            .collect();

        println!("\n5-DAY FORECAST for {}-{}:", a, b);
        for i in 0..5 {
            if let Some((long_side, short_side)) = &forecast_trades[i] {
                let last_ts = timestamps.last().unwrap();
                let future_ts = (*last_ts + chrono::Duration::days((i + 1) as i64))
                    .format("%Y-%m-%d")
                    .to_string();
                println!("{} → {} / {}", future_ts, long_side, short_side);
            } else {
                println!("Day {} → FLAT", i + 1);
            }
        }

        // --- Two entry thresholds: aggressive (1.5 SD) and conservative
        // (2.0 SD) -- each backtested and charted independently so they
        // can be compared side by side and ranked.
        let thresholds: [(f64, f64); 2] = [(1.5, 0.5), (2.0, 0.5)];
        let mut pair_bts = Vec::<BacktestResult>::new();
        let mut pair_charts = Vec::<(String, String)>::new();

        for &(entry, exit) in &thresholds {
            let sigs = generate_signals(&rz, entry, exit);

            // --- Historical per-stock signals ---
            let mut stock_sigs = per_stock_signals(&sigs, &xa, &yb);
            while stock_sigs.len() < xa.len() {
                stock_sigs.insert(0, StockSignal::Flat);
            }

            let mut forecast_stock_sigs = per_stock_signals(&forecast_sigs, &forecast_a, &forecast_b);
            while forecast_stock_sigs.len() < forecast_a.len() {
                forecast_stock_sigs.insert(0, StockSignal::Flat);
            }

            let tag = format!("{}sd", entry).replace('.', "_");
            let left_png = format!("output/{}_{}_{}_signals.png", a, b, tag);
            let right_png = format!("output/{}_{}_{}_prices.png", a, b, tag);

            plot_signals_with_forecast_svg(
                &left_png,
                &format!("Signals for {}-{} ({} SD entry)", a, b, entry),
                &hist_dates,
                &rz,
                &sigs,
                &forecast_dates,
                &forecast_z,
                &forecast_sigs,
            )?;

            plot_prices_with_signals_svg(
                &right_png,
                &format!("Price + Signals for {}-{} ({} SD entry)", a, b, entry),
                &hist_dates,
                &xa,
                &yb,
                &stock_sigs,
                &forecast_dates,
                &forecast_a,
                &forecast_b,
                &forecast_stock_sigs,
            )?;

            pair_charts.push((format!("{} SD Signals", entry), left_png));
            pair_charts.push((format!("{} SD Prices", entry), right_png));

            // --- Backtest ---
            // Notional=1.0, cost=0.02% (2 bps) per leg per position change
            // -- see engine/backtest.rs for the full cost-model writeup.
            // Note x/y here are the RAW price legs (not log prices): the
            // signal that decides WHEN to trade comes from the log-space
            // spread above, but the simulated dollar P&L from actually
            // holding those positions is computed from real price changes,
            // which is what backtest_pair expects.
            let mut bt = backtest_pair(x, y, &sigs, 1.0, 0.0002);
            bt.pair = (a.clone(), b.clone());
            bt.threshold = entry;
            bt.half_life_bars = hl;
            pair_bts.push(bt);
        }

        // --- Equity curve: 1.5 SD vs 2.0 SD side by side for this pair ---
        let equity_png = format!("output/{}_{}_equity.png", a, b);
        plot_equity_curves_svg(
            &equity_png,
            &format!("Equity curve for {}-{}", a, b),
            &hist_dates,
            &pair_bts[0].equity_curve,
            &format!("{} SD (Sharpe {:.2})", pair_bts[0].threshold, pair_bts[0].sharpe_ratio),
            &pair_bts[1].equity_curve,
            &format!("{} SD (Sharpe {:.2})", pair_bts[1].threshold, pair_bts[1].sharpe_ratio),
        )?;
        pair_charts.push(("Equity Curve".to_string(), equity_png));

        // --- Strategy summary (position-sizing sanity check) -----------
        // A simple parametric 95% Value-at-Risk figure (Jorion, "Value at
        // Risk," standard risk-management reference): VaR_95% = 1.645 *
        // sigma, where 1.645 is the 95th-percentile z-score of a standard
        // normal distribution and sigma is the ONE-DAY dollar P&L standard
        // deviation. `volatility` on BacktestResult is already this same
        // number annualized (multiplied by sqrt(252) -- see engine/
        // backtest.rs), so it's un-annualized here by dividing back out,
        // rather than re-deriving a separate ad hoc "spread volatility" as
        // an earlier version of this summary did (which mixed a raw-price,
        // full-sample, look-ahead-biased spread statistic into an
        // otherwise dollar-denominated risk figure -- inconsistent units,
        // and inconsistent with the walk-forward spread used everywhere
        // else above).
        let primary = &pair_bts[0]; // 1.5 SD (the more active) threshold
        let daily_dollar_vol = primary.volatility / 252.0_f64.sqrt();
        let var_95 = 1.645 * daily_dollar_vol;
        println!(
            "{} / {} | side: mean-reversion | half-life: {} | daily P&L vol: {:.4} | VaR95 (1-day): {:.4}",
            a, b,
            hl.map(|h| format!("{:.1} bars", h)).unwrap_or_else(|| "n/a".to_string()),
            daily_dollar_vol, var_95
        );

        // Capture the full equity path (dates + values) for each threshold
        // before pair_bts is moved into `results` below -- this is the
        // data a real interactive chart needs; the summary stats alone
        // (Sharpe, max drawdown, ...) can't reconstruct the day-by-day
        // line.
        for bt in &pair_bts {
            equity_curve_entries.push(EquityCurveEntry {
                pair: format!("{} / {}", a, b),
                threshold: bt.threshold,
                sharpe: bt.sharpe_ratio,
                sortino: bt.sortino_ratio,
                half_life_bars: bt.half_life_bars,
                max_drawdown: bt.max_drawdown,
                net_pnl: bt.total_pnl,
                volatility: bt.volatility,
                trades: bt.trades,
                dates: hist_dates.clone(),
                equity: bt.equity_curve.clone(),
            });
        }

        chart_groups.push((format!("{} / {}", a, b), pair_charts));
        results.extend(pair_bts);
    }

    // 8. Rank pairs best-to-trade by Sharpe ratio (descending) so the same
    // ordering drives both the CSV export and the dashboard table.
    results.sort_by(|a, b| {
        b.sharpe_ratio
            .partial_cmp(&a.sharpe_ratio)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    println!("\nPairs ranked best to trade (by Sharpe ratio):");
    for (i, r) in results.iter().enumerate() {
        println!(
            "{:>2}. {} / {} | entry={:.1}SD | rf={:.2}% | vol={:.4} | Sharpe={:.3} | Sortino={:.3} | MaxDD={:.4} | Cost={:.4} | NetPnL={:.4} | trades={}",
            i + 1, r.pair.0, r.pair.1, r.threshold, r.risk_free_rate * 100.0, r.volatility,
            r.sharpe_ratio, r.sortino_ratio, r.max_drawdown, r.total_costs, r.total_pnl, r.trades
        );
    }

    // 9. Dashboard -- order each pair's chart group to match the ranking
    // above (best pair first), so the page reads best-to-worst top to
    // bottom, same as the ranking table sitting above it.
    let mut pair_rank_order = Vec::<String>::new();
    let mut seen_pairs = std::collections::HashSet::new();
    for r in &results {
        let label = format!("{} / {}", r.pair.0, r.pair.1);
        if seen_pairs.insert(label.clone()) {
            pair_rank_order.push(label);
        }
    }
    let ordered_chart_groups: Vec<(String, Vec<(String, String)>)> = pair_rank_order
        .iter()
        .filter_map(|label| {
            chart_groups
                .iter()
                .find(|(group_label, _)| group_label == label)
                .cloned()
        })
        .collect();

    export_full_dashboard("output/dashboard.html", &ordered_chart_groups, &results)?;
    println!("Dashboard saved to output/dashboard.html");

    // 10. Export backtests
    println!("About to write {} backtests to output/backtests_energy.csv", results.len());
    std::io::stdout().flush().ok();
    export_backtests("output/backtests_energy.csv", &results)?;
    println!("Exported {} backtests to output/backtests_energy.csv", results.len());

    // 11. Export full equity curves as JSON, ordered to match the ranking
    // table (best Sharpe first), for interactive/dynamic charts.
    equity_curve_entries.sort_by(|a, b| {
        b.sharpe.partial_cmp(&a.sharpe).unwrap_or(std::cmp::Ordering::Equal)
    });
    export_equity_curves("output/equity_curves.json", &equity_curve_entries)?;
    println!("Exported {} equity curves to output/equity_curves.json", equity_curve_entries.len());

    Ok(())
}
