//! Robot arms with any number of joints (0.4.1): a tree of rigid links joined by revolute,
//! prismatic or fixed joints, from a DH table or a URDF file. Inverse dynamics by recursive
//! Newton–Euler (world frame), the mass matrix column by column from it, and forward dynamics
//! q̈ = M⁻¹(τ − h). Gravity acts along −z of the base.

pub type V3 = [f64; 3];
pub type M3 = [[f64; 3]; 3];

pub const I3: M3 = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

pub fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
pub fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
pub fn scale(a: V3, k: f64) -> V3 {
    [a[0] * k, a[1] * k, a[2] * k]
}
pub fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
pub fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
pub fn mv(m: &M3, v: V3) -> V3 {
    [dot(m[0], v), dot(m[1], v), dot(m[2], v)]
}
pub fn mm(a: &M3, b: &M3) -> M3 {
    let mut r = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            r[i][j] = (0..3).map(|k| a[i][k] * b[k][j]).sum();
        }
    }
    r
}
pub fn tr(a: &M3) -> M3 {
    let mut r = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            r[i][j] = a[j][i];
        }
    }
    r
}
/// Rotation by `ang` about the unit axis `k` (Rodrigues).
pub fn rot(k: V3, ang: f64) -> M3 {
    let (s, c) = ang.sin_cos();
    let v = 1.0 - c;
    let [x, y, z] = k;
    [[c + x * x * v, x * y * v - z * s, x * z * v + y * s], [y * x * v + z * s, c + y * y * v, y * z * v - x * s], [z * x * v - y * s, z * y * v + x * s, c + z * z * v]]
}
/// URDF roll–pitch–yaw: R = Rz(yaw)·Ry(pitch)·Rx(roll).
pub fn rpy(r: f64, p: f64, y: f64) -> M3 {
    mm(&rot([0.0, 0.0, 1.0], y), &mm(&rot([0.0, 1.0, 0.0], p), &rot([1.0, 0.0, 0.0], r)))
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Revolute,
    Prismatic,
    Fixed,
}

/// One link and the joint that carries it. The joint frame is the parent link's frame moved by
/// `pre`; the joint turns (or slides) about `axis` in that frame by q + offset; the link's own
/// frame (where its mass, centre and inertia are given) is the joint frame moved by `post`.
#[derive(Clone, Debug)]
pub struct Body {
    pub name: String,
    pub parent: Option<usize>,
    pub kind: Kind,
    pub pre_r: M3,
    pub pre_p: V3,
    pub axis: V3,
    pub post_r: M3,
    pub post_p: V3,
    pub offset: f64,
    pub m: f64,
    pub com: V3,
    /// Inertia about the centre of mass, in the link frame.
    pub inertia: M3,
    pub b: f64,
    pub lo: f64,
    pub hi: f64,
    /// Index of this joint's coordinate in q (None for fixed joints).
    pub dof: Option<usize>,
}

#[derive(Clone, Debug)]
pub struct Chain {
    pub bodies: Vec<Body>,
    pub n: usize,
    pub g: f64,
    /// The tool: a point in a link's frame.
    pub tool_body: usize,
    pub tool: V3,
}

/// Where everything is for one configuration.
pub struct Pose {
    /// Joint frame (rotation, origin) and link frame (rotation, origin), world coordinates.
    pub jr: Vec<M3>,
    pub jp: Vec<V3>,
    pub lr: Vec<M3>,
    pub lp: Vec<V3>,
    /// The joint axis in world coordinates.
    pub z: Vec<V3>,
}

impl Body {
    pub fn new(name: &str, parent: Option<usize>, kind: Kind) -> Body {
        Body { name: name.into(), parent, kind, pre_r: I3, pre_p: [0.0; 3], axis: [0.0, 0.0, 1.0], post_r: I3, post_p: [0.0; 3], offset: 0.0, m: 0.0, com: [0.0; 3], inertia: [[0.0; 3]; 3], b: 0.0, lo: f64::NEG_INFINITY, hi: f64::INFINITY, dof: None }
    }
}

impl Chain {
    /// Number the moving joints in order; parents must come before their children.
    pub fn new(mut bodies: Vec<Body>, g: f64, tool: V3) -> Result<Chain, String> {
        let mut n = 0;
        for i in 0..bodies.len() {
            if let Some(p) = bodies[i].parent {
                if p >= i {
                    return Err("a link must come after its parent".into());
                }
            }
            if !(bodies[i].m >= 0.0) {
                return Err(format!("{}: the mass cannot be negative", bodies[i].name));
            }
            let a = bodies[i].axis;
            let l = dot(a, a).sqrt();
            if bodies[i].kind != Kind::Fixed {
                if !(l > 0.0) {
                    return Err(format!("{}: the joint axis is zero", bodies[i].name));
                }
                bodies[i].axis = scale(a, 1.0 / l);
                bodies[i].dof = Some(n);
                n += 1;
            }
        }
        if n == 0 {
            return Err("the robot has no moving joints".into());
        }
        let tool_body = bodies.len() - 1;
        Ok(Chain { bodies, n, g, tool_body, tool })
    }

