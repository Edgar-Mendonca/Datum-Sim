//! How each block runs. A block reads its inputs (packed one after another), writes its outputs,
//! and may own continuous states (derivatives), discrete updates at major steps, times that the
//! solver must hit exactly, and zero-crossing surfaces that locate switching.

use crate::expr::{self, Mat};
use std::collections::{BTreeMap, VecDeque};
use std::path::Path;

/// Where the solver is: `t` is the evaluation time, `tseg` the start of the current step
/// (piecewise-constant sources use it, so a step never straddles a jump), `major` at accepted steps.
#[derive(Clone, Copy, Debug)]
pub struct Ctx {
    pub t: f64,
    pub tseg: f64,
    pub major: bool,
}

impl Ctx {
    fn tq(&self) -> f64 {
        if self.major {
            self.t
        } else {
            self.tseg
        }
    }
}

pub trait Rt: Send + Sync {
    fn nx(&self) -> usize {
        0
    }
    fn init(&mut self, _x: &mut [f64]) {}
    /// Does any output depend directly on the inputs (now)? If not, the block breaks loops.
    fn feed(&self) -> bool {
        true
    }
    fn out(&self, c: &Ctx, x: &[f64], u: &[f64], y: &mut [f64]);
    fn deriv(&self, _c: &Ctx, _x: &[f64], _u: &[f64], _dx: &mut [f64]) {}
    /// At major steps, after outputs are logged: discrete states, modes, histories.
    fn update(&mut self, _c: &Ctx, _x: &mut [f64], _u: &[f64]) {}
    /// The next time after `t` that a step must land on exactly.
    fn next_hit(&self, _t: f64) -> f64 {
        f64::INFINITY
    }
    fn nzc(&self) -> usize {
        0
    }
    fn zc(&self, _c: &Ctx, _x: &[f64], _u: &[f64], _z: &mut [f64]) {}
    fn clone_box(&self) -> Box<dyn Rt>;
}

/// Evaluated parameters of one block.
pub struct Pv<'a> {
    pub m: BTreeMap<String, Mat>,
    pub t: BTreeMap<String, String>,
    pub vars: &'a BTreeMap<String, Mat>,
    pub base: Option<&'a Path>,
}

impl Pv<'_> {
    fn mat(&self, k: &str) -> Result<Mat, String> {
        self.m.get(k).cloned().ok_or_else(|| format!("missing parameter '{k}'"))
    }
    fn s(&self, k: &str) -> Result<f64, String> {
        let v = self.mat(k)?;
        v.s().map_err(|e| format!("{k}: {e}"))
    }
    fn v(&self, k: &str) -> Result<Vec<f64>, String> {
        let m = self.mat(k)?;
        if m.r > 1 && m.c > 1 {
            return Err(format!("{k}: expected a vector, got a {}×{} matrix", m.r, m.c));
        }
        Ok(m.d)
    }
    fn txt(&self, k: &str) -> &str {
        self.t.get(k).map_or("", |s| s.as_str())
    }
    fn pos(&self, k: &str) -> Result<f64, String> {
        let v = self.s(k)?;
        if v > 0.0 && v.is_finite() {
            Ok(v)
        } else {
            Err(format!("{k} must be a positive number"))
        }
    }
}

macro_rules! cb {
    () => {
        fn clone_box(&self) -> Box<dyn Rt> {
            Box::new(self.clone())
        }
    };
}

/// Element `j` of input `i` with width-1 inputs broadcast.
#[derive(Clone, Debug)]
struct Ins {
    off: Vec<usize>,
    w: Vec<usize>,
}
impl Ins {
    fn new(w: &[usize]) -> Ins {
        let mut off = Vec::new();
        let mut o = 0;
        for x in w {
            off.push(o);
            o += x;
        }
        Ins { off, w: w.to_vec() }
    }
    #[inline]
    fn g(&self, u: &[f64], i: usize, j: usize) -> f64 {
        if self.w[i] == 1 {
            u[self.off[i]]
        } else {
            u[self.off[i] + j]
        }
    }
}

/// The common width of elementwise inputs: all equal, or 1 (broadcast).
fn ewidth(kind: &str, w: &[usize]) -> Result<usize, String> {
    let m = w.iter().cloned().max().unwrap_or(1).max(1);
    if w.iter().any(|x| *x != 1 && *x != m) {
        return Err(format!("{kind}: input widths {w:?} do not match (each must be {m} or 1)"));
    }
    Ok(m)
}

// ---------------------------------------------------------------- sources

#[derive(Clone)]
struct Constant(Vec<f64>);
impl Rt for Constant {
    fn out(&self, _: &Ctx, _: &[f64], _: &[f64], y: &mut [f64]) {
        y.copy_from_slice(&self.0);
    }
    fn feed(&self) -> bool {
        false
    }
    cb!();
}

#[derive(Clone)]
struct StepS {
    time: f64,
    a: f64,
    b: f64,
}
impl Rt for StepS {
    fn out(&self, c: &Ctx, _: &[f64], _: &[f64], y: &mut [f64]) {
        y[0] = if c.tq() >= self.time { self.b } else { self.a };
    }
    fn feed(&self) -> bool {
        false
    }
    fn next_hit(&self, t: f64) -> f64 {
        if t < self.time {
            self.time
        } else {
            f64::INFINITY
        }
    }
    cb!();
}

#[derive(Clone)]
struct Ramp {
    slope: f64,
    start: f64,
    y0: f64,
}
impl Rt for Ramp {
    fn out(&self, c: &Ctx, _: &[f64], _: &[f64], y: &mut [f64]) {
        y[0] = self.y0 + if c.t > self.start { self.slope * (c.t - self.start) } else { 0.0 };
    }
    fn feed(&self) -> bool {
        false
    }
    fn next_hit(&self, t: f64) -> f64 {
        if t < self.start {
            self.start
        } else {
            f64::INFINITY
        }
    }
    cb!();
}

#[derive(Clone)]
struct Sine {
    a: f64,
    w: f64,
    ph: f64,
    b: f64,
}
impl Rt for Sine {
    fn out(&self, c: &Ctx, _: &[f64], _: &[f64], y: &mut [f64]) {
        y[0] = self.a * (self.w * c.t + self.ph).sin() + self.b;
    }
    fn feed(&self) -> bool {
        false
    }
    cb!();
}

