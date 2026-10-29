//! The motor library: DC machines (separately excited, shunt, series, compound), induction machines
//! (squirrel-cage, double-cage, wound-rotor), synchronous machines (PMSM, reluctance, wound-field
//! with dampers), BLDC and stepper motors, and the drive pieces (Clarke/Park transforms, three-phase
//! source, inverter, PWM, encoder). SI units; all machines use the motor convention, a mechanical
//! speed ω (rad/s), angle θ (rad) and a load-torque input T_L. Three-phase signals are vectors of
//! width 3; the αβ and dq transforms are amplitude-invariant.

use crate::rt::{Ctx, Pv, Rate, Rt};
use std::f64::consts::PI;

const SQ3: f64 = 1.732_050_807_568_877_2;
const TAU3: f64 = 2.0 * PI / 3.0;

pub fn clarke(a: f64, b: f64, c: f64) -> (f64, f64) {
    ((2.0 / 3.0) * (a - 0.5 * (b + c)), (b - c) / SQ3)
}
pub fn inv_clarke(al: f64, be: f64) -> (f64, f64, f64) {
    (al, -0.5 * al + 0.5 * SQ3 * be, -0.5 * al - 0.5 * SQ3 * be)
}
pub fn park(al: f64, be: f64, th: f64) -> (f64, f64) {
    let (s, c) = th.sin_cos();
    (al * c + be * s, -al * s + be * c)
}
pub fn inv_park(d: f64, q: f64, th: f64) -> (f64, f64) {
    let (s, c) = th.sin_cos();
    (d * c - q * s, d * s + q * c)
}

macro_rules! cb {
    () => {
        fn clone_box(&self) -> Box<dyn Rt> {
            Box::new(self.clone())
        }
    };
}

