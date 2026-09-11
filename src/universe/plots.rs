use anyhow::Result;
use plotters::prelude::*;
use chrono::NaiveDate;
use plotters::style::RGBColor;
use crate::engine::signals::StockSignal;

const ORANGE: RGBColor = RGBColor(255, 165, 0);


pub fn plot_series_svg(path: &str, title: &str, data: &[f64]) -> Result<()> {
    let root = SVGBackend::new(path, (1024, 512)).into_drawing_area();
    root.fill(&WHITE)?;

    let (min, max) = data.iter().fold((f64::MAX, f64::MIN), |(mn, mx), v| {
        (mn.min(*v), mx.max(*v))
    });

    let mut chart = ChartBuilder::on(&root)
        .caption(title, ("sans-serif", 24))
        .margin(20)
        .set_all_label_area_size(40)
        .build_cartesian_2d(0..data.len(), min..max)?;

    chart.configure_mesh().draw()?;

    chart.draw_series(LineSeries::new(
        (0..data.len()).map(|i| (i, data[i])),
        &BLUE,
    ))?;

    root.present()?;   // flush to disk
    //drop(root);        // release file handle

    Ok(())
}


use crate::engine::signals::Signal;

pub fn plot_zscore_with_signals_svg(
    path: &str,
    title: &str,
    z: &[f64],
    sigs: &[Signal],
) -> anyhow::Result<()> {
    let root = SVGBackend::new(path, (800, 400)).into_drawing_area();
    root.fill(&WHITE)?;

    let n = z.len();
    let min_y = z.iter().cloned().fold(f64::INFINITY, f64::min).min(-3.0);
    let max_y = z.iter().cloned().fold(f64::NEG_INFINITY, f64::max).max(3.0);

    let mut chart = ChartBuilder::on(&root)
        .caption(title, ("sans-serif", 20))
        .margin(10)
        .x_label_area_size(30)
        .y_label_area_size(40)
        .build_cartesian_2d(0..n, min_y..max_y)?;

    chart.configure_mesh().draw()?;

    // z-score line
    chart.draw_series(LineSeries::new(
        (0..n).map(|i| (i, z[i])),
        &BLUE,
    ))?;

    // signals as markers
    chart.draw_series(
        (0..n).filter_map(|i| match sigs[i] {
            Signal::LongSpread => Some(Circle::new((i, z[i]), 3, GREEN.filled())),
            Signal::ShortSpread => Some(Circle::new((i, z[i]), 3, RED.filled())),
            Signal::Flat => None,
        }),
    )?;

    root.present()?;   // flush
    //drop(root);        // release file handle

    Ok(())
}


// pub fn plot_signals_with_forecast_svg(
//     path: &str,
//     title: &str,
//     hist_z: &[f64],
//     hist_signals: &[Signal],
//     forecast_z: &[f64],
//     forecast_signals: &[Signal],
// ) -> anyhow::Result<()> {

//     let root = SVGBackend::new(path, (1200, 600)).into_drawing_area();
//     root.fill(&WHITE)?;

//     let total_len = hist_z.len() + forecast_z.len();

//     let mut chart = ChartBuilder::on(&root)
//         .caption(title, ("sans-serif", 28))
//         .margin(20)
//         .set_all_label_area_size(40)
//         .build_cartesian_2d(0..total_len, -4.0..4.0)?;

//     chart.configure_mesh().draw()?;

//     // --- Historical z-score (blue) ---
//     chart.draw_series(LineSeries::new(
//         (0..hist_z.len()).map(|i| (i, hist_z[i])),
//         &BLUE,
//     ))?;

//     // --- Forecast z-score (red dashed) ---
//     chart.draw_series(LineSeries::new(
//         (hist_z.len()..total_len).map(|i| {
//             let idx = i - hist_z.len();
//             (i, forecast_z[idx])
//         }),
//         &RED.mix(0.7),
//     ))?;

//     // --- Historical signals ---
//     for i in 0..hist_signals.len() {
//         match hist_signals[i] {
//             Signal::LongSpread => {
//                 chart.draw_series(std::iter::once(Circle::new((i, hist_z[i]), 4, GREEN.filled())))?;
//             }
//             Signal::ShortSpread => {
//                 chart.draw_series(std::iter::once(Circle::new((i, hist_z[i]), 4, RED.filled())))?;
//             }
//             _ => {}
//         }
//     }

