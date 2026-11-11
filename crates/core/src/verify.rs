//! Verification: blocks and whole models against closed-form answers. The same cases feed the
//! verification record in docs/verification/.

#[cfg(test)]
mod tests {
    use crate::compile::{compile, simulate, Results};
    use crate::doc::{Doc, Op};
    use crate::examples;
    use crate::model::Model;
    use std::collections::BTreeMap;

    fn none() -> BTreeMap<String, f64> {
        BTreeMap::new()
    }

    /// Build a model with ops, as the window would.
    struct Mk {
        d: Doc,
    }
    impl Mk {
        fn new(stop: f64, solver: &str) -> Mk {
            let mut d = Doc::new(Model::new());
            let mut s = d.m.sim.clone();
            s.stop = stop;
            s.solver = solver.into();
            s.step = 1e-3;
            s.rtol = 1e-8;
            s.atol = 1e-10;
            d.apply(Op::SetSim { sim: s }).unwrap();
            Mk { d }
        }
        fn add(&mut self, kind: &str, params: &[(&str, &str)]) -> u32 {
            let id = self.d.apply(Op::Add { parent: 0, kind: kind.into(), x: 0.0, y: 0.0 }).unwrap()[0];
            for (k, v) in params {
                self.d.apply(Op::SetParam { id, key: (*k).into(), value: (*v).into() }).unwrap();
            }
            id
        }
        fn wire(&mut self, a: u32, ap: usize, b: u32, bp: usize) {
            self.d.apply(Op::Connect { from: (a, ap), to: (b, bp), pts: vec![] }).unwrap();
        }
        fn run(&self) -> Results {
            simulate(&self.d.m, None, &none()).unwrap_or_else(|e| panic!("{e:?}"))
        }
    }

    fn at(r: &Results, ch: usize, t: f64) -> f64 {
        let i = r.t.partition_point(|x| *x < t).min(r.t.len() - 1);
        if i == 0 || (r.t[i] - t).abs() < 1e-12 {
            return r.y[ch][i];
        }
        let (t0, t1) = (r.t[i - 1], r.t[i]);
        r.y[ch][i - 1] + (t - t0) / (t1 - t0) * (r.y[ch][i] - r.y[ch][i - 1])
    }

    fn max_err(r: &Results, ch: usize, f: impl Fn(f64) -> f64) -> f64 {
        r.t.iter().zip(&r.y[ch]).map(|(t, y)| (y - f(*t)).abs()).fold(0.0, f64::max)
    }

    /// First-order lag 1/(τs+1) to a unit step at t = 0.5, with every solver.
    #[test]
    fn first_order_step_all_solvers() {
        for (solver, tol) in [("rk45", 1e-6), ("stiff", 2e-4), ("rk4", 1e-9), ("heun", 1e-6), ("euler", 2e-3)] {
            let mut m = Mk::new(5.0, solver);
            let s = m.add("Step", &[("time", "0.5")]);
            let g = m.add("TransferFcn", &[("num", "[1]"), ("den", "[2 1]")]);
            let sc = m.add("Scope", &[]);
            m.wire(s, 0, g, 0);
            m.wire(g, 0, sc, 0);
            let r = m.run();
            let e = max_err(&r, 0, |t| if t < 0.5 { 0.0 } else { 1.0 - (-(t - 0.5) / 2.0).exp() });
            assert!(e < tol, "{solver}: error {e}");
            assert!(r.t.contains(&0.5), "{solver}: the step time is hit exactly");
        }
    }

    /// The mass–spring–damper example against the underdamped closed form.
    #[test]
    fn mass_spring_damper_closed_form() {
        let mut model = examples::get("msd").unwrap();
        model.sim.rtol = 1e-9;
        model.sim.atol = 1e-11;
        let r = simulate(&model, None, &none()).unwrap();
        let (m, k, z): (f64, f64, f64) = (1.0, 10.0, 0.2);
        let wn = (k / m).sqrt();
        let wd = wn * (1.0 - z * z).sqrt();
        let xs = 10.0 / k;
        let exact = |t: f64| {
            if t < 0.5 {
                return 0.0;
            }
            let t = t - 0.5;
            xs * (1.0 - (-z * wn * t).exp() * ((wd * t).cos() + z / (1.0 - z * z).sqrt() * (wd * t).sin()))
        };
        assert!(max_err(&r, 0, exact) < 1e-6, "error {}", max_err(&r, 0, exact));
        // Peak overshoot exp(−ζπ/√(1−ζ²))
        let peak = r.y[0].iter().cloned().fold(0.0, f64::max);
        let mp = (-z * std::f64::consts::PI / (1.0 - z * z).sqrt()).exp();
        assert!((peak / xs - 1.0 - mp).abs() < 1e-3, "overshoot {}", peak / xs - 1.0);
    }

    /// P control of an integrator: y = 1 − e^(−K t).
    #[test]
    fn proportional_loop() {
        let mut m = Mk::new(3.0, "rk45");
        let s = m.add("Step", &[("time", "0")]);
        let e = m.add("Sum", &[("signs", "+-")]);
        let c = m.add("PID", &[("P", "4")]);
        let i = m.add("Integrator", &[]);
        let sc = m.add("Scope", &[]);
        m.wire(s, 0, e, 0);
        m.wire(e, 0, c, 0);
        m.wire(c, 0, i, 0);
        m.wire(i, 0, e, 1);
        m.wire(i, 0, sc, 0);
        let r = m.run();
        assert!(max_err(&r, 0, |t| 1.0 - (-4.0 * t).exp()) < 1e-6);
    }

    /// PI control of a first-order plant 1/(s+1) with Kp = 1, Ki = 1: the loop is 1/(s+1)
    /// (pole-zero cancellation), so y = 1 − e^(−t).
    #[test]
    fn pi_loop_closed_form() {
        let mut m = Mk::new(5.0, "rk45");
        let s = m.add("Step", &[("time", "0")]);
        let e = m.add("Sum", &[("signs", "+-")]);
        let c = m.add("PID", &[("P", "1"), ("I", "1")]);
        let p = m.add("TransferFcn", &[("num", "[1]"), ("den", "[1 1]")]);
        let sc = m.add("Scope", &[]);
        m.wire(s, 0, e, 0);
        m.wire(e, 0, c, 0);
        m.wire(c, 0, p, 0);
        m.wire(p, 0, e, 1);
        m.wire(p, 0, sc, 0);
        let r = m.run();
        assert!(max_err(&r, 0, |t| 1.0 - (-t).exp()) < 1e-6);
    }

    /// A transport delay reproduces its input exactly, later.
    #[test]
    fn transport_delay() {
        let mut m = Mk::new(4.0, "rk45");
        let s = m.add("Sine", &[("freq", "0.5")]);
        let d = m.add("Delay", &[("delay", "0.7")]);
        let sc = m.add("Scope", &[]);
        m.wire(s, 0, d, 0);
        m.wire(d, 0, sc, 0);
        let r = m.run();
        let w = std::f64::consts::PI;
        assert!(max_err(&r, 0, |t| if t < 0.7 { 0.0 } else { (w * (t - 0.7)).sin() }) < 1e-3);
    }

    /// Static blocks: sum, gain, product, saturation, dead zone, lookup, function, mux/demux.
    #[test]
    fn static_blocks() {
        let mut m = Mk::new(1.0, "rk4");
        let c = m.add("Constant", &[("value", "[1 -2 3]")]);
        let g = m.add("Gain", &[("k", "[1 2 3]")]);
        let sat = m.add("Saturation", &[("upper", "2"), ("lower", "-1")]);
        let dz = m.add("DeadZone", &[]);
        let lu = m.add("Lookup1D", &[("x", "[0 1 2]"), ("y", "[0 10 40]")]);
        let dm = m.add("Demux", &[("n", "3")]);
        let f = m.add("Fcn", &[("expr", "u1*u2 + u3"), ("n", "3")]);
        let mx = m.add("Mux", &[("n", "2")]);
        let sel = m.add("Selector", &[("idx", "[3 1]")]);
        let sc = m.add("Scope", &[("n", "6")]);
        m.wire(c, 0, g, 0); // [1 -4 9]
        m.wire(g, 0, sat, 0); // [1 -1 2]
        m.wire(g, 0, dz, 0); // [0.5 -3.5 8.5]
        m.wire(sat, 0, lu, 0); // [10 0 40]
        m.wire(g, 0, dm, 0);
        m.wire(dm, 0, f, 0);
        m.wire(dm, 1, f, 1);
        m.wire(dm, 2, f, 2); // 1·(−4) + 9 = 5
        m.wire(f, 0, mx, 0);
        m.wire(lu, 0, mx, 1); // [5 10 0 40]
        m.wire(mx, 0, sel, 0); // [0 5]
        for (k, b) in [sat, dz, lu, f, mx, sel].iter().enumerate() {
            m.wire(*b, 0, sc, k);
        }
        let r = m.run();
        let last = |k: usize| r.y[k][r.t.len() - 1];
        let want = [1.0, -1.0, 2.0, 0.5, -3.5, 8.5, 10.0, 0.0, 40.0, 5.0, 5.0, 10.0, 0.0, 40.0, 0.0, 5.0];
        assert_eq!(r.chans.len(), want.len());
        for (k, w) in want.iter().enumerate() {
            assert!((last(k) - w).abs() < 1e-12, "channel {} = {} want {w}", r.chans[k].name, last(k));
        }
    }

    /// The N-joint arm block: held by its gravity torques (computed by the engine's own inverse
    /// dynamics at the start pose) it stays still; with no torque it falls.
    #[test]
    fn robot_arm_n_block() {
        let mut m = Mk::new(1.0, "rk45");
        let a = m.add("RobotArm", &[("b", "0"), ("robot", "DH table")]);
        // The gravity torques at q0 = [0 -0.5 1 0 0 0].
        let def = crate::blocks::def("RobotArm").unwrap();
        let get = |k: &str| def.params.iter().find(|x| x.key == k).unwrap().def.to_string();
        let mm = |k: &str| crate::expr::eval(&get(k), &|_| None).unwrap();
        let vars = Default::default();
        let mut pv = crate::rt::Pv { m: Default::default(), t: Default::default(), vars: &vars, base: None };
        for k in ["dh", "tool", "g", "b", "q0"] {
            pv.m.insert(k.into(), mm(k));
        }
        pv.m.insert("b".into(), crate::expr::Mat::scalar(0.0));
        pv.t.insert("robot".into(), "DH table".into());
        pv.t.insert("conv".into(), "modified".into());
        let ch = crate::robot::chain(&pv).unwrap();
        let q0 = [0.0, -0.5, 1.0, 0.0, 0.0, 0.0];
        let g = ch.rnea(&q0, &[0.0; 6], &[0.0; 6], true);
        let c = m.add("Constant", &[("value", &format!("[{}]", g.iter().map(|v| format!("{v:.17e}")).collect::<Vec<_>>().join(" ")))]);
        let s = m.add("Scope", &[]);
        m.wire(c, 0, a, 0);
        m.wire(a, 0, s, 0);
        let r = m.run();
        let drift = r.y.iter().zip(q0).map(|(y, q)| (y.last().unwrap() - q).abs()).fold(0.0, f64::max);
        assert!(drift < 1e-6, "{drift}");
        m.d.apply(Op::SetParam { id: c, key: "value".into(), value: "0".into() }).unwrap();
        let r = m.run();
        assert!((r.y[1].last().unwrap() + 0.5).abs() > 0.1, "it falls");
    }

