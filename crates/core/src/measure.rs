//! Measurements on logged signals for the scopes: the value at a time (the signal probe and the
//! cursors) and step-response marks (rise, peak, overshoot, settling) drawn on a plot.

use serde::Serialize;

/// Linear interpolation of a logged series at time t (end values outside the run).
pub fn value_at(t: &[f64], y: &[f64], at: f64) -> f64 {
    if t.is_empty() {
        return f64::NAN;
    }
    let i = t.partition_point(|x| *x < at);
    if i == 0 {
        return y[0];
    }
    if i >= t.len() {
        return y[t.len() - 1];
    }
    let (t0, t1) = (t[i - 1], t[i]);
    if t1 <= t0 {
        return y[i];
    }
    y[i - 1] + (at - t0) / (t1 - t0) * (y[i] - y[i - 1])
}

/// Step-response marks of a signal that moves from its first value to its last.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct StepMarks {
    pub y0: f64,
    pub yf: f64,
    /// Times the signal first reaches 10 % and 90 % of the change.
    pub t10: f64,
    pub t90: f64,
    pub rise: f64,
    pub peak: f64,
    pub tpeak: f64,
    /// Percent of the change beyond the final value (0 when there is none).
    pub overshoot: f64,
    /// The time after which the signal stays within 2 % of the change around the final value.
    pub settle: f64,
}

/// The first time after index `from` the signal reaches `level` going in direction `dir`.
fn reach(t: &[f64], y: &[f64], level: f64, dir: f64) -> f64 {
    for i in 1..t.len() {
        let (a, b) = ((y[i - 1] - level) * dir, (y[i] - level) * dir);
        if a < 0.0 && b >= 0.0 {
            return t[i - 1] + (t[i] - t[i - 1]) * (-a) / (b - a);
        }
        if i == 1 && a >= 0.0 {
            return t[0];
        }
    }
    f64::NAN
}

pub fn step_marks(t: &[f64], y: &[f64]) -> Result<StepMarks, String> {
    if t.len() < 3 || t.len() != y.len() {
        return Err("The signal has too few points.".into());
    }
    let (y0, yf) = (y[0], y[y.len() - 1]);
    let dy = yf - y0;
    if !(dy.abs() > 1e-12 * y0.abs().max(yf.abs()).max(1e-300)) || !dy.is_finite() {
        return Err("The signal ends where it starts: it is not a step response.".into());
    }
    let dir = dy.signum();
    let t10 = reach(t, y, y0 + 0.1 * dy, dir);
    let t90 = reach(t, y, y0 + 0.9 * dy, dir);
    let (mut ip, mut best) = (0, f64::NEG_INFINITY);
    for (i, v) in y.iter().enumerate() {
        if (v - y0) * dir > best {
            best = (v - y0) * dir;
            ip = i;
        }
    }
    let overshoot = (((y[ip] - yf) * dir) / dy.abs() * 100.0).max(0.0);
    let band = 0.02 * dy.abs();
    let settle = match (0..y.len()).rev().find(|i| (y[*i] - yf).abs() > band) {
        None => t[0],
        Some(i) if i + 1 < y.len() => {
            // Between the last sample outside the band and the next one: where it enters.
            let (a, b) = ((y[i] - yf).abs() - band, (y[i + 1] - yf).abs() - band);
            t[i] + (t[i + 1] - t[i]) * a / (a - b).max(1e-300)
        }
        Some(i) => t[i],
    };
    Ok(StepMarks { y0, yf, t10, t90, rise: t90 - t10, peak: y[ip], tpeak: t[ip], overshoot, settle })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A second-order step response (ζ = 0.5, ωn = 1) sampled finely: overshoot and peak time
    /// from their closed forms, rise and settling time from the exact response.
    #[test]
    fn second_order_marks() {
        let (z, wn): (f64, f64) = (0.5, 1.0);
        let wd = wn * (1.0 - z * z).sqrt();
        let f = |t: f64| 1.0 - (-z * wn * t).exp() * ((wd * t).cos() + z / (1.0 - z * z).sqrt() * (wd * t).sin());
        let t: Vec<f64> = (0..=40_000).map(|k| k as f64 * 1e-3).collect();
        let y: Vec<f64> = t.iter().map(|t| f(*t)).collect();
        let m = step_marks(&t, &y).unwrap();
        let mp = 100.0 * (-z * std::f64::consts::PI / (1.0 - z * z).sqrt()).exp();
        assert!((m.overshoot - mp).abs() < 1e-3, "overshoot {} vs {mp}", m.overshoot);
        assert!((m.tpeak - std::f64::consts::PI / wd).abs() < 1e-3, "peak time {}", m.tpeak);
        // The exact 10 % and 90 % times by bisection on the closed form.
        let solve = |lv: f64| {
            let (mut a, mut b) = (0.0, 3.0);
            for _ in 0..80 {
                let c = 0.5 * (a + b);
                if f(c) < lv { a = c } else { b = c }
            }
            a
        };
        assert!((m.t10 - solve(0.1)).abs() < 1e-6 && (m.t90 - solve(0.9)).abs() < 1e-6, "t10 {} t90 {}", m.t10, m.t90);
        // Settling: the response is inside 2 % from then on, and was outside just before.
        assert!(t.iter().zip(&y).filter(|(t, _)| **t > m.settle + 1e-6).all(|(_, v)| (v - 1.0).abs() <= 0.02 + 1e-12));
        assert!((f(m.settle - 1e-3) - 1.0).abs() > 0.0195, "settle {}", m.settle);
        assert!((value_at(&t, &y, 1.2345) - f(1.2345)).abs() < 1e-6);
    }

    #[test]
    fn not_a_step() {
        assert!(step_marks(&[0.0, 1.0, 2.0], &[1.0, 2.0, 1.0]).is_err());
        assert!(step_marks(&[0.0, 1.0], &[0.0, 1.0]).is_err());
    }
}
