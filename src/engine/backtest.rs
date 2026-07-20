// Drop-in replacement for the original engine/backtest.rs, extended with
// transaction costs. The ORIGINAL had no cost model at all (confirmed
// directly from the cloned repo -- see docs/NOTES.md), which for a
// mean-reversion strategy trading small deviations is a serious gap: a
// strategy that looks profitable purely because it's trading noise (with
// no cost to penalize excessive trading) is a classic false positive.
//
// Cost model: every time the position CHANGES (Flat -> LongSpread,
// LongSpread -> Flat, Flat -> ShortSpread, ShortSpread -> Flat, or a
// direct flip between LongSpread and ShortSpread), a cost is charged on
// BOTH legs of the pair (you cross the bid-ask spread on each stock every
// time you trade it), proportional to notional and a cost-in-basis-points
// assumption. This is a simple, standard, and conservative fixed-cost
// model (does not model market impact or partial fills) -- a reasonable
// first-order assumption, not meant to be the final word on realistic
// costs, and documented as such.
//
// IMPORTANT PROPERTY, verified by test below: setting cost_bps = 0.0
// reproduces the EXACT same total_pnl the original (cost-free) function
// produced. This is what makes this a true extension rather than an
// accidental behavior change to the existing, already-working code path.
use super::signals::Signal;

#[derive(Debug, Clone)]
pub struct BacktestResult {
    pub pair: (String, String),
    pub total_pnl: f64,
    pub total_costs: f64,   // NEW: reported separately so gross vs net PnL is always visible
    pub trades: usize,
}

pub fn backtest_pair(
    x: &[f64],
    y: &[f64],
    signals: &[Signal],
    notional: f64,
    cost_bps: f64,  // e.g. 0.0005 = 5 basis points per leg per transition
) -> BacktestResult {
    let n = x.len().min(y.len()).min(signals.len());
    let mut pnl = 0.0;
    let mut total_costs = 0.0;
    let mut trades = 0usize;
    let mut prev_sig = Signal::Flat;

    for i in 1..n {
        let sig = signals[i];
        if !matches!(sig, Signal::Flat) && matches!(prev_sig, Signal::Flat) {
            trades += 1;
        }

        let dx = x[i] - x[i - 1];
        let dy = y[i] - y[i - 1];

        match sig {
            Signal::LongSpread => {
                pnl += notional * (dx - dy);
            }
            Signal::ShortSpread => {
                pnl += notional * (-dx + dy);
            }
            Signal::Flat => {}
        }

        // Charge cost on any position CHANGE, not just entries -- exits
        // and direct long<->short flips also cross the spread on both legs.
        // Written as an explicit match (not `sig != prev_sig`) specifically
        // so this compiles against the ORIGINAL Signal enum unchanged --
        // that enum only derives Clone/Copy/Debug, not PartialEq, and this
        // avoids requiring an edit to signals.rs just to add a cost model.
        let position_changed = match (prev_sig, sig) {
            (Signal::Flat, Signal::Flat) => false,
            (Signal::LongSpread, Signal::LongSpread) => false,
            (Signal::ShortSpread, Signal::ShortSpread) => false,
            _ => true,
        };
        if position_changed {
            // 2 legs (x and y), cost proportional to notional value traded
            // on each leg.
            total_costs += 2.0 * notional * cost_bps;
        }

        prev_sig = sig;
    }

    pnl -= total_costs;

    BacktestResult {
        pair: ("".into(), "".into()),
        total_pnl: pnl,
        total_costs,
        trades,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_synthetic(n: usize) -> (Vec<f64>, Vec<f64>, Vec<Signal>) {
        let mut x = vec![100.0; n];
        let mut y = vec![50.0; n];
        for t in 1..n {
            x[t] = x[t - 1] + if t % 2 == 0 { 0.5 } else { -0.3 };
            y[t] = y[t - 1] + if t % 3 == 0 { 0.2 } else { -0.1 };
        }
        // Alternate signals to force many position changes -- deliberately
        // adversarial (a real strategy wouldn't flip this often) so the
        // cost accounting gets properly exercised.
        let signals: Vec<Signal> = (0..n)
            .map(|t| match t % 4 {
                0 => Signal::Flat,
                1 => Signal::LongSpread,
                2 => Signal::Flat,
                _ => Signal::ShortSpread,
            })
            .collect();
        (x, y, signals)
    }

    #[test]
    fn zero_cost_matches_original_costless_behavior() {
        let (x, y, signals) = make_synthetic(200);
        let result_zero_cost = backtest_pair(&x, &y, &signals, 1.0, 0.0);

        // Manually replicate the ORIGINAL (pre-fix) cost-free logic
        // exactly, as a reference, to prove the extended version with
        // cost_bps=0.0 produces an identical total_pnl.
        let n = x.len().min(y.len()).min(signals.len());
        let mut pnl_ref = 0.0;
        let mut prev = Signal::Flat;
        for i in 1..n {
            let sig = signals[i];
            let dx = x[i] - x[i - 1];
            let dy = y[i] - y[i - 1];
            match sig {
                Signal::LongSpread => pnl_ref += 1.0 * (dx - dy),
                Signal::ShortSpread => pnl_ref += 1.0 * (-dx + dy),
                Signal::Flat => {}
            }
            prev = sig;
        }
        let _ = prev;

        assert!((result_zero_cost.total_pnl - pnl_ref).abs() < 1e-9,
                 "zero-cost pnl {} does not match original reference {}", result_zero_cost.total_pnl, pnl_ref);
        assert_eq!(result_zero_cost.total_costs, 0.0);
    }

    #[test]
    fn nonzero_cost_reduces_pnl_by_exactly_the_cost_amount() {
        let (x, y, signals) = make_synthetic(200);
        let cost_bps = 0.001; // 10 bps per leg
        let result_zero = backtest_pair(&x, &y, &signals, 1.0, 0.0);
        let result_costed = backtest_pair(&x, &y, &signals, 1.0, cost_bps);

        // The only difference between the two runs should be exactly the
        // accumulated transaction cost -- gross PnL (price-driven part)
        // must be identical regardless of cost_bps.
        let implied_gross_costed = result_costed.total_pnl + result_costed.total_costs;
        assert!((implied_gross_costed - result_zero.total_pnl).abs() < 1e-9,
                 "gross pnl changed when only cost_bps changed: {} vs {}",
                 implied_gross_costed, result_zero.total_pnl);
        assert!(result_costed.total_costs > 0.0, "expected nonzero costs given many position changes");
    }
}
