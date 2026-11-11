//! Physical components (stage C, 0.5.1 on): parts connected by undirected ports. A node shares
//! one across value (speed, velocity, voltage, temperature, pressure); the through values flowing
//! into it from its parts add to zero (torque, force, current, heat flow, volume flow).
//!
//! Each connected group of parts becomes one network, run as one block of the signal engine:
//! - rigid links (gears, rack and pinion, fixed frames) merge nodes into groups whose potentials
//!   are in fixed ratio, so inertias joined rigidly are one state (no index problems);
//! - a group with capacitance (inertia, mass, capacitor to ground, heat capacity, fluid volume)
//!   has its potential as a state: C·φ̇ = −Σ flows;
//! - other groups' potentials, and the flows of constraint parts (sources of an across value,
//!   transducers, floating capacitors), are solved at each evaluation by Newton's method;
//! - inductors and springs keep their flow as a state.

use crate::rt::{Ctx, Rate, Rt};

pub const GROUND: usize = usize::MAX;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum Dom {
    Rot,
    Trans,
    Elec,
    Therm,
    Hyd,
}

impl Dom {
    pub fn name(self) -> &'static str {
        match self {
            Dom::Rot => "rotational",
            Dom::Trans => "translational",
            Dom::Elec => "electrical",
            Dom::Therm => "thermal",
            Dom::Hyd => "hydraulic",
        }
    }
}

/// How a resistive part's flow follows its across difference.
#[derive(Clone, Debug)]
pub enum Law {
    /// f = g·Δe
    Lin(f64),
    /// Smooth friction: f = (Fc + (Fs − Fc)·e^{−(Δe/ws)²})·tanh(Δe/(ws/10)) + c·Δe
    Friction { fc: f64, fs: f64, ws: f64, c: f64 },
    /// f = k·Δe/√(|Δe| + δ): an orifice (turbulent, regularised near zero).
    Orifice { k: f64, d: f64 },
    /// Radiation between two temperatures in °C: f = k·((Ta+273.15)⁴ − (Tb+273.15)⁴)
    Rad(f64),
    /// An ideal diode with an on-resistance and an off-conductance: on while it conducts.
    Diode { ron: f64, goff: f64, vf: f64 },
    /// A switch closed while signal input `u` ≥ 0.5.
    Switch { ron: f64, goff: f64, u: usize },
    /// A valve: f = k·u·Δe/√(|Δe| + δ), with opening u (0…1) from a signal.
    Valve { k: f64, d: f64, u: usize },
    /// A relief valve: opens linearly above the set difference.
    Relief { pset: f64, g: f64 },
}

/// The primitive elements a part is made of. Nodes are network node indices (or GROUND).
#[derive(Clone, Debug)]
pub enum Prim {
    /// Capacitance from a node to ground (inertia, mass, capacitor to ground, heat capacity,
    /// volume), with the initial potential when it sets one.
    Cap { n: usize, c: f64, init: Option<f64> },
    /// A capacitor between two nodes: state Δe (x), flow j (unknown).
    CapF { a: usize, b: usize, c: f64, x: usize, j: usize },
    /// An inductor or spring: state flow (x); ẋ = Δe/L.
    Ind { a: usize, b: usize, l: f64, x: usize },
    Res { a: usize, b: usize, law: Law, mode: usize },
    /// A transducer: (e_a − e_b) = k·(e_c − e_d), flow j at a→b, −k·j at c→d.
    Tdx { a: usize, b: usize, c: usize, d: usize, k: f64, j: usize },
    /// An across source: Δe = u·scale, flow j.
    ASrc { a: usize, b: usize, u: Option<usize>, v: f64, j: usize },
    /// A through source: injects u (or v) from b into a.
    FSrc { a: usize, b: usize, u: Option<usize>, v: f64 },
    /// A gyrator across domains: f_ab = k·(e_c − e_d), f_cd = −k·(e_a − e_b).
    Gyr { a: usize, b: usize, c: usize, d: usize, k: f64 },
    /// A sensor of Δe (and of its integral, when `x` is set).
    MeasA { a: usize, b: usize, y: usize, x: Option<usize>, yx: usize },
    /// A series sensor of flow: Δe = 0, flow j measured.
    MeasF { a: usize, b: usize, y: usize, j: usize },
    /// Backlash: state relative position (x); flow k·dz(x) + c·Δe while in contact.
    Lash { a: usize, b: usize, k: f64, c: f64, gap: f64, x: usize },
    /// A hard stop on a mass: x is its position; pushes back beyond [lo, hi].
    Stop { a: usize, k: f64, c: f64, lo: f64, hi: f64, x: usize },
    /// Integrates a node's potential (a position from a velocity) for parts that need it.
    Pos { a: usize, x: usize, x0: f64 },
}

