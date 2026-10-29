//! Discrete-time blocks (0.1.4): discrete transfer function and state space, discrete integrator,
//! discrete PID, and the extra signal sources (square, triangle, chirp, repeating sequence).
//!
//! A discrete block keeps its own state and changes only at its sample hits: at a hit (a major
//! step landing on k·Ts) the output is computed from the state and the input, the state moves on in
//! `update`, and between hits the output holds. The solver lands on every hit (`next_hit`).

use crate::expr::Mat;
use crate::rt::{is_hit, next_sample, tf2ss, Ctx, Pv, Rate, Rt};
use std::f64::consts::PI;

macro_rules! cb {
    () => {
        fn clone_box(&self) -> Box<dyn Rt> {
            Box::new(self.clone())
        }
    };
}

fn mv(m: &Mat, v: &[f64], out: &mut [f64], add: bool) {
    for (i, o) in out.iter_mut().enumerate() {
        let s: f64 = v.iter().enumerate().map(|(j, x)| m.at(i, j) * x).sum();
        if add {
            *o += s;
        } else {
            *o = s;
        }
    }
}

/// x[k+1] = A·x[k] + B·u[k],  y[k] = C·x[k] + D·u[k].
#[derive(Clone)]
struct DLin {
    a: Mat,
    b: Mat,
    c: Mat,
    d: Mat,
    x0: Vec<f64>,
    x: Vec<f64>,
    held: Vec<f64>,
    ts: f64,
}
impl DLin {
    fn y(&self, u: &[f64], y: &mut [f64]) {
        mv(&self.c, &self.x, y, false);
        mv(&self.d, u, y, true);
    }
}
impl Rt for DLin {
    fn rate(&self) -> Rate {
        Rate::Discrete(self.ts)
    }
    fn init(&mut self, _: &mut [f64]) {
        self.x.clone_from(&self.x0);
        let z = vec![0.0; self.b.c];
        let mut h = vec![0.0; self.c.r];
        self.y(&z, &mut h);
        self.held = h;
    }
    fn feed(&self) -> bool {
        self.d.d.iter().any(|v| *v != 0.0)
    }
    fn out(&self, c: &Ctx, _: &[f64], u: &[f64], y: &mut [f64]) {
        if c.major && is_hit(c.t, self.ts) {
            self.y(u, y);
        } else {
            y.copy_from_slice(&self.held);
        }
    }
    fn update(&mut self, c: &Ctx, _: &mut [f64], u: &[f64]) {
        if is_hit(c.t, self.ts) {
            let mut h = vec![0.0; self.c.r];
            self.y(u, &mut h);
            self.held = h;
            let mut xn = vec![0.0; self.x.len()];
            mv(&self.a, &self.x, &mut xn, false);
            mv(&self.b, u, &mut xn, true);
            self.x = xn;
        }
    }
    fn next_hit(&self, t: f64) -> f64 {
        next_sample(t, self.ts)
    }
    cb!();
}

#[derive(Clone, Copy, PartialEq)]
enum Edge {
    None,
    Rising,
    Falling,
    Either,
}
impl Edge {
    fn parse(s: &str) -> Edge {
        match s {
            "rising" => Edge::Rising,
            "falling" => Edge::Falling,
            "either" => Edge::Either,
            _ => Edge::None,
        }
    }
    /// Did the reset signal cross zero this way between `a` (before) and `b` (now)?
    pub fn hit(self, a: f64, b: f64) -> bool {
        if !a.is_finite() {
            return false;
        }
        let up = a <= 0.0 && b > 0.0;
        let down = a >= 0.0 && b < 0.0;
        match self {
            Edge::None => false,
            Edge::Rising => up,
            Edge::Falling => down,
            Edge::Either => up || down,
        }
    }
}

pub fn edge_hit(kind: &str, a: f64, b: f64) -> bool {
    Edge::parse(kind).hit(a, b)
}

#[derive(Clone, Copy, PartialEq)]
enum Method {
    Forward,
    Backward,
    Trapezoidal,
}