#[derive(Clone)]
struct Pulse {
    a: f64,
    per: f64,
    on: f64,
    delay: f64,
}
impl Rt for Pulse {
    fn out(&self, c: &Ctx, _: &[f64], _: &[f64], y: &mut [f64]) {
        let t = c.tq() - self.delay;
        y[0] = if t >= 0.0 && t.rem_euclid(self.per) < self.on { self.a } else { 0.0 };
    }
    fn feed(&self) -> bool {
        false
    }
    fn next_hit(&self, t: f64) -> f64 {
        if t < self.delay {
            return self.delay;
        }
        let k = ((t - self.delay) / self.per).floor();
        let base = self.delay + k * self.per;
        // A tiny tolerance so a hit just reached is not returned again.
        let eps = 1e-12 * self.per.max(t.abs());
        [base + self.on, base + self.per, base + self.per + self.on].into_iter().find(|h| *h > t + eps).unwrap_or(f64::INFINITY)
    }
    cb!();
}

#[derive(Clone)]
struct Clock;
impl Rt for Clock {
    fn out(&self, c: &Ctx, _: &[f64], _: &[f64], y: &mut [f64]) {
        y[0] = c.t;
    }
    fn feed(&self) -> bool {
        false
    }
    cb!();
}

/// A table of times and value rows; linear or held. Used by From table and From file.
#[derive(Clone)]
struct Table {
    t: Vec<f64>,
    v: Vec<Vec<f64>>,
    hold: bool,
}
impl Table {
    fn at(&self, t: f64, y: &mut [f64]) {
        let n = self.t.len();
        let i = self.t.partition_point(|x| *x <= t);
        if i == 0 {
            y.copy_from_slice(&self.v[0]);
        } else if i >= n {
            y.copy_from_slice(&self.v[n - 1]);
        } else if self.hold {
            y.copy_from_slice(&self.v[i - 1]);
        } else {
            let (t0, t1) = (self.t[i - 1], self.t[i]);
            let f = if t1 > t0 { (t - t0) / (t1 - t0) } else { 1.0 };
            for (j, yj) in y.iter_mut().enumerate() {
                *yj = self.v[i - 1][j] + f * (self.v[i][j] - self.v[i - 1][j]);
            }
        }
    }
}
impl Rt for Table {
    fn out(&self, c: &Ctx, _: &[f64], _: &[f64], y: &mut [f64]) {
        self.at(if self.hold { c.tq() } else { c.t }, y);
    }
    fn feed(&self) -> bool {
        false
    }
    fn next_hit(&self, t: f64) -> f64 {
        if self.t.len() > 10_000 {
            return f64::INFINITY;
        }
        let i = self.t.partition_point(|x| *x <= t);
        self.t.get(i).cloned().unwrap_or(f64::INFINITY)
    }
    cb!();
}

fn read_csv(path: &Path) -> Result<Table, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let mut t = Vec::new();
    let mut v = Vec::new();
    for line in text.lines() {
        let cells: Vec<&str> = line.split([',', ';', '\t']).map(|c| c.trim()).collect();
        let nums: Option<Vec<f64>> = cells.iter().map(|c| c.parse::<f64>().ok()).collect();
        match nums {
            Some(row) if row.len() >= 2 => {
                if let Some(w) = v.first().map(|r: &Vec<f64>| r.len()) {
                    if row.len() - 1 != w {
                        return Err(format!("{}: rows have different numbers of columns", path.display()));
                    }
                }
                if t.last().is_some_and(|l| row[0] < *l) {
                    return Err(format!("{}: times must increase", path.display()));
                }
                t.push(row[0]);
                v.push(row[1..].to_vec());
            }
            _ if t.is_empty() => continue, // header
            _ => {}
        }
    }
    if t.is_empty() {
        return Err(format!("{}: no rows of numbers (time, value, …)", path.display()));
    }
    Ok(Table { t, v, hold: false })
}

#[derive(Clone)]
struct Noise {
    mean: f64,
    sd: f64,
    ts: f64,
    seed: u64,
    state: u64,
    val: f64,
}
impl Noise {
    fn next(&mut self) -> f64 {
        let mut u = || {
            self.state ^= self.state << 13;
            self.state ^= self.state >> 7;
            self.state ^= self.state << 17;
            ((self.state >> 11) as f64 + 0.5) / (1u64 << 53) as f64
        };
        let (a, b) = (u(), u());
        self.mean + self.sd * (-2.0 * a.ln()).sqrt() * (2.0 * std::f64::consts::PI * b).cos()
    }
}
impl Rt for Noise {
    fn init(&mut self, _: &mut [f64]) {
        self.state = self.seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
        self.val = self.next();
    }
    fn out(&self, _: &Ctx, _: &[f64], _: &[f64], y: &mut [f64]) {
        y[0] = self.val;
    }
    fn feed(&self) -> bool {
        false
    }
    fn update(&mut self, c: &Ctx, _: &mut [f64], _: &[f64]) {
        if is_hit(c.t, self.ts) && c.t > 0.0 {
            self.val = self.next();
        }
    }
    fn next_hit(&self, t: f64) -> f64 {
        next_sample(t, self.ts)
    }
    cb!();
}

fn is_hit(t: f64, ts: f64) -> bool {
    let k = (t / ts).round();
    (t - k * ts).abs() <= 1e-9 * ts.max(t.abs() * 1e-3)
}

fn next_sample(t: f64, ts: f64) -> f64 {
    let k = (t / ts).floor() + 1.0;
    let mut h = k * ts;
    if h <= t + 1e-12 * ts.max(t.abs()) {
        h += ts;
    }
    h
}

// ---------------------------------------------------------------- math

#[derive(Clone)]
struct Gain {
    k: Mat,
    ins: usize,
    mat: bool,
}
impl Rt for Gain {
    fn out(&self, _: &Ctx, _: &[f64], u: &[f64], y: &mut [f64]) {
        if self.mat {
            for (i, yi) in y.iter_mut().enumerate() {
                *yi = (0..self.k.c).map(|j| self.k.at(i, j) * u[j]).sum();
            }
        } else {
            let k = &self.k.d;
            for (j, yj) in y.iter_mut().enumerate() {
                let kj = if k.len() == 1 { k[0] } else { k[j] };
                *yj = kj * if self.ins == 1 { u[0] } else { u[j] };
            }
        }
    }
    cb!();
}

#[derive(Clone)]
struct Ew {
    ins: Ins,
    signs: Vec<f64>,
    op: u8, // b's' sum, b'p' product
}
impl Rt for Ew {
    fn out(&self, _: &Ctx, _: &[f64], u: &[f64], y: &mut [f64]) {
        for (j, yj) in y.iter_mut().enumerate() {
            let mut acc = if self.op == b's' { 0.0 } else { 1.0 };
            for (i, s) in self.signs.iter().enumerate() {
                let v = self.ins.g(u, i, j);
                match (self.op, *s > 0.0) {
                    (b's', true) => acc += v,
                    (b's', false) => acc -= v,
                    (_, true) => acc *= v,
                    _ => acc /= v,
                }
            }
            *yj = acc;
        }
    }
    cb!();
}