/// A rigid link: e_a = r·e_b (with efficiency η weighting a's contribution when merged).
#[derive(Clone, Debug)]
pub struct Link {
    pub a: usize,
    pub b: usize,
    pub r: f64,
    pub eta: f64,
    pub what: String,
}

#[derive(Clone)]
pub struct Network {
    pub prims: Vec<Prim>,
    pub nnodes: usize,
    /// Per node: (group, scale s, weight w): e = s·φ_group.
    node: Vec<(usize, f64, f64)>,
    /// Per group: capacitance (0 = algebraic), grounded, state index, unknown index.
    gcap: Vec<f64>,
    gground: Vec<bool>,
    gstate: Vec<Option<usize>>,
    gunk: Vec<Option<usize>>,
    /// States: group potentials first, then element states.
    pub nx: usize,
    pub x0: Vec<f64>,
    /// Unknowns: algebraic group potentials, then constraint flows.
    nz: usize,
    nj: usize,
    pub nu: usize,
    pub ny: usize,
    /// Modes of diodes (on = true), by `mode` index.
    modes: Vec<bool>,
    z0: Vec<f64>,
}

#[derive(Clone)]
struct Sol {
    e: Vec<f64>,
    j: Vec<f64>,
    ok: bool,
}

fn de(e: &[f64], a: usize, b: usize) -> f64 {
    let g = |n: usize| if n == GROUND { 0.0 } else { e[n] };
    g(a) - g(b)
}

impl Network {
    /// Build from primitives (element state and flow indices already numbered from 0 within
    /// their kinds: `nelx` element states, `nj` constraint flows, `nmodes` modes) and links.
    pub fn new(prims: Vec<Prim>, links: Vec<Link>, nnodes: usize, nelx: usize, nj: usize, nmodes: usize, nu: usize, ny: usize, elx0: Vec<f64>) -> Result<Network, String> {
        // Weighted union–find: parent, scale (e_n = s·e_parent), weight.
        let mut par: Vec<usize> = (0..nnodes).collect();
        let mut sc = vec![1.0; nnodes];
        let mut wt = vec![1.0; nnodes];
        let mut grounded = vec![false; nnodes];
        fn find(par: &mut Vec<usize>, sc: &mut Vec<f64>, wt: &mut Vec<f64>, n: usize) -> (usize, f64, f64) {
            if par[n] == n {
                return (n, 1.0, 1.0);
            }
            let p = par[n];
            let (r, s, w) = find(par, sc, wt, p);
            par[n] = r;
            sc[n] *= s;
            wt[n] *= w;
            (r, sc[n], wt[n])
        }
        for l in &links {
            if l.b == GROUND || l.a == GROUND {
                let n = if l.a == GROUND { l.b } else { l.a };
                if n != GROUND {
                    let (r, _, _) = find(&mut par, &mut sc, &mut wt, n);
                    grounded[r] = true;
                }
                continue;
            }
            let (ra, sa, wa) = find(&mut par, &mut sc, &mut wt, l.a);
            let (rb, sb, wb) = find(&mut par, &mut sc, &mut wt, l.b);
            if ra == rb {
                // Already linked: consistent only with the same ratio.
                if (sa - l.r * sb).abs() > 1e-9 * (1.0 + sa.abs()) {
                    return Err(format!("{}: it closes a loop of rigid connections with a different ratio (the parts would lock)", l.what));
                }
                continue;
            }
            // e_a = r e_b: sa φa = r sb φb → φa = (r sb / sa) φb. Hang ra under rb.
            par[ra] = rb;
            sc[ra] = l.r * sb / sa;
            wt[ra] = l.eta * wb / wa;
            grounded[rb] = grounded[rb] || grounded[ra];
        }
        let mut groups: Vec<usize> = vec![];
        let mut node = vec![(0usize, 1.0, 1.0); nnodes];
        for n in 0..nnodes {
            let (r, s, w) = find(&mut par, &mut sc, &mut wt, n);
            let g = match groups.iter().position(|x| *x == r) {
                Some(g) => g,
                None => {
                    groups.push(r);
                    groups.len() - 1
                }
            };
            node[n] = (g, s, w);
        }
        let ng = groups.len();
        let gground: Vec<bool> = groups.iter().map(|r| grounded[*r]).collect();
        let mut gcap = vec![0.0; ng];
        let mut ginit: Vec<Option<f64>> = vec![None; ng];
        for p in &prims {
            if let Prim::Cap { n, c, init } = p {
                if *n == GROUND {
                    continue;
                }
                let (g, s, w) = node[*n];
                gcap[g] += w * s * s * c;
                if ginit[g].is_none() {
                    ginit[g] = init.map(|v| v / s);
                }
            }
        }
        let mut gstate = vec![None; ng];
        let mut gunk = vec![None; ng];
        let (mut nx, mut nz) = (0, 0);
        let mut x0 = vec![];
        for g in 0..ng {
            if gground[g] {
                continue;
            }
            if gcap[g] > 0.0 {
                gstate[g] = Some(nx);
                x0.push(ginit[g].unwrap_or(0.0));
                nx += 1;
            } else {
                gunk[g] = Some(nz);
                nz += 1;
            }
        }
        // Element states after the group states.
        let shift = nx;
        let prims: Vec<Prim> = prims.into_iter().map(|mut p| {
            match &mut p {
                Prim::CapF { x, .. } | Prim::Ind { x, .. } | Prim::Lash { x, .. } | Prim::Stop { x, .. } | Prim::Pos { x, .. } => *x += shift,
                Prim::MeasA { x: Some(x), .. } => *x += shift,
                _ => {}
            }
            p
        }).collect();
        x0.extend(elx0);
        let nx = shift + nelx;
        let z0 = vec![0.0; nz + nj];
        Ok(Network { prims, nnodes, node, gcap, gground, gstate, gunk, nx, x0, nz, nj, nu, ny, modes: vec![false; nmodes], z0 })
    }

