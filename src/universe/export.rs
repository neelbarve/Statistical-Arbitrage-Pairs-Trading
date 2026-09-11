// src/universe/export.rs
use anyhow::Result;
use crate::engine::backtest::BacktestResult;

// `results` is expected to already be sorted best-to-trade first (see the
// `results.sort_by(...)` call in main.rs, which sorts by Sharpe ratio
// descending) -- each row's position in that order becomes its `rank`
// column below.
pub fn export_backtests(path: &str, results: &[BacktestResult]) -> Result<()> {
    let mut wtr = csv::Writer::from_path(path)?;
    // Column order intentionally mirrors the dashboard's ranking table
    // (see universe/export_html.rs) so a reader flipping between the CSV
    // and the HTML dashboard sees the same layout in both places.
    wtr.write_record(&[
        "rank", "stock1", "stock2", "entry_threshold_sd", "half_life_bars",
        "gross_pnl", "total_costs", "net_pnl", "risk_free_rate", "volatility",
        "sharpe_ratio", "sortino_ratio", "max_drawdown", "trades",
    ])?;
    for (i, r) in results.iter().enumerate() {
        // gross_pnl = net_pnl + total_costs (net_pnl already has costs
        // subtracted -- see engine/backtest.rs)
        let gross = r.total_pnl + r.total_costs;
        // half_life_bars is a diagnostic of the underlying spread, not a
        // per-threshold quantity -- it's the same number for both this
        // pair's 1.5 SD and 2.0 SD rows. "n/a" only happens if a pair
        // somehow reached export without ever getting a half-life gate
        // check (shouldn't occur via the normal main.rs pipeline, which
        // skips such pairs before they're ever backtested -- see
        // model::spread::half_life).
        let half_life_str = match r.half_life_bars {
            Some(h) => format!("{:.2}", h),
            None => "n/a".to_string(),
        };
        wtr.write_record(&[
            (i + 1).to_string(),
            r.pair.0.clone(),
            r.pair.1.clone(),
            format!("{:.2}", r.threshold),
            half_life_str,
            gross.to_string(),
            r.total_costs.to_string(),
            r.total_pnl.to_string(),
            format!("{:.4}", r.risk_free_rate),
            format!("{:.4}", r.volatility),
            format!("{:.4}", r.sharpe_ratio),
            format!("{:.4}", r.sortino_ratio),
            format!("{:.4}", r.max_drawdown),
            r.trades.to_string(),
        ])?;
    }
    wtr.flush()?;
    Ok(())
}
