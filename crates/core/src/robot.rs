//! The robot arm module (0.3.6): a two-link planar arm with joint torques, a joint trajectory,
//! an arm controller (computed torque or PD with gravity compensation), inverse kinematics, and an
//! animated view of the arm (a sink the window draws).
//!
//! Angles: q1 of link 1 from the horizontal, q2 of link 2 relative to link 1; gravity acts along
//! −y. M(q)·q̈ + C(q, q̇)·q̇ + G(q) + b·q̇ = τ.

use crate::expr::Mat;
use crate::rt::{Ctx, Pv, Rate, Rt};

/// The arm's dynamic parameters.
#[derive(Clone, Copy, Debug)]
pub struct Arm {
    pub m1: f64,
    pub m2: f64,
    pub l1: f64,
    pub l2: f64,
    pub lc1: f64,
    pub lc2: f64,
    pub i1: f64,
    pub i2: f64,
    pub g: f64,
    pub b: f64,
}

impl Arm {
    /// Mass matrix (m11, m12, m22), Coriolis/centrifugal terms and gravity.
    pub fn terms(&self, q: [f64; 2], qd: [f64; 2]) -> ([f64; 3], [f64; 2], [f64; 2]) {
        let (c2, s2) = (q[1].cos(), q[1].sin());
        let m22 = self.m2 * self.lc2 * self.lc2 + self.i2;
        let m12 = self.m2 * (self.lc2 * self.lc2 + self.l1 * self.lc2 * c2) + self.i2;
        let m11 = self.m1 * self.lc1 * self.lc1 + self.i1 + self.m2 * (self.l1 * self.l1 + self.lc2 * self.lc2 + 2.0 * self.l1 * self.lc2 * c2) + self.i2;
        let h = self.m2 * self.l1 * self.lc2 * s2;
        let c = [-h * qd[1] * (2.0 * qd[0] + qd[1]), h * qd[0] * qd[0]];
        let c12 = (q[0] + q[1]).cos();
        let g = [(self.m1 * self.lc1 + self.m2 * self.l1) * self.g * q[0].cos() + self.m2 * self.lc2 * self.g * c12, self.m2 * self.lc2 * self.g * c12];
        ([m11, m12, m22], c, g)
    }
    /// Joint accelerations for joint torques.
    pub fn accel(&self, q: [f64; 2], qd: [f64; 2], tau: [f64; 2]) -> [f64; 2] {
        let (m, c, g) = self.terms(q, qd);
        let r = [tau[0] - c[0] - g[0] - self.b * qd[0], tau[1] - c[1] - g[1] - self.b * qd[1]];
        let det = m[0] * m[2] - m[1] * m[1];
        [(m[2] * r[0] - m[1] * r[1]) / det, (m[0] * r[1] - m[1] * r[0]) / det]
    }
    /// Kinetic plus potential energy (for checks).
    pub fn energy(&self, q: [f64; 2], qd: [f64; 2]) -> f64 {
        let (m, _, _) = self.terms(q, qd);
        let ke = 0.5 * (m[0] * qd[0] * qd[0] + 2.0 * m[1] * qd[0] * qd[1] + m[2] * qd[1] * qd[1]);
        let pe = self.g * ((self.m1 * self.lc1 + self.m2 * self.l1) * q[0].sin() + self.m2 * self.lc2 * (q[0] + q[1]).sin());
        ke + pe
    }
    pub fn tip(&self, q: [f64; 2]) -> [f64; 2] {
        [self.l1 * q[0].cos() + self.l2 * (q[0] + q[1]).cos(), self.l1 * q[0].sin() + self.l2 * (q[0] + q[1]).sin()]
    }
}

fn s(p: &Pv, k: &str) -> Result<f64, String> {
    p.m.get(k).ok_or_else(|| format!("{k} is missing"))?.s().map_err(|_| format!("{k} must be a single number"))
}