    fn potentials(&self, x: &[f64], z: &[f64]) -> Vec<f64> {
        (0..self.nnodes).map(|n| {
            let (g, s, _) = self.node[n];
            let phi = if self.gground[g] { 0.0 } else if let Some(k) = self.gstate[g] { x[k] } else { z[self.gunk[g].unwrap()] };
            s * phi
        }).collect()
    }

    /// Flows into elements at each node, and the constraint residuals.
    fn flows(&self, t: f64, x: &[f64], e: &[f64], j: &[f64], u: &[f64], modes: &[bool]) -> (Vec<f64>, Vec<f64>) {
        let _ = t;
        let mut f = vec![0.0; self.nnodes];
        let mut r = vec![0.0; self.nj];
        let mut add = |n: usize, v: f64| {
            if n != GROUND {
                f[n] += v;
            }
        };
        for p in &self.prims {
            match p {
                Prim::Cap { .. } | Prim::Pos { .. } | Prim::MeasA { .. } => {}
                Prim::CapF { a, b, x: xi, j: ji, .. } => {
                    add(*a, j[*ji]);
                    add(*b, -j[*ji]);
                    r[*ji] = de(e, *a, *b) - x[*xi];
                }
                Prim::Ind { a, b, x: xi, .. } => {
                    add(*a, x[*xi]);
                    add(*b, -x[*xi]);
                }
                Prim::Res { a, b, law, mode } => {
                    let v = res_flow(law, de(e, *a, *b), e, *a, *b, u, modes.get(*mode).cloned().unwrap_or(false));
                    add(*a, v);
                    add(*b, -v);
                }
                Prim::Tdx { a, b, c, d, k, j: ji } => {
                    add(*a, j[*ji]);
                    add(*b, -j[*ji]);
                    add(*c, -k * j[*ji]);
                    add(*d, k * j[*ji]);
                    r[*ji] = de(e, *a, *b) - k * de(e, *c, *d);
                }
                Prim::ASrc { a, b, u: ui, v, j: ji } => {
                    add(*a, j[*ji]);
                    add(*b, -j[*ji]);
                    r[*ji] = de(e, *a, *b) - ui.map_or(*v, |k| u[k] * v);
                }
                Prim::FSrc { a, b, u: ui, v } => {
                    let q = ui.map_or(*v, |k| u[k] * v);
                    add(*a, -q);
                    add(*b, q);
                }
                Prim::Gyr { a, b, c, d, k } => {
                    let (dab, dcd) = (de(e, *a, *b), de(e, *c, *d));
                    add(*a, k * dcd);
                    add(*b, -k * dcd);
                    add(*c, -k * dab);
                    add(*d, k * dab);
                }
                Prim::MeasF { a, b, j: ji, .. } => {
                    add(*a, j[*ji]);
                    add(*b, -j[*ji]);
                    r[*ji] = de(e, *a, *b);
                }
                Prim::Lash { a, b, k, c, gap, x: xi } => {
                    let h = gap / 2.0;
                    let th = x[*xi];
                    let v = if th > h { k * (th - h) + c * de(e, *a, *b) } else if th < -h { k * (th + h) + c * de(e, *a, *b) } else { 0.0 };
                    add(*a, v);
                    add(*b, -v);
                }
                Prim::Stop { a, k, c, lo, hi, x: xi } => {
                    let xx = x[*xi];
                    let v = if xx > *hi { k * (xx - hi) + c * de(e, *a, GROUND).max(0.0) } else if xx < *lo { k * (xx - lo) + c * de(e, *a, GROUND).min(0.0) } else { 0.0 };
                    add(*a, v);
                }
            }
        }
        (f, r)
    }

