use crate::model::cointegration::engle_granger;

pub fn scan_cointegration(
    data: &[(String, Vec<f64>, Vec<chrono::NaiveDateTime>)]
) -> Vec<(String, String, f64, bool)> {

    let mut out = Vec::new();

    for i in 0..data.len() {
        for j in i + 1..data.len() {
            let (ref a_name, ref a, _) = data[i];
            let (ref b_name, ref b, _) = data[j];

            let n = a.len().min(b.len());
            let a = &a[..n];
            let b = &b[..n];

            if let Ok(res) = engle_granger(a, b) {
                out.push((
                    a_name.clone(),
                    b_name.clone(),
                    res.adf_stat,
                    res.is_cointegrated,
                ));
            }
        }
    }

    out
}

