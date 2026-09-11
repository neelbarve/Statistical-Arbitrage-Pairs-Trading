// Exports the raw, bar-by-bar equity curve for every backtested pair as
// JSON, so a web page (e.g. an interactive dashboard or pitch deck) can
// draw REAL, zoomable/hoverable line charts instead of embedding the
// static PNG images this project also produces. The CSV/HTML exports
// elsewhere only carry SUMMARY numbers (final Sharpe, max drawdown, etc.);
// this file is the one place the full day-by-day equity path leaves Rust.
//
// Written by hand (string formatting) rather than pulling in a JSON
// serialization crate's derive macros, matching how every other export in
// this project (export.rs's CSV, export_html.rs's HTML) is already built:
// plain `write!`/`writeln!` calls, no extra dependency or macro magic
// needed for a format this simple.
use anyhow::Result;
use chrono::NaiveDate;
use std::fs::File;
use std::io::Write;

pub struct EquityCurveEntry {
    pub pair: String,
    pub threshold: f64,
    pub sharpe: f64,
    pub sortino: f64,
    pub half_life_bars: Option<f64>,
    pub max_drawdown: f64,
    pub net_pnl: f64,
    pub volatility: f64,
    pub trades: usize,
    pub dates: Vec<NaiveDate>,
    pub equity: Vec<f64>,
}

/// Writes `entries` as a single JSON object: `{"series": [...]}`. Each
/// array entry is one (pair, threshold) backtest, carrying both its
/// summary stats (for tooltips/legends) and its full `dates`/`equity`
/// arrays (for the actual chart line). `dates` and `equity` are always the
/// same length -- one entry per traded bar.
pub fn export_equity_curves(path: &str, entries: &[EquityCurveEntry]) -> Result<()> {
    let mut file = File::create(path)?;

    writeln!(file, "{{")?;
    writeln!(file, "  \"series\": [")?;

    for (i, e) in entries.iter().enumerate() {
        writeln!(file, "    {{")?;
        writeln!(file, "      \"pair\": \"{}\",", json_escape(&e.pair))?;
        writeln!(file, "      \"threshold\": {},", e.threshold)?;
        writeln!(file, "      \"sharpe\": {},", finite_or_null(e.sharpe))?;
        writeln!(file, "      \"sortino\": {},", finite_or_null(e.sortino))?;
        writeln!(
            file,
            "      \"halfLifeBars\": {},",
            e.half_life_bars.map(finite_or_null).unwrap_or_else(|| "null".to_string())
        )?;
        writeln!(file, "      \"maxDrawdown\": {},", finite_or_null(e.max_drawdown))?;
        writeln!(file, "      \"netPnl\": {},", finite_or_null(e.net_pnl))?;
        writeln!(file, "      \"volatility\": {},", finite_or_null(e.volatility))?;
        writeln!(file, "      \"trades\": {},", e.trades)?;

        write!(file, "      \"dates\": [")?;
        for (j, d) in e.dates.iter().enumerate() {
            if j > 0 {
                write!(file, ",")?;
            }
            write!(file, "\"{}\"", d.format("%Y-%m-%d"))?;
        }
        writeln!(file, "],")?;

        write!(file, "      \"equity\": [")?;
        for (j, v) in e.equity.iter().enumerate() {
            if j > 0 {
                write!(file, ",")?;
            }
            write!(file, "{}", finite_or_null(*v))?;
        }
        writeln!(file, "]")?;

        write!(file, "    }}")?;
        if i + 1 < entries.len() {
            writeln!(file, ",")?;
        } else {
            writeln!(file)?;
        }
    }

    writeln!(file, "  ]")?;
    writeln!(file, "}}")?;
    Ok(())
}

// JSON has no representation for NaN/Infinity -- guard every float with
// this so a pathological input (e.g. a division by exactly zero slipping
// through somewhere upstream) produces valid, parseable `null` in the
// output instead of a literal "NaN" token that would break every
// JSON.parse() call on the consuming end.
fn finite_or_null(v: f64) -> String {
    if v.is_finite() {
        format!("{}", v)
    } else {
        "null".to_string()
    }
}

// Ticker pair labels only ever contain letters, digits, spaces and "/" in
// this project, but escaping defensively costs nothing and avoids ever
// emitting invalid JSON if that assumption changes later.
fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_valid_parseable_json_with_matching_array_lengths() {
        let dir = std::env::temp_dir().join(format!("statarbrust-equity-json-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("equity_curves.json");

        let entries = vec![
            EquityCurveEntry {
                pair: "COP / FANG".to_string(),
                threshold: 1.5,
                sharpe: 0.379,
                sortino: 0.940,
                half_life_bars: Some(38.9),
                max_drawdown: 5.36,
                net_pnl: 16.4,
                volatility: 3.25,
                trades: 54,
                dates: vec![
                    NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
                    NaiveDate::from_ymd_opt(2020, 1, 2).unwrap(),
                ],
                equity: vec![0.0, 1.5],
            },
            EquityCurveEntry {
                pair: "SLB / HAL".to_string(),
                threshold: 2.0,
                sharpe: 0.0,
                sortino: 0.0,
                half_life_bars: None,
                max_drawdown: 0.0,
                net_pnl: 0.0,
                volatility: 0.0,
                trades: 0,
                dates: vec![NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()],
                equity: vec![0.0],
            },
        ];

        export_equity_curves(path.to_str().unwrap(), &entries).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();

        // Parse it back with a tiny hand-rolled check rather than pulling
        // in a JSON parser crate just for this test: every array must be
        // well-formed and every `dates`/`equity` pair the same length.
        assert!(text.contains("\"pair\": \"COP / FANG\""));
        assert!(text.contains("\"halfLifeBars\": null")); // SLB / HAL had no measurable half-life
        assert!(!text.contains("NaN"));
        assert!(!text.contains("Infinity"));

        // Round-trip through serde_json (already a project dependency, so
        // this is a legitimate correctness check, not a new dependency).
        let parsed: serde_json::Value = serde_json::from_str(&text).expect("must be valid JSON");
        let series = parsed["series"].as_array().unwrap();
        assert_eq!(series.len(), 2);
        for entry in series {
            let dates = entry["dates"].as_array().unwrap();
            let equity = entry["equity"].as_array().unwrap();
            assert_eq!(dates.len(), equity.len(), "dates/equity length mismatch for {}", entry["pair"]);
        }

        std::fs::remove_file(path).ok();
        std::fs::remove_dir_all(dir).ok();
    }
}