    /// The 7-joint arm under computed torque follows a joint trajectory (exact model: error below
    /// 1e-5 rad); with PD + gravity it settles on the final way point; joint motors at stall give
    /// η·N·Kt·V/R.
    #[test]
    fn arm_controllers_and_motors() {
        for law in ["computed torque", "PD + gravity"] {
            let mut m = Mk::new(4.0, "rk45");
            m.d.m.sim.rtol = 1e-8;
            let a = m.add("RobotArm", &[("robot", "7-joint arm"), ("q0", "[0 0 0 -1.5 0 1.5 0.8]"), ("b", "0.5")]);
            m.d.apply(Op::Rename { id: a, name: "Robot arm".into() }).unwrap();
            let tr = m.add("ArmTrajectory", &[("points", "[0 0 0 0 -1.5 0 1.5 0.8; 2 0.5 -0.3 0.2 -2 0.3 1.8 0.5]")]);
            let c = m.add("ArmControllerN", &[("law", law), ("Kp", "400"), ("Kd", "40")]);
            let s = m.add("Scope", &[("n", "2")]);
            for k in 0..3 {
                m.wire(tr, k, c, k);
            }
            m.wire(a, 0, c, 3);
            m.wire(a, 1, c, 4);
            m.wire(c, 0, a, 0);
            m.wire(a, 0, s, 0);
            m.wire(tr, 0, s, 1);
            let r = m.run();
            let err = (0..r.t.len()).map(|i| (0..7).map(|k| (r.y[k][i] - r.y[7 + k][i]).abs()).fold(0.0, f64::max)).fold(0.0, f64::max);
            let fin = (0..7).map(|k| (r.y[k].last().unwrap() - r.y[7 + k].last().unwrap()).abs()).fold(0.0, f64::max);
            if law == "computed torque" {
                assert!(err < 1e-5, "{law}: {err}");
            } else {
                assert!(fin < 1e-3, "{law}: final error {fin}");
            }
        }
        let mut m = Mk::new(0.5, "rk45");
        let v = m.add("Constant", &[("value", "[12 6]")]);
        let w = m.add("Constant", &[("value", "[0 0]")]);
        let d = m.add("JointDrive", &[]);
        let s = m.add("Scope", &[]);
        m.wire(v, 0, d, 0);
        m.wire(w, 0, d, 1);
        m.wire(d, 0, s, 0);
        let r = m.run();
        let want = |vv: f64| 0.9 * 100.0 * 0.1 * vv / 0.5;
        assert!((r.y[0].last().unwrap() - want(12.0)).abs() < 1e-6 && (r.y[1].last().unwrap() - want(6.0)).abs() < 1e-6);
    }

    /// Tool path → inverse kinematics → the arm's own forward kinematics: the joint values put
    /// the 6-joint arm's tool on the path. (A Gain feeds the angles to a scope, unconnected to the
    /// arm's dynamics.)
    #[test]
    fn tool_path_ik() {
        let mut m = Mk::new(6.0, "rk4");
        m.d.m.sim.step = 0.02;
        let a = m.add("RobotArm", &[]);
        m.d.apply(Op::Rename { id: a, name: "Robot arm".into() }).unwrap();
        let z = m.add("Constant", &[("value", "0")]);
        m.wire(z, 0, a, 0);
        let tp = m.add("ToolPath", &[]);
        let ik = m.add("ArmIKN", &[]);
        let s = m.add("Scope", &[("n", "2")]);
        m.wire(tp, 0, ik, 0);
        m.wire(ik, 0, s, 0);
        m.wire(tp, 0, s, 1);
        let r = m.run();
        let c = crate::urdf::library("arm6", 9.81).unwrap().chain;
        let mut worst = 0.0f64;
        for i in (0..r.t.len()).step_by(10) {
            let q: Vec<f64> = (0..6).map(|k| r.y[k][i]).collect();
            let p = c.tool_pos(&q);
            for k in 0..3 {
                worst = worst.max((p[k] - r.y[6 + k][i]).abs());
            }
        }
        assert!(worst < 1e-8, "{worst}");
        // An IK block naming no arm is a model error that says so.
        m.d.apply(Op::SetParam { id: ik, key: "arm".into(), value: "Nope".into() }).unwrap();
        let e = simulate(&m.d.m, None, &none()).unwrap_err();
        assert!(e.iter().any(|i| i.msg.contains("no Robot arm (any joints) block named 'Nope'")), "{e:?}");
    }

    /// The 3D view's frames: the 6-joint arm falling under gravity, fed to a RobotView. Each
    /// frame's tool matches the chain's own forward kinematics at the logged angles.
    #[test]
    fn robot_view_frames() {
        let mut m = Mk::new(0.5, "rk45");
        let a = m.add("RobotArm", &[]);
        let z = m.add("Constant", &[("value", "0")]);
        let v = m.add("RobotView", &[]);
        m.wire(z, 0, a, 0);
        m.wire(a, 0, v, 0);
        let r = m.run();
        let f = crate::robot::frames(&m.d.m, None, v, &r, 50).unwrap();
        assert_eq!(f.lib, "arm6");
        assert!(f.t.len() <= 52 && !f.meshes.is_empty() && f.links[0].len() == 12 * f.parents.len());
        let last = f.tool.last().unwrap();
        assert!(last.iter().all(|x| x.is_finite()));
        // Unconnected or fed from elsewhere: a clear message.
        let mut m2 = Mk::new(0.1, "rk45");
        let v2 = m2.add("RobotView", &[]);
        let r2 = m2.run();
        assert!(crate::robot::frames(&m2.d.m, None, v2, &r2, 50).unwrap_err().contains("Robot arm"));
    }

    /// The robot arm example: computed torque with an exact model makes each joint follow its
    /// trajectory (the error stays below 1e-4 rad), and the joint angles reach the last way point.
    #[test]
    fn robot_arm_tracks() {
        let m = examples::get("robotarm").unwrap();
        let r = simulate(&m, None, &none()).unwrap();
        // Channels: q (2) and q_ref (2) into the Joint angles scope.
        let ix = |name: &str| r.chans.iter().position(|c| c.name == name).unwrap_or_else(|| panic!("{name}: {:?}", r.chans.iter().map(|c| &c.name).collect::<Vec<_>>()));
        let (q1, q2, r1, r2) = (ix("Arm.q[1]"), ix("Arm.q[2]"), ix("Trajectory.q_ref[1]"), ix("Trajectory.q_ref[2]"));
        let err = (0..r.t.len()).map(|i| (r.y[q1][i] - r.y[r1][i]).abs().max((r.y[q2][i] - r.y[r2][i]).abs())).fold(0.0, f64::max);
        assert!(err < 1e-4, "{err}");
        assert!((r.y[q1].last().unwrap() + 1.2).abs() < 1e-4 && (r.y[q2].last().unwrap() - 1.6).abs() < 1e-4);
    }

    /// Saturation at no load: at synchronous speed (a huge inertia holds it) the rotor carries no
    /// current, so V = Rs·I + jω(Lls·I + ψ_m) with I = ψ_m/Lm·(1 + (ψ_m/ψ_sat)^k). The stator
    /// current's peak matches that, and is larger than without saturation; the iron loss is
    /// 1.5·(ω·ψ_m)²/R_fe.
    #[test]
    fn induction_saturation_and_iron_loss() {
        let w = 2.0 * std::f64::consts::PI * 50.0;
        let run = |psat: &str| {
            let mut m = Mk::new(1.0, "rk45");
            m.d.m.sim.rtol = 1e-8;
            m.d.m.sim.max_step = 1e-4;
            let v = m.add("ThreePhase", &[("Vll", "400"), ("f", "50")]);
            let tl = m.add("Constant", &[("value", "0")]);
            let im = m.add("InductionSC", &[("Rs", "0.5"), ("Lls", "0.003"), ("Lm", "0.1"), ("Llr", "0.003"), ("Rr", "0.4"), ("p", "2"), ("J", "1e9"), ("w0", &format!("{}", w / 2.0)), ("psat", psat), ("ksat", "6"), ("Rfe", "300")]);
            let sc = m.add("Scope", &[("n", "2")]);
            m.wire(v, 0, im, 0);
            m.wire(tl, 0, im, 1);
            m.wire(im, 2, sc, 0);
            m.wire(im, 4, sc, 1);
            let r = m.run();
            let n0 = r.t.partition_point(|t| *t < 0.9);
            let peak = r.y[0][n0..].iter().fold(0.0f64, |a, v| a.max(v.abs()));
            (peak, *r.y[3].last().unwrap())
        };
        let vp = 400.0 * 2f64.sqrt() / 3f64.sqrt();
        // Solve the no-load phasor equation for ψ_m by bisection.
        let solve = |psat: f64| {
            let cur = |psi: f64| psi / 0.1 * (1.0 + if psat > 0.0 { (psi / psat).powi(6) } else { 0.0 });
            let (mut lo, mut hi) = (0.0, 2.0);
            for _ in 0..200 {
                let mid = 0.5 * (lo + hi);
                let i = cur(mid);
                if ((0.5 * i).powi(2) + (w * (0.003 * i + mid)).powi(2)).sqrt() > vp { hi = mid } else { lo = mid }
            }
            (cur(lo), lo)
        };
        let (i_lin, _) = solve(0.0);
        let (i_sat, psi) = solve(0.9);
        let (p_lin, _) = run("0");
        let (p_sat, pfe) = run("0.9");
        assert!((p_lin / i_lin - 1.0).abs() < 2e-3 && (p_sat / i_sat - 1.0).abs() < 2e-3, "lin {p_lin} vs {i_lin}; sat {p_sat} vs {i_sat}");
        assert!(p_sat > 1.3 * p_lin, "{p_sat} {p_lin}");
        let want = 1.5 * (w * psi).powi(2) / 300.0;
        assert!((pfe / want - 1.0).abs() < 5e-3, "{pfe} vs {want}");
    }

    /// Inverter losses with currents (10, −5, −5) A on 600 V: conduction 1.1·20 + 0.01·150 = 23.5 W,
    /// switching 10 kHz·2 mJ·(20/100)·(600/300) = 8 W. A thermal network under a 100 W step:
    /// T = 40 + Σ 100·Rᵢ·(1 − e^(−t/τᵢ)).
    #[test]
    fn losses_and_thermal() {
        let mut m = Mk::new(0.001, "rk45");
        let i = m.add("Constant", &[("value", "[10 -5 -5]")]);
        let dc = m.add("Constant", &[("value", "600")]);
        let l = m.add("InverterLoss", &[]);
        let sc = m.add("Scope", &[("n", "3")]);
        m.wire(i, 0, l, 0);
        m.wire(dc, 0, l, 1);
        for k in 0..3 {
            m.wire(l, k, sc, k);
        }
        let r = m.run();
        let v: Vec<f64> = r.y.iter().map(|y| *y.last().unwrap()).collect();
        assert!((v[0] - 23.5).abs() < 1e-12 && (v[1] - 8.0).abs() < 1e-12 && (v[2] - 31.5).abs() < 1e-12, "{v:?}");
        let mut m = Mk::new(3.0, "rk45");
        m.d.m.sim.rtol = 1e-9;
        let p = m.add("Step", &[("time", "0"), ("final", "100")]);
        let th = m.add("ThermalNetwork", &[]);
        let sc = m.add("Scope", &[]);
        m.wire(p, 0, th, 0);
        m.wire(th, 0, sc, 0);
        let r = m.run();
        let want = |t: f64| 40.0 + [(0.1, 0.001), (0.2, 0.01), (0.3, 1.0)].iter().map(|(rr, tau)| 100.0 * rr * (1.0 - (-t / tau).exp())).sum::<f64>();
        assert!(max_err(&r, 0, want) < 1e-5, "{}", max_err(&r, 0, want));
    }

