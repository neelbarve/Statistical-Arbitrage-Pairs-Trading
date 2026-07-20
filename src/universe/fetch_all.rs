use anyhow::Result;
use crate::data::fetch::fetch_prices;

pub async fn fetch_universe(tickers: &[&str]) -> Result<Vec<(String, Vec<f64>, Vec<chrono::NaiveDateTime>)>> {
    let mut out = Vec::new();

    for &t in tickers {
        println!("Fetching {}", t);

        let (prices, timestamps) = fetch_prices(t).await?;
        out.push((t.to_string(), prices, timestamps));

        // Stay polite to the data provider.
        tokio::time::sleep(std::time::Duration::from_millis(800)).await;
    }

    Ok(out)
}