//     // --- Forecast signals ---
//     for i in 0..forecast_signals.len() {
//         let x = hist_z.len() + i;
//         match forecast_signals[i] {
//             Signal::LongSpread => {
//                 chart.draw_series(std::iter::once(Circle::new((x, forecast_z[i]), 5, GREEN.filled())))?;
//             }
//             Signal::ShortSpread => {
//                 chart.draw_series(std::iter::once(Circle::new((x, forecast_z[i]), 5, RED.filled())))?;
//             }
//             _ => {}
//         }
//     }

//     root.present()?;
//     Ok(())
// }

pub fn plot_signals_with_forecast_svg(
    path: &str,
    title: &str,
    hist_dates: &[NaiveDate],
    hist_z: &[f64],
    hist_signals: &[Signal],
    forecast_dates: &[NaiveDate],
    forecast_z: &[f64],
    forecast_signals: &[Signal],
) -> anyhow::Result<()> {

    let root = BitMapBackend::new(path, (900, 700)).into_drawing_area();
    root.fill(&WHITE)?;

    let min_y = -4.0;
    let max_y = 4.0;

    let mut chart = ChartBuilder::on(&root)
        .caption(title, ("sans-serif", 28))
        .margin(20)
        .set_all_label_area_size(50)
        .build_cartesian_2d(
            hist_dates[0]..forecast_dates[forecast_dates.len() - 1],
            min_y..max_y,
        )?;

    chart.configure_mesh()
        .x_labels(15)
        .x_label_formatter(&|d| d.format("%Y-%m-%d").to_string())
        .draw()?;

    // --- Entry threshold reference bands: 1.5 SD (dotted) and 2.0 SD (dashed) ---
    let x0 = hist_dates[0];
    let x1 = forecast_dates[forecast_dates.len() - 1];
    for &level in &[1.5, -1.5, 2.0, -2.0] {
        chart.draw_series(std::iter::once(PathElement::new(
            vec![(x0, level), (x1, level)],
            ShapeStyle {
                color: RGBColor(150, 150, 150).mix(0.6),
                filled: false,
                stroke_width: 1,
            },
        )))?;
    }

    // --- Historical z-score (blue) ---
    chart.draw_series(LineSeries::new(
        hist_dates.iter().zip(hist_z.iter()).map(|(d, z)| (*d, *z)),
        &BLUE,
    ))?;

    // --- Forecast z-score (red dashed) ---
    chart.draw_series(LineSeries::new(
        forecast_dates.iter().zip(forecast_z.iter()).map(|(d, z)| (*d, *z)),
        &RED.mix(0.7),
    ))?;

    // --- Historical signals ---
    for i in 0..hist_signals.len() {
        match hist_signals[i] {
            Signal::LongSpread => {
                chart.draw_series(std::iter::once(TriangleMarker::new((hist_dates[i], hist_z[i]), 4, GREEN.filled())))?;
            }
            Signal::ShortSpread => {
                chart.draw_series(std::iter::once(TriangleMarker::new((hist_dates[i], hist_z[i]), 4, RED.filled())))?;
            }
            _ => {}
        }
    }

    // --- Forecast signals ---
    for i in 0..forecast_signals.len() {
        match forecast_signals[i] {
            Signal::LongSpread => {
                chart.draw_series(std::iter::once(Circle::new((forecast_dates[i], forecast_z[i]), 5, GREEN.filled())))?;
            }
            Signal::ShortSpread => {
                chart.draw_series(std::iter::once(Circle::new((forecast_dates[i], forecast_z[i]), 5, RED.filled())))?;
            }
            _ => {}
        }
    }

    root.present()?;
    Ok(())
}



// pub fn plot_prices_with_signals_svg(
//     path: &str,
//     title: &str,
//     dates_hist: &[NaiveDate],
//     price_A: &[f64],
//     price_B: &[f64],
//     //signals_hist: &[Signal],
//     signals_hist: &[StockSignal],
//     dates_fore: &[NaiveDate],
//     forecast_A: &[f64],
//     forecast_B: &[f64],
//     //signals_fore: &[Signal],
//     signals_fore: &[StockSignal],
// ) -> anyhow::Result<()> {

//     let root = SVGBackend::new(path, (1400, 700)).into_drawing_area();
//     root.fill(&WHITE)?;

//     let min_price = price_A.iter().chain(price_B).cloned().fold(f64::INFINITY, f64::min);
//     let max_price = price_A.iter().chain(price_B).cloned().fold(f64::NEG_INFINITY, f64::max);

//     let mut chart = ChartBuilder::on(&root)
//         .caption(title, ("sans-serif", 28))
//         .margin(20)
//         .set_all_label_area_size(50)
//         .build_cartesian_2d(
//             dates_hist[0]..dates_fore[dates_fore.len() - 1],
//             min_price..max_price,
//         )?;