    /// Space-vector modulation: references of amplitude 1.1 (beyond the sine-PWM limit of 1) give
    /// phase voltages of exactly 1.1·V_dc/2 with the common-mode offset, and are clipped without
    /// it. Dead time: with m = 0 and currents (10, −5, −5) A, the poles lose ±V_dc·td·fc, so
    /// v_a = −4/3·e and v_b = v_c = 2/3·e with e = V_dc·td·fc.
    #[test]
    fn svpwm_and_dead_time() {
        let run = |modn: &str| {
            let mut m = Mk::new(0.02, "rk4");
            m.d.m.sim.step = 1e-5;
            let src = m.add("ThreePhase", &[("Vll", &format!("{}", 1.1 * 3f64.sqrt() / 2f64.sqrt())), ("f", "50")]);
            let dc = m.add("Constant", &[("value", "2")]);
            let inv = m.add("Inverter", &[("mod", modn)]);
            let sc = m.add("Scope", &[]);
            m.wire(src, 0, inv, 0);
            m.wire(dc, 0, inv, 1);
            m.wire(inv, 0, sc, 0);
            let r = m.run();
            r.t.iter().zip(&r.y[0]).map(|(t, v)| (v - 1.1 * (2.0 * std::f64::consts::PI * 50.0 * t).cos()).abs()).fold(0.0, f64::max)
        };
        assert!(run("space vector") < 1e-9, "{}", run("space vector"));
        assert!(run("sine") > 0.05, "{}", run("sine"));
        let mut m = Mk::new(0.001, "rk4");
        let z = m.add("Constant", &[("value", "[0 0 0]")]);
        let dc = m.add("Constant", &[("value", "600")]);
        let i = m.add("Constant", &[("value", "[10 -5 -5]")]);
        let inv = m.add("Inverter", &[("td", "2e-6"), ("fc", "10000")]);
        let sc = m.add("Scope", &[]);
        m.wire(z, 0, inv, 0);
        m.wire(dc, 0, inv, 1);
        m.wire(i, 0, inv, 2);
        m.wire(inv, 0, sc, 0);
        let r = m.run();
        let e = 600.0 * 2e-6 * 10000.0 * (50f64).tanh();
        let v: Vec<f64> = r.y.iter().map(|y| *y.last().unwrap()).collect();
        assert!((v[0] + 4.0 / 3.0 * e).abs() < 1e-9 && (v[1] - 2.0 / 3.0 * e).abs() < 1e-9 && (v[2] - 2.0 / 3.0 * e).abs() < 1e-9, "{v:?}");
    }

    /// Algebraic loops are solved: y = u − k·y gives u/(1 + k); y + y³ = 2 gives 1; and inside
    /// a dynamic loop x' = −y, y = x − k·y the state decays as e^(−t/(1+k)).
    #[test]
    fn algebraic_loops_solved() {
        let mut m = Mk::new(1.0, "rk45");
        let u = m.add("Constant", &[("value", "3")]);
        let s = m.add("Sum", &[("signs", "+-")]);
        let k = m.add("Gain", &[("k", "2")]);
        let sc = m.add("Scope", &[("n", "2")]);
        m.wire(u, 0, s, 0);
        m.wire(s, 0, k, 0);
        m.wire(k, 0, s, 1);
        m.wire(s, 0, sc, 0);
        // y + y³ = 2: y = 2 − y³.
        let two = m.add("Constant", &[("value", "2")]);
        let s2 = m.add("Sum", &[("signs", "+-")]);
        let cube = m.add("Fcn", &[("expr", "u^3")]);
        m.wire(two, 0, s2, 0);
        m.wire(s2, 0, cube, 0);
        m.wire(cube, 0, s2, 1);
        m.wire(s2, 0, sc, 1);
        let sim = compile(&m.d.m, None, &none()).unwrap();
        assert!(sim.warnings.iter().any(|w| w.msg.contains("solved by iteration")), "{:?}", sim.warnings);
        let r = m.run();
        assert!((r.y[0].last().unwrap() - 1.0).abs() < 1e-9 && (r.y[1].last().unwrap() - 1.0).abs() < 1e-9, "{} {}", r.y[0].last().unwrap(), r.y[1].last().unwrap());
        // Dynamic: x' = −y, y = x − 2y (so y = x/3), x(0) = 1.
        let mut m = Mk::new(3.0, "rk45");
        let i = m.add("Integrator", &[("ic", "1")]);
        let s = m.add("Sum", &[("signs", "+-")]);
        let k = m.add("Gain", &[("k", "2")]);
        let neg = m.add("Gain", &[("k", "-1")]);
        let sc = m.add("Scope", &[]);
        m.wire(i, 0, s, 0);
        m.wire(s, 0, k, 0);
        m.wire(k, 0, s, 1);
        m.wire(s, 0, neg, 0);
        m.wire(neg, 0, i, 0);
        m.wire(i, 0, sc, 0);
        let r = m.run();
        assert!(max_err(&r, 0, |t| (-t / 3.0).exp()) < 1e-5, "{}", max_err(&r, 0, |t| (-t / 3.0).exp()));
    }

    /// Buses: three signals (one of them a vector) grouped by a Bus Creator, passed through
    /// Goto/From tags, and picked out by name in another order: each arrives unchanged.
    #[test]
    fn bus_picks_by_name() {
        let mut m = Mk::new(1.0, "rk4");
        let a = m.add("Constant", &[("value", "2")]);
        let b = m.add("Constant", &[("value", "[3 4]")]);
        let c = m.add("Constant", &[("value", "5")]);
        let bc = m.add("BusCreator", &[("n", "3"), ("names", "speed, pos, torque")]);
        m.wire(a, 0, bc, 0);
        m.wire(b, 0, bc, 1);
        m.wire(c, 0, bc, 2);
        let bs = m.add("BusSelector", &[("signals", "torque, pos")]);
        // Through Goto/From tags.
        let go = m.add("Goto", &[("tag", "bus")]);
        let fr = m.add("From", &[("tag", "bus")]);
        m.wire(bc, 0, go, 0);
        m.wire(fr, 0, bs, 0);
        let s = m.add("Scope", &[("n", "2")]);
        m.wire(bs, 0, s, 0);
        m.wire(bs, 1, s, 1);
        let r = m.run();
        let last: Vec<f64> = r.y.iter().map(|y| *y.last().unwrap()).collect();
        assert_eq!(last, vec![5.0, 3.0, 4.0]);
        // An unknown name is a model error that names the bus's signals.
        m.d.apply(Op::SetParam { id: bs, key: "signals".into(), value: "torque, current".into() }).unwrap();
        let e = simulate(&m.d.m, None, &none()).unwrap_err();
        assert!(e.iter().any(|i| i.msg.contains("no signal 'current'") && i.msg.contains("speed, pos, torque")), "{e:?}");
    }

    /// Live tuning: a gain changed at t = 1 s while running gives the same result as a Step block
    /// that changes the gain's input at 1 s. Pacing at ×50 real time takes about t/50 seconds.
    #[test]
    fn live_change_matches_a_step() {
        use crate::compile::{changed_blocks, compile, run, Control};
        use std::sync::Arc;
        let mut m = Mk::new(2.0, "rk4");
        m.d.m.vars.push(crate::model::Var { name: "K".into(), expr: "1".into(), ..Default::default() });
        let c = m.add("Constant", &[("value", "1")]);
        let g = m.add("Gain", &[("k", "K")]);
        let i = m.add("Integrator", &[]);
        let s = m.add("Scope", &[]);
        m.wire(c, 0, g, 0);
        m.wire(g, 0, i, 0);
        m.wire(i, 0, s, 0);
        let model = m.d.m.clone();
        let (a, b): (BTreeMap<String, f64>, BTreeMap<String, f64>) = (BTreeMap::new(), [("K".to_string(), 3.0)].into());
        assert_eq!(changed_blocks(&model, None, &a, &b).unwrap(), vec![g]);
        let ctl = Arc::new(Control::default());
        let ctl2 = ctl.clone();
        let mut new_sim = compile(&model, None, &b).unwrap();
        let mut once = Some(new_sim.take_rts(&[g]));
        let mut cb = move |r: &Results| {
            if r.t.last().is_some_and(|t| *t >= 1.0 - 1e-9) {
                if let Some(sw) = once.take() {
                    ctl2.swap.lock().unwrap().extend(sw);
                }
            }
        };
        let live = run(compile(&model, None, &none()).unwrap(), Some(&ctl), Some(&mut cb)).unwrap();
        // The same with a Step from 1 to 3 at 1 s into the integrator.
        let mut m2 = Mk::new(2.0, "rk4");
        let st = m2.add("Step", &[("time", "1"), ("initial", "1"), ("final", "3")]);
        let i2 = m2.add("Integrator", &[]);
        let s2 = m2.add("Scope", &[]);
        m2.wire(st, 0, i2, 0);
        m2.wire(i2, 0, s2, 0);
        let step = m2.run();
        for t in [0.5, 1.0, 1.5, 2.0] {
            assert!((at(&live, 0, t) - at(&step, 0, t)).abs() < 1e-9, "t {t}: live {} step {}", at(&live, 0, t), at(&step, 0, t));
        }
        // Pacing: 2 s of model time at ×50 takes about 40 ms.
        let ctl = Control::default();
        ctl.set_pace(50.0);
        let t0 = std::time::Instant::now();
        run(compile(&model, None, &none()).unwrap(), Some(&ctl), None).unwrap();
        let el = t0.elapsed().as_secs_f64();
        assert!(el > 0.035 && el < 0.5, "{el}");
    }

    /// The DC motor built from blocks in a subsystem and the DC motor block agree.
    #[test]
    fn dc_motor_subsystem_matches_block() {
        let model = examples::get("dcmotor").unwrap();
        let r1 = simulate(&model, None, &none()).unwrap();
        // Same loop with the library block.
        let mut m = Mk::new(5.0, "rk45");
        for (n, v) in [("R", "1"), ("L", "0.5"), ("Ke", "0.01"), ("Kt", "0.01"), ("J", "0.01"), ("b", "0.1")] {
            m.d.m.vars.push(crate::model::Var { name: n.into(), expr: v.into(), ..Default::default() });
        }
        let r = m.add("Step", &[("time", "0.2")]);
        let e = m.add("Sum", &[("signs", "+-")]);
        let c = m.add("PID", &[("P", "100"), ("I", "200"), ("D", "10"), ("upper", "24"), ("lower", "-24")]);
        let tl = m.add("Step", &[("time", "3"), ("final", "0.05")]);
        let mo = m.add("DCMotor", &[("R", "R"), ("L", "L"), ("Ke", "Ke"), ("Kt", "Kt"), ("J", "J"), ("b", "b")]);
        let sc = m.add("Scope", &[]);
        m.wire(r, 0, e, 0);
        m.wire(e, 0, c, 0);
        m.wire(c, 0, mo, 0);
        m.wire(tl, 0, mo, 1);
        m.wire(mo, 0, e, 1);
        m.wire(mo, 0, sc, 0);
        let r2 = m.run();
        for t in [0.5, 1.0, 2.0, 3.5, 5.0] {
            let (a, b) = (at(&r1, 0, t), at(&r2, 0, t));
            assert!((a - b).abs() < 1e-4, "t = {t}: subsystem {a} vs block {b}");
        }
        // The controller holds the speed at 1 rad/s before and after the load step.
        assert!((at(&r1, 0, 2.9) - 1.0).abs() < 0.01);
        assert!((at(&r1, 0, 5.0) - 1.0).abs() < 0.02);
    }

