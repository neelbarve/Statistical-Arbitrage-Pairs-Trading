// src/model/cointegration.rs
// REPLACES the original: the original's `adf_test()` was a self-documented
// placeholder ("Very rough ADF-like statistic placeholder (for demo)")
// with an unaugmented (not "augmented") Dickey-Fuller regression and an
// arbitrary -2.5 cutoff. This version calls into the new model::adf module
// (proper AIC-selected lag augmentation, real MacKinnon (2010) response-
// surface critical values verified against statsmodels -- see
// docs/NOTES.md for the verification transcript).
//
// EXTERNAL API UNCHANGED: CointegrationResult{beta, adf_stat, is_cointegrated}
// and engle_granger(x, y) -> Result<CointegrationResult> are identical to
// the original, so universe/cointegration_scan.rs (the only caller) needs
// NO changes.
use anyhow::Result;
use crate::model::adf;

#[derive(Debug, Clone, Copy)]
pub struct CointegrationResult {
    #[allow(dead_code)]
    pub beta: f64,
    pub adf_stat: f64,
    pub is_cointegrated: bool,
}

pub fn engle_granger(x: &[f64], y: &[f64]) -> Result<CointegrationResult> {
    let n = x.len().min(y.len());
    if n < 50 {
        anyhow::bail!("not enough data for cointegration test");
    }

    let x = &x[..n];
    let y = &y[..n];

    // Use log prices (unchanged from original -- standard practice for
    // pairs trading, since it makes beta interpretable as an elasticity
    // and keeps the spread scale-invariant to price level).
    let x_log: Vec<f64> = x.iter().map(|v| v.ln()).collect();
    let y_log: Vec<f64> = y.iter().map(|v| v.ln()).collect();

    // model::adf::engle_granger does BOTH the step-1 OLS (y_log = alpha +
    // beta*x_log + residual) AND the step-2 ADF-on-residuals test, with
    // proper lag augmentation and real critical values.
    let result = adf::engle_granger(&x_log, &y_log);

    Ok(CointegrationResult {
        beta: result.beta,
        adf_stat: result.adf.tstat,
        is_cointegrated: result.is_cointegrated_5pct,
    })
}
