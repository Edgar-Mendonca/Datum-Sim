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
        let r = m.add("Step", &[("time", "0")]);
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
        let mut m = Mk::new(1.0, "rk45");
        let s = m.add("Sum", &[("signs", "+-")]);
        let g = m.add("Gain", &[("k", "2")]);
        m.wire(s, 0, g, 0);
        m.wire(g, 0, s, 1);
        let e = compile(&m.d.m, None, &none()).err().unwrap();
        assert!(e.iter().any(|i| i.msg.starts_with("Algebraic loop: ") && i.msg.contains("Gain") && i.msg.contains("Sum")), "{e:?}");
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
}