//     chart.configure_mesh()
//         .x_labels(15)
//         .x_label_formatter(&|d| d.format("%Y-%m-%d").to_string())
//         .draw()?;

//     // Historical A
//     chart.draw_series(LineSeries::new(
//         dates_hist.iter().zip(price_A).map(|(d, p)| (*d, *p)),
//         &BLUE,
//     ))?;

//     // Historical B
//     chart.draw_series(LineSeries::new(
//         dates_hist.iter().zip(price_B).map(|(d, p)| (*d, *p)),
//         &ORANGE,
//     ))?;

//     // Forecast A
//     chart.draw_series(LineSeries::new(
//         dates_fore.iter().zip(forecast_A).map(|(d, p)| (*d, *p)),
//         &BLUE.mix(0.5),
//     ))?;

//     // Forecast B
//     chart.draw_series(LineSeries::new(
//         dates_fore.iter().zip(forecast_B).map(|(d, p)| (*d, *p)),
//         &ORANGE.mix(0.5),
//     ))?;

//     // Historical signals
//     // for i in 0..signals_hist.len() {
//     //     let d = dates_hist[i];
//     //     match signals_hist[i] {
//     //         Signal::LongSpread => {
//     //             chart.draw_series(std::iter::once(Circle::new((d, price_A[i]), 4, GREEN.filled())))?;
//     //             chart.draw_series(std::iter::once(Circle::new((d, price_B[i]), 4, GREEN.filled())))?;
//     //         }
//     //         Signal::ShortSpread => {
//     //             chart.draw_series(std::iter::once(Circle::new((d, price_A[i]), 4, RED.filled())))?;
//     //             chart.draw_series(std::iter::once(Circle::new((d, price_B[i]), 4, RED.filled())))?;
//     //         }
//     //         _ => {}
//     //     }
//     // }

//     // Historical per-stock signals
//     // for i in 0..signals_hist.len() {
//     //     let d = dates_hist[i];
//     //     match signals_hist[i] {
//     //         StockSignal::BuyA => {
//     //             chart.draw_series(std::iter::once(TriangleMarker::new((d, price_A[i]), 4, GREEN.filled())))?;
//     //         }
//     //         StockSignal::SellA => {
//     //             chart.draw_series(std::iter::once(TriangleMarker::new((d, price_A[i]), 4, RED.filled())))?;
//     //         }
//     //         StockSignal::BuyB => {
//     //             chart.draw_series(std::iter::once(TriangleMarker::new((d, price_B[i]), 4, GREEN.filled())))?;
//     //         }
//     //         StockSignal::SellB => {
//     //             chart.draw_series(std::iter::once(TriangleMarker::new((d, price_B[i]), 4, RED.filled())))?;
//     //         }
//     //         StockSignal::Flat => {}
//     //     }
//     // }

//         // --- HISTORICAL SIGNAL MARKERS (TRIANGLES) ---
//     for i in 0..signals_hist.len() {
//         let d = dates_hist[i];
//         match signals_hist[i] {
//             StockSignal::BuyA => {
//                 chart.draw_series(std::iter::once(
//                     TriangleMarker::new((d, price_A[i]), 6, GREEN.filled())
//                 ))?;
//             }
//             StockSignal::SellA => {
//                 chart.draw_series(std::iter::once(
//                     TriangleMarker::new((d, price_A[i]), 6, RED.filled())
//                 ))?;
//             }
//             StockSignal::BuyB => {
//                 chart.draw_series(std::iter::once(
//                     TriangleMarker::new((d, price_B[i]), 6, GREEN.filled())
//                 ))?;
//             }
//             StockSignal::SellB => {
//                 chart.draw_series(std::iter::once(
//                     TriangleMarker::new((d, price_B[i]), 6, RED.filled())
//                 ))?;
//             }
//             _ => {}
//         }
//     }



//     // --- FORECAST SIGNAL MARKERS (CIRCLES ONLY FOR 5 DAYS) ---
//     for i in 0..signals_fore.len() {
//         let d = dates_fore[i];

//         match signals_fore[i] {
//             StockSignal::BuyA => {
//                 chart.draw_series(std::iter::once(
//                     Circle::new((d, forecast_A[i]), 6, GREEN.filled())
//                 ))?;
//             }
//             StockSignal::SellA => {
//                 chart.draw_series(std::iter::once(
//                     Circle::new((d, forecast_A[i]), 6, RED.filled())
//                 ))?;
//             }
//             StockSignal::BuyB => {
//                 chart.draw_series(std::iter::once(
//                     Circle::new((d, forecast_B[i]), 6, GREEN.filled())
//                 ))?;
//             }
//             StockSignal::SellB => {
//                 chart.draw_series(std::iter::once(
//                     Circle::new((d, forecast_B[i]), 6, RED.filled())
//                 ))?;
//             }
//             _ => {}
//         }
//     }


