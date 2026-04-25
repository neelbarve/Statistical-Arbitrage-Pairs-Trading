use anyhow::Result;

/// Forecast ARMA(1,2) for `steps` ahead.
/// Model:
///   x_t = c + phi * x_{t-1} + theta1 * e_{t-1} + theta2 * e_{t-2} + e_t
///
/// We estimate phi, theta1, theta2 using OLS on lagged regressors.
/// Residuals e_t are computed from fitted model.
/// Forecast uses last residuals.
pub fn forecast_arma12(series: &[f64], steps: usize) -> Result<Vec<f64>> {
    let n = series.len();
    if n < 10 {
        anyhow::bail!("Series too short for ARMA(1,2)");
    }

    // Build regressors:
    // y[t] = series[t]
    // X[t] = [1, x[t-1], e[t-1], e[t-2]]
    let mut y = Vec::new();
    let mut xmat = Vec::new();

    // First pass: estimate AR(1) to get initial residuals
    let phi_init = {
        let num: f64 = series.windows(2).map(|w| w[0] * w[1]).sum();
        let den: f64 = series[..n - 1].iter().map(|v| v * v).sum();
        num / den
    };

    let mut residuals = vec![0.0; n];
    for t in 1..n {
        residuals[t] = series[t] - phi_init * series[t - 1];
    }

    // Build regression rows
    for t in 2..n {
        y.push(series[t]);
        xmat.push(vec![
            1.0,
            series[t - 1],
            residuals[t - 1],
            residuals[t - 2],
        ]);
    }

    // Solve OLS: beta = (X'X)^(-1) X'y
    let xtx = {
        let mut m = [[0.0; 4]; 4];
        for row in &xmat {
            for i in 0..4 {
                for j in 0..4 {
                    m[i][j] += row[i] * row[j];
                }
            }
        }
        m
    };

    let xty = {
        let mut v = [0.0; 4];
        for (row, &yt) in xmat.iter().zip(y.iter()) {
            for i in 0..4 {
                v[i] += row[i] * yt;
            }
        }
        v
    };

    // Solve 4x4 linear system
    let beta = solve_4x4(xtx, xty)?;

    let c = beta[0];
    let phi = beta[1];
    let theta1 = beta[2];
    let theta2 = beta[3];

    // Recompute residuals using fitted model
    for t in 2..n {
        residuals[t] = series[t] - (c + phi * series[t - 1] + theta1 * residuals[t - 1] + theta2 * residuals[t - 2]);
    }

    // Forecast
    let mut out = Vec::new();
    let mut last_x = series[n - 1];
    let mut e1 = residuals[n - 1];
    let mut e2 = residuals[n - 2];

    for _ in 0..steps {
        let next = c + phi * last_x + theta1 * e1 + theta2 * e2;
        out.push(next);

        // Update for next iteration
        e2 = e1;
        e1 = 0.0; // future residuals assumed zero
        last_x = next;
    }

    Ok(out)
}

/// Solve 4x4 linear system using Gaussian elimination
fn solve_4x4(mut a: [[f64; 4]; 4], mut b: [f64; 4]) -> Result<[f64; 4]> {
    for i in 0..4 {
        // pivot
        let mut max = i;
        for r in i + 1..4 {
            if a[r][i].abs() > a[max][i].abs() {
                max = r;
            }
        }
        a.swap(i, max);
        b.swap(i, max);

        let pivot = a[i][i];
        if pivot.abs() < 1e-12 {
            anyhow::bail!("Singular matrix");
        }

        for j in i..4 {
            a[i][j] /= pivot;
        }
        b[i] /= pivot;

        for r in 0..4 {
            if r != i {
                let f = a[r][i];
                for j in i..4 {
                    a[r][j] -= f * a[i][j];
                }
                b[r] -= f * b[i];
            }
        }
    }
    Ok(b)
}
