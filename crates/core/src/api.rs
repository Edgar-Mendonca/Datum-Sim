//! Automation (0.2.1): one session API for the command line and for Python. A session holds one
//! model; requests and answers are JSON, so the command line's `serve` mode is a thin loop and
//! every other language can drive the engine the same way.
//!
//! Blocks are named by their path: "PID", or "DC motor/Gain" inside the subsystem "DC motor".
//! Signals are named as in the scope legends ("DC motor/ω", or a wire's signal name).

use crate::compile::{self, Results};
use crate::doc::{Doc, Op};
use crate::model::{self, Block, Diagram, Model, Var};
use crate::{examples, sweep};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Default)]
pub struct Session {
    pub doc: Doc,
    path: Option<PathBuf>,
}

/// The block at a path of names. Names may themselves hold '/' (a gain called "1/L"), so each
/// level takes the block whose whole name matches the rest of the path, else the subsystem whose
/// name is followed by '/'.
pub fn block_at<'a>(m: &'a Model, path: &str) -> Result<&'a Block, String> {
    let mut rest = path.trim().trim_matches('/');
    if rest.is_empty() {
        return Err("Give a block's name, or its path such as \"DC motor/Gain\".".into());
    }
    let mut d: &Diagram = &m.root;
    loop {
        if let Some(b) = d.blocks.iter().find(|b| b.name == rest) {
            return Ok(b);
        }
        let sub = d.blocks.iter().filter(|b| b.sub.is_some() && rest.starts_with(&format!("{}/", b.name))).max_by_key(|b| b.name.len());
        match sub {
            Some(b) => {
                rest = &rest[b.name.len() + 1..];
                d = b.sub.as_ref().unwrap();
            }
            None => {
                let here: Vec<&str> = d.blocks.iter().map(|b| b.name.as_str()).collect();
                return Err(format!("No block called '{rest}' here. Blocks: {}", here.join(", ")));
            }
        }
    }
}

/// Every block with its path, kind and parameter texts (mask parameters for masked subsystems).
fn blocks_list(m: &Model) -> Vec<Value> {
    fn walk(d: &Diagram, prefix: &str, out: &mut Vec<Value>) {
        for b in &d.blocks {
            if b.kind == "Note" {
                continue;
            }
            let path = format!("{prefix}{}", b.name);
            let keys: Vec<String> = match (&b.mask, crate::blocks::def(&b.kind)) {
                (Some(mk), _) => mk.params.iter().map(|p| p.key.clone()).collect(),
                (None, Some(d)) => d.params.iter().map(|p| p.key.to_string()).collect(),
                _ => vec![],
            };
            let params: BTreeMap<String, String> = keys.into_iter().map(|k| (k.clone(), crate::blocks::param(b, &k))).collect();
            out.push(json!({ "path": path, "kind": b.kind, "params": params, "masked": b.mask.is_some() }));
            if let Some(s) = &b.sub {
                walk(s, &format!("{path}/"), out);
            }
        }
    }
    let mut out = vec![];
    walk(&m.root, "", &mut out);
    out
}

/// Results as JSON: time and the chosen signals (all logged ones when none are named),
/// reduced to at most `points` time points with peaks kept (0 = every point).
pub fn results_json(r: &Results, names: &[String], points: usize) -> Result<Value, String> {
    let cols = pick(r, names)?;
    let (t, ys) = if points > 0 && r.t.len() > points {
        let ys: Vec<&[f64]> = cols.iter().map(|c| r.y[*c].as_slice()).collect();
        // Up to four points a bucket for each signal (its first, last, lowest and highest).
        compile::decimate_multi(&r.t, &ys, r.t[0], *r.t.last().unwrap(), (points / (2 + 2 * ys.len())).max(10))
    } else {
        (r.t.clone(), cols.iter().map(|c| r.y[*c].clone()).collect())
    };
    let signals: Vec<Value> = cols.iter().zip(ys).map(|(c, y)| json!({ "name": r.chans[*c].name, "y": y })).collect();
    Ok(json!({ "t": t, "signals": signals, "stats": r.stats, "truncated": r.truncated }))
}