    /// Every example compiles and runs, and its loops settle where they should.
    #[test]
    fn examples_run() {
        for ex in examples::list() {
            let m = examples::get(ex.id).unwrap();
            let r = simulate(&m, None, &none()).unwrap_or_else(|e| panic!("{}: {e:?}", ex.id));
            assert!(r.y.iter().all(|y| y.iter().all(|v| v.is_finite())), "{}", ex.id);
            let last = |k: usize| r.y[k][r.t.len() - 1];
            match ex.id {
                "pmsm" => assert!((last(0) - 100.0).abs() < 1.0, "pmsm speed {}", last(0)),
                "tank" => assert!((last(0) - 2.0).abs() < 0.02, "tank level {}", last(0)),
                "cruise" => assert!((last(0) - 25.0).abs() < 0.05, "cruise speed {}", last(0)),
                "dcpos" => assert!((last(0) - 1.0).abs() < 0.01, "angle {}", last(0)),
                // Induction motors run just below synchronous speed (157 rad/s) and carry their load.
                "imdol" => assert!((152.0..156.0).contains(&last(0)) && (last(1) - last(2)).abs() < 0.5, "imdol {} {} {}", last(0), last(1), last(2)),
                // The junction warms above the 40 °C ambient once the load is on.
                "harmonics" => {
                    let s = crate::measure::spectrum(&r.t, &r.y[0], 50.0, 4, 60).unwrap();
                    assert!(s.thd > 0.005 && s.thd < 0.5 && s.harmonics[0] > 5.0, "harmonics THD {} I1 {}", s.thd, s.harmonics[0]);
                }
                "imsat" => assert!((150.0..157.0).contains(&last(0)), "imsat speed {}", last(0)),
                "drivethermal" => assert!(last(r.y.len() - 1) > 41.0 && last(r.y.len() - 1) < 150.0, "junction {}", last(r.y.len() - 1)),
                "imvf" => assert!((150.0..156.0).contains(&last(0)) && (last(1) - 50.0).abs() < 0.5, "imvf {} {}", last(0), last(1)),
                "pmsmfoc" | "bldc" => assert!((last(0) - last(1)).abs() < 0.005 * last(1), "{} speed {} vs {}", ex.id, last(0), last(1)),
                "syncmotor" => assert!((last(0) - 50.0 * std::f64::consts::PI).abs() < 0.01 && (last(1) - 3000.0).abs() < 30.0, "sync {} {}", last(0), last(1)),
                "dcseries" => {
                    // Torque balance Laf·i² = 300 + 0.02·ω² at the end.
                    let (w, i) = (last(0), last(1));
                    assert!((0.0053 * i * i - 300.0 - 0.02 * w * w).abs() < 0.01 * 0.0053 * i * i, "dc series {w} {i}");
                }
                // Braked to standstill, carrying the load on i_q; the DC link rose while braking.
                "pmsmdigital" => assert!(last(0).abs() < 2.0 && (last(3) - 12.5).abs() < 0.5 && last(4) > 650.0, "pmsmdigital {} {} {}", last(0), last(3), last(4)),
                "dcdigital" => assert!((last(0) - 1.0).abs() < 0.01 && (last(1) - 1.0).abs() < 0.01, "dcdigital {} {}", last(0), last(1)),
                "srm" => assert!((last(0) - 150.0).abs() < 4.5, "srm speed {}", last(0)),
                // Each copy at its own no-load speed V/(Ke + R·b/Kt) with its own values.
                "masked" => {
                    let w = |r: f64, ke: f64, kt: f64, b: f64| 12.0 / (ke + r * b / kt);
                    assert!((last(0) - w(2.0, 0.01, 0.01, 1e-6)).abs() < 1e-3 * last(0) && (last(1) - w(0.2, 0.12, 0.12, 5e-5)).abs() < 1e-3 * last(1), "masked {} {}", last(0), last(1));
                }
                // Twenty full steps of π/100, give or take the last one in progress.
                "stepper" => assert!((last(0) - 0.2 * std::f64::consts::PI).abs() < 0.02, "stepper angle {}", last(0)),
                "thermostat" => {
                    let late: Vec<f64> = r.t.iter().zip(&r.y[0]).filter(|(t, _)| **t > 8000.0).map(|x| *x.1).collect();
                    let (lo, hi) = (late.iter().cloned().fold(f64::INFINITY, f64::min), late.iter().cloned().fold(f64::NEG_INFINITY, f64::max));
                    assert!(lo > 19.49 && hi < 20.51, "thermostat band {lo}…{hi}");
                    assert!(r.stats.events > 5);
                }
                _ => {}
            }
        }
    }

    #[test]
    fn algebraic_loop_is_reported() {
        // Solved, with a warning that names the loop.
        let mut m = Mk::new(1.0, "rk45");
        let s = m.add("Sum", &[("signs", "+-")]);
        let g = m.add("Gain", &[("k", "2")]);
        m.wire(s, 0, g, 0);
        m.wire(g, 0, s, 1);
        let w = compile(&m.d.m, None, &none()).unwrap().warnings;
        assert!(w.iter().any(|i| i.msg.starts_with("Algebraic loop ") && i.msg.contains("Gain") && i.msg.contains("Sum")), "{w:?}");
        // y = 1 + y has no solution: the run stops with an error that names the loop.
        let mut m = Mk::new(1.0, "rk45");
        let c = m.add("Constant", &[("value", "1")]);
        let s = m.add("Sum", &[("signs", "++")]);
        let g = m.add("Gain", &[("k", "1")]);
        m.wire(c, 0, s, 0);
        m.wire(s, 0, g, 0);
        m.wire(g, 0, s, 1);
        let e = simulate(&m.d.m, None, &none()).unwrap_err();
        assert!(e.iter().any(|i| i.msg.contains("algebraic loop") && i.msg.contains("no solution")), "{e:?}");
    }

    #[test]
    fn helpful_errors() {
        let mut m = Mk::new(1.0, "rk45");
        let g = m.add("Gain", &[("k", "Kx")]);
        let e = compile(&m.d.m, None, &none()).err().unwrap();
        assert!(e.iter().any(|i| i.id == g && i.msg.contains("unknown name 'Kx'")), "{e:?}");
        let mut m = Mk::new(1.0, "rk45");
        m.add("Gain", &[]);
        let s = compile(&m.d.m, None, &none()).unwrap();
        assert!(s.warnings.iter().any(|w| w.warn && w.msg.contains("not connected")));
    }

    /// A sweep: the same model at several values of a variable.
    #[test]
    fn variable_override() {
        let m = examples::get("msd").unwrap();
        let mut o = BTreeMap::new();
        o.insert("zeta".to_string(), 1.0);
        let r = simulate(&m, None, &o).unwrap();
        let peak = r.y[0].iter().cloned().fold(0.0, f64::max);
        assert!(peak <= 1.0 + 1e-6, "critically damped: no overshoot ({peak})");
    }

    /// Discrete blocks: a zero-order hold samples at its sample times; a unit delay lags one sample.
    #[test]
    fn discrete() {
        let mut m = Mk::new(1.0, "rk45");
        let c = m.add("Clock", &[]);
        let z = m.add("ZOH", &[("ts", "0.25")]);
        let u = m.add("UnitDelay", &[("ts", "0.25")]);
        let sc = m.add("Scope", &[("n", "2")]);
        m.wire(c, 0, z, 0);
        m.wire(c, 0, u, 0);
        m.wire(z, 0, sc, 0);
        m.wire(u, 0, sc, 1);
        let r = m.run();
        assert!((at(&r, 0, 0.6) - 0.5).abs() < 1e-12);
        assert!((at(&r, 1, 0.6) - 0.25).abs() < 1e-12);
    }

    // ------------------------------------------------------------ motor library (0.1.2)

    fn last(r: &Results, ch: usize) -> f64 {
        *r.y[ch].last().unwrap()
    }

    /// Apply a typical-ratings preset as the Inspector would.
    fn preset(kind: &str, i: usize) -> Vec<(String, String)> {
        crate::motors::presets(kind)[i].values.clone()
    }

    fn add_owned(m: &mut Mk, kind: &str, params: &[(String, String)], extra: &[(&str, &str)]) -> u32 {
        let mut v: Vec<(&str, &str)> = params.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        v.extend_from_slice(extra);
        m.add(kind, &v)
    }

    /// Air-gap torque of an induction machine from its exact equivalent circuit at slip s.
    /// Rotor branches are (R, X) pairs in parallel; all values per phase, Vph RMS.
    fn im_torque(vph: f64, f: f64, p: f64, rs: f64, xls: f64, xm: f64, rotor: &[(f64, f64)], s: f64) -> f64 {
        type C = (f64, f64);
        let add = |a: C, b: C| (a.0 + b.0, a.1 + b.1);
        let mul = |a: C, b: C| (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0);
        let inv = |a: C| { let d = a.0 * a.0 + a.1 * a.1; (a.0 / d, -a.1 / d) };
        let mut y = inv((0.0, xm));
        for (r, x) in rotor {
            y = add(y, inv((r / s, *x)));
        }
        let zp = inv(y);
        let z = add((rs, xls), zp);
        let i = mul((vph, 0.0), inv(z));
        let i2 = i.0 * i.0 + i.1 * i.1;
        let ws = 2.0 * std::f64::consts::PI * f / p;
        3.0 * i2 * zp.0 / ws
    }

    /// Induction motor at a held speed: the torque settles to the equivalent-circuit value.
    /// Squirrel-cage, double-cage and wound-rotor (short-circuited) machines.
    #[test]
    fn induction_torque_matches_equivalent_circuit() {
        use std::f64::consts::PI;
        let w = 2.0 * PI * 50.0;
        for kind in ["InductionSC", "InductionDC", "InductionWR"] {
            for s in [0.03, 0.2, 1.0] {
                // At standstill the decaying DC flux takes longest to die away.
                let mut m = Mk::new(if s == 1.0 { 12.0 } else { 3.0 }, "rk45");
                m.d.m.sim.rtol = 1e-9;
                let pr = preset(kind, 0);
                let w0 = format!("{}", (1.0 - s) * w / 2.0);
                let mot = add_owned(&mut m, kind, &pr, &[("J", "1e12"), ("w0", &w0)]);
                let src = m.add("ThreePhase", &[("Vll", "400"), ("f", "50")]);
                let tl = m.add("Constant", &[("value", "0")]);
                let sc = m.add("Scope", &[]);
                m.wire(src, 0, mot, 0);
                if kind == "InductionWR" {
                    let z = m.add("Constant", &[("value", "[0 0 0]")]);
                    m.wire(z, 0, mot, 1);
                    m.wire(tl, 0, mot, 2);
                } else {
                    m.wire(tl, 0, mot, 1);
                }
                m.wire(mot, 1, sc, 0);
                let r = m.run();
                let g = |k: &str| pr.iter().find(|(a, _)| a == k).unwrap().1.parse::<f64>().unwrap();
                let mut rotor = vec![(g("Rr"), w * g("Llr"))];
                if kind == "InductionDC" {
                    rotor.push((g("Rr2"), w * g("Llr2")));
                }
                let te = im_torque(400.0 / 3f64.sqrt(), 50.0, 2.0, g("Rs"), w * g("Lls"), w * g("Lm"), &rotor, s);
                let got = last(&r, 0);
                assert!((got - te).abs() < 2e-3 * te.abs(), "{kind} s = {s}: torque {got}, equivalent circuit {te}");
            }
        }
    }

    /// A three-phase source: phase peak V_LL·√2/√3, 120° apart.
    #[test]
    fn three_phase_source() {
        let mut m = Mk::new(0.02, "rk45");
        let src = m.add("ThreePhase", &[("Vll", "400"), ("f", "50")]);
        let sc = m.add("Scope", &[]);
        m.wire(src, 0, sc, 0);
        let r = m.run();
        let pk = 400.0 * 2f64.sqrt() / 3f64.sqrt();
        assert!((at(&r, 0, 0.0) - pk).abs() < 1e-9);
        assert!((at(&r, 1, 0.0) + pk / 2.0).abs() < 1e-9);
        assert!(max_err(&r, 2, |t| pk * (2.0 * std::f64::consts::PI * 50.0 * t + 2.0 * std::f64::consts::PI / 3.0).cos()) < 1e-9);
    }

