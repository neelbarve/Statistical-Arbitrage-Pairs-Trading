use anyhow::Result;
use chrono::NaiveDateTime;
use time::{macros::datetime, OffsetDateTime};
use yahoo_finance_api as yahoo;

pub async fn fetch_prices(symbol: &str) -> Result<(Vec<f64>, Vec<NaiveDateTime>)> {
    let provider = yahoo::YahooConnector::new()?;
    let start = datetime!(2010-01-01 0:00:00 UTC);
    let end = OffsetDateTime::now_utc();
    let response = provider.get_quote_history(symbol, start, end).await?;

    let mut quotes = response.quotes()?;
    if quotes.len() < 50 {
        anyhow::bail!("Not enough data for {}", symbol);
    }

    // Ensure ascending chronological order
    quotes.sort_by_key(|q| q.timestamp);

    let closes: Vec<f64> = quotes.iter().map(|q| q.adjclose).collect();
    let timestamps: Vec<NaiveDateTime> = quotes
        .iter()
        .map(|q| {
            NaiveDateTime::from_timestamp_opt(q.timestamp as i64, 0)
                .unwrap_or_else(|| NaiveDateTime::from_timestamp(0, 0))
        })
        .collect();

    Ok((closes, timestamps))
}