/// The arm parameters (any block that models the arm: the arm itself and its controller).
fn arm(p: &Pv) -> Result<Arm, String> {
    let a = Arm { m1: s(p, "m1")?, m2: s(p, "m2")?, l1: s(p, "l1")?, l2: s(p, "l2")?, lc1: s(p, "lc1")?, lc2: s(p, "lc2")?, i1: s(p, "I1")?, i2: s(p, "I2")?, g: s(p, "g")?, b: s(p, "b")? };
    if !(a.m1 > 0.0 && a.m2 > 0.0 && a.l1 > 0.0 && a.l2 > 0.0) || a.i1 < 0.0 || a.i2 < 0.0 || a.b < 0.0 {
        return Err("masses and lengths must be positive; inertias and friction at least 0".into());
    }
    Ok(a)
}

fn two(p: &Pv, k: &str) -> Result<[f64; 2], String> {
    let v = p.v(k)?;
    match v.as_slice() {
        [a] => Ok([*a, *a]),
        [a, b] => Ok([*a, *b]),
        _ => Err(format!("{k} takes one value or two (one per joint)")),
    }
}

#[derive(Clone)]
struct ArmRt {
    a: Arm,
    q0: [f64; 2],
    /// A single torque for both joints (a scalar input).
    one: bool,
}
impl Rt for ArmRt {
    fn rate(&self) -> Rate {
        Rate::Continuous
    }
    fn nx(&self) -> usize {
        4
    }
    fn init(&mut self, x: &mut [f64]) {
        x.copy_from_slice(&[self.q0[0], self.q0[1], 0.0, 0.0]);
    }
    fn feed(&self) -> bool {
        false
    }
    fn out(&self, _: &Ctx, x: &[f64], _: &[f64], y: &mut [f64]) {
        let t = self.a.tip([x[0], x[1]]);
        y.copy_from_slice(&[x[0], x[1], x[2], x[3], t[0], t[1]]);
    }
    fn deriv(&self, _: &Ctx, x: &[f64], u: &[f64], dx: &mut [f64]) {
        let tau = if self.one { [u[0], u[0]] } else { [u[0], u[1]] };
        let a = self.a.accel([x[0], x[1]], [x[2], x[3]], tau);
        dx.copy_from_slice(&[x[2], x[3], a[0], a[1]]);
    }
    fn clone_box(&self) -> Box<dyn Rt> {
        Box::new(self.clone())
    }
}

/// Quintic segments through way points (t, q1, q2) with zero velocity and acceleration at each.
#[derive(Clone)]
struct Traj {
    pts: Vec<(f64, [f64; 2])>,
}
impl Traj {
    fn at(&self, t: f64) -> ([f64; 2], [f64; 2], [f64; 2]) {
        let n = self.pts.len();
        if t <= self.pts[0].0 {
            return (self.pts[0].1, [0.0; 2], [0.0; 2]);
        }
        if t >= self.pts[n - 1].0 {
            return (self.pts[n - 1].1, [0.0; 2], [0.0; 2]);
        }
        let k = self.pts.iter().rposition(|p| p.0 <= t).unwrap();
        let ((t0, a), (t1, b)) = (self.pts[k], self.pts[k + 1]);
        let h = t1 - t0;
        let s = (t - t0) / h;
        let (p, dp, ddp) = (s * s * s * (10.0 - 15.0 * s + 6.0 * s * s), 30.0 * s * s * (1.0 - s) * (1.0 - s) / h, 60.0 * s * (1.0 - s) * (1.0 - 2.0 * s) / (h * h));
        let f = |g: f64| [a[0] + (b[0] - a[0]) * g, a[1] + (b[1] - a[1]) * g];
        let d = |g: f64| [(b[0] - a[0]) * g, (b[1] - a[1]) * g];
        (f(p), d(dp), d(ddp))
    }
}
impl Rt for Traj {
    fn rate(&self) -> Rate {
        Rate::Continuous
    }
    fn feed(&self) -> bool {
        false
    }
    fn out(&self, c: &Ctx, _: &[f64], _: &[f64], y: &mut [f64]) {
        let (q, qd, qdd) = self.at(c.t);
        y.copy_from_slice(&[q[0], q[1], qd[0], qd[1], qdd[0], qdd[1]]);
    }
    fn next_hit(&self, t: f64) -> f64 {
        self.pts.iter().map(|p| p.0).find(|x| *x > t + 1e-12).unwrap_or(f64::INFINITY)
    }
    fn clone_box(&self) -> Box<dyn Rt> {
        Box::new(self.clone())
    }
}

