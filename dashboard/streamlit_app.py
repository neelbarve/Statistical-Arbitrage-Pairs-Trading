"""
Streamlit dashboard for statarbrust.

Reads the artifacts the Rust pipeline (`cargo run --release`) already
writes to output/ -- backtests_energy.csv plus the per-pair chart PNGs --
and presents them as an interactive ranking table + per-pair chart rows,
mirroring output/dashboard.html but filterable and sortable in the browser.

Run from the repo root:

    pip install -r dashboard/requirements.txt
    streamlit run dashboard/streamlit_app.py
"""

import re
import subprocess
import time
from pathlib import Path

import pandas as pd
import streamlit as st

REPO_ROOT = Path(__file__).resolve().parents[1]
OUTPUT_DIR = REPO_ROOT / "output"
CSV_PATH = OUTPUT_DIR / "backtests_energy.csv"

st.set_page_config(
    page_title="statarbrust — Pairs Trading Dashboard",
    page_icon="📈",
    layout="wide",
)

CHART_FILE_RE = re.compile(
    r"^(?P<a>[A-Z]+)_(?P<b>[A-Z]+)_(?:equity|(?P<tag>.+)_(?P<kind>signals|prices))\.png$"
)


def tag_to_threshold(tag: str) -> float:
    """'1_5sd' -> 1.5, '2sd' -> 2.0 (mirrors the Rust filename tag)."""
    return float(tag.removesuffix("sd").replace("_", "."))


@st.cache_data(show_spinner=False)
def load_results(csv_mtime: float) -> pd.DataFrame:
    # half_life_bars is "n/a" (text) for the rare case a pair reached the
    # CSV without a measurable half-life -- shouldn't normally happen since
    # main.rs skips such pairs before they're ever backtested, but treat it
    # as a proper missing value (NaN) rather than a stray string either way.
    df = pd.read_csv(CSV_PATH, na_values=["n/a"])
    df["pair"] = df["stock1"] + " / " + df["stock2"]
    return df


@st.cache_data(show_spinner=False)
def discover_pair_charts(_output_dir_listing: tuple) -> dict:
    """Map 'A / B' -> ordered list of (title, Path) for every chart PNG
    that belongs to that pair, discovered by filename rather than by
    recomputing Rust's float-formatting rules -- so this keeps working even
    if the naming scheme changes on the Rust side."""
    groups: dict[str, list[tuple[float, int, str, Path]]] = {}
    for path in OUTPUT_DIR.glob("*.png"):
        m = CHART_FILE_RE.match(path.name)
        if not m:
            continue
        pair_label = f"{m.group('a')} / {m.group('b')}"
        if m.group("tag") is None:
            # "<A>_<B>_equity.png"
            sort_key = (float("inf"), 2, "Equity Curve")
        else:
            threshold = tag_to_threshold(m.group("tag"))
            kind = m.group("kind")
            title = f"{threshold:g} SD {kind.capitalize()}"
            sort_key = (threshold, 0 if kind == "signals" else 1, title)
        groups.setdefault(pair_label, []).append((*sort_key, path))

    ordered = {}
    for pair_label, items in groups.items():
        items.sort(key=lambda t: t[:3])
        ordered[pair_label] = [(title, path) for *_, title, path in items]
    return ordered


def regenerate_data():
    with st.status("Running `cargo run --release`…", expanded=True) as status:
        try:
            proc = subprocess.run(
                ["cargo", "run", "--release"],
                cwd=REPO_ROOT,
                capture_output=True,
                text=True,
                timeout=600,
            )
        except FileNotFoundError:
            status.update(label="cargo not found on PATH", state="error")
            st.error("`cargo` isn't on PATH in this environment -- run `cargo run --release` manually, then reload this page.")
            return
        except subprocess.TimeoutExpired:
            status.update(label="Timed out after 10 minutes", state="error")
            return

        st.code((proc.stdout or "") + (proc.stderr or ""), language="text")
        if proc.returncode == 0:
            status.update(label="Done", state="complete")
            st.cache_data.clear()
            time.sleep(0.5)
            st.rerun()
        else:
            status.update(label=f"Failed (exit code {proc.returncode})", state="error")


st.title("📈 statarbrust — Pairs Trading Dashboard")
st.caption(
    "Cointegrated energy pairs, mean-reversion signals at 1.5 SD / 2.0 SD entry, "
    "and their backtest results. Data comes from `output/`, written by `cargo run --release`."
)

with st.expander("Methodology & references"):
    st.markdown(
        """
- **Cointegration**: Engle-Granger two-step test on log prices (proper Augmented Dickey-Fuller,
  AIC-selected lag order, real MacKinnon (2010) critical values), tested in **both** regression
  directions since the test's power isn't symmetric in finite samples (Engle & Granger, 1987,
  *Econometrica*; Chan, *Algorithmic Trading*, 2013).
- **Hedge ratio & spread**: **walk-forward** OLS on log prices, refit on a trailing 252-bar
  (~1 year) window every 21 bars — never fit on the whole history at once, which would let the
  model "see" future prices. The formation period (first 252 bars) is excluded from trading.
- **Entry signal**: rolling z-score of the spread (10-day mean vs. 40-day mean, divided by the
  40-day standard deviation) — **1 SD = 1 standard deviation of the spread over that trailing
  40-day window**. Entries at 1.5 SD / 2.0 SD, exits at 0.5 SD.
- **Half-life**: Ornstein-Uhlenbeck mean-reversion half-life (Uhlenbeck & Ornstein, 1930),
  estimated by regressing the spread's day-over-day change on its own lagged level. Pairs with
  no measurable mean reversion are excluded from trading even if they passed cointegration.
- **Sharpe / Sortino**: annualized (×√252) from the backtest's own bar-by-bar P&L, net of
  transaction costs, against a fixed annual risk-free rate. Sortino only penalizes downside
  deviations below that same hurdle (Sortino & Price, 1994).
- **Where this simplifies**: pair *selection* (which tickers are cointegrated) still uses the
  full price history rather than a periodically re-formed universe (Gatev, Goetzmann &
  Rouwenhorst, 2006, use a rolling 12-month formation / 6-month trading cycle) — only the
  *traded* hedge ratio is walk-forward. See `README.md` for the full write-up, including every
  place this project's choices diverge from one reference or another and why.
        """
    )

