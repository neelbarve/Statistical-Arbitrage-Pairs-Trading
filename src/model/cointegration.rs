use anyhow::Result;

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

    // FIX 2: Use log prices (Python uses log prices for cointegration)
    let x_log: Vec<f64> = x.iter().map(|v| v.ln()).collect();
    let y_log: Vec<f64> = y.iter().map(|v| v.ln()).collect();

    // OLS: y_log = alpha + beta * x_log
    let mean_x = mean(&x_log);
    let mean_y = mean(&y_log);


    let mut num = 0.0;
    let mut den = 0.0;
    for i in 0..n {
        let dx = x_log[i] - mean_x;
        let dy = y_log[i] - mean_y;
        num += dx * dy;
        den += dx * dx;
    }
    let beta = num / den;
    let alpha = mean_y - beta * mean_x;

    // residuals
    let residuals: Vec<f64> = (0..n)
        .map(|i| y_log[i] - (alpha + beta * x_log[i]))
        .collect();

    // Very rough ADF-like statistic placeholder (for demo)
    let adf_stat = adf_test(&residuals);

    // For now, treat "strongly negative" as cointegrated (demo logic)
    let is_cointegrated = adf_stat < -2.5;

    Ok(CointegrationResult {
        beta,
        adf_stat,
        is_cointegrated,
    })
}

fn mean(v: &[f64]) -> f64 {
    v.iter().sum::<f64>() / v.len() as f64
}

// Very simplified AR(1) regression on residuals: r_t = phi * r_{t-1} + e_t
// Real Engle–Granger ADF regression: Δr_t = α + β r_{t-1} + ε_t
fn adf_test(residuals: &[f64]) -> f64 {
    let n = residuals.len();
    if n < 5 {
        return 0.0;
    }

    // Build Δr_t and r_{t-1}
    let mut dy = Vec::new();
    let mut r_lag = Vec::new();

    for t in 1..n {
        dy.push(residuals[t] - residuals[t - 1]);   // Δr_t
        r_lag.push(residuals[t - 1]);               // r_{t-1}
    }

    // OLS regression: dy = alpha + beta * r_lag
    let mean_x = mean(&r_lag);
    let mean_y = mean(&dy);

    let mut num = 0.0;
    let mut den = 0.0;

    for i in 0..r_lag.len() {
        num += (r_lag[i] - mean_x) * (dy[i] - mean_y);
        den += (r_lag[i] - mean_x).powi(2);
    }

    let beta = num / den;

    // Compute residuals
    let mut eps = Vec::new();
    for i in 0..r_lag.len() {
        eps.push(dy[i] - beta * r_lag[i]);
    }

    // Variance of residuals
    let var = eps.iter().map(|e| e * e).sum::<f64>() / (r_lag.len() as f64 - 1.0);

    // Standard error of beta
    let se = (var / den).sqrt();

    // t-statistic for beta
    beta / se
}