/// Small dense inverse (Gauss–Jordan) for the inductance matrices (up to 3×3).
fn inverse(m: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, String> {
    let n = m.len();
    let mut a: Vec<Vec<f64>> = m.iter().map(|r| { let mut r = r.clone(); r.resize(2 * n, 0.0); r }).collect();
    for (i, row) in a.iter_mut().enumerate() {
        row[n + i] = 1.0;
    }
    for k in 0..n {
        let p = (k..n).max_by(|&x, &y| a[x][k].abs().total_cmp(&a[y][k].abs())).unwrap();
        if a[p][k].abs() < 1e-300 || !a[p][k].is_finite() {
            return Err("the inductances give a singular matrix (check that every inductance is positive)".into());
        }
        a.swap(k, p);
        let d = a[k][k];
        a[k].iter_mut().for_each(|v| *v /= d);
        for i in 0..n {
            if i != k {
                let f = a[i][k];
                if f != 0.0 {
                    for j in 0..2 * n {
                        a[i][j] -= f * a[k][j];
                    }
                }
            }
        }
    }
    Ok(a.into_iter().map(|r| r[n..].to_vec()).collect())
}

fn mul(m: &[Vec<f64>], v: &[f64]) -> Vec<f64> {
    m.iter().map(|r| r.iter().zip(v).map(|(a, b)| a * b).sum()).collect()
}

fn pos(p: &Pv, k: &str) -> Result<f64, String> {
    let v = p.m.get(k).and_then(|m| m.s().ok()).ok_or_else(|| format!("{k} must be a single number"))?;
    if v > 0.0 && v.is_finite() {
        Ok(v)
    } else {
        Err(format!("{k} must be a positive number"))
    }
}
fn num(p: &Pv, k: &str) -> Result<f64, String> {
    let v = p.m.get(k).and_then(|m| m.s().ok()).ok_or_else(|| format!("{k} must be a single number"))?;
    if v.is_finite() {
        Ok(v)
    } else {
        Err(format!("{k} must be a finite number"))
    }
}
fn nonneg(p: &Pv, k: &str) -> Result<f64, String> {
    let v = num(p, k)?;
    if v >= 0.0 {
        Ok(v)
    } else {
        Err(format!("{k} cannot be negative"))
    }
}

fn need(iw: &[usize], widths: &[usize], names: &[&str]) -> Result<(), String> {
    for (i, (w, n)) in widths.iter().zip(names).enumerate() {
        let got = iw.get(i).cloned().unwrap_or(1);
        if got != *w {
            return Err(if *w == 3 {
                format!("input {} ({n}) needs a three-phase signal of width 3 (a Three-phase source, or a Mux of three signals); it has width {got}", i + 1)
            } else {
                format!("input {} ({n}) needs width {w}; it has width {got}", i + 1)
            });
        }
    }
    Ok(())
}

// ---------------------------------------------------------------- DC machines

#[derive(Clone, Copy, PartialEq, Debug)]
enum Dc {
    SepEx,
    Shunt,
    Series,
    Compound,
}

#[derive(Clone)]
struct DcMachine {
    kind: Dc,
    ra: f64,
    la: f64,
    rf: f64,
    lf: f64,
    laf: f64,
    rse: f64,
    lse: f64,
    lafse: f64,
    j: f64,
    b: f64,
    w0: f64,
}

impl DcMachine {
    /// (field flux linkage constant k = e/ω, armature current, field current) from the states.
    fn k(&self, x: &[f64]) -> (f64, f64, f64) {
        match self.kind {
            Dc::Series => (self.laf * x[0], x[0], x[0]),
            Dc::Compound => (self.laf * x[1] + self.lafse * x[0], x[0], x[1]),
            _ => (self.laf * x[1], x[0], x[1]),
        }
    }
    fn mech(&self) -> usize {
        if self.kind == Dc::Series { 1 } else { 2 }
    }
}

impl Rt for DcMachine {
    fn nx(&self) -> usize {
        self.mech() + 2
    }
    fn init(&mut self, x: &mut [f64]) {
        x.iter_mut().for_each(|v| *v = 0.0);
        x[self.mech()] = self.w0;
    }
    fn feed(&self) -> bool {
        false
    }
    fn out(&self, _: &Ctx, x: &[f64], _: &[f64], y: &mut [f64]) {
        let m = self.mech();
        let (k, ia, iff) = self.k(x);
        let te = k * ia;
        if self.kind == Dc::Series {
            y.copy_from_slice(&[x[m], ia, te, x[m + 1]]);
        } else {
            y.copy_from_slice(&[x[m], ia, iff, te, x[m + 1]]);
        }
    }
    fn deriv(&self, _: &Ctx, x: &[f64], u: &[f64], dx: &mut [f64]) {
        let m = self.mech();
        let w = x[m];
        let (k, ia, iff) = self.k(x);
        let tl = *u.last().unwrap();
        match self.kind {
            Dc::SepEx => {
                dx[0] = (u[0] - self.ra * ia - k * w) / self.la;
                dx[1] = (u[1] - self.rf * iff) / self.lf;
            }
            Dc::Shunt => {
                dx[0] = (u[0] - self.ra * ia - k * w) / self.la;
                dx[1] = (u[0] - self.rf * iff) / self.lf;
            }
            Dc::Series => {
                dx[0] = (u[0] - (self.ra + self.rf) * ia - k * w) / (self.la + self.lf);
            }
            Dc::Compound => {
                dx[0] = (u[0] - (self.ra + self.rse) * ia - k * w) / (self.la + self.lse);
                dx[1] = (u[0] - self.rf * iff) / self.lf;
            }
        }
        dx[m] = (k * ia - self.b * w - tl) / self.j;
        dx[m + 1] = w;
    }
    cb!();
}

// ---------------------------------------------------------------- induction machines

/// Induction machine in the stationary αβ frame, flux linkages as states: stator, then each rotor
/// circuit (one for squirrel-cage and wound-rotor, two for double-cage), then ω and θ.
#[derive(Clone)]
struct Induction {
    rs: f64,
    rr: Vec<f64>,
    minv: Vec<Vec<f64>>,
    p: f64,
    j: f64,
    b: f64,
    w0: f64,
    wound: bool,
}

impl Induction {
    fn nr(&self) -> usize {
        self.rr.len()
    }
    /// Currents per axis from the flux linkages: (α currents, β currents), stator first.
    fn currents(&self, x: &[f64]) -> (Vec<f64>, Vec<f64>) {
        let n = 1 + self.nr();
        let pa: Vec<f64> = (0..n).map(|k| x[2 * k]).collect();
        let pb: Vec<f64> = (0..n).map(|k| x[2 * k + 1]).collect();
        (mul(&self.minv, &pa), mul(&self.minv, &pb))
    }
    fn te(&self, x: &[f64], ia: &[f64], ib: &[f64]) -> f64 {
        1.5 * self.p * (x[0] * ib[0] - x[1] * ia[0])
    }
}

impl Rt for Induction {
    fn nx(&self) -> usize {
        2 * (1 + self.nr()) + 2
    }
    fn init(&mut self, x: &mut [f64]) {
        x.iter_mut().for_each(|v| *v = 0.0);
        let m = 2 * (1 + self.nr());
        x[m] = self.w0;
    }
    fn feed(&self) -> bool {
        false
    }
    fn out(&self, _: &Ctx, x: &[f64], _: &[f64], y: &mut [f64]) {
        let m = 2 * (1 + self.nr());
        let (ia, ib) = self.currents(x);
        let (a, b, c) = inv_clarke(ia[0], ib[0]);
        y[0] = x[m];
        y[1] = self.te(x, &ia, &ib);
        y[2..5].copy_from_slice(&[a, b, c]);
        y[5] = x[m + 1];
        if self.wound {
            // Rotor currents in the rotor's own frame.
            let (ra, rb) = park(ia[1], ib[1], self.p * x[m + 1]);
            let (a, b, c) = inv_clarke(ra, rb);
            y[6..9].copy_from_slice(&[a, b, c]);
        }
    }
    fn deriv(&self, _: &Ctx, x: &[f64], u: &[f64], dx: &mut [f64]) {
        let m = 2 * (1 + self.nr());
        let (w, th) = (x[m], x[m + 1]);
        let we = self.p * w;
        let (ia, ib) = self.currents(x);
        let (va, vb) = clarke(u[0], u[1], u[2]);
        dx[0] = va - self.rs * ia[0];
        dx[1] = vb - self.rs * ib[0];
        // Rotor voltages (wound rotor) are given in the rotor frame: turn them to the stator frame.
        let (vra, vrb) = if self.wound {
            let (a, b) = clarke(u[3], u[4], u[5]);
            inv_park(a, b, self.p * th)
        } else {
            (0.0, 0.0)
        };
        for k in 0..self.nr() {
            let (pa, pb) = (x[2 + 2 * k], x[3 + 2 * k]);
            let (v_a, v_b) = if k == 0 { (vra, vrb) } else { (0.0, 0.0) };
            dx[2 + 2 * k] = v_a - self.rr[k] * ia[1 + k] - we * pb;
            dx[3 + 2 * k] = v_b - self.rr[k] * ib[1 + k] + we * pa;
        }
        let tl = *u.last().unwrap();
        dx[m] = (self.te(x, &ia, &ib) - self.b * w - tl) / self.j;
        dx[m + 1] = w;
    }
    cb!();
}

// ---------------------------------------------------------------- synchronous machines

/// PMSM (or synchronous reluctance when ψ = 0) with three-phase stator inputs.
#[derive(Clone)]
struct Pmsm3 {
    rs: f64,
    ld: f64,
    lq: f64,
    psi: f64,
    p: f64,
    j: f64,
    b: f64,
    w0: f64,
}
impl Pmsm3 {
    fn te(&self, id: f64, iq: f64) -> f64 {
        1.5 * self.p * (self.psi * iq + (self.ld - self.lq) * id * iq)
    }
}
impl Rt for Pmsm3 {
    fn nx(&self) -> usize {
        4
    }
    fn init(&mut self, x: &mut [f64]) {
        x.copy_from_slice(&[0.0, 0.0, self.w0, 0.0]);
    }
    fn feed(&self) -> bool {
        false
    }
    fn out(&self, _: &Ctx, x: &[f64], _: &[f64], y: &mut [f64]) {
        let (al, be) = inv_park(x[0], x[1], self.p * x[3]);
        let (a, b, c) = inv_clarke(al, be);
        y.copy_from_slice(&[x[2], self.te(x[0], x[1]), a, b, c, x[3]]);
    }
    fn deriv(&self, _: &Ctx, x: &[f64], u: &[f64], dx: &mut [f64]) {
        let (id, iq, w, th) = (x[0], x[1], x[2], x[3]);
        let (al, be) = clarke(u[0], u[1], u[2]);
        let (vd, vq) = park(al, be, self.p * th);
        let we = self.p * w;
        dx[0] = (vd - self.rs * id + we * self.lq * iq) / self.ld;
        dx[1] = (vq - self.rs * iq - we * (self.ld * id + self.psi)) / self.lq;
        dx[2] = (self.te(id, iq) - self.b * w - u[3]) / self.j;
        dx[3] = w;
    }
    cb!();
}

/// Wound-field salient-pole synchronous machine with d and q damper windings, in the rotor dq
/// frame; flux linkages ψd, ψq, ψf, ψkd, ψkq as states. All rotor quantities are referred to the
/// stator.
#[derive(Clone)]
struct SyncField {
    rs: f64,
    rf: f64,
    rkd: f64,
    rkq: f64,
    md: Vec<Vec<f64>>,
    mdi: Vec<Vec<f64>>,
    mqi: Vec<Vec<f64>>,
    p: f64,
    j: f64,
    b: f64,
    w0: f64,
    th0: f64,
    if0: f64,
}
impl SyncField {
    fn cur(&self, x: &[f64]) -> ([f64; 3], [f64; 2]) {
        let d = mul(&self.mdi, &[x[0], x[2], x[3]]);
        let q = mul(&self.mqi, &[x[1], x[4]]);
        ([d[0], d[1], d[2]], [q[0], q[1]])
    }
}
impl Rt for SyncField {
    fn nx(&self) -> usize {
        7
    }
    fn init(&mut self, x: &mut [f64]) {
        // Excited at the initial field current, no stator or damper current.
        let fl = mul(&self.md, &[0.0, self.if0, 0.0]);
        x.copy_from_slice(&[fl[0], 0.0, fl[1], fl[2], 0.0, self.w0, self.th0]);
    }
    fn feed(&self) -> bool {
        false
    }
    fn out(&self, _: &Ctx, x: &[f64], _: &[f64], y: &mut [f64]) {
        let (d, q) = self.cur(x);
        let te = 1.5 * self.p * (x[0] * q[0] - x[1] * d[0]);
        let (al, be) = inv_park(d[0], q[0], self.p * x[6]);
        let (a, b, c) = inv_clarke(al, be);
        y.copy_from_slice(&[x[5], te, a, b, c, d[1], x[6]]);
    }
    fn deriv(&self, _: &Ctx, x: &[f64], u: &[f64], dx: &mut [f64]) {
        let (d, q) = self.cur(x);
        let (w, th) = (x[5], x[6]);
        let we = self.p * w;
        let (al, be) = clarke(u[0], u[1], u[2]);
        let (vd, vq) = park(al, be, self.p * th);
        dx[0] = vd - self.rs * d[0] + we * x[1];
        dx[1] = vq - self.rs * q[0] - we * x[0];
        dx[2] = u[3] - self.rf * d[1];
        dx[3] = -self.rkd * d[2];
        dx[4] = -self.rkq * q[1];
        let te = 1.5 * self.p * (x[0] * q[0] - x[1] * d[0]);
        dx[5] = (te - self.b * w - u[4]) / self.j;
        dx[6] = w;
    }
    cb!();
}

// ---------------------------------------------------------------- BLDC and stepper

/// BLDC motor with ideal six-step commutation: two phases conduct at a time with flat back-EMF,
/// so the drive sees one DC circuit: 2L·di/dt = V − 2R·i − 2·Ke·ω, T = 2·Ke·i.
#[derive(Clone)]
struct Bldc {
    r: f64,
    l: f64,
    ke: f64,
    p: f64,
    j: f64,
    b: f64,
}
impl Rt for Bldc {
    fn nx(&self) -> usize {
        3
    }
    fn feed(&self) -> bool {
        false
    }
    fn out(&self, _: &Ctx, x: &[f64], _: &[f64], y: &mut [f64]) {
        let the = self.p * x[2];
        let h = |k: f64| if (the - k * TAU3).sin() >= 0.0 { 1.0 } else { 0.0 };
        y.copy_from_slice(&[x[1], x[0], 2.0 * self.ke * x[0], x[2], h(0.0), h(1.0), h(2.0)]);
    }
    fn deriv(&self, _: &Ctx, x: &[f64], u: &[f64], dx: &mut [f64]) {
        dx[0] = (u[0] - 2.0 * self.r * x[0] - 2.0 * self.ke * x[1]) / (2.0 * self.l);
        dx[1] = (2.0 * self.ke * x[0] - self.b * x[1] - u[1]) / self.j;
        dx[2] = x[1];
    }
    cb!();
}

/// Two-phase hybrid stepper: Nr rotor teeth, torque constant Km, detent torque Td.
#[derive(Clone)]
struct Stepper {
    r: f64,
    l: f64,
    km: f64,
    nr: f64,
    td: f64,
    j: f64,
    b: f64,
}
impl Stepper {
    fn te(&self, x: &[f64]) -> f64 {
        let a = self.nr * x[3];
        -self.km * x[0] * a.sin() + self.km * x[1] * a.cos() - self.td * (4.0 * a).sin()
    }
}
impl Rt for Stepper {
    fn nx(&self) -> usize {
        4
    }
    fn feed(&self) -> bool {
        false
    }
    fn out(&self, _: &Ctx, x: &[f64], _: &[f64], y: &mut [f64]) {
        y.copy_from_slice(&[x[2], self.te(x), x[3], x[0], x[1]]);
    }
    fn deriv(&self, _: &Ctx, x: &[f64], u: &[f64], dx: &mut [f64]) {
        let a = self.nr * x[3];
        dx[0] = (u[0] - self.r * x[0] + self.km * x[2] * a.sin()) / self.l;
        dx[1] = (u[1] - self.r * x[1] - self.km * x[2] * a.cos()) / self.l;
        dx[2] = (self.te(x) - self.b * x[2] - u[2]) / self.j;
        dx[3] = x[2];
    }
    cb!();
}

// ---------------------------------------------------------------- drive pieces

#[derive(Clone)]
enum Tf {
    Clarke,
    InvClarke,
    Park,
    InvPark,
}
#[derive(Clone)]
struct Transform(Tf);
impl Rt for Transform {
    fn out(&self, _: &Ctx, _: &[f64], u: &[f64], y: &mut [f64]) {
        match self.0 {
            Tf::Clarke => {
                let (a, b) = clarke(u[0], u[1], u[2]);
                y.copy_from_slice(&[a, b]);
            }
            Tf::InvClarke => {
                let (a, b, c) = inv_clarke(u[0], u[1]);
                y.copy_from_slice(&[a, b, c]);
            }
            Tf::Park => {
                let (d, q) = park(u[0], u[1], u[2]);
                y.copy_from_slice(&[d, q]);
            }
            Tf::InvPark => {
                let (a, b) = inv_park(u[0], u[1], u[2]);
                y.copy_from_slice(&[a, b]);
            }
        }
    }
    cb!();
}

#[derive(Clone)]
struct ThreePhase {
    amp: f64,
    w: f64,
    ph: f64,
}
impl Rt for ThreePhase {
    fn feed(&self) -> bool {
        false
    }
    fn out(&self, c: &Ctx, _: &[f64], _: &[f64], y: &mut [f64]) {
        for (k, yk) in y.iter_mut().enumerate() {
            *yk = self.amp * (self.w * c.t + self.ph - k as f64 * TAU3).cos();
        }
    }
    cb!();
}

/// A triangle carrier in [−1, 1] with its peaks at whole and half periods.
fn carrier(t: f64, fc: f64) -> f64 {
    let ph = (t * fc).rem_euclid(1.0);
    if ph < 0.5 { 1.0 - 4.0 * ph } else { -3.0 + 4.0 * ph }
}

/// Sine-triangle PWM: each reference (−1…1) is compared with one triangle carrier. The switching
/// instants are located as zero-crossings; the carrier's corners are hit exactly.
#[derive(Clone)]
struct Pwm {
    fc: f64,
    on: Vec<bool>,
    /// Inverter output: phase voltages from the legs (needs Vdc); else ±1 or 0/1 gate signals.
    inverter: bool,
    unipolar: bool,
}
impl Rt for Pwm {
    fn init(&mut self, _: &mut [f64]) {
        self.on.iter_mut().for_each(|v| *v = false);
    }
    fn feed(&self) -> bool {
        // Gates change only at located crossings (updates), but the inverter's output scales with Vdc.
        self.inverter
    }
    fn out(&self, _: &Ctx, _: &[f64], u: &[f64], y: &mut [f64]) {
        let n = self.on.len();
        let leg = |k: usize| if self.on[k] { 1.0 } else { -1.0 };
        if self.inverter {
            let vdc = u[n];
            let mean = (0..n).map(leg).sum::<f64>() / n as f64;
            for (k, yk) in y.iter_mut().enumerate() {
                *yk = 0.5 * vdc * (leg(k) - mean);
            }
        } else {
            for (k, yk) in y.iter_mut().enumerate() {
                *yk = if self.unipolar { (leg(k) + 1.0) / 2.0 } else { leg(k) };
            }
        }
    }
    fn update(&mut self, c: &Ctx, _: &mut [f64], u: &[f64]) {
        let cr = carrier(c.t, self.fc);
        for (k, on) in self.on.iter_mut().enumerate() {
            *on = u[k] > cr;
        }
    }
    fn next_hit(&self, t: f64) -> f64 {
        let half = 0.5 / self.fc;
        let k = (t / half).floor() + 1.0;
        let mut h = k * half;
        if h <= t + 1e-12 * half.max(t.abs()) {
            h += half;
        }
        h
    }
    fn nzc(&self) -> usize {
        self.on.len()
    }
    fn zc(&self, c: &Ctx, _: &[f64], u: &[f64], z: &mut [f64]) {
        let cr = carrier(c.t, self.fc);
        for (k, zk) in z.iter_mut().enumerate() {
            *zk = u[k] - cr;
        }
    }
    cb!();
}

/// Average-model inverter: phase voltages Vdc/2·(m − mean m), references limited to ±1.
#[derive(Clone)]
struct InverterAvg;
impl Rt for InverterAvg {
    fn out(&self, _: &Ctx, _: &[f64], u: &[f64], y: &mut [f64]) {
        let m: Vec<f64> = (0..3).map(|k| u[k].clamp(-1.0, 1.0)).collect();
        let mean = m.iter().sum::<f64>() / 3.0;
        for k in 0..3 {
            y[k] = 0.5 * u[3] * (m[k] - mean);
        }
    }
    cb!();
}

/// Incremental encoder: the angle in counts (4 per pulse), and a speed estimate from the change
/// of counts over each sample time.
#[derive(Clone)]
struct Encoder {
    q: f64,
    ts: f64,
    last: f64,
    w: f64,
}
impl Encoder {
    fn meas(&self, th: f64) -> f64 {
        self.q * (th / self.q).floor()
    }
}
impl Rt for Encoder {
    fn rate(&self) -> Rate {
        Rate::Discrete(self.ts)
    }
    fn init(&mut self, _: &mut [f64]) {
        self.last = f64::NAN;
        self.w = 0.0;
    }
    fn out(&self, _: &Ctx, _: &[f64], u: &[f64], y: &mut [f64]) {
        y[0] = self.meas(u[0]);
        y[1] = self.w;
    }
    fn update(&mut self, c: &Ctx, _: &mut [f64], u: &[f64]) {
        let k = (c.t / self.ts).round();
        if (c.t - k * self.ts).abs() <= 1e-9 * self.ts.max(c.t.abs() * 1e-3) {
            let m = self.meas(u[0]);
            if self.last.is_finite() {
                self.w = (m - self.last) / self.ts;
            }
            self.last = m;
        }
    }
    fn next_hit(&self, t: f64) -> f64 {
        let k = (t / self.ts).floor() + 1.0;
        let mut h = k * self.ts;
        if h <= t + 1e-12 * self.ts.max(t.abs()) {
            h += self.ts;
        }
        h
    }
    cb!();
}

// ---------------------------------------------------------------- switched reluctance

/// Three-phase switched reluctance motor without saturation: L_k(θ) = L0 − L1·cos(Nr·θ − k·2π/3),
/// flux linkages as states, T = Σ ½·i_k²·dL_k/dθ. With an asymmetric bridge the phase currents
/// cannot reverse (the diodes): a phase's flux stops falling at zero.
#[derive(Clone)]
struct Srm {
    r: f64,
    l0: f64,
    l1: f64,
    nr: f64,
    j: f64,
    b: f64,
    w0: f64,
    th0: f64,
    oneway: bool,
}
impl Srm {
    fn l(&self, th: f64, k: usize) -> (f64, f64) {
        let a = self.nr * th - k as f64 * TAU3;
        (self.l0 - self.l1 * a.cos(), self.l1 * self.nr * a.sin())
    }
    fn i(&self, x: &[f64], k: usize) -> f64 {
        let psi = if self.oneway { x[k].max(0.0) } else { x[k] };
        psi / self.l(x[4], k).0
    }
    fn te(&self, x: &[f64]) -> f64 {
        (0..3).map(|k| 0.5 * self.i(x, k).powi(2) * self.l(x[4], k).1).sum()
    }
}
impl Rt for Srm {
    fn nx(&self) -> usize {
        5
    }
    fn init(&mut self, x: &mut [f64]) {
        x.copy_from_slice(&[0.0, 0.0, 0.0, self.w0, self.th0]);
    }
    fn feed(&self) -> bool {
        false
    }
    fn out(&self, _: &Ctx, x: &[f64], _: &[f64], y: &mut [f64]) {
        y.copy_from_slice(&[x[3], self.te(x), self.i(x, 0), self.i(x, 1), self.i(x, 2), x[4]]);
    }
    fn deriv(&self, _: &Ctx, x: &[f64], u: &[f64], dx: &mut [f64]) {
        for k in 0..3 {
            let d = u[k] - self.r * self.i(x, k);
            dx[k] = if self.oneway && x[k] <= 0.0 && d < 0.0 { 0.0 } else { d };
        }
        dx[3] = (self.te(x) - self.b * x[3] - u[3]) / self.j;
        dx[4] = x[3];
    }
    fn update(&mut self, _: &Ctx, x: &mut [f64], _: &[f64]) {
        if self.oneway {
            x[..3].iter_mut().for_each(|v| *v = v.max(0.0));
        }
    }
    cb!();
}

// ---------------------------------------------------------------- DC link, sensors

/// DC link capacitor fed from a source through a resistance: C·dV/dt = i_s − i_load.
#[derive(Clone)]
struct DcLink {
    c: f64,
    rs: f64,
    v0: f64,
    diode: bool,
}
impl DcLink {
    fn is(&self, vs: f64, v: f64) -> f64 {
        let i = (vs - v) / self.rs;
        if self.diode { i.max(0.0) } else { i }
    }
}
impl Rt for DcLink {
    fn nx(&self) -> usize {
        1
    }
    fn init(&mut self, x: &mut [f64]) {
        x[0] = self.v0;
    }
    fn out(&self, _: &Ctx, x: &[f64], u: &[f64], y: &mut [f64]) {
        y[0] = x[0];
        y[1] = self.is(u[0], x[0]);
    }
    fn deriv(&self, _: &Ctx, x: &[f64], u: &[f64], dx: &mut [f64]) {
        dx[0] = (self.is(u[0], x[0]) - u[1]) / self.c;
    }
    cb!();
}

/// The current an average-model inverter draws from its DC link: power balance,
/// i_dc = ½·(Σ m_k·i_k − mean(m)·Σ i_k) with m limited to ±1, as in the Inverter.
#[derive(Clone)]
struct InverterDc;
impl Rt for InverterDc {
    fn out(&self, _: &Ctx, _: &[f64], u: &[f64], y: &mut [f64]) {
        let m: Vec<f64> = (0..3).map(|k| u[k].clamp(-1.0, 1.0)).collect();
        let mean = m.iter().sum::<f64>() / 3.0;
        let isum: f64 = u[3..6].iter().sum();
        y[0] = 0.5 * (m.iter().zip(&u[3..6]).map(|(a, b)| a * b).sum::<f64>() - mean * isum);
    }
    cb!();
}

/// A current (or any) sensor: gain, offset, first-order bandwidth and noise held each sample time.
#[derive(Clone)]
struct Sensor {
    k: f64,
    off: f64,
    wc: f64,
    sd: f64,
    ts: f64,
    seed: u64,
    state: u64,
    noise: Vec<f64>,
}
impl Sensor {
    fn gauss(&mut self) -> f64 {
        let mut u = || {
            self.state ^= self.state << 13;
            self.state ^= self.state >> 7;
            self.state ^= self.state << 17;
            ((self.state >> 11) as f64 + 0.5) / (1u64 << 53) as f64
        };
        let (a, b) = (u(), u());
        (-2.0 * a.ln()).sqrt() * (2.0 * PI * b).cos()
    }
    fn draw(&mut self) {
        for k in 0..self.noise.len() {
            self.noise[k] = if self.sd > 0.0 { self.sd * self.gauss() } else { 0.0 };
        }
    }
}
impl Rt for Sensor {
    fn nx(&self) -> usize {
        if self.wc > 0.0 { self.noise.len() } else { 0 }
    }
    fn init(&mut self, x: &mut [f64]) {
        x.iter_mut().for_each(|v| *v = 0.0);
        self.state = self.seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
        self.draw();
    }
    fn feed(&self) -> bool {
        self.wc <= 0.0
    }
    fn out(&self, _: &Ctx, x: &[f64], u: &[f64], y: &mut [f64]) {
        for (k, yk) in y.iter_mut().enumerate() {
            let s = if self.wc > 0.0 { x[k] } else { u[k] };
            *yk = self.k * s + self.off + self.noise[k];
        }
    }
    fn deriv(&self, _: &Ctx, x: &[f64], u: &[f64], dx: &mut [f64]) {
        for k in 0..dx.len() {
            dx[k] = self.wc * (u[k] - x[k]);
        }
    }
    fn update(&mut self, c: &Ctx, _: &mut [f64], _: &[f64]) {
        if self.sd > 0.0 && crate::rt::is_hit(c.t, self.ts) && c.t > 0.0 {
            self.draw();
        }
    }
    fn next_hit(&self, t: f64) -> f64 {
        if self.sd > 0.0 { crate::rt::next_sample(t, self.ts) } else { f64::INFINITY }
    }
    cb!();
}

// ---------------------------------------------------------------- build

pub const KINDS: &[&str] = &[
    "DCSepEx", "DCShunt", "DCSeries", "DCCompound", "InductionSC", "InductionDC", "InductionWR", "PMSM3", "SynRM", "SyncField",
    "BLDC", "Stepper", "Clarke", "InvClarke", "Park", "InvPark", "ThreePhase", "Inverter", "PWM", "Encoder",
    "SRM", "DCLink", "InverterDC", "Sensor",
];

pub fn build(kind: &str, p: &Pv, iw: &[usize]) -> Option<Result<(Box<dyn Rt>, Vec<usize>), String>> {
    if !KINDS.contains(&kind) {
        return None;
    }
    Some(build_inner(kind, p, iw))
}

fn ok(r: impl Rt + 'static, w: Vec<usize>) -> Result<(Box<dyn Rt>, Vec<usize>), String> {
    Ok((Box::new(r), w))
}

fn build_inner(kind: &str, p: &Pv, iw: &[usize]) -> Result<(Box<dyn Rt>, Vec<usize>), String> {
    match kind {
        "DCSepEx" | "DCShunt" | "DCSeries" | "DCCompound" => {
            let k = match kind {
                "DCSepEx" => Dc::SepEx,
                "DCShunt" => Dc::Shunt,
                "DCSeries" => Dc::Series,
                _ => Dc::Compound,
            };
            need(iw, &vec![1; iw.len()], &["", "", ""])?;
            let m = DcMachine {
                kind: k,
                ra: nonneg(p, "Ra")?,
                la: pos(p, "La")?,
                rf: nonneg(p, "Rf")?,
                lf: if k == Dc::Series { nonneg(p, "Lf")? } else { pos(p, "Lf")? },
                laf: pos(p, "Laf")?,
                rse: if k == Dc::Compound { nonneg(p, "Rse")? } else { 0.0 },
                lse: if k == Dc::Compound { nonneg(p, "Lse")? } else { 0.0 },
                lafse: if k == Dc::Compound { num(p, "Lafse")? } else { 0.0 },
                j: pos(p, "J")?,
                b: nonneg(p, "b")?,
                w0: num(p, "w0")?,
            };
            let n = if k == Dc::Series { 4 } else { 5 };
            ok(m, vec![1; n])
        }
        "InductionSC" | "InductionDC" | "InductionWR" => {
            let wound = kind == "InductionWR";
            if wound {
                need(iw, &[3, 3, 1], &["stator voltages", "rotor voltages", "load torque"])?;
            } else {
                need(iw, &[3, 1], &["stator voltages", "load torque"])?;
            }
            let (lls, lm) = (pos(p, "Lls")?, pos(p, "Lm")?);
            let mut rr = vec![pos(p, "Rr")?];
            let mut llr = vec![pos(p, "Llr")?];
            if kind == "InductionDC" {
                rr.push(pos(p, "Rr2")?);
                llr.push(pos(p, "Llr2")?);
            }
            let n = 1 + rr.len();
            let mut mm = vec![vec![lm; n]; n];
            mm[0][0] = lls + lm;
            for k in 0..rr.len() {
                mm[k + 1][k + 1] = llr[k] + lm;
            }
            let minv = inverse(&mm)?;
            let pp = pos(p, "p")?;
            let m = Induction { rs: nonneg(p, "Rs")?, rr, minv, p: pp, j: pos(p, "J")?, b: nonneg(p, "b")?, w0: num(p, "w0")?, wound };
            ok(m, if wound { vec![1, 1, 3, 1, 3] } else { vec![1, 1, 3, 1] })
        }
        "PMSM3" | "SynRM" => {
            need(iw, &[3, 1], &["stator voltages", "load torque"])?;
            let psi = if kind == "SynRM" { 0.0 } else { nonneg(p, "psi")? };
            let m = Pmsm3 { rs: nonneg(p, "Rs")?, ld: pos(p, "Ld")?, lq: pos(p, "Lq")?, psi, p: pos(p, "p")?, j: pos(p, "J")?, b: nonneg(p, "b")?, w0: num(p, "w0")? };
            ok(m, vec![1, 1, 3, 1])
        }
        "SyncField" => {
            need(iw, &[3, 1, 1], &["stator voltages", "field voltage", "load torque"])?;
            let (ll, lmd, lmq) = (pos(p, "Ll")?, pos(p, "Lmd")?, pos(p, "Lmq")?);
            let (llf, llkd, llkq) = (pos(p, "Llf")?, pos(p, "Llkd")?, pos(p, "Llkq")?);
            let md = vec![vec![ll + lmd, lmd, lmd], vec![lmd, llf + lmd, lmd], vec![lmd, lmd, llkd + lmd]];
            let mq = vec![vec![ll + lmq, lmq], vec![lmq, llkq + lmq]];
            let m = SyncField {
                rs: nonneg(p, "Rs")?,
                rf: pos(p, "Rf")?,
                rkd: pos(p, "Rkd")?,
                rkq: pos(p, "Rkq")?,
                mdi: inverse(&md)?,
                mqi: inverse(&mq)?,
                md,
                p: pos(p, "p")?,
                j: pos(p, "J")?,
                b: nonneg(p, "b")?,
                w0: num(p, "w0")?,
                th0: num(p, "th0")?,
                if0: num(p, "if0")?,
            };
            ok(m, vec![1, 1, 3, 1, 1])
        }
        "BLDC" => {
            need(iw, &[1, 1], &["voltage", "load torque"])?;
            ok(Bldc { r: nonneg(p, "R")?, l: pos(p, "L")?, ke: pos(p, "Ke")?, p: pos(p, "p")?, j: pos(p, "J")?, b: nonneg(p, "b")? }, vec![1, 1, 1, 1, 3])
        }
        "Stepper" => {
            need(iw, &[1, 1, 1], &["phase A voltage", "phase B voltage", "load torque"])?;
            ok(Stepper { r: nonneg(p, "R")?, l: pos(p, "L")?, km: pos(p, "Km")?, nr: pos(p, "Nr")?, td: nonneg(p, "Td")?, j: pos(p, "J")?, b: nonneg(p, "b")? }, vec![1; 5])
        }
        "Clarke" => {
            need(iw, &[3], &["abc"])?;
            ok(Transform(Tf::Clarke), vec![2])
        }
        "InvClarke" => {
            need(iw, &[2], &["αβ"])?;
            ok(Transform(Tf::InvClarke), vec![3])
        }
        "Park" => {
            need(iw, &[2, 1], &["αβ", "angle"])?;
            ok(Transform(Tf::Park), vec![2])
        }
        "InvPark" => {
            need(iw, &[2, 1], &["dq", "angle"])?;
            ok(Transform(Tf::InvPark), vec![2])
        }
        "ThreePhase" => {
            let vll = nonneg(p, "Vll")?;
            ok(ThreePhase { amp: vll * SQ3.recip() * 2f64.sqrt(), w: 2.0 * PI * num(p, "f")?, ph: num(p, "phase")? }, vec![3])
        }
        "Inverter" => {
            need(iw, &[3, 1], &["modulation", "DC voltage"])?;
            if p.t.get("model").map(|s| s.as_str()) == Some("switched (PWM)") {
                ok(Pwm { fc: pos(p, "fc")?, on: vec![false; 3], inverter: true, unipolar: false }, vec![3])
            } else {
                ok(InverterAvg, vec![3])
            }
        }
        "PWM" => {
            let w = iw.first().cloned().unwrap_or(1);
            let unipolar = p.t.get("levels").map(|s| s.as_str()) == Some("0 / 1");
            ok(Pwm { fc: pos(p, "fc")?, on: vec![false; w], inverter: false, unipolar }, vec![w])
        }
        "Encoder" => {
            need(iw, &[1], &["angle"])?;
            let ppr = pos(p, "ppr")?;
            ok(Encoder { q: 2.0 * PI / (4.0 * ppr), ts: pos(p, "ts")?, last: f64::NAN, w: 0.0 }, vec![1, 1])
        }
        "SRM" => {
            need(iw, &[3, 1], &["phase voltages", "load torque"])?;
            let (lmax, lmin) = (pos(p, "Lmax")?, pos(p, "Lmin")?);
            if lmin >= lmax {
                return Err("the unaligned inductance Lmin must be below the aligned Lmax".into());
            }
            let m = Srm {
                r: nonneg(p, "R")?,
                l0: 0.5 * (lmax + lmin),
                l1: 0.5 * (lmax - lmin),
                nr: pos(p, "Nr")?,
                j: pos(p, "J")?,
                b: nonneg(p, "b")?,
                w0: num(p, "w0")?,
                th0: num(p, "th0")?,
                oneway: p.t.get("currents").map(|s| s.as_str()) != Some("both ways"),
            };
            ok(m, vec![1, 1, 3, 1])
        }
        "DCLink" => {
            need(iw, &[1, 1], &["source voltage", "load current"])?;
            let diode = p.t.get("source").map(|s| s.as_str()) == Some("diode rectifier (charges only)");
            ok(DcLink { c: pos(p, "C")?, rs: pos(p, "Rs")?, v0: num(p, "V0")?, diode }, vec![1, 1])
        }
        "InverterDC" => {
            need(iw, &[3, 3], &["modulation", "phase currents"])?;
            ok(InverterDc, vec![1])
        }
        "Sensor" => {
            let w = iw.first().cloned().unwrap_or(1);
            let bw = nonneg(p, "bw")?;
            let seed = num(p, "seed")?.abs() as u64;
            ok(Sensor { k: num(p, "k")?, off: num(p, "offset")?, wc: 2.0 * PI * bw, sd: nonneg(p, "sd")?, ts: pos(p, "ts")?, seed, state: 1, noise: vec![0.0; w] }, vec![w])
        }
        _ => Err(format!("unknown motor block '{kind}'")),
    }
}

// ---------------------------------------------------------------- rating presets

/// A named set of typical parameter values for a rating; the user replaces them with nameplate data.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Preset {
    pub name: String,
    pub values: Vec<(String, String)>,
}