#[derive(Clone)]
struct Map1(fn(f64, &[f64]) -> f64, Vec<f64>);
impl Rt for Map1 {
    fn out(&self, _: &Ctx, _: &[f64], u: &[f64], y: &mut [f64]) {
        for (yj, uj) in y.iter_mut().zip(u) {
            *yj = (self.0)(*uj, &self.1);
        }
    }
    cb!();
}

#[derive(Clone)]
struct MinMax {
    ins: Ins,
    max: bool,
}
impl Rt for MinMax {
    fn out(&self, _: &Ctx, _: &[f64], u: &[f64], y: &mut [f64]) {
        for (j, yj) in y.iter_mut().enumerate() {
            let it = (0..self.ins.w.len()).map(|i| self.ins.g(u, i, j));
            *yj = if self.max { it.fold(f64::NEG_INFINITY, f64::max) } else { it.fold(f64::INFINITY, f64::min) };
        }
    }
    cb!();
}

#[derive(Clone)]
struct Fcn {
    e: expr::Node,
    n: usize,
}
impl Rt for Fcn {
    fn out(&self, c: &Ctx, _: &[f64], u: &[f64], y: &mut [f64]) {
        let mut s = [0.0f64; 34];
        s[0] = c.t;
        s[1..=self.n].copy_from_slice(&u[..self.n]);
        s[self.n + 1] = u[0];
        y[0] = self.e.eval_s(&s[..self.n + 2]);
    }
    cb!();
}

// ---------------------------------------------------------------- continuous

#[derive(Clone)]
struct Integrator {
    ic: Vec<f64>,
    hi: f64,
    lo: f64,
    ins: usize,
}
impl Rt for Integrator {
    fn nx(&self) -> usize {
        self.ic.len()
    }
    fn init(&mut self, x: &mut [f64]) {
        for (xi, v) in x.iter_mut().zip(&self.ic) {
            *xi = v.clamp(self.lo, self.hi);
        }
    }
    fn feed(&self) -> bool {
        false
    }
    fn out(&self, _: &Ctx, x: &[f64], _: &[f64], y: &mut [f64]) {
        for (yj, xj) in y.iter_mut().zip(x) {
            *yj = xj.clamp(self.lo, self.hi);
        }
    }
    fn deriv(&self, _: &Ctx, x: &[f64], u: &[f64], dx: &mut [f64]) {
        for (j, d) in dx.iter_mut().enumerate() {
            let v = if self.ins == 1 { u[0] } else { u[j] };
            *d = if (x[j] >= self.hi && v > 0.0) || (x[j] <= self.lo && v < 0.0) { 0.0 } else { v };
        }
    }
    fn update(&mut self, _: &Ctx, x: &mut [f64], _: &[f64]) {
        for xj in x.iter_mut() {
            *xj = xj.clamp(self.lo, self.hi);
        }
    }
    fn nzc(&self) -> usize {
        if self.hi.is_finite() || self.lo.is_finite() {
            2 * self.ic.len()
        } else {
            0
        }
    }
    fn zc(&self, _: &Ctx, x: &[f64], _: &[f64], z: &mut [f64]) {
        for (j, xj) in x.iter().enumerate() {
            z[2 * j] = if self.hi.is_finite() { xj - self.hi } else { -1.0 };
            z[2 * j + 1] = if self.lo.is_finite() { xj - self.lo } else { 1.0 };
        }
    }
    cb!();
}

#[derive(Clone)]
struct Derivative {
    last: Option<(f64, Vec<f64>)>,
    prev_y: Vec<f64>,
}
impl Rt for Derivative {
    fn out(&self, c: &Ctx, _: &[f64], u: &[f64], y: &mut [f64]) {
        match &self.last {
            Some((tl, ul)) if c.t > *tl => {
                for j in 0..y.len() {
                    y[j] = (u[j] - ul[j]) / (c.t - tl);
                }
            }
            _ => y.copy_from_slice(&self.prev_y),
        }
    }
    fn update(&mut self, c: &Ctx, _: &mut [f64], u: &[f64]) {
        let mut y = vec![0.0; u.len()];
        self.out(c, &[], u, &mut y);
        if self.last.as_ref().is_none_or(|(tl, _)| c.t > *tl) {
            self.prev_y = y;
            self.last = Some((c.t, u.to_vec()));
        }
    }
    cb!();
}

/// A linear state-space system (TF, ZPK and State space all become one).
#[derive(Clone)]
pub struct Lin {
    pub a: Mat,
    pub b: Mat,
    pub c: Mat,
    pub d: Mat,
    pub x0: Vec<f64>,
}
impl Rt for Lin {
    fn nx(&self) -> usize {
        self.a.r
    }
    fn init(&mut self, x: &mut [f64]) {
        x.copy_from_slice(&self.x0);
    }
    fn feed(&self) -> bool {
        self.d.d.iter().any(|v| *v != 0.0)
    }
    fn out(&self, _: &Ctx, x: &[f64], u: &[f64], y: &mut [f64]) {
        for (i, yi) in y.iter_mut().enumerate() {
            let mut s = 0.0;
            for (j, xj) in x.iter().enumerate() {
                s += self.c.at(i, j) * xj;
            }
            for (j, uj) in u.iter().enumerate() {
                s += self.d.at(i, j) * uj;
            }
            *yi = s;
        }
    }
    fn deriv(&self, _: &Ctx, x: &[f64], u: &[f64], dx: &mut [f64]) {
        for (i, d) in dx.iter_mut().enumerate() {
            let mut s = 0.0;
            for (j, xj) in x.iter().enumerate() {
                s += self.a.at(i, j) * xj;
            }
            for (j, uj) in u.iter().enumerate() {
                s += self.b.at(i, j) * uj;
            }
            *d = s;
        }
    }
    cb!();
}