/// Computed torque τ = M(q)(q̈_r + Kp·e + Kd·ė) + C + G + b·q̇, or PD with gravity compensation
/// τ = Kp·e + Kd·ė + G. Inputs: q_ref, q̇_ref, q̈_ref (from the trajectory), q, q̇ (from the arm).
#[derive(Clone)]
struct ArmCtl {
    a: Arm,
    kp: [f64; 2],
    kd: [f64; 2],
    ct: bool,
    tmax: f64,
}
impl Rt for ArmCtl {
    fn out(&self, _: &Ctx, _: &[f64], u: &[f64], y: &mut [f64]) {
        let (qr, qdr, qddr, q, qd) = ([u[0], u[1]], [u[2], u[3]], [u[4], u[5]], [u[6], u[7]], [u[8], u[9]]);
        let e = [qr[0] - q[0], qr[1] - q[1]];
        let ed = [qdr[0] - qd[0], qdr[1] - qd[1]];
        let v = [self.kp[0] * e[0] + self.kd[0] * ed[0], self.kp[1] * e[1] + self.kd[1] * ed[1]];
        let (m, c, g) = self.a.terms(q, qd);
        let tau = if self.ct {
            let w = [qddr[0] + v[0], qddr[1] + v[1]];
            [m[0] * w[0] + m[1] * w[1] + c[0] + g[0] + self.a.b * qd[0], m[1] * w[0] + m[2] * w[1] + c[1] + g[1] + self.a.b * qd[1]]
        } else {
            [v[0] + g[0], v[1] + g[1]]
        };
        y[0] = tau[0].clamp(-self.tmax, self.tmax);
        y[1] = tau[1].clamp(-self.tmax, self.tmax);
    }
    fn clone_box(&self) -> Box<dyn Rt> {
        Box::new(self.clone())
    }
}

/// Inverse kinematics of the two-link arm: the joint angles that put the tip at (x, y).
#[derive(Clone)]
struct Ik {
    l1: f64,
    l2: f64,
    up: bool,
}
impl Rt for Ik {
    fn out(&self, _: &Ctx, _: &[f64], u: &[f64], y: &mut [f64]) {
        let (x, yy) = (u[0], u[1]);
        let c2 = ((x * x + yy * yy - self.l1 * self.l1 - self.l2 * self.l2) / (2.0 * self.l1 * self.l2)).clamp(-1.0, 1.0);
        let q2 = if self.up { -c2.acos() } else { c2.acos() };
        let q1 = yy.atan2(x) - (self.l2 * q2.sin()).atan2(self.l1 + self.l2 * q2.cos());
        y[0] = q1;
        y[1] = q2;
    }
    fn clone_box(&self) -> Box<dyn Rt> {
        Box::new(self.clone())
    }
}

pub const KINDS: &[&str] = &["RobotArm2", "ArmTrajectory", "ArmController", "ArmIK", "RobotArm", "ArmIKN", "ToolPath"];

/// Blocks that take their robot from a Robot arm block named in their "arm" parameter.
pub const USES_ARM: &[&str] = &["ArmIKN"];

/// Inverse kinematics for any arm: the tool position (x y z), or position and roll–pitch–yaw,
/// to joint values. Each solution starts from the last one, so the arm moves smoothly.
#[derive(Clone)]
struct IkN {
    c: crate::multibody::Chain,
    ori: bool,
    last: Vec<f64>,
    seed: Vec<f64>,
}
impl IkN {
    fn solve(&self, u: &[f64]) -> Vec<f64> {
        let r = if self.ori { Some(crate::multibody::rpy(u[3], u[4], u[5])) } else { None };
        self.c.ik([u[0], u[1], u[2]], r.as_ref(), &self.last, 100).0
    }
}
impl Rt for IkN {
    fn init(&mut self, _: &mut [f64]) {
        self.last = self.seed.clone();
        self.last.resize(self.c.n, 0.0);
    }
    fn out(&self, _: &Ctx, _: &[f64], u: &[f64], y: &mut [f64]) {
        y.copy_from_slice(&self.solve(u));
    }
    fn update(&mut self, _: &Ctx, _: &mut [f64], u: &[f64]) {
        self.last = self.solve(u);
    }
    fn clone_box(&self) -> Box<dyn Rt> {
        Box::new(self.clone())
    }
}

