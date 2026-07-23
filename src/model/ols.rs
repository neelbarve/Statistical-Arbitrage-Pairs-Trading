// Generic multiple linear regression via normal equations: beta = (X'X)^-1 X'y.
// No external crates (ndarray/linfa) -- plain Vec<Vec<f64>> and a direct
// Gauss-Jordan inverse, since the regressor counts here are always small
// (at most ~15-20 lag terms), where a naive approach is both simple to
// verify by hand and fast enough.
//
// Returns (coefficients, residuals, std_errors) so the caller can compute
// t-statistics on any individual coefficient (specifically: the ADF test's
// coefficient on the lagged level term).

pub struct OlsResult {
    pub coefficients: Vec<f64>,
    pub residuals: Vec<f64>,
    pub std_errors: Vec<f64>,
    pub n: usize,
    pub k: usize, // number of regressors (including any constant column supplied)
}

/// X: n x k design matrix (row-major, each inner Vec is one row/observation).
/// y: n-length response vector.
pub fn ols(x: &[Vec<f64>], y: &[f64]) -> OlsResult {
    let n = x.len();
    let k = x[0].len();
    assert_eq!(y.len(), n, "x and y must have the same number of observations");

    // X'X (k x k) and X'y (k-length)
    let mut xtx = vec![vec![0.0; k]; k];
    let mut xty = vec![0.0; k];
    for i in 0..n {
        for a in 0..k {
            xty[a] += x[i][a] * y[i];
            for b in 0..k {
                xtx[a][b] += x[i][a] * x[i][b];
            }
        }
    }

    let xtx_inv = invert(&xtx);

    // beta = (X'X)^-1 X'y
    let mut beta = vec![0.0; k];
    for a in 0..k {
        for b in 0..k {
            beta[a] += xtx_inv[a][b] * xty[b];
        }
    }

    // residuals and residual variance (for standard errors)
    let mut residuals = vec![0.0; n];
    let mut rss = 0.0;
    for i in 0..n {
        let mut yhat = 0.0;
        for a in 0..k {
            yhat += x[i][a] * beta[a];
        }
        residuals[i] = y[i] - yhat;
        rss += residuals[i] * residuals[i];
    }
    let dof = (n - k).max(1) as f64;
    let sigma2 = rss / dof;

    // std error of beta_a = sqrt(sigma2 * (X'X)^-1_{aa})
    let mut std_errors = vec![0.0; k];
    for a in 0..k {
        std_errors[a] = (sigma2 * xtx_inv[a][a]).sqrt();
    }

    OlsResult { coefficients: beta, residuals, std_errors, n, k }
}

/// AIC for a regression with `k` parameters and residual sum of squares `rss`
/// over `n` observations -- standard formula: n*ln(rss/n) + 2k.
/// Used to select ADF lag order the same way statsmodels' autolag='AIC' does:
/// compute AIC for every candidate lag order and take the global minimum.
pub fn aic(n: usize, k: usize, rss: f64) -> f64 {
    let n_f = n as f64;
    n_f * (rss / n_f).ln() + 2.0 * (k as f64)
}

fn invert(m: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let n = m.len();
    let mut a: Vec<Vec<f64>> = m.iter().map(|row| row.clone()).collect();
    let mut inv = vec![vec![0.0; n]; n];
    for i in 0..n {
        inv[i][i] = 1.0;
    }
    // Gauss-Jordan elimination with partial pivoting
    for col in 0..n {
        let mut pivot_row = col;
        let mut best = a[col][col].abs();
        for r in (col + 1)..n {
            if a[r][col].abs() > best {
                best = a[r][col].abs();
                pivot_row = r;
            }
        }
        a.swap(col, pivot_row);
        inv.swap(col, pivot_row);

        let pivot = a[col][col];
        assert!(pivot.abs() > 1e-12, "singular matrix in OLS normal equations");
        for j in 0..n {
            a[col][j] /= pivot;
            inv[col][j] /= pivot;
        }
        for r in 0..n {
            if r == col {
                continue;
            }
            let factor = a[r][col];
            for j in 0..n {
                a[r][j] -= factor * a[col][j];
                inv[r][j] -= factor * inv[col][j];
            }
        }
    }
    inv
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovers_known_linear_relationship() {
        // y = 2 + 3*x1 - 1*x2, exactly, no noise -> OLS must recover exactly.
        let x = vec![
            vec![1.0, 1.0, 2.0],
            vec![1.0, 2.0, 1.0],
            vec![1.0, 3.0, 4.0],
            vec![1.0, 4.0, 2.0],
            vec![1.0, 5.0, 3.0],
        ];
        let y: Vec<f64> = x.iter().map(|row| 2.0 + 3.0 * row[1] - 1.0 * row[2]).collect();
        let result = ols(&x, &y);
        assert!((result.coefficients[0] - 2.0).abs() < 1e-8);
        assert!((result.coefficients[1] - 3.0).abs() < 1e-8);
        assert!((result.coefficients[2] - (-1.0)).abs() < 1e-8);
        for r in &result.residuals {
            assert!(r.abs() < 1e-8);
        }
    }
}