/// Numerator and denominator (highest power first) → controllable canonical form.
pub fn tf2ss(num: &[f64], den: &[f64]) -> Result<Lin, String> {
    let den: Vec<f64> = den.iter().cloned().skip_while(|v| *v == 0.0).collect();
    let num: Vec<f64> = num.iter().cloned().skip_while(|v| *v == 0.0).collect();
    if den.is_empty() {
        return Err("the denominator is zero".into());
    }
    let n = den.len() - 1;
    if num.len() > den.len() {
        return Err("the numerator degree is higher than the denominator's (not physically realisable)".into());
    }
    if n > 200 {
        return Err("the denominator degree is too high".into());
    }
    let a0 = den[0];
    let a: Vec<f64> = den.iter().map(|v| v / a0).collect();
    let mut b = vec![0.0; n + 1 - num.len()];
    b.extend(num.iter().map(|v| v / a0));
    let d0 = b[0];
    let mut am = vec![0.0; n * n];
    for j in 0..n {
        am[j] = -a[j + 1];
    }
    for i in 1..n {
        am[i * n + i - 1] = 1.0;
    }
    let mut bm = vec![0.0; n];
    if n > 0 {
        bm[0] = 1.0;
    }
    let cm: Vec<f64> = (0..n).map(|j| b[j + 1] - a[j + 1] * d0).collect();
    Ok(Lin {
        a: Mat { r: n, c: n, d: am },
        b: Mat { r: n, c: 1, d: bm },
        c: Mat { r: 1, c: n, d: cm },
        d: Mat::scalar(d0),
        x0: vec![0.0; n],
    })
}

/// "-1+2j -1-2j, 3" → complex numbers.
pub fn parse_roots(s: &str) -> Result<Vec<(f64, f64)>, String> {
    let s = s.replace(['[', ']'], " ");
    let mut out = Vec::new();
    for tok in s.split([' ', ',', ';', '\t']).filter(|x| !x.is_empty()) {
        let tok = tok.replace('i', "j");
        let v = if let Some(body) = tok.strip_suffix('j') {
            // split a+bj at the last sign that is not at the start or after an exponent
            let bytes = body.as_bytes();
            let k = (1..bytes.len()).rev().find(|&i| (bytes[i] == b'+' || bytes[i] == b'-') && !matches!(bytes[i - 1], b'e' | b'E'));
            let (re, im) = match k {
                Some(k) => (&body[..k], &body[k..]),
                None => ("0", body),
            };
            let im = match im {
                "" | "+" => "1",
                "-" => "-1",
                x => x,
            };
            (re.parse::<f64>().map_err(|_| format!("bad number '{tok}'"))?, im.parse::<f64>().map_err(|_| format!("bad number '{tok}'"))?)
        } else {
            (tok.parse::<f64>().map_err(|_| format!("bad number '{tok}'"))?, 0.0)
        };
        out.push(v);
    }
    Ok(out)
}

/// Polynomial with these roots (highest power first); complex roots must come in pairs.
pub fn poly(roots: &[(f64, f64)]) -> Result<Vec<f64>, String> {
    let mut c: Vec<(f64, f64)> = vec![(1.0, 0.0)];
    for &(re, im) in roots {
        let mut n = vec![(0.0, 0.0); c.len() + 1];
        for (i, &(a, b)) in c.iter().enumerate() {
            n[i].0 += a;
            n[i].1 += b;
            // − root · coefficient
            n[i + 1].0 -= a * re - b * im;
            n[i + 1].1 -= a * im + b * re;
        }
        c = n;
    }
    let scale = c.iter().map(|x| x.0.abs()).fold(1.0, f64::max);
    if c.iter().any(|x| x.1.abs() > 1e-9 * scale) {
        return Err("complex values must come in conjugate pairs (a+bj and a-bj)".into());
    }
    Ok(c.into_iter().map(|x| x.0).collect())
}

#[derive(Clone)]
struct Pid {
    p: f64,
    i: f64,
    d: f64,
    n: f64,
    hi: f64,
    lo: f64,
    ic: f64,
}
impl Pid {
    fn raw(&self, x: &[f64], e: f64) -> f64 {
        self.p * e + self.i * x[0] + if self.d != 0.0 { self.d * self.n * (e - x[1]) } else { 0.0 }
    }
}
impl Rt for Pid {
    fn nx(&self) -> usize {
        2
    }
    fn init(&mut self, x: &mut [f64]) {
        x[0] = self.ic;
        x[1] = 0.0;
    }
    fn feed(&self) -> bool {
        self.p != 0.0 || self.d != 0.0
    }
    fn out(&self, _: &Ctx, x: &[f64], u: &[f64], y: &mut [f64]) {
        y[0] = self.raw(x, u[0]).clamp(self.lo, self.hi);
    }
    fn deriv(&self, _: &Ctx, x: &[f64], u: &[f64], dx: &mut [f64]) {
        let e = u[0];
        let r = self.raw(x, e);
        // Clamping anti-windup: stop integrating while the output is limited and e pushes further.
        let wind = (r > self.hi && e * self.i > 0.0) || (r < self.lo && e * self.i < 0.0);
        dx[0] = if wind { 0.0 } else { e };
        dx[1] = if self.d != 0.0 { self.n * (e - x[1]) } else { 0.0 };
    }
    cb!();
}

#[derive(Clone)]
struct Delay {
    delay: f64,
    y0: Vec<f64>,
    hist: VecDeque<(f64, Vec<f64>)>,
}
impl Rt for Delay {
    fn feed(&self) -> bool {
        self.delay <= 0.0
    }
    fn init(&mut self, _: &mut [f64]) {
        self.hist.clear();
    }
    fn out(&self, c: &Ctx, _: &[f64], u: &[f64], y: &mut [f64]) {
        if self.delay <= 0.0 {
            y.copy_from_slice(u);
            return;
        }
        let tt = c.t - self.delay;
        let h = &self.hist;
        if h.is_empty() || tt < h[0].0 {
            for (j, yj) in y.iter_mut().enumerate() {
                *yj = if self.y0.len() == 1 { self.y0[0] } else { self.y0[j] };
            }
            return;
        }
        let i = h.partition_point(|e| e.0 <= tt);
        if i >= h.len() {
            y.copy_from_slice(&h[h.len() - 1].1);
            return;
        }
        let (a, b) = (&h[i - 1], &h[i]);
        let f = if b.0 > a.0 { (tt - a.0) / (b.0 - a.0) } else { 1.0 };
        for (j, yj) in y.iter_mut().enumerate() {
            *yj = a.1[j] + f * (b.1[j] - a.1[j]);
        }
    }
    fn update(&mut self, c: &Ctx, _: &mut [f64], u: &[f64]) {
        if self.hist.back().is_some_and(|e| e.0 >= c.t) {
            self.hist.pop_back();
        }
        self.hist.push_back((c.t, u.to_vec()));
        while self.hist.len() > 2 && self.hist[1].0 < c.t - self.delay {
            self.hist.pop_front();
        }
    }
    cb!();
}