/// Channel indices for signal names (each name must match one logged signal).
pub fn pick(r: &Results, names: &[String]) -> Result<Vec<usize>, String> {
    if names.is_empty() {
        return Ok((0..r.chans.len()).filter(|i| !r.chans[*i].probe).collect());
    }
    names.iter().map(|n| {
        r.chans.iter().position(|c| &c.name == n).ok_or_else(|| {
            let all: Vec<&str> = r.chans.iter().map(|c| c.name.as_str()).collect();
            format!("No logged signal called '{n}'. Logged: {}. Set log_all to log every signal.", all.join(", "))
        })
    }).collect()
}

fn issues_text(e: Vec<compile::Issue>) -> String {
    e.iter().filter(|i| !i.warn).map(|i| i.msg.clone()).collect::<Vec<_>>().join("\n")
}

fn num_or_text(v: &Value) -> Result<String, String> {
    match v {
        Value::Number(n) => Ok(n.to_string()),
        Value::String(s) => Ok(s.clone()),
        Value::Bool(b) => Ok(b.to_string()),
        _ => Err("A value must be a number or text".into()),
    }
}

/// Variable overrides from {"name": value}: numbers only (as sweeps take them).
fn overrides(v: Option<&Value>) -> Result<BTreeMap<String, f64>, String> {
    let mut o = BTreeMap::new();
    if let Some(Value::Object(m)) = v {
        for (k, x) in m {
            let f = x.as_f64().or_else(|| x.as_str().and_then(|s| s.trim().parse().ok())).ok_or_else(|| format!("{k}: a variable override must be a number"))?;
            o.insert(k.clone(), f);
        }
    }
    Ok(o)
}

impl Session {
    pub fn new() -> Session {
        Session { doc: Doc::new(Model::new()), path: None }
    }

