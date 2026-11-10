//! Linearising whole models between analysis points (0.2.3), and finding operating points (trim,
//! 0.2.5). The model is copied with hidden blocks at the points (a held test input, a cut, a
//! measuring gain), compiled, and differentiated by central differences around the operating
//! point, so nonlinear blocks and motors work too; linear blocks come out exact.

use crate::compile::{self, Sim};
use crate::lti::Ss;
use crate::model::{Block, Diagram, Model, Wire};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// Where to linearise.
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum At {
    /// The model's initial state at the start time.
    Initial,
    /// The state reached at time `t` in a run.
    Time { t: f64 },
    /// A saved state (an operating point from trim).
    State { t: f64, x: Vec<f64> },
}

#[derive(Clone, Debug, Serialize)]
pub struct Linear {
    /// All inputs and outputs: loop openings first (input k and output k are the same loop, with
    /// the sign turned so that siso(k, k) is the loop transfer L), then inputs, then outputs.
    pub ss: Ss,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub loops: usize,
    pub t: f64,
    pub x: Vec<f64>,
    pub notes: Vec<String>,
}

struct Pt {
    kind: String,
    name: String,
    /// Hidden blocks: the held input (in and open) and the measuring gain.
    hold: Option<u32>,
    meas: u32,
}

fn max_id(d: &Diagram) -> u32 {
    d.blocks.iter().map(|b| b.id.max(b.sub.as_ref().map_or(0, max_id))).chain(d.wires.iter().map(|w| w.id)).max().unwrap_or(0)
}

fn hidden(id: u32, kind: &str, name: String, params: &[(&str, &str)]) -> Block {
    Block { id, kind: kind.into(), name, params: params.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(), ..Default::default() }
}

/// Copy of the model with the analysis points made real.
fn with_points(m: &Model) -> (Model, Vec<Pt>) {
    let mut m2 = m.clone();
    let mut next = max_id(&m.root).max(m.next) + 1;
    let mut pts = vec![];
    fn walk(d: &mut Diagram, next: &mut u32, pts: &mut Vec<Pt>, prefix: &str) {
        let names: BTreeMap<u32, String> = d.blocks.iter().map(|b| (b.id, b.name.clone())).collect();
        // One point per signal (every wire from the same output carries it).
        let mut sigs: Vec<((u32, usize), String, String)> = vec![];
        for w in &d.wires {
            if !w.point.is_empty() && !sigs.iter().any(|s| s.0 == w.from) {
                let base = if w.name.is_empty() { names.get(&w.from.0).cloned().unwrap_or_default() } else { w.name.clone() };
                let name = if w.from.1 > 0 && w.name.is_empty() { format!("{prefix}{base}.{}", w.from.1 + 1) } else { format!("{prefix}{base}") };
                sigs.push((w.from, w.point.clone(), name));
            }
        }
        for (from, kind, name) in sigs {
            let mut id = || {
                *next += 1;
                *next
            };
            let meas = id();
            d.blocks.push(hidden(meas, "Gain", format!("__lin{meas}"), &[("k", "1")]));
            let mut hold = None;
            let wid = id();
            match kind.as_str() {
                "in" => {
                    let (c, s) = (id(), id());
                    d.blocks.push(hidden(c, "Constant", format!("__lin{c}"), &[("value", "0")]));
                    d.blocks.push(hidden(s, "Sum", format!("__lin{s}"), &[("signs", "++")]));
                    for w in d.wires.iter_mut().filter(|w| w.from == from) {
                        w.from = (s, 0);
                    }
                    d.wires.push(Wire { id: id(), from, to: (s, 0), ..Default::default() });
                    d.wires.push(Wire { id: id(), from: (c, 0), to: (s, 1), ..Default::default() });
                    hold = Some(c);
                }
                "open" => {
                    let c = id();
                    d.blocks.push(hidden(c, "Constant", format!("__lin{c}"), &[("value", "0")]));
                    for w in d.wires.iter_mut().filter(|w| w.from == from) {
                        w.from = (c, 0);
                    }
                    hold = Some(c);
                }
                _ => {}
            }
            d.wires.push(Wire { id: wid, from, to: (meas, 0), ..Default::default() });
            pts.push(Pt { kind, name, hold, meas });
        }
        for b in d.blocks.iter_mut() {
            let p = format!("{prefix}{}/", b.name);
            if let Some(s) = b.sub.as_mut() {
                walk(s, next, pts, &p);
            }
        }
    }
    walk(&mut m2.root, &mut next, &mut pts, "");
    m2.next = next + 1;
    (m2, pts)
}

