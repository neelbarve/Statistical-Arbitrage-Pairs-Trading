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
use model::spread::{hedge_ratio_ols, spread, zscore, rolling_zscore, std_spread};
use engine::backtest::{backtest_pair, BacktestResult};
use engine::align::align_series;
use universe::export::export_backtests;
use universe::export_html::export_full_dashboard;
use universe::plots::{plot_series_svg, plot_prices_with_signals_svg, plot_signals_with_forecast_svg, plot_equity_curves_svg};
use crate::engine::forecast::{normalize_with_last_window};
use crate::engine::signals::{generate_signals, trade_direction};
use chrono::NaiveDate;
use crate::model::forecast::forecast_arma12;
use crate::engine::signals::{per_stock_signals, StockSignal};


//use crate::engine::signals::per_stock_signals_from_spread;










#[tokio::main]
async fn main() -> Result<()> {

    // Charts grouped per pair (pair label -> [(chart title, path), ...]) so
    // the dashboard can lay out each pair's charts side by side instead of
    // interleaving every pair into one flat grid.
    let mut chart_groups = Vec::<(String, Vec<(String, String)>)>::new();

    // Create output folder if it doesn't exist
    std::fs::create_dir_all("output")?;
    //println!("Output folder ready.");

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

    // 5. Cointegration
    let coint = scan_cointegration(&data);
    let coint_pairs: Vec<_> = coint.into_iter()
        .filter(|(_, _, _, is_c)| *is_c)
        .collect();

    println!("Cointegrated pairs:");
    for (a, b, adf, _) in &coint_pairs {
        println!("{}/{} ADF={:.3}", a, b, adf);
    }

    // 6. Demo if empty
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

    // 7. Backtest + charts
    let mut results = Vec::<BacktestResult>::new();

    for (a, b, _, _) in &coint_pairs {
        let xa = data.iter().find(|(n, _, _)| n == a).unwrap().1.clone();
        let yb = data.iter().find(|(n, _, _)| n == b).unwrap().1.clone();
        let n = xa.len().min(yb.len());
        let (x, y) = (&xa[..n], &yb[..n]);

        let (alpha, beta) = hedge_ratio_ols(x, y)?;
        let spr = spread(alpha, beta, x, y);
        let _z = zscore(&spr);
        let rz = rolling_zscore(&spr, 10, 40);

        let forecast_A = forecast_arma12(&xa, 5)?;
        let forecast_B = forecast_arma12(&yb, 5)?;


                // ---- 5-DAY FORECAST ----
        let forecast_spread = forecast_arma12(&spr, 5)?;
        //let forecast_spread = forecast_spread;
        let forecast_z = normalize_with_last_window(&spr, &forecast_spread);
        let forecast_sigs = generate_signals(&forecast_z, 0.2, 0.1);



        // --- STEP 3: Build forecast dates (5 days ahead) ---
        let last_date = hist_dates.last().unwrap();
        let forecast_dates: Vec<NaiveDate> = (1..=5)
            .map(|i| *last_date + chrono::Duration::days(i))
            .collect();


        // Convert to BUY/SELL instructions
        let forecast_trades: Vec<_> = forecast_sigs
            .iter()
            .map(|s| trade_direction(a, b, *s))
            .collect();

        // Print forecast for debugging
        println!("\n5-DAY FORECAST for {}-{}:", a, b);
        for i in 0..5 {
            if let Some((long_side, short_side)) = &forecast_trades[i] {

                let last_ts = timestamps.last().unwrap();
                let future_ts = (*last_ts + chrono::Duration::days((i+1) as i64))
                    .format("%Y-%m-%d")
                    .to_string();

                println!("{} → {} / {}", future_ts, long_side, short_side);

            } else {
                println!("Day {} → FLAT", i + 1);
            }
        }

        // --- Two entry thresholds: aggressive (1.5 SD) and conservative (2.0 SD) ---
        // Each is backtested and charted independently so they can be
        // compared side by side and ranked.
        let thresholds: [(f64, f64); 2] = [(1.5, 0.5), (2.0, 0.5)];
        let mut pair_bts = Vec::<BacktestResult>::new();
        let mut pair_charts = Vec::<(String, String)>::new();

        for &(entry, exit) in &thresholds {
            let sigs = generate_signals(&rz, entry, exit);

            // --- Historical per-stock signals ---
            let mut stock_sigs = per_stock_signals(&sigs, &xa, &yb);
            // --- FIX: pad signals to match price length ---
            while stock_sigs.len() < xa.len() {
                stock_sigs.insert(0, StockSignal::Flat);
            }

            let mut forecast_stock_sigs = per_stock_signals(&forecast_sigs, &forecast_A, &forecast_B);
            while forecast_stock_sigs.len() < forecast_A.len() {
                forecast_stock_sigs.insert(0, StockSignal::Flat);
            }

            let tag = format!("{}sd", entry).replace('.', "_");
            let left_png = format!("output/{}_{}_{}_signals.png", a, b, tag);
            let right_png = format!("output/{}_{}_{}_prices.png", a, b, tag);

            // Debug prints
            println!("[{} SD] hist_dates.len() = {}", entry, hist_dates.len());
            println!("[{} SD] xa.len() = {}", entry, xa.len());
            println!("[{} SD] yb.len() = {}", entry, yb.len());
            println!("[{} SD] sigs.len() = {}", entry, sigs.len());
            println!("[{} SD] stock_sigs.len() = {}", entry, stock_sigs.len());
            println!("[{} SD] forecast_A.len() = {}", entry, forecast_A.len());
            println!("[{} SD] forecast_sigs.len() = {}", entry, forecast_sigs.len());
            println!("[{} SD] forecast_stock_sigs.len() = {}", entry, forecast_stock_sigs.len());

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
                &forecast_A,
                &forecast_B,
                &forecast_stock_sigs,
            )?;

            pair_charts.push((format!("{} SD Signals", entry), left_png));
            pair_charts.push((format!("{} SD Prices", entry), right_png));

            // --- Backtest ---
            // Use brokerage charge of 0.02% per leg per transition (0.0002)
            let mut bt = backtest_pair(x, y, &sigs, 1.0, 0.0002);
            bt.pair = (a.clone(), b.clone());
            bt.threshold = entry;
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
            "{:>2}. {} / {} | entry={:.1}SD | rf={:.2}% | vol={:.4} | Sharpe={:.3} | MaxDD={:.4} | Cost={:.4} | NetPnL={:.4} | trades={}",
            i + 1, r.pair.0, r.pair.1, r.threshold, r.risk_free_rate * 100.0, r.volatility, r.sharpe_ratio, r.max_drawdown, r.total_costs, r.total_pnl, r.trades
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

    // 10. Strategy summary
    println!("\nStrategy summary:");
    for (a, b, _, _) in &coint_pairs {
        let xa = data.iter().find(|(n, _, _)| n == a).unwrap().1.clone();
        let yb = data.iter().find(|(n, _, _)| n == b).unwrap().1.clone();
        let n = xa.len().min(yb.len());
        let (x, y) = (&xa[..n], &yb[..n]);

        let (alpha, beta) = hedge_ratio_ols(x, y)?;
        let spr = spread(alpha, beta, x, y);
        let sd = std_spread(&spr);

        let notional = 1.5*sd ;  // target 1.5 SD move in spread
        let var_95 = 1.65 * notional;
        let last_spread = *spr.last().unwrap();

        println!(
            "{} / {} | side: mean-reversion | notional: {:.2} | last_spread: {:.4} | VaR95: {:.4}",
            a, b, notional, last_spread, var_95
        );
    }

    // 11. Export backtests
    println!("About to write {} backtests to output/backtests_energy.csv", results.len());
    std::io::stdout().flush().ok();
    export_backtests("output/backtests_energy.csv", &results)?;
    println!("Exported {} backtests to output/backtests_energy.csv", results.len());

    Ok(())
}
