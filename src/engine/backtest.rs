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

// Annualized risk-free rate assumed for the Sharpe ratio below (e.g. a
// representative T-bill yield). Per-bar equity changes are treated as
// returns on a unit notional, so this is subtracted on the same basis --
// see README.md "Assumptions" for the caveat this implies.
pub const RISK_FREE_RATE_ANNUAL: f64 = 0.04;
const TRADING_BARS_PER_YEAR: f64 = 252.0;

#[derive(Debug, Clone)]
pub struct BacktestResult {
    pub pair: (String, String),
    pub threshold: f64,     // NEW: entry z-score threshold (e.g. 1.5 or 2.0) this result belongs to
    pub total_pnl: f64,
    pub total_costs: f64,   // NEW: reported separately so gross vs net PnL is always visible
    pub trades: usize,
    pub equity_curve: Vec<f64>,   // NEW: cumulative net PnL at each bar, for equity-curve plots
    pub max_drawdown: f64,        // NEW: largest peak-to-trough drop in the equity curve
    pub risk_free_rate: f64,      // NEW: annualized risk-free rate used in the Sharpe/Sortino ratios below
    pub volatility: f64,          // NEW: annualized volatility of per-bar equity changes
    pub sharpe_ratio: f64,        // NEW: annualized, risk-free-adjusted Sharpe ratio
    pub sortino_ratio: f64,       // NEW: like Sharpe, but only penalizes downside moves (see sortino_ratio() below)
    // NEW: Ornstein-Uhlenbeck half-life (in bars) of the underlying spread
    // this pair trades -- see model::spread::half_life() for the full
    // explanation. This field is NOT computed in here: half-life is a
    // property of the *spread* (price_x - alpha - beta*price_y), and this
    // function only ever sees the raw x/y price legs plus a list of
    // already-decided signals, not the alpha/beta that produced them. The
    // caller (main.rs) computes it once per pair from the spread it built,
    // then copies it onto this struct -- the same pattern already used for
    // `pair` and `threshold` above. `None` means the spread showed no
    // measurable mean reversion in this sample.
    pub half_life_bars: Option<f64>,
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
    let mut equity_curve = Vec::with_capacity(n.max(1));
    equity_curve.push(0.0);