/// Straight tool moves through way points (t, x, y, z), each starting and ending at rest.
#[derive(Clone)]
struct ToolPath {
    pts: Vec<(f64, [f64; 3])>,
}
impl Rt for ToolPath {
    fn rate(&self) -> Rate {
        Rate::Continuous
    }
    fn feed(&self) -> bool {
        false
    }
    fn out(&self, c: &Ctx, _: &[f64], _: &[f64], y: &mut [f64]) {
        let (n, t) = (self.pts.len(), c.t);
        let (p, v) = if t <= self.pts[0].0 {
            (self.pts[0].1, [0.0; 3])
        } else if t >= self.pts[n - 1].0 {
            (self.pts[n - 1].1, [0.0; 3])
        } else {
            let k = self.pts.iter().rposition(|p| p.0 <= t).unwrap();
            let ((t0, a), (t1, b)) = (self.pts[k], self.pts[k + 1]);
            let h = t1 - t0;
            let s = (t - t0) / h;
            let (g, dg) = (s * s * s * (10.0 - 15.0 * s + 6.0 * s * s), 30.0 * s * s * (1.0 - s) * (1.0 - s) / h);
            (std::array::from_fn(|i| a[i] + (b[i] - a[i]) * g), std::array::from_fn(|i| (b[i] - a[i]) * dg))
        };
        y[..3].copy_from_slice(&p);
        y[3..6].copy_from_slice(&v);
    }
    fn next_hit(&self, t: f64) -> f64 {
        self.pts.iter().map(|p| p.0).find(|x| *x > t + 1e-12).unwrap_or(f64::INFINITY)
    }
    fn clone_box(&self) -> Box<dyn Rt> {
        Box::new(self.clone())
    }
}

/// A robot arm with any number of joints (0.4.1): states q and q̇; outputs q, q̇ and the tool's
/// position. One torque for every joint, or a single value for all.
#[derive(Clone)]
pub struct ArmN {
    pub c: crate::multibody::Chain,
    pub q0: Vec<f64>,
}
impl Rt for ArmN {
    fn rate(&self) -> Rate {
        Rate::Continuous
    }
    fn nx(&self) -> usize {
        2 * self.c.n
    }
    fn init(&mut self, x: &mut [f64]) {
        let n = self.c.n;
        for i in 0..n {
            x[i] = self.q0.get(i).cloned().unwrap_or(0.0);
            x[n + i] = 0.0;
        }
    }
    fn feed(&self) -> bool {
        false
    }
    fn out(&self, _: &Ctx, x: &[f64], _: &[f64], y: &mut [f64]) {
        let n = self.c.n;
        y[..2 * n].copy_from_slice(x);
        y[2 * n..2 * n + 3].copy_from_slice(&self.c.tool_pos(&x[..n]));
    }
    fn deriv(&self, _: &Ctx, x: &[f64], u: &[f64], dx: &mut [f64]) {
        let n = self.c.n;
        let tau: Vec<f64> = (0..n).map(|i| if u.len() == 1 { u[0] } else { u[i] }).collect();
        let a = self.c.accel(&x[..n], &x[n..], &tau);
        dx[..n].copy_from_slice(&x[n..]);
        dx[n..].copy_from_slice(&a);
    }
    fn clone_box(&self) -> Box<dyn Rt> {
        Box::new(self.clone())
    }
}

/// What the 3D view draws: the robot's meshes, and for each frame every link's frame (rotation
/// row by row, then origin: 12 numbers per link), the joint origins and the tool.
#[derive(Debug, serde::Serialize)]
pub struct Frames {
    /// "arm6" or "arm7" for the robots that come with the app (their meshes are in the window's
    /// robots/ folder); "" for others (drawn as a skeleton).
    pub lib: String,
    pub meshes: Vec<crate::urdf::Mesh>,
    pub parents: Vec<Option<usize>>,
    pub t: Vec<f64>,
    pub links: Vec<Vec<f64>>,
    pub joints: Vec<Vec<f64>>,
    pub tool: Vec<[f64; 3]>,
}