    /// Group sums of flows (weighted, scaled) and the residuals of the unknowns.
    fn residual(&self, t: f64, x: &[f64], z: &[f64], u: &[f64], modes: &[bool]) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let e = self.potentials(x, z);
        let (f, rc) = self.flows(t, x, &e, &z[self.nz..], u, modes);
        let ng = self.gcap.len();
        let mut sg = vec![0.0; ng];
        for n in 0..self.nnodes {
            let (g, s, w) = self.node[n];
            sg[g] += w * s * f[n];
        }
        let mut r = vec![0.0; self.nz + self.nj];
        for g in 0..ng {
            if let Some(k) = self.gunk[g] {
                r[k] = sg[g];
            }
        }
        r[self.nz..].copy_from_slice(&rc);
        (r, sg, e)
    }

    fn solve(&self, t: f64, x: &[f64], u: &[f64], modes: &[bool]) -> Sol {
        let n = self.nz + self.nj;
        let mut z = self.z0.clone();
        if n == 0 {
            let e = self.potentials(x, &z);
            return Sol { e, j: vec![], ok: true };
        }
        let norm = |r: &[f64]| r.iter().fold(0.0f64, |a, v| a.max(v.abs()));
        let (mut r, _, _) = self.residual(t, x, &z, u, modes);
        let mut ok = false;
        for _ in 0..50 {
            let scale = 1.0 + z.iter().fold(0.0f64, |a, v| a.max(v.abs()));
            if norm(&r) <= 1e-11 * scale {
                ok = true;
                break;
            }
            let mut jac = vec![0.0; n * n];
            for c in 0..n {
                let h = 1e-7 * z[c].abs().max(1e-3);
                let mut zp = z.clone();
                zp[c] += h;
                let (rp, _, _) = self.residual(t, x, &zp, u, modes);
                for i in 0..n {
                    jac[i * n + c] = (rp[i] - r[i]) / h;
                }
            }
            let Some(lu) = crate::linalg::Lu::new(jac, n) else { break };
            let mut d: Vec<f64> = r.iter().map(|v| -v).collect();
            lu.solve(&mut d);
            for c in 0..n {
                z[c] += d[c];
            }
            r = self.residual(t, x, &z, u, modes).0;
        }
        if !ok {
            let scale = 1.0 + z.iter().fold(0.0f64, |a, v| a.max(v.abs()));
            ok = norm(&r) <= 1e-8 * scale;
        }
        let e = self.potentials(x, &z);
        Sol { e, j: z[self.nz..].to_vec(), ok }
    }

    /// Is the algebraic part solvable at all (a non-singular Jacobian)? For the model check.
    pub fn check(&self, u: &[f64]) -> Result<(), String> {
        let n = self.nz + self.nj;
        if n == 0 {
            return Ok(());
        }
        let x = self.x0.clone();
        let z = self.z0.clone();
        let (r, _, _) = self.residual(0.0, &x, &z, u, &self.modes);
        let mut jac = vec![0.0; n * n];
        for c in 0..n {
            let mut zp = z.clone();
            zp[c] += 1e-6;
            let (rp, _, _) = self.residual(0.0, &x, &zp, u, &self.modes);
            for i in 0..n {
                jac[i * n + c] = (rp[i] - r[i]) / 1e-6;
            }
        }
        if crate::linalg::Lu::new(jac, n).is_none() {
            return Err("the network cannot be solved: a node has no path to a reference (add a Fixed, Ground or Ambient part), or sources fight each other (an across source in parallel with a capacitance or another source, or a through source in series with an inductor or spring). Add a small resistance, damper or spring to break it.".into());
        }
        Ok(())
    }

    /// Derivatives and outputs.
    pub fn eval(&self, t: f64, x: &[f64], u: &[f64], dx: Option<&mut [f64]>, y: Option<&mut [f64]>) {
        let s = self.solve(t, x, u, &self.modes);
        let z: Vec<f64> = {
            let mut z = vec![0.0; self.nz];
            for g in 0..self.gcap.len() {
                if let Some(k) = self.gunk[g] {
                    let (n0, s0) = (0..self.nnodes).find(|n| self.node[*n].0 == g).map(|n| (n, self.node[n].1)).unwrap();
                    z[k] = s.e[n0] / s0;
                }
            }
            z.extend(&s.j);
            z
        };
        let (_, sg, e) = self.residual(t, x, &z, u, &self.modes);
        if let Some(dx) = dx {
            dx.fill(0.0);
            if !s.ok {
                dx.fill(f64::NAN);
                return;
            }
            for g in 0..self.gcap.len() {
                if let Some(k) = self.gstate[g] {
                    dx[k] = -sg[g] / self.gcap[g];
                }
            }
            for p in &self.prims {
                match p {
                    Prim::CapF { c, x: xi, j: ji, .. } => dx[*xi] = s.j[*ji] / c,
                    Prim::Ind { a, b, l, x: xi } => dx[*xi] = de(&e, *a, *b) / l,
                    Prim::Lash { a, b, x: xi, .. } => dx[*xi] = de(&e, *a, *b),
                    Prim::Stop { a, x: xi, .. } | Prim::Pos { a, x: xi, .. } => dx[*xi] = de(&e, *a, GROUND),
                    Prim::MeasA { a, b, x: Some(xi), .. } => dx[*xi] = de(&e, *a, *b),
                    _ => {}
                }
            }
        }
        if let Some(y) = y {
            for p in &self.prims {
                match p {
                    Prim::MeasA { a, b, y: yi, x: xs, yx } => {
                        y[*yi] = de(&e, *a, *b);
                        if let Some(xi) = xs {
                            y[*yx] = x[*xi];
                        }
                    }
                    Prim::MeasF { y: yi, j: ji, .. } => y[*yi] = s.j[*ji],
                    _ => {}
                }
            }
        }
    }

    fn diodes(&self) -> Vec<(usize, &Prim)> {
        self.prims.iter().filter_map(|p| match p {
            Prim::Res { law: Law::Diode { .. }, mode, .. } => Some((*mode, p)),
            _ => None,
        }).collect()
    }
}

