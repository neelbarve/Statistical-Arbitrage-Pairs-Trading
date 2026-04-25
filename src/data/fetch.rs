use anyhow::Result;
use serde_json::Value;
use chrono::NaiveDateTime;
use tokio::time::{sleep, Duration};

pub async fn fetch_prices(symbol: &str) -> Result<(Vec<f64>, Vec<NaiveDateTime>)> {
    let api_key = "6e1563cb89207d58f3ec6da73e3a1d992871fc52";

    let url = format!(
        "https://api.tiingo.com/tiingo/daily/{}/prices?startDate=2010-01-01&resampleFreq=daily&token={}",
        symbol,
        api_key
    );

    // Try up to 5 times if rate-limited
    for attempt in 1..=5 {
        let resp_text = reqwest::get(&url).await?.text().await?;

        // Parse JSON
        let json: Value = serde_json::from_str(&resp_text)
            .map_err(|_| anyhow::anyhow!("Invalid JSON returned: {}", resp_text))?;

        // CASE 1: Tiingo returned an error object
        if json.is_object() {
            let msg = resp_text.to_lowercase();

            // Rate limit hit → wait and retry
            if msg.contains("allocation") || msg.contains("limit") {
                println!("Rate limit hit for {} — retrying attempt {}/5", symbol, attempt);
                sleep(Duration::from_secs(60)).await;
                continue;
            }

            // Other errors → bail
            anyhow::bail!("Tiingo error for {}: {}", symbol, resp_text);
        }

        // CASE 2: Expected array of price objects
        let arr = json.as_array().ok_or_else(|| {
            anyhow::anyhow!("Unexpected JSON format for {}: {}", symbol, resp_text)
        })?;

        let mut closes = Vec::new();
        let mut timestamps = Vec::new();

        for entry in arr {
            if let Some(close) = entry["close"].as_f64() {
                closes.push(close);
            }
            if let Some(date_str) = entry["date"].as_str() {
                // Tiingo format: "2024-01-05T00:00:00.000Z"
                let ts = NaiveDateTime::parse_from_str(date_str, "%Y-%m-%dT%H:%M:%S%.fZ")?;
                timestamps.push(ts);
            }
        }

        if closes.len() < 50 {
            anyhow::bail!("Not enough data for {}", symbol);
        }

        return Ok((closes, timestamps));
    }

    anyhow::bail!("Failed to fetch {} after retries", symbol)
}
