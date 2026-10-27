//! The family hardening standard (FAMILY §11): failed edits change nothing; random models and
//! edits never panic and keep their invariants; damaged files open or give a clear error; and a
//! speed gate on a large model.

#[cfg(test)]
mod tests {
    use crate::blocks;
    use crate::compile::{compile, run};
    use crate::doc::{Doc, Op};
    use crate::examples;
    use crate::model::{self, Diagram, Model};
    use std::collections::{BTreeMap, HashSet};

    /// A small deterministic generator (no extra crate).
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }
        fn below(&mut self, n: usize) -> usize {
            (self.next() % n.max(1) as u64) as usize
        }
        fn pick<'a, T>(&mut self, v: &'a [T]) -> Option<&'a T> {
            if v.is_empty() {
                None
            } else {
                Some(&v[self.below(v.len())])
            }
        }
    }

    fn invariants(m: &Model) {
        let mut seen = HashSet::new();
        fn walk(d: &Diagram, seen: &mut HashSet<u32>) {
            let ids: HashSet<u32> = d.blocks.iter().map(|b| b.id).collect();
            let mut inputs = HashSet::new();
            for b in &d.blocks {
                assert!(seen.insert(b.id), "duplicate id {}", b.id);
                if let Some(s) = &b.sub {
                    walk(s, seen);
                }
            }
            for w in &d.wires {
                assert!(seen.insert(w.id), "duplicate wire id {}", w.id);
                assert!(ids.contains(&w.from.0) && ids.contains(&w.to.0), "dangling wire");
                assert!(inputs.insert(w.to), "two wires into one input");
                let (a, b) = (d.block(w.from.0).unwrap(), d.block(w.to.0).unwrap());
                assert!(w.from.1 < blocks::ports(a).1 && w.to.1 < blocks::ports(b).0, "wire to a missing port");
            }
        }
        walk(&m.root, &mut seen);
        assert!(m.next > m.max_id());
    }

    #[test]
    fn failed_edits_change_nothing() {
        let mut d = Doc::new(examples::get("dcmotor").unwrap());
        let before = d.m.clone();
        let some = d.m.root.blocks[0].id;
        let bad = vec![
            Op::Add { parent: 0, kind: "NoSuchBlock".into(), x: 0.0, y: 0.0 },
            Op::Add { parent: 99999, kind: "Gain".into(), x: 0.0, y: 0.0 },
            Op::Move { ids: vec![99999], dx: 1.0, dy: 1.0 },
            Op::Move { ids: vec![some], dx: f64::NAN, dy: 0.0 },
            Op::SetParam { id: some, key: "nope".into(), value: "1".into() },
            Op::Rename { id: some, name: "".into() },
            Op::Rename { id: some, name: "a/b".into() },
            Op::Connect { from: (some, 7), to: (some, 0), pts: vec![] },
            Op::Paste { parent: 0, clip: "{garbage".into(), dx: 0.0, dy: 0.0 },
            Op::Group { ids: vec![99999] },
            Op::Ungroup { id: some },
            Op::SetSim { sim: crate::model::SimSettings { stop: -1.0, ..Default::default() } },
            // A batch that fails half way must undo its first half too.
            Op::Batch(vec![Op::Add { parent: 0, kind: "Gain".into(), x: 0.0, y: 0.0 }, Op::Rename { id: 99999, name: "x".into() }]),
        ];
        for op in bad {
            let desc = format!("{op:?}");
            assert!(d.apply(op).is_err(), "should fail: {desc}");
            assert_eq!(d.m, before, "changed by a failed edit: {desc}");
            assert!(!d.can_undo());
        }
    }

    #[test]
    fn undo_redo_round_trip() {
        let mut d = Doc::new(Model::new());
        let a = d.apply(Op::Add { parent: 0, kind: "Step".into(), x: 0.0, y: 0.0 }).unwrap()[0];
        let b = d.apply(Op::Add { parent: 0, kind: "Scope".into(), x: 100.0, y: 0.0 }).unwrap()[0];
        d.apply(Op::Connect { from: (a, 0), to: (b, 0), pts: vec![] }).unwrap();
        let full = d.m.clone();
        d.apply(Op::Group { ids: vec![a] }).unwrap();
        invariants(&d.m);
        assert!(compile(&d.m, None, &BTreeMap::new()).is_ok());
        assert!(d.undo());
        assert_eq!(d.m, full);
        assert!(d.redo());
        assert_ne!(d.m, full);
        // Ungroup puts the step back with its wire.
        let sid = d.m.root.blocks.iter().find(|b| b.kind == "Subsystem").unwrap().id;
        d.apply(Op::Ungroup { id: sid }).unwrap();
        invariants(&d.m);
        assert_eq!(d.m.root.wires.len(), 1);
    }

    /// Random edits never panic, keep the invariants, and whatever compiles runs.
    #[test]
    fn random_models() {
        let kinds: Vec<&str> = blocks::library().iter().filter(|d| d.planned.is_empty()).map(|d| d.kind).filter(|k| *k != "FromFile").collect();
        let texts = ["1", "-1", "[1 2]", "0", "inf", "1e308", "x", "[1 2; 3 4]", "[]", "+-", "**/", "u1*2", "nan", "2*pi", "(", "[1 0 1]", "-2+1j -2-1j"];
        let n: u64 = std::env::var("DATUM_FUZZ").ok().and_then(|v| v.parse().ok()).unwrap_or(60);
        for seed in 1..n {
            let mut r = Rng(seed * 7919);
            let mut d = Doc::new(Model::new());
            let mut s = d.m.sim.clone();
            s.stop = 0.5;
            s.solver = ["rk45", "stiff", "rk4", "euler", "heun"][r.below(5)].into();
            let _ = d.apply(Op::SetSim { sim: s });
            for _ in 0..40 {
                let ids: Vec<u32> = d.m.root.blocks.iter().map(|b| b.id).collect();
                let op = match r.below(10) {
                    0..=2 => Op::Add { parent: 0, kind: r.pick(&kinds).unwrap().to_string(), x: r.below(800) as f64, y: r.below(600) as f64 },
                    3..=5 if ids.len() >= 2 => {
                        let (a, b) = (*r.pick(&ids).unwrap(), *r.pick(&ids).unwrap());
                        Op::Connect { from: (a, r.below(3)), to: (b, r.below(3)), pts: vec![] }
                    }
                    6 if !ids.is_empty() => {
                        let id = *r.pick(&ids).unwrap();
                        let b = d.m.block(id).unwrap();
                        let keys: Vec<String> = blocks::def(&b.kind).map_or(vec![], |d| d.params.iter().map(|p| p.key.to_string()).collect());
                        match r.pick(&keys) {
                            Some(k) => Op::SetParam { id, key: k.clone(), value: r.pick(&texts).unwrap().to_string() },
                            None => Op::Flip { ids: vec![id] },
                        }
                    }
                    7 if ids.len() > 2 => Op::Group { ids: ids[..r.below(ids.len()) + 1].to_vec() },
                    8 if !ids.is_empty() => Op::Delete { ids: vec![*r.pick(&ids).unwrap()] },
                    _ => {
                        if r.below(2) == 0 {
                            d.undo();
                        } else {
                            d.redo();
                        }
                        continue;
                    }
                };
                let _ = d.apply(op);
                invariants(&d.m);
            }
            if let Ok(sim) = compile(&d.m, None, &BTreeMap::new()) {
                let _ = run(sim, None, None);
            }
            // The file round-trips.
            let back = model::from_json(&model::to_json(&d.m)).unwrap();
            assert_eq!(back, d.m, "seed {seed}");
        }
    }

    /// Damaged files never panic: cut short, garbled, nested too deep, wrong types.
    #[test]
    fn damaged_files() {
        let good = model::to_json(&examples::get("pmsm").unwrap());
        for cut in (0..good.len()).step_by(37) {
            let _ = model::from_json(&good[..cut]);
        }
        let mut r = Rng(42);
        for _ in 0..300 {
            let mut bytes = good.clone().into_bytes();
            for _ in 0..5 {
                let i = r.below(bytes.len());
                bytes[i] = b"{}[],:\"0x-e9 "[r.below(13)];
            }
            if let Ok(m) = model::from_json(&String::from_utf8_lossy(&bytes)) {
                invariants(&m);
                let _ = compile(&m, None, &BTreeMap::new());
            }
        }
        let deep = format!("{{\"root\":{}{}}}", "{\"blocks\":[{\"kind\":\"Subsystem\",\"sub\":".repeat(100), "{}}]}".repeat(100));
        assert!(model::from_json(&deep).is_err());
        for bad in ["", "null", "[]", "42", "{\"format\":99}", "{\"root\":{\"blocks\":7}}", "{\"sim\":{\"stop\":-5}}"] {
            match model::from_json(bad) {
                Ok(m) => invariants(&m),
                Err(e) => assert!(!e.is_empty()),
            }
        }
        // Duplicate ids and wires to nowhere are repaired.
        let mut m = examples::get("msd").unwrap();
        let id0 = m.root.blocks[0].id;
        m.root.blocks[1].id = id0;
        m.root.wires[0].to = (12345, 0);
        let m2 = model::from_json(&model::to_json(&m)).unwrap();
        invariants(&m2);
    }

    /// Speed gate: a 500-block model compiles, routes and runs quickly.
    #[test]
    fn speed_gate() {
        let mut d = Doc::new(Model::new());
        let mut prev = d.apply(Op::Add { parent: 0, kind: "Step".into(), x: 0.0, y: 0.0 }).unwrap()[0];
        for i in 1..500 {
            let kind = ["Gain", "TransferFcn", "Saturation", "Integrator", "Sum"][i % 5];
            let id = d.apply(Op::Add { parent: 0, kind: kind.into(), x: (i % 25) as f64 * 120.0, y: (i / 25) as f64 * 90.0 }).unwrap()[0];
            if kind == "Gain" {
                d.apply(Op::SetParam { id, key: "k".into(), value: "0.999".into() }).unwrap();
            }
            if kind == "Integrator" {
                d.apply(Op::SetParam { id, key: "upper".into(), value: "10".into() }).unwrap();
                d.apply(Op::SetParam { id, key: "lower".into(), value: "-10".into() }).unwrap();
            }
            d.apply(Op::Connect { from: (prev, 0), to: (id, 0), pts: vec![] }).unwrap();
            prev = id;
        }
        let t = std::time::Instant::now();
        let v = crate::view::view(&d.m, 0).unwrap();
        let route_ms = t.elapsed().as_millis();
        assert_eq!(v.wires.len(), 499);
        let t = std::time::Instant::now();
        let sim = compile(&d.m, None, &BTreeMap::new()).unwrap();
        let r = run(sim, None, None).unwrap();
        let run_ms = t.elapsed().as_millis();
        assert!(!r.t.is_empty());
        let limit = if cfg!(debug_assertions) { 8000 } else { 1500 };
        assert!(route_ms < limit as u128, "routing 500 blocks took {route_ms} ms");
        assert!(run_ms < limit as u128, "running 500 blocks took {run_ms} ms");
        eprintln!("speed gate: route {route_ms} ms, compile+run {run_ms} ms ({} steps)", r.stats.steps);
    }
}