fn res_flow(law: &Law, d: f64, e: &[f64], a: usize, b: usize, u: &[f64], on: bool) -> f64 {
    match law {
        Law::Lin(g) => g * d,
        Law::Friction { fc, fs, ws, c } => (fc + (fs - fc) * (-(d / ws).powi(2)).exp()) * (d / (ws / 10.0)).tanh() + c * d,
        Law::Orifice { k, d: dd } => k * d / (d.abs() + dd).sqrt(),
        Law::Rad(k) => {
            let ta = if a == GROUND { 0.0 } else { e[a] } + 273.15;
            let tb = if b == GROUND { 0.0 } else { e[b] } + 273.15;
            k * (ta.powi(4) - tb.powi(4))
        }
        Law::Diode { ron, goff, vf } => if on { (d - vf) / ron } else { goff * d },
        Law::Switch { ron, goff, u: ui } => if u[*ui] >= 0.5 { d / ron } else { goff * d },
        Law::Valve { k, d: dd, u: ui } => k * u[*ui].clamp(0.0, 1.0) * d / (d.abs() + dd).sqrt(),
        Law::Relief { pset, g } => if d > *pset { g * (d - pset) } else { 0.0 },
    }
}

/// The network as a block of the signal engine.
#[derive(Clone)]
pub struct NetRt {
    pub net: Network,
    /// Whether an output depends directly on an input (found when built, see `feeds`).
    pub feed: bool,
}

impl Rt for NetRt {
    fn rate(&self) -> Rate {
        Rate::Continuous
    }
    fn nx(&self) -> usize {
        self.net.nx
    }
    fn init(&mut self, x: &mut [f64]) {
        x.copy_from_slice(&self.net.x0);
        self.net.z0.fill(0.0);
        // Diodes start conducting when forward-biased at the start.
        let s = self.net.solve(0.0, x, &vec![0.0; self.net.nu], &self.net.modes);
        let _ = s;
    }
    fn feed(&self) -> bool {
        self.feed
    }
    fn out(&self, c: &Ctx, x: &[f64], u: &[f64], y: &mut [f64]) {
        self.net.eval(c.t, x, u, None, Some(y));
    }
    fn deriv(&self, c: &Ctx, x: &[f64], u: &[f64], dx: &mut [f64]) {
        self.net.eval(c.t, x, u, Some(dx), None);
    }
    fn update(&mut self, c: &Ctx, x: &mut [f64], u: &[f64]) {
        let s = self.net.solve(c.t, x, u, &self.net.modes);
        if s.ok {
            let mut z = vec![0.0; self.net.nz];
            for g in 0..self.net.gcap.len() {
                if let Some(k) = self.net.gunk[g] {
                    let (n0, s0) = (0..self.net.nnodes).find(|n| self.net.node[*n].0 == g).map(|n| (n, self.net.node[n].1)).unwrap();
                    z[k] = s.e[n0] / s0;
                }
            }
            z.extend(&s.j);
            self.net.z0 = z;
        }
        // Diode modes: on while current flows forward, off while the voltage stays below Vf.
        let ds: Vec<(usize, Prim)> = self.net.diodes().into_iter().map(|(m, p)| (m, p.clone())).collect();
        for (m, p) in ds {
            if let Prim::Res { a, b, law: Law::Diode { ron, vf, .. }, .. } = p {
                let d = de(&s.e, a, b);
                let on = self.net.modes[m];
                self.net.modes[m] = if on { (d - vf) / ron > 0.0 } else { d > vf };
            }
        }
    }
    fn nzc(&self) -> usize {
        self.net.diodes().len()
    }
    fn zc(&self, c: &Ctx, x: &[f64], u: &[f64], z: &mut [f64]) {
        let s = self.net.solve(c.t, x, u, &self.net.modes);
        for (k, (m, p)) in self.net.diodes().into_iter().enumerate() {
            if let Prim::Res { a, b, law: Law::Diode { ron, vf, .. }, .. } = p {
                let d = de(&s.e, *a, *b);
                z[k] = if self.net.modes[m] { (d - vf) / ron } else { d - vf };
            }
        }
    }
    fn clone_box(&self) -> Box<dyn Rt> {
        Box::new(self.clone())
    }
}

