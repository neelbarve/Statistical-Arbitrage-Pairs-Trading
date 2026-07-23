// src/engine/backtest.rs
//use anyhow::Result;
use super::signals::Signal;

#[derive(Debug, Clone)]
pub struct BacktestResult {
    pub pair: (String, String),
    pub total_pnl: f64,
    pub trades: usize,
}

pub fn backtest_pair(
    x: &[f64],
    y: &[f64],
    signals: &[Signal],
    notional: f64,
) -> BacktestResult {
    let n = x.len().min(y.len()).min(signals.len());
    let mut pnl = 0.0;
    let mut trades = 0usize;
    let mut prev_sig = Signal::Flat;

    for i in 1..n {
        let sig = signals[i];
        if !matches!(sig, Signal::Flat) && matches!(prev_sig, Signal::Flat) {
            trades += 1;
        }

        let dx = x[i] - x[i-1];
        let dy = y[i] - y[i-1];

        match sig {
            Signal::LongSpread => {
                pnl += notional * (dx - dy);
            }
            Signal::ShortSpread => {
                pnl += notional * (-dx + dy);
            }
            Signal::Flat => {}
        }

        prev_sig = sig;
    }

    BacktestResult {
        pair: ("".into(), "".into()), // filled by caller
        total_pnl: pnl,
        trades,
    }
}