fn msg(e: Vec<compile::Issue>) -> String {
    e.into_iter().filter(|i| !i.warn).map(|i| i.msg).collect::<Vec<_>>().join("\n")
}

/// The compiled model with its points, at the operating point: (system, t, x, inputs, outputs).
struct Prep {
    sim: Sim,
    t: f64,
    x: Vec<f64>,
    /// (node index of the held input, name, loop?)
    ins: Vec<(usize, String, bool)>,
    /// (signal slot, name, loop?)
    outs: Vec<(usize, String, bool)>,
}

fn prepare(m: &Model, base: Option<&Path>, over: &BTreeMap<String, f64>, at: &At) -> Result<Prep, String> {
    let (m2, pts) = with_points(m);
    if pts.is_empty() {
        return Err("Mark where to look first: right-click a wire › Analysis point › Loop opening (or an Input and an Output).".into());
    }
    let sim = compile::compile(&m2, base, over).map_err(msg)?;
    let (mut sim, t, x) = match at {
        At::Initial => {
            let x = sim.x0.clone();
            (sim, m.sim.start, x)
        }
        At::Time { t } => {
            let (s, x) = compile::run_to(sim, *t)?;
            (s, *t, x)
        }
        At::State { t, x } => {
            if x.len() != sim.nx() {
                return Err(format!("The saved operating point has {} states; the model now has {}. Find it again.", x.len(), sim.nx()));
            }
            (sim, *t, x.clone())
        }
    };
    let mut order: Vec<&Pt> = pts.iter().filter(|p| p.kind == "open").collect();
    order.extend(pts.iter().filter(|p| p.kind == "in"));
    order.extend(pts.iter().filter(|p| p.kind == "out"));
    let (mut ins, mut outs) = (vec![], vec![]);
    for p in &order {
        let (slot, w) = sim.out_of(p.meas).ok_or("An analysis point lost its signal.")?;
        let lp = p.kind == "open";
        if let Some(h) = p.hold {
            if w > 1 {
                return Err(format!("{}: an input or loop opening must be on a single signal, not a vector of {w}.", p.name));
            }
            let i = sim.node_of(h).ok_or("An analysis point lost its input.")?;
            sim.hold(i, 0.0);
            ins.push((i, if lp { format!("Loop at {}", p.name) } else { p.name.clone() }, lp));
        }
        if p.kind != "in" {
            for e in 0..w {
                let n = if w > 1 { format!("{}[{}]", p.name, e + 1) } else { p.name.clone() };
                outs.push((slot + e, if lp { format!("Loop at {n}") } else { n }, lp));
            }
        }
    }
    Ok(Prep { sim, t, x, ins, outs })
}