    /// DC machines at steady state: separately excited no-load speed V/(Laf·i_f), series and
    /// compound torque and voltage balance.
    #[test]
    fn dc_machines_steady_state() {
        // Separately excited, no load, no friction: ω = Va / (Laf · Vf/Rf).
        let mut m = Mk::new(4.0, "rk45");
        let mot = m.add("DCSepEx", &[("b", "0")]);
        let va = m.add("Constant", &[("value", "220")]);
        let vf = m.add("Constant", &[("value", "220")]);
        let tl = m.add("Constant", &[("value", "0")]);
        let sc = m.add("Scope", &[]);
        m.wire(va, 0, mot, 0);
        m.wire(vf, 0, mot, 1);
        m.wire(tl, 0, mot, 2);
        m.wire(mot, 0, sc, 0);
        let r = m.run();
        assert!((last(&r, 0) - 220.0 / 1.3).abs() < 1e-3, "sep. excited speed {}", last(&r, 0));

        // Series: Laf·i² = T_L and V = (Ra + Rf)·i + Laf·i·ω.
        let mut m = Mk::new(10.0, "rk45");
        let mot = m.add("DCSeries", &[("b", "0")]);
        let v = m.add("Constant", &[("value", "220")]);
        let tl = m.add("Constant", &[("value", "20")]);
        let sc = m.add("Scope", &[("n", "2")]);
        m.wire(v, 0, mot, 0);
        m.wire(tl, 0, mot, 1);
        m.wire(mot, 0, sc, 0);
        m.wire(mot, 1, sc, 1);
        let r = m.run();
        let i = (20.0f64 / 0.052).sqrt();
        let w = (220.0 - 0.6 * i) / (0.052 * i);
        assert!((last(&r, 1) - i).abs() < 1e-4 * i, "series current {} vs {i}", last(&r, 1));
        assert!((last(&r, 0) - w).abs() < 1e-4 * w, "series speed {} vs {w}", last(&r, 0));

        // Compound: i_f = V/Rf; (Laf·i_f + Lafse·i_a)·i_a = T_L.
        let mut m = Mk::new(6.0, "rk45");
        let mot = m.add("DCCompound", &[("b", "0")]);
        let v = m.add("Constant", &[("value", "220")]);
        let tl = m.add("Constant", &[("value", "30")]);
        let sc = m.add("Scope", &[("n", "2")]);
        m.wire(v, 0, mot, 0);
        m.wire(tl, 0, mot, 1);
        m.wire(mot, 0, sc, 0);
        m.wire(mot, 1, sc, 1);
        let r = m.run();
        let (laf, lse) = (1.25, 0.005);
        let ia = (-laf + (laf * laf + 4.0 * lse * 30.0f64).sqrt()) / (2.0 * lse);
        let w = (220.0 - 0.6 * ia) / (laf + lse * ia);
        assert!((last(&r, 1) - ia).abs() < 1e-4 * ia, "compound current {} vs {ia}", last(&r, 1));
        assert!((last(&r, 0) - w).abs() < 1e-4 * w, "compound speed {} vs {w}", last(&r, 0));
    }

    /// The wound-field synchronous motor on a stiff grid takes a load step and settles back to
    /// synchronous speed with its torque equal to the load.
    #[test]
    fn synchronous_motor_holds_synchronism() {
        use std::f64::consts::PI;
        let mut m = Mk::new(8.0, "rk45");
        m.d.m.sim.rtol = 1e-7;
        let pr = preset("SyncField", 0);
        let g = |k: &str| pr.iter().find(|(a, _)| a == k).unwrap().1.parse::<f64>().unwrap();
        let ws = 2.0 * PI * 50.0 / 2.0;
        let (w0, th0, vf) = (format!("{ws}"), format!("{}", -PI / 4.0), format!("{}", g("Rf") * g("if0")));
        let mot = add_owned(&mut m, "SyncField", &pr, &[("w0", &w0), ("th0", &th0)]);
        let src = m.add("ThreePhase", &[("Vll", "3300"), ("f", "50")]);
        let f = m.add("Constant", &[("value", &vf)]);
        let tl = m.add("Step", &[("time", "1"), ("final", "3000")]);
        let sc = m.add("Scope", &[("n", "2")]);
        m.wire(src, 0, mot, 0);
        m.wire(f, 0, mot, 1);
        m.wire(tl, 0, mot, 2);
        m.wire(mot, 0, sc, 0);
        m.wire(mot, 1, sc, 1);
        let r = m.run();
        // Before the step: exact no-load equilibrium.
        assert!((at(&r, 0, 0.9) - ws).abs() < 1e-6 * ws, "no-load speed {}", at(&r, 0, 0.9));
        assert!(at(&r, 1, 0.9).abs() < 1.0, "no-load torque {}", at(&r, 1, 0.9));
        assert!((last(&r, 0) - ws).abs() < 1e-4 * ws, "speed after the step {}", last(&r, 0));
        assert!((last(&r, 1) - 3000.0).abs() < 0.01 * 3000.0, "torque after the step {}", last(&r, 1));
    }

    /// The three-phase PMSM, fed through the inverse transforms at its own angle, matches the
    /// d–q PMSM block exactly.
    #[test]
    fn pmsm_three_phase_matches_dq() {
        let mut m = Mk::new(0.3, "rk45");
        let pr: [(&str, &str); 7] = [("Rs", "0.15"), ("Ld", "0.0025"), ("Lq", "0.004"), ("psi", "0.4"), ("p", "4"), ("J", "0.03"), ("b", "0.001")];
        let m3 = m.add("PMSM3", &pr);
        let dq = m.add("PMSM", &pr);
        let vdq = m.add("Constant", &[("value", "[-20 150]")]);
        let vd = m.add("Constant", &[("value", "-20")]);
        let vq = m.add("Constant", &[("value", "150")]);
        let tl = m.add("Constant", &[("value", "5")]);
        let gp = m.add("Gain", &[("k", "4")]);
        let ip = m.add("InvPark", &[]);
        let ic = m.add("InvClarke", &[]);
        let sc = m.add("Scope", &[("n", "4")]);
        m.wire(vdq, 0, ip, 0);
        m.wire(m3, 3, gp, 0);
        m.wire(gp, 0, ip, 1);
        m.wire(ip, 0, ic, 0);
        m.wire(ic, 0, m3, 0);
        m.wire(tl, 0, m3, 1);
        m.wire(vd, 0, dq, 0);
        m.wire(vq, 0, dq, 1);
        m.wire(tl, 0, dq, 2);
        m.wire(m3, 0, sc, 0);
        m.wire(dq, 0, sc, 1);
        m.wire(m3, 1, sc, 2);
        m.wire(dq, 3, sc, 3);
        let r = m.run();
        let e = r.y[0].iter().zip(&r.y[1]).map(|(a, b)| (a - b).abs()).fold(0.0, f64::max);
        let et = r.y[2].iter().zip(&r.y[3]).map(|(a, b)| (a - b).abs()).fold(0.0, f64::max);
        assert!(e < 1e-4 && et < 1e-3, "speed difference {e}, torque difference {et}");
        assert!(last(&r, 0) > 50.0, "the motor runs up ({})", last(&r, 0));
    }

    /// BLDC with no load or friction runs to V/(2·Ke).
    #[test]
    fn bldc_no_load_speed() {
        let mut m = Mk::new(0.2, "rk45");
        let mot = m.add("BLDC", &[("b", "0")]);
        let v = m.add("Constant", &[("value", "24")]);
        let tl = m.add("Constant", &[("value", "0")]);
        let sc = m.add("Scope", &[]);
        m.wire(v, 0, mot, 0);
        m.wire(tl, 0, mot, 1);
        m.wire(mot, 0, sc, 0);
        let r = m.run();
        assert!((last(&r, 0) - 24.0 / 0.09).abs() < 1e-4, "{}", last(&r, 0));
    }

    /// A hybrid stepper moves one full step, π/(2·Nr), when the current moves from phase A to B.
    #[test]
    fn stepper_full_step() {
        let mut m = Mk::new(0.5, "rk45");
        let mot = m.add("Stepper", &[]);
        let va = m.add("Step", &[("time", "0.05"), ("initial", "1.65"), ("final", "0")]);
        let vb = m.add("Step", &[("time", "0.05"), ("initial", "0"), ("final", "1.65")]);
        let tl = m.add("Constant", &[("value", "0")]);
        let sc = m.add("Scope", &[]);
        m.wire(va, 0, mot, 0);
        m.wire(vb, 0, mot, 1);
        m.wire(tl, 0, mot, 2);
        m.wire(mot, 2, sc, 0);
        let r = m.run();
        assert!(at(&r, 0, 0.05).abs() < 1e-9);
        let step = std::f64::consts::PI / 100.0;
        assert!((last(&r, 0) - step).abs() < 1e-5, "angle {} vs {step}", last(&r, 0));
    }

    /// Inverter: the average model gives Vdc/2·(m − mean m); the switched model has the same mean
    /// over whole carrier periods. (Here crossings land exactly on zero, which once lost a switching
    /// instant: the solver now keeps a zero-crossing function's last non-zero sign.)
    #[test]
    fn inverter_average_and_switched() {
        for model in ["average", "switched (PWM)"] {
            let mut m = Mk::new(0.1, "rk45");
            let inv = m.add("Inverter", &[("model", model), ("fc", "1000")]);
            let mi = m.add("Constant", &[("value", "[0.5 -0.2 0.1]")]);
            let vdc = m.add("Constant", &[("value", "600")]);
            let int = m.add("Integrator", &[]);
            let sc = m.add("Scope", &[]);
            m.wire(mi, 0, inv, 0);
            m.wire(vdc, 0, inv, 1);
            m.wire(inv, 0, int, 0);
            m.wire(int, 0, sc, 0);
            let r = m.run();
            for (k, want) in [110.0, -100.0, -10.0].iter().enumerate() {
                let got = last(&r, k) / 0.1;
                assert!((got - want).abs() < 1e-3 * 600.0, "{model} phase {k}: mean {got} vs {want}");
            }
            if model != "average" {
                assert!(r.stats.events >= 600, "switching instants located ({} events)", r.stats.events);
            }
        }
    }

    // ------------------------------------------------------------ discrete control (0.1.4)

    /// Values at the sample instants k·Ts.
    fn samples(r: &Results, ch: usize, ts: f64, n: usize) -> Vec<f64> {
        (0..n).map(|k| {
            let t = k as f64 * ts;
            let i = r.t.iter().position(|x| (x - t).abs() < 1e-9).unwrap_or_else(|| panic!("no sample at t = {t}"));
            r.y[ch][i]
        }).collect()
    }

    /// 0.5/(z − 0.5) to a unit step: y[k] = 1 − 0.5^k, held between samples; the state-space form
    /// of the same system gives the same.
    #[test]
    fn discrete_tf_and_ss_step() {
        let mut m = Mk::new(2.0, "rk45");
        let s = m.add("Step", &[("time", "0")]);
        let tf = m.add("DiscreteTF", &[("num", "[0.5]"), ("den", "[1 -0.5]"), ("ts", "0.1")]);
        let ss = m.add("DiscreteSS", &[("A", "[0.5]"), ("B", "[0.5]"), ("C", "[1]"), ("D", "0"), ("ts", "0.1")]);
        let sc = m.add("Scope", &[("n", "2")]);
        m.wire(s, 0, tf, 0);
        m.wire(s, 0, ss, 0);
        m.wire(tf, 0, sc, 0);
        m.wire(ss, 0, sc, 1);
        let r = m.run();
        for (k, (a, b)) in samples(&r, 0, 0.1, 20).iter().zip(samples(&r, 1, 0.1, 20)).enumerate() {
            let want = 1.0 - 0.5f64.powi(k as i32);
            assert!((a - want).abs() < 1e-12 && (b - want).abs() < 1e-12, "k = {k}: {a}, {b}, want {want}");
        }
        assert!((at(&r, 0, 0.35) - (1.0 - 0.5f64.powi(3))).abs() < 1e-12, "held between samples");
    }

    /// The discrete integrator's three methods on a ramp u = t (K = 1, Ts = 0.1): forward Euler
    /// Ts²·k(k−1)/2, backward Euler Ts²·k(k+1)/2, trapezoidal the exact (k·Ts)²/2.
    #[test]
    fn discrete_integrator_methods() {
        let mut m = Mk::new(2.0, "rk45");
        let c = m.add("Clock", &[]);
        let ids: Vec<u32> = ["forward Euler", "backward Euler", "trapezoidal"].iter().map(|meth| m.add("DiscreteIntegrator", &[("method", meth), ("ts", "0.1")])).collect();
        let sc = m.add("Scope", &[("n", "3")]);
        for (k, id) in ids.iter().enumerate() {
            m.wire(c, 0, *id, 0);
            m.wire(*id, 0, sc, k);
        }
        let r = m.run();
        let ts: f64 = 0.1;
        for k in 0..20 {
            let kf = k as f64;
            let want = [ts * ts * kf * (kf - 1.0) / 2.0, ts * ts * kf * (kf + 1.0) / 2.0, (kf * ts).powi(2) / 2.0];
            for (ch, w) in want.iter().enumerate() {
                let got = samples(&r, ch, ts, 20)[k];
                assert!((got - w).abs() < 1e-12, "method {ch}, k = {k}: {got} vs {w}");
            }
        }
    }