    pub fn open(&mut self, path: &Path) -> Result<(), String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("Could not open {}: {e}", path.display()))?;
        self.doc = Doc::new(model::from_json(&text)?);
        self.path = Some(path.to_path_buf());
        Ok(())
    }

    /// The model file's folder (relative file names in blocks are read from there).
    pub fn base(&self) -> Option<PathBuf> {
        self.path.as_ref().and_then(|p| p.parent().map(Path::to_path_buf))
    }

    pub fn set_var(&mut self, name: &str, value: &str) -> Result<(), String> {
        let mut vars: Vec<Var> = self.doc.m.vars.clone();
        match vars.iter_mut().find(|v| v.name == name) {
            Some(v) => v.expr = value.to_string(),
            None => vars.push(Var { name: name.into(), expr: value.into(), ..Default::default() }),
        }
        self.doc.apply(Op::SetVars { vars })?;
        Ok(())
    }

    pub fn set_param(&mut self, block: &str, key: &str, value: &str) -> Result<(), String> {
        let id = block_at(&self.doc.m, block)?.id;
        self.doc.apply(Op::SetParam { id, key: key.into(), value: value.into() })?;
        Ok(())
    }

    /// Change run settings from {"stop": 5, "solver": "rk4", …}.
    pub fn set_sim(&mut self, v: &Value) -> Result<(), String> {
        let mut s = self.doc.m.sim.clone();
        let Value::Object(m) = v else { return Err("Run settings must be an object".into()) };
        for (k, x) in m {
            let f = || x.as_f64().ok_or_else(|| format!("{k} must be a number"));
            match k.as_str() {
                "start" => s.start = f()?,
                "stop" => s.stop = f()?,
                "step" => s.step = f()?,
                "max_step" => s.max_step = f()?,
                "rtol" => s.rtol = f()?,
                "atol" => s.atol = f()?,
                "solver" => s.solver = x.as_str().ok_or("solver must be text")?.to_string(),
                "log_all" => s.log_all = x.as_bool().ok_or("log_all must be true or false")?,
                _ => return Err(format!("Unknown run setting '{k}'. Settings: start, stop, step, max_step, rtol, atol, solver, log_all")),
            }
        }
        if !["rk45", "stiff", "rk4", "heun", "euler"].contains(&s.solver.as_str()) {
            return Err(format!("Unknown solver '{}'. Solvers: rk45, stiff, rk4, heun, euler", s.solver));
        }
        self.doc.apply(Op::SetSim { sim: s })?;
        Ok(())
    }

    pub fn run(&self, over: &BTreeMap<String, f64>) -> Result<Results, String> {
        compile::simulate(&self.doc.m, self.base().as_deref(), over).map_err(issues_text)
    }

    pub fn save(&mut self, path: &Path) -> Result<(), String> {
        std::fs::write(path, model::to_json(&self.doc.m)).map_err(|e| format!("Could not save {}: {e}", path.display()))?;
        self.path = Some(path.to_path_buf());
        Ok(())
    }

    /// One JSON request: {"cmd": "...", …}. The answer is the result (the caller wraps errors).
    pub fn handle(&mut self, q: &Value) -> Result<Value, String> {
        let cmd = q.get("cmd").and_then(Value::as_str).ok_or("A request needs a \"cmd\"")?;
        let s = |k: &str| q.get(k).and_then(Value::as_str).ok_or_else(|| format!("'{cmd}' needs \"{k}\""));
        match cmd {
            "version" => Ok(json!({ "version": env!("CARGO_PKG_VERSION"), "format": model::FORMAT })),
            "open" => {
                self.open(Path::new(s("path")?))?;
                Ok(json!({ "name": self.doc.m.name }))
            }
            "new" => {
                let m = match q.get("example").and_then(Value::as_str) {
                    Some(id) => examples::get(id).ok_or_else(|| format!("No example '{id}'. Examples: {}", examples::list().iter().map(|e| e.id).collect::<Vec<_>>().join(", ")))?,
                    None => Model::new(),
                };
                self.doc = Doc::new(m);
                self.path = None;
                Ok(json!({ "name": self.doc.m.name }))
            }
            "examples" => Ok(serde_json::to_value(examples::list()).unwrap_or_default()),
            "info" => {
                let m = &self.doc.m;
                let signals = match compile::compile(m, self.base().as_deref(), &BTreeMap::new()) {
                    Ok(sim) => sim.chans.iter().filter(|c| !c.probe).map(|c| c.name.clone()).collect::<Vec<_>>(),
                    Err(_) => vec![],
                };
                Ok(json!({ "name": m.name, "notes": m.notes, "vars": m.vars, "sim": m.sim, "blocks": blocks_list(m), "signals": signals, "path": self.path }))
            }
            "set_var" => {
                self.set_var(s("name")?, &num_or_text(q.get("value").ok_or("'set_var' needs \"value\"")?)?)?;
                Ok(json!(true))
            }
            "set_param" => {
                self.set_param(s("block")?, s("key")?, &num_or_text(q.get("value").ok_or("'set_param' needs \"value\"")?)?)?;
                Ok(json!(true))
            }
            "set_sim" => {
                self.set_sim(q.get("sim").ok_or("'set_sim' needs \"sim\"")?)?;
                Ok(json!(true))
            }
            "check" => {
                let issues = match compile::compile(&self.doc.m, self.base().as_deref(), &BTreeMap::new()) {
                    Ok(sim) => sim.warnings,
                    Err(e) => e,
                };
                let ok = issues.iter().all(|i| i.warn);
                Ok(json!({ "ok": ok, "issues": issues }))
            }
            "run" => {
                let r = self.run(&overrides(q.get("set"))?)?;
                let names: Vec<String> = q.get("signals").and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
                results_json(&r, &names, q.get("points").and_then(Value::as_u64).unwrap_or(0) as usize)
            }
            "sweep" => {
                let var = s("var")?;
                let values: Vec<f64> = q.get("values").and_then(Value::as_array).ok_or("'sweep' needs \"values\"")?.iter().filter_map(Value::as_f64).collect();
                if values.is_empty() {
                    return Err("'sweep' needs at least one value".into());
                }
                let runs = sweep::sweep(&self.doc.m, self.base().as_deref(), var, &values).map_err(issues_text)?;
                let names: Vec<String> = q.get("signals").and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
                let points = q.get("points").and_then(Value::as_u64).unwrap_or(0) as usize;
                let out: Result<Vec<Value>, String> = values.iter().zip(runs.iter()).map(|(v, r)| Ok(json!({ "value": v, "results": results_json(r, &names, points)? }))).collect();
                Ok(json!({ "var": var, "runs": out? }))
            }
            "save" => {
                self.save(Path::new(s("path")?))?;
                Ok(json!(true))
            }
            _ => Err(format!("Unknown request '{cmd}'. Requests: version, open, new, examples, info, set_var, set_param, set_sim, check, run, sweep, save")),
        }
    }

    /// The `serve` loop: one JSON request per line in, one answer per line out, with the request's
    /// "id" echoed: {"id": …, "ok": true, "result": …} or {"id": …, "ok": false, "error": "…"}.
    pub fn answer(&mut self, line: &str) -> String {
        let (id, res) = match serde_json::from_str::<Value>(line) {
            Ok(q) => (q.get("id").cloned().unwrap_or(Value::Null), self.handle(&q)),
            Err(e) => (Value::Null, Err(format!("Not a JSON request: {e}"))),
        };
        match res {
            Ok(v) => json!({ "id": id, "ok": true, "result": v }).to_string(),
            Err(e) => json!({ "id": id, "ok": false, "error": e }).to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ask(s: &mut Session, q: Value) -> Value {
        let a: Value = serde_json::from_str(&s.answer(&q.to_string())).unwrap();
        assert!(a["ok"].as_bool().unwrap(), "{q} → {a}");
        a["result"].clone()
    }

    /// The mass–spring–damper through the session: settings, a variable, a run, a sweep.
    #[test]
    fn session_runs_a_model() {
        let mut s = Session::new();
        ask(&mut s, json!({ "cmd": "new", "example": "msd" }));
        let info = ask(&mut s, json!({ "cmd": "info" }));
        assert!(info["vars"].as_array().unwrap().iter().any(|v| v["name"] == "zeta"));
        let sig = info["signals"][0].as_str().unwrap().to_string();
        ask(&mut s, json!({ "cmd": "set_sim", "sim": { "stop": 20, "rtol": 1e-9, "atol": 1e-11 } }));
        ask(&mut s, json!({ "cmd": "set_var", "name": "zeta", "value": 1 }));
        let r = ask(&mut s, json!({ "cmd": "run", "signals": [sig] }));
        let y = r["signals"][0]["y"].as_array().unwrap();
        let peak = y.iter().filter_map(Value::as_f64).fold(0.0, f64::max);
        assert!(peak <= 1.0 + 1e-6 && (y.last().unwrap().as_f64().unwrap() - 1.0).abs() < 1e-3, "critically damped to 1 (peak {peak})");
        let sw = ask(&mut s, json!({ "cmd": "sweep", "var": "zeta", "values": [0.1, 1.0], "points": 200 }));
        assert_eq!(sw["runs"].as_array().unwrap().len(), 2);
        let n = sw["runs"][0]["results"]["t"].as_array().unwrap().len();
        assert!(n <= 200, "reduced to at most 200 points ({n})");
    }

    /// Blocks by path, inside subsystems; clear errors for unknown names and requests.
    #[test]
    fn paths_and_errors() {
        let mut s = Session::new();
        ask(&mut s, json!({ "cmd": "new", "example": "dcmotor" }));
        let info = ask(&mut s, json!({ "cmd": "info" }));
        let inner = info["blocks"].as_array().unwrap().iter().find(|b| b["path"].as_str().unwrap().contains('/') && b["kind"] == "Gain").unwrap()["path"].as_str().unwrap().to_string();
        ask(&mut s, json!({ "cmd": "set_param", "block": inner, "key": "k", "value": "2" }));
        assert_eq!(block_at(&s.doc.m, &inner).unwrap().params.get("k").map(String::as_str), Some("2"));
        for bad in [json!({ "cmd": "set_param", "block": "Nope", "key": "k", "value": 1 }), json!({ "cmd": "fly" }), json!({ "cmd": "run", "signals": ["nothing"] }), json!({ "cmd": "set_sim", "sim": { "solver": "magic" } })] {
            let a: Value = serde_json::from_str(&s.answer(&bad.to_string())).unwrap();
            assert!(!a["ok"].as_bool().unwrap() && a["error"].as_str().unwrap().len() > 10, "{bad} → {a}");
        }
        let a: Value = serde_json::from_str(&s.answer("not json")).unwrap();
        assert!(!a["ok"].as_bool().unwrap());
    }
}
