// src/universe/export.rs
use anyhow::Result;
use crate::engine::backtest::BacktestResult;

pub fn export_backtests(path: &str, results: &[BacktestResult]) -> Result<()> {
    let mut wtr = csv::Writer::from_path(path)?;
    wtr.write_record(&["stock1", "stock2", "total_pnl", "trades"])?;
    for r in results {
        wtr.write_record(&[
            &r.pair.0,
            &r.pair.1,
            &r.total_pnl.to_string(),
            &r.trades.to_string(),
        ])?;
    }
    wtr.flush()?;
    Ok(())
}
