#![allow(dead_code)]
pub fn compute_spread(x: &[f64], y: &[f64], beta: f64) -> Vec<f64> {
    let n = x.len().min(y.len());
    (0..n).map(|i| y[i] - beta * x[i]).collect()
}

pub fn zscore(spread: &[f64], window: usize) -> Vec<f64> {
    let n = spread.len();
    let mut z = vec![0.0; n];

    if window == 0 || n < window {
        return z;
    }

    for i in window - 1..n {
        let slice = &spread[i + 1 - window..=i];
        let m = mean(slice);
        let s = std(slice, m);
        z[i] = if s > 0.0 { (spread[i] - m) / s } else { 0.0 };
    }

    z
}

fn mean(v: &[f64]) -> f64 {
    v.iter().sum::<f64>() / v.len() as f64
}

fn std(v: &[f64], m: f64) -> f64 {
    let var = v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / v.len() as f64;
    var.sqrt()
}