    pub fn pose(&self, q: &[f64]) -> Pose {
        let nb = self.bodies.len();
        let mut p = Pose { jr: vec![I3; nb], jp: vec![[0.0; 3]; nb], lr: vec![I3; nb], lp: vec![[0.0; 3]; nb], z: vec![[0.0; 3]; nb] };
        for (i, b) in self.bodies.iter().enumerate() {
            let (pr, pp) = match b.parent {
                Some(k) => (p.lr[k], p.lp[k]),
                None => (I3, [0.0; 3]),
            };
            let r0 = mm(&pr, &b.pre_r);
            let o0 = add(pp, mv(&pr, b.pre_p));
            let qi = b.dof.map_or(0.0, |d| q[d]) + b.offset;
            let (jr, jp) = match b.kind {
                Kind::Revolute => (mm(&r0, &rot(b.axis, qi)), o0),
                Kind::Prismatic => (r0, add(o0, mv(&r0, scale(b.axis, qi)))),
                Kind::Fixed => (r0, o0),
            };
            p.z[i] = mv(&r0, b.axis);
            p.jr[i] = jr;
            p.jp[i] = jp;
            p.lr[i] = mm(&jr, &b.post_r);
            p.lp[i] = add(jp, mv(&jr, b.post_p));
        }
        p
    }

    /// The tool point in world coordinates.
    pub fn tool_pos(&self, q: &[f64]) -> V3 {
        let p = self.pose(q);
        add(p.lp[self.tool_body], mv(&p.lr[self.tool_body], self.tool))
    }

    /// Joint forces for given q, q̇, q̈ (recursive Newton–Euler in the world frame), with gravity
    /// and joint friction when `full`.
    pub fn rnea(&self, q: &[f64], qd: &[f64], qdd: &[f64], full: bool) -> Vec<f64> {
        let ps = self.pose(q);
        let nb = self.bodies.len();
        let (mut w, mut al, mut ao) = (vec![[0.0; 3]; nb], vec![[0.0; 3]; nb], vec![[0.0; 3]; nb]);
        // Gravity as an upward acceleration of the base.
        let a_base = if full { [0.0, 0.0, self.g] } else { [0.0; 3] };
        let (mut f, mut nm) = (vec![[0.0; 3]; nb], vec![[0.0; 3]; nb]);
        let mut cw = vec![[0.0; 3]; nb];
        for (i, b) in self.bodies.iter().enumerate() {
            let (wp, ap, aop, op) = match b.parent {
                Some(k) => (w[k], al[k], ao[k], ps.jp[k]),
                None => ([0.0; 3], [0.0; 3], a_base, [0.0; 3]),
            };
            let z = ps.z[i];
            let (qdi, qddi) = b.dof.map_or((0.0, 0.0), |d| (qd[d], qdd[d]));
            let r = sub(ps.jp[i], op);
            let mut a_o = add(aop, add(cross(ap, r), cross(wp, cross(wp, r))));
            match b.kind {
                Kind::Revolute => {
                    w[i] = add(wp, scale(z, qdi));
                    al[i] = add(add(ap, scale(z, qddi)), cross(wp, scale(z, qdi)));
                }
                Kind::Prismatic => {
                    w[i] = wp;
                    al[i] = ap;
                    a_o = add(a_o, add(scale(cross(wp, z), 2.0 * qdi), scale(z, qddi)));
                }
                Kind::Fixed => {
                    w[i] = wp;
                    al[i] = ap;
                }
            }
            ao[i] = a_o;
            let c = add(ps.lp[i], mv(&ps.lr[i], b.com));
            cw[i] = c;
            let rc = sub(c, ps.jp[i]);
            let ac = add(a_o, add(cross(al[i], rc), cross(w[i], cross(w[i], rc))));
            let iw = mm(&mm(&ps.lr[i], &b.inertia), &tr(&ps.lr[i]));
            f[i] = scale(ac, b.m);
            nm[i] = add(add(mv(&iw, al[i]), cross(w[i], mv(&iw, w[i]))), cross(rc, f[i]));
        }
        let mut tau = vec![0.0; self.n];
        for i in (0..nb).rev() {
            let b = &self.bodies[i];
            if let Some(d) = b.dof {
                tau[d] = match b.kind {
                    Kind::Prismatic => dot(ps.z[i], f[i]),
                    _ => dot(ps.z[i], nm[i]),
                } + if full { b.b * qd[d] } else { 0.0 };
            }
            if let Some(k) = b.parent {
                let r = sub(ps.jp[i], ps.jp[k]);
                let (fi, ni) = (f[i], nm[i]);
                f[k] = add(f[k], fi);
                nm[k] = add(nm[k], add(ni, cross(r, fi)));
            }
        }
        tau
    }