/// Discrete integrator (accumulator) with limits and an optional reset input.
#[derive(Clone)]
struct DInt {
    k: f64,
    ts: f64,
    m: Method,
    ic: Vec<f64>,
    hi: f64,
    lo: f64,
    w: usize,
    ins: usize,
    reset: Edge,
    x: Vec<f64>,
    held: Vec<f64>,
    last_r: f64,
}
impl DInt {
    fn u(&self, u: &[f64], j: usize) -> f64 {
        if self.ins == 1 { u[0] } else { u[j] }
    }
    fn ic(&self, j: usize) -> f64 {
        if self.ic.len() == 1 { self.ic[0] } else { self.ic[j] }
    }
    fn reset_now(&self, u: &[f64]) -> bool {
        self.reset != Edge::None && self.reset.hit(self.last_r, u[self.ins])
    }
    fn y(&self, u: &[f64], j: usize, reset: bool) -> f64 {
        let x = if reset { self.ic(j) } else { self.x[j] };
        let v = match self.m {
            Method::Forward => x,
            Method::Backward => x + self.k * self.ts * self.u(u, j),
            Method::Trapezoidal => x + 0.5 * self.k * self.ts * self.u(u, j),
        };
        v.clamp(self.lo, self.hi)
    }
}
impl Rt for DInt {
    fn rate(&self) -> Rate {
        Rate::Discrete(self.ts)
    }
    fn init(&mut self, _: &mut [f64]) {
        self.x = (0..self.w).map(|j| self.ic(j).clamp(self.lo, self.hi)).collect();
        self.held.clone_from(&self.x);
        self.last_r = f64::NAN;
    }
    fn feed(&self) -> bool {
        self.m != Method::Forward || self.reset != Edge::None
    }
    fn out(&self, c: &Ctx, _: &[f64], u: &[f64], y: &mut [f64]) {
        if c.major && is_hit(c.t, self.ts) {
            let r = self.reset_now(u);
            for (j, yj) in y.iter_mut().enumerate() {
                *yj = self.y(u, j, r);
            }
        } else {
            y.copy_from_slice(&self.held);
        }
    }
    fn update(&mut self, c: &Ctx, _: &mut [f64], u: &[f64]) {
        if !is_hit(c.t, self.ts) {
            return;
        }
        let r = self.reset_now(u);
        for j in 0..self.w {
            let y = self.y(u, j, r);
            let x = if r { self.ic(j) } else { self.x[j] };
            self.held[j] = y;
            self.x[j] = match self.m {
                Method::Forward => x + self.k * self.ts * self.u(u, j),
                Method::Backward => y,
                Method::Trapezoidal => y + 0.5 * self.k * self.ts * self.u(u, j),
            }
            .clamp(self.lo, self.hi);
        }
        if self.reset != Edge::None {
            self.last_r = u[self.ins];
        }
    }
    fn next_hit(&self, t: f64) -> f64 {
        next_sample(t, self.ts)
    }
    cb!();
}

#[derive(Clone, Copy, PartialEq)]
enum Aw {
    Clamping,
    Back,
    None,
}

/// Discrete parallel PID: P·e + I·Ts·Σe + filtered D (backward Euler), output limits and
/// anti-windup by clamping or back-calculation.
#[derive(Clone)]
struct DPid {
    p: f64,
    i: f64,
    d: f64,
    n: f64,
    ts: f64,
    hi: f64,
    lo: f64,
    aw: Aw,
    kb: f64,
    ic: f64,
    xi: f64,
    df: f64,
    e1: f64,
    held: f64,
}
impl DPid {
    /// (unlimited output, filtered derivative) at error e.
    fn v(&self, e: f64) -> (f64, f64) {
        let e1 = if self.e1.is_finite() { self.e1 } else { e };
        let df = (self.df + self.d * self.n * (e - e1)) / (1.0 + self.n * self.ts);
        (self.p * e + self.xi + df, df)
    }
}
impl Rt for DPid {
    fn rate(&self) -> Rate {
        Rate::Discrete(self.ts)
    }
    fn init(&mut self, _: &mut [f64]) {
        self.xi = self.ic;
        self.df = 0.0;
        self.e1 = f64::NAN;
        self.held = self.ic.clamp(self.lo, self.hi);
    }
    fn feed(&self) -> bool {
        self.p != 0.0 || self.d != 0.0
    }
    fn out(&self, c: &Ctx, _: &[f64], u: &[f64], y: &mut [f64]) {
        y[0] = if c.major && is_hit(c.t, self.ts) { self.v(u[0]).0.clamp(self.lo, self.hi) } else { self.held };
    }
    fn update(&mut self, c: &Ctx, _: &mut [f64], u: &[f64]) {
        if !is_hit(c.t, self.ts) {
            return;
        }
        let e = u[0];
        let (v, df) = self.v(e);
        let y = v.clamp(self.lo, self.hi);
        self.held = y;
        self.df = df;
        self.e1 = e;
        match self.aw {
            Aw::None => self.xi += self.i * self.ts * e,
            // Stop integrating while the output is limited and the error would drive it further.
            Aw::Clamping => {
                if y == v || e * v < 0.0 {
                    self.xi += self.i * self.ts * e;
                }
            }
            Aw::Back => self.xi += self.ts * (self.i * e + self.kb * (y - v)),
        }
    }
    fn next_hit(&self, t: f64) -> f64 {
        next_sample(t, self.ts)
    }
    cb!();
}

