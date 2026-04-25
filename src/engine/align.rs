use chrono::NaiveDateTime;

pub fn align_series(
    series: &[(String, Vec<f64>, Vec<NaiveDateTime>)]
) -> Vec<(String, Vec<f64>, Vec<NaiveDateTime>)> {

    // Find minimum length across all series
    let min_len = series
        .iter()
        .map(|(_, prices, _)| prices.len())
        .min()
        .unwrap();

    // Trim all series to same length
    series
        .iter()
        .map(|(name, prices, timestamps)| {
            let start = prices.len() - min_len;

            (
                name.clone(),
                prices[start..].to_vec(),
                timestamps[start..].to_vec(),
            )
        })
        .collect()
}