with st.sidebar:
    st.header("Data")
    if CSV_PATH.exists():
        mtime = CSV_PATH.stat().st_mtime
        st.caption(f"Last generated: {time.strftime('%Y-%m-%d %H:%M:%S', time.localtime(mtime))}")
    else:
        st.warning("No `output/backtests_energy.csv` found yet.")
    if st.button("🔄 Regenerate data (runs the Rust pipeline)", use_container_width=True):
        regenerate_data()

if not CSV_PATH.exists():
    st.error(
        "No results yet. Run `cargo run --release` from the repo root first "
        "(requires network access to fetch prices from Yahoo Finance), or click "
        "**Regenerate data** in the sidebar."
    )
    st.stop()

df = load_results(CSV_PATH.stat().st_mtime)
chart_groups = discover_pair_charts(tuple(sorted(p.name for p in OUTPUT_DIR.glob("*.png"))))

with st.sidebar:
    st.header("Filters")
    thresholds = sorted(df["entry_threshold_sd"].unique())
    selected_thresholds = st.multiselect("Entry threshold (SD)", thresholds, default=thresholds)
    min_trades = st.slider("Minimum trades", 0, int(df["trades"].max()), 0)
    pair_query = st.text_input("Filter by ticker (e.g. XOM)").strip().upper()

filtered = df[df["entry_threshold_sd"].isin(selected_thresholds) & (df["trades"] >= min_trades)]
if pair_query:
    filtered = filtered[filtered["pair"].str.contains(pair_query)]

# --- KPIs -------------------------------------------------------------
best = df.iloc[0]
col1, col2, col3, col4 = st.columns(4)
col1.metric("Best pair (Sharpe)", best["pair"], f"{best['sharpe_ratio']:.3f}")
col2.metric("Best net PnL", f"{df.loc[df['net_pnl'].idxmax(), 'pair']}", f"{df['net_pnl'].max():.2f}")
col3.metric("Pairs analyzed", df["pair"].nunique())
col4.metric("Rows shown", len(filtered))

# --- Ranking table ------------------------------------------------------
st.subheader("Pairs ranked best to trade (by Sharpe ratio)")

display_cols = {
    "rank": "Rank",
    "pair": "Pair",
    "entry_threshold_sd": "Entry SD",
    "half_life_bars": "Half-Life (bars)",
    "risk_free_rate": "Risk-Free Rate",
    "volatility": "Volatility",
    "sharpe_ratio": "Sharpe",
    "sortino_ratio": "Sortino",
    "max_drawdown": "Max Drawdown",
    "total_costs": "Transaction Cost",
    "net_pnl": "Net PnL",
    "trades": "Trades",
}
table = filtered[list(display_cols)].rename(columns=display_cols)

def _na_or(fmt):
    # pandas Styler.format callables receive the raw (possibly-NaN) cell
    # value -- render a missing half-life as "n/a" instead of the string
    # "nan", which would otherwise look like a bug in the table.
    return lambda v: "n/a" if pd.isna(v) else fmt.format(v)

styled = table.style.map(
    lambda v: "color: #16a34a" if v >= 0 else "color: #dc2626", subset=["Net PnL"]
).format({
    "Entry SD": "{:.1f}".format,
    "Half-Life (bars)": _na_or("{:.1f}"),
    "Risk-Free Rate": "{:.2%}".format,
    "Volatility": "{:.4f}".format,
    "Sharpe": "{:.3f}".format,
    "Sortino": "{:.3f}".format,
    "Max Drawdown": "{:.4f}".format,
    "Transaction Cost": "{:.4f}".format,
    "Net PnL": "{:.4f}".format,
})
st.dataframe(styled, use_container_width=True, hide_index=True)

st.download_button(
    "Download filtered results as CSV",
    table.to_csv(index=False).encode("utf-8"),
    file_name="backtests_energy_filtered.csv",
    mime="text/csv",
)

# --- Per-pair charts, side by side, best pair first ----------------------
st.subheader("Charts by pair")
st.caption("Each pair's own charts sit in one row -- price+signals for both thresholds, plus the equity curve.")

ranked_pairs = filtered.drop_duplicates("pair")["pair"].tolist()
for i, pair_label in enumerate(ranked_pairs):
    charts = chart_groups.get(pair_label, [])
    if not charts:
        continue
    row = filtered[filtered["pair"] == pair_label].iloc[0]
    half_life_label = "n/a" if pd.isna(row["half_life_bars"]) else f"{row['half_life_bars']:.1f} bars"
    with st.expander(
        f"#{int(row['rank'])} — {pair_label} — Sharpe {row['sharpe_ratio']:.3f} — half-life {half_life_label}",
        expanded=(i < 3),
    ):
        cols = st.columns(len(charts))
        for col, (title, path) in zip(cols, charts):
            with col:
                st.image(str(path), caption=title, use_container_width=True)
