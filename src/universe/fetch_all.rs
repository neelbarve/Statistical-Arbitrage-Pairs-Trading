use anyhow::Result;
use crate::data::fetch::fetch_prices;

pub async fn fetch_universe(tickers: &[&str]) -> Result<Vec<(String, Vec<f64>, Vec<chrono::NaiveDateTime>)>> {
    let mut out = Vec::new();

    for &t in tickers {
        println!("Fetching {}", t);

        // CHANGE: fetch_prices must now return (prices, timestamps)
        let (prices, timestamps) = fetch_prices(t).await?;
        out.push((t.to_string(), prices, timestamps));

        tokio::time::sleep(std::time::Duration::from_millis(1200)).await;
    }

    Ok(out)
}