    /// A discrete PI loop around b/(z − a): the run equals the difference equations, and the
    /// integral action removes the error.
    #[test]
    fn discrete_pi_loop_difference_equations() {
        let (a, b, kp, ki, ts) = (0.8, 0.2, 1.5, 4.0, 0.1);
        let mut m = Mk::new(5.0, "rk45");
        let r0 = m.add("Step", &[("time", "0")]);
        let e = m.add("Sum", &[("signs", "+-")]);
        let pid = m.add("DiscretePID", &[("P", "1.5"), ("I", "4"), ("ts", "0.1")]);
        let pl = m.add("DiscreteTF", &[("num", "[0.2]"), ("den", "[1 -0.8]"), ("ts", "0.1")]);
        let sc = m.add("Scope", &[]);
        m.wire(r0, 0, e, 0);
        m.wire(e, 0, pid, 0);
        m.wire(pid, 0, pl, 0);
        m.wire(pl, 0, e, 1);
        m.wire(pl, 0, sc, 0);
        let r = m.run();
        let (mut y, mut xi) = (0.0f64, 0.0f64);
        let got = samples(&r, 0, ts, 50);
        for (k, g) in got.iter().enumerate() {
            assert!((g - y).abs() < 1e-12, "k = {k}: {g} vs {y}");
            let err = 1.0 - y;
            let u = kp * err + xi;
            xi += ki * ts * err;
            y = a * y + b * u;
        }
        assert!((last(&r, 0) - 1.0).abs() < 1e-5, "no steady-state error ({})", last(&r, 0));
    }

    /// Discrete PID derivative on a ramp settles to D; clamping keeps the integral from winding up.
    #[test]
    fn discrete_pid_derivative_and_windup() {
        let mut m = Mk::new(1.0, "rk45");
        let c = m.add("Clock", &[]);
        let d = m.add("DiscretePID", &[("P", "0"), ("D", "2"), ("N", "50"), ("ts", "0.01")]);
        let one = m.add("Constant", &[("value", "1")]);
        let cl = m.add("DiscretePID", &[("P", "0"), ("I", "10"), ("ts", "0.01"), ("upper", "1"), ("lower", "-1")]);
        let nb = m.add("DiscretePID", &[("P", "0"), ("I", "10"), ("ts", "0.01"), ("upper", "1"), ("lower", "-1"), ("aw", "none")]);
        let sc = m.add("Scope", &[("n", "3")]);
        m.wire(c, 0, d, 0);
        m.wire(one, 0, cl, 0);
        m.wire(one, 0, nb, 0);
        m.wire(d, 0, sc, 0);
        m.wire(cl, 0, sc, 1);
        m.wire(nb, 0, sc, 2);
        let r = m.run();
        assert!((last(&r, 0) - 2.0).abs() < 1e-9, "derivative of a ramp → D ({})", last(&r, 0));
        assert!(last(&r, 1) == 1.0 && last(&r, 2) == 1.0, "both limited");
        // Inspect the integral: a negative error now brings the clamped one off the limit at once.
        let mut m2 = Mk::new(1.2, "rk45");
        let sq = m2.add("Step", &[("time", "1"), ("initial", "1"), ("final", "-1")]);
        let cl = m2.add("DiscretePID", &[("P", "0"), ("I", "10"), ("ts", "0.01"), ("upper", "1"), ("lower", "-1")]);
        let nb = m2.add("DiscretePID", &[("P", "0"), ("I", "10"), ("ts", "0.01"), ("upper", "1"), ("lower", "-1"), ("aw", "none")]);
        let sc = m2.add("Scope", &[("n", "2")]);
        m2.wire(sq, 0, cl, 0);
        m2.wire(sq, 0, nb, 0);
        m2.wire(cl, 0, sc, 0);
        m2.wire(nb, 0, sc, 1);
        let r = m2.run();
        assert!(at(&r, 0, 1.05) < 0.7, "clamped integral unwinds at once ({})", at(&r, 0, 1.05));
        assert!(at(&r, 1, 1.05) == 1.0, "unclamped integral is still wound up ({})", at(&r, 1, 1.05));
    }

    /// Rate transition: hold samples, delay lags one output sample.
    #[test]
    fn rate_transition() {
        let mut m = Mk::new(1.0, "rk45");
        let c = m.add("Clock", &[]);
        let h = m.add("RateTransition", &[("ts", "0.25")]);
        let d = m.add("RateTransition", &[("ts", "0.25"), ("mode", "delay one sample")]);
        let sc = m.add("Scope", &[("n", "2")]);
        m.wire(c, 0, h, 0);
        m.wire(c, 0, d, 0);
        m.wire(h, 0, sc, 0);
        m.wire(d, 0, sc, 1);
        let r = m.run();
        assert!((at(&r, 0, 0.6) - 0.5).abs() < 1e-12 && (at(&r, 1, 0.6) - 0.25).abs() < 1e-12);
    }

    /// Integrator reset on a rising edge: the state returns to its initial value at 0.5 and 1.5 s.
    /// (The reset signal rests at exactly zero between pulses; the solver once took endless tiny
    /// steps there. Crossings now go strictly from one side to the other.)
    #[test]
    fn integrator_reset() {
        let mut m = Mk::new(2.0, "rk45");
        let one = m.add("Constant", &[("value", "1")]);
        let p = m.add("Pulse", &[("period", "1"), ("delay", "0.5")]);
        let i = m.add("Integrator", &[("reset", "rising")]);
        let di = m.add("DiscreteIntegrator", &[("reset", "rising"), ("ts", "0.1")]);
        let sc = m.add("Scope", &[("n", "2")]);
        m.wire(one, 0, i, 0);
        m.wire(p, 0, i, 1);
        m.wire(one, 0, di, 0);
        m.wire(p, 0, di, 1);
        m.wire(i, 0, sc, 0);
        m.wire(di, 0, sc, 1);
        let r = m.run();
        for (t, want) in [(0.4, 0.4), (1.0, 0.5), (1.4, 0.9), (1.6, 0.1), (2.0, 0.5)] {
            assert!((at(&r, 0, t) - want).abs() < 1e-9, "continuous at {t}: {}", at(&r, 0, t));
        }
        for (t, want) in [(0.45, 0.4), (1.05, 0.5), (1.65, 0.1)] {
            assert!((at(&r, 1, t) - want).abs() < 1e-9, "discrete at {t}: {}", at(&r, 1, t));
        }
    }

    /// Goto/From: local in the same diagram, global into a subsystem, and clear errors.
    #[test]
    fn goto_from() {
        let mut m = Mk::new(1.0, "rk45");
        let c = m.add("Constant", &[("value", "3")]);
        let g = m.add("Goto", &[("tag", "speed")]);
        let f = m.add("From", &[("tag", "speed")]);
        let sc = m.add("Scope", &[]);
        m.wire(c, 0, g, 0);
        m.wire(f, 0, sc, 0);
        assert_eq!(last(&m.run(), 0), 3.0);
        // A From inside a subsystem sees only a global Goto.
        let mut m = Mk::new(1.0, "rk45");
        let c = m.add("Constant", &[("value", "3")]);
        let g = m.add("Goto", &[("tag", "speed")]);
        let f = m.add("From", &[("tag", "speed")]);
        m.wire(c, 0, g, 0);
        m.d.apply(Op::Group { ids: vec![f] }).unwrap();
        let e = compile(&m.d.m, None, &none()).err().expect("a local Goto is not seen inside a subsystem");
        assert!(e.iter().any(|i| i.msg.contains("no Goto sends the tag 'speed'")), "{e:?}");
        m.d.apply(Op::SetParam { id: g, key: "scope".into(), value: "global".into() }).unwrap();
        assert!(compile(&m.d.m, None, &none()).is_ok());
        // Two Gotos with one tag.
        let g2 = m.add("Goto", &[("tag", "speed")]);
        m.wire(c, 0, g2, 0);
        let e = compile(&m.d.m, None, &none()).err().unwrap();
        assert!(e.iter().any(|i| i.msg.contains("another Goto already sends the tag 'speed'")), "{e:?}");
    }

    /// Square, triangle, chirp and repeating sequence against their definitions.
    #[test]
    fn periodic_sources() {
        let mut m = Mk::new(3.0, "rk45");
        let sq = m.add("Square", &[("amp", "2"), ("freq", "1"), ("duty", "25"), ("bias", "1")]);
        let tr = m.add("Triangle", &[("amp", "2"), ("freq", "0.5")]);
        let ch = m.add("Chirp", &[("f0", "0.1"), ("f1", "2"), ("T", "2")]);
        let rs = m.add("RepSeq", &[("times", "[0 1 1.5]"), ("values", "[0 2 0]")]);
        let sc = m.add("Scope", &[("n", "4")]);
        for (k, b) in [sq, tr, ch, rs].iter().enumerate() {
            m.wire(*b, 0, sc, k);
        }
        let r = m.run();
        for (t, want) in [(0.1, 3.0), (0.3, -1.0), (1.2, 3.0), (1.9, -1.0)] {
            assert!((at(&r, 0, t) - want).abs() < 1e-12, "square at {t}");
        }
        assert!(r.t.iter().any(|t| (t - 0.25).abs() < 1e-12) && r.t.iter().any(|t| (t - 1.25).abs() < 1e-12), "square edges hit");
        for (t, want) in [(0.5, 2.0), (1.0, 0.0), (1.5, -2.0), (2.25, 1.0)] {
            assert!((at(&r, 1, t) - want).abs() < 1e-9, "triangle at {t}: {}", at(&r, 1, t));
        }
        let chirp = |t: f64| if t <= 2.0 { (2.0 * std::f64::consts::PI * (0.1 * t + 0.25 * 1.9 * t * t)).sin() } else { (2.0 * std::f64::consts::PI * (0.2 + 1.9 + 2.0 * (t - 2.0))).sin() };
        assert!(max_err(&r, 2, chirp) < 1e-12);
        for (t, want) in [(0.5, 1.0), (1.25, 1.0), (2.0, 1.0), (2.75, 1.0), (3.0, 0.0)] {
            assert!((at(&r, 3, t) - want).abs() < 1e-9, "sequence at {t}: {}", at(&r, 3, t));
        }
    }

    /// Sample-time rates: constant, discrete, continuous, inherited and mixed.
    #[test]
    fn sample_time_rates() {
        use crate::rt::Rate;
        let mut m = Mk::new(1.0, "rk45");
        let k = m.add("Constant", &[]);
        let gk = m.add("Gain", &[]);
        let dtf = m.add("DiscreteTF", &[("ts", "0.1")]);
        let gd = m.add("Gain", &[]);
        let i = m.add("Integrator", &[]);
        let sum = m.add("Sum", &[("signs", "++")]);
        let z2 = m.add("ZOH", &[("ts", "0.02")]);
        m.wire(k, 0, gk, 0);
        m.wire(gk, 0, dtf, 0);
        m.wire(dtf, 0, gd, 0);
        m.wire(gk, 0, i, 0);
        m.wire(gd, 0, sum, 0);
        m.wire(i, 0, sum, 1);
        m.wire(i, 0, z2, 0);
        let sim = compile(&m.d.m, None, &none()).unwrap();
        let rate = |id: u32| sim.rates.iter().find(|(b, _)| *b == id).unwrap().1;
        assert_eq!(rate(k), Rate::Constant);
        assert_eq!(rate(gk), Rate::Constant);
        assert_eq!(rate(dtf), Rate::Discrete(0.1));
        assert_eq!(rate(gd), Rate::Discrete(0.1));
        assert_eq!(rate(i), Rate::Continuous);
        assert_eq!(rate(sum), Rate::Mixed);
        assert_eq!(rate(z2), Rate::Discrete(0.02));
        // A subsystem shows the combined rate of its contents.
        let sub = m.d.apply(Op::Group { ids: vec![dtf, gd] }).unwrap()[0];
        let sim = compile(&m.d.m, None, &none()).unwrap();
        let v = crate::compile::rate_view(&m.d.m, &sim);
        let s = v.iter().find(|r| r.id == sub).expect("the subsystem has a rate");
        assert_eq!((s.rate, s.ts), ("discrete", 0.1));
        assert!(v.iter().any(|r| r.id == dtf && r.rate == "discrete"), "blocks inside are listed too");
    }

