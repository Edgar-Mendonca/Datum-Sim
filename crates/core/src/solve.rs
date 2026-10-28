//! Solvers and the stepping loop. Fixed step: Euler, Heun, RK4. Variable step: Dormand–Prince 5(4)
//! with error control and 4th-order dense output, and a Rosenbrock 2(3) method for stiff systems
//! (Shampine & Reichelt's "ode23s" scheme, Jacobian by finite differences).
//! The loop lands exactly on times blocks ask for (jumps, samples), locates zero-crossings on the
//! dense output, and calls the model at every accepted (major) step.

use crate::linalg::Lu;

/// A system of ODEs. `tseg` is the start of the current step (see `rt::Ctx`).
pub trait Ode {
    fn n(&self) -> usize;
    fn f(&mut self, t: f64, tseg: f64, x: &[f64], dx: &mut [f64]);
}

/// What the loop asks of the model besides derivatives.
pub trait Hooks {
    /// The next time after `t` the loop must land on exactly.
    fn next_hit(&self, _t: f64) -> f64 {
        f64::INFINITY
    }
    /// An accepted step at `t`: log, update discrete states and modes. Return false to stop.
    fn major(&mut self, t: f64, x: &mut [f64]) -> bool;
    fn nzc(&self) -> usize {
        0
    }
    fn zc(&mut self, _t: f64, _x: &[f64], _z: &mut [f64]) {}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    Euler,
    Heun,
    Rk4,
    Dopri5,
    Stiff,
}

