// Augmented Dickey-Fuller test and Engle-Granger two-step cointegration test.
//
// This replaces the original implementation's self-documented placeholder
// (`// Very rough ADF-like statistic placeholder (for demo)`, comparing
// against an arbitrary -2.5 cutoff with no augmentation lags at all --
// i.e. a plain, unaugmented Dickey-Fuller test, not an ADF test, and no
// properly sourced critical value).
//
// Two things were fixed, independently:
//
//   1. THE REGRESSION ITSELF is now actually "augmented": lagged
//      difference terms are included to whiten serially-correlated
//      residuals, with the lag order chosen by AIC (same selection
//      principle statsmodels' `autolag='aic'` uses) rather than fixed at
//      zero. This part required no external numeric table -- it's just
//      doing the OLS regression MacKinnon's test actually specifies.
//
//   2. THE CRITICAL VALUE is now MacKinnon's actual (2010) finite-sample
//      response-surface formula for the 2-variable, constant-in-the-
//      cointegrating-regression case (`crit(nobs) = beta_inf + beta1/nobs
//      + beta2/nobs^2`), NOT an arbitrary constant. The coefficients
//      below were read directly from statsmodels' `adfvalues.py` source
//      (an open, peer-reviewed implementation of MacKinnon's tables) --
//      not copied from an image or recalled from memory -- and were
//      cross-checked against statsmodels' own printed critical values on
//      a synthetic n=500 test case, matching to ~4 decimal places (see
//      docs/NOTES.md in the main project for the verification transcript).
//
// Reference: MacKinnon, J.G. (2010). "Critical Values for Cointegration
// Tests." Queen's University Dept. of Economics Working Paper No. 1227.
use crate::model::ols::{aic, ols};

pub struct AdfResult {
    pub tstat: f64,
    pub lags_used: usize,
    pub crit_1pct: f64,
    pub crit_5pct: f64,
    pub crit_10pct: f64,
    pub is_stationary_5pct: bool,
}

/// MacKinnon (2010) response-surface critical values for regression type
/// "c" (constant in the cointegrating regression, no trend), N=2 variables
/// -- i.e. exactly the case for a 2-asset pairs-trading spread
/// (y = alpha + beta*x + residual). Coefficients transcribed from
/// statsmodels/tsa/adfvalues.py `tau_c_2010[N=2]` row.
fn mackinnon_critical_value(nobs: f64, level_coeffs: [f64; 3]) -> f64 {
    let [beta_inf, beta1, beta2] = level_coeffs;
    beta_inf + beta1 / nobs + beta2 / (nobs * nobs)
}

const CRIT_1PCT_N2: [f64; 3] = [-3.89644, -10.9519, -33.527];
const CRIT_5PCT_N2: [f64; 3] = [-3.33613, -6.1101, -6.823];
const CRIT_10PCT_N2: [f64; 3] = [-3.04445, -4.2412, -2.720];

/// Schwert (1989)'s rule of thumb for the maximum lag to consider, the
/// same starting point statsmodels' autolag search uses by default.
fn max_lag_schwert(n: usize) -> usize {
    (12.0 * (n as f64 / 100.0).powf(0.25)).floor() as usize
}