// ---------------------------------------------------------------- sources

/// Square, triangle and repeating sequence: periodic, with every corner hit exactly.
#[derive(Clone)]
enum Wave {
    Square { duty: f64 },
    Triangle,
    /// Times (from 0, increasing; the last is the period) and values, linear between.
    Seq { t: Vec<f64>, v: Vec<f64> },
}
#[derive(Clone)]
struct Periodic {
    wave: Wave,
    amp: f64,
    per: f64,
    bias: f64,
}
impl Periodic {
    fn corners(&self) -> Vec<f64> {
        match &self.wave {
            Wave::Square { duty } => vec![0.0, duty * self.per],
            Wave::Triangle => vec![0.0, 0.25 * self.per, 0.75 * self.per],
            Wave::Seq { t, .. } => t.clone(),
        }
    }
}
impl Rt for Periodic {
    fn rate(&self) -> Rate {
        Rate::Continuous
    }
    fn feed(&self) -> bool {
        false
    }
    fn out(&self, c: &Ctx, _: &[f64], _: &[f64], y: &mut [f64]) {
        // The square wave jumps: inside a step it uses the step's start time, as Pulse does.
        let t = if matches!(self.wave, Wave::Square { .. }) { c.tq() } else { c.t };
        let ph = t.rem_euclid(self.per);
        y[0] = match &self.wave {
            Wave::Square { duty } => self.bias + if ph < duty * self.per { self.amp } else { -self.amp },
            Wave::Triangle => {
                let f = ph / self.per;
                let tri = if f < 0.25 { 4.0 * f } else if f < 0.75 { 2.0 - 4.0 * f } else { 4.0 * f - 4.0 };
                self.bias + self.amp * tri
            }
            Wave::Seq { t, v } => {
                let k = t.partition_point(|x| *x <= ph).clamp(1, t.len() - 1);
                let (t0, t1) = (t[k - 1], t[k]);
                let s = if t1 > t0 { (ph - t0) / (t1 - t0) } else { 1.0 };
                v[k - 1] + s * (v[k] - v[k - 1])
            }
        };
    }
    fn next_hit(&self, t: f64) -> f64 {
        let base = (t / self.per).floor() * self.per;
        let eps = 1e-12 * self.per.max(t.abs());
        for k in 0..3 {
            for c in self.corners() {
                let h = base + k as f64 * self.per + c;
                if h > t + eps {
                    return h;
                }
            }
        }
        f64::INFINITY
    }
    cb!();
}

/// Linear frequency sweep from f0 to f1 over T seconds, then f1.
#[derive(Clone)]
struct Chirp {
    amp: f64,
    f0: f64,
    f1: f64,
    tt: f64,
}
impl Rt for Chirp {
    fn rate(&self) -> Rate {
        Rate::Continuous
    }
    fn feed(&self) -> bool {
        false
    }
    fn out(&self, c: &Ctx, _: &[f64], _: &[f64], y: &mut [f64]) {
        let t = c.t.max(0.0);
        let k = (self.f1 - self.f0) / self.tt;
        let ph = if t <= self.tt { self.f0 * t + 0.5 * k * t * t } else { self.f0 * self.tt + 0.5 * k * self.tt * self.tt + self.f1 * (t - self.tt) };
        y[0] = self.amp * (2.0 * PI * ph).sin();
    }
    cb!();
}

// ---------------------------------------------------------------- build

