mod data;
mod model;
mod engine;
mod universe;

use anyhow::Result;
use universe::energy_list::energy_universe;
use universe::fetch_all::fetch_universe;
use universe::correlation::correlation_matrix;
use universe::cointegration_scan::scan_cointegration;
use model::spread::{hedge_ratio_ols, spread, zscore, rolling_zscore, std_spread};
use engine::signals::generate_signals;
use engine::backtest::{backtest_pair, BacktestResult};
use universe::export::export_backtests;
use engine::align::align_series;
use universe::plots::{plot_series_svg, plot_zscore_with_signals_svg};
use universe::export_html::export_svg_dashboard;

#[tokio::main]
async fn main() -> Result<()> {

    // Collect SVG paths for dashboard
    let mut svg_files: Vec<String> = Vec::new();

    // 1. Universe identical to Python
    let tickers = energy_universe();

    // 2. Fetch all data (Yahoo, adjusted close)
    let mut data = fetch_universe(&tickers).await?;

    // 3. Align all series by length
    data = align_series(&data);

    // 4. Correlation matrix
    let corrs = correlation_matrix(&data);
    println!("Top correlated pairs:");
    for (a, b, c) in corrs.iter().take(5) {
        println!("{}/{} corr={:.3}", a, b, c);
    }

    // 5. Cointegration scan
    let coint = scan_cointegration(&data);
    let coint_pairs: Vec<_> = coint.into_iter()
        .filter(|(_, _, _, is_c)| *is_c)
        .collect();

    println!("Cointegrated pairs:");
    for (a, b, adf, _) in &coint_pairs {
        println!("{}/{} ADF={:.3}", a, b, adf);
    }

    // 6. If no cointegrated pairs → demo plots
    if coint_pairs.is_empty() {
        println!("No cointegrated pairs found — generating demo plots for top correlated pair.");

        let (a, b, _) = &corrs[0];

        let xa = data.iter().find(|(n, _)| n == a).unwrap().1.clone();
        let yb = data.iter().find(|(n, _)| n == b).unwrap().1.clone();
        let n = xa.len().min(yb.len());
        let (x, y) = (&xa[..n], &yb[..n]);

        let (alpha, beta) = hedge_ratio_ols(x, y)?;
        let spr = spread(alpha, beta, x, y);
        let z = zscore(&spr);

        plot_series_svg("demo_spread.svg", &format!("Spread {} / {}", a, b), &spr)?;
        plot_series_svg("demo_zscore.svg", &format!("Z-score {} / {}", a, b), &z)?;

        println!("Generated demo_spread.svg and demo_zscore.svg");
        return Ok(());
    }

    // 7. Backtest cointegrated pairs
    let mut results = Vec::<BacktestResult>::new();

    for (a, b, _, _) in &coint_pairs {
        let xa = data.iter().find(|(n, _)| n == a).unwrap().1.clone();
        let yb = data.iter().find(|(n, _)| n == b).unwrap().1.clone();
        let n = xa.len().min(yb.len());
        let (x, y) = (&xa[..n], &yb[..n]);

        let (alpha, beta) = hedge_ratio_ols(x, y)?;
        let spr = spread(alpha, beta, x, y);
        let z = zscore(&spr);
        let rz = rolling_zscore(&spr, 5, 20);
        let sigs = generate_signals(&rz, 1.0, 0.5);

        let filename_prefix = format!("{}_{}", a, b);

        // Spread chart
        let spread_path = format!("output/spread_{}.svg", filename_prefix);
        plot_series_svg(&spread_path, &format!("Spread {} / {}", a, b), &spr)?;
        svg_files.push(spread_path);

        // Z-score chart
        let zscore_path = format!("output/zscore_{}.svg", filename_prefix);
        plot_series_svg(&zscore_path, &format!("Z-score {} / {}", a, b), &z)?;
        svg_files.push(zscore_path);

        // Signals chart
        let sig_path = format!("output/signals_{}.svg", filename_prefix);
        plot_zscore_with_signals_svg(&sig_path, &format!("Signals {} / {}", a, b), &rz, &sigs)?;
        svg_files.push(sig_path);

        // Backtest
        let mut bt = backtest_pair(x, y, &sigs, 1.0);
        bt.pair = (a.clone(), b.clone());
        results.push(bt);
    }

    // 8. Strategy summary
    println!("\nStrategy summary:");
    for (a, b, _, _) in &coint_pairs {
        let xa = data.iter().find(|(n, _)| n == a).unwrap().1.clone();
        let yb = data.iter().find(|(n, _)| n == b).unwrap().1.clone();
        let n = xa.len().min(yb.len());
        let (x, y) = (&xa[..n], &yb[..n]);

        let (alpha, beta) = hedge_ratio_ols(x, y)?;
        let spr = spread(alpha, beta, x, y);
        let sd = std_spread(&spr);

        let notional = 1.0;
        let var_95 = 1.65 * sd * notional;
        let last_spread = *spr.last().unwrap();

        println!(
            "{} / {} | side: mean-reversion | notional: {:.2} | last_spread: {:.4} | VaR95: {:.4}",
            a, b, notional, last_spread, var_95
        );
    }

    // 9. Export backtests
    export_backtests("backtests_energy.csv", &results)?;
    println!("Exported {} backtests to backtests_energy.csv", results.len());

    // 10. Export dashboard
    export_svg_dashboard("output/dashboard.html", &svg_files)?;
    println!("Dashboard saved to output/dashboard.html");

    Ok(())
}