// ---------------------------------------------------------------- discontinuities

#[derive(Clone)]
struct RateLimiter {
    up: f64,
    down: f64,
    last: Option<(f64, Vec<f64>)>,
}
impl Rt for RateLimiter {
    fn out(&self, c: &Ctx, _: &[f64], u: &[f64], y: &mut [f64]) {
        match &self.last {
            Some((tl, yl)) if c.t > *tl => {
                let dt = c.t - tl;
                for j in 0..y.len() {
                    y[j] = u[j].clamp(yl[j] + self.down * dt, yl[j] + self.up * dt);
                }
            }
            Some((_, yl)) => y.copy_from_slice(yl),
            None => y.copy_from_slice(u),
        }
    }
    fn update(&mut self, c: &Ctx, _: &mut [f64], u: &[f64]) {
        let mut y = vec![0.0; u.len()];
        self.out(c, &[], u, &mut y);
        self.last = Some((c.t, y));
    }
    cb!();
}

#[derive(Clone)]
struct Relay {
    on: f64,
    off: f64,
    onv: f64,
    offv: f64,
    mode: Vec<bool>,
}
impl Rt for Relay {
    fn out(&self, _: &Ctx, _: &[f64], _: &[f64], y: &mut [f64]) {
        for (yj, m) in y.iter_mut().zip(&self.mode) {
            *yj = if *m { self.onv } else { self.offv };
        }
    }
    fn feed(&self) -> bool {
        false
    }
    fn update(&mut self, _: &Ctx, _: &mut [f64], u: &[f64]) {
        for (m, uj) in self.mode.iter_mut().zip(u) {
            if *uj >= self.on {
                *m = true;
            } else if *uj <= self.off {
                *m = false;
            }
        }
    }
    fn init(&mut self, _: &mut [f64]) {
        self.mode.iter_mut().for_each(|m| *m = false);
    }
    fn nzc(&self) -> usize {
        self.mode.len()
    }
    fn zc(&self, _: &Ctx, _: &[f64], u: &[f64], z: &mut [f64]) {
        for (j, m) in self.mode.iter().enumerate() {
            z[j] = if *m { u[j] - self.off } else { u[j] - self.on };
        }
    }
    cb!();
}

#[derive(Clone)]
struct Backlash {
    half: f64,
    y: Vec<f64>,
    y0: f64,
}
impl Rt for Backlash {
    fn out(&self, _: &Ctx, _: &[f64], u: &[f64], y: &mut [f64]) {
        for j in 0..y.len() {
            let mut v = self.y[j];
            if u[j] - self.half > v {
                v = u[j] - self.half;
            } else if u[j] + self.half < v {
                v = u[j] + self.half;
            }
            y[j] = v;
        }
    }
    fn update(&mut self, c: &Ctx, _: &mut [f64], u: &[f64]) {
        let mut y = vec![0.0; u.len()];
        self.out(c, &[], u, &mut y);
        self.y = y;
    }
    fn init(&mut self, _: &mut [f64]) {
        self.y.iter_mut().for_each(|v| *v = self.y0);
    }
    cb!();
}

// ---------------------------------------------------------------- discrete

#[derive(Clone)]
struct Zoh {
    ts: f64,
    held: Vec<f64>,
}
impl Rt for Zoh {
    fn out(&self, c: &Ctx, _: &[f64], u: &[f64], y: &mut [f64]) {
        if c.major && is_hit(c.t, self.ts) {
            y.copy_from_slice(u);
        } else {
            y.copy_from_slice(&self.held);
        }
    }
    fn update(&mut self, c: &Ctx, _: &mut [f64], u: &[f64]) {
        if is_hit(c.t, self.ts) {
            self.held.copy_from_slice(u);
        }
    }
    fn next_hit(&self, t: f64) -> f64 {
        next_sample(t, self.ts)
    }
    cb!();
}

#[derive(Clone)]
struct UnitDelay {
    ts: f64,
    ic: Vec<f64>,
    /// The input at the last sample (the next output) and the output held until the next sample.
    s: Vec<f64>,
    held: Vec<f64>,
}
impl Rt for UnitDelay {
    fn init(&mut self, _: &mut [f64]) {
        for (j, v) in self.s.iter_mut().enumerate() {
            *v = if self.ic.len() == 1 { self.ic[0] } else { self.ic[j] };
        }
        self.held.clone_from(&self.s);
    }
    fn feed(&self) -> bool {
        false
    }
    fn out(&self, c: &Ctx, _: &[f64], _: &[f64], y: &mut [f64]) {
        if c.major && is_hit(c.t, self.ts) {
            y.copy_from_slice(&self.s);
        } else {
            y.copy_from_slice(&self.held);
        }
    }
    fn update(&mut self, c: &Ctx, _: &mut [f64], u: &[f64]) {
        if is_hit(c.t, self.ts) {
            self.held.clone_from(&self.s);
            self.s.copy_from_slice(u);
        }
    }
    fn next_hit(&self, t: f64) -> f64 {
        next_sample(t, self.ts)
    }
    cb!();
}

// ---------------------------------------------------------------- routing and lookup

/// Copies input elements to outputs by index (Mux, Demux, Selector, pass-throughs).
#[derive(Clone)]
struct Route(Vec<usize>);
impl Rt for Route {
    fn out(&self, _: &Ctx, _: &[f64], u: &[f64], y: &mut [f64]) {
        for (yj, i) in y.iter_mut().zip(&self.0) {
            *yj = u[*i];
        }
    }
    cb!();
}

#[derive(Clone)]
struct Switch {
    ins: Ins,
    crit: u8,
    th: f64,
}
impl Rt for Switch {
    fn out(&self, _: &Ctx, _: &[f64], u: &[f64], y: &mut [f64]) {
        for (j, yj) in y.iter_mut().enumerate() {
            let c = self.ins.g(u, 1, j);
            let pass = match self.crit {
                0 => c >= self.th,
                1 => c > self.th,
                _ => c != 0.0,
            };
            *yj = self.ins.g(u, if pass { 0 } else { 2 }, j);
        }
    }
    cb!();
}

pub fn interp1(x: &[f64], y: &[f64], v: f64, extrap: bool) -> f64 {
    let n = x.len();
    if n == 1 {
        return y[0];
    }
    let i = x.partition_point(|a| *a <= v).clamp(1, n - 1);
    let (x0, x1) = (x[i - 1], x[i]);
    if !extrap && v <= x[0] {
        return y[0];
    }
    if !extrap && v >= x[n - 1] {
        return y[n - 1];
    }
    y[i - 1] + (v - x0) * (y[i] - y[i - 1]) / (x1 - x0)
}

