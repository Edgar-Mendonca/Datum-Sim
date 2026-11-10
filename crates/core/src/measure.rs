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

/// The spectrum of a signal over whole cycles of its fundamental, and its harmonic distortion.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Spectrum {
    /// The fundamental (Hz) and the window analysed: the last `cycles` periods of the run.
    pub f0: f64,
    pub cycles: usize,
    pub t0: f64,
    pub t1: f64,
    pub dc: f64,
    /// Peak amplitude of each harmonic h = 1 … hmax (index 0 is the fundamental).
    pub harmonics: Vec<f64>,
    /// Total harmonic distortion: √(Σ h≥2 Aₕ²) / A₁, as a fraction.
    pub thd: f64,
    /// The whole spectrum up to hmax: frequencies (Hz) and peak amplitudes.
    pub f: Vec<f64>,
    pub mag: Vec<f64>,
}

/// In-place radix-2 FFT of (re, im); the length must be a power of two.
fn fft(re: &mut [f64], im: &mut [f64]) {
    let n = re.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let ang = -2.0 * std::f64::consts::PI / len as f64;
        for s in (0..n).step_by(len) {
            for k in 0..len / 2 {
                let (wr, wi) = ((ang * k as f64).cos(), (ang * k as f64).sin());
                let (a, b) = (s + k, s + k + len / 2);
                let (xr, xi) = (re[b] * wr - im[b] * wi, re[b] * wi + im[b] * wr);
                re[b] = re[a] - xr;
                im[b] = im[a] - xi;
                re[a] += xr;
                im[a] += xi;
            }
        }
        len <<= 1;
    }
}

/// `n` samples of the signal over [t0, t1), linear between logged points.
fn resample(t: &[f64], y: &[f64], t0: f64, t1: f64, n: usize) -> Vec<f64> {
    (0..n).map(|k| value_at(t, y, t0 + (t1 - t0) * k as f64 / n as f64)).collect()
}

/// Peak amplitudes of the DFT bins 0 … n/2 of samples over a window.
fn amplitudes(s: &[f64]) -> Vec<f64> {
    let n = s.len();
    let (mut re, mut im) = (s.to_vec(), vec![0.0; n]);
    fft(&mut re, &mut im);
    (0..=n / 2).map(|k| re[k].hypot(im[k]) / n as f64 * if k == 0 { 1.0 } else { 2.0 }).collect()
}

/// Spectrum and THD over the last `cycles` whole periods of the fundamental `f0` (Hz; 0 finds it:
/// the strongest component of the run's second half).
pub fn spectrum(t: &[f64], y: &[f64], f0: f64, cycles: usize, hmax: usize) -> Result<Spectrum, String> {
    if t.len() < 8 {
        return Err("The signal has too few points.".into());
    }
    let (ta, tb) = (t[0], *t.last().unwrap());
    let hmax = hmax.clamp(2, 200);
    let f0 = if f0 > 0.0 {
        f0
    } else {
        let w0 = ta + (tb - ta) / 2.0;
        let a = amplitudes(&resample(t, y, w0, tb, 1 << 14));
        let k = (1..a.len()).max_by(|i, j| a[*i].total_cmp(&a[*j])).unwrap_or(1);
        if a[k] <= 1e-12 * a[0].abs().max(1e-300) {
            return Err("The signal does not oscillate: give the fundamental frequency.".into());
        }
        // Refine around the peak bin with the window of whole cycles it implies.
        k as f64 / (tb - w0)
    };
    let period = 1.0 / f0;
    let cycles = cycles.max(1).min(((tb - ta) / period).floor() as usize);
    if cycles == 0 {
        return Err(format!("The run is shorter than one period of {f0} Hz."));
    }
    let t0 = tb - cycles as f64 * period;
    let mut n = 1024;
    while n < 16 * hmax * cycles && n < 1 << 18 {
        n <<= 1;
    }
    let a = amplitudes(&resample(t, y, t0, tb, n));
    let harmonics: Vec<f64> = (1..=hmax).map(|h| a.get(h * cycles).cloned().unwrap_or(0.0)).collect();
    if harmonics[0] <= 0.0 {
        return Err("No fundamental at that frequency.".into());
    }
    let thd = harmonics[1..].iter().map(|v| v * v).sum::<f64>().sqrt() / harmonics[0];
    let top = ((hmax + 1) * cycles).min(a.len() - 1);
    Ok(Spectrum { f0, cycles, t0, t1: tb, dc: a[0], harmonics, thd, f: (0..=top).map(|k| k as f64 / (tb - t0)).collect(), mag: a[..=top].to_vec() })
}

#[cfg(test)]
mod tests {
    /// A 50 Hz sine (2) with 10 % fifth and 5 % seventh harmonics and an offset: THD √(0.1² + 0.05²),
    /// each harmonic's amplitude, the DC, and the fundamental found without being told.
    #[test]
    fn spectrum_and_thd() {
        use std::f64::consts::PI;
        let t: Vec<f64> = (0..=200_000).map(|i| i as f64 * 1e-6).collect();
        let y: Vec<f64> = t.iter().map(|t| 0.3 + 2.0 * (2.0 * PI * 50.0 * t).sin() + 0.2 * (2.0 * PI * 250.0 * t).sin() + 0.1 * (2.0 * PI * 350.0 * t + 1.0).sin()).collect();
        for f0 in [50.0, 0.0] {
            let s = super::spectrum(&t, &y, f0, 4, 40).unwrap();
            assert!((s.f0 - 50.0).abs() < 1e-9, "{}", s.f0);
            assert!((s.harmonics[0] - 2.0).abs() < 1e-4 && (s.harmonics[4] - 0.2).abs() < 1e-4 && (s.harmonics[6] - 0.1).abs() < 1e-4, "{:?}", &s.harmonics[..8]);
            assert!((s.thd - 0.1118034).abs() < 1e-4 && (s.dc - 0.3).abs() < 1e-4, "{} {}", s.thd, s.dc);
        }
    }

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