// ---------------------------------------------------------------- the parts

/// A part's ports: name, domain, and whether an unconnected port is the reference (frame,
/// ground) rather than open.
pub type PortSpec = (&'static str, Dom, bool);

/// The physical ports of a part kind (None: not a physical part).
pub fn ports_of(kind: &str) -> Option<&'static [PortSpec]> {
    use Dom::*;
    Some(match kind {
        "RotInertia" => &[("flange", Rot, false)],
        "RotSpring" | "RotDamper" | "RotFriction" | "Backlash" => &[("a", Rot, false), ("b", Rot, true)],
        "Gear" => &[("in", Rot, false), ("out", Rot, false)],
        "RotFixed" => &[("flange", Rot, false)],
        "TorqueSource" | "SpeedSource" => &[("a", Rot, false), ("b", Rot, true)],
        "RotSensor" => &[("flange", Rot, false)],
        "TorqueSensor" => &[("a", Rot, false), ("b", Rot, false)],
        _ => return None,
    })
}

pub fn is_part(kind: &str) -> bool {
    ports_of(kind).is_some()
}

/// A part's elements in local numbering: nodes 0… are its ports, then its internal nodes;
/// x, j, u, y and mode indices count from 0 within the part.
pub struct Part {
    pub internal: usize,
    pub prims: Vec<Prim>,
    pub links: Vec<Link>,
    pub nu: usize,
    pub ny: usize,
    pub xs: Vec<f64>,
    pub nj: usize,
    pub nmodes: usize,
}

fn num(p: &crate::rt::Pv, k: &str) -> Result<f64, String> {
    p.m.get(k).ok_or_else(|| format!("{k} is missing"))?.s().map_err(|_| format!("{k} must be a single number"))
}
fn pos(p: &crate::rt::Pv, k: &str) -> Result<f64, String> {
    let v = num(p, k)?;
    if v > 0.0 { Ok(v) } else { Err(format!("{k} must be positive")) }
}

/// The signal ports of a part kind: (inputs, outputs).
pub fn signals_of(kind: &str) -> (usize, usize) {
    match kind {
        "TorqueSource" | "SpeedSource" => (1, 0),
        "RotSensor" => (0, 2),
        "TorqueSensor" => (0, 1),
        _ => (0, 0),
    }
}

