use std::fs::File;
use std::io::Write;
use crate::engine::backtest::BacktestResult;

pub fn export_svg_dashboard(path: &str, svg_paths: &[String]) -> anyhow::Result<()> {
    let mut file = File::create(path)?;

    writeln!(file, "<html><body>")?;

    for svg in svg_paths {
        writeln!(file, "<h2>{}</h2>", svg)?;
        writeln!(file, "<object type=\"image/svg+xml\" data=\"{}\"></object>", svg)?;
        writeln!(file, "<hr>")?;
    }

    writeln!(file, "</body></html>")?;
    Ok(())
}

pub fn export_chart_dashboard(path: &str, charts: &[(String, String)]) -> anyhow::Result<()> {
    let mut file = File::create(path)?;

    writeln!(file, "<!DOCTYPE html>")?;
    writeln!(file, "<html lang=\"en\">")?;
    writeln!(file, "<head>")?;
    writeln!(file, "  <meta charset=\"utf-8\">")?;
    writeln!(file, "  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">")?;
    writeln!(file, "  <title>Strategy Chart Dashboard</title>")?;
    writeln!(file, "  <style>")?;
    writeln!(file, "    :root {{ color-scheme: light dark; }}")?;
    writeln!(file, "    body {{ font-family: Inter, Segoe UI, Arial, sans-serif; margin: 0; background: #07111f; color: #f5f7fb; }}")?;
    writeln!(file, "    .page {{ max-width: 1400px; margin: 0 auto; padding: 24px; }}")?;
    writeln!(file, "    .hero {{ background: linear-gradient(135deg, #16304f, #0d1724); border-radius: 18px; padding: 24px 28px; box-shadow: 0 16px 40px rgba(0,0,0,.25); margin-bottom: 20px; }}")?;
    writeln!(file, "    .hero h1 {{ margin: 0 0 8px; font-size: 1.8rem; }}")?;
    writeln!(file, "    .hero p {{ margin: 0; color: #b6c7dd; }}")?;
    writeln!(file, "    .chart-grid {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(320px, 1fr)); gap: 18px; }}")?;
    writeln!(file, "    .card {{ background: #10243b; border: 1px solid #23374f; border-radius: 16px; padding: 14px; box-shadow: 0 10px 24px rgba(0,0,0,.18); }}")?;
    writeln!(file, "    .card h2 {{ margin: 0 0 10px; font-size: 1.05rem; color: #f8fafc; }}")?;
    writeln!(file, "    .card img {{ width: 100%; height: auto; border-radius: 12px; display: block; background: white; }}")?;
    writeln!(file, "    .card .meta {{ margin-top: 8px; font-size: 0.9rem; color: #8ca0bc; }}")?;
    writeln!(file, "  </style>")?;
    writeln!(file, "</head>")?;
    writeln!(file, "<body>")?;
    writeln!(file, "  <div class=\"page\">")?;
    writeln!(file, "    <section class=\"hero\">")?;
    writeln!(file, "      <h1>Signal and price dashboard</h1>")?;
    writeln!(file, "      <p>All generated charts from the current run are shown together for quick review.</p>")?;
    writeln!(file, "    </section>")?;
    writeln!(file, "    <section class=\"chart-grid\">")?;

    for (title, path) in charts {
        writeln!(file, "      <article class=\"card\">")?;
        writeln!(file, "        <h2>{}</h2>", title)?;
        writeln!(file, "        <img src=\"{}\" alt=\"{}\">", path, title)?;
        writeln!(file, "        <div class=\"meta\">{}</div>", path)?;
        writeln!(file, "      </article>")?;
    }

    writeln!(file, "    </section>")?;
    writeln!(file, "  </div>")?;
    writeln!(file, "</body>")?;
    writeln!(file, "</html>")?;
    Ok(())
}

// Same layout as export_chart_dashboard, plus a "best pairs to trade"
// ranking table (sorted best-first by the caller) rendered above the chart
// groups, and img src attributes written relative to the dashboard file's
// own directory (paths passed in from main.rs are "output/..." but the
// dashboard itself lives inside output/, so the plain "output/..." src used
// to 404 in the browser -- this is the fix for that).
//
// `chart_groups` is one entry per pair -- (pair label, that pair's charts)
// -- so every chart belonging to the same pair renders together, side by
// side, instead of being interleaved with every other pair in one flat grid.
pub fn export_full_dashboard(
    path: &str,
    chart_groups: &[(String, Vec<(String, String)>)],
    rankings: &[BacktestResult],
) -> anyhow::Result<()> {
    let mut file = File::create(path)?;

    writeln!(file, "<!DOCTYPE html>")?;
    writeln!(file, "<html lang=\"en\">")?;
    writeln!(file, "<head>")?;
    writeln!(file, "  <meta charset=\"utf-8\">")?;
    writeln!(file, "  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">")?;
    writeln!(file, "  <title>Strategy Chart Dashboard</title>")?;
    writeln!(file, "  <style>")?;
    writeln!(file, "    :root {{ color-scheme: light dark; }}")?;
    writeln!(file, "    body {{ font-family: Inter, Segoe UI, Arial, sans-serif; margin: 0; background: #07111f; color: #f5f7fb; }}")?;
    writeln!(file, "    .page {{ max-width: 1400px; margin: 0 auto; padding: 24px; }}")?;
    writeln!(file, "    .hero {{ background: linear-gradient(135deg, #16304f, #0d1724); border-radius: 18px; padding: 24px 28px; box-shadow: 0 16px 40px rgba(0,0,0,.25); margin-bottom: 20px; }}")?;
    writeln!(file, "    .hero h1 {{ margin: 0 0 8px; font-size: 1.8rem; }}")?;
    writeln!(file, "    .hero p {{ margin: 0; color: #b6c7dd; }}")?;
    writeln!(file, "    .table-wrap {{ overflow-x: auto; margin-bottom: 28px; -webkit-overflow-scrolling: touch; }}")?;
    writeln!(file, "    table {{ width: 100%; min-width: 640px; border-collapse: collapse; }}")?;
    writeln!(file, "    th, td {{ text-align: left; padding: 8px 12px; border-bottom: 1px solid #23374f; font-size: 0.92rem; white-space: nowrap; }}")?;
    writeln!(file, "    th {{ color: #8ca0bc; font-weight: 600; }}")?;
    writeln!(file, "    tr:hover {{ background: #0d1c30; }}")?;
    writeln!(file, "    .pos {{ color: #4ade80; }}")?;
    writeln!(file, "    .neg {{ color: #f87171; }}")?;
    writeln!(file, "    .chart-grid {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(320px, 1fr)); gap: 18px; }}")?;
    writeln!(file, "    .card {{ background: #10243b; border: 1px solid #23374f; border-radius: 16px; padding: 14px; box-shadow: 0 10px 24px rgba(0,0,0,.18); }}")?;
    writeln!(file, "    .card h2, .card h3 {{ margin: 0 0 10px; font-size: 1.05rem; color: #f8fafc; }}")?;
    writeln!(file, "    .card img {{ width: 100%; height: auto; border-radius: 12px; display: block; background: white; }}")?;
    writeln!(file, "    .card .meta {{ margin-top: 8px; font-size: 0.9rem; color: #8ca0bc; }}")?;
    writeln!(file, "    .pair-group {{ margin-bottom: 30px; }}")?;
    writeln!(file, "    .pair-group h2 {{ margin: 0 0 12px; font-size: 1.25rem; color: #f8fafc; border-bottom: 1px solid #23374f; padding-bottom: 8px; }}")?;
    // Each pair's own charts sit in their own grid (not the shared
    // .chart-grid) so they lay out side by side within the pair, and only
    // wrap onto a new line -- rather than shrinking below readable size --
    // once the viewport can't fit them all in one row.
    writeln!(file, "    .pair-row {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(260px, 1fr)); gap: 14px; }}")?;
    writeln!(file, "    details.methodology {{ background: #10243b; border: 1px solid #23374f; border-radius: 14px; padding: 4px 20px 4px; margin-bottom: 20px; }}")?;
    writeln!(file, "    details.methodology summary {{ cursor: pointer; padding: 14px 0; font-weight: 600; color: #f8fafc; }}")?;
    writeln!(file, "    details.methodology p, details.methodology li {{ color: #b6c7dd; font-size: 0.92rem; line-height: 1.5; }}")?;
    writeln!(file, "    details.methodology ul {{ margin: 0 0 16px; padding-left: 20px; }}")?;
    writeln!(file, "    details.methodology code {{ background: #0d1c30; padding: 1px 5px; border-radius: 4px; font-size: 0.88em; }}")?;
    writeln!(file, "  </style>")?;
    writeln!(file, "</head>")?;
    writeln!(file, "<body>")?;
    writeln!(file, "  <div class=\"page\">")?;
    writeln!(file, "    <section class=\"hero\">")?;
    writeln!(file, "      <h1>Signal and price dashboard</h1>")?;
    writeln!(file, "      <p>Each pair's charts are grouped side by side below the ranking table, best pair first.</p>")?;
    writeln!(file, "    </section>")?;

    writeln!(file, "    <details class=\"methodology\">")?;
    writeln!(file, "      <summary>Methodology &amp; references (click to expand)</summary>")?;
    writeln!(file, "      <ul>")?;
    writeln!(file, "        <li><b>Cointegration</b>: Engle-Granger two-step test on log prices, with proper Augmented Dickey-Fuller (AIC-selected lag order) and MacKinnon (2010) response-surface critical values -- tested in both regression directions, since Engle-Granger's finite-sample power isn't symmetric (Engle &amp; Granger, 1987, <i>Econometrica</i>; Chan, <i>Algorithmic Trading</i>, 2013).</li>")?;
    writeln!(file, "        <li><b>Hedge ratio &amp; spread</b>: walk-forward OLS on log prices, refit on a trailing 252-bar (~1 year) window every 21 bars -- never fit on the whole history at once, which would let the model \"see\" future prices. Formation-period bars before the first full window are excluded from trading entirely.</li>")?;
    writeln!(file, "        <li><b>Entry signal</b>: rolling z-score of the spread (10-day mean vs. 40-day mean, divided by the 40-day standard deviation) -- 1&nbsp;SD = 1 standard deviation of the spread over that trailing 40-day window. Entries at 1.5&nbsp;SD and 2.0&nbsp;SD, exits at 0.5&nbsp;SD.</li>")?;
    writeln!(file, "        <li><b>Half-life</b>: Ornstein-Uhlenbeck mean-reversion half-life (Uhlenbeck &amp; Ornstein, 1930), estimated by regressing the spread's day-over-day change on its own lagged level. Pairs with no measurable mean reversion are excluded from trading even if they passed the cointegration test.</li>")?;
    writeln!(file, "        <li><b>Sharpe / Sortino</b>: annualized (&times;&radic;252) from the backtest's own bar-by-bar P&amp;L, net of transaction costs, against a {:.0}% annual risk-free rate. Sortino only penalizes downside deviations below that same hurdle (Sortino &amp; Price, 1994).</li>", rankings.first().map(|r| r.risk_free_rate * 100.0).unwrap_or(4.0))?;
    writeln!(file, "        <li><b>Where this project simplifies</b>: pair <i>selection</i> (which tickers are cointegrated) still uses the full price history rather than a periodically re-formed universe (Gatev, Goetzmann &amp; Rouwenhorst, 2006, use a rolling 12-month formation / 6-month trading cycle); only the traded hedge ratio is walk-forward. See <code>README.md</code> Assumptions &amp; Limitations for the full list, including where this project's choices diverge from one reference or another.</li>")?;
    writeln!(file, "      </ul>")?;
    writeln!(file, "    </details>")?;

    if !rankings.is_empty() {
        writeln!(file, "    <section>")?;
        writeln!(file, "      <h2>Pairs ranked best to trade (by Sharpe ratio)</h2>")?;
        writeln!(file, "      <div class=\"table-wrap\">")?;
        writeln!(file, "      <table>")?;
        writeln!(file, "        <thead><tr><th>Rank</th><th>Pair</th><th>Entry SD</th><th>Half-Life (bars)</th><th>Risk-Free Rate</th><th>Volatility</th><th>Sharpe</th><th>Sortino</th><th>Max Drawdown</th><th>Transaction Cost</th><th>Net PnL</th><th>Trades</th></tr></thead>")?;
        writeln!(file, "        <tbody>")?;
        for (i, r) in rankings.iter().enumerate() {
            let pnl_class = if r.total_pnl >= 0.0 { "pos" } else { "neg" };
            let half_life_str = match r.half_life_bars {
                Some(h) => format!("{:.1}", h),
                None => "n/a".to_string(),
            };
            writeln!(
                file,
                "          <tr><td>{}</td><td>{} / {}</td><td>{:.1}</td><td>{}</td><td>{:.2}%</td><td>{:.4}</td><td>{:.3}</td><td>{:.3}</td><td>{:.4}</td><td>{:.4}</td><td class=\"{}\">{:.4}</td><td>{}</td></tr>",
                i + 1,
                r.pair.0,
                r.pair.1,
                r.threshold,
                half_life_str,
                r.risk_free_rate * 100.0,
                r.volatility,
                r.sharpe_ratio,
                r.sortino_ratio,
                r.max_drawdown,
                r.total_costs,
                pnl_class,
                r.total_pnl,
                r.trades,
            )?;
        }
        writeln!(file, "        </tbody>")?;
        writeln!(file, "      </table>")?;
        writeln!(file, "      </div>")?;
        writeln!(file, "    </section>")?;
    }

    writeln!(file, "    <section>")?;

    for (pair_label, charts) in chart_groups {
        writeln!(file, "      <div class=\"pair-group\">")?;
        writeln!(file, "        <h2>{}</h2>", pair_label)?;
        writeln!(file, "        <div class=\"pair-row\">")?;
        for (title, path) in charts {
            // Charts are written next to this dashboard file (both under
            // output/), so the <img> src must be just the filename -- not
            // the "output/..." path used when the chart was written to disk.
            let filename = path.rsplit('/').next().unwrap_or(path);
            writeln!(file, "          <article class=\"card\">")?;
            writeln!(file, "            <h3>{}</h3>", title)?;
            writeln!(file, "            <img src=\"{}\" alt=\"{} {}\">", filename, pair_label, title)?;
            writeln!(file, "            <div class=\"meta\">{}</div>", filename)?;
            writeln!(file, "          </article>")?;
        }
        writeln!(file, "        </div>")?;
        writeln!(file, "      </div>")?;
    }

    writeln!(file, "    </section>")?;
    writeln!(file, "  </div>")?;
    writeln!(file, "</body>")?;
    writeln!(file, "</html>")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_dashboard_with_cards_and_images() {
        let dir = std::env::temp_dir().join(format!("statarbrust-dashboard-test-{}", std::process::id()));
        let path = dir.join("dashboard.html");
        let charts = vec![
            ("Signals".to_string(), "signals.png".to_string()),
            ("Prices".to_string(), "prices.png".to_string()),
        ];

        std::fs::create_dir_all(&dir).unwrap();
        export_chart_dashboard(path.to_str().unwrap(), &charts).unwrap();

        let html = std::fs::read_to_string(&path).unwrap();
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("chart-grid"));
        assert!(html.contains("Signals"));
        assert!(html.contains("signals.png"));

        std::fs::remove_file(path).ok();
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn full_dashboard_rewrites_img_src_relative_to_dashboard_and_lists_rankings() {
        let dir = std::env::temp_dir().join(format!("statarbrust-full-dashboard-test-{}", std::process::id()));
        let path = dir.join("dashboard.html");
        // Mirror how main.rs builds chart paths: "output/<file>.png", while
        // the dashboard file itself is also written into output/.
        let chart_groups = vec![(
            "AAA / BBB".to_string(),
            vec![
                ("1.5 SD Signals".to_string(), "output/AAA_BBB_1_5sd_signals.png".to_string()),
                ("1.5 SD Prices".to_string(), "output/AAA_BBB_1_5sd_prices.png".to_string()),
                ("Equity Curve".to_string(), "output/AAA_BBB_equity.png".to_string()),
            ],
        )];
        let mut result = BacktestResult {
            pair: ("AAA".into(), "BBB".into()),
            threshold: 1.5,
            total_pnl: 12.5,
            total_costs: 0.5,
            trades: 3,
            equity_curve: vec![0.0, 5.0, 12.5],
            max_drawdown: 1.0,
            risk_free_rate: 0.04,
            volatility: 2.1,
            sharpe_ratio: 0.8,
            sortino_ratio: 1.1,
            half_life_bars: Some(12.5),
        };

        std::fs::create_dir_all(&dir).unwrap();
        export_full_dashboard(path.to_str().unwrap(), &chart_groups, std::slice::from_mut(&mut result)).unwrap();

        let html = std::fs::read_to_string(&path).unwrap();
        // The bug: img src used to still carry the "output/" prefix even
        // though the dashboard html lives inside output/ itself, which
        // 404'd in the browser. It must now be just the filename.
        assert!(html.contains("src=\"AAA_BBB_1_5sd_signals.png\""));
        assert!(!html.contains("src=\"output/"));
        assert!(html.contains("AAA / BBB"));
        assert!(html.contains("ranked best to trade"));
        // The pair's charts must all be grouped under one pair-row, not
        // scattered into a single flat grid mixed with other pairs.
        assert!(html.contains("pair-row"));

        std::fs::remove_file(path).ok();
        std::fs::remove_dir_all(dir).ok();
    }
}
