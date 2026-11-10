//! Machine parameters from the nameplate (0.3.5): the induction motor's equivalent circuit from
//! rated power, voltage, frequency, speed, efficiency and power factor.
//!
//! With the stator star-connected and iron and friction losses neglected:
//! - slip s from the rated and synchronous speeds; input power P/η; current P/(η·√3·V·pf);
//! - air-gap power P/(1 − s), so the stator copper loss gives Rs = (P/η − P/(1 − s))/(3 I²);
//! - Rr, the leakage X (stator and rotor equal) and Xm from three equations: the input impedance
//!   at the rated slip (real and imaginary parts) and its size at standstill, from the locked-rotor
//!   current ratio I_A/I_N of the data sheet (7 when not given). Newton's method solves them.
//!   (The air-gap power adds nothing: it follows from the real part and Rs.)

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Plate {
    /// Rated output (W), line voltage (V rms), frequency (Hz), speed (rpm), efficiency, power factor.
    pub p: f64,
    pub v: f64,
    pub f: f64,
    pub n: f64,
    pub eff: f64,
    pub pf: f64,
    /// Locked-rotor (starting) current over rated current, I_A/I_N; 0 = 7.
    #[serde(default)]
    pub ist: f64,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Circuit {
    pub rs: f64,
    pub rr: f64,
    pub lls: f64,
    pub llr: f64,
    pub lm: f64,
    pub pp: f64,
    /// Rated current (A) and slip, for the record.
    pub i: f64,
    pub slip: f64,
}

#[derive(Clone, Copy)]
struct C(f64, f64);
impl C {
    fn add(self, o: C) -> C {
        C(self.0 + o.0, self.1 + o.1)
    }
    fn mul(self, o: C) -> C {
        C(self.0 * o.0 - self.1 * o.1, self.0 * o.1 + self.1 * o.0)
    }
    fn div(self, o: C) -> C {
        let d = o.0 * o.0 + o.1 * o.1;
        C((self.0 * o.0 + self.1 * o.1) / d, (self.1 * o.0 - self.0 * o.1) / d)
    }
    fn abs2(self) -> f64 {
        self.0 * self.0 + self.1 * self.1
    }
}

/// Input impedance and rotor current per ampere of stator current.
fn circuit(rs: f64, rr: f64, xl: f64, xm: f64, s: f64) -> (C, C) {
    let zr = C(rr / s, xl);
    let zm = C(0.0, xm);
    let par = zm.mul(zr).div(zm.add(zr));
    (C(rs, xl).add(par), zm.div(zm.add(zr)))
}

pub fn induction(pl: &Plate) -> Result<Circuit, String> {
    let Plate { p, v, f, n, eff, pf, ist } = *pl;
    let ist = if ist > 0.0 { ist } else { 7.0 };
    if !(ist > 1.5) {
        return Err("The starting current ratio I_A/I_N must be above 1.5.".into());
    }
    if !(p > 0.0 && v > 0.0 && f > 0.0 && n > 0.0) {
        return Err("Power, voltage, frequency and speed must be positive.".into());
    }
    if !(eff > 0.0 && eff < 1.0 && pf > 0.0 && pf < 1.0) {
        return Err("Efficiency and power factor must be between 0 and 1.".into());
    }
    let pp = (60.0 * f / n).floor().max(1.0);
    let ns = 60.0 * f / pp;
    let s = (ns - n) / ns;
    if !(s > 0.0 && s < 0.3) {
        return Err(format!("The rated speed {n} rpm gives a slip of {:.1} % at {pp} pole pairs: check the speed and frequency.", s * 100.0));
    }
    let pin = p / eff;
    let i = pin / (3f64.sqrt() * v * pf);
    let vph = v / 3f64.sqrt();
    let pag = p / (1.0 - s);
    let rs = (pin - pag) / (3.0 * i * i);
    if rs <= 0.0 {
        return Err("The efficiency is too high for this slip: the stator would have no copper loss. Check the efficiency and the speed.".into());
    }
    let z = vph / i;
    let (zr, zx) = (z * pf, z * (1.0 - pf * pf).sqrt());
    // Unknowns: ln Rr, ln Xl, ln Xm (kept positive). Residuals scaled to be of order 1.
    let res = |u: &[f64; 3]| -> [f64; 3] {
        let (rr, xl, xm) = (u[0].exp(), u[1].exp(), u[2].exp());
        let (zin, _) = circuit(rs, rr, xl, xm, s);
        let (z1, _) = circuit(rs, rr, xl, xm, 1.0);
        [(zin.0 - zr) / z, (zin.1 - zx) / z, z1.abs2().sqrt() * ist / z - 1.0]
    };
    let mut u = [(s * (zr - rs).max(0.1 * zr)).ln(), (0.08 * z).ln(), (zx * 1.2).max(0.5 * z).ln()];
    for _ in 0..200 {
        let r = res(&u);
        if r.iter().all(|v| v.abs() < 1e-13) {
            break;
        }
        let mut j = [[0.0; 3]; 3];
        for c in 0..3 {
            let mut up = u;
            up[c] += 1e-7;
            let rp = res(&up);
            for k in 0..3 {
                j[k][c] = (rp[k] - r[k]) / 1e-7;
            }
        }
        let flat: Vec<f64> = j.iter().flatten().cloned().collect();
        let lu = crate::linalg::Lu::new(flat, 3).ok_or("The nameplate data has no matching equivalent circuit.")?;
        let mut d = vec![-r[0], -r[1], -r[2]];
        lu.solve(&mut d);
        let step = d.iter().fold(0.0f64, |a, v| a.max(v.abs()));
        let k = if step > 0.5 { 0.5 / step } else { 1.0 };
        for c in 0..3 {
            u[c] += k * d[c];
        }
    }
    let r = res(&u);
    if r.iter().any(|v| v.abs() > 1e-8 || !v.is_finite()) {
        return Err("No equivalent circuit matches this nameplate (the power factor and efficiency may not fit the slip).".into());
    }
    let we = 2.0 * std::f64::consts::PI * f;
    let (rr, xl, xm) = (u[0].exp(), u[1].exp(), u[2].exp());
    Ok(Circuit { rs, rr, lls: xl / we, llr: xl / we, lm: xm / we, pp, i, slip: s })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A machine's own rated point back to its parameters: build the nameplate of a known
    /// equivalent circuit at 2 % slip, then recover Rs, Rr, Lls = Llr and Lm.
    #[test]
    fn round_trip() {
        let (rs, rr, ll, lm, f, v, pp) = (0.37, 0.31, 0.0039, 0.117, 50.0, 400.0, 2.0);
        let we = 2.0 * std::f64::consts::PI * f;
        let s = 0.02;
        let vph = v / 3f64.sqrt();
        let (zin, ir) = circuit(rs, rr, we * ll, we * lm, s);
        let i = vph / zin.abs2().sqrt();
        let pf = zin.0 / zin.abs2().sqrt();
        let pag = 3.0 * i * i * ir.abs2() * rr / s;
        let pmech = pag * (1.0 - s);
        let pin = 3.0 * vph * i * pf;
        let n = 60.0 * f / pp * (1.0 - s);
        let (z1, _) = circuit(rs, rr, we * ll, we * lm, 1.0);
        let ist = (vph / z1.abs2().sqrt()) / i;
        let c = induction(&Plate { p: pmech, v, f, n, eff: pmech / pin, pf, ist }).unwrap();
        for (got, want) in [(c.rs, rs), (c.rr, rr), (c.lls, ll), (c.llr, ll), (c.lm, lm), (c.pp, pp)] {
            assert!((got / want - 1.0).abs() < 1e-6, "{c:?}");
        }
        // A typical 11 kW plate gives a sensible circuit.
        let c = induction(&Plate { p: 11e3, v: 400.0, f: 50.0, n: 1460.0, eff: 0.9, pf: 0.85, ist: 0.0 }).unwrap();
        assert!(c.pp == 2.0 && c.rs > 0.05 && c.rs < 2.0 && c.lm > 0.02 && c.lm < 1.0 && c.i > 15.0 && c.i < 30.0, "{c:?}");
        assert!(induction(&Plate { p: 11e3, v: 400.0, f: 50.0, n: 1460.0, eff: 0.995, pf: 0.85, ist: 7.0 }).is_err());
    }
}
