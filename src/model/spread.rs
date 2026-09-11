// src/model/spread.rs
use anyhow::Result;

pub fn hedge_ratio_ols(x: &[f64], y: &[f64]) -> Result<(f64, f64)> {
    // regress x ~ a + b y
    let n = x.len().min(y.len());
    let (x, y) = (&x[..n], &y[..n]);

    let mean_x = x.iter().sum::<f64>() / n as f64;
    let mean_y = y.iter().sum::<f64>() / n as f64;

    let mut num = 0.0;
    let mut den = 0.0;
    for i in 0..n {
        let dy = y[i] - mean_y;
        num += (x[i] - mean_x) * dy;
        den += dy * dy;
    }
    let beta = num / den;
    let alpha = mean_x - beta * mean_y;
    Ok((alpha, beta))
}

pub fn spread(alpha: f64, beta: f64, x: &[f64], y: &[f64]) -> Vec<f64> {
    let n = x.len().min(y.len());
    (0..n).map(|i| x[i] - (alpha + beta * y[i])).collect()
}

pub fn zscore(series: &[f64]) -> Vec<f64> {
    let n = series.len();
    let mean = series.iter().sum::<f64>() / n as f64;
    let var = series.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n as f64;
    let std = var.sqrt().max(1e-8);
    series.iter().map(|v| (v - mean) / std).collect()
}

pub fn rolling_zscore(series: &[f64], short: usize, long: usize) -> Vec<f64> {
    let n = series.len();
    let mut out = vec![0.0; n];
    for i in 0..n {
        if i + 1 < long { continue; }
        let s = i + 1 - short;
        let l = i + 1 - long;
        let short_slice = &series[s..=i];
        let long_slice = &series[l..=i];

        let mean_long = long_slice.iter().sum::<f64>() / long as f64;
        let var_long = long_slice.iter()
            .map(|v| (v - mean_long).powi(2))
            .sum::<f64>() / long as f64;
        let std_long = var_long.sqrt().max(1e-8);

        let mean_short = short_slice.iter().sum::<f64>() / short as f64;
        out[i] = (mean_short - mean_long) / std_long;
    }
    out
}

pub fn std_spread(s: &[f64]) -> f64 {
    let m = mean(s);
    std(s, m)
}

