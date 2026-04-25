// src/engine/signals.rs
#[derive(Clone, Copy, Debug)]
pub enum Signal {
    LongSpread,   // long x, short y
    ShortSpread,  // short x, long y
    Flat,
}


pub fn zscore_ratio(price_A: &[f64], price_B: &[f64]) -> Vec<f64> {
    let ratio: Vec<f64> = price_A.iter().zip(price_B).map(|(a, b)| a / b).collect();

    let mean = ratio.iter().sum::<f64>() / ratio.len() as f64;
    let std = (ratio.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / ratio.len() as f64).sqrt();

    ratio.iter().map(|r| (r - mean) / std).collect()
}

pub fn per_stock_signals_from_spread(
    spread_signals: &[Signal],
    price_A: &[f64],
    price_B: &[f64],
) -> Vec<StockSignal> {
    let mut out = Vec::new();

    for i in 1..spread_signals.len() {
        let dA = price_A[i] - price_A[i - 1];
        let dB = price_B[i] - price_B[i - 1];

        match spread_signals[i] {
            Signal::ShortSpread => {
                // Spread too high → A is rich relative to B
                // SELL the one that jumped more
                if dA > dB {
                    out.push(StockSignal::SellA);
                } else {
                    out.push(StockSignal::SellB);
                }
            }
            Signal::LongSpread => {
                // Spread too low → A is cheap relative to B
                // BUY the one that fell more
                if dA < dB {
                    out.push(StockSignal::BuyA);
                } else {
                    out.push(StockSignal::BuyB);
                }
            }
            _ => out.push(StockSignal::Flat),
        }
    }

    // align length
    out.insert(0, StockSignal::Flat);
    out
}


// pub fn per_stock_signals_ratio_threshold(
//     price_A: &[f64],
//     price_B: &[f64],
//     threshold: f64,
// ) -> Vec<StockSignal> {
//     let ratio_z = zscore_ratio(price_A, price_B);
//     let mut out = Vec::new();

//     for i in 1..ratio_z.len() {
//         let z = ratio_z[i];

//         // recent price changes
//         let dA = price_A[i] - price_A[i - 1];
//         let dB = price_B[i] - price_B[i - 1];

//         if z > threshold {
//             // ratio too high → one stock jumped → SELL the one that jumped more
//             if dA > dB {
//                 out.push(StockSignal::SellA);
//             } else {
//                 out.push(StockSignal::SellB);
//             }
//         } else if z < -threshold {
//             // ratio too low → one stock crashed → BUY the one that fell more
//             if dA < dB {
//                 out.push(StockSignal::BuyA);
//             } else {
//                 out.push(StockSignal::BuyB);
//             }
//         } else {
//             out.push(StockSignal::Flat);
//         }
//     }

//     // align length
//     out.insert(0, StockSignal::Flat);
//     out
// }


pub fn generate_signals(z: &[f64], entry: f64, exit: f64) -> Vec<Signal> {
    let mut out = Vec::with_capacity(z.len());
    let mut state = Signal::Flat;

    for &v in z {
        state = match state {
            Signal::Flat => {
                if v >= entry { Signal::ShortSpread }
                else if v <= -entry { Signal::LongSpread }
                else { Signal::Flat }
            }
            Signal::LongSpread => {
                if v >= -exit { Signal::LongSpread } else { Signal::Flat }
            }
            Signal::ShortSpread => {
                if v <= exit { Signal::ShortSpread } else { Signal::Flat }
            }
        };
        out.push(state);
    }
    out
}

pub fn trade_direction(
    a: &str,
    b: &str,
    sig: Signal,
) -> Option<(String, String)> {
    match sig {
        Signal::LongSpread => {
            // Buy A, Sell B
            Some((format!("BUY {}", a), format!("SELL {}", b)))
        }
        Signal::ShortSpread => {
            // Sell A, Buy B
            Some((format!("SELL {}", a), format!("BUY {}", b)))
        }
        Signal::Flat => None,
    }
}

//use crate::engine::signals::Signal;

// pub fn trade_direction(a: &str, b: &str, sig: Signal) -> Option<(String, String)> {
//     match sig {
//         Signal::LongSpread => Some((format!("BUY {}", a), format!("SELL {}", b))),
//         Signal::ShortSpread => Some((format!("SELL {}", a), format!("BUY {}", b))),
//         Signal::Flat => None,
//     }
// }


// pub fn forecast_spread_ar1(spread: &[f64], steps: usize) -> Vec<f64> {
//     let n = spread.len();
//     let window = &spread[n - 100..]; // last 100 points for stability

//     let mut num = 0.0;
//     let mut den = 0.0;
//     for i in 0..window.len() - 1 {
//         num += window[i] * window[i + 1];
//         den += window[i] * window[i];
//     }
//     let phi = num / den;

//     let mut out = Vec::new();
//     let mut last = spread[n - 1];

//     for _ in 0..steps {
//         last = phi * last;
//         out.push(last);
//     }

//     out
// }

// forecast helper function to predict future spread values using an AR(1) model
// pub fn normalize_with_last_window(hist: &[f64], pred: &[f64]) -> Vec<f64> {
//     let w = 20;
//     let slice = &hist[hist.len() - w..];

//     let mean = slice.iter().sum::<f64>() / w as f64;
//     let var = slice.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / w as f64;
//     let sd = var.sqrt();

//     pred.iter().map(|v| (v - mean) / sd).collect()
// }

// ----------------------
// PER-STOCK SIGNAL LOGIC
// ----------------------

#[derive(Debug, Clone, Copy)]
pub enum StockSignal {
    BuyA,
    SellA,
    BuyB,
    SellB,
    Flat,
}

// pub fn per_stock_signals(
//     spread_signals: &[Signal],
//     price_A: &[f64],
//     price_B: &[f64],
//     forecast_A: &[f64],
//     forecast_B: &[f64],
// ) -> Vec<StockSignal> {
//     let mut out = Vec::new();

//     for i in 0..spread_signals.len() {
//         let dev_A = price_A[i] - forecast_A[i];
//         let dev_B = price_B[i] - forecast_B[i];

//         match spread_signals[i] {
//             Signal::LongSpread => {
//                 // Spread too low → BUY cheap, SELL rich
//                 if dev_A < dev_B {
//                     out.push(StockSignal::BuyA);
//                 } else {
//                     out.push(StockSignal::BuyB);
//                 }
//             }
//             Signal::ShortSpread => {
//                 // Spread too high → SELL rich, BUY cheap
//                 if dev_A > dev_B {
//                     out.push(StockSignal::SellA);
//                 } else {
//                     out.push(StockSignal::SellB);
//                 }
//             }
//             _ => out.push(StockSignal::Flat),
//         }
//     }

//     out
// }

// using price ratio over spread deviation for per-stock signals, 
// since ratio is more directly interpretable and less noisy than spread deviations
pub fn per_stock_signals(
    spread_signals: &[Signal],
    price_A: &[f64],
    price_B: &[f64],
) -> Vec<StockSignal> {
    let mut out = Vec::new();

    for i in 0..spread_signals.len() {
        let ratio = price_A[i] / price_B[i];

        match spread_signals[i] {
            Signal::LongSpread => {
                // Spread too low → ratio too low → A is cheap, B is expensive
                out.push(StockSignal::BuyA);
            }
            Signal::ShortSpread => {
                // Spread too high → ratio too high → A is expensive, B is cheap
                out.push(StockSignal::SellA);
            }
            _ => out.push(StockSignal::Flat),
        }
    }

    out
}