//         // --- FORECAST PRICE LINES (DASHED) ---
//     use plotters::style::ShapeStyle;

//     // helper: draw dashed line between points
//     fn draw_dashed_line<'a, DB: DrawingBackend>(
//         chart: &mut ChartContext<DB, Cartesian2d<NaiveDate, f64>>,
//         dates: &[NaiveDate],
//         values: &[f64],
//         color: RGBColor,
//     ) -> Result<(), DrawingErrorKind<DB::ErrorType>> {
//         let dash = 6;
//         let gap = 6;

//         for i in 1..dates.len() {
//             let x0 = dates[i - 1];
//             let y0 = values[i - 1];
//             let x1 = dates[i];
//             let y1 = values[i];

//             // draw small segments to simulate dashes
//             chart.draw_series(std::iter::once(PathElement::new(
//                 vec![(x0, y0), (x1, y1)],
//                 ShapeStyle {
//                     color: color.to_rgba(),
//                     filled: false,
//                     stroke_width: 2,
//                 },
//             )))?;
//         }

//         Ok(())
//     }

//     // call dashed line drawer
//     draw_dashed_line(&mut chart, dates_fore, forecast_A, BLUE.mix(0.6))?;
//     draw_dashed_line(&mut chart, dates_fore, forecast_B, ORANGE.mix(0.6))?;
//

// U P D A T E D P R I C E  LINES  T O  SOLID  FOR  BETTER VISIBILITY WITH SIGNALS
pub fn plot_prices_with_signals_svg(
    path: &str,
    title: &str,
    dates_hist: &[NaiveDate],
    price_A: &[f64],
    price_B: &[f64],
    signals_hist: &[StockSignal],
    dates_fore: &[NaiveDate],
    forecast_A: &[f64],
    forecast_B: &[f64],
    signals_fore: &[StockSignal],
) -> anyhow::Result<()> {
    let root = BitMapBackend::new(path, (900, 700)).into_drawing_area();
    root.fill(&WHITE)?;

    // include forecast in min/max so scale fits everything
    let min_price = price_A
        .iter()
        .chain(price_B)
        .chain(forecast_A)
        .chain(forecast_B)
        .cloned()
        .fold(f64::INFINITY, f64::min);

    let max_price = price_A
        .iter()
        .chain(price_B)
        .chain(forecast_A)
        .chain(forecast_B)
        .cloned()
        .fold(f64::NEG_INFINITY, f64::max);

    let mut chart = ChartBuilder::on(&root)
        .caption(title, ("sans-serif", 28))
        .margin(20)
        .set_all_label_area_size(50)
        .build_cartesian_2d(
            dates_hist[0]..dates_fore[dates_fore.len() - 1],
            min_price..max_price,
        )?;

    chart
        .configure_mesh()
        .x_labels(15)
        .x_label_formatter(&|d| d.format("%Y-%m-%d").to_string())
        .draw()?;

    // --- HISTORICAL PRICES ---
    chart.draw_series(LineSeries::new(
        dates_hist.iter().zip(price_A.iter()).map(|(d, p)| (*d, *p)),
        &BLUE,
    ))?;

    chart.draw_series(LineSeries::new(
        dates_hist.iter().zip(price_B.iter()).map(|(d, p)| (*d, *p)),
        &ORANGE,
    ))?;

    // --- FORECAST PRICES (VISUALLY SEPARATE: LIGHTER COLORS) ---
    chart.draw_series(LineSeries::new(
        dates_fore.iter().zip(forecast_A.iter()).map(|(d, p)| (*d, *p)),
        &BLUE.mix(0.6),
    ))?;

    chart.draw_series(LineSeries::new(
        dates_fore.iter().zip(forecast_B.iter()).map(|(d, p)| (*d, *p)),
        &ORANGE.mix(0.6),
    ))?;

    // --- HISTORICAL SIGNALS: TRIANGLES ---
    for i in 0..signals_hist.len() {
        let d = dates_hist[i];
        match signals_hist[i] {
            StockSignal::BuyA => {
                chart.draw_series(std::iter::once(
                    TriangleMarker::new((d, price_A[i]), 4, GREEN.filled()),
                ))?;
            }
            StockSignal::SellA => {
                chart.draw_series(std::iter::once(
                    TriangleMarker::new((d, price_A[i]), 4, RED.filled()),
                ))?;
            }
            StockSignal::BuyB => {
                chart.draw_series(std::iter::once(
                    TriangleMarker::new((d, price_B[i]), 4, GREEN.filled()),
                ))?;
            }
            StockSignal::SellB => {
                chart.draw_series(std::iter::once(
                    TriangleMarker::new((d, price_B[i]), 4, RED.filled()),
                ))?;
            }
            StockSignal::Flat => {}
        }
    }

    // --- FORECAST SIGNALS: CIRCLES (ONLY 5 DAYS) ---
    for i in 0..signals_fore.len() {
        let d = dates_fore[i];
        match signals_fore[i] {
            StockSignal::BuyA => {
                chart.draw_series(std::iter::once(
                    Circle::new((d, forecast_A[i]), 5, GREEN.filled()),
                ))?;
            }
            StockSignal::SellA => {
                chart.draw_series(std::iter::once(
                    Circle::new((d, forecast_A[i]), 5, RED.filled()),
                ))?;
            }
            StockSignal::BuyB => {
                chart.draw_series(std::iter::once(
                    Circle::new((d, forecast_B[i]), 5, GREEN.filled()),
                ))?;
            }
            StockSignal::SellB => {
                chart.draw_series(std::iter::once(
                    Circle::new((d, forecast_B[i]), 5, RED.filled()),
                ))?;
            }
            StockSignal::Flat => {}
        }
    }

    root.present()?;
    Ok(())
}