pub fn part(kind: &str, p: &crate::rt::Pv, name: &str) -> Result<Part, String> {
    let mut pt = Part { internal: 0, prims: vec![], links: vec![], nu: 0, ny: 0, xs: vec![], nj: 0, nmodes: 0 };
    let (nu, ny) = signals_of(kind);
    pt.nu = nu;
    pt.ny = ny;
    match kind {
        "RotInertia" => pt.prims.push(Prim::Cap { n: 0, c: pos(p, "J")?, init: Some(num(p, "w0")?) }),
        "RotSpring" => {
            let k = pos(p, "k")?;
            pt.prims.push(Prim::Ind { a: 0, b: 1, l: 1.0 / k, x: 0 });
            pt.xs.push(k * num(p, "phi0")?);
        }
        "RotDamper" => pt.prims.push(Prim::Res { a: 0, b: 1, law: Law::Lin(num(p, "d")?), mode: 0 }),
        "RotFriction" => {
            let (fc, fs, ws, c) = (num(p, "Tc")?, num(p, "Ts")?, pos(p, "ws")?, num(p, "d")?);
            if fs < fc {
                return Err("the static (break-away) torque must be at least the sliding torque".into());
            }
            pt.prims.push(Prim::Res { a: 0, b: 1, law: Law::Friction { fc, fs, ws, c }, mode: 0 });
        }
        "Backlash" => {
            pt.prims.push(Prim::Lash { a: 0, b: 1, k: pos(p, "k")?, c: num(p, "d")?, gap: num(p, "gap")?.max(0.0), x: 0 });
            pt.xs.push(0.0);
        }
        "Gear" => {
            let (n, eta) = (num(p, "N")?, num(p, "eta")?);
            if n == 0.0 || !(eta > 0.0 && eta <= 1.0) {
                return Err("the ratio must not be 0 and the efficiency must be in (0, 1]".into());
            }
            pt.links.push(Link { a: 0, b: 1, r: n, eta, what: name.into() });
        }
        "RotFixed" => pt.links.push(Link { a: 0, b: GROUND, r: 0.0, eta: 1.0, what: name.into() }),
        "TorqueSource" => pt.prims.push(Prim::FSrc { a: 0, b: 1, u: Some(0), v: 1.0 }),
        "SpeedSource" => {
            pt.prims.push(Prim::ASrc { a: 0, b: 1, u: Some(0), v: 1.0, j: 0 });
            pt.nj = 1;
        }
        "RotSensor" => {
            pt.prims.push(Prim::MeasA { a: 0, b: GROUND, y: 0, x: Some(0), yx: 1 });
            pt.xs.push(num(p, "phi0")?);
        }
        "TorqueSensor" => {
            pt.prims.push(Prim::MeasF { a: 0, b: 1, y: 0, j: 0 });
            pt.nj = 1;
        }
        k => return Err(format!("'{k}' is not a physical part")),
    }
    Ok(pt)
}

/// How the compiler groups physical parts into networks: per network, its members (flat index,
/// the global node of each port) and its node count.
#[derive(Clone, Debug)]
pub struct Plan {
    pub members: Vec<(usize, Vec<usize>)>,
    pub nnodes: usize,
}

/// Group the physical parts (by flat index, with their block ids) into networks, from the
/// physical connections: `conns` pairs (block id, port).
pub fn plan(parts: &[(usize, u32, &str)], conns: &[((u32, usize), (u32, usize))]) -> Vec<Plan> {
    // Keys: every (part, port).
    let mut keys: Vec<(u32, usize)> = vec![];
    for (_, id, kind) in parts {
        for k in 0..ports_of(kind).map_or(0, |p| p.len()) {
            keys.push((*id, k));
        }
    }
    let ix = |k: (u32, usize), keys: &Vec<(u32, usize)>| keys.iter().position(|x| *x == k);
    let mut uf: Vec<usize> = (0..keys.len()).collect();
    fn root(uf: &mut [usize], i: usize) -> usize {
        let mut r = i;
        while uf[r] != r {
            r = uf[r];
        }
        uf[i] = r;
        r
    }
    for (a, b) in conns {
        if let (Some(i), Some(j)) = (ix(*a, &keys), ix(*b, &keys)) {
            let (ri, rj) = (root(&mut uf, i), root(&mut uf, j));
            uf[ri] = rj;
        }
    }
    let mut size = vec![0usize; keys.len()];
    for i in 0..keys.len() {
        let r = root(&mut uf, i);
        size[r] += 1;
    }
    // Networks: parts sharing a node, joined through their own ports.
    let mut net: Vec<usize> = (0..parts.len()).collect();
    fn nroot(net: &mut [usize], i: usize) -> usize {
        let mut r = i;
        while net[r] != r {
            r = net[r];
        }
        net[i] = r;
        r
    }
    let owner: Vec<usize> = keys.iter().map(|k| parts.iter().position(|p| p.1 == k.0).unwrap()).collect();
    for i in 0..keys.len() {
        for j in 0..i {
            if root(&mut uf, i) == root(&mut uf, j) {
                let (a, b) = (nroot(&mut net, owner[i]), nroot(&mut net, owner[j]));
                net[a] = b;
            }
        }
    }
    let mut plans: Vec<(usize, Plan, Vec<usize>)> = vec![];
    for (pi, (fi, id, kind)) in parts.iter().enumerate() {
        let r = nroot(&mut net, pi);
        let at = match plans.iter().position(|x| x.0 == r) {
            Some(a) => a,
            None => {
                plans.push((r, Plan { members: vec![], nnodes: 0 }, vec![]));
                plans.len() - 1
            }
        };
        let spec = ports_of(kind).unwrap();
        let mut nodes = vec![];
        for (k, s) in spec.iter().enumerate() {
            let ki = ix((*id, k), &keys).unwrap();
            let rk = root(&mut uf, ki);
            if size[rk] == 1 && s.2 {
                nodes.push(GROUND);
                continue;
            }
            let (_, plan, roots) = &mut plans[at];
            let n = match roots.iter().position(|x| *x == rk) {
                Some(n) => n,
                None => {
                    roots.push(rk);
                    plan.nnodes += 1;
                    roots.len() - 1
                }
            };
            nodes.push(n);
        }
        plans[at].1.members.push((*fi, nodes));
    }
    plans.into_iter().map(|p| p.1).collect()
}