/// The frames for the RobotView block `view` from the last run's results: its input must come
/// straight from a Robot arm block's q output.
pub fn frames(m: &crate::model::Model, base: Option<&std::path::Path>, view: u32, r: &crate::compile::Results, max: usize) -> Result<Frames, String> {
    let parent = m.parent_of(view).unwrap_or(0);
    let d = m.diagram(parent).ok_or("The view's diagram is missing.")?;
    let src = d.wires.iter().find(|w| w.to == (view, 0)).map(|w| w.from).ok_or("Connect the 3D view to a Robot arm's q output.")?;
    let arm = d.block(src.0).filter(|b| b.kind == "RobotArm" && src.1 == 0).ok_or("The 3D view must be fed the q output of a Robot arm (any joints) block.")?;
    let (c, lib, meshes) = crate::compile::with_params(m, arm.id, base, |p| {
        let c = chain(p)?;
        let lib = match p.t.get("robot").map(|s| s.as_str()) {
            Some("6-joint industrial arm") => "arm6",
            Some("7-joint arm") => "arm7",
            _ => "",
        };
        let meshes = if lib.is_empty() { vec![] } else { crate::urdf::library(lib, 9.81)?.meshes };
        Ok((c, lib.to_string(), meshes))
    })?;
    let cols: Vec<usize> = (0..r.chans.len()).filter(|i| r.chans[*i].sink == view && !r.chans[*i].probe).collect();
    if cols.len() != c.n {
        return Err(format!("Run the model first: the view needs the arm's {} joint angles.", c.n));
    }
    let len = r.t.len();
    let step = len.div_ceil(max.max(2)).max(1);
    let mut idx: Vec<usize> = (0..len).step_by(step).collect();
    if idx.last() != Some(&(len - 1)) && len > 0 {
        idx.push(len - 1);
    }
    let mut f = Frames { lib, meshes, parents: c.bodies.iter().map(|b| b.parent).collect(), t: vec![], links: vec![], joints: vec![], tool: vec![] };
    for i in idx {
        let q: Vec<f64> = cols.iter().map(|k| r.y[*k][i]).collect();
        let ps = c.pose(&q);
        f.t.push(r.t[i]);
        f.links.push(ps.lr.iter().zip(&ps.lp).flat_map(|(rr, pp)| rr.iter().flatten().cloned().chain(pp.iter().cloned()).collect::<Vec<_>>()).collect());
        f.joints.push(ps.jp.iter().flatten().cloned().collect());
        f.tool.push(c.tool_pos(&q));
    }
    Ok(f)
}

/// The chain a RobotArm block describes: the planar two-link arm, or a DH table.
pub fn chain(p: &Pv) -> Result<crate::multibody::Chain, String> {
    let g = s(p, "g")?;
    let b = p.v("b")?;
    let tool = p.v("tool")?;
    let tool: [f64; 3] = match tool.as_slice() {
        [x, y, z] => [*x, *y, *z],
        _ => return Err("the tool offset takes three values (x y z) in the last link's frame".into()),
    };
    let robot = p.t.get("robot").map(|s| s.as_str()).unwrap_or("DH table");
    let with_tool = |mut c: crate::multibody::Chain| {
        c.tool = tool;
        c
    };
    match robot {
        "6-joint industrial arm" => Ok(with_tool(crate::urdf::library("arm6", g)?.chain)),
        "7-joint arm" => Ok(with_tool(crate::urdf::library("arm7", g)?.chain)),
        "URDF file" => {
            let f = p.t.get("file").map(|s| s.trim()).unwrap_or("");
            if f.is_empty() {
                return Err("give the URDF file".into());
            }
            let path = match p.base {
                Some(b) if !std::path::Path::new(f).is_absolute() => b.join(f),
                _ => std::path::PathBuf::from(f),
            };
            let text = std::fs::read_to_string(&path).map_err(|e| format!("could not read {}: {e}", path.display()))?;
            let link = p.t.get("toollink").map(|s| s.trim()).unwrap_or("");
            Ok(crate::urdf::parse(&text, &[], link, tool, g)?.chain)
        }
        "two-link planar" => {
            let h = std::f64::consts::FRAC_PI_2;
            crate::multibody::Chain::from_dh(&[[h, 0.0, 0.0, 0.0, 1.0, 0.25, 0.0, 0.0, 0.0, 0.0208, 0.0208], [0.0, 0.5, 0.0, 0.0, 0.8, 0.2, 0.0, 0.0, 0.0, 0.0107, 0.0107]], false, g, [0.4, 0.0, 0.0], &b)
        }
        _ => {
            let m: &Mat = p.m.get("dh").ok_or("the DH table is missing")?;
            if m.c != 11 || m.r == 0 {
                return Err("the DH table has one row per joint of 11 values: α a d θ₀ m cx cy cz Ixx Iyy Izz".into());
            }
            let rows: Vec<[f64; 11]> = (0..m.r).map(|i| std::array::from_fn(|j| m.at(i, j))).collect();
            crate::multibody::Chain::from_dh(&rows, p.t.get("conv").map(|s| s.as_str()) == Some("standard"), g, tool, &b)
        }
    }
}