// Equity curve comparison for the two entry thresholds (1.5 SD vs 2.0 SD)
// backtested on the same pair, so the two strategies can be compared at a
// glance alongside their Sharpe ratio / max drawdown.
pub fn plot_equity_curves_svg(
    path: &str,
    title: &str,
    dates: &[NaiveDate],
    equity_a: &[f64],
    label_a: &str,
    equity_b: &[f64],
    label_b: &str,
) -> anyhow::Result<()> {
    let root = BitMapBackend::new(path, (900, 500)).into_drawing_area();
    root.fill(&WHITE)?;

    let n = dates.len().min(equity_a.len()).min(equity_b.len());

    let min_y = equity_a[..n]
        .iter()
        .chain(&equity_b[..n])
        .cloned()
        .fold(0.0_f64, f64::min);
    let max_y = equity_a[..n]
        .iter()
        .chain(&equity_b[..n])
        .cloned()
        .fold(0.0_f64, f64::max);
    // Pad the range slightly so a flat line isn't drawn on the axis edge.
    let pad = ((max_y - min_y).abs() * 0.1).max(1e-6);

    let mut chart = ChartBuilder::on(&root)
        .caption(title, ("sans-serif", 24))
        .margin(20)
        .set_all_label_area_size(50)
        .build_cartesian_2d(dates[0]..dates[n - 1], (min_y - pad)..(max_y + pad))?;

    chart
        .configure_mesh()
        .x_labels(15)
        .x_label_formatter(&|d| d.format("%Y-%m-%d").to_string())
        .y_desc("Cumulative net PnL")
        .draw()?;

    chart
        .draw_series(LineSeries::new(
            dates[..n].iter().zip(equity_a[..n].iter()).map(|(d, e)| (*d, *e)),
            &BLUE,
        ))?
        .label(label_a)
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &BLUE));

    chart
        .draw_series(LineSeries::new(
            dates[..n].iter().zip(equity_b[..n].iter()).map(|(d, e)| (*d, *e)),
            &ORANGE,
        ))?
        .label(label_b)
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &ORANGE));

    chart
        .configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .draw()?;

    root.present()?;
    Ok(())
}

// Plot side by side with signals, so we can visually verify that signals align with price movements as expected.
// use plotters::prelude::*;
// pub fn plot_two_charts_side_by_side<F1, F2>(
//     path: &str,
//     left_plot: F1,
//     right_plot: F2,
// ) -> anyhow::Result<()>
// where
//     F1: Fn(&DrawingArea<SVGBackend, Shift>),
//     F2: Fn(&DrawingArea<SVGBackend, Shift>),
// {
//     // Create a wide SVG canvas
//     let root = SVGBackend::new(path, (1800, 700)).into_drawing_area();
//     root.fill(&WHITE)?;

//     // Split horizontally into two equal halves
//     let (left_area, right_area) = root.split_horizontally(900);

//     // Draw left chart
//     left_plot(&left_area);

//     // Draw right chart
//     right_plot(&right_area);

//     root.present()?;
//     Ok(())
// }

