//! Linear time-invariant systems (A, B, C, D): eigenvalues, frequency response, margins, Nyquist,
//! root locus, sensitivity, closed loop, step and impulse responses, and PID design. Pure Rust, so
//! the analyses are fast enough to follow sliders and can be tested against closed forms.

use serde::Serialize;

pub type M = Vec<Vec<f64>>;

#[derive(Clone, Debug, Serialize)]
pub struct Ss {
    pub a: M,
    pub b: M,
    pub c: M,
    pub d: M,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cx {
    pub re: f64,
    pub im: f64,
}
impl Cx {
    pub fn new(re: f64, im: f64) -> Cx {
        Cx { re, im }
    }
    pub fn abs(self) -> f64 {
        self.re.hypot(self.im)
    }
    pub fn arg(self) -> f64 {
        self.im.atan2(self.re)
    }
    fn add(self, o: Cx) -> Cx {
        Cx::new(self.re + o.re, self.im + o.im)
    }
    fn sub(self, o: Cx) -> Cx {
        Cx::new(self.re - o.re, self.im - o.im)
    }
    fn mul(self, o: Cx) -> Cx {
        Cx::new(self.re * o.re - self.im * o.im, self.re * o.im + self.im * o.re)
    }
    fn div(self, o: Cx) -> Cx {
        let d = o.re * o.re + o.im * o.im;
        Cx::new((self.re * o.re + self.im * o.im) / d, (self.im * o.re - self.re * o.im) / d)
    }
    fn sc(self, k: f64) -> Cx {
        Cx::new(self.re * k, self.im * k)
    }
}

impl Ss {
    pub fn n(&self) -> usize {
        self.a.len()
    }
    pub fn gain(k: f64) -> Ss {
        Ss { a: vec![], b: vec![], c: vec![vec![]], d: vec![vec![k]] }
    }
    /// One input and one output of a larger system.
    pub fn siso(&self, i: usize, o: usize) -> Ss {
        Ss {
            a: self.a.clone(),
            b: self.b.iter().map(|r| vec![r[i]]).collect(),
            c: vec![self.c[o].clone()],
            d: vec![vec![self.d[o][i]]],
        }
    }
    fn d0(&self) -> f64 {
        self.d.first().and_then(|r| r.first()).cloned().unwrap_or(0.0)
    }
    /// G(s) of a single-input single-output system at a complex s.
    pub fn at(&self, s: Cx) -> Cx {
        let n = self.n();
        if n == 0 {
            return Cx::new(self.d0(), 0.0);
        }
        // Solve (sI − A) v = B, then C v + D.
        let mut m: Vec<Vec<Cx>> = (0..n).map(|i| (0..n).map(|j| Cx::new(if i == j { s.re } else { 0.0 } - self.a[i][j], if i == j { s.im } else { 0.0 })).collect()).collect();
        let mut v: Vec<Cx> = (0..n).map(|i| Cx::new(self.b[i][0], 0.0)).collect();
        if !csolve(&mut m, &mut v) {
            return Cx::new(f64::INFINITY, 0.0);
        }
        let mut y = Cx::new(self.d0(), 0.0);
        for i in 0..n {
            y = y.add(v[i].sc(self.c[0][i]));
        }
        y
    }
    pub fn poles(&self) -> Vec<Cx> {
        eig(&self.a)
    }
    /// Unity negative feedback around a SISO loop: T = L / (1 + L).
    pub fn feedback(&self) -> Result<Ss, String> {
        let d = self.d0();
        if (1.0 + d).abs() < 1e-12 {
            return Err("The loop has a direct gain of −1: the closed loop is not defined.".into());
        }
        let n = self.n();
        let k = 1.0 / (1.0 + d);
        let a = (0..n).map(|i| (0..n).map(|j| self.a[i][j] - self.b[i][0] * self.c[0][j] * k).collect()).collect();
        let b = (0..n).map(|i| vec![self.b[i][0] * k]).collect();
        let c = vec![(0..n).map(|j| self.c[0][j] * k).collect()];
        Ok(Ss { a, b, c, d: vec![vec![d * k]] })
    }
    /// Closed-loop poles with the loop gain multiplied by k.
    pub fn poles_at_gain(&self, k: f64) -> Vec<Cx> {
        let d = self.d0();
        let n = self.n();
        let den = 1.0 + k * d;
        if den.abs() < 1e-14 {
            return vec![];
        }
        let a: M = (0..n).map(|i| (0..n).map(|j| self.a[i][j] - k * self.b[i][0] * self.c[0][j] / den).collect()).collect();
        eig(&a)
    }
}

/// Complex Gaussian elimination with partial pivoting; false if singular.
fn csolve(m: &mut [Vec<Cx>], v: &mut [Cx]) -> bool {
    let n = v.len();
    for k in 0..n {
        let p = (k..n).max_by(|a, b| m[*a][k].abs().total_cmp(&m[*b][k].abs())).unwrap();
        if m[p][k].abs() < 1e-300 {
            return false;
        }
        m.swap(k, p);
        v.swap(k, p);
        for i in k + 1..n {
            let f = m[i][k].div(m[k][k]);
            if f.re == 0.0 && f.im == 0.0 {
                continue;
            }
            for j in k..n {
                m[i][j] = m[i][j].sub(f.mul(m[k][j]));
            }
            v[i] = v[i].sub(f.mul(v[k]));
        }
    }
    for k in (0..n).rev() {
        let mut s = v[k];
        for j in k + 1..n {
            s = s.sub(m[k][j].mul(v[j]));
        }
        v[k] = s.div(m[k][k]);
    }
    true
}

/// Eigenvalues of a real square matrix: reduction to Hessenberg form, then the shifted QR
/// algorithm (the classic elmhes/hqr pair). Sorted by real part, then imaginary part.
pub fn eig(a0: &M) -> Vec<Cx> {
    let n = a0.len();
    if n == 0 {
        return vec![];
    }
    let mut a = a0.clone();
    // Hessenberg by stabilised elementary similarity transforms.
    for m in 1..n.saturating_sub(1) {
        let (mut x, mut i) = (0.0f64, m);
        for j in m..n {
            if a[j][m - 1].abs() > x.abs() {
                x = a[j][m - 1];
                i = j;
            }
        }
        if i != m {
            for j in m - 1..n {
                let t = a[i][j];
                a[i][j] = a[m][j];
                a[m][j] = t;
            }
            for r in a.iter_mut() {
                r.swap(i, m);
            }
        }
        if x != 0.0 {
            for i in m + 1..n {
                let mut y = a[i][m - 1];
                if y != 0.0 {
                    y /= x;
                    a[i][m - 1] = y;
                    for j in m..n {
                        a[i][j] -= y * a[m][j];
                    }
                    for j in 0..n {
                        a[j][m] += y * a[j][i];
                    }
                }
            }
        }
    }
    for i in 2..n {
        for j in 0..i - 1 {
            a[i][j] = 0.0;
        }
    }
    let (mut wr, mut wi) = (vec![0.0; n], vec![0.0; n]);
    let mut anorm = 0.0;
    for i in 0..n {
        for j in i.saturating_sub(1)..n {
            anorm += a[i][j].abs();
        }
    }
    let mut nn = n as isize - 1;
    let mut t = 0.0;
    while nn >= 0 {
        let mut its = 0;
        loop {
            let u = nn as usize;
            let mut l = u;
            while l >= 1 {
                let mut s = a[l - 1][l - 1].abs() + a[l][l].abs();
                if s == 0.0 {
                    s = anorm;
                }
                if a[l][l - 1].abs() + s == s {
                    a[l][l - 1] = 0.0;
                    break;
                }
                l -= 1;
            }
            let mut x = a[u][u];
            if l == u {
                wr[u] = x + t;
                wi[u] = 0.0;
                nn -= 1;
                break;
            }
            let mut y = a[u - 1][u - 1];
            let mut w = a[u][u - 1] * a[u - 1][u];
            if l == u - 1 {
                let p = 0.5 * (y - x);
                let q = p * p + w;
                let z = q.abs().sqrt();
                x += t;
                if q >= 0.0 {
                    let z = p + z.copysign(p);
                    wr[u - 1] = x + z;
                    wr[u] = if z != 0.0 { x - w / z } else { x + z };
                    wi[u - 1] = 0.0;
                    wi[u] = 0.0;
                } else {
                    wr[u - 1] = x + p;
                    wr[u] = x + p;
                    wi[u - 1] = -z;
                    wi[u] = z;
                }
                nn -= 2;
                break;
            }
            if its == 60 {
                // Did not converge: return what the diagonal says (never happens on sane models).
                for i in 0..=u {
                    wr[i] = a[i][i] + t;
                }
                nn = -1;
                break;
            }
            if its == 10 || its == 20 {
                t += x;
                for i in 0..=u {
                    a[i][i] -= x;
                }
                let s = a[u][u - 1].abs() + a[u - 1][u - 2].abs();
                x = 0.75 * s;
                y = x;
                w = -0.4375 * s * s;
            }
            its += 1;
            let mut m = u - 2;
            let (mut p, mut q, mut r);
            loop {
                let z = a[m][m];
                let rr = x - z;
                let ss = y - z;
                p = (rr * ss - w) / a[m + 1][m] + a[m][m + 1];
                q = a[m + 1][m + 1] - z - rr - ss;
                r = a[m + 2][m + 1];
                let s = p.abs() + q.abs() + r.abs();
                p /= s;
                q /= s;
                r /= s;
                if m == l {
                    break;
                }
                let uu = a[m][m - 1].abs() * (q.abs() + r.abs());
                let vv = p.abs() * (a[m - 1][m - 1].abs() + z.abs() + a[m + 1][m + 1].abs());
                if uu + vv == vv {
                    break;
                }
                m -= 1;
            }
            for i in m + 2..=u {
                a[i][i - 2] = 0.0;
                if i != m + 2 {
                    a[i][i - 3] = 0.0;
                }
            }
            let mut k = m;
            while k < u {
                if k != m {
                    p = a[k][k - 1];
                    q = a[k + 1][k - 1];
                    r = if k + 1 != u { a[k + 2][k - 1] } else { 0.0 };
                    x = p.abs() + q.abs() + r.abs();
                    if x != 0.0 {
                        p /= x;
                        q /= x;
                        r /= x;
                    }
                }
                let s = (p * p + q * q + r * r).sqrt().copysign(p);
                if s != 0.0 {
                    if k == m {
                        if l != m {
                            a[k][k - 1] = -a[k][k - 1];
                        }
                    } else {
                        a[k][k - 1] = -s * x;
                    }
                    p += s;
                    x = p / s;
                    y = q / s;
                    let z = r / s;
                    q /= p;
                    r /= p;
                    for j in k..=u {
                        let mut pp = a[k][j] + q * a[k + 1][j];
                        if k + 1 != u {
                            pp += r * a[k + 2][j];
                            a[k + 2][j] -= pp * z;
                        }
                        a[k + 1][j] -= pp * y;
                        a[k][j] -= pp * x;
                    }
                    let mmin = if u < k + 3 { u } else { k + 3 };
                    for i in l..=mmin {
                        let mut pp = x * a[i][k] + y * a[i][k + 1];
                        if k + 1 != u {
                            pp += z * a[i][k + 2];
                            a[i][k + 2] -= pp * r;
                        }
                        a[i][k + 1] -= pp * q;
                        a[i][k] -= pp;
                    }
                }
                k += 1;
            }
        }
    }
    let mut v: Vec<Cx> = (0..n).map(|i| Cx::new(wr[i], wi[i])).collect();
    v.sort_by(|a, b| a.re.total_cmp(&b.re).then(a.im.total_cmp(&b.im)));
    v
}

/// Frequencies two decades either side of the poles (rad/s), log-spaced.
pub fn grid(sys: &Ss, pts: usize) -> Vec<f64> {
    let mags: Vec<f64> = sys.poles().iter().map(|p| p.abs()).filter(|m| *m > 1e-9 && m.is_finite()).collect();
    let lo = mags.iter().cloned().fold(f64::INFINITY, f64::min);
    let hi = mags.iter().cloned().fold(0.0, f64::max);
    let (lo, hi) = if mags.is_empty() { (1e-2, 1e2) } else { (lo / 100.0, hi * 100.0) };
    let (l0, l1) = (lo.log10(), hi.log10().max(lo.log10() + 1.0));
    (0..pts).map(|i| 10f64.powf(l0 + (l1 - l0) * i as f64 / (pts - 1) as f64)).collect()
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Bode {
    pub w: Vec<f64>,
    pub mag_db: Vec<f64>,
    pub phase: Vec<f64>,
}

/// Magnitude (dB) and unwrapped phase (degrees), starting within (−360°, 0°] where possible.
pub fn bode(sys: &Ss, w: &[f64]) -> Bode {
    let mut b = Bode { w: w.to_vec(), ..Default::default() };
    let mut last = f64::NAN;
    for &wi in w {
        let g = sys.at(Cx::new(0.0, wi));
        b.mag_db.push(20.0 * g.abs().log10());
        let mut ph = g.arg().to_degrees();
        if last.is_finite() {
            while ph - last > 180.0 {
                ph -= 360.0;
            }
            while ph - last < -180.0 {
                ph += 360.0;
            }
        } else if ph > 0.0 && sys.n() > 0 {
            // Most loops start with phase at or below 0°.
            ph -= 360.0 * (ph / 360.0).ceil();
            if ph <= -360.0 {
                ph += 360.0;
            }
        }
        b.phase.push(ph);
        last = ph;
    }
    b
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Margins {
    /// Gain margin (as a factor), at the phase crossover wpc; infinite if the phase never crosses.
    pub gm: f64,
    pub gm_db: f64,
    pub wpc: f64,
    /// Phase margin (degrees) at the gain crossover wgc; infinite if |L| never crosses 1.
    pub pm: f64,
    pub wgc: f64,
}

/// Gain and phase margins of a loop L: the smallest of each over the crossovers.
pub fn margins(l: &Ss) -> Margins {
    let w = grid(l, 3000);
    let b = bode(l, &w);
    let mut m = Margins { gm: f64::INFINITY, gm_db: f64::INFINITY, wpc: f64::NAN, pm: f64::INFINITY, wgc: f64::NAN };
    let refine = |f: &dyn Fn(f64) -> f64, mut a: f64, mut c: f64| {
        let fa = f(a);
        for _ in 0..60 {
            let mid = (a * c).sqrt();
            if (f(mid) > 0.0) == (fa > 0.0) {
                a = mid;
            } else {
                c = mid;
            }
        }
        (a * c).sqrt()
    };
    for i in 1..w.len() {
        // Gain crossover: |L| through 0 dB.
        let (a, c) = (b.mag_db[i - 1], b.mag_db[i]);
        if a.is_finite() && c.is_finite() && (a > 0.0) != (c > 0.0) {
            let wc = refine(&|x| 20.0 * l.at(Cx::new(0.0, x)).abs().log10(), w[i - 1], w[i]);
            let ph = b.phase[i - 1] + (b.phase[i] - b.phase[i - 1]) * ((wc / w[i - 1]).ln() / (w[i] / w[i - 1]).ln());
            let pm = 180.0 + ph - 360.0 * ((180.0 + ph) / 360.0 + 0.5).floor();
            if pm.abs() < m.pm.abs() || !m.pm.is_finite() {
                m.pm = pm;
                m.wgc = wc;
            }
        }
        // Phase crossover: the phase through −180° (mod 360°).
        let (pa, pc) = (b.phase[i - 1], b.phase[i]);
        let k = ((pa.max(pc) + 180.0) / 360.0).floor();
        let lvl = -180.0 + 360.0 * k;
        if (pa - lvl) * (pc - lvl) < 0.0 {
            let x = w[i - 1] * (w[i] / w[i - 1]).powf((lvl - pa) / (pc - pa));
            let mag = l.at(Cx::new(0.0, x)).abs();
            let gm = 1.0 / mag;
            if gm.ln().abs() < m.gm.ln().abs() || !m.gm.is_finite() {
                m.gm = gm;
                m.gm_db = 20.0 * gm.log10();
                m.wpc = x;
            }
        }
    }
    m
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Response {
    pub t: Vec<f64>,
    pub y: Vec<f64>,
}

/// e^(A h) by scaling and squaring with a Taylor series.
pub fn expm(a: &M, h: f64) -> M {
    let n = a.len();
    let norm = a.iter().map(|r| r.iter().map(|v| v.abs()).sum::<f64>()).fold(0.0, f64::max) * h.abs();
    let s = if norm > 0.5 { (norm / 0.5).log2().ceil() as i32 } else { 0 };
    let k = h / 2f64.powi(s);
    let id: M = (0..n).map(|i| (0..n).map(|j| if i == j { 1.0 } else { 0.0 }).collect()).collect();
    let mut e = id.clone();
    let mut term = id;
    for q in 1..20 {
        term = mm(&term, a).into_iter().map(|r| r.into_iter().map(|v| v * k / q as f64).collect()).collect();
        for i in 0..n {
            for j in 0..n {
                e[i][j] += term[i][j];
            }
        }
    }
    for _ in 0..s {
        e = mm(&e, &e);
    }
    e
}

fn mm(a: &M, b: &M) -> M {
    let (n, p, q) = (a.len(), b.len(), b.first().map_or(0, |r| r.len()));
    (0..n).map(|i| (0..q).map(|j| (0..p).map(|k| a[i][k] * b[k][j]).sum()).collect()).collect()
}

/// Step (impulse = false) or impulse response over [0, tf], exact for a piecewise-constant input.
pub fn response(sys: &Ss, tf: f64, steps: usize, impulse: bool) -> Response {
    let n = sys.n();
    let h = tf / steps as f64;
    // Discretise [A B; 0 0] in one exponential: Ad and Bd = ∫ e^(As) ds B.
    let mut big: M = vec![vec![0.0; n + 1]; n + 1];
    for i in 0..n {
        big[i][..n].copy_from_slice(&sys.a[i]);
        big[i][n] = sys.b[i][0];
    }
    let e = expm(&big, h);
    let mut x: Vec<f64> = if impulse { (0..n).map(|i| sys.b[i][0]).collect() } else { vec![0.0; n] };
    let u = if impulse { 0.0 } else { 1.0 };
    let mut r = Response::default();
    for k in 0..=steps {
        let y: f64 = (0..n).map(|i| sys.c[0][i] * x[i]).sum::<f64>() + if impulse { 0.0 } else { sys.d0() };
        r.t.push(k as f64 * h);
        r.y.push(y);
        let xn: Vec<f64> = (0..n).map(|i| (0..n).map(|j| e[i][j] * x[j]).sum::<f64>() + e[i][n] * u).collect();
        x = xn;
    }
    r
}

/// A time span that shows the slowest stable mode settle (8 time constants), within limits.
pub fn span(sys: &Ss) -> f64 {
    let slow = sys.poles().iter().filter(|p| p.re < -1e-9).map(|p| -p.re).fold(f64::INFINITY, f64::min);
    let osc = sys.poles().iter().map(|p| p.im.abs()).filter(|v| *v > 1e-9).fold(0.0, f64::max);
    let mut t = if slow.is_finite() { 8.0 / slow } else { 10.0 };
    if osc > 0.0 {
        t = t.max(6.0 * std::f64::consts::PI / osc);
    }
    t.clamp(1e-6, 1e6)
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Loop {
    /// Open-loop poles in the right half-plane, closed-loop poles there, and the encirclements
    /// of −1 (clockwise) that the Nyquist criterion implies: N = Z − P.
    pub p_rhp: usize,
    pub z_rhp: usize,
    pub encirclements: i64,
    pub stable: bool,
    pub nyq_re: Vec<f64>,
    pub nyq_im: Vec<f64>,
    pub s_db: Vec<f64>,
    pub t_db: Vec<f64>,
    pub ms: f64,
    pub w_ms: f64,
    /// Root locus: gains and the closed-loop poles at each.
    pub rl_k: Vec<f64>,
    pub rl_poles: Vec<Vec<[f64; 2]>>,
}

fn rhp(p: &[Cx]) -> usize {
    let tol = 1e-9 * p.iter().map(|x| x.abs()).fold(1.0, f64::max);
    p.iter().filter(|x| x.re > tol).count()
}

/// Nyquist, sensitivity and root locus of a SISO loop L (negative feedback).
pub fn loop_analysis(l: &Ss, w: &[f64]) -> Loop {
    let mut r = Loop { ms: 0.0, ..Default::default() };
    r.p_rhp = rhp(&l.poles());
    r.z_rhp = rhp(&l.poles_at_gain(1.0));
    r.encirclements = r.z_rhp as i64 - r.p_rhp as i64;
    r.stable = r.z_rhp == 0 && l.poles_at_gain(1.0).iter().all(|p| p.re < 0.0);
    for &wi in w {
        let g = l.at(Cx::new(0.0, wi));
        r.nyq_re.push(g.re);
        r.nyq_im.push(g.im);
        let one = Cx::new(1.0, 0.0);
        let s = one.div(one.add(g));
        let t = g.div(one.add(g));
        r.s_db.push(20.0 * s.abs().log10());
        r.t_db.push(20.0 * t.abs().log10());
        if s.abs() > r.ms {
            r.ms = s.abs();
            r.w_ms = wi;
        }
    }
    let n = 160;
    for i in 0..=n {
        let k = 10f64.powf(-3.0 + 6.0 * i as f64 / n as f64);
        r.rl_k.push(k);
        r.rl_poles.push(l.poles_at_gain(k).into_iter().map(|p| [p.re, p.im]).collect());
    }
    r
}

// ---------------------------------------------------------------- PID design

#[derive(Clone, Copy, Debug, Default, Serialize, PartialEq)]
pub struct Pid {
    pub p: f64,
    pub i: f64,
    pub d: f64,
    pub n: f64,
}

impl Pid {
    /// The controller as a state space (parallel form with the derivative filter N).
    pub fn ss(&self) -> Ss {
        let Pid { p, i, d, n } = *self;
        if d != 0.0 {
            // (p + d n) s² + (p n + i) s + i n over s² + n s.
            tf(&[p + d * n, p * n + i, i * n], &[1.0, n, 0.0])
        } else if i != 0.0 {
            tf(&[p, i], &[1.0, 0.0])
        } else {
            Ss::gain(p)
        }
    }
}

/// Controllable canonical form of num/den (highest power first, proper).
pub fn tf(num: &[f64], den: &[f64]) -> Ss {
    let a0 = den[0];
    let den: Vec<f64> = den.iter().map(|v| v / a0).collect();
    let n = den.len() - 1;
    let mut nm = vec![0.0; n + 1 - num.len().min(n + 1)];
    nm.extend(num.iter().map(|v| v / a0));
    let d = nm[0];
    let a: M = (0..n).map(|i| (0..n).map(|j| if i == 0 { -den[j + 1] } else if j + 1 == i { 1.0 } else { 0.0 }).collect()).collect();
    let b: M = (0..n).map(|i| vec![if i == 0 { 1.0 } else { 0.0 }]).collect();
    let c = vec![(0..n).map(|j| nm[j + 1] - d * den[j + 1]).collect()];
    Ss { a, b, c, d: vec![vec![d]] }
}

/// Series connection: first `a`, then `b` (SISO).
pub fn series(a: &Ss, b: &Ss) -> Ss {
    let (na, nb) = (a.n(), b.n());
    let n = na + nb;
    let mut am: M = vec![vec![0.0; n]; n];
    for i in 0..na {
        am[i][..na].copy_from_slice(&a.a[i]);
    }
    for i in 0..nb {
        for j in 0..na {
            am[na + i][j] = b.b[i][0] * a.c[0][j];
        }
        am[na + i][na..].copy_from_slice(&b.a[i]);
    }
    let bm: M = (0..n).map(|i| vec![if i < na { a.b[i][0] } else { b.b[i - na][0] * a.d0() }]).collect();
    let cm = vec![(0..n).map(|j| if j < na { b.d0() * a.c[0][j] } else { b.c[0][j - na] }).collect()];
    Ss { a: am, b: bm, c: cm, d: vec![vec![a.d0() * b.d0()]] }
}

#[derive(Clone, Debug, Serialize)]
pub struct Design {
    pub pid: Pid,
    pub rule: String,
    pub margins: Margins,
    pub note: String,
}

/// Loop shaping: gains that put the crossover of C·G at `wc` with phase margin `pm` (degrees).
/// A PI when the plant leaves room for it, else a PID with Td = Ti/4; N = 10·wc.
pub fn design(g: &Ss, wc: f64, pm: f64, form: &str) -> Result<Design, String> {
    if !(wc > 0.0) {
        return Err("The bandwidth must be positive.".into());
    }
    let gj = g.at(Cx::new(0.0, wc));
    if !(gj.abs() > 0.0) || !gj.abs().is_finite() {
        return Err("The plant has no gain at that frequency: choose another bandwidth.".into());
    }
    // Phase the controller must add at wc, in (−180°, 180°].
    let mut need = -180.0 + pm - gj.arg().to_degrees();
    need -= 360.0 * ((need + 180.0) / 360.0).floor();
    if need > 180.0 {
        need -= 360.0;
    }
    let n = 10.0 * wc;
    let mut note = String::new();
    let use_pi = form == "PI" || (form != "PID" && need < -1.0 && need > -89.0);
    let shape = |ph: f64| {
        let mut pid = Pid { n, ..Default::default() };
        if use_pi {
            let ti = 1.0 / (wc * (-ph.clamp(-89.0, -1.0).to_radians()).tan());
            pid.p = 1.0;
            pid.i = 1.0 / ti;
            pid.n = 0.0;
        } else {
            // Ideal PID with Td = Ti/4: phase = atan(a/4 − 1/a) with a = wc·Ti.
            let tn = ph.clamp(-85.0, 85.0).to_radians().tan();
            let a = 2.0 * tn + 2.0 * (tn * tn + 1.0).sqrt();
            let ti = a / wc;
            pid.p = 1.0;
            pid.i = 1.0 / ti;
            pid.d = ti / 4.0;
        }
        pid
    };
    // The derivative filter takes some phase: correct the target until the real phase matches.
    let mut cmd = need;
    let mut pid = shape(cmd);
    for _ in 0..12 {
        let got = pid.ss().at(Cx::new(0.0, wc)).arg().to_degrees();
        cmd += need - got;
        pid = shape(cmd);
    }
    if use_pi && form == "PI" && (need >= -1.0 || need <= -89.0) {
        note = "A PI controller cannot give that phase margin at that bandwidth; the nearest PI is shown.".into();
    }
    if !use_pi && cmd.abs() > 85.0 {
        note = "The plant's phase at that bandwidth needs more than a PID can give; the margin will be lower than asked.".into();
    }
    // Scale the gain so |C G| = 1 at wc.
    let c = pid.ss().at(Cx::new(0.0, wc));
    let k = 1.0 / (c.abs() * gj.abs());
    pid.p *= k;
    pid.i *= k;
    pid.d *= k;
    let m = margins(&series(&pid.ss(), g));
    Ok(Design { pid, rule: if pid.d != 0.0 { "PID by loop shaping".into() } else { "PI by loop shaping".into() }, margins: m, note })
}

/// Ziegler–Nichols (frequency response): from the plant's ultimate gain and period.
pub fn ziegler_nichols(g: &Ss) -> Result<Design, String> {
    let m = margins(g);
    if !m.gm.is_finite() || !(m.wpc > 0.0) {
        return Err("Ziegler–Nichols needs a plant whose phase reaches −180° (a third-order or delayed plant); this one never does.".into());
    }
    let ku = m.gm;
    let pu = 2.0 * std::f64::consts::PI / m.wpc;
    let p = 0.6 * ku;
    let (ti, td) = (pu / 2.0, pu / 8.0);
    let pid = Pid { p, i: p / ti, d: p * td, n: 20.0 / td.max(1e-12) };
    Ok(Design { pid, rule: format!("Ziegler–Nichols: Ku {:.4}, Pu {:.4} s", ku, pu), margins: margins(&series(&pid.ss(), g)), note: "Classic rule, aimed at quarter-amplitude decay: expect overshoot.".into() })
}

/// SIMC (Skogestad) PI from a first-order-plus-dead-time fit of the plant's step response.
pub fn simc(g: &Ss) -> Result<Design, String> {
    let tf = span(g);
    let r = response(g, tf, 2000, false);
    let k = g.at(Cx::new(0.0, 0.0)).re;
    if !k.is_finite() || k.abs() < 1e-12 || g.poles().iter().any(|p| p.re > -1e-9) {
        return Err("SIMC needs a stable plant with a non-zero steady-state gain.".into());
    }
    let at = |f: f64| {
        let i = r.y.iter().position(|y| y / k >= f).unwrap_or(r.y.len() - 1).max(1);
        let (y0, y1) = (r.y[i - 1] / k, r.y[i] / k);
        r.t[i - 1] + (r.t[i] - r.t[i - 1]) * ((f - y0) / (y1 - y0)).clamp(0.0, 1.0)
    };
    // Two-point fit: 28 % and 63 % of the final value.
    let (t28, t63) = (at(0.283), at(0.632));
    let tau = (1.5 * (t63 - t28)).max(1e-9);
    let theta = (t63 - tau).max(0.0);
    let tc = theta.max(0.1 * tau);
    let kc = tau / (k * (tc + theta));
    let ti = tau.min(4.0 * (tc + theta));
    let pid = Pid { p: kc, i: kc / ti, d: 0.0, n: 0.0 };
    Ok(Design { pid, rule: format!("SIMC PI: K {:.4}, τ {:.4} s, θ {:.4} s", k, tau, theta), margins: margins(&series(&pid.ss(), g)), note: String::new() })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol * (1.0 + b.abs())
    }

    /// Eigenvalues of a companion matrix are the roots of its polynomial.
    #[test]
    fn eigenvalues() {
        // (s+1)(s+2)(s²+2s+5) = s⁴ + 5s³ + 13s² + 19s + 10
        let s = tf(&[1.0], &[1.0, 5.0, 13.0, 19.0, 10.0]);
        let p = s.poles();
        let want = [(-2.0, 0.0), (-1.0, -2.0), (-1.0, 0.0), (-1.0, 2.0)];
        for w in want {
            assert!(p.iter().any(|q| close(q.re, w.0, 1e-9) && close(q.im, w.1, 1e-9)), "{p:?}");
        }
        // A non-symmetric 5×5 with known eigenvalues: triangular after a similarity.
        let a = vec![vec![2.0, 1.0, 0.0, 0.0, 3.0], vec![0.0, -1.0, 4.0, 0.0, 0.0], vec![0.0, 0.0, -3.0, 2.0, 0.0], vec![0.0, 0.0, 0.0, 5.0, 1.0], vec![0.0, 0.0, 0.0, 0.0, -7.0]];
        let e: Vec<f64> = eig(&a).iter().map(|c| c.re).collect();
        assert!(e.iter().zip([-7.0, -3.0, -1.0, 2.0, 5.0]).all(|(x, y)| close(*x, y, 1e-9)), "{e:?}");
    }

    /// G = 1/(s+1)³ under unity feedback: phase −180° at √3 rad/s, gain margin 8 (18.06 dB);
    /// gain crossover at 0 dB never happens at gain 1 (|G(0)| = 1), so take K = 4.
    #[test]
    fn margins_of_a_textbook_loop() {
        let g = tf(&[4.0], &[1.0, 3.0, 3.0, 1.0]);
        let m = margins(&g);
        assert!(close(m.wpc, 3f64.sqrt(), 1e-6) && close(m.gm, 2.0, 1e-6), "{m:?}");
        // |G| = 1 where (1+w²)^{3/2} = 4 → w = √(4^{2/3} − 1); phase = −3·atan(w).
        let wgc = (4f64.powf(2.0 / 3.0) - 1.0).sqrt();
        assert!(close(m.wgc, wgc, 1e-6) && close(m.pm, 180.0 - 3.0 * wgc.atan().to_degrees(), 1e-6), "{m:?}");
        // Closed loop stable (K = 4 < 8), and the Nyquist count agrees.
        let l = loop_analysis(&g, &grid(&g, 400));
        assert!(l.stable && l.encirclements == 0 && l.p_rhp == 0);
        let g10 = tf(&[10.0], &[1.0, 3.0, 3.0, 1.0]);
        let l = loop_analysis(&g10, &grid(&g10, 400));
        assert!(!l.stable && l.z_rhp == 2 && l.encirclements == 2);
    }

    /// Step response of 1/(s+1) is 1 − e^(−t); closed loop of 1/s is 1/(s+1); impulse of 1/(s+1) is e^(−t).
    #[test]
    fn responses() {
        let g = tf(&[1.0], &[1.0, 1.0]);
        let r = response(&g, 5.0, 500, false);
        assert!(r.t.iter().zip(&r.y).all(|(t, y)| close(*y, 1.0 - (-t).exp(), 1e-9)));
        let cl = tf(&[1.0], &[1.0, 0.0]).feedback().unwrap();
        let r = response(&cl, 5.0, 100, false);
        assert!(r.t.iter().zip(&r.y).all(|(t, y)| close(*y, 1.0 - (-t).exp(), 1e-9)));
        let r = response(&g, 5.0, 100, true);
        assert!(r.t.iter().zip(&r.y).all(|(t, y)| close(*y, (-t).exp(), 1e-9)));
        // Sensitivity peak of L = 1/s is 1 (|S| = w/√(1+w²) < 1).
        let l = loop_analysis(&tf(&[1.0], &[1.0, 0.0]), &grid(&g, 200));
        assert!(l.ms <= 1.0 && l.ms > 0.99);
    }

    /// Loop shaping meets its targets: crossover and phase margin as asked, on a DC motor plant
    /// (PI) and a double integrator (PID).
    #[test]
    fn pid_design_meets_targets() {
        let motor = tf(&[2.0], &[0.5, 1.5, 1.0]); // 2/((0.5s+1)(s+1))
        let d = design(&motor, 5.0, 60.0, "auto").unwrap();
        assert!(close(d.margins.wgc, 5.0, 1e-4) && close(d.margins.pm, 60.0, 1e-3), "{d:?}");
        let d = design(&motor, 1.0, 60.0, "auto").unwrap();
        assert!(d.pid.d == 0.0 && close(d.margins.pm, 60.0, 1e-3), "{d:?}");
        let dint = tf(&[1.0], &[1.0, 0.0, 0.0]);
        let d = design(&dint, 2.0, 45.0, "auto").unwrap();
        assert!(d.pid.d > 0.0, "{d:?}");
        // The filter N moves the phase a little: within a few degrees of the target.
        assert!(close(d.margins.wgc, 2.0, 1e-4) && close(d.margins.pm, 45.0, 1e-3), "{d:?}");
        // Ziegler–Nichols on 1/(s+1)³: Ku = 8, Pu = 2π/√3.
        let zn = ziegler_nichols(&tf(&[1.0], &[1.0, 3.0, 3.0, 1.0])).unwrap();
        assert!(close(zn.pid.p, 4.8, 1e-5) && close(zn.pid.i, 4.8 / (std::f64::consts::PI / 3f64.sqrt()), 1e-5), "{zn:?}");
        // SIMC on a first-order plant 2/(3s+1): K 2, τ 3, θ ≈ 0.
        let s = simc(&tf(&[2.0], &[3.0, 1.0])).unwrap();
        assert!(s.rule.contains("K 2.0000, τ 3.00") && s.margins.pm > 60.0, "{s:?}");
    }

    #[test]
    fn series_matches_product() {
        let a = tf(&[1.0], &[1.0, 1.0]);
        let b = tf(&[2.0, 1.0], &[1.0, 3.0]);
        let s = series(&a, &b);
        let w = Cx::new(0.0, 1.7);
        let x = a.at(w).mul(b.at(w));
        let y = s.at(w);
        assert!(close(x.re, y.re, 1e-12) && close(x.im, y.im, 1e-12));
    }
}
