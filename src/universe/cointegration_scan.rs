// Screens every ticker pair in the universe for cointegration, using the
// Engle-Granger two-step test (see src/model/cointegration.rs and
// src/model/adf.rs for the actual statistics).
//
// ----------------------------------------------------------------------
// WHAT "COINTEGRATED" MEANS HERE, IN PLAIN LANGUAGE (for readers who are
// not statisticians):
//
// Two stock prices are "cointegrated" if, even though each one individually
// wanders around unpredictably (a "random walk"), some fixed linear
// combination of the two -- e.g. `price_A - beta * price_B` -- does NOT
// wander unpredictably. Instead that combination (called the "spread")
// tends to drift back toward its own average over time. That's the whole
// premise of pairs trading: when the spread strays far from its average,
// bet that it will come back, by buying whichever stock is "cheap" and
// selling whichever is "rich" relative to the pair's usual relationship.
//
// The Engle-Granger test checks this in two steps:
//   Step 1: fit `dependent = alpha + beta * independent` by ordinary least
//           squares (OLS) -- this finds the best-fit linear relationship.
//   Step 2: test whether the leftover residuals from that fit (i.e. the
//           spread) are "stationary" (mean-reverting) using an Augmented
//           Dickey-Fuller (ADF) test. If the residuals ARE stationary, the
//           pair is cointegrated.
//
// IMPORTANT SUBTLETY: which stock you call "dependent" vs "independent" in
// step 1 changes the resulting residuals, and the ADF test's statistical
// POWER (its ability to detect real mean reversion) is not symmetric
// between the two choices in finite samples. This is a well-known
// practical caveat -- see Engle & Granger (1987) "Co-integration and Error
// Correction," Econometrica, and the practical discussion in Ernest P.
// Chan, "Algorithmic Trading: Winning Strategies and Their Rationale"
// (2013), chapter 5. The standard fix, used below, is to run the test in
// BOTH directions and treat the pair as cointegrated if EITHER direction
// passes.
// ----------------------------------------------------------------------
//
// DIRECTION CONSISTENCY WITH THE TRADING CODE (why this file looks the way
// it does): the rest of this codebase -- see src/model/spread.rs
// `hedge_ratio_ols`/`walk_forward_hedge_ratio`, and src/main.rs -- always
// treats the FIRST ticker of a pair (`a`) as the dependent variable, i.e.
// it always fits `price_a = alpha + beta * price_b`. So that the
// significance test reported here is for the *exact same regression* that
// will later generate the tradeable spread (rather than an unrelated
// regression in the opposite direction, which was a real inconsistency in
// an earlier version of this file), the "primary" direction tested below
// also makes `a` the dependent variable. The opposite direction is still
// checked too, purely as the literature-recommended robustness fallback
// described above, but never changes which regression is actually traded.
use crate::model::cointegration::engle_granger;

pub fn scan_cointegration(
    data: &[(String, Vec<f64>, Vec<chrono::NaiveDateTime>)]
) -> Vec<(String, String, f64, bool)> {

    let mut out = Vec::new();

    for i in 0..data.len() {
        for j in i + 1..data.len() {
            let (ref a_name, ref a, _) = data[i];
            let (ref b_name, ref b, _) = data[j];

            let n = a.len().min(b.len());
            let a = &a[..n];
            let b = &b[..n];

            // `engle_granger(x, y)` fits `y = alpha + beta * x` (y is the
            // dependent variable -- see model/cointegration.rs). To match
            // the trading code's convention of "first ticker (a) is always
            // dependent," we therefore call it with the arguments SWAPPED:
            // engle_granger(b, a) fits `a = alpha + beta * b`.
            let primary = engle_granger(b, a);

            // Robustness fallback: also test the reverse direction
            // (`b = alpha + beta * a`). A pair that's cointegrated in
            // reverse but not in the primary/traded direction is still
            // counted as part of the tradeable universe below -- but
            // trading itself (spread, hedge ratio, signals) always uses
            // the primary direction regardless of which one passed, so the
            // reported adf_stat is always the primary direction's.
            let reverse = engle_granger(a, b);

            if let (Ok(primary), Ok(reverse)) = (primary, reverse) {
                let is_cointegrated = primary.is_cointegrated || reverse.is_cointegrated;
                out.push((
                    a_name.clone(),
                    b_name.clone(),
                    primary.adf_stat,
                    is_cointegrated,
                ));
            }
        }
    }

    out
}
