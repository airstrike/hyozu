//! Parity fixtures. Each expected value is derived in closed form from the
//! normal equations, the same least squares `LINEST`, `LOGEST`, `TREND` and
//! NumPy's `polyfit` solve, so no spreadsheet or network is needed.
use super::*;

const TOLERANCE: f64 = 1e-9;

/// x = 1..5, y = [2, 4, 5, 4, 5]. x̄ = 3, ȳ = 4, Sxx = 10, Sxy = 6.
/// LINEST(y, x) = {0.6, 2.2}. Fitted 2.8, 3.4, 4, 4.6, 5.2 leave residuals
/// -0.8, 0.6, 1, -0.6, -0.2, so SSres = 2.4, SStot = 6 and R² = 0.6.
const Y: [f64; 5] = [2., 4., 5., 4., 5.];

fn close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.);
    assert!(
        (actual - expected).abs() <= TOLERANCE * scale,
        "{actual} differs from {expected}"
    );
}

fn samples(x: impl Fn(f64) -> f64, y: impl Fn(f64) -> f64) -> Vec<[f64; 2]> {
    (1..=5).zip(Y).map(|(i, v)| [x(f64::from(i)), y(v)]).collect()
}

#[test]
fn linear_matches_linest() {
    let fit = linear(&samples(|x| x, |y| y), None).unwrap();
    let Model::Linear { slope, intercept } = *fit.model() else {
        panic!("linear model")
    };
    close(slope, 0.6);
    close(intercept, 2.2);
    close(fit.r_squared(), 0.6);
    // TREND(y, x, 7) = 0.6·7 + 2.2
    close(fit.predict(7.).unwrap(), 6.4);
    assert_eq!(fit.equation(), "y = 0.6x + 2.2");
}

#[test]
fn linear_through_origin_matches_linest_without_constant() {
    // LINEST(y, x, FALSE): m = Σxy / Σx² = 66 / 55 = 1.2. Residuals
    // 0.8, 1.6, 1.4, -0.8, -1 give SSres = 6.8; the uncentered total is
    // Σy² = 86, so R² = 1 - 6.8 / 86 = 198 / 215.
    let fit = linear(&samples(|x| x, |y| y), Some(0.)).unwrap();
    let Model::Linear { slope, intercept } = *fit.model() else {
        panic!("linear model")
    };
    close(slope, 1.2);
    assert_eq!(intercept, 0.);
    close(fit.r_squared(), 198. / 215.);
    assert_eq!(fit.equation(), "y = 1.2x");
}

#[test]
fn linear_through_fixed_intercept_regresses_the_offset_response() {
    // LINEST(y - 1, x, FALSE): Σx(y - 1) = 51, so m = 51 / 55.
    // SSres = Σ(y - 1)² - m·51 = 51 - 2601 / 55 = 204 / 55 and R² = 1 - 4 / 55.
    let fit = linear(&samples(|x| x, |y| y), Some(1.)).unwrap();
    let Model::Linear { slope, intercept } = *fit.model() else {
        panic!("linear model")
    };
    close(slope, 51. / 55.);
    assert_eq!(intercept, 1.);
    close(fit.r_squared(), 51. / 55.);
}

#[test]
fn quadratic_matches_polyfit() {
    // x = -2..2, y = [1, 0, 1, 3, 4]. With symmetric x the normal equations
    // split: c₁ = Σxy / Σx² = 9 / 10, and [5 10; 10 34]·[b; c₂] = [9; 23]
    // gives b = 76 / 70 = 38 / 35, c₂ = 25 / 70 = 5 / 14.
    // SSres = Σy² - β·Xᵀy = 27 - 1826 / 70 = 32 / 35, SStot = 27 - 81 / 5 = 54 / 5,
    // so R² = 1 - 16 / 189 = 173 / 189.
    let p = [[-2., 1.], [-1., 0.], [0., 1.], [1., 3.], [2., 4.]];
    let fit = polynomial(&p, Order::Two, None).unwrap();
    let Model::Polynomial { order, coefficients } = fit.model() else {
        panic!("polynomial model")
    };
    assert_eq!(*order, Order::Two);
    let expected = [5. / 14., 9. / 10., 38. / 35.];
    assert_eq!(coefficients.len(), expected.len());
    for (&a, e) in coefficients.iter().zip(expected) {
        close(a, e);
    }
    close(fit.r_squared(), 173. / 189.);
    assert_eq!(fit.equation(), "y = 0.35714x² + 0.9x + 1.0857");
}