/// Linearise the model between its analysis points.
pub fn linearise(m: &Model, base: Option<&Path>, over: &BTreeMap<String, f64>, at: &At) -> Result<Linear, String> {
    let Prep { mut sim, t, x, ins, outs } = prepare(m, base, over, at)?;
    let (n, ni, no) = (x.len(), ins.len(), outs.len());
    if ni == 0 || no == 0 {
        return Err("Linearising needs an input and an output: mark a loop opening, or an Input and an Output.".into());
    }
    let mut dx = vec![0.0; n];
    let mut eval = |sim: &mut Sim, x: &[f64], u: &[f64]| -> (Vec<f64>, Vec<f64>) {
        for (k, (i, _, _)) in ins.iter().enumerate() {
            sim.hold(*i, u[k]);
        }
        sim.eval(t, x, &mut dx);
        (dx.clone(), outs.iter().map(|o| sim.signal(o.0)).collect())
    };
    let u0 = vec![0.0; ni];
    let (f0, y0) = eval(&mut sim, &x, &u0);
    if f0.iter().chain(&y0).any(|v| !v.is_finite()) {
        return Err("The model gives non-finite values at this operating point.".into());
    }
    let (mut a, mut b) = (vec![vec![0.0; n]; n], vec![vec![0.0; ni]; n]);
    let (mut c, mut d) = (vec![vec![0.0; n]; no], vec![vec![0.0; ni]; no]);
    for j in 0..n {
        let h = 1e-6 * x[j].abs().max(1.0);
        let (mut xp, mut xm) = (x.clone(), x.clone());
        xp[j] += h;
        xm[j] -= h;
        let (fp, yp) = eval(&mut sim, &xp, &u0);
        let (fm, ym) = eval(&mut sim, &xm, &u0);
        for i in 0..n {
            a[i][j] = (fp[i] - fm[i]) / (2.0 * h);
        }
        for i in 0..no {
            c[i][j] = (yp[i] - ym[i]) / (2.0 * h);
        }
    }
    for j in 0..ni {
        let h = 1e-6;
        let (mut up, mut um) = (u0.clone(), u0.clone());
        up[j] += h;
        um[j] -= h;
        let (fp, yp) = eval(&mut sim, &x, &up);
        let (fm, ym) = eval(&mut sim, &x, &um);
        for i in 0..n {
            b[i][j] = (fp[i] - fm[i]) / (2.0 * h);
        }
        for i in 0..no {
            d[i][j] = (yp[i] - ym[i]) / (2.0 * h);
        }
    }
    // Snap round-off to zero so structural zeros stay zero.
    let snap = |m: &mut Vec<Vec<f64>>| m.iter_mut().flatten().for_each(|v| if v.abs() < 1e-9 { *v = 0.0 });
    snap(&mut a);
    snap(&mut b);
    snap(&mut c);
    snap(&mut d);
    // A loop opening measures −L: turn the sign so siso(k, k) is the loop transfer L.
    for (i, o) in outs.iter().enumerate() {
        if o.2 {
            c[i].iter_mut().for_each(|v| *v = -*v);
            d[i].iter_mut().for_each(|v| *v = -*v);
        }
    }
    let loops = ins.iter().filter(|i| i.2).count();
    let mut notes = vec![];
    if sim.has_discrete() {
        notes.push("The model has discrete blocks: they are held at their values, so their dynamics are not in the linear model.".into());
    }
    if n == 0 {
        notes.push("The model has no continuous states: the linear model is a static gain.".into());
    }
    Ok(Linear {
        ss: Ss { a, b, c, d },
        inputs: ins.into_iter().map(|i| i.1).collect(),
        outputs: outs.into_iter().map(|o| o.1).collect(),
        loops,
        t,
        x,
        notes,
    })
}