/// Build a network's block from its plan and its members' parameters.
pub fn build(plan: &Plan, kinds: &[&str], names: &[&str], pvs: &[&crate::rt::Pv]) -> Result<(NetRt, Vec<usize>), (usize, String)> {
    let (mut prims, mut links) = (vec![], vec![]);
    let (mut nx, mut nj, mut nu, mut ny, mut nm) = (0, 0, 0, 0, 0);
    let mut xs = vec![];
    let mut nnodes = plan.nnodes;
    for (k, (_, nodes)) in plan.members.iter().enumerate() {
        let part = part(kinds[k], pvs[k], names[k]).map_err(|e| (k, e))?;
        let base = nnodes;
        nnodes += part.internal;
        let map = |n: usize| if n == GROUND { GROUND } else if n < nodes.len() { nodes[n] } else { base + n - nodes.len() };
        for mut p in part.prims {
            match &mut p {
                Prim::Cap { n, .. } => *n = map(*n),
                Prim::CapF { a, b, x, j, .. } => { *a = map(*a); *b = map(*b); *x += nx; *j += nj; }
                Prim::Ind { a, b, x, .. } => { *a = map(*a); *b = map(*b); *x += nx; }
                Prim::Res { a, b, law, mode } => {
                    *a = map(*a);
                    *b = map(*b);
                    *mode += nm;
                    match law {
                        Law::Switch { u, .. } | Law::Valve { u, .. } => *u += nu,
                        _ => {}
                    }
                }
                Prim::Tdx { a, b, c, d, j, .. } => { *a = map(*a); *b = map(*b); *c = map(*c); *d = map(*d); *j += nj; }
                Prim::ASrc { a, b, u, j, .. } => { *a = map(*a); *b = map(*b); if let Some(u) = u { *u += nu; } *j += nj; }
                Prim::FSrc { a, b, u, .. } => { *a = map(*a); *b = map(*b); if let Some(u) = u { *u += nu; } }
                Prim::Gyr { a, b, c, d, .. } => { *a = map(*a); *b = map(*b); *c = map(*c); *d = map(*d); }
                Prim::MeasA { a, b, y, x, yx } => { *a = map(*a); *b = map(*b); *y += ny; *yx += ny; if let Some(x) = x { *x += nx; } }
                Prim::MeasF { a, b, y, j } => { *a = map(*a); *b = map(*b); *y += ny; *j += nj; }
                Prim::Lash { a, b, x, .. } => { *a = map(*a); *b = map(*b); *x += nx; }
                Prim::Stop { a, x, .. } | Prim::Pos { a, x, .. } => { *a = map(*a); *x += nx; }
            }
            prims.push(p);
        }
        for mut l in part.links {
            l.a = map(l.a);
            l.b = map(l.b);
            links.push(l);
        }
        nx += part.xs.len();
        xs.extend(part.xs);
        nj += part.nj;
        nu += part.nu;
        ny += part.ny;
        nm += part.nmodes;
    }
    let net = Network::new(prims, links, nnodes, nx, nj, nm, nu, ny, xs).map_err(|e| (0, e))?;
    net.check(&vec![0.0; nu]).map_err(|e| (0, e))?;
    let feed = feeds(&net);
    Ok((NetRt { net, feed }, vec![1; ny]))
}

/// Whether any output moves when the inputs change with the states held: a speed sensor on an
/// inertia reads a state (no direct feedthrough, so no algebraic loop through a controller), a
/// torque sensor or a node without inertia does not.
fn feeds(net: &Network) -> bool {
    if net.nu == 0 || net.ny == 0 {
        return false;
    }
    let mut y0 = vec![0.0; net.ny];
    net.eval(0.0, &net.x0, &vec![0.0; net.nu], None, Some(&mut y0));
    [1.0, -3.7, 25.0].iter().any(|&a| {
        let u: Vec<f64> = (0..net.nu).map(|i| a * (1.0 + 0.31 * i as f64)).collect();
        let mut y = vec![0.0; net.ny];
        net.eval(0.0, &net.x0, &u, None, Some(&mut y));
        y.iter().zip(&y0).any(|(a, b)| (a - b).abs() > 1e-9 * (1.0 + b.abs()) || !a.is_finite())
    })
}