#[test]
fn quadratic_through_origin_matches_linest_without_constant() {
    // x = [1, 2, 3], y = [1, 3, 8], columns x and x². Σx² = 14, Σx³ = 36,
    // Σx⁴ = 98, Σxy = 31, Σx²y = 85; det = 76, so c₁ = -22 / 76 = -11 / 38
    // and c₂ = 74 / 76 = 37 / 38. SSres = 74 - 2804 / 38 = 4 / 19 over the
    // uncentered Σy² = 74 gives R² = 701 / 703.
    let fit = polynomial(&[[1., 1.], [2., 3.], [3., 8.]], Order::Two, Some(0.)).unwrap();
    let Model::Polynomial { coefficients, .. } = fit.model() else {
        panic!("polynomial model")
    };
    close(coefficients[0], 37. / 38.);
    close(coefficients[1], -11. / 38.);
    assert_eq!(coefficients[2], 0.);
    close(fit.r_squared(), 701. / 703.);
    assert_eq!(fit.equation(), "y = 0.97368x² - 0.28947x");
}

#[test]
fn every_order_recovers_an_exact_polynomial_away_from_the_origin() {
    // Dyadic coefficients and integer x make every y exact in binary, so the
    // least-squares solution is the generating polynomial and R² = 1.
    let generator = [1.5, -0.25, 0.125, -0.0625, 0.03125, -0.015625, 0.0078125];
    for order in Order::ALL {
        let n = order.degree();
        let truth: Vec<_> = generator[..=n].to_vec();
        let p: Vec<_> = (0..=n + 4)
            .map(|i| {
                let x = f64::from(i as u32) - 1.;
                [x, truth.iter().rev().fold(0., |acc, c| acc * x + c)]
            })
            .collect();
        let fit = polynomial(&p, order, None).unwrap();
        let Model::Polynomial { coefficients, .. } = fit.model() else {
            panic!("polynomial model")
        };
        for (&a, &e) in coefficients.iter().rev().zip(&truth) {
            close(a, e);
        }
        close(fit.r_squared(), 1.);
        for &[x, y] in &p {
            close(fit.predict(x).unwrap(), y);
        }
    }
}

#[test]
fn exponential_matches_logest() {
    // ln y = [2, 4, 5, 4, 5] is the linear fixture, so ln y = 0.6x + 2.2:
    // LOGEST = {e^0.6, e^2.2} and R² = 0.6 in the regression Excel reports.
    let fit = exponential(&samples(|x| x, f64::exp), None).unwrap();
    let Model::Exponential { scale, rate } = *fit.model() else {
        panic!("exponential model")
    };
    close(rate, 0.6);
    close(scale, 2.2f64.exp());
    close(rate.exp(), 0.6f64.exp());
    close(fit.r_squared(), 0.6);
    // GROWTH(y, x, 6) = e^(2.2 + 0.6·6)
    close(fit.predict(6.).unwrap(), 5.8f64.exp());
    assert_eq!(fit.equation(), "y = 9.025e^(0.6x)");
}

#[test]
fn exponential_with_fixed_scale_matches_logest_without_constant() {
    // Fixing a = e makes ln y - 1 = r·x, the fixed-intercept linear fixture.
    let fit = exponential(&samples(|x| x, f64::exp), Some(1f64.exp())).unwrap();
    let Model::Exponential { scale, rate } = *fit.model() else {
        panic!("exponential model")
    };
    close(scale, 1f64.exp());
    close(rate, 51. / 55.);
    close(fit.r_squared(), 51. / 55.);
}

#[test]
fn logarithmic_regresses_on_log_x() {
    // x = e^1..e^5 turns the linear fixture into y = 0.6·ln x + 2.2.
    let fit = logarithmic(&samples(f64::exp, |y| y)).unwrap();
    let Model::Logarithmic { scale, intercept } = *fit.model() else {
        panic!("logarithmic model")
    };
    close(scale, 0.6);
    close(intercept, 2.2);
    close(fit.r_squared(), 0.6);
    assert_eq!(fit.equation(), "y = 0.6ln(x) + 2.2");
}

#[test]
fn power_regresses_log_y_on_log_x() {
    // Both logs recover the linear fixture: y = e^2.2·x^0.6, R² = 0.6.
    let fit = power(&samples(f64::exp, f64::exp)).unwrap();
    let Model::Power { scale, exponent } = *fit.model() else {
        panic!("power model")
    };
    close(exponent, 0.6);
    close(scale, 2.2f64.exp());
    close(fit.r_squared(), 0.6);
    assert_eq!(fit.equation(), "y = 9.025x^0.6");
}

#[test]
fn moving_average_trails_in_input_order() {
    // Period 3 over [2, 4, 5, 4, 5]: (2 + 4 + 5) / 3, (4 + 5 + 4) / 3, (5 + 4 + 5) / 3.
    let line = moving_average(&samples(|x| x, |y| y), Period::new(3).unwrap()).unwrap();
    let points: Vec<_> = line.points.iter().map(|d| (d.x, d.y)).collect();
    assert_eq!(points.len(), 3);
    for (&(x, y), (ex, ey)) in points.iter().zip([(3., 11. / 3.), (4., 13. / 3.), (5., 14. / 3.)]) {
        assert_eq!(x, ex);
        close(y, ey);
    }
    assert_eq!(
        moving_average(&samples(|x| x, |y| y), Period::new(5).unwrap()).unwrap_err(),
        Error::TooFewPoints { required: 6, found: 5 }
    );
}

