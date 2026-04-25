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