/// Everything the Control tab shows for one input–output pair of a linearised model: the Bode
/// plot, poles, step and impulse; for a loop (the same loop opening as input and output) also the
/// margins, Nyquist, sensitivity, root locus and the closed loop's step and impulse.
pub fn report(l: &Linear, inp: usize, outp: usize) -> Result<serde_json::Value, String> {
    use crate::lti;
    use serde_json::json;
    if inp >= l.inputs.len() || outp >= l.outputs.len() {
        return Err("Pick an input and an output of the linear model.".into());
    }
    let g = l.ss.siso(inp, outp);
    let is_loop = inp < l.loops && outp == inp;
    let w = lti::grid(&g, 600);
    let fin = |v: &[f64]| v.iter().map(|x| if x.is_finite() { json!(x) } else { serde_json::Value::Null }).collect::<Vec<_>>();
    let bode = lti::bode(&g, &w);
    let poles: Vec<[f64; 2]> = g.poles().iter().map(|p| [p.re, p.im]).collect();
    let mut v = json!({
        "inputs": l.inputs, "outputs": l.outputs, "loops": l.loops, "t": l.t, "nx": l.x.len(), "notes": l.notes,
        "loop": is_loop, "w": fin(&bode.w), "mag_db": fin(&bode.mag_db), "phase": fin(&bode.phase), "poles": poles,
        "a": l.ss.a, "b": l.ss.b, "c": l.ss.c, "d": l.ss.d,
        "dc": g.at(lti::Cx::new(0.0, 0.0)).re,
    });
    let resp = |s: &lti::Ss| {
        let unstable = s.poles().iter().any(|p| p.re > 1e-9);
        let tf = lti::span(s);
        let st = lti::response(s, tf, 600, false);
        let im = lti::response(s, tf, 600, true);
        let marks = if unstable { serde_json::Value::Null } else { crate::measure::step_marks(&st.t, &st.y).map(|m| json!(m)).unwrap_or(serde_json::Value::Null) };
        json!({ "t": fin(&st.t), "step": fin(&st.y), "impulse": fin(&im.y), "unstable": unstable, "marks": marks })
    };
    if is_loop {
        let m = lti::margins(&g);
        let la = lti::loop_analysis(&g, &w);
        let cl = g.feedback()?;
        v["margins"] = json!({ "gm": if m.gm.is_finite() { json!(m.gm) } else { serde_json::Value::Null }, "gm_db": if m.gm_db.is_finite() { json!(m.gm_db) } else { serde_json::Value::Null },
            "wpc": if m.wpc.is_finite() { json!(m.wpc) } else { serde_json::Value::Null }, "pm": if m.pm.is_finite() { json!(m.pm) } else { serde_json::Value::Null }, "wgc": if m.wgc.is_finite() { json!(m.wgc) } else { serde_json::Value::Null } });
        v["nyquist"] = json!({ "re": fin(&la.nyq_re), "im": fin(&la.nyq_im), "p": la.p_rhp, "z": la.z_rhp, "n": la.encirclements, "stable": la.stable });
        v["sens"] = json!({ "s_db": fin(&la.s_db), "t_db": fin(&la.t_db), "ms": la.ms, "w_ms": la.w_ms });
        v["rlocus"] = json!({ "k": la.rl_k, "poles": la.rl_poles, "open": poles });
        v["closed"] = resp(&cl);
        v["closed_poles"] = json!(cl.poles().iter().map(|p| [p.re, p.im]).collect::<Vec<_>>());
    } else {
        v["open"] = resp(&g);
    }
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::examples;
    use crate::lti;

    fn mark(m: &mut Model, from_name: &str, point: &str) {
        let id = m.root.blocks.iter().find(|b| b.name == from_name).unwrap().id;
        for w in m.root.wires.iter_mut().filter(|w| w.from.0 == id) {
            w.point = point.into();
        }
    }

    /// The mass–spring–damper between its force and its position: 1/(m s² + c s + k).
    #[test]
    fn msd_exact() {
        let mut m = examples::get("msd").unwrap();
        mark(&mut m, "Force", "in");
        mark(&mut m, "Position", "out");
        let l = linearise(&m, None, &BTreeMap::new(), &At::Initial).unwrap();
        assert_eq!((l.inputs.clone(), l.outputs.clone(), l.loops), (vec!["Force".to_string()], vec!["Position".to_string()], 0));
        let g = l.ss.siso(0, 0);
        let (mm, k) = (1.0f64, 10.0f64);
        let c = 2.0 * 0.2 * (k * mm).sqrt();
        for w in [0.1, 1.0, 3.0, 10.0] {
            let got = g.at(lti::Cx::new(0.0, w));
            let den = lti::Cx::new(k - mm * w * w, c * w);
            assert!((got.abs() * den.abs() - 1.0).abs() < 1e-6 && (got.arg() + den.arg()).abs() < 1e-6, "w {w}: {got:?} {:?}", l.ss);
        }
    }

    /// A loop opened at the PID's output in the DC motor position example: L = PID · motor, with
    /// the motor's voltage-to-angle transfer Kt / (s((L s + R)(J s + b) + Kt Ke)).
    #[test]
    fn loop_at_pid() {
        let mut m = examples::get("dcpos").unwrap();
        mark(&mut m, "PID", "open");
        let l = linearise(&m, None, &BTreeMap::new(), &At::Initial).unwrap();
        assert_eq!(l.loops, 1);
        let lp = l.ss.siso(0, 0);
        let pid = lti::Pid { p: 50.0, i: 5.0, d: 5.0, n: 100.0 };
        let (r, ll, ke, kt, j, b) = (2.0, 0.002, 0.05, 0.05, 1e-4, 1e-5);
        for w in [0.5, 5.0, 50.0, 500.0] {
            let s = lti::Cx::new(0.0, w);
            let got = lp.at(s);
            // Motor: Kt / (s((L s + R)(J s + b) + Kt Ke)).
            let e1 = lti::Cx::new(r, ll * w);
            let e2 = lti::Cx::new(b, j * w);
            let prod = lti::Cx::new(e1.re * e2.re - e1.im * e2.im + kt * ke, e1.re * e2.im + e1.im * e2.re);
            let den = lti::Cx::new(-w * prod.im, w * prod.re);
            let c = pid.ss().at(s);
            let want_mag = c.abs() * kt / den.abs();
            let want_arg = c.arg() - den.arg();
            let darg = (got.arg() - want_arg).rem_euclid(2.0 * std::f64::consts::PI);
            assert!((got.abs() / want_mag - 1.0).abs() < 1e-5 && !(1e-5..=2.0 * std::f64::consts::PI - 1e-5).contains(&darg), "w {w}: {got:?} vs {want_mag} ∠{want_arg}");
        }
        // The closed loop's poles are those of the feedback system (this loop is linearly
        // unstable with D = 5 and N = 100; the voltage limit holds the real one).
        let cl = lp.feedback().unwrap().poles();
        let la = lti::loop_analysis(&lp, &lti::grid(&lp, 300));
        assert_eq!(la.z_rhp, cl.iter().filter(|p| p.re > 1e-6).count());
    }
}