fn need(iw: &[usize], w: &[usize], what: &str) -> Result<(), String> {
    if iw.len() != w.len() || iw.iter().zip(w).any(|(a, b)| a != b) {
        return Err(format!("its inputs must be {what}"));
    }
    Ok(())
}

pub fn build(kind: &str, p: &Pv, iw: &[usize]) -> Option<Result<(Box<dyn Rt>, Vec<usize>), String>> {
    let r = || -> Result<(Box<dyn Rt>, Vec<usize>), String> {
        match kind {
            "ArmIKN" => {
                let c = chain(p)?;
                let ori = p.t.get("mode").map(|s| s.as_str()) == Some("position and orientation");
                let w = if ori { 6 } else { 3 };
                if iw.len() != 1 || iw[0] != w {
                    return Err(format!("its input must be the tool's {} (width {w})", if ori { "position and roll, pitch, yaw" } else { "position x, y, z" }));
                }
                let n = c.n;
                let seed = p.v("seed")?;
                Ok((Box::new(IkN { c, ori, last: vec![], seed }), vec![n]))
            }
            "ToolPath" => {
                let m: &Mat = p.m.get("points").ok_or("points is missing")?;
                if m.c != 4 || m.r < 2 {
                    return Err("way points are rows of (time, x, y, z), at least two".into());
                }
                let pts: Vec<(f64, [f64; 3])> = (0..m.r).map(|i| (m.at(i, 0), [m.at(i, 1), m.at(i, 2), m.at(i, 3)])).collect();
                if pts.windows(2).any(|w| w[1].0 <= w[0].0) {
                    return Err("the way points' times must rise".into());
                }
                Ok((Box::new(ToolPath { pts }), vec![3, 3]))
            }
            "RobotArm" => {
                let c = chain(p)?;
                let n = c.n;
                if iw.len() != 1 || (iw[0] != n && iw[0] != 1) {
                    return Err(format!("its input must be the joint torques τ (width {n})"));
                }
                let q0 = p.v("q0")?;
                Ok((Box::new(ArmN { c, q0 }), vec![n, n, 3]))
            }
            "RobotArm2" => {
                // Width 1 is allowed (the same torque on both joints); it also lets a loop through a
                // controller settle its widths.
                if iw.len() != 1 || iw[0] > 2 {
                    return Err("its input must be the joint torques τ (width 2)".into());
                }
                Ok((Box::new(ArmRt { a: arm(p)?, q0: two(p, "q0")?, one: iw[0] == 1 }), vec![2, 2, 2]))
            }
            "ArmTrajectory" => {
                let m: &Mat = p.m.get("points").ok_or("points is missing")?;
                if m.c != 3 || m.r < 2 {
                    return Err("way points are rows of (time, q1, q2), at least two".into());
                }
                let pts: Vec<(f64, [f64; 2])> = (0..m.r).map(|i| (m.at(i, 0), [m.at(i, 1), m.at(i, 2)])).collect();
                if pts.windows(2).any(|w| w[1].0 <= w[0].0) {
                    return Err("the way points' times must rise".into());
                }
                Ok((Box::new(Traj { pts }), vec![2, 2, 2]))
            }
            "ArmController" => {
                need(iw, &[2, 2, 2, 2, 2], "q_ref, q̇_ref, q̈_ref, q and q̇ (width 2 each)")?;
                let ct = p.t.get("law").map(|s| s.as_str()) != Some("PD + gravity");
                let tmax = s(p, "tmax")?;
                Ok((Box::new(ArmCtl { a: arm(p)?, kp: two(p, "Kp")?, kd: two(p, "Kd")?, ct, tmax: if tmax > 0.0 { tmax } else { f64::INFINITY } }), vec![2]))
            }
            "ArmIK" => {
                need(iw, &[2], "the tip position (x, y)")?;
                let (l1, l2) = (s(p, "l1")?, s(p, "l2")?);
                Ok((Box::new(Ik { l1, l2, up: p.t.get("elbow").map(|s| s.as_str()) == Some("up") }), vec![2]))
            }
            _ => Err(String::new()),
        }
    };
    KINDS.contains(&kind).then(r)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arm() -> Arm {
        Arm { m1: 1.0, m2: 0.8, l1: 0.5, l2: 0.4, lc1: 0.25, lc2: 0.2, i1: 1.0 * 0.25 / 12.0, i2: 0.8 * 0.16 / 12.0, g: 9.81, b: 0.0 }
    }

    /// Without torque or friction the arm swings and its energy stays the same (RK4, small steps).
    #[test]
    fn energy_is_conserved() {
        let a = arm();
        let mut x = [0.3, 0.5, 0.0, 0.0];
        let e0 = a.energy([x[0], x[1]], [x[2], x[3]]);
        let f = |x: &[f64; 4]| {
            let ac = a.accel([x[0], x[1]], [x[2], x[3]], [0.0, 0.0]);
            [x[2], x[3], ac[0], ac[1]]
        };
        let h = 1e-4;
        for _ in 0..20_000 {
            let k1 = f(&x);
            let x2: [f64; 4] = std::array::from_fn(|i| x[i] + 0.5 * h * k1[i]);
            let k2 = f(&x2);
            let x3: [f64; 4] = std::array::from_fn(|i| x[i] + 0.5 * h * k2[i]);
            let k3 = f(&x3);
            let x4: [f64; 4] = std::array::from_fn(|i| x[i] + h * k3[i]);
            let k4 = f(&x4);
            x = std::array::from_fn(|i| x[i] + h / 6.0 * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]));
        }
        let e1 = a.energy([x[0], x[1]], [x[2], x[3]]);
        assert!((e1 - e0).abs() < 1e-9 * e0.abs().max(1.0), "{e0} → {e1}");
        assert!(x[2].abs() + x[3].abs() > 0.1, "it moved");
    }

    /// Gravity compensation holds the arm still; inverse kinematics inverts the tip position.
    #[test]
    fn statics_and_kinematics() {
        let a = arm();
        for q in [[0.3, 0.5], [-1.0, 2.0], [1.2, -0.7]] {
            let (_, _, g) = a.terms(q, [0.0, 0.0]);
            let acc = a.accel(q, [0.0, 0.0], g);
            assert!(acc[0].abs() < 1e-12 && acc[1].abs() < 1e-12);
            for up in [false, true] {
                let ik = Ik { l1: a.l1, l2: a.l2, up };
                let mut y = [0.0; 2];
                ik.out(&Ctx { t: 0.0, tseg: 0.0, major: true }, &[], &a.tip(q), &mut y);
                let t = a.tip(y);
                let want = a.tip(q);
                assert!((t[0] - want[0]).abs() < 1e-12 && (t[1] - want[1]).abs() < 1e-12);
            }
        }
        // The trajectory starts and ends at rest on its way points.
        let tr = Traj { pts: vec![(0.0, [0.0, 0.0]), (2.0, [1.0, -0.5])] };
        let (q, qd, qdd) = tr.at(2.0);
        assert_eq!((q, qd, qdd), ([1.0, -0.5], [0.0; 2], [0.0; 2]));
        let (q, qd, _) = tr.at(1.0);
        assert!((q[0] - 0.5).abs() < 1e-12 && (qd[0] - 30.0 / 16.0 / 2.0).abs() < 1e-12, "{q:?} {qd:?}");
    }
}
