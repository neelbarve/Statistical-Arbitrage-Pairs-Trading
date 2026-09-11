// src/universe/export.rs
use anyhow::Result;
use crate::engine::backtest::BacktestResult;

// `results` is expected to already be sorted best-to-trade first (see
// `rank_by_sharpe` in main.rs) -- the row's position becomes its `rank`.
pub fn export_backtests(path: &str, results: &[BacktestResult]) -> Result<()> {
    let mut wtr = csv::Writer::from_path(path)?;
    // Export both gross and total costs so users can see gross vs net PnL
    wtr.write_record(&[
        "rank", "stock1", "stock2", "entry_threshold_sd", "gross_pnl", "total_costs",
        "net_pnl", "risk_free_rate", "volatility", "sharpe_ratio", "max_drawdown", "trades",
    ])?;
    for (i, r) in results.iter().enumerate() {
        // gross_pnl = net_pnl + total_costs
        let gross = r.total_pnl + r.total_costs;
        wtr.write_record(&[
            (i + 1).to_string(),
            r.pair.0.clone(),
            r.pair.1.clone(),
            format!("{:.2}", r.threshold),
            gross.to_string(),
            r.total_costs.to_string(),
            r.total_pnl.to_string(),
            format!("{:.4}", r.risk_free_rate),
            format!("{:.4}", r.volatility),
            format!("{:.4}", r.sharpe_ratio),
            format!("{:.4}", r.max_drawdown),
            r.trades.to_string(),
        ])?;
    }
    wtr.flush()?;
    Ok(())
}