/// Runs the augmented Dickey-Fuller test on `series` (intended to be OLS
/// residuals from the cointegrating regression -- see engle_granger below).
/// No constant term in the ADF regression itself: by construction, OLS
/// residuals from a regression that already included a constant have
/// (numerically, up to float error) zero mean, so re-including a constant
/// here would be redundant -- this matches the standard Engle-Granger
/// residual-based ADF convention.
pub fn augmented_dickey_fuller(series: &[f64]) -> AdfResult {
    let n = series.len();
    let max_lag = max_lag_schwert(n).min(n / 3).max(0); // guard: never use more lags than the data can support

    // Precompute first differences: d[t] = series[t] - series[t-1], for t=1..n-1
    let d: Vec<f64> = (1..n).map(|t| series[t] - series[t - 1]).collect();

    let mut best_aic = f64::INFINITY;
    let mut best_result: Option<(f64, usize)> = None;

    for lag in 0..=max_lag {
        // Regression: d[t] = gamma * series[t-1] + sum_{i=1..lag} delta_i * d[t-1-i] + eps_t
        // Valid t range: need series[t-1] and d[t-1-i] for i up to `lag`,
        // where d is already a "differences" array indexed from series[1]-series[0].
        // t (in series-space) ranges from (lag+1) to n-1.
        let start_t = lag + 1;
        if start_t >= n {
            continue; // not enough data for this many lags
        }
        let num_obs = n - start_t;
        if num_obs < lag + 5 {
            continue; // not enough degrees of freedom to bother
        }

        let mut x_rows: Vec<Vec<f64>> = Vec::with_capacity(num_obs);
        let mut y_vals: Vec<f64> = Vec::with_capacity(num_obs);
        for t in start_t..n {
            let mut row = vec![series[t - 1]]; // gamma coefficient's regressor
            for i in 1..=lag {
                row.push(d[t - 1 - i]); // d is 0-indexed from series[1]-series[0], so d[t-1-i] = series[t-i]-series[t-1-i]
            }
            x_rows.push(row);
            y_vals.push(d[t - 1]); // d[t-1] = series[t] - series[t-1]
        }

        let fit = ols(&x_rows, &y_vals);
        let rss: f64 = fit.residuals.iter().map(|r| r * r).sum();
        let k = 1 + lag;
        let this_aic = aic(num_obs, k, rss.max(1e-12));

        if this_aic < best_aic {
            best_aic = this_aic;
            let gamma = fit.coefficients[0];
            let se_gamma = fit.std_errors[0];
            let tstat = gamma / se_gamma;
            best_result = Some((tstat, lag));
        }
    }

    let (tstat, lags_used) = best_result.expect("ADF: no valid lag order found -- series too short");

    let nobs = n as f64;
    let crit_1pct = mackinnon_critical_value(nobs, CRIT_1PCT_N2);
    let crit_5pct = mackinnon_critical_value(nobs, CRIT_5PCT_N2);
    let crit_10pct = mackinnon_critical_value(nobs, CRIT_10PCT_N2);

    AdfResult {
        tstat,
        lags_used,
        crit_1pct,
        crit_5pct,
        crit_10pct,
        is_stationary_5pct: tstat < crit_5pct,
    }
}

pub struct EngleGrangerResult {
    pub alpha: f64,
    pub beta: f64,
    pub adf: AdfResult,
    pub is_cointegrated_5pct: bool,
}

/// Step 1: OLS regression y = alpha + beta*x + residual (WITH constant).
/// Step 2: ADF test on the residuals (see augmented_dickey_fuller above).
/// This is the full two-step Engle-Granger procedure -- both steps use
/// ONLY the data passed in; whether that data is the full history or a
/// formation-period subset is the CALLER's responsibility (see
/// docs/NOTES.md walk-forward restructuring notes for why this matters).
pub fn engle_granger(x: &[f64], y: &[f64]) -> EngleGrangerResult {
    let n = x.len().min(y.len());
    let x_rows: Vec<Vec<f64>> = (0..n).map(|i| vec![1.0, x[i]]).collect();
    let y_vals: Vec<f64> = y[..n].to_vec();

    let step1 = ols(&x_rows, &y_vals);
    let alpha = step1.coefficients[0];
    let beta = step1.coefficients[1];
    let residuals = step1.residuals;

    let adf = augmented_dickey_fuller(&residuals);
    let is_cointegrated_5pct = adf.is_stationary_5pct;

    EngleGrangerResult { alpha, beta, adf, is_cointegrated_5pct }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mackinnon_formula_matches_statsmodels_n500() {
        // Ground truth from statsmodels (coint() internally calls
        // mackinnoncrit(N=2, regression='c', nobs=500)), obtained by
        // actually running statsmodels in this project's sandbox -- see
        // docs/NOTES.md for the transcript. This test exists so that any
        // future edit to the coefficients above gets caught immediately
        // against a real, external, peer-reviewed reference.
        let crit_1 = mackinnon_critical_value(500.0, CRIT_1PCT_N2);
        let crit_5 = mackinnon_critical_value(500.0, CRIT_5PCT_N2);
        let crit_10 = mackinnon_critical_value(500.0, CRIT_10PCT_N2);
        assert!((crit_1 - (-3.91852)).abs() < 0.001, "got {}", crit_1);
        assert!((crit_5 - (-3.34840)).abs() < 0.001, "got {}", crit_5);
        assert!((crit_10 - (-3.05296)).abs() < 0.001, "got {}", crit_10);
    }
}