    /// Log all signals: every block output (and each subsystem output, by the subsystem) is logged
    /// for the probe, and the scope channels are unchanged.
    #[test]
    fn log_all_signals_for_the_probe() {
        let mut m = Mk::new(1.0, "rk45");
        let s = m.add("Step", &[("time", "0")]);
        let g = m.add("Gain", &[("k", "3")]);
        let i = m.add("Integrator", &[]);
        let sc = m.add("Scope", &[]);
        m.wire(s, 0, g, 0);
        m.wire(g, 0, i, 0);
        m.wire(i, 0, sc, 0);
        let before = m.run();
        assert!(before.chans.iter().all(|c| !c.probe));
        let sub = m.d.apply(Op::Group { ids: vec![g, i] }).unwrap()[0];
        let mut sim = m.d.m.sim.clone();
        sim.log_all = true;
        m.d.apply(Op::SetSim { sim }).unwrap();
        let r = m.run();
        let scope: Vec<_> = r.chans.iter().filter(|c| !c.probe).collect();
        assert_eq!(scope.len(), 1, "the scope still logs one signal");
        let ch = |id: u32| r.chans.iter().position(|c| c.probe && c.sink == id && c.port == 0).unwrap_or_else(|| panic!("no probe for block {id}"));
        assert!((last(&r, ch(s)) - 1.0).abs() < 1e-12);
        assert!((last(&r, ch(g)) - 3.0).abs() < 1e-12, "a block inside the subsystem");
        assert!((last(&r, ch(sub)) - 3.0).abs() < 1e-9, "the subsystem's output, by the subsystem block");
        assert!((crate::measure::value_at(&r.t, &r.y[ch(sub)], 0.5) - 1.5).abs() < 1e-9);
    }

    /// Diagram tools (0.1.6): a wire's name names its signal (and every wire from the same output);
    /// notes take no part in a run; colours are checked; Find looks inside subsystems; the file
    /// keeps names and colours.
    #[test]
    fn wire_names_notes_colours_find() {
        let mut m = Mk::new(1.0, "rk45");
        let s = m.add("Step", &[("time", "0")]);
        let g = m.add("Gain", &[("k", "2")]);
        let sc = m.add("Scope", &[("n", "2")]);
        m.wire(s, 0, g, 0);
        m.wire(s, 0, sc, 1);
        m.wire(g, 0, sc, 0);
        let w = m.d.m.root.wires.iter().find(|w| w.from == (s, 0) && w.to == (g, 0)).unwrap().id;
        m.d.apply(Op::NameWire { id: w, name: "demand".into() }).unwrap();
        assert!(m.d.m.root.wires.iter().filter(|w| w.from == (s, 0)).all(|w| w.name == "demand"), "both wires from the step carry the name");
        let note = m.add("Note", &[("text", "Explains the loop")]);
        let r = m.run();
        assert_eq!(r.chans[1].name, "demand", "the legend uses the signal's name");
        assert!(compile(&m.d.m, None, &none()).unwrap().nodes.iter().all(|n| n.id != note), "a note is not compiled");
        assert!(m.d.apply(Op::NameWire { id: w, name: "a/b".into() }).is_err());
        m.d.apply(Op::SetColor { ids: vec![g], color: "blue".into() }).unwrap();
        assert!(m.d.apply(Op::SetColor { ids: vec![g], color: "teal-ish".into() }).is_err());
        let back = crate::model::from_json(&crate::model::to_json(&m.d.m)).unwrap();
        assert_eq!(back.root.block(g).unwrap().color, "blue");
        assert!(back.root.wires.iter().any(|w| w.name == "demand"));
        // Find: by name, by kind's library name, by signal name, inside a subsystem.
        let sub = m.d.apply(Op::Group { ids: vec![g] }).unwrap()[0];
        let hits = crate::view::find(&m.d.m, "gain");
        assert!(hits.iter().any(|h| h.id == g && h.parent == sub && !h.path.is_empty()), "{hits:?}");
        assert!(crate::view::find(&m.d.m, "DEMAND").iter().any(|h| h.id == s), "a named signal finds its source");
        assert!(crate::view::find(&m.d.m, "  ").is_empty());
    }

    fn mask(params: &[(&str, &str)]) -> crate::model::Mask {
        crate::model::Mask {
            params: params.iter().map(|(k, d)| crate::model::MaskParam { key: (*k).into(), label: format!("Value {k}"), def: (*d).into(), ..Default::default() }).collect(),
            icon: "K·u".into(),
            help: "Multiplies by K.".into(),
            presets: vec![crate::model::MaskPreset { name: "Double".into(), values: vec![("K".into(), "2".into())] }],
        }
    }

    /// Masks (0.2.0): the blocks inside use the mask's parameters by name; an inner mask hides an
    /// outer one and the model's variables; values are set like any block's; errors point at the
    /// subsystem; bad masks are refused; the file keeps the mask; a format 1 file still opens.
    #[test]
    fn masks_scope_their_parameters() {
        let mut m = Mk::new(1.0, "rk45");
        m.d.m.vars.push(crate::model::Var { name: "K".into(), expr: "100".into(), ..Default::default() });
        let one = m.add("Constant", &[("value", "1")]);
        let g = m.add("Gain", &[("k", "K")]);
        let sc = m.add("Scope", &[]);
        m.wire(one, 0, g, 0);
        m.wire(g, 0, sc, 0);
        assert_eq!(last(&m.run(), 0), 100.0, "the model's K");
        let sub = m.d.apply(Op::Group { ids: vec![g] }).unwrap()[0];
        m.d.apply(Op::SetMask { id: sub, mask: Some(mask(&[("K", "3")])) }).unwrap();
        assert_eq!(last(&m.run(), 0), 3.0, "the mask's default hides the model's K");
        m.d.apply(Op::SetParam { id: sub, key: "K".into(), value: "4*2".into() }).unwrap();
        assert_eq!(last(&m.run(), 0), 8.0, "a value set on the masked block");
        // A mask parameter can use the model's variables.
        m.d.apply(Op::SetParam { id: sub, key: "K".into(), value: "K/50".into() }).unwrap();
        assert_eq!(last(&m.run(), 0), 2.0, "K/50 with the outer K = 100");
        // Nested: grouping makes a new outer subsystem; its mask's K hides the model's K for
        // everything inside, so the inner mask's K/50 is now 7/50.
        let outer = m.d.apply(Op::Group { ids: vec![sub] }).unwrap()[0];
        m.d.apply(Op::SetMask { id: outer, mask: Some(mask(&[("K", "7"), ("G", "2")])) }).unwrap();
        assert!((last(&m.run(), 0) - 0.14).abs() < 1e-12, "K/50 in the outer mask's scope");
        // An error in a mask value points at the subsystem.
        m.d.apply(Op::SetParam { id: sub, key: "K".into(), value: "Q+1".into() }).unwrap();
        let e = compile(&m.d.m, None, &none()).err().unwrap();
        assert!(e.iter().any(|i| i.id == sub && i.msg.contains("unknown name 'Q'")), "{e:?}");
        // Refused: unknown parameters, bad names, duplicates, presets of unknown keys, non-subsystems.
        assert!(m.d.apply(Op::SetParam { id: sub, key: "nope".into(), value: "1".into() }).is_err());
        for bad in [mask(&[("2x", "1")]), mask(&[("t", "1")]), mask(&[("u3", "1")]), mask(&[("A", "1"), ("A", "2")]), mask(&[("A", "1")])] {
            assert!(m.d.apply(Op::SetMask { id: sub, mask: Some(bad.clone()) }).is_err(), "{bad:?}");
        }
        assert!(m.d.apply(Op::SetMask { id: one, mask: Some(mask(&[("K", "1")])) }).is_err());
        // The file keeps the mask and its values; the view shows the mask's parameters.
        m.d.apply(Op::SetParam { id: sub, key: "K".into(), value: "5".into() }).unwrap();
        let back = crate::model::from_json(&crate::model::to_json(&m.d.m)).unwrap();
        assert_eq!(back.block(sub).unwrap().mask.as_ref().unwrap().params[0].key, "K");
        assert_eq!(back.block(sub).unwrap().params.get("K").map(|s| s.as_str()), Some("5"));
        let v = crate::view::view(&m.d.m, outer).unwrap();
        let bv = v.blocks.iter().find(|b| b.id == sub).unwrap();
        assert_eq!((bv.params.get("K").map(|s| s.as_str()), bv.label.as_str()), (Some("5"), "K·u"));
        // Removing the mask drops its values; the inside then sees the model's K again.
        m.d.apply(Op::SetMask { id: outer, mask: None }).unwrap();
        m.d.apply(Op::SetMask { id: sub, mask: None }).unwrap();
        assert!(m.d.m.block(sub).unwrap().params.is_empty());
        assert_eq!(last(&m.run(), 0), 100.0);
        // A format 1 file opens (and is saved as format 2).
        let mut old: serde_json::Value = serde_json::from_str(&crate::model::to_json(&m.d.m)).unwrap();
        old["format"] = serde_json::json!(1);
        let o = crate::model::from_json(&old.to_string()).unwrap();
        assert_eq!(o.format, crate::model::FORMAT);
        assert_eq!(crate::model::FORMAT, 2);
    }

    /// My blocks: a masked subsystem saved as a library clip pastes back as an independent copy
    /// (copied, not linked) with its mask, at the place asked for; plain blocks are refused.
    #[test]
    fn my_blocks_copy_a_subsystem() {
        let mut m = Mk::new(1.0, "rk45");
        let one = m.add("Constant", &[("value", "1")]);
        let g = m.add("Gain", &[("k", "K")]);
        let sc = m.add("Scope", &[("n", "2")]);
        m.wire(one, 0, g, 0);
        m.wire(g, 0, sc, 0);
        let sub = m.d.apply(Op::Group { ids: vec![g] }).unwrap()[0];
        m.d.apply(Op::SetMask { id: sub, mask: Some(mask(&[("K", "3")])) }).unwrap();
        assert!(m.d.block_clip(one).is_err(), "only subsystems");
        let clip = m.d.block_clip(sub).unwrap();
        let made = m.d.apply(Op::Paste { parent: 0, clip, dx: 400.0, dy: 120.0 }).unwrap();
        let copy = *made.iter().find(|id| m.d.m.block(**id).is_some_and(|b| b.kind == "Subsystem")).unwrap();
        let b = m.d.m.block(copy).unwrap().clone();
        assert_eq!((b.x, b.y), (400.0, 120.0));
        assert!(b.mask.is_some() && copy != sub, "a copy with its mask and its own id");
        m.d.apply(Op::SetParam { id: copy, key: "K".into(), value: "5".into() }).unwrap();
        m.wire(one, 0, copy, 0);
        m.wire(copy, 0, sc, 1);
        let r = m.run();
        assert_eq!((last(&r, 0), last(&r, 1)), (3.0, 5.0), "each copy keeps its own values");
    }

    // ------------------------------------------------------------ motor extras and power balance