#[derive(Clone)]
struct Lookup1 {
    x: Vec<f64>,
    y: Vec<f64>,
    extrap: bool,
}
impl Rt for Lookup1 {
    fn out(&self, _: &Ctx, _: &[f64], u: &[f64], y: &mut [f64]) {
        for (yj, uj) in y.iter_mut().zip(u) {
            *yj = interp1(&self.x, &self.y, *uj, self.extrap);
        }
    }
    cb!();
}

#[derive(Clone)]
struct Lookup2 {
    x: Vec<f64>,
    y: Vec<f64>,
    t: Mat,
}
impl Rt for Lookup2 {
    fn out(&self, _: &Ctx, _: &[f64], u: &[f64], y: &mut [f64]) {
        let col: Vec<f64> = (0..self.t.r).map(|i| {
            let row: Vec<f64> = (0..self.t.c).map(|j| self.t.at(i, j)).collect();
            interp1(&self.y, &row, u[1], false)
        }).collect();
        y[0] = interp1(&self.x, &col, u[0], false);
    }
    cb!();
}

#[derive(Clone)]
struct Sink;
impl Rt for Sink {
    fn out(&self, _: &Ctx, _: &[f64], _: &[f64], _: &mut [f64]) {}
    cb!();
}

// ---------------------------------------------------------------- motors

#[derive(Clone)]
struct DcMotor {
    r: f64,
    l: f64,
    ke: f64,
    kt: f64,
    j: f64,
    b: f64,
    w0: f64,
}
impl Rt for DcMotor {
    fn nx(&self) -> usize {
        3
    }
    fn init(&mut self, x: &mut [f64]) {
        x.copy_from_slice(&[0.0, self.w0, 0.0]);
    }
    fn feed(&self) -> bool {
        false
    }
    fn out(&self, _: &Ctx, x: &[f64], _: &[f64], y: &mut [f64]) {
        y[0] = x[1];
        y[1] = x[0];
        y[2] = x[2];
    }
    fn deriv(&self, _: &Ctx, x: &[f64], u: &[f64], dx: &mut [f64]) {
        let (i, w) = (x[0], x[1]);
        dx[0] = (u[0] - self.r * i - self.ke * w) / self.l;
        dx[1] = (self.kt * i - self.b * w - u[1]) / self.j;
        dx[2] = w;
    }
    cb!();
}

#[derive(Clone)]
struct Pmsm {
    rs: f64,
    ld: f64,
    lq: f64,
    psi: f64,
    p: f64,
    j: f64,
    b: f64,
}
impl Pmsm {
    fn te(&self, id: f64, iq: f64) -> f64 {
        1.5 * self.p * (self.psi * iq + (self.ld - self.lq) * id * iq)
    }
}
impl Rt for Pmsm {
    fn nx(&self) -> usize {
        4
    }
    fn feed(&self) -> bool {
        false
    }
    fn out(&self, _: &Ctx, x: &[f64], _: &[f64], y: &mut [f64]) {
        y[0] = x[2];
        y[1] = x[0];
        y[2] = x[1];
        y[3] = self.te(x[0], x[1]);
        y[4] = x[3];
    }
    fn deriv(&self, _: &Ctx, x: &[f64], u: &[f64], dx: &mut [f64]) {
        let (id, iq, w) = (x[0], x[1], x[2]);
        let we = self.p * w;
        dx[0] = (u[0] - self.rs * id + we * self.lq * iq) / self.ld;
        dx[1] = (u[1] - self.rs * iq - we * (self.ld * id + self.psi)) / self.lq;
        dx[2] = (self.te(id, iq) - self.b * w - u[2]) / self.j;
        dx[3] = w;
    }
    cb!();
}

// ---------------------------------------------------------------- build