/// A value to six significant digits, written as plainly as possible.
fn f(v: f64) -> String {
    let r: f64 = format!("{v:.5e}").parse().unwrap_or(v);
    let a = r.abs();
    if a != 0.0 && !(1e-4..1e7).contains(&a) {
        format!("{r:e}")
    } else {
        format!("{r}")
    }
}

fn preset(name: &str, kv: &[(&str, f64)]) -> Preset {
    Preset { name: name.into(), values: kv.iter().map(|(k, v)| (k.to_string(), f(*v))).collect() }
}

/// Induction machine values from per-unit data: line voltage, power (kW), frequency, pole pairs,
/// per-unit resistances and reactances, and the inertia constant H (s).
#[allow(clippy::too_many_arguments)]
fn im(name: &str, vll: f64, kw: f64, hz: f64, pp: f64, rs: f64, rr: f64, xl: f64, xm: f64, h: f64, cage2: Option<(f64, f64)>) -> Preset {
    let s = kw * 1e3 / 0.85;
    let zb = vll * vll / s;
    let lb = zb / (2.0 * PI * hz);
    let wm = 2.0 * PI * hz / pp;
    let mut kv = vec![("Rs", rs * zb), ("Lls", xl * lb), ("Lm", xm * lb), ("Rr", rr * zb), ("Llr", xl * lb), ("p", pp), ("J", 2.0 * h * s / (wm * wm)), ("b", 0.0)];
    if let Some((r2, x2)) = cage2 {
        // Outer cage: high resistance, low leakage (starting); inner cage: the running cage.
        kv[3] = ("Rr", 3.0 * rr * zb);
        kv[4] = ("Llr", 0.3 * xl * lb);
        kv.push(("Rr2", r2 * zb));
        kv.push(("Llr2", x2 * lb));
    }
    preset(name, &kv)
}

