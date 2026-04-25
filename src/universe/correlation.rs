pub fn correlation_matrix(
    data: &[(String, Vec<f64>, Vec<chrono::NaiveDateTime>)]
) -> Vec<(String, String, f64)> {
    let mut out = Vec::new();

    for i in 0..data.len() {
        for j in i + 1..data.len() {
            let (ref a_name, ref a, _) = data[i];
            let (ref b_name, ref b, _) = data[j];

            let n = a.len().min(b.len());
            let a = &a[..n];
            let b = &b[..n];

            let corr = pearson(a, b);
            out.push((a_name.clone(), b_name.clone(), corr));
        }
    }

    out
}


fn pearson(a: &[f64], b: &[f64]) -> f64 {
    let n = a.len();
    let mean_a = a.iter().sum::<f64>() / n as f64;
    let mean_b = b.iter().sum::<f64>() / n as f64;

    let mut num = 0.0;
    let mut den_a = 0.0;
    let mut den_b = 0.0;

    for i in 0..n {
        let da = a[i] - mean_a;
        let db = b[i] - mean_b;
        num += da * db;
        den_a += da * da;
        den_b += db * db;
    }

    num / (den_a.sqrt() * den_b.sqrt())
}
