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
use universe::plots::{plot_series_svg, plot_prices_with_signals_svg, plot_signals_with_forecast_svg};
use crate::engine::forecast::{normalize_with_last_window};
use crate::engine::signals::{generate_signals, trade_direction};
use chrono::NaiveDate;
use crate::model::forecast::forecast_arma12;
use crate::engine::signals::{per_stock_signals, StockSignal};


//use crate::engine::signals::per_stock_signals_from_spread;










#[tokio::main]
async fn main() -> Result<()> {

    let mut chart_files = Vec::<String>::new();

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
        let sigs = generate_signals(&rz, 1.5, 0.5);

        let forecast_A = forecast_arma12(&xa, 5)?;
        let forecast_B = forecast_arma12(&yb, 5)?;


                // ---- 5-DAY FORECAST ----
        let forecast_spread = forecast_arma12(&spr, 5)?;
        //let forecast_spread = forecast_spread; 
        let forecast_z = normalize_with_last_window(&spr, &forecast_spread);
        let forecast_sigs = generate_signals(&forecast_z, 0.2, 0.1);


        // --- Historical per-stock signals ---
        let mut stock_sigs = per_stock_signals(&sigs, &xa, &yb);
        // --- FIX: pad signals to match price length ---
        while stock_sigs.len() < xa.len() {
            stock_sigs.insert(0, StockSignal::Flat);
        }

        // --- Compute per-stock forecast signals ---
        // let forecast_stock_sigs = per_stock_signals(
        //     &forecast_sigs,       
        //     &forecast_A,
        //     &forecast_B,
        // );
        let mut forecast_stock_sigs = per_stock_signals(&forecast_sigs, &forecast_A, &forecast_B);
        while forecast_stock_sigs.len() < forecast_A.len() {
            forecast_stock_sigs.insert(0, StockSignal::Flat);
        }





        // --- STEP 3: Build forecast dates (5 days ahead) ---
        let last_date = hist_dates.last().unwrap();
        let forecast_dates: Vec<NaiveDate> = (1..=5)
            .map(|i| *last_date + chrono::Duration::days(i))
            .collect();


        //let out_path_signals = format!("output/{}_{}_signals.svg", a, b);
        let left_png= format!("output/{}_{}_signals.png", a, b);
        let right_png = format!("output/{}_{}_prices.png", a, b);
        //let out_path_prices  = format!("output/{}_{}_prices.svg", a, b);


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


        // Debug prints 
        println!("hist_dates.len() = {}", hist_dates.len());
        println!("xa.len() = {}", xa.len());
        println!("yb.len() = {}", yb.len());
        println!("sigs.len() = {}", sigs.len());
        println!("stock_sigs.len() = {}", stock_sigs.len());
        //println!("Non-flat stock_sigs = {}", stock_sigs.iter().filter(|s| **s == StockSignal::Flat).count());

        println!("forecast_A.len() = {}", forecast_A.len());
        println!("forecast_sigs.len() = {}", forecast_sigs.len());
        println!("forecast_stock_sigs.len() = {}", forecast_stock_sigs.len());
        //println!("Non-flat forecast_stock_sigs = {}", forecast_stock_sigs.iter().filter(|s| **s == StockSignal::Flat).count());


        plot_signals_with_forecast_svg(
            &left_png,
            &format!("Signals for {}-{}", a, b),
            &hist_dates,
            &rz,
            &sigs,
            &forecast_dates,
            &forecast_z,
            &forecast_sigs,
        )?;

        // pub fn plot_signals_with_forecast_area(
        //     area: &DrawingArea<SVGBackend, Shift>,
        //     title: &str,
        //     dates_hist: &[NaiveDate],
        //     rz: &[f64],
        //     sigs: &[Signal],
        //     dates_fore: &[NaiveDate],
        //     forecast_z: &[f64],
        //     forecast_sigs: &[Signal],
        // ) {
        //     let root = area.clone();
        //     let _ = root.fill(&WHITE);

        //     // same logic as your SVG version
        // }


        plot_prices_with_signals_svg(
            &right_png,
            &format!("Price + Signals for {}-{}", a, b),
            &hist_dates,
            &xa,
            &yb,
            &stock_sigs,
            &forecast_dates,
            &forecast_A,
            &forecast_B,
            &forecast_stock_sigs,
        )?;

        chart_files.push(left_png.clone());
        chart_files.push(right_png.clone());

        // let combined_path = format!("output/{}_{}_combined.svg", a, b);
        // plot_two_charts_side_by_side(
        //     &combined_path,
        //     &out_path_signals,
        //     &out_path_prices,
        // )?;

        // let combined_svg = format!("output/{}_{}_combined.svg", a, b);
        // combine_two_pngs_side_by_side(
        //     &combined_svg,
        //     &left_png,
        //     &right_png,
        // )?;

        let _combined_path = format!("output/{}_{}_combined.svg", a, b);

        // plot_two_charts_side_by_side(
        //     &_combined_path,
        //     |area| {
        //         plot_signals_with_forecast_area(
        //             area,
        //             &format!("Signals for {}-{}", a, b),
        //             &hist_dates,
        //             &rz,
        //             &sigs,
        //             &forecast_dates,
        //             &forecast_z,
        //             &forecast_sigs,
        //         );
        //     },
        //     |area| {
        //         plot_prices_with_signals_area(
        //             area,
        //             &format!("Price + Signals for {}-{}", a, b),
        //             &hist_dates,
        //             &xa,
        //             &yb,
        //             &stock_sigs,
        //             &forecast_dates,
        //             &forecast_A,
        //             &forecast_B,
        //             &forecast_stock_sigs,
        //         );
        //     },
        // )?;



        //println!("Saved chart to {} and {}", out_path_signals, out_path_prices);
        //println!("Saved charts to {}, {}, and {}", left_png, right_png, combined_svg);

        // Print trade directions for debugging
        // for i in 0..sigs.len() {
        //     if let Some((long_side, short_side)) = trade_direction(a, b, sigs[i]) {
                
        //         let ts = timestamps[i].format("%Y-%m-%d %H:%M").to_string();
        //         println!(
        //             "Pair {}-{} | {} | {} / {}",
        //             a, b, ts, long_side, short_side
        //         );

        //     }
        // }
        let _filename_prefix = format!("{}_{}", a, b);

        // --- FIXED: define paths ---
        //let spread_path = format!("output/spread_{}.svg", _filename_prefix);
        //let zscore_path = format!("output/zscore_{}.svg", filename_prefix);
        //let sig_path = format!("output/signals_{}.svg", filename_prefix);

        // --- Generate charts ---
        //plot_series_svg(&spread_path, &format!("Spread {} / {}", a, b), &spr)?;
        //plot_series_svg(&zscore_path, &format!("Z-score {} / {}", a, b), &z)?;
        //plot_zscore_with_signals_svg(&sig_path, &format!("Signals {} / {}", a, b), &rz, &sigs)?;

        //std::thread::sleep(std::time::Duration::from_millis(30));


        // --- Collect for dashboard ---
        //svg_files.push(spread_path);
        //svg_files.push(zscore_path);
        //svg_files.push(sig_path);

        //std::thread::sleep(std::time::Duration::from_millis(30));

        // --- Backtest ---
        // Use brokerage charge of 0.02% per leg per transition (0.0002)
        let mut bt = backtest_pair(x, y, &sigs, 1.0, 0.0002);
        bt.pair = (a.clone(), b.clone());
        results.push(bt);
    }

    // 8. Dashboard
    //export_svg_dashboard("output/dashboard.html", &svg_files)?;
    //println!("Dashboard saved to output/dashboard.html");

    // for i in 0..sigs.len() {
    // if let Some((long_side, short_side)) = trade_direction(a, b, sigs[i]) {
    //     println!(
    //         "Pair {}-{} | t={} | {} / {}",
    //         a, b, i, long_side, short_side
    //     );
    // }
    //

    


    // 9. Strategy summary
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

    // 10. Export backtests
    println!("About to write {} backtests to output/backtests_energy.csv", results.len());
    std::io::stdout().flush().ok();
    export_backtests("output/backtests_energy.csv", &results)?;
    println!("Exported {} backtests to output/backtests_energy.csv", results.len());

    Ok(())
}