// ---------------------------------------------------------------------------
// Ornstein-Uhlenbeck half-life of mean reversion.
// ---------------------------------------------------------------------------
// PLAIN-LANGUAGE EXPLANATION: cointegration ("Engle-Granger" elsewhere in
// this codebase) tells you a spread eventually drifts back toward its
// average -- but not HOW FAST. A spread that takes 3 trading days, on
// average, to travel halfway back to its mean after a shock is a very
// different, much more tradeable animal than one that takes 3 years, even
// if both pass the same cointegration test. "Half-life" is the standard
// number quants use to answer "how fast": it's the expected number of bars
// (days, here) for the spread to close HALF the distance back to its own
// long-run average, starting from wherever it currently is.
//
// HOW IT'S ESTIMATED: the spread is modeled as an Ornstein-Uhlenbeck (OU)
// process -- the continuous-time version of "a value that gets pulled back
// toward a fixed target, with random noise added at every instant." (The OU
// process itself dates to Uhlenbeck & Ornstein (1930), "On the Theory of
// the Brownian Motion," Physical Review.) Discretized to daily bars, the OU
// process implies that today's CHANGE in the spread is proportional to how
// far yesterday's spread was from its mean:
//
//     spread[t] - spread[t-1]  =  lambda * spread[t-1]  +  c  +  noise
//
// where `lambda` (the "mean-reversion speed") should be NEGATIVE for a
// mean-reverting series: the further spread[t-1] sits above its average,
// the more negative the next change tends to be (pulling it back down),
// and vice versa. This is exactly a linear regression -- "regress the
// day-over-day change in the spread on yesterday's level" -- so it's fit
// with the same generic OLS helper (`model::ols::ols`) used everywhere
// else in this codebase.
//
// Once `lambda` is known, the OU process has a closed-form half-life:
//
//     half_life = -ln(2) / lambda      (standard result; see e.g. Chan,
//                                        "Algorithmic Trading" (2013), ch.2,
//                                        or any stochastic-calculus
//                                        treatment of the OU process)
//
// If the fitted `lambda` comes out >= 0, the regression found NO evidence
// of mean reversion at all in this sample (the series looks like it's
// trending, or just a random walk) -- "half-life" is mathematically
// undefined in that case (the formula would return a negative or infinite
// number), so this function returns `None` instead of a nonsense value.
// Callers should treat `None` as "do not trade this pair as a mean-
// reversion strategy," regardless of what the separate cointegration test
// said, since a pair can occasionally pass a cointegration test on
// borderline statistical grounds without showing usable mean reversion
// within the specific window being measured.
pub fn half_life(spread: &[f64]) -> Option<f64> {
    let n = spread.len();
    if n < 10 {
        return None; // not enough bars to fit a trustworthy regression
    }

    // Build the regression: one row per day-over-day transition.
    //   regressor (X): [1.0 (intercept), spread[t-1] (yesterday's level)]
    //   response  (y): spread[t] - spread[t-1] (today's change)
    let mut x_rows: Vec<Vec<f64>> = Vec::with_capacity(n - 1);
    let mut deltas: Vec<f64> = Vec::with_capacity(n - 1);
    for t in 1..n {
        x_rows.push(vec![1.0, spread[t - 1]]);
        deltas.push(spread[t] - spread[t - 1]);
    }

    let fit = crate::model::ols::ols(&x_rows, &deltas);
    let lambda = fit.coefficients[1]; // coefficient on spread[t-1]

    // Reject lambda values that are zero *or effectively zero up to
    // floating-point noise* (e.g. a perfectly flat or perfectly trending
    // input can fit a `lambda` of +-1e-16 purely from rounding in the OLS
    // solve) -- a strict `lambda >= 0.0` sign check alone would let a
    // freak tiny-negative rounding error through and divide by it, which
    // would misreport a multi-thousand-bar "half-life" as if it were a
    // confident measurement. Anything this close to zero has no usable
    // mean-reversion signal either way.
    const MIN_MEANINGFUL_LAMBDA: f64 = -1e-6;
    if !lambda.is_finite() || lambda >= MIN_MEANINGFUL_LAMBDA {
        return None; // no mean reversion detected -- see doc comment above
    }

    Some(-(std::f64::consts::LN_2) / lambda)
}

fn mean(v: &[f64]) -> f64 {
    v.iter().sum::<f64>() / v.len() as f64
}

fn std(v: &[f64], m: f64) -> f64 {
    let var = v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / v.len() as f64;
    var.sqrt()
}

// ---------------------------------------------------------------------------
// NEW: walk-forward (rolling-window) hedge ratio estimation.
// ---------------------------------------------------------------------------
// hedge_ratio_ols() above fits alpha/beta ONCE using the entire series
// passed to it -- verified directly against how it's actually called in
// this project (main.rs and universe/cointegration_scan.rs both pass full
// history), which means every historical spread/z-score value was computed
// using a hedge ratio that "knew" the entire future price relationship
// between the two stocks. This is look-ahead bias, even though
// rolling_zscore() above is already correctly walk-forward on top of it.
//
// walk_forward_hedge_ratio() re-estimates alpha/beta periodically using
// ONLY a trailing window of past data -- see docs/NOTES.md for the full
// writeup and a verified test proving it does not anticipate a regime
// change before that change is inside the trailing window.
//
// CONVENTION MATCHES hedge_ratio_ols/spread EXACTLY: regresses
// x ~ alpha + beta*y (x dependent, y independent), and
// out.spread[t] = x[t] - (alpha_t + beta_t * y[t]).
use crate::model::ols::ols;

pub struct WalkForwardHedgeRatio {
    pub alphas: Vec<f64>,
    pub betas: Vec<f64>,
    pub spread: Vec<f64>,  // NaN for bars before the first full lookback window
}