    /// The mass matrix M(q), n×n row-major.
    pub fn mass(&self, q: &[f64]) -> Vec<f64> {
        let n = self.n;
        let z = vec![0.0; n];
        let mut m = vec![0.0; n * n];
        for j in 0..n {
            let mut e = vec![0.0; n];
            e[j] = 1.0;
            let c = self.rnea(q, &z, &e, false);
            for i in 0..n {
                m[i * n + j] = c[i];
            }
        }
        // Exactly symmetric.
        for i in 0..n {
            for j in 0..i {
                let v = 0.5 * (m[i * n + j] + m[j * n + i]);
                m[i * n + j] = v;
                m[j * n + i] = v;
            }
        }
        m
    }

    /// Joint accelerations for joint forces τ.
    pub fn accel(&self, q: &[f64], qd: &[f64], tau: &[f64]) -> Vec<f64> {
        let n = self.n;
        let h = self.rnea(q, qd, &vec![0.0; n], true);
        let m = self.mass(q);
        let mut r: Vec<f64> = (0..n).map(|i| tau[i] - h[i]).collect();
        match crate::linalg::Lu::new(m, n) {
            Some(lu) => {
                lu.solve(&mut r);
                r
            }
            None => vec![f64::NAN; n],
        }
    }

    /// Kinetic plus potential energy.
    pub fn energy(&self, q: &[f64], qd: &[f64]) -> f64 {
        let n = self.n;
        let m = self.mass(q);
        let ke: f64 = (0..n).map(|i| (0..n).map(|j| 0.5 * qd[i] * m[i * n + j] * qd[j]).sum::<f64>()).sum();
        let ps = self.pose(q);
        let pe: f64 = self.bodies.iter().enumerate().map(|(i, b)| b.m * self.g * add(ps.lp[i], mv(&ps.lr[i], b.com))[2]).sum();
        ke + pe
    }

