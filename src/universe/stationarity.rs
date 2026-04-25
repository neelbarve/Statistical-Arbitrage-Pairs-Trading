#![allow(dead_code)]
pub fn adf_like(series: &[f64]) -> f64 {
    let n = series.len();
    let mut num = 0.0;
    let mut den = 0.0;

    for i in 1..n {
        let r = series[i];
        let r1 = series[i - 1];
        num += r * r1;
        den += r1 * r1;
    }

    let phi = num / den;
    (phi - 1.0) * (n as f64).sqrt()
}