pub const KINDS: &[&str] = &["DiscreteTF", "DiscreteSS", "DiscreteIntegrator", "DiscretePID", "Square", "Triangle", "Chirp", "RepSeq"];

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
    let w0 = iw.first().cloned().unwrap_or(1);
    match kind {
        "DiscreteTF" => {
            if w0 != 1 {
                return Err(format!("the discrete transfer function takes a scalar input; it has width {w0}"));
            }
            let l = tf2ss(&p.v("num")?, &p.v("den")?).map_err(|e| e.replace("physically realisable", "causal: it would need future inputs"))?;
            let n = l.a.r;
            ok(DLin { a: l.a, b: l.b, c: l.c, d: l.d, x0: vec![0.0; n], x: vec![], held: vec![], ts: p.pos("ts")? }, vec![1])
        }
        "DiscreteSS" => {
            let (a, b, c) = (p.mat("A")?, p.mat("B")?, p.mat("C")?);
            let mut d = p.mat("D")?;
            let n = a.r;
            if a.c != n || b.r != n || c.c != n {
                return Err(format!("sizes do not fit: A {}×{}, B {}×{}, C {}×{} (A must be n×n, B n×m, C p×n)", a.r, a.c, b.r, b.c, c.r, c.c));
            }
            if w0 != b.c {
                return Err(format!("B has {} columns but the input has width {w0}", b.c));
            }
            if d.is_scalar() && d.d[0] == 0.0 {
                d = Mat { r: c.r, c: b.c, d: vec![0.0; c.r * b.c] };
            }
            if d.r != c.r || d.c != b.c {
                return Err(format!("D must be {}×{}", c.r, b.c));
            }
            let x0 = p.v("x0")?;
            let x0 = if x0.len() == 1 { vec![x0[0]; n] } else { x0 };
            if x0.len() != n {
                return Err(format!("the initial state needs {n} values"));
            }
            let r = c.r;
            ok(DLin { a, b, c, d, x0, x: vec![], held: vec![], ts: p.pos("ts")? }, vec![r])
        }
        "DiscreteIntegrator" => {
            let ic = p.v("ic")?;
            let (hi, lo) = (p.s("upper")?, p.s("lower")?);
            if lo > hi {
                return Err("the lower limit is above the upper limit".into());
            }
            let w = if ic.len() > 1 { ic.len() } else { w0 };
            if w0 != 1 && w0 != w {
                return Err(format!("the initial condition has {} elements but the input has width {w0}", ic.len()));
            }
            let reset = Edge::parse(p.txt("reset"));
            if reset != Edge::None && iw.get(1).cloned().unwrap_or(1) != 1 {
                return Err("the reset input must be a scalar".into());
            }
            let m = match p.txt("method") {
                "backward Euler" => Method::Backward,
                "trapezoidal" => Method::Trapezoidal,
                _ => Method::Forward,
            };
            ok(DInt { k: p.s("k")?, ts: p.pos("ts")?, m, ic, hi, lo, w, ins: w0, reset, x: vec![], held: vec![0.0; w], last_r: f64::NAN }, vec![w])
        }
        "DiscretePID" => {
            if w0 != 1 {
                return Err(format!("the discrete PID takes a scalar error; it has width {w0}"));
            }
            let (hi, lo) = (p.s("upper")?, p.s("lower")?);
            if lo > hi {
                return Err("the lower limit is above the upper limit".into());
            }
            let n = p.s("N")?;
            if n < 0.0 {
                return Err("the filter N cannot be negative".into());
            }
            let aw = match p.txt("aw") {
                "back-calculation" => Aw::Back,
                "none" => Aw::None,
                _ => Aw::Clamping,
            };
            let ic = p.s("ic")?;
            ok(DPid { p: p.s("P")?, i: p.s("I")?, d: p.s("D")?, n, ts: p.pos("ts")?, hi, lo, aw, kb: p.s("kb")?, ic, xi: ic, df: 0.0, e1: f64::NAN, held: 0.0 }, vec![1])
        }
        "Square" | "Triangle" => {
            let f = p.pos("freq")?;
            let wave = if kind == "Square" {
                let duty = p.s("duty")?;
                if !(0.0..=100.0).contains(&duty) {
                    return Err("the duty cycle must be between 0 and 100 %".into());
                }
                Wave::Square { duty: duty / 100.0 }
            } else {
                Wave::Triangle
            };
            ok(Periodic { wave, amp: p.s("amp")?, per: 1.0 / f, bias: p.s("bias")? }, vec![1])
        }
        "RepSeq" => {
            let (t, v) = (p.v("times")?, p.v("values")?);
            if t.len() < 2 || t.len() != v.len() {
                return Err("Times and Values need the same number of points, at least two".into());
            }
            if t[0] != 0.0 || t.windows(2).any(|w| w[1] < w[0]) || t[t.len() - 1] <= 0.0 {
                return Err("Times must start at 0 and increase; the last time is the period".into());
            }
            let per = t[t.len() - 1];
            ok(Periodic { wave: Wave::Seq { t, v }, amp: 1.0, per, bias: 0.0 }, vec![1])
        }
        "Chirp" => {
            let (f0, f1) = (p.s("f0")?, p.s("f1")?);
            if f0 < 0.0 || f1 < 0.0 {
                return Err("frequencies cannot be negative".into());
            }
            ok(Chirp { amp: p.s("amp")?, f0, f1, tt: p.pos("T")? }, vec![1])
        }
        _ => Err(format!("unknown block '{kind}'")),
    }
}