/// `lookback`: trailing window size used to fit alpha/beta at each
/// re-estimation point (e.g. 60 trading days -- tune per pair's mean-
/// reversion half-life).
/// `reestimate_every`: how often to refit (e.g. every 5 bars).
///
/// Bars before the first `lookback` bars are available get NaN spread --
/// this is the "formation period": real out-of-sample trading can only
/// start once a full lookback window exists. Exclude these bars from
/// backtesting (see docs/NOTES.md main.rs restructuring notes).
pub fn walk_forward_hedge_ratio(x: &[f64], y: &[f64], lookback: usize, reestimate_every: usize) -> WalkForwardHedgeRatio {
    let n = x.len().min(y.len());
    let mut alphas = vec![f64::NAN; n];
    let mut betas = vec![f64::NAN; n];
    let mut spread = vec![f64::NAN; n];

    let mut current_alpha = f64::NAN;
    let mut current_beta = f64::NAN;

    for t in lookback..n {
        if (t - lookback) % reestimate_every == 0 || current_alpha.is_nan() {
            let window_x = &x[(t - lookback)..t];
            let window_y = &y[(t - lookback)..t];
            let y_rows: Vec<Vec<f64>> = window_y.iter().map(|&yi| vec![1.0, yi]).collect();
            let fit = ols(&y_rows, window_x);
            current_alpha = fit.coefficients[0];
            current_beta = fit.coefficients[1];
        }
        alphas[t] = current_alpha;
        betas[t] = current_beta;
        spread[t] = x[t] - (current_alpha + current_beta * y[t]);
    }

    WalkForwardHedgeRatio { alphas, betas, spread }
}

#[cfg(test)]
mod walk_forward_tests {
    use super::*;

    #[test]
    fn hedge_ratio_never_uses_future_data() {
        let n = 200;
        let mut y = vec![0.0; n];
        let mut x = vec![0.0; n];
        for t in 0..n {
            y[t] = t as f64 * 0.1;
            let true_beta = if t < 100 { 2.0 } else { 5.0 };
            x[t] = 1.0 + true_beta * y[t];
        }
        let result = walk_forward_hedge_ratio(&x, &y, 30, 5);
        assert!((result.betas[90] - 2.0).abs() < 0.1);
        assert!((result.betas[140] - 5.0).abs() < 0.1);
    }

    #[test]
    fn half_life_recovers_known_mean_reversion_speed() {
        // Simulate a textbook OU process by hand: spread[t] = spread[t-1]
        // + lambda*(spread[t-1] - mu) + noise, with a small DETERMINISTIC
        // "noise" term (no external `rand` crate dependency needed -- this
        // project has none) so the test is exactly reproducible.
        let true_lambda = -0.1;
        let mu = 50.0;
        let n = 500;
        let mut spread = vec![0.0; n];
        spread[0] = mu + 20.0; // start 20 units away from the long-run mean
        for t in 1..n {
            let noise = ((t % 7) as f64 - 3.0) * 0.05; // deterministic, mean ~0, small vs. the signal
            spread[t] = spread[t - 1] + true_lambda * (spread[t - 1] - mu) + noise;
        }

        let estimated = half_life(&spread).expect("mean-reverting series must yield a half-life");
        let theoretical = -(std::f64::consts::LN_2) / true_lambda; // ~6.93 bars
        assert!(
            (estimated - theoretical).abs() < 1.0,
            "estimated half-life {} too far from theoretical {}",
            estimated,
            theoretical
        );
    }

    #[test]
    fn half_life_is_none_for_a_trending_non_mean_reverting_series() {
        // A pure linear trend (constant +1.0 step every bar) has no
        // mean-reversion component at all -- the fitted lambda must come
        // out to (essentially) zero, which half_life() must reject rather
        // than report a misleading half-life.
        let spread: Vec<f64> = (0..200).map(|t| t as f64).collect();
        assert!(half_life(&spread).is_none());
    }
}
