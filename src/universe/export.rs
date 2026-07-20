// src/universe/export.rs
use anyhow::Result;
use crate::engine::backtest::BacktestResult;

pub fn export_backtests(path: &str, results: &[BacktestResult]) -> Result<()> {
    let mut wtr = csv::Writer::from_path(path)?;
    // Export both gross and total costs so users can see gross vs net PnL
    wtr.write_record(&["stock1", "stock2", "gross_pnl", "total_costs", "net_pnl", "trades"])?;
    for r in results {
        // gross_pnl = net_pnl + total_costs
        let gross = r.total_pnl + r.total_costs;
        wtr.write_record(&[
            &r.pair.0,
            &r.pair.1,
            &gross.to_string(),
            &r.total_costs.to_string(),
            &r.total_pnl.to_string(),
            &r.trades.to_string(),
        ])?;
    }
    wtr.flush()?;
    Ok(())
}