/// Build a runnable block from its kind, evaluated parameters and input widths.
/// Returns the block and the widths of its outputs.
pub fn build(kind: &str, p: &Pv, iw: &[usize]) -> Result<(Box<dyn Rt>, Vec<usize>), String> {
    let one = |r: Box<dyn Rt>, w: usize| Ok((r, vec![w]));
    let w0 = iw.first().cloned().unwrap_or(1);
    let scalar_in = |what: &str| -> Result<(), String> {
        if iw.iter().any(|w| *w != 1) {
            return Err(format!("{what} takes scalar inputs (width 1); got widths {iw:?}"));
        }
        Ok(())
    };
    match kind {
        "Constant" => {
            let v = p.v("value")?;
            if v.is_empty() {
                return Err("Value is empty".into());
            }
            let n = v.len();
            one(Box::new(Constant(v)), n)
        }
        "Step" => one(Box::new(StepS { time: p.s("time")?, a: p.s("initial")?, b: p.s("final")? }), 1),
        "Ramp" => one(Box::new(Ramp { slope: p.s("slope")?, start: p.s("start")?, y0: p.s("initial")? }), 1),
        "Sine" => one(Box::new(Sine { a: p.s("amp")?, w: 2.0 * std::f64::consts::PI * p.s("freq")?, ph: p.s("phase")?, b: p.s("bias")? }), 1),
        "Pulse" => {
            let per = p.pos("period")?;
            let wd = p.s("width")?;
            if !(0.0..=100.0).contains(&wd) {
                return Err("Pulse width must be between 0 and 100 %".into());
            }
            one(Box::new(Pulse { a: p.s("amp")?, per, on: per * wd / 100.0, delay: p.s("delay")? }), 1)
        }
        "Clock" => one(Box::new(Clock), 1),
        "Table" => {
            let (t, v) = (p.v("times")?, p.v("values")?);
            if t.is_empty() || t.len() != v.len() {
                return Err("Times and Values must have the same length (at least 1)".into());
            }
            if t.windows(2).any(|w| w[1] < w[0]) || t.iter().any(|x| !x.is_finite()) {
                return Err("Times must increase".into());
            }
            let hold = p.txt("interp") == "hold";
            one(Box::new(Table { t, v: v.into_iter().map(|x| vec![x]).collect(), hold }), 1)
        }
        "FromFile" => {
            let f = p.txt("file");
            let path = match p.base {
                Some(b) if Path::new(f).is_relative() => b.join(f),
                _ => Path::new(f).to_path_buf(),
            };
            let tb = read_csv(&path)?;
            let w = tb.v[0].len();
            one(Box::new(tb), w)
        }
        "Noise" => one(Box::new(Noise { mean: p.s("mean")?, sd: p.s("sd")?, ts: p.pos("ts")?, seed: p.s("seed")?.abs() as u64, state: 1, val: 0.0 }), 1),
        "Gain" => {
            let k = p.mat("k")?;
            if k.is_empty() {
                return Err("Gain is empty".into());
            }
            if k.r > 1 {
                if k.c != w0 {
                    return Err(format!("the gain matrix has {} columns but the input has width {w0}", k.c));
                }
                let r = k.r;
                return one(Box::new(Gain { k, ins: w0, mat: true }), r);
            }
            let w = if k.len() == 1 { w0 } else { k.len() };
            if k.len() > 1 && w0 != 1 && w0 != k.len() {
                return Err(format!("the gain has {} elements but the input has width {w0}", k.len()));
            }
            one(Box::new(Gain { k, ins: w0, mat: false }), w)
        }
        "Sum" | "Product" => {
            let (key, chars) = if kind == "Sum" { ("signs", "+-") } else { ("ops", "*/") };
            let plus = chars.chars().next().unwrap();
            // One sign per input port (the ports were counted from the same characters).
            let mut signs: Vec<f64> = p.txt(key).chars().filter(|c| chars.contains(*c)).map(|c| if c == plus { 1.0 } else { -1.0 }).collect();
            signs.resize(iw.len().max(1), 1.0);
            let w = ewidth(kind, iw)?;
            one(Box::new(Ew { ins: Ins::new(iw), signs, op: if kind == "Sum" { b's' } else { b'p' } }), w)
        }
        "Divide" => {
            let w = ewidth(kind, iw)?;
            one(Box::new(Ew { ins: Ins::new(iw), signs: vec![1.0, -1.0], op: b'p' }), w)
        }
        "Abs" => one(Box::new(Map1(|u, _| u.abs(), vec![])), w0),
        "Sign" => one(Box::new(Map1(|u, _| if u > 0.0 { 1.0 } else if u < 0.0 { -1.0 } else { 0.0 }, vec![])), w0),
        "Saturation" => {
            let (hi, lo) = (p.s("upper")?, p.s("lower")?);
            if lo > hi {
                return Err("the lower limit is above the upper limit".into());
            }
            one(Box::new(Map1(|u, k| u.clamp(k[1], k[0]), vec![hi, lo])), w0)
        }
        "DeadZone" => {
            let (a, b) = (p.s("start")?, p.s("end")?);
            if a > b {
                return Err("the start of the zone is after its end".into());
            }
            one(Box::new(Map1(|u, k| if u > k[1] { u - k[1] } else if u < k[0] { u - k[0] } else { 0.0 }, vec![a, b])), w0)
        }
        "Quantizer" => one(Box::new(Map1(|u, k| k[0] * (u / k[0]).round(), vec![p.pos("q")?])), w0),
        "MinMax" => {
            let w = ewidth(kind, iw)?;
            one(Box::new(MinMax { ins: Ins::new(iw), max: p.txt("fn") == "max" }), w)
        }
        "Fcn" => {
            scalar_in("The Function block")?;
            let n = iw.len();
            let mut names: Vec<String> = vec!["t".into()];
            names.extend((1..=n).map(|i| format!("u{i}")));
            names.push("u".into());
            let mut e = expr::parse(p.txt("expr")).map_err(|e| format!("Expression: {e}"))?;
            // Model variables are folded in as constants.
            fold(&mut e, p.vars);
            let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
            e.bind(&refs).map_err(|e| format!("Expression: {e}"))?;
            one(Box::new(Fcn { e, n }), 1)
        }
        "Integrator" => {
            let ic = p.v("ic")?;
            let (hi, lo) = (p.s("upper")?, p.s("lower")?);
            if lo > hi {
                return Err("the lower limit is above the upper limit".into());
            }
            let w = if ic.len() > 1 { ic.len() } else { w0 };
            if w0 != 1 && w0 != w {
                return Err(format!("the initial condition has {} elements but the input has width {w0}", ic.len()));
            }
            let ic = if ic.len() == 1 { vec![ic[0]; w] } else { ic };
            one(Box::new(Integrator { ic, hi, lo, ins: w0 }), w)
        }
        "Derivative" => one(Box::new(Derivative { last: None, prev_y: vec![0.0; w0] }), w0),
        "TransferFcn" | "ZPK" => {
            scalar_in("A transfer function")?;
            let lin = if kind == "TransferFcn" {
                tf2ss(&p.v("num")?, &p.v("den")?)?
            } else {
                let z = poly(&parse_roots(p.txt("zeros"))?)?;
                let k = p.s("k")?;
                tf2ss(&z.iter().map(|v| v * k).collect::<Vec<_>>(), &poly(&parse_roots(p.txt("poles"))?)?)?
            };
            one(Box::new(lin), 1)
        }
        "StateSpace" => {
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
            one(Box::new(Lin { a, b, c, d, x0 }), r)
        }
        "PID" => {
            scalar_in("The PID block")?;
            let (hi, lo) = (p.s("upper")?, p.s("lower")?);
            if lo > hi {
                return Err("the lower limit is above the upper limit".into());
            }
            one(Box::new(Pid { p: p.s("P")?, i: p.s("I")?, d: p.s("D")?, n: p.pos("N")?, hi, lo, ic: p.s("ic")? }), 1)
        }
        "Delay" => {
            let delay = p.s("delay")?;
            if delay < 0.0 {
                return Err("the delay cannot be negative".into());
            }
            let y0 = p.v("initial")?;
            if y0.len() != 1 && y0.len() != w0 {
                return Err("the initial output must be one value or one per element".into());
            }
            one(Box::new(Delay { delay, y0, hist: VecDeque::new() }), w0)
        }
        "RateLimiter" => {
            let (up, down) = (p.s("rising")?, p.s("falling")?);
            if up < 0.0 || down > 0.0 {
                return Err("the rising rate must be ≥ 0 and the falling rate ≤ 0".into());
            }
            one(Box::new(RateLimiter { up, down, last: None }), w0)
        }
        "Relay" => {
            let (on, off) = (p.s("on")?, p.s("off")?);
            if off > on {
                return Err("the off point must not be above the on point".into());
            }
            one(Box::new(Relay { on, off, onv: p.s("onv")?, offv: p.s("offv")?, mode: vec![false; w0] }), w0)
        }
        "Backlash" => {
            let y0 = p.s("ic")?;
            one(Box::new(Backlash { half: p.s("width")?.abs() / 2.0, y: vec![y0; w0], y0 }), w0)
        }
        "ZOH" => one(Box::new(Zoh { ts: p.pos("ts")?, held: vec![0.0; w0] }), w0),
        "UnitDelay" => {
            let ic = p.v("ic")?;
            if ic.len() != 1 && ic.len() != w0 {
                return Err("the initial output must be one value or one per element".into());
            }
            one(Box::new(UnitDelay { ts: p.pos("ts")?, ic, s: vec![0.0; w0], held: vec![0.0; w0] }), w0)
        }
        "Mux" => {
            let w: usize = iw.iter().sum();
            one(Box::new(Route((0..w).collect())), w)
        }
        "Demux" => {
            let n = iw.len().max(1);
            let k = p.m.get("n").and_then(|m| m.s().ok()).map_or(2, |v| v.round().clamp(1.0, 32.0) as usize);
            if w0 % k != 0 {
                return Err(format!("a signal of width {w0} cannot be split into {k} equal parts"));
            }
            let _ = n;
            let part = w0 / k;
            Ok((Box::new(Route((0..w0).collect())), vec![part; k]))
        }
        "Selector" => {
            let idx = p.v("idx")?;
            let mut v = Vec::new();
            for i in idx {
                if i < 1.0 || i.fract() != 0.0 || i as usize > w0 {
                    return Err(format!("index {i} is outside 1…{w0}"));
                }
                v.push(i as usize - 1);
            }
            let n = v.len();
            if n == 0 {
                return Err("no indices".into());
            }
            one(Box::new(Route(v)), n)
        }
        "Switch" => {
            let w = ewidth(kind, iw)?;
            let crit = match p.txt("crit") {
                "u2 > threshold" => 1,
                "u2 != 0" => 2,
                _ => 0,
            };
            one(Box::new(Switch { ins: Ins::new(iw), crit, th: p.s("th")? }), w)
        }
        "Lookup1D" => {
            let (x, y) = (p.v("x")?, p.v("y")?);
            if x.is_empty() || x.len() != y.len() {
                return Err("x and y must have the same length (at least 1)".into());
            }
            if x.windows(2).any(|w| w[1] <= w[0]) {
                return Err("x must increase".into());
            }
            one(Box::new(Lookup1 { x, y, extrap: p.txt("extrap") == "linear" }), w0)
        }
        "Lookup2D" => {
            scalar_in("The 2-D lookup table")?;
            let (x, y, t) = (p.v("x")?, p.v("y")?, p.mat("table")?);
            if t.r != x.len() || t.c != y.len() || x.is_empty() || y.is_empty() {
                return Err(format!("the table must be {}×{} (rows × columns)", x.len(), y.len()));
            }
            if x.windows(2).any(|w| w[1] <= w[0]) || y.windows(2).any(|w| w[1] <= w[0]) {
                return Err("breakpoints must increase".into());
            }
            one(Box::new(Lookup2 { x, y, t }), 1)
        }
        "Scope" | "XY" | "Display" | "ToFile" | "ToPlot" | "Outport" => Ok((Box::new(Sink), vec![])),
        // Pass-throughs left by flattening subsystems.
        "Inport" | "Pass" => one(Box::new(Route((0..w0).collect())), w0),
        "DCMotor" => {
            scalar_in("The DC motor")?;
            one(Box::new(DcMotor { r: p.s("R")?, l: p.pos("L")?, ke: p.s("Ke")?, kt: p.s("Kt")?, j: p.pos("J")?, b: p.s("b")?, w0: p.s("w0")? }), 1)
                .map(|(r, _)| (r, vec![1, 1, 1]))
        }
        "PMSM" => {
            scalar_in("The PMSM")?;
            let m = Pmsm { rs: p.s("Rs")?, ld: p.pos("Ld")?, lq: p.pos("Lq")?, psi: p.s("psi")?, p: p.pos("p")?, j: p.pos("J")?, b: p.s("b")? };
            Ok((Box::new(m), vec![1; 5]))
        }
        _ => Err(format!("unknown block type '{kind}'")),
    }
}

