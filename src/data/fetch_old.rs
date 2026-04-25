use anyhow::Result;
use serde_json::Value;

pub async fn fetch_prices(symbol: &str) -> Result<Vec<f64>> {
    let api_key = "6e1563cb89207d58f3ec6da73e3a1d992871fc52";

    // Request full daily history from 2010 to today
    let url = format!(
        "https://api.tiingo.com/tiingo/daily/{}/prices?startDate=2010-01-01&resampleFreq=daily&token={}",
        symbol,
        api_key
    );

    let resp_text = reqwest::get(&url).await?.text().await?;

    let json: Value = serde_json::from_str(&resp_text)
        .map_err(|_| anyhow::anyhow!("Invalid JSON returned: {}", resp_text))?;

    let arr = json.as_array().ok_or_else(|| anyhow::anyhow!("Unexpected JSON format"))?;

    let mut closes = Vec::new();

    for entry in arr {
        if let Some(close) = entry["close"].as_f64() {
            closes.push(close);
        }
    }

    if closes.len() < 50 {
        println!("DEBUG JSON:\n{}", json);
        anyhow::bail!("Not enough historical data for {}", symbol);
    }

    Ok(closes)
}
