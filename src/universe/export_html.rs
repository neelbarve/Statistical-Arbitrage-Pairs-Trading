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
// grid, and img src attributes written relative to the dashboard file's own
// directory (chart_files/paths passed in from main.rs are "output/..." but
// the dashboard itself lives inside output/, so the plain "output/..." src
// used to 404 in the browser -- this is the fix for that).
pub fn export_full_dashboard(
    path: &str,
    charts: &[(String, String)],
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

    if !rankings.is_empty() {
        writeln!(file, "    <section>")?;
        writeln!(file, "      <h2>Pairs ranked best to trade (by Sharpe ratio)</h2>")?;
        writeln!(file, "      <div class=\"table-wrap\">")?;
        writeln!(file, "      <table>")?;
        writeln!(file, "        <thead><tr><th>Rank</th><th>Pair</th><th>Entry SD</th><th>Risk-Free Rate</th><th>Volatility</th><th>Sharpe</th><th>Max Drawdown</th><th>Transaction Cost</th><th>Net PnL</th><th>Trades</th></tr></thead>")?;
        writeln!(file, "        <tbody>")?;
        for (i, r) in rankings.iter().enumerate() {
            let pnl_class = if r.total_pnl >= 0.0 { "pos" } else { "neg" };
            writeln!(
                file,
                "          <tr><td>{}</td><td>{} / {}</td><td>{:.1}</td><td>{:.2}%</td><td>{:.4}</td><td>{:.3}</td><td>{:.4}</td><td>{:.4}</td><td class=\"{}\">{:.4}</td><td>{}</td></tr>",
                i + 1,
                r.pair.0,
                r.pair.1,
                r.threshold,
                r.risk_free_rate * 100.0,
                r.volatility,
                r.sharpe_ratio,
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

    writeln!(file, "    <section class=\"chart-grid\">")?;

    for (title, path) in charts {
        // Charts are written next to this dashboard file (both under
        // output/), so the <img> src must be just the filename -- not the
        // "output/..." path used when the chart was written to disk.
        let filename = path.rsplit('/').next().unwrap_or(path);
        writeln!(file, "      <article class=\"card\">")?;
        writeln!(file, "        <h2>{}</h2>", title)?;
        writeln!(file, "        <img src=\"{}\" alt=\"{}\">", filename, title)?;
        writeln!(file, "        <div class=\"meta\">{}</div>", filename)?;
        writeln!(file, "      </article>")?;
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
        let charts = vec![
            ("AAA BBB signals".to_string(), "output/AAA_BBB_signals.png".to_string()),
        ];
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
        };

        std::fs::create_dir_all(&dir).unwrap();
        export_full_dashboard(path.to_str().unwrap(), &charts, std::slice::from_mut(&mut result)).unwrap();

        let html = std::fs::read_to_string(&path).unwrap();
        // The bug: img src used to still carry the "output/" prefix even
        // though the dashboard html lives inside output/ itself, which
        // 404'd in the browser. It must now be just the filename.
        assert!(html.contains("src=\"AAA_BBB_signals.png\""));
        assert!(!html.contains("src=\"output/"));
        assert!(html.contains("AAA / BBB"));
        assert!(html.contains("ranked best to trade"));

        std::fs::remove_file(path).ok();
        std::fs::remove_dir_all(dir).ok();
    }
}