/// Wound-field synchronous machine values from per-unit data; if0 gives rated voltage at no load.
fn sm(name: &str, vll: f64, mva: f64, hz: f64, pp: f64, h: f64) -> Preset {
    let s = mva * 1e6;
    let zb = vll * vll / s;
    let lb = zb / (2.0 * PI * hz);
    let wm = 2.0 * PI * hz / pp;
    let lmd = 1.0 * lb;
    let vpk = vll * (2.0f64 / 3.0).sqrt();
    preset(name, &[
        ("Rs", 0.004 * zb), ("Ll", 0.12 * lb), ("Lmd", lmd), ("Lmq", 0.6 * lb), ("Rf", 0.0012 * zb), ("Llf", 0.18 * lb),
        ("Rkd", 0.02 * zb), ("Llkd", 0.12 * lb), ("Rkq", 0.03 * zb), ("Llkq", 0.2 * lb), ("p", pp), ("J", 2.0 * h * s / (wm * wm)),
        ("b", 0.0), ("if0", vpk / (2.0 * PI * hz * lmd)),
    ])
}

pub fn presets(kind: &str) -> Vec<Preset> {
    let dc = |name: &str, ra: f64, la: f64, rf: f64, lf: f64, laf: f64, j: f64| preset(name, &[("Ra", ra), ("La", la), ("Rf", rf), ("Lf", lf), ("Laf", laf), ("J", j), ("b", 0.0)]);
    match kind {
        "DCSepEx" => vec![
            dc("LV 220 V · 5 kW · 1500 rpm (field 220 V)", 0.6, 0.012, 220.0, 50.0, 1.3, 0.15),
            dc("LV 500 V · 100 kW · 1500 rpm (field 220 V)", 0.05, 0.002, 100.0, 20.0, 1.39, 4.0),
            dc("Large 750 V · 1 MW · 600 rpm (field 220 V)", 0.006, 0.0005, 50.0, 10.0, 2.6, 200.0),
        ],
        "DCShunt" => vec![
            dc("LV 220 V · 5 kW · 1500 rpm", 0.6, 0.012, 220.0, 50.0, 1.3, 0.15),
            dc("LV 500 V · 100 kW · 1500 rpm", 0.05, 0.002, 230.0, 40.0, 1.41, 4.0),
            dc("Large 750 V · 1 MW · 600 rpm", 0.006, 0.0005, 170.0, 20.0, 2.6, 200.0),
        ],
        "DCSeries" => vec![
            preset("LV 220 V · 5 kW (hoist)", &[("Ra", 0.4), ("La", 0.01), ("Rf", 0.2), ("Lf", 0.01), ("Laf", 0.052), ("J", 0.15), ("b", 0.0)]),
            preset("Traction 600 V · 300 kW", &[("Ra", 0.03), ("La", 0.002), ("Rf", 0.02), ("Lf", 0.002), ("Laf", 0.0053), ("J", 15.0), ("b", 0.0)]),
        ],
        "DCCompound" => vec![preset("LV 220 V · 5 kW · 1500 rpm", &[
            ("Ra", 0.5), ("La", 0.01), ("Rse", 0.1), ("Lse", 0.002), ("Lafse", 0.005), ("Rf", 220.0), ("Lf", 50.0), ("Laf", 1.25), ("J", 0.15), ("b", 0.0),
        ])],
        "InductionSC" | "InductionWR" => vec![
            im("LV 400 V · 11 kW · 50 Hz · 4-pole", 400.0, 11.0, 50.0, 2.0, 0.03, 0.025, 0.1, 3.0, 0.1, None),
            im("LV 690 V · 250 kW · 50 Hz · 4-pole", 690.0, 250.0, 50.0, 2.0, 0.01, 0.01, 0.1, 3.5, 0.5, None),
            im("MV 3.3 kV · 500 kW · 50 Hz · 4-pole", 3300.0, 500.0, 50.0, 2.0, 0.008, 0.008, 0.12, 3.5, 0.6, None),
            im("MV 6.6 kV · 2 MW · 50 Hz · 4-pole", 6600.0, 2000.0, 50.0, 2.0, 0.006, 0.007, 0.12, 3.8, 0.8, None),
            im("HV 11 kV · 5 MW · 50 Hz · 4-pole", 11000.0, 5000.0, 50.0, 2.0, 0.005, 0.005, 0.13, 4.0, 1.0, None),
            im("LV 460 V · 15 kW · 60 Hz · 4-pole", 460.0, 15.0, 60.0, 2.0, 0.03, 0.025, 0.1, 3.0, 0.1, None),
        ],
        "InductionDC" => vec![
            im("LV 400 V · 11 kW · 50 Hz · 4-pole", 400.0, 11.0, 50.0, 2.0, 0.03, 0.025, 0.1, 3.0, 0.1, Some((0.012, 0.15))),
            im("MV 3.3 kV · 500 kW · 50 Hz · 4-pole", 3300.0, 500.0, 50.0, 2.0, 0.008, 0.008, 0.12, 3.5, 0.6, Some((0.006, 0.16))),
            im("HV 11 kV · 5 MW · 50 Hz · 4-pole", 11000.0, 5000.0, 50.0, 2.0, 0.005, 0.005, 0.13, 4.0, 1.0, Some((0.004, 0.17))),
        ],
        "SyncField" => vec![
            sm("MV 3.3 kV · 1 MVA · 50 Hz · 4-pole", 3300.0, 1.0, 50.0, 2.0, 1.0),
            sm("HV 11 kV · 5 MVA · 50 Hz · 4-pole", 11000.0, 5.0, 50.0, 2.0, 1.5),
            sm("HV 13.8 kV · 20 MVA · 60 Hz · 8-pole", 13800.0, 20.0, 60.0, 4.0, 2.0),
        ],
        "PMSM3" | "PMSM" => vec![
            preset("Servo 48 V · 1 kW · 3000 rpm", &[("Rs", 0.05), ("Ld", 0.2e-3), ("Lq", 0.2e-3), ("psi", 0.021), ("p", 4.0), ("J", 1e-4), ("b", 1e-5)]),
            preset("Industrial 400 V · 15 kW (interior magnets)", &[("Rs", 0.15), ("Ld", 2.5e-3), ("Lq", 4e-3), ("psi", 0.4), ("p", 4.0), ("J", 0.03), ("b", 1e-3)]),
            preset("Traction 690 V · 200 kW (interior magnets)", &[("Rs", 0.01), ("Ld", 0.2e-3), ("Lq", 0.6e-3), ("psi", 0.2), ("p", 4.0), ("J", 0.5), ("b", 0.01)]),
        ],
        "SynRM" => vec![
            preset("LV 400 V · 5.5 kW", &[("Rs", 0.8), ("Ld", 0.12), ("Lq", 0.02), ("p", 2.0), ("J", 0.02), ("b", 1e-3)]),
            preset("LV 400 V · 30 kW", &[("Rs", 0.1), ("Ld", 0.025), ("Lq", 0.004), ("p", 2.0), ("J", 0.2), ("b", 5e-3)]),
        ],
        "BLDC" => vec![
            preset("24 V · 100 W", &[("R", 0.3), ("L", 0.3e-3), ("Ke", 0.02), ("p", 4.0), ("J", 2e-5), ("b", 1e-6)]),
            preset("48 V · 750 W", &[("R", 0.05), ("L", 0.1e-3), ("Ke", 0.045), ("p", 4.0), ("J", 3e-4), ("b", 1e-5)]),
            preset("310 V · 3 kW", &[("R", 0.3), ("L", 2e-3), ("Ke", 0.45), ("p", 4.0), ("J", 2e-3), ("b", 1e-4)]),
        ],
        "Stepper" => vec![
            preset("NEMA 17 · 1.8° · 1.7 A", &[("R", 1.65), ("L", 2.8e-3), ("Km", 0.25), ("Nr", 50.0), ("Td", 0.01), ("J", 5.4e-6), ("b", 1e-5)]),
            preset("NEMA 23 · 1.8° · 2.8 A", &[("R", 0.9), ("L", 3.6e-3), ("Km", 0.45), ("Nr", 50.0), ("Td", 0.03), ("J", 3e-5), ("b", 3e-5)]),
        ],
        "SRM" => vec![
            preset("LV 400 V · 4 kW · 6/4", &[("R", 1.3), ("Lmax", 0.06), ("Lmin", 0.008), ("Nr", 4.0), ("J", 0.008), ("b", 1e-3)]),
            preset("LV 400 V · 7.5 kW · 12/8", &[("R", 0.5), ("Lmax", 0.04), ("Lmin", 0.006), ("Nr", 8.0), ("J", 0.02), ("b", 2e-3)]),
        ],
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transforms_round_trip() {
        let (a, b, c) = (1.0, -0.3, -0.7);
        let (al, be) = clarke(a, b, c);
        let (x, y, z) = inv_clarke(al, be);
        assert!((x - a).abs() < 1e-12 && (y - b).abs() < 1e-12 && (z - c).abs() < 1e-12);
        let (d, q) = park(al, be, 0.7);
        let (al2, be2) = inv_park(d, q, 0.7);
        assert!((al2 - al).abs() < 1e-12 && (be2 - be).abs() < 1e-12);
        // A balanced cosine set at angle θ is (amplitude, 0) in dq at θ.
        let th: f64 = 1.1;
        let (al, be) = clarke(th.cos(), (th - TAU3).cos(), (th + TAU3).cos());
        let (d, q) = park(al, be, th);
        assert!((d - 1.0).abs() < 1e-12 && q.abs() < 1e-12);
    }
    #[test]
    fn small_inverse() {
        let m = vec![vec![2.0, 1.0, 1.0], vec![1.0, 3.0, 1.0], vec![1.0, 1.0, 4.0]];
        let i = inverse(&m).unwrap();
        for r in 0..3 {
            for c in 0..3 {
                let v: f64 = (0..3).map(|k| m[r][k] * i[k][c]).sum();
                assert!((v - if r == c { 1.0 } else { 0.0 }).abs() < 1e-12);
            }
        }
    }
    /// Every preset value reads back as the number it was made from (to six digits), and names a
    /// parameter of its block.
    #[test]
    fn presets_round_trip() {
        assert_eq!(f(50.0), "50");
        assert_eq!(f(220.0), "220");
        assert_eq!(f(5.4e-6), "5.4e-6");
        assert_eq!(f(0.0028), "0.0028");
        for kind in KINDS.iter().chain(&["PMSM"]) {
            let def = crate::blocks::def(kind).unwrap();
            for p in presets(kind) {
                for (k, v) in &p.values {
                    assert!(def.params.iter().any(|q| q.key == k), "{kind} {}: no parameter {k}", p.name);
                    let x: f64 = v.parse().unwrap_or_else(|_| panic!("{kind} {k} = {v}"));
                    assert!(x.is_finite() && x >= 0.0, "{kind} {k} = {v}");
                }
            }
        }
        // Spot checks against the numbers they come from.
        let s = &presets("Stepper")[0].values;
        assert!(s.iter().any(|(k, v)| k == "Nr" && v == "50"));
    }
    #[test]
    fn carrier_shape() {
        assert_eq!(carrier(0.0, 1.0), 1.0);
        assert_eq!(carrier(0.5, 1.0), -1.0);
        assert!((carrier(0.25, 1.0)).abs() < 1e-12);
    }
}