/// Replace model variables in a Function expression by their (scalar) values.
fn fold(e: &mut expr::Node, vars: &BTreeMap<String, Mat>) {
    use expr::Node;
    match e {
        Node::Var(n) => {
            if let Some(m) = vars.get(n.as_str()) {
                if m.is_scalar() {
                    *e = Node::Num(m.d[0]);
                }
            }
        }
        Node::Neg(a) => fold(a, vars),
        Node::Bin(_, a, b) | Node::Cmp(_, a, b) => {
            fold(a, vars);
            fold(b, vars);
        }
        Node::Call(_, args) => args.iter_mut().for_each(|a| fold(a, vars)),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roots_and_polys() {
        assert_eq!(parse_roots("-1+2j -1-2j, 3").unwrap(), vec![(-1.0, 2.0), (-1.0, -2.0), (3.0, 0.0)]);
        assert_eq!(parse_roots("[2j -2j]").unwrap(), vec![(0.0, 2.0), (0.0, -2.0)]);
        assert_eq!(poly(&[(-1.0, 2.0), (-1.0, -2.0)]).unwrap(), vec![1.0, 2.0, 5.0]);
        assert!(poly(&[(-1.0, 2.0)]).is_err());
        assert!(parse_roots("abc").is_err());
    }
    #[test]
    fn tf_realisation() {
        let l = tf2ss(&[2.0, 3.0], &[1.0, 3.0, 2.0]).unwrap();
        assert_eq!(l.a.d, vec![-3.0, -2.0, 1.0, 0.0]);
        assert_eq!(l.c.d, vec![2.0, 3.0]);
        assert_eq!(l.d.d, vec![0.0]);
        assert!(tf2ss(&[1.0, 0.0, 0.0], &[1.0, 1.0]).is_err());
        let p = tf2ss(&[1.0, 1.0], &[1.0, 2.0]).unwrap(); // (s+1)/(s+2) = 1 − 1/(s+2)
        assert_eq!((p.d.d[0], p.c.d[0]), (1.0, -1.0));
    }
    #[test]
    fn pulse_hits() {
        let p = Pulse { a: 1.0, per: 1.0, on: 0.25, delay: 0.5 };
        assert_eq!(p.next_hit(0.0), 0.5);
        assert_eq!(p.next_hit(0.5), 0.75);
        assert_eq!(p.next_hit(0.75), 1.5);
        assert_eq!(next_sample(0.0, 0.1), 0.1);
        assert!((next_sample(0.1, 0.1) - 0.2).abs() < 1e-15);
    }
}