impl Method {
    pub fn parse(s: &str) -> Method {
        match s {
            "euler" => Method::Euler,
            "heun" => Method::Heun,
            "rk4" => Method::Rk4,
            "stiff" => Method::Stiff,
            _ => Method::Dopri5,
        }
    }
    pub fn adaptive(self) -> bool {
        matches!(self, Method::Dopri5 | Method::Stiff)
    }
    /// The order used for step-size control (and the convergence tests).
    pub fn order(self) -> i32 {
        match self {
            Method::Euler => 1,
            Method::Heun => 2,
            Method::Rk4 => 4,
            Method::Dopri5 => 5,
            Method::Stiff => 2,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Opts {
    pub method: Method,
    pub t0: f64,
    pub t1: f64,
    /// The fixed step (fixed-step solvers).
    pub h: f64,
    /// The largest step (variable-step solvers).
    pub hmax: f64,
    pub rtol: f64,
    pub atol: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Stats {
    pub steps: u64,
    pub rejected: u64,
    pub fevals: u64,
    pub events: u64,
    pub stopped: bool,
}

/// One-step methods with their stage storage and dense output.
pub struct Stepper {
    m: Method,
    k: Vec<Vec<f64>>,
    tmp: Vec<f64>,
    x0: Vec<f64>,
    x1: Vec<f64>,
    h: f64,
    jac: Vec<f64>,
    pub fevals: u64,
}

// Dormand–Prince 5(4) tableau.
const C: [f64; 7] = [0.0, 0.2, 0.3, 0.8, 8.0 / 9.0, 1.0, 1.0];
const A: [[f64; 6]; 7] = [
    [0.0; 6],
    [0.2, 0.0, 0.0, 0.0, 0.0, 0.0],
    [3.0 / 40.0, 9.0 / 40.0, 0.0, 0.0, 0.0, 0.0],
    [44.0 / 45.0, -56.0 / 15.0, 32.0 / 9.0, 0.0, 0.0, 0.0],
    [19372.0 / 6561.0, -25360.0 / 2187.0, 64448.0 / 6561.0, -212.0 / 729.0, 0.0, 0.0],
    [9017.0 / 3168.0, -355.0 / 33.0, 46732.0 / 5247.0, 49.0 / 176.0, -5103.0 / 18656.0, 0.0],
    [35.0 / 384.0, 0.0, 500.0 / 1113.0, 125.0 / 192.0, -2187.0 / 6784.0, 11.0 / 84.0],
];
const E: [f64; 7] = [71.0 / 57600.0, 0.0, -71.0 / 16695.0, 71.0 / 1920.0, -17253.0 / 339200.0, 22.0 / 525.0, -1.0 / 40.0];
const D: [f64; 7] = [
    -12715105075.0 / 11282082432.0,
    0.0,
    87487479700.0 / 32700410799.0,
    -10690763975.0 / 1880347072.0,
    701980252875.0 / 199316789632.0,
    -1453857185.0 / 822651844.0,
    69997945.0 / 29380423.0,
];

impl Stepper {
    pub fn new(m: Method, n: usize) -> Stepper {
        Stepper { m, k: vec![vec![0.0; n]; 7], tmp: vec![0.0; n], x0: vec![0.0; n], x1: vec![0.0; n], h: 0.0, jac: vec![], fevals: 0 }
    }

    fn f<S: Ode>(&mut self, s: &mut S, t: f64, tseg: f64, i: usize) {
        self.fevals += 1;
        let (x, k) = (&self.tmp, &mut self.k[i]);
        s.f(t, tseg, x, k);
    }

    /// Take one step of size h from (t, x) into `xn`. Returns the scaled error (≤ 1 is acceptable;
    /// 0 for fixed-step methods).
    pub fn step<S: Ode>(&mut self, s: &mut S, t: f64, x: &[f64], h: f64, xn: &mut [f64], rtol: f64, atol: f64) -> f64 {
        let n = x.len();
        self.h = h;
        self.x0.copy_from_slice(x);
        let mut err = 0.0;
        match self.m {
            Method::Euler => {
                self.tmp.copy_from_slice(x);
                self.f(s, t, t, 0);
                for i in 0..n {
                    xn[i] = x[i] + h * self.k[0][i];
                }
            }
            Method::Heun => {
                self.tmp.copy_from_slice(x);
                self.f(s, t, t, 0);
                for i in 0..n {
                    self.tmp[i] = x[i] + h * self.k[0][i];
                }
                self.f(s, t + h, t, 1);
                for i in 0..n {
                    xn[i] = x[i] + 0.5 * h * (self.k[0][i] + self.k[1][i]);
                }
            }
            Method::Rk4 => {
                self.tmp.copy_from_slice(x);
                self.f(s, t, t, 0);
                for (st, (c, a)) in [(1, (0.5, 0.5)), (2, (0.5, 0.5)), (3, (1.0, 1.0))] {
                    for i in 0..n {
                        self.tmp[i] = x[i] + a * h * self.k[st - 1][i];
                    }
                    self.f(s, t + c * h, t, st);
                }
                for i in 0..n {
                    xn[i] = x[i] + h / 6.0 * (self.k[0][i] + 2.0 * self.k[1][i] + 2.0 * self.k[2][i] + self.k[3][i]);
                }
            }
            Method::Dopri5 => {
                self.tmp.copy_from_slice(x);
                self.f(s, t, t, 0);
                for st in 1..7 {
                    for i in 0..n {
                        let mut acc = 0.0;
                        for (j, a) in A[st].iter().enumerate().take(st) {
                            acc += a * self.k[j][i];
                        }
                        self.tmp[i] = x[i] + h * acc;
                    }
                    if st == 6 {
                        xn.copy_from_slice(&self.tmp);
                    }
                    self.f(s, t + C[st] * h, t, st);
                }
                let mut sum = 0.0;
                for i in 0..n {
                    let e: f64 = h * (0..7).map(|j| E[j] * self.k[j][i]).sum::<f64>();
                    let sc = atol + rtol * x[i].abs().max(xn[i].abs());
                    sum += (e / sc).powi(2);
                }
                err = if n > 0 { (sum / n as f64).sqrt() } else { 0.0 };
            }
            Method::Stiff => {
                err = self.rosenbrock(s, t, x, h, xn, rtol, atol);
            }
        }
        if !err.is_finite() || xn.iter().any(|v| !v.is_finite()) {
            err = f64::INFINITY;
        }
        self.x1.copy_from_slice(xn);
        err
    }

    #[allow(clippy::too_many_arguments)]
    fn rosenbrock<S: Ode>(&mut self, s: &mut S, t: f64, x: &[f64], h: f64, xn: &mut [f64], rtol: f64, atol: f64) -> f64 {
        let n = x.len();
        let d = 1.0 / (2.0 + std::f64::consts::SQRT_2);
        let e32 = 6.0 + std::f64::consts::SQRT_2;
        // F0, Jacobian and dF/dt by finite differences.
        self.tmp.copy_from_slice(x);
        self.f(s, t, t, 0); // k[0] = F0
        let f0 = self.k[0].clone();
        self.jac.resize(n * n, 0.0);
        for j in 0..n {
            let dx = 1e-7 * x[j].abs().max(1e-3);
            self.tmp.copy_from_slice(x);
            self.tmp[j] += dx;
            self.f(s, t, t, 6);
            for i in 0..n {
                self.jac[i * n + j] = (self.k[6][i] - f0[i]) / dx;
            }
        }
        let dt = 1e-7 * t.abs().max(h).max(1e-9);
        self.tmp.copy_from_slice(x);
        self.f(s, t + dt, t, 6);
        let ft: Vec<f64> = (0..n).map(|i| (self.k[6][i] - f0[i]) / dt).collect();
        let mut w = vec![0.0; n * n];
        for i in 0..n {
            for j in 0..n {
                w[i * n + j] = if i == j { 1.0 } else { 0.0 } - h * d * self.jac[i * n + j];
            }
        }
        let Some(lu) = Lu::new(w, n) else { return f64::INFINITY };
        // k1
        let mut k1: Vec<f64> = (0..n).map(|i| f0[i] + h * d * ft[i]).collect();
        lu.solve(&mut k1);
        // F1 at the midpoint
        for i in 0..n {
            self.tmp[i] = x[i] + 0.5 * h * k1[i];
        }
        self.f(s, t + 0.5 * h, t, 1);
        let f1 = self.k[1].clone();
        let mut k2: Vec<f64> = (0..n).map(|i| f1[i] - k1[i]).collect();
        lu.solve(&mut k2);
        for i in 0..n {
            k2[i] += k1[i];
            xn[i] = x[i] + h * k2[i];
        }
        self.tmp.copy_from_slice(xn);
        self.f(s, t + h, t, 2);
        let f2 = &self.k[2];
        let mut k3: Vec<f64> = (0..n).map(|i| f2[i] - e32 * (k2[i] - f1[i]) - 2.0 * (k1[i] - f0[i]) + h * d * ft[i]).collect();
        lu.solve(&mut k3);
        let mut sum = 0.0;
        for i in 0..n {
            let e = h / 6.0 * (k1[i] - 2.0 * k2[i] + k3[i]);
            let sc = atol + rtol * x[i].abs().max(xn[i].abs());
            sum += (e / sc).powi(2);
        }
        self.k[3] = k1;
        self.k[4] = k2;
        if n > 0 {
            (sum / n as f64).sqrt()
        } else {
            0.0
        }
    }

    /// The state at t + θh inside the last step (θ in [0, 1]).
    pub fn dense(&self, th: f64, out: &mut [f64]) {
        let n = out.len();
        let h = self.h;
        match self.m {
            Method::Dopri5 => {
                for i in 0..n {
                    let (y0, y1) = (self.x0[i], self.x1[i]);
                    let yd = y1 - y0;
                    let bspl = h * self.k[0][i] - yd;
                    let r4 = yd - h * self.k[6][i] - bspl;
                    let r5 = h * (0..7).map(|j| D[j] * self.k[j][i]).sum::<f64>();
                    out[i] = y0 + th * (yd + (1.0 - th) * (bspl + th * (r4 + (1.0 - th) * r5)));
                }
            }
            Method::Stiff => {
                let d = 1.0 / (2.0 + std::f64::consts::SQRT_2);
                for i in 0..n {
                    out[i] = self.x0[i]
                        + h * (th * (1.0 - th) / (1.0 - 2.0 * d) * self.k[3][i] + th * (th - 2.0 * d) / (1.0 - 2.0 * d) * self.k[4][i]);
                }
            }
            _ => {
                for i in 0..n {
                    out[i] = self.x0[i] + th * (self.x1[i] - self.x0[i]);
                }
            }
        }
    }
}

fn sign_change(a: f64, b: f64) -> bool {
    (a < 0.0 && b >= 0.0) || (a > 0.0 && b <= 0.0)
}

/// Run from t0 to t1. `x` holds the initial state and ends with the final state.
pub fn drive<S: Ode + Hooks>(sys: &mut S, x: &mut [f64], o: &Opts) -> Result<Stats, String> {
    let n = sys.n();
    let mut st = Stepper::new(o.method, n);
    let mut stats = Stats::default();
    let span = o.t1 - o.t0;
    if !(span > 0.0) {
        return Err("The stop time must be after the start time.".into());
    }
    let mut t = o.t0;
    if !sys.major(t, x) {
        stats.stopped = true;
        return Ok(stats);
    }
    let nz = sys.nzc();
    let mut z0 = vec![0.0; nz];
    let mut z1 = vec![0.0; nz];
    sys.zc(t, x, &mut z0);
    let hmax = if o.method.adaptive() { o.hmax.min(span) } else { o.h };
    let mut h = if o.method.adaptive() { (span * 1e-3).min(hmax) } else { o.h };
    let mut xn = vec![0.0; n];
    let mut xe = vec![0.0; n];
    let tiny = |t: f64| 1e-12 * t.abs().max(span);
    while t < o.t1 - tiny(t) {
        let hit = sys.next_hit(t).min(o.t1);
        let mut hh = h.min(hmax).min(hit - t);
        // Avoid leaving a sliver before a hit.
        if hit - t - hh < tiny(t) * 100.0 {
            hh = hit - t;
        }
        if hh < 1e-14 * t.abs().max(1.0) {
            return Err(format!(
                "The solver could not take a step at t = {t:.6} (the step became too small). The model may be stiff or switch too often: try the stiff solver, or loosen the tolerances."
            ));
        }
        let err = st.step(sys, t, x, hh, &mut xn, o.rtol, o.atol);
        if o.method.adaptive() {
            if err > 1.0 {
                stats.rejected += 1;
                let fac = if err.is_finite() { (0.9 * err.powf(-1.0 / o.method.order() as f64)).clamp(0.1, 0.9) } else { 0.1 };
                h = hh * fac;
                if stats.rejected > 10_000_000 {
                    return Err("The solver rejected too many steps.".into());
                }
                continue;
            }
        } else if !err.is_finite() {
            return Err(format!("The solution became infinite or undefined at t = {:.6}. Try a smaller step or a variable-step solver.", t + hh));
        }
        // Zero-crossings: locate the first one on the dense output and stop the step there.
        let mut tn = t + hh;
        if nz > 0 {
            sys.zc(tn, &xn, &mut z1);
            if (0..nz).any(|i| sign_change(z0[i], z1[i])) {
                let (mut lo, mut hi) = (0.0f64, 1.0f64);
                for _ in 0..60 {
                    if (hi - lo) * hh <= 4.0 * tiny(t) {
                        break;
                    }
                    let mid = 0.5 * (lo + hi);
                    st.dense(mid, &mut xe);
                    sys.zc(t + mid * hh, &xe, &mut z1);
                    if (0..nz).any(|i| sign_change(z0[i], z1[i])) {
                        hi = mid;
                    } else {
                        lo = mid;
                    }
                }
                st.dense(hi, &mut xn);
                tn = t + hi * hh;
                stats.events += 1;
            }
        }
        x.copy_from_slice(&xn);
        t = tn;
        stats.steps += 1;
        if !sys.major(t, x) {
            stats.stopped = true;
            break;
        }
        if nz > 0 {
            // A function that lands exactly on zero keeps its last sign, so the crossing still
            // registers when it moves on.
            sys.zc(t, x, &mut z1);
            for (a, b) in z0.iter_mut().zip(&z1) {
                if *b != 0.0 {
                    *a = *b;
                }
            }
        }
        if o.method.adaptive() {
            let fac = if err > 0.0 { (0.9 * err.powf(-1.0 / o.method.order() as f64)).clamp(0.2, 5.0) } else { 5.0 };
            h = (hh * fac).min(hmax);
        }
    }
    stats.fevals = st.fevals;
    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A plain ODE with logging, for the conformance tests.
    struct Plain<F: FnMut(f64, &[f64], &mut [f64])> {
        n: usize,
        f: F,
        log: Vec<(f64, Vec<f64>)>,
    }
    impl<F: FnMut(f64, &[f64], &mut [f64])> Ode for Plain<F> {
        fn n(&self) -> usize {
            self.n
        }
        fn f(&mut self, t: f64, _: f64, x: &[f64], dx: &mut [f64]) {
            (self.f)(t, x, dx)
        }
    }
    impl<F: FnMut(f64, &[f64], &mut [f64])> Hooks for Plain<F> {
        fn major(&mut self, t: f64, x: &mut [f64]) -> bool {
            self.log.push((t, x.to_vec()));
            true
        }
    }

    fn run(m: Method, n: usize, x0: Vec<f64>, t1: f64, h: f64, tol: f64, f: impl FnMut(f64, &[f64], &mut [f64])) -> (Vec<f64>, Stats) {
        let mut p = Plain { n, f, log: vec![] };
        let mut x = x0;
        let o = Opts { method: m, t0: 0.0, t1, h, hmax: t1, rtol: tol, atol: tol * 1e-2 };
        let s = drive(&mut p, &mut x, &o).unwrap();
        (x, s)
    }

    /// Order of convergence: halving the step divides the error by about 2^p.
    #[test]
    fn convergence_order() {
        for (m, p) in [(Method::Euler, 1.0), (Method::Heun, 2.0), (Method::Rk4, 4.0)] {
            let err = |h: f64| {
                let (x, _) = run(m, 1, vec![1.0], 1.0, h, 1e-6, |_, x, d| d[0] = -2.0 * x[0]);
                (x[0] - (-2.0f64).exp()).abs()
            };
            let ratio = err(0.02) / err(0.01);
            let order = ratio.log2();
            assert!((order - p).abs() < 0.15, "{m:?}: order {order}");
        }
    }

    /// Dahlquist y' = λy and a harmonic oscillator (energy kept), variable-step solvers.
    #[test]
    fn variable_step_accuracy() {
        for (m, tol, want) in [(Method::Dopri5, 1e-8, 1e-6), (Method::Stiff, 1e-6, 1e-3)] {
            let (x, _) = run(m, 1, vec![1.0], 2.0, 0.0, tol, |_, x, d| d[0] = -3.0 * x[0]);
            assert!((x[0] - (-6.0f64).exp()).abs() < want, "{m:?} dahlquist {}", x[0]);
            let (x, _) = run(m, 2, vec![1.0, 0.0], 20.0, 0.0, tol, |_, x, d| {
                d[0] = x[1];
                d[1] = -x[0];
            });
            assert!((x[0] - 20f64.cos()).abs() < want * 20.0, "{m:?} oscillator {}", x[0]);
            let energy = x[0] * x[0] + x[1] * x[1];
            assert!((energy - 1.0).abs() < want * 20.0, "{m:?} energy {energy}");
        }
    }

    /// Dense output of Dormand–Prince is 4th-order accurate between steps.
    #[test]
    fn dense_output() {
        let mut p = Plain { n: 1, f: |_: f64, x: &[f64], d: &mut [f64]| d[0] = x[0], log: vec![] };
        let mut st = Stepper::new(Method::Dopri5, 1);
        let mut xn = vec![0.0];
        st.step(&mut p, 0.0, &[1.0], 0.1, &mut xn, 1e-6, 1e-8);
        let mut y = vec![0.0];
        st.dense(0.5, &mut y);
        assert!((y[0] - 0.05f64.exp()).abs() < 1e-8, "{}", y[0]);
    }

    /// Van der Pol with μ = 1000 (stiff): the stiff solver gets there in few steps.
    #[test]
    fn van_der_pol_stiff() {
        let mu = 1000.0;
        let (x, s) = run(Method::Stiff, 2, vec![2.0, 0.0], 3000.0, 0.0, 1e-5, move |_, x, d| {
            d[0] = x[1];
            d[1] = mu * ((1.0 - x[0] * x[0]) * x[1]) - x[0];
        });
        // Reference: x(3000) ≈ −1.5 (one relaxation period is ≈ 1614 for μ = 1000)
        assert!(x[0] < -1.0 && x[0] > -2.1, "x = {}", x[0]);
        assert!(s.steps < 20_000, "steps {}", s.steps);
    }

    /// Robertson's chemical kinetics (very stiff): mass is conserved and the known values reached.
    #[test]
    fn robertson() {
        let (x, _) = run(Method::Stiff, 3, vec![1.0, 0.0, 0.0], 40.0, 0.0, 1e-6, |_, y, d| {
            d[0] = -0.04 * y[0] + 1e4 * y[1] * y[2];
            d[1] = 0.04 * y[0] - 1e4 * y[1] * y[2] - 3e7 * y[1] * y[1];
            d[2] = 3e7 * y[1] * y[1];
        });
        // Reference at t = 40 (Hairer & Wanner): y1 = 0.7158271, y2 = 9.185535e-6, y3 = 0.2841637
        assert!((x[0] - 0.7158271).abs() < 2e-4, "y1 {}", x[0]);
        assert!((x[1] - 9.185535e-6).abs() < 2e-7, "y2 {}", x[1]);
        assert!((x[0] + x[1] + x[2] - 1.0).abs() < 1e-6);
    }

    /// A bouncing ball: impact times located on the dense output match the closed form.
    #[test]
    fn bouncing_ball_events() {
        struct Ball {
            log: Vec<f64>,
        }
        impl Ode for Ball {
            fn n(&self) -> usize {
                2
            }
            fn f(&mut self, _: f64, _: f64, x: &[f64], d: &mut [f64]) {
                d[0] = x[1];
                d[1] = -9.81;
            }
        }
        impl Hooks for Ball {
            fn major(&mut self, t: f64, x: &mut [f64]) -> bool {
                if x[0] <= 1e-9 && x[1] < 0.0 {
                    self.log.push(t);
                    x[0] = 0.0;
                    x[1] *= -0.8;
                }
                true
            }
            fn nzc(&self) -> usize {
                1
            }
            fn zc(&mut self, _: f64, x: &[f64], z: &mut [f64]) {
                z[0] = x[0];
            }
        }
        let mut b = Ball { log: vec![] };
        let mut x = vec![10.0, 0.0];
        let o = Opts { method: Method::Dopri5, t0: 0.0, t1: 5.0, h: 0.0, hmax: 0.1, rtol: 1e-8, atol: 1e-10 };
        drive(&mut b, &mut x, &o).unwrap();
        let t1 = (2.0 * 10.0 / 9.81f64).sqrt();
        assert!((b.log[0] - t1).abs() < 1e-7, "first impact {} vs {}", b.log[0], t1);
        let t2 = t1 + 2.0 * 0.8 * t1;
        assert!((b.log[1] - t2).abs() < 1e-6, "second impact {} vs {}", b.log[1], t2);
    }
}