    /// Steady state at a held speed: electrical power in − stator copper loss = air-gap power.
    /// Induction: P_ag = T_e·ω_sync; synchronous (PMSM3, SynRM): T_e·ω.
    #[test]
    fn power_balance_ac_machines() {
        use std::f64::consts::PI;
        let wsync = 2.0 * PI * 50.0 / 2.0;
        for kind in ["InductionSC", "InductionDC", "InductionWR"] {
            let mut m = Mk::new(3.0, "rk45");
            m.d.m.sim.rtol = 1e-9;
            let pr = preset(kind, 0);
            let w0 = format!("{}", 0.97 * wsync);
            let mot = add_owned(&mut m, kind, &pr, &[("J", "1e12"), ("w0", &w0)]);
            let src = m.add("ThreePhase", &[("Vll", "400"), ("f", "50")]);
            let tl = m.add("Constant", &[("value", "0")]);
            let sc = m.add("Scope", &[("n", "3")]);
            m.wire(src, 0, mot, 0);
            if kind == "InductionWR" {
                let z = m.add("Constant", &[("value", "[0 0 0]")]);
                m.wire(z, 0, mot, 1);
                m.wire(tl, 0, mot, 2);
            } else {
                m.wire(tl, 0, mot, 1);
            }
            m.wire(mot, 1, sc, 0);
            m.wire(mot, 2, sc, 1);
            m.wire(src, 0, sc, 2);
            let r = m.run();
            let rs: f64 = pr.iter().find(|(k, _)| k == "Rs").unwrap().1.parse().unwrap();
            let pin: f64 = (0..3).map(|k| last(&r, 4 + k) * last(&r, 1 + k)).sum();
            let loss: f64 = (0..3).map(|k| rs * last(&r, 1 + k).powi(2)).sum();
            let pag = last(&r, 0) * wsync;
            assert!((pin - loss - pag).abs() < 1e-3 * pin.abs(), "{kind}: P_in − loss = {}, T_e·ω_s = {pag}", pin - loss);
        }
        for kind in ["PMSM3", "SynRM"] {
            // Long enough for the slowest electrical time constant (SynRM Ld/Rs = 0.15 s).
            let mut m = Mk::new(4.0, "rk45");
            m.d.m.sim.rtol = 1e-9;
            let mot = m.add(kind, &[("J", "1e12"), ("w0", "100")]);
            let vdq = m.add("Constant", &[("value", "[-40 150]")]);
            // The electrical angle: pole pairs × θ (4 for the PMSM3, 2 for the SynRM defaults).
            let gp = m.add("Gain", &[("k", if kind == "PMSM3" { "4" } else { "2" })]);
            let ip = m.add("InvPark", &[]);
            let ic = m.add("InvClarke", &[]);
            let tl = m.add("Constant", &[("value", "0")]);
            let sc = m.add("Scope", &[("n", "4")]);
            m.wire(vdq, 0, ip, 0);
            m.wire(mot, 3, gp, 0);
            m.wire(gp, 0, ip, 1);
            m.wire(ip, 0, ic, 0);
            m.wire(ic, 0, mot, 0);
            m.wire(tl, 0, mot, 1);
            m.wire(mot, 1, sc, 0);
            m.wire(mot, 2, sc, 1);
            m.wire(ic, 0, sc, 2);
            m.wire(mot, 0, sc, 3);
            let r = m.run();
            let rs = if kind == "PMSM3" { 0.15 } else { 0.8 };
            let pin: f64 = (0..3).map(|k| last(&r, 4 + k) * last(&r, 1 + k)).sum();
            let loss: f64 = (0..3).map(|k| rs * last(&r, 1 + k).powi(2)).sum();
            let pm = last(&r, 0) * last(&r, 7);
            assert!((pin - loss - pm).abs() < 1e-4 * pin.abs().max(1.0), "{kind}: P_in − loss = {}, T_e·ω = {pm}", pin - loss);
        }
    }

    /// DC machines and BLDC at steady state: V·i = R·i² + T_e·ω.
    #[test]
    fn power_balance_dc_machines() {
        let mut m = Mk::new(4.0, "rk45");
        let mot = m.add("DCSepEx", &[]);
        let va = m.add("Constant", &[("value", "220")]);
        let tl = m.add("Constant", &[("value", "20")]);
        let sc = m.add("Scope", &[("n", "3")]);
        m.wire(va, 0, mot, 0);
        m.wire(va, 0, mot, 1);
        m.wire(tl, 0, mot, 2);
        m.wire(mot, 0, sc, 0);
        m.wire(mot, 1, sc, 1);
        m.wire(mot, 3, sc, 2);
        let r = m.run();
        let (w, ia, te) = (last(&r, 0), last(&r, 1), last(&r, 2));
        assert!((220.0 * ia - 0.6 * ia * ia - te * w).abs() < 1e-6 * 220.0 * ia, "sep. excited");
        let mut m = Mk::new(0.3, "rk45");
        let mot = m.add("BLDC", &[]);
        let v = m.add("Constant", &[("value", "24")]);
        let tl = m.add("Constant", &[("value", "0.5")]);
        let sc = m.add("Scope", &[("n", "3")]);
        m.wire(v, 0, mot, 0);
        m.wire(tl, 0, mot, 1);
        m.wire(mot, 0, sc, 0);
        m.wire(mot, 1, sc, 1);
        m.wire(mot, 2, sc, 2);
        let r = m.run();
        let (w, i, te) = (last(&r, 0), last(&r, 1), last(&r, 2));
        assert!((24.0 * i - 2.0 * 0.05 * i * i - te * w).abs() < 1e-6 * 24.0 * i, "BLDC");
    }

    /// Switched reluctance motor: static torque ½·i²·dL/dθ at a held angle, and the energy balance
    /// of a free rotor pulled into alignment (input − copper loss = kinetic + magnetic energy).
    #[test]
    fn srm_torque_and_energy() {
        use std::f64::consts::PI;
        let (r, lmax, lmin, nr) = (1.3, 0.06, 0.008, 4.0);
        let l1 = 0.5 * (lmax - lmin);
        let th0 = PI / 8.0 / 2.0; // a quarter of the way from unaligned to aligned for phase a
        let mut m = Mk::new(0.5, "rk45");
        let th0s = format!("{th0}");
        let mot = m.add("SRM", &[("J", "1e12"), ("th0", &th0s)]);
        let v = m.add("Constant", &[("value", "[13 0 0]")]);
        let tl = m.add("Constant", &[("value", "0")]);
        let sc = m.add("Scope", &[]);
        m.wire(v, 0, mot, 0);
        m.wire(tl, 0, mot, 1);
        m.wire(mot, 1, sc, 0);
        let res = m.run();
        let want = 0.5 * 100.0 * l1 * nr * (nr * th0).sin();
        assert!((last(&res, 0) - want).abs() < 1e-6 * want, "static torque {} vs {want}", last(&res, 0));

        // Free rotor, b = 0: integrate the input power less copper loss with an Integrator.
        let mut m = Mk::new(0.3, "rk45");
        m.d.m.sim.rtol = 1e-10;
        m.d.m.sim.atol = 1e-12;
        let mot = m.add("SRM", &[("b", "0"), ("th0", &th0s)]);
        let v = m.add("Constant", &[("value", "[13 0 0]")]);
        let tl = m.add("Constant", &[("value", "0")]);
        let dm = m.add("Demux", &[("n", "3")]);
        let pw = m.add("Fcn", &[("expr", "13*u - 1.3*u^2")]);
        let e = m.add("Integrator", &[]);
        let sc = m.add("Scope", &[("n", "4")]);
        m.wire(v, 0, mot, 0);
        m.wire(tl, 0, mot, 1);
        m.wire(mot, 2, dm, 0);
        m.wire(dm, 0, pw, 0);
        m.wire(pw, 0, e, 0);
        m.wire(e, 0, sc, 0);
        m.wire(mot, 0, sc, 1);
        m.wire(dm, 0, sc, 2);
        m.wire(mot, 3, sc, 3);
        let res = m.run();
        let (ein, w, i, th) = (last(&res, 0), last(&res, 1), last(&res, 2), last(&res, 3));
        let l = 0.5 * (lmax + lmin) - l1 * (nr * th).cos();
        let stored = 0.5 * 0.008 * w * w + 0.5 * l * i * i;
        assert!((ein - stored).abs() < 1e-5 * ein, "energy in {ein} vs kinetic + magnetic {stored}");
        let _ = r;
    }

    /// DC link: steady voltage drop R_s·i_load; a diode rectifier lets regeneration charge it.
    /// Inverter DC current: V_dc·i_dc equals the power into the motor at every point.
    #[test]
    fn dc_link_and_inverter_current() {
        let mut m = Mk::new(1.0, "rk45");
        let vs = m.add("Constant", &[("value", "600")]);
        let il = m.add("Constant", &[("value", "10")]);
        let regen = m.add("Constant", &[("value", "-10")]);
        let a = m.add("DCLink", &[]);
        let b = m.add("DCLink", &[("source", "diode rectifier (charges only)")]);
        let sc = m.add("Scope", &[("n", "2")]);
        m.wire(vs, 0, a, 0);
        m.wire(il, 0, a, 1);
        m.wire(vs, 0, b, 0);
        m.wire(regen, 0, b, 1);
        m.wire(a, 0, sc, 0);
        m.wire(b, 0, sc, 1);
        let r = m.run();
        assert!((last(&r, 0) - 599.0).abs() < 1e-4, "{}", last(&r, 0));
        assert!((last(&r, 1) - (600.0 + 10.0 / 0.002)).abs() < 1e-4, "regeneration charges the link: {}", last(&r, 1));

        let mut m = Mk::new(0.2, "rk45");
        let mot = m.add("PMSM3", &[]);
        let src = m.add("ThreePhase", &[("Vll", "300"), ("f", "20")]);
        let g = m.add("Gain", &[("k", "1/300")]);
        let vdc = m.add("Constant", &[("value", "600")]);
        let inv = m.add("Inverter", &[]);
        let idc = m.add("InverterDC", &[]);
        let tl = m.add("Constant", &[("value", "0")]);
        let sc = m.add("Scope", &[("n", "3")]);
        m.wire(src, 0, g, 0);
        m.wire(g, 0, inv, 0);
        m.wire(vdc, 0, inv, 1);
        m.wire(inv, 0, mot, 0);
        m.wire(tl, 0, mot, 1);
        m.wire(g, 0, idc, 0);
        m.wire(mot, 2, idc, 1);
        m.wire(idc, 0, sc, 0);
        m.wire(inv, 0, sc, 1);
        m.wire(mot, 2, sc, 2);
        let r = m.run();
        for i in 0..r.t.len() {
            let p: f64 = (0..3).map(|k| r.y[1 + k][i] * r.y[4 + k][i]).sum();
            assert!((600.0 * r.y[0][i] - p).abs() < 1e-9 * p.abs().max(1.0), "t = {}: {} vs {p}", r.t[i], 600.0 * r.y[0][i]);
        }
    }

    /// Sensor: first-order bandwidth, gain and offset; repeatable noise with the set spread.
    #[test]
    fn sensor() {
        let mut m = Mk::new(0.1, "rk45");
        let s = m.add("Step", &[("time", "0")]);
        let a = m.add("Sensor", &[("k", "2"), ("offset", "0.5"), ("bw", "100")]);
        let b = m.add("Sensor", &[("sd", "0.1"), ("ts", "0.0001")]);
        let sc = m.add("Scope", &[("n", "2")]);
        m.wire(s, 0, a, 0);
        m.wire(s, 0, b, 0);
        m.wire(a, 0, sc, 0);
        m.wire(b, 0, sc, 1);
        let r = m.run();
        let wc = 2.0 * std::f64::consts::PI * 100.0;
        assert!(max_err(&r, 0, |t| 0.5 + 2.0 * (1.0 - (-wc * t).exp())) < 1e-6);
        // The logged values at the sample times (between them the noise holds).
        let n: Vec<f64> = samples(&r, 1, 1e-4, 1000).iter().skip(1).map(|v| v - 1.0).collect();
        let mean = n.iter().sum::<f64>() / n.len() as f64;
        let sd = (n.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n.len() as f64).sqrt();
        assert!(mean.abs() < 0.01 && (sd - 0.1).abs() < 0.01, "noise mean {mean}, sd {sd}");
        let r2 = m.run();
        assert_eq!(r.y[1], r2.y[1], "same seed, same noise");
    }

    /// Encoder: the angle is quantised to 2π/(4·PPR), and the speed estimate is within one count
    /// per sample time.
    #[test]
    fn encoder_quantises() {
        let mut m = Mk::new(0.5, "rk45");
        let th = m.add("Ramp", &[("slope", "10")]);
        let enc = m.add("Encoder", &[("ppr", "1024"), ("ts", "0.001")]);
        let sc = m.add("Scope", &[("n", "2")]);
        m.wire(th, 0, enc, 0);
        m.wire(enc, 0, sc, 0);
        m.wire(enc, 1, sc, 1);
        let r = m.run();
        let q = 2.0 * std::f64::consts::PI / 4096.0;
        for (t, y) in r.t.iter().zip(&r.y[0]) {
            assert!(10.0 * t - y >= -1e-9 && 10.0 * t - y < q + 1e-9);
            assert!((y / q - (y / q).round()).abs() < 1e-6);
        }
        assert!((last(&r, 1) - 10.0).abs() <= q / 0.001 + 1e-9, "speed {}", last(&r, 1));
    }
}
