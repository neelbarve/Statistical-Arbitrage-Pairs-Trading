// pub fn forecast_spread_ar1(spread: &[f64], steps: usize) -> Vec<f64> {
//     let n = spread.len();
//     let window = &spread[n - 100..];

//     let mut num = 0.0;
//     let mut den = 0.0;
//     for i in 0..window.len() - 1 {
//         num += window[i] * window[i + 1];
//         den += window[i] * window[i];
//     }
//     let phi = num / den;

//     let mut out = Vec::new();
//     let mut last = spread[n - 1];

//     for _ in 0..steps {
//         last = phi * last;
//         out.push(last);
//     }

//     out
// }

pub fn normalize_with_last_window(hist: &[f64], pred: &[f64]) -> Vec<f64> {
    let w = 20;
    let slice = &hist[hist.len() - w..];

    let mean = slice.iter().sum::<f64>() / w as f64;
    let var = slice.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / w as f64;
    let sd = var.sqrt();

    pred.iter().map(|v| (v - mean) / sd).collect()
}

