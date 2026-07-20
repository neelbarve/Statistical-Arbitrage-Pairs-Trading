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
}