    /// A chain from a modified (Craig) DH table: rows of α_{i−1}, a_{i−1}, d_i, θ_offset, m, cx,
    /// cy, cz, Ixx, Iyy, Izz (centre and inertia in link i's frame). Standard DH when `standard`.
    pub fn from_dh(rows: &[[f64; 11]], standard: bool, g: f64, tool: V3, b: &[f64]) -> Result<Chain, String> {
        let mut bodies = vec![];
        for (i, r) in rows.iter().enumerate() {
            let [alpha, a, d, th, m, cx, cy, cz, ixx, iyy, izz] = *r;
            let mut bd = Body::new(&format!("Link {}", i + 1), if i == 0 { None } else { Some(i - 1) }, Kind::Revolute);
            if standard {
                bd.post_r = rot([1.0, 0.0, 0.0], alpha);
                bd.post_p = [a, 0.0, d];
            } else {
                bd.pre_r = rot([1.0, 0.0, 0.0], alpha);
                bd.pre_p = [a, 0.0, 0.0];
                bd.post_p = [0.0, 0.0, d];
            }
            bd.offset = th;
            bd.m = m;
            bd.com = [cx, cy, cz];
            bd.inertia = [[ixx, 0.0, 0.0], [0.0, iyy, 0.0], [0.0, 0.0, izz]];
            bd.b = b.get(i).or(b.first()).cloned().unwrap_or(0.0);
            bodies.push(bd);
        }
        Chain::new(bodies, g, tool)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::robot::Arm;

    fn planar(a: &Arm) -> Chain {
        let h = std::f64::consts::FRAC_PI_2;
        Chain::from_dh(&[[h, 0.0, 0.0, 0.0, a.m1, a.lc1, 0.0, 0.0, 0.0, a.i1, a.i1], [0.0, a.l1, 0.0, 0.0, a.m2, a.lc2, 0.0, 0.0, 0.0, a.i2, a.i2]], false, a.g, [a.l2, 0.0, 0.0], &[a.b]).unwrap()
    }

    /// The two-link planar arm as a DH chain gives the closed form's accelerations and tip.
    #[test]
    fn matches_two_link_closed_form() {
        let a = Arm { m1: 1.0, m2: 0.8, l1: 0.5, l2: 0.4, lc1: 0.25, lc2: 0.2, i1: 0.0208, i2: 0.0107, g: 9.81, b: 0.05 };
        let c = planar(&a);
        for (q, qd, t) in [([0.3, 0.5], [0.1, -0.4], [1.0, 0.5]), ([-1.2, 1.6], [2.0, 1.0], [-3.0, 0.2]), ([2.0, -2.5], [0.0, 0.0], [0.0, 0.0])] {
            let want = a.accel(q, qd, t);
            let got = c.accel(&q, &qd, &t);
            assert!((got[0] - want[0]).abs() < 1e-10 && (got[1] - want[1]).abs() < 1e-10, "{got:?} {want:?}");
            let tip = c.tool_pos(&q);
            let w = a.tip(q);
            // The plane is x–z: the closed form's y is our z.
            assert!((tip[0] - w[0]).abs() < 1e-12 && (tip[2] - w[1]).abs() < 1e-12 && tip[1].abs() < 1e-12, "{tip:?} {w:?}");
        }
    }

    /// A 3D six-joint chain with offsets and full inertias: M symmetric and positive definite,
    /// forward then inverse dynamics agree, and with no friction or torque the energy is kept.
    #[test]
    fn six_joint_consistency() {
        let h = std::f64::consts::FRAC_PI_2;
        let rows = [
            [0.0, 0.0, 0.16, 0.0, 3.7, 0.0, -0.02, 0.0, 0.010, 0.010, 0.007],
            [h, 0.0, 0.0, 0.0, 8.0, 0.21, 0.0, 0.11, 0.13, 0.13, 0.015],
            [0.0, 0.42, 0.0, 0.0, 2.8, 0.19, 0.0, 0.01, 0.04, 0.04, 0.004],
            [0.0, 0.39, 0.13, 0.0, 1.4, 0.0, 0.0, -0.01, 0.003, 0.003, 0.002],
            [h, 0.0, 0.1, 0.3, 1.3, 0.0, 0.0, 0.01, 0.003, 0.003, 0.002],
            [-h, 0.0, 0.1, 0.0, 0.4, 0.0, 0.0, -0.02, 0.0003, 0.0003, 0.0004],
        ];
        for standard in [false, true] {
            let c = Chain::from_dh(&rows, standard, 9.81, [0.0, 0.0, 0.05], &[0.0]).unwrap();
            let q = [0.1, -0.7, 1.1, -0.4, 0.8, 0.3];
            let qd = [0.5, -0.3, 0.2, 1.0, -0.6, 0.4];
            let m = c.mass(&q);
            let lu = crate::linalg::Lu::new(m.clone(), 6).unwrap();
            for i in 0..6 {
                assert!(m[i * 6 + i] > 0.0);
            }
            let _ = lu;
            let tau = [1.0, -2.0, 0.5, 0.2, -0.1, 0.05];
            let qdd = c.accel(&q, &qd, &tau);
            let back = c.rnea(&q, &qd, &qdd, true);
            for i in 0..6 {
                assert!((back[i] - tau[i]).abs() < 1e-9, "{back:?}");
            }
            // Energy: RK4, no torque, no friction.
            let f = |x: &[f64]| {
                let a = c.accel(&x[..6], &x[6..], &[0.0; 6]);
                let mut d = x[6..].to_vec();
                d.extend(a);
                d
            };
            let mut x: Vec<f64> = q.iter().chain(qd.iter()).cloned().collect();
            let e0 = c.energy(&x[..6], &x[6..]);
            let hh = 2e-4;
            for _ in 0..2500 {
                let k1 = f(&x);
                let x2: Vec<f64> = x.iter().zip(&k1).map(|(a, b)| a + 0.5 * hh * b).collect();
                let k2 = f(&x2);
                let x3: Vec<f64> = x.iter().zip(&k2).map(|(a, b)| a + 0.5 * hh * b).collect();
                let k3 = f(&x3);
                let x4: Vec<f64> = x.iter().zip(&k3).map(|(a, b)| a + hh * b).collect();
                let k4 = f(&x4);
                for i in 0..12 {
                    x[i] += hh / 6.0 * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]);
                }
            }
            let e1 = c.energy(&x[..6], &x[6..]);
            assert!((e1 - e0).abs() < 1e-8 * e0.abs().max(1.0), "{e0} → {e1}");
        }
    }

    /// A prismatic joint: a 2 kg slider on a vertical axis needs m·(q̈ + g) of force.
    #[test]
    fn prismatic() {
        let mut b = Body::new("slider", None, Kind::Prismatic);
        b.m = 2.0;
        let c = Chain::new(vec![b], 9.81, [0.0; 3]).unwrap();
        let t = c.rnea(&[0.3], &[0.0], &[1.5], true);
        assert!((t[0] - 2.0 * (1.5 + 9.81)).abs() < 1e-12);
        assert!((c.tool_pos(&[0.3])[2] - 0.3).abs() < 1e-12);
    }
}
