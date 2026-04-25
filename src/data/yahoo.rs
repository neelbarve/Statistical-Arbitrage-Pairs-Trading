use yahoo_finance_api as yahoo;
use anyhow::Result;

pub async fn fetch_yahoo(symbol: &str) -> Result<Vec<f64>> {
    let provider = yahoo::YahooConnector::new();
    let response = provider
    .get_quote_history(symbol, "2010-01-01", "2024-12-31")
    .await?;
    let quotes = response.quotes()?;
    
    let closes: Vec<f64> = quotes.iter().map(|q| q.adjclose).collect();


    Ok(closes)
}