    for i in 1..n {
        let sig = signals[i];
        if !matches!(sig, Signal::Flat) && matches!(prev_sig, Signal::Flat) {
            trades += 1;
        }

        let dx = x[i] - x[i - 1];
        let dy = y[i] - y[i - 1];

        let mut step_pnl = 0.0;
        match sig {
            Signal::LongSpread => {
                step_pnl += notional * (dx - dy);
            }
            Signal::ShortSpread => {
                step_pnl += notional * (-dx + dy);
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
        let mut step_cost = 0.0;
        if position_changed {
            // 2 legs (x and y), cost proportional to notional value traded
            // on each leg.
            step_cost = 2.0 * notional * cost_bps;
            total_costs += step_cost;
        }

        pnl += step_pnl - step_cost;
        equity_curve.push(pnl);

        prev_sig = sig;
    }

    let max_drawdown = max_drawdown(&equity_curve);
    let volatility = volatility(&equity_curve);
    let sharpe_ratio = sharpe_ratio(&equity_curve, RISK_FREE_RATE_ANNUAL);
    let sortino_ratio = sortino_ratio(&equity_curve, RISK_FREE_RATE_ANNUAL);

    BacktestResult {
        pair: ("".into(), "".into()),
        threshold: 0.0,
        total_pnl: pnl,
        total_costs,
        trades,
        equity_curve,
        max_drawdown,
        risk_free_rate: RISK_FREE_RATE_ANNUAL,
        volatility,
        sharpe_ratio,
        sortino_ratio,
        half_life_bars: None, // filled in by the caller -- see field doc comment above
    }
}

// Largest peak-to-trough decline in the equity curve (in the same dollar
// units as total_pnl). Always >= 0.0.
fn max_drawdown(equity: &[f64]) -> f64 {
    let mut peak = f64::NEG_INFINITY;
    let mut worst = 0.0;
    for &e in equity {
        if e > peak {
            peak = e;
        }
        let dd = peak - e;
        if dd > worst {
            worst = dd;
        }
    }
    worst
}

fn bar_returns(equity: &[f64]) -> Vec<f64> {
    equity.windows(2).map(|w| w[1] - w[0]).collect()
}

fn mean_and_sd(returns: &[f64]) -> (f64, f64) {
    let mean = returns.iter().sum::<f64>() / returns.len() as f64;
    let var = returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / returns.len() as f64;
    (mean, var.sqrt())
}

// Annualized volatility (standard deviation) of bar-over-bar equity
// changes. Returns 0.0 when there isn't enough data to measure spread.
fn volatility(equity: &[f64]) -> f64 {
    if equity.len() < 3 {
        return 0.0;
    }
    let (_, sd) = mean_and_sd(&bar_returns(equity));
    sd * TRADING_BARS_PER_YEAR.sqrt()
}

// Annualized, risk-free-adjusted Sharpe ratio computed from bar-over-bar
// equity changes, assuming ~252 trading bars per year. Returns 0.0 when
// there is not enough data or no variance (flat equity curve) to avoid
// division by zero.
fn sharpe_ratio(equity: &[f64], risk_free_rate_annual: f64) -> f64 {
    if equity.len() < 3 {
        return 0.0;
    }
    let returns = bar_returns(equity);
    let (mean, sd) = mean_and_sd(&returns);
    if sd <= 1e-12 {
        return 0.0;
    }
    let rf_per_bar = risk_free_rate_annual / TRADING_BARS_PER_YEAR;
    ((mean - rf_per_bar) / sd) * TRADING_BARS_PER_YEAR.sqrt()
}

// PLAIN-LANGUAGE EXPLANATION OF WHY THIS EXISTS ALONGSIDE SHARPE:
// The Sharpe ratio above divides by the FULL standard deviation of returns
// -- which treats a big, welcome up-day exactly the same as an equally big,
// unwelcome down-day, because squaring a deviation throws away its sign.
// For a mean-reversion strategy in particular, the return distribution is
// often lumpy/skewed (many small gains as the spread creeps back to its
// mean, occasional sharp losses when it doesn't), so penalizing upside
// swings as if they were risk can understate how good the strategy really
// is. The Sortino ratio (Sortino & Price, 1994, "Performance Measurement
// in a Downside Risk Framework," The Journal of Investing) fixes this by
// only counting deviations BELOW a minimum-acceptable-return (MAR) hurdle
// in the denominator -- days that beat the hurdle contribute zero "risk,"
// no matter how large the gain. This function uses the same per-bar
// risk-free rate as the Sharpe ratio above as that hurdle, so the two
// ratios share an identical numerator (excess return over the risk-free
// rate) and differ ONLY in which kind of volatility divides it -- making
// them directly comparable side by side.
fn sortino_ratio(equity: &[f64], risk_free_rate_annual: f64) -> f64 {
    if equity.len() < 3 {
        return 0.0;
    }
    let returns = bar_returns(equity);

    // Guard #1: if the RAW returns have (essentially) zero variance --
    // e.g. a strategy that generated zero trades, so every single bar's
    // return is identically 0.0 -- there is no meaningful risk-adjusted
    // ratio to report, exactly like sharpe_ratio's guard above. This check
    // has to come BEFORE the downside-only calculation below: a constant
    // 0.0 return sits *below* a positive risk-free hurdle on every single
    // bar, which has zero variance in that shortfall but is NOT zero --
    // without this guard, a strategy that simply never traded would
    // compute a large, nonsensical NEGATIVE Sortino ratio (found exactly
    // this way while testing: an idle pair scored Sortino ≈ -15.9, while
    // its Sharpe correctly showed 0.0 for the same idle equity curve).
    let (_, raw_sd) = mean_and_sd(&returns);
    if raw_sd <= 1e-12 {
        return 0.0;
    }

    let rf_per_bar = risk_free_rate_annual / TRADING_BARS_PER_YEAR;
    let mean = returns.iter().sum::<f64>() / returns.len() as f64;

    // "Downside deviation": like a standard deviation, but bars that beat
    // the risk-free hurdle are treated as contributing zero risk (squared
    // deviation of 0), instead of counting their distance from the mean
    // like an ordinary standard deviation would.
    let downside_sum_sq: f64 = returns
        .iter()
        .map(|r| {
            let excess_over_hurdle = r - rf_per_bar;
            if excess_over_hurdle < 0.0 {
                excess_over_hurdle * excess_over_hurdle
            } else {
                0.0
            }
        })
        .sum();
    let downside_deviation = (downside_sum_sq / returns.len() as f64).sqrt();

    // Guard #2: the strategy DID have real variance overall (guard #1
    // passed), but never once fell below the risk-free hurdle (e.g. every
    // single trade was a winner) -- downside_deviation is ~0 here for a
    // completely different, much rarer reason than guard #1. Dividing by
    // it would blow up toward +infinity; report 0.0 rather than an
    // unbounded ratio, the same conservative choice sharpe_ratio makes
    // for its own near-zero-denominator case.
    if downside_deviation <= 1e-12 {
        return 0.0;
    }
    ((mean - rf_per_bar) / downside_deviation) * TRADING_BARS_PER_YEAR.sqrt()
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

    #[test]
    fn sortino_is_zero_not_negative_for_a_pair_that_never_traded() {
        // Regression test for a real bug found while validating this
        // pipeline's output: a pair whose z-score never crossed the entry
        // threshold produces an all-Flat signal vector, so its equity
        // curve is identically 0.0 at every bar. A flat 0% return sits
        // just below a POSITIVE risk-free rate every single day, with
        // zero variance in that shortfall -- which the original downside-
        // deviation-only guard didn't recognize as "no real signal here,"
        // and it computed a large, misleading negative Sortino ratio
        // (~-15.9) for a pair that took no risk and had no position at
        // all. Sharpe correctly reports 0.0 for the same input; Sortino
        // must match.
        let n = 100;
        let x = vec![100.0; n];
        let y = vec![50.0; n];
        let signals = vec![Signal::Flat; n];

        let result = backtest_pair(&x, &y, &signals, 1.0, 0.0002);
        assert_eq!(result.trades, 0);
        assert_eq!(result.sharpe_ratio, 0.0);
        assert_eq!(
            result.sortino_ratio, 0.0,
            "an all-Flat (never-traded) equity curve must score Sortino 0.0, matching Sharpe -- not a large negative number"
        );
    }
}