#[test]
fn forecast_extends_the_drawn_range() {
    let fit = linear(&samples(|x| x, |y| y), None).unwrap();
    let line = fit.line(Forecast::new(0.5, 2.).unwrap()).unwrap();
    let ends = [line.points[0], line.points[line.points.len() - 1]];
    assert_eq!((ends[0].x, ends[1].x), (0.5, 7.));
    close(ends[0].y, 2.5);
    close(ends[1].y, 6.4);
    let curve = exponential(&samples(|x| x, f64::exp), None)
        .unwrap()
        .line(Forecast::new(0., 1.).unwrap())
        .unwrap();
    assert!(curve.points.iter().all(|d| d.y.is_finite()));
    assert_eq!(curve.points.last().unwrap().x, 6.);
}

#[test]
fn large_offset_x_stays_stable() {
    let b = 1e12;
    let fit = linear(&[[b, 1.], [b + 1., 4.], [b + 2., 7.]], None).unwrap();
    let line = fit.line(Forecast::NONE).unwrap();
    close(line.points[0].y, 1.);
    close(line.points[1].y, 7.);
    close(fit.r_squared(), 1.);
}

#[test]
fn domain_errors_name_the_offending_sample() {
    let p = [[1., 2.], [2., 0.], [3., 5.]];
    assert_eq!(exponential(&p, None).unwrap_err(), Error::NonpositiveY([2., 0.]));
    assert_eq!(power(&p).unwrap_err(), Error::NonpositiveY([2., 0.]));
    let q = [[-1., 2.], [2., 3.], [3., 5.]];
    assert_eq!(logarithmic(&q).unwrap_err(), Error::NonpositiveX([-1., 2.]));
    assert_eq!(power(&q).unwrap_err(), Error::NonpositiveX([-1., 2.]));
    assert_eq!(exponential(&q, Some(0.)).unwrap_err(), Error::Intercept(0.));
    assert_eq!(
        linear(&q, Some(f64::NAN)).unwrap_err().to_string(),
        "invalid fixed intercept NaN"
    );
}

#[test]
fn too_few_positions_for_the_order_is_an_error() {
    let p = [[1., 1.], [2., 4.], [3., 9.], [4., 16.], [4., 17.]];
    assert_eq!(polynomial(&p, Order::Four, None).unwrap_err(), Error::TooFewPoints {
        required: 5,
        found: 4
    });
    assert!(polynomial(&p, Order::Three, None).is_ok());
    // A fixed intercept frees one position, but x = 0 adds nothing.
    let q = [[0., 1.], [1., 2.], [2., 5.]];
    assert!(polynomial(&q, Order::Two, Some(1.)).is_ok());
    assert_eq!(
        polynomial(&q, Order::Three, Some(1.)).unwrap_err(),
        Error::TooFewPoints { required: 3, found: 2 }
    );
    assert_eq!(linear(&[[2., 1.], [2., 3.]], None).unwrap_err(), Error::TooFewPoints {
        required: 2,
        found: 1
    });
}

#[test]
fn options_reject_values_excel_does_not_offer() {
    assert_eq!(Order::try_from(7), Err(Error::Order(7)));
    assert_eq!(Order::try_from(1), Err(Error::Order(1)));
    assert_eq!(Order::try_from(4), Ok(Order::Four));
    assert_eq!(Period::new(1), Err(Error::Period(1)));
    assert!(Forecast::new(-1., 0.).is_err());
    assert!(Forecast::new(0., f64::INFINITY).is_err());
    let fit = logarithmic(&samples(|x| x, |y| y)).unwrap();
    assert_eq!(
        fit.line(Forecast::new(1., 0.).unwrap()).unwrap_err(),
        Error::ForecastDomain(0.)
    );
    assert!(fit.line(Forecast::new(0.5, 0.).unwrap()).is_ok());
    assert_eq!(fit.predict(-1.), None);
}

#[test]
fn nonfinite_samples_are_skipped() {
    let mut p = samples(|x| x, |y| y);
    p.insert(2, [f64::NAN, 3.]);
    p.push([6., f64::INFINITY]);
    let fit = linear(&p, None).unwrap();
    close(fit.r_squared(), 0.6);
    assert_eq!(fit.domain(), [1., 5.]);
}

#[test]
fn equation_numbers_keep_five_significant_digits() {
    assert_eq!(number(1234.567), "1234.6");
    assert_eq!(number(-0.000123456), "-0.00012346");
    assert_eq!(number(0.0000123456), "1.2346E-05");
    assert_eq!(number(12345678901.), "1.2346E+10");
    assert_eq!(number(2.), "2");
}
