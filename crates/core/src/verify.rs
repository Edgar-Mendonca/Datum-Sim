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
                // Induction motors run just below synchronous speed (157 rad/s) and carry their load.
                "imdol" => assert!((152.0..156.0).contains(&last(0)) && (last(1) - last(2)).abs() < 0.5, "imdol {} {} {}", last(0), last(1), last(2)),
                "imvf" => assert!((150.0..156.0).contains(&last(0)) && (last(1) - 50.0).abs() < 0.5, "imvf {} {}", last(0), last(1)),
                "pmsmfoc" | "bldc" => assert!((last(0) - last(1)).abs() < 0.005 * last(1), "{} speed {} vs {}", ex.id, last(0), last(1)),
                "syncmotor" => assert!((last(0) - 50.0 * std::f64::consts::PI).abs() < 0.01 && (last(1) - 3000.0).abs() < 30.0, "sync {} {}", last(0), last(1)),
                "dcseries" => {
                    // Torque balance Laf·i² = 300 + 0.02·ω² at the end.
                    let (w, i) = (last(0), last(1));
                    assert!((0.0053 * i * i - 300.0 - 0.02 * w * w).abs() < 0.01 * 0.0053 * i * i, "dc series {w} {i}");
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
