//! Compile a model into a runnable system: flatten subsystems, evaluate parameters with the model
//! variables, infer signal widths, sort blocks by direct feedthrough (an algebraic loop is an
//! error), and lay out signals and states. Then run it with any solver and collect results.

use crate::blocks::{self, Pk};
use crate::expr::{self, Mat};
use crate::model::{Block, Diagram, Model};
use crate::rt::{self, Ctx, Pv, Rate, Rt};
use crate::solve::{self, Hooks, Method, Ode, Opts, Stats};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Issue {
    /// The block (0 = the model); the window selects it.
    pub id: u32,
    pub msg: String,
    pub warn: bool,
}

fn err(id: u32, msg: impl Into<String>) -> Issue {
    Issue { id, msg: msg.into(), warn: false }
}

/// A block after flattening.
struct Flat {
    id: u32,
    name: String,
    kind: String,
    block: Block,
    inputs: Vec<Option<(usize, usize)>>,
    nout: usize,
    out_names: Vec<String>,
    /// The diagram it sits in ("" for the top level, "Motor/" inside a subsystem named Motor).
    parent: String,
}

pub struct Node {
    pub id: u32,
    pub name: String,
    pub kind: String,
    rt: Box<dyn Rt>,
    /// (signal offset, width) of each input; a missing wire reads the zero slot.
    ins: Vec<(usize, usize)>,
    pub out_off: usize,
    pub out_w: Vec<usize>,
    xo: usize,
    nx: usize,
    zo: usize,
    nz: usize,
}

impl Clone for Node {
    fn clone(&self) -> Node {
        Node {
            id: self.id,
            name: self.name.clone(),
            kind: self.kind.clone(),
            rt: self.rt.clone_box(),
            ins: self.ins.clone(),
            out_off: self.out_off,
            out_w: self.out_w.clone(),
            xo: self.xo,
            nx: self.nx,
            zo: self.zo,
            nz: self.nz,
        }
    }
}

/// One logged scalar: a sink's input element.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Chan {
    /// The sink block (top-level or inside a subsystem).
    pub sink: u32,
    pub port: usize,
    pub elem: usize,
    /// The signal's name: source block (and port) and element.
    pub name: String,
    /// Logged for the signal probe ("log all signals"): `sink` and `port` are then the block
    /// output the wire starts from, not a sink.
    pub probe: bool,
    #[serde(skip)]
    slot: usize,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Results {
    pub t: Vec<f64>,
    pub chans: Vec<Chan>,
    /// One series per channel, sampled at `t`.
    pub y: Vec<Vec<f64>>,
    pub stats: RunStats,
    pub truncated: bool,
}

#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct RunStats {
    pub steps: u64,
    pub rejected: u64,
    pub fevals: u64,
    pub events: u64,
    pub stopped: bool,
    pub ms: f64,
}

/// Run control shared with the window: stop, pause, single step.
#[derive(Default)]
pub struct Control {
    pub stop: AtomicBool,
    /// 0 run, 1 paused, 2 take one step then pause
    pub pause: AtomicU8,
}

pub const MAX_POINTS: usize = 2_000_000;

#[derive(Clone)]
pub struct Sim {
    /// Each block's rate (by block id), for the sample-time colours.
    pub rates: Vec<(u32, Rate)>,
    pub nodes: Vec<Node>,
    order: Vec<usize>,
    sig: Vec<f64>,
    ubuf: Vec<f64>,
    pub x0: Vec<f64>,
    nz: usize,
    pub chans: Vec<Chan>,
    pub settings: crate::model::SimSettings,
    pub warnings: Vec<Issue>,
    /// Sinks writing CSV files at the end: (block, file, channel range).
    pub files: Vec<(u32, String, Vec<usize>)>,
    tseg: f64,
}

fn eval_vars(m: &Model, over: &BTreeMap<String, f64>) -> Result<BTreeMap<String, Mat>, Vec<Issue>> {
    let mut vars: BTreeMap<String, Mat> = BTreeMap::new();
    let mut issues = vec![];
    for v in &m.vars {
        let name = v.name.trim();
        if name.is_empty() {
            continue;
        }
        if !name.chars().next().is_some_and(|c| c.is_alphabetic() || c == '_') || !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            issues.push(err(0, format!("Variable name '{name}' must start with a letter and use only letters, digits and _")));
            continue;
        }
        if let Some(o) = over.get(name) {
            vars.insert(name.to_string(), Mat::scalar(*o));
            continue;
        }
        match expr::eval(&v.expr, &|n| vars.get(n).cloned()) {
            Ok(x) => {
                vars.insert(name.to_string(), x);
            }
            Err(e) => issues.push(err(0, format!("Variable {name}: {e}"))),
        }
    }
    if issues.is_empty() {
        Ok(vars)
    } else {
        Err(issues)
    }
}

/// Flatten the diagram tree into a list of blocks with resolved input sources.
/// Flattened blocks, and each subsystem's (in, out) pass-through nodes.
type Subs = BTreeMap<u32, (Vec<usize>, Vec<usize>)>;

fn flatten(m: &Model, issues: &mut Vec<Issue>) -> (Vec<Flat>, Subs) {
    let mut flat: Vec<Flat> = Vec::new();
    // block id → flat index; subsystem id → (in pass nodes, out pass nodes)
    let mut at: BTreeMap<u32, usize> = BTreeMap::new();
    let mut subs: BTreeMap<u32, (Vec<usize>, Vec<usize>)> = BTreeMap::new();

    fn nodes(d: &Diagram, prefix: &str, flat: &mut Vec<Flat>, at: &mut BTreeMap<u32, usize>, subs: &mut BTreeMap<u32, (Vec<usize>, Vec<usize>)>, inside: bool) {
        for b in &d.blocks {
            let name = format!("{prefix}{}", b.name);
            // Notes are only for reading.
            if b.kind == "Note" {
                continue;
            }
            if b.kind == "Subsystem" {
                let sd = b.sub.clone().unwrap_or_default();
                nodes(&sd, &format!("{name}/"), flat, at, subs, true);
                let (ins, outs) = blocks::sub_ports(&sd);
                let map = |ids: Vec<u32>| ids.iter().filter_map(|i| at.get(i).cloned()).collect::<Vec<_>>();
                subs.insert(b.id, (map(ins), map(outs)));
                continue;
            }
            let (ni, no) = blocks::ports(b);
            // Ports of subsystems become pass-throughs; at the top level, In is a source of zero
            // and Out a plain sink.
            let (kind, ni, no) = match b.kind.as_str() {
                "Inport" if inside => ("Inport", 1, 1),
                "Outport" if inside => ("Pass", 1, 1),
                "Inport" => ("Constant", 0, 1),
                k => (k, ni, no),
            };
            let (_, onames) = blocks::port_names(b);
            let out_names = (0..no).map(|i| match onames.get(i) {
                Some(n) if !n.is_empty() => format!("{name}.{n}"),
                _ if no > 1 => format!("{name}.{}", i + 1),
                _ => name.clone(),
            }).collect();
            let mut block = b.clone();
            if b.kind == "Inport" && !inside {
                block.kind = "Constant".into();
                block.params.insert("value".into(), "0".into());
            }
            at.insert(b.id, flat.len());
            flat.push(Flat { id: b.id, name, kind: kind.into(), block, inputs: vec![None; ni], nout: no, out_names, parent: prefix.to_string() });
        }
    }
    nodes(&m.root, "", &mut flat, &mut at, &mut subs, false);

    fn wires(d: &Diagram, flat: &mut [Flat], at: &BTreeMap<u32, usize>, subs: &BTreeMap<u32, (Vec<usize>, Vec<usize>)>, issues: &mut Vec<Issue>) {
        for w in &d.wires {
            let src = match subs.get(&w.from.0) {
                Some((_, outs)) => outs.get(w.from.1).map(|i| (*i, 0)),
                None => at.get(&w.from.0).map(|i| (*i, w.from.1)),
            };
            let dst = match subs.get(&w.to.0) {
                Some((ins, _)) => ins.get(w.to.1).map(|i| (*i, 0)),
                None => at.get(&w.to.0).map(|i| (*i, w.to.1)),
            };
            match (src, dst) {
                (Some(s), Some((di, dp))) if s.1 < flat[s.0].nout && dp < flat[di].inputs.len() => {
                    flat[di].inputs[dp] = Some(s);
                    // A named wire names its signal (in legends and results).
                    if !w.name.is_empty() {
                        flat[s.0].out_names[s.1] = w.name.clone();
                    }
                }
                _ => issues.push(Issue { id: w.to.0, msg: "A wire points to a port that no longer exists; it is ignored.".into(), warn: true }),
            }
        }
        for b in &d.blocks {
            if let Some(s) = &b.sub {
                wires(s, flat, at, subs, issues);
            }
        }
    }
    wires(&m.root, &mut flat, &at, &subs, issues);
    goto_from(&mut flat, issues);
    (flat, subs)
}

/// Each From becomes a pass-through fed by the signal into the Goto with its tag.
fn goto_from(flat: &mut [Flat], issues: &mut Vec<Issue>) {
    let gotos: Vec<(usize, String, bool, String)> = flat.iter().enumerate().filter(|(_, f)| f.kind == "Goto")
        .map(|(i, f)| (i, blocks::param(&f.block, "tag").trim().to_string(), blocks::param(&f.block, "scope") == "global", f.parent.clone())).collect();
    // Two Gotos may not send the same tag to the same From.
    for (k, (i, tag, glob, par)) in gotos.iter().enumerate() {
        if gotos[..k].iter().any(|(_, t, g, p)| t == tag && (*g || *glob || p == par)) {
            issues.push(err(flat[*i].id, format!("{}: another Goto already sends the tag '{tag}' here", flat[*i].name)));
        }
    }
    for i in 0..flat.len() {
        if flat[i].kind != "From" {
            continue;
        }
        let tag = blocks::param(&flat[i].block, "tag").trim().to_string();
        let found = gotos.iter().find(|(_, t, g, p)| *t == tag && (*g || *p == flat[i].parent));
        match found {
            Some((g, ..)) => {
                let src = flat[*g].inputs.first().cloned().flatten();
                flat[i].kind = "Pass".into();
                flat[i].inputs = vec![src];
            }
            None => issues.push(err(flat[i].id, format!("{}: no Goto sends the tag '{tag}' (a local Goto must be in the same diagram)", flat[i].name))),
        }
    }
}

fn params<'a>(f: &Flat, vars: &'a BTreeMap<String, Mat>, base: Option<&'a Path>) -> Result<Pv<'a>, String> {
    let def_kind = if f.kind == "Pass" { "Outport" } else { f.kind.as_str() };
    let mut pv = Pv { m: BTreeMap::new(), t: BTreeMap::new(), vars, base };
    let Some(def) = blocks::def(def_kind).or_else(|| blocks::def(&f.block.kind)) else {
        return Err(format!("unknown block type '{}'", f.block.kind));
    };
    for p in &def.params {
        let text = f.block.params.get(p.key).cloned().unwrap_or_else(|| p.def.to_string());
        if p.kind == Pk::Num {
            let v = expr::eval(&text, &|n| vars.get(n).cloned()).map_err(|e| format!("{}: {e}", p.label))?;
            if v.d.iter().any(|x| x.is_nan()) {
                return Err(format!("{}: the value is not a number", p.label));
            }
            pv.m.insert(p.key.into(), v);
        } else {
            pv.t.insert(p.key.into(), text);
        }
    }
    Ok(pv)
}

/// Compile with optional variable overrides (used by sweeps).
pub fn compile(m: &Model, base: Option<&Path>, over: &BTreeMap<String, f64>) -> Result<Sim, Vec<Issue>> {
    let vars = eval_vars(m, over)?;
    let mut issues = vec![];
    let (flat, subs) = flatten(m, &mut issues);
    let n = flat.len();
    // Parameters
    let mut pvs = Vec::with_capacity(n);
    for f in &flat {
        match params(f, &vars, base) {
            Ok(p) => pvs.push(Some(p)),
            Err(e) => {
                issues.push(err(f.id, format!("{}: {e}", f.name)));
                pvs.push(None);
            }
        }
    }
    if issues.iter().any(|i| !i.warn) {
        return Err(issues);
    }
    // Widths: iterate to a fixed point (loops through integrators settle in a few passes).
    let mut ow: Vec<Vec<usize>> = flat.iter().map(|f| vec![1; f.nout]).collect();
    let mut built: Vec<Option<Box<dyn Rt>>> = (0..n).map(|_| None).collect();
    let mut last_iw: Vec<Option<Vec<usize>>> = vec![None; n];
    let mut errs: Vec<Option<String>> = vec![None; n];
    let mut settled = false;
    for _pass in 0..50 {
        let mut changed = false;
        for i in 0..n {
            let iw: Vec<usize> = flat[i].inputs.iter().map(|s| s.map_or(1, |(b, p)| ow[b].get(p).cloned().unwrap_or(1))).collect();
            if last_iw[i].as_ref() == Some(&iw) {
                continue;
            }
            last_iw[i] = Some(iw.clone());
            match rt::build(&flat[i].kind, pvs[i].as_ref().unwrap(), &iw) {
                Ok((r, w)) => {
                    errs[i] = None;
                    built[i] = Some(r);
                    let w: Vec<usize> = (0..flat[i].nout).map(|k| w.get(k).cloned().unwrap_or(1)).collect();
                    if w != ow[i] {
                        ow[i] = w;
                        changed = true;
                    }
                }
                Err(e) => errs[i] = Some(e),
            }
        }
        if !changed {
            settled = true;
            break;
        }
    }
    if !settled {
        return Err(vec![err(0, "Signal widths keep growing: a loop passes through a Mux that widens the signal every time round.")]);
    }
    for (i, e) in errs.iter().enumerate() {
        if let Some(e) = e {
            issues.push(err(flat[i].id, format!("{}: {e}", flat[i].name)));
        }
    }
    if issues.iter().any(|i| !i.warn) {
        return Err(issues);
    }
    for i in 0..n {
        let iw: Vec<usize> = flat[i].inputs.iter().map(|s| s.map_or(1, |(b, p)| ow[b].get(p).cloned().unwrap_or(1))).collect();
        if last_iw[i].as_ref() != Some(&iw) || built[i].is_none() {
            return Err(vec![err(flat[i].id, format!("{}: its input widths did not settle", flat[i].name))]);
        }
    }
    let rts: Vec<Box<dyn Rt>> = built.into_iter().map(|r| r.unwrap()).collect();
    let rates = rates(&flat, &rts);
    // Unconnected inputs read zero (a warning).
    for f in &flat {
        for (k, s) in f.inputs.iter().enumerate() {
            if s.is_none() {
                issues.push(Issue { id: f.id, msg: format!("{}: input {} is not connected (0 is used)", f.name, k + 1), warn: true });
            }
        }
    }
    // Execution order: Kahn's algorithm on direct-feedthrough edges.
    let mut indeg = vec![0usize; n];
    let mut users: Vec<Vec<usize>> = vec![vec![]; n];
    for (i, f) in flat.iter().enumerate() {
        if rts[i].feed() {
            for (src, _) in f.inputs.iter().flatten() {
                indeg[i] += 1;
                users[*src].push(i);
            }
        }
    }
    let mut order = Vec::with_capacity(n);
    let mut q: Vec<usize> = (0..n).filter(|i| indeg[*i] == 0).collect();
    while let Some(i) = q.pop() {
        order.push(i);
        for &u in &users[i] {
            indeg[u] -= 1;
            if indeg[u] == 0 {
                q.push(u);
            }
        }
    }
    if order.len() < n {
        // Walk back along feedthrough inputs inside the remaining set until a block repeats.
        let left: Vec<bool> = (0..n).map(|i| indeg[i] > 0).collect();
        let start = (0..n).find(|i| left[*i]).unwrap();
        let mut path = vec![start];
        let mut cur = start;
        loop {
            let next = flat[cur].inputs.iter().flatten().map(|s| s.0).find(|s| left[*s]);
            let Some(nx) = next else { break };
            if let Some(pos) = path.iter().position(|p| *p == nx) {
                path = path[pos..].to_vec();
                break;
            }
            path.push(nx);
            cur = nx;
        }
        path.reverse();
        let mut names: Vec<String> = path.iter().map(|i| flat[*i].name.clone()).collect();
        names.push(names[0].clone());
        issues.push(err(flat[path[0]].id, format!(
            "Algebraic loop: {}. Every loop needs a block with state (an Integrator, Transfer function with more poles than zeros, Delay or Unit delay).",
            names.join(" → ")
        )));
        return Err(issues);
    }
    // Lay out signals (outputs of a block are contiguous), states and zero-crossings.
    let mut off = 0;
    let mut outs = vec![];
    for w in &ow {
        outs.push(off);
        off += w.iter().sum::<usize>();
    }
    let zero = off;
    let port_slot = |b: usize, p: usize| outs[b] + ow[b][..p].iter().sum::<usize>();
    let (mut xo, mut zo) = (0, 0);
    let mut nodes = Vec::with_capacity(n);
    for (i, (f, mut r)) in flat.iter().zip(rts).enumerate() {
        let ins = f.inputs.iter().map(|s| s.map_or((zero, 1), |(b, p)| (port_slot(b, p), ow[b][p]))).collect();
        let (nx, nz) = (r.nx(), r.nzc());
        let mut x = vec![0.0; nx];
        r.init(&mut x);
        nodes.push((Node { id: f.id, name: f.name.clone(), kind: f.kind.clone(), rt: r, ins, out_off: outs[i], out_w: ow[i].clone(), xo, nx, zo, nz }, x));
        xo += nx;
        zo += nz;
    }
    let mut x0 = vec![];
    let nodes: Vec<Node> = nodes.into_iter().map(|(nd, x)| {
        x0.extend(x);
        nd
    }).collect();
    // Logged channels and file writers.
    let mut chans = vec![];
    let mut files = vec![];
    for (i, f) in flat.iter().enumerate() {
        if !matches!(f.kind.as_str(), "Scope" | "XY" | "Display" | "ToFile" | "ToPlot") {
            continue;
        }
        let first = chans.len();
        for (p, s) in f.inputs.iter().enumerate() {
            let (slot, w) = nodes[i].ins[p];
            let base = s.map_or_else(|| "(not connected)".to_string(), |(b, bp)| flat[b].out_names[bp].clone());
            for e in 0..w {
                let name = if w > 1 { format!("{base}[{}]", e + 1) } else { base.clone() };
                chans.push(Chan { sink: f.id, port: p, elem: e, name, probe: false, slot: slot + e });
            }
        }
        if f.kind == "ToFile" {
            files.push((f.id, blocks::param(&f.block, "file"), (first..chans.len()).collect()));
        }
    }
    // Every block output, for the probe: by the block (or subsystem) the wire leaves from.
    if m.sim.log_all {
        let mut outs: Vec<(u32, usize, usize, usize)> = vec![];
        for (i, f) in flat.iter().enumerate() {
            for p in 0..f.nout {
                outs.push((f.id, p, i, p));
            }
        }
        for (sid, (_, o)) in &subs {
            for (p, i) in o.iter().enumerate() {
                outs.push((*sid, p, *i, 0));
            }
        }
        for (id, port, i, p) in outs {
            let w = ow[i][p];
            for e in 0..w {
                let base = &flat[i].out_names[p];
                let name = if w > 1 { format!("{base}[{}]", e + 1) } else { base.clone() };
                chans.push(Chan { sink: id, port, elem: e, name, probe: true, slot: port_slot(i, p) + e });
            }
        }
    }
    let nz = zo;
    Ok(Sim {
        rates,
        nodes,
        order,
        sig: vec![0.0; zero + 1],
        ubuf: vec![0.0; 64],
        x0,
        nz,
        chans,
        settings: m.sim.clone(),
        warnings: issues,
        files,
        tseg: m.sim.start,
    })
}

/// Combine the rates of a block's inputs: constants drop out; one rate is kept; several make it
/// mixed.
fn combine(rs: impl Iterator<Item = Rate>) -> Rate {
    let mut out = Rate::Constant;
    for r in rs {
        out = match (out, r) {
            (o, Rate::Constant | Rate::Inherit) => o,
            (Rate::Constant, r) => r,
            (Rate::Discrete(a), Rate::Discrete(b)) if (a - b).abs() <= 1e-12 * a.max(b) => Rate::Discrete(a),
            (Rate::Continuous, Rate::Continuous) => Rate::Continuous,
            _ => Rate::Mixed,
        };
    }
    out
}

/// Rates by fixed-point iteration over the inputs, as the widths are.
fn rates(flat: &[Flat], rts: &[Box<dyn Rt>]) -> Vec<(u32, Rate)> {
    let n = flat.len();
    let own: Vec<Rate> = (0..n).map(|i| if rts[i].nx() > 0 { Rate::Continuous } else { rts[i].rate() }).collect();
    let mut r: Vec<Rate> = own.iter().map(|o| if *o == Rate::Inherit { Rate::Constant } else { *o }).collect();
    for _ in 0..=n.min(200) {
        let mut changed = false;
        for i in 0..n {
            if own[i] != Rate::Inherit {
                continue;
            }
            // A block with no inputs and no rate of its own (a time source) is continuous.
            let v = if flat[i].inputs.is_empty() { Rate::Continuous } else { combine(flat[i].inputs.iter().flatten().map(|(b, _)| r[*b])) };
            if r[i] != v {
                r[i] = v;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    (0..n).map(|i| (flat[i].id, r[i])).collect()
}

/// One block's rate as the window shows it: "constant", "continuous", "discrete" (with ts) or
/// "mixed". Subsystems take the combined rate of everything inside them.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct RateView {
    pub id: u32,
    pub rate: &'static str,
    pub ts: f64,
}

pub fn rate_view(m: &Model, sim: &Sim) -> Vec<RateView> {
    let by: BTreeMap<u32, Rate> = sim.rates.iter().cloned().collect();
    fn inside(d: &Diagram, by: &BTreeMap<u32, Rate>, out: &mut Vec<Rate>) {
        for b in &d.blocks {
            match &b.sub {
                Some(s) => inside(s, by, out),
                None => out.extend(by.get(&b.id).cloned()),
            }
        }
    }
    fn walk(d: &Diagram, by: &BTreeMap<u32, Rate>, v: &mut Vec<RateView>) {
        for b in &d.blocks {
            let r = match &b.sub {
                Some(s) => {
                    let mut rs = vec![];
                    inside(s, by, &mut rs);
                    walk(s, by, v);
                    Some(combine(rs.into_iter()))
                }
                None => by.get(&b.id).cloned(),
            };
            if let Some(r) = r {
                let (rate, ts) = match r {
                    Rate::Constant | Rate::Inherit => ("constant", 0.0),
                    Rate::Continuous => ("continuous", 0.0),
                    Rate::Discrete(ts) => ("discrete", ts),
                    Rate::Mixed => ("mixed", 0.0),
                };
                v.push(RateView { id: b.id, rate, ts });
            }
        }
    }
    let mut v = vec![];
    walk(&m.root, &by, &mut v);
    v
}

impl Sim {
    pub fn nx(&self) -> usize {
        self.x0.len()
    }

    fn gather(sig: &[f64], ins: &[(usize, usize)], ubuf: &mut Vec<f64>) {
        ubuf.clear();
        for (o, w) in ins {
            ubuf.extend_from_slice(&sig[*o..*o + *w]);
        }
    }

    /// Compute every output at (t, x).
    fn outputs(&mut self, c: &Ctx, x: &[f64]) {
        let Sim { nodes, order, sig, ubuf, .. } = self;
        for &i in order.iter() {
            let nd = &nodes[i];
            Self::gather(sig, &nd.ins, ubuf);
            let w: usize = nd.out_w.iter().sum();
            nd.rt.out(c, &x[nd.xo..nd.xo + nd.nx], ubuf, &mut sig[nd.out_off..nd.out_off + w]);
        }
    }

    fn zcs(&mut self, c: &Ctx, x: &[f64], z: &mut [f64]) {
        self.outputs(c, x);
        let Sim { nodes, sig, ubuf, .. } = self;
        for nd in nodes.iter().filter(|n| n.nz > 0) {
            Self::gather(sig, &nd.ins, ubuf);
            nd.rt.zc(c, &x[nd.xo..nd.xo + nd.nx], ubuf, &mut z[nd.zo..nd.zo + nd.nz]);
        }
    }

    pub fn opts(&self) -> Opts {
        let s = &self.settings;
        let span = s.stop - s.start;
        Opts {
            method: Method::parse(&s.solver),
            t0: s.start,
            t1: s.stop,
            h: s.step.min(span),
            hmax: if s.max_step > 0.0 { s.max_step } else { span / 500.0 },
            rtol: s.rtol,
            atol: s.atol,
        }
    }
}

impl Ode for Sim {
    fn n(&self) -> usize {
        self.x0.len()
    }
    fn f(&mut self, t: f64, tseg: f64, x: &[f64], dx: &mut [f64]) {
        let c = Ctx { t, tseg, major: false };
        self.outputs(&c, x);
        let Sim { nodes, sig, ubuf, .. } = self;
        for nd in nodes.iter().filter(|n| n.nx > 0) {
            Self::gather(sig, &nd.ins, ubuf);
            nd.rt.deriv(&c, &x[nd.xo..nd.xo + nd.nx], ubuf, &mut dx[nd.xo..nd.xo + nd.nx]);
        }
    }
}

/// The running simulation: the system, the results, and a callback for live updates.
pub struct Runner<'a> {
    pub sim: Sim,
    pub res: Results,
    pub ctl: Option<&'a Control>,
    pub on_major: Option<&'a mut dyn FnMut(&Results)>,
}

impl Hooks for Runner<'_> {
    fn next_hit(&self, t: f64) -> f64 {
        self.sim.nodes.iter().map(|n| n.rt.next_hit(t)).fold(f64::INFINITY, f64::min)
    }
    fn major(&mut self, t: f64, x: &mut [f64]) -> bool {
        let c = Ctx { t, tseg: t, major: true };
        self.sim.outputs(&c, x);
        self.sim.tseg = t;
        if self.res.t.len() < MAX_POINTS {
            self.res.t.push(t);
            for (k, ch) in self.sim.chans.iter().enumerate() {
                self.res.y[k].push(self.sim.sig[ch.slot]);
            }
        } else {
            self.res.truncated = true;
        }
        let Sim { nodes, sig, ubuf, .. } = &mut self.sim;
        for nd in nodes.iter_mut() {
            Sim::gather(sig, &nd.ins, ubuf);
            nd.rt.update(&c, &mut x[nd.xo..nd.xo + nd.nx], ubuf);
        }
        if let Some(cb) = self.on_major.as_mut() {
            cb(&self.res);
        }
        if let Some(ctl) = self.ctl {
            loop {
                if ctl.stop.load(Ordering::Relaxed) {
                    return false;
                }
                match ctl.pause.load(Ordering::Relaxed) {
                    0 => break,
                    2 => {
                        ctl.pause.store(1, Ordering::Relaxed);
                        break;
                    }
                    _ => std::thread::sleep(std::time::Duration::from_millis(15)),
                }
            }
        }
        true
    }
    fn nzc(&self) -> usize {
        self.sim.nz
    }
    fn zc(&mut self, t: f64, x: &[f64], z: &mut [f64]) {
        let c = Ctx { t, tseg: self.sim.tseg, major: false };
        self.sim.zcs(&c, x, z);
    }
}

impl Ode for Runner<'_> {
    fn n(&self) -> usize {
        self.sim.n()
    }
    fn f(&mut self, t: f64, tseg: f64, x: &[f64], dx: &mut [f64]) {
        self.sim.f(t, tseg, x, dx)
    }
}

/// Run a compiled system to the stop time.
pub fn run<'a>(sim: Sim, ctl: Option<&'a Control>, on_major: Option<&'a mut dyn FnMut(&Results)>) -> Result<Results, String> {
    let started = std::time::Instant::now();
    let opts = sim.opts();
    if !opts.method.adaptive() && !(opts.h > 0.0) {
        return Err("The fixed step must be positive.".into());
    }
    let mut x = sim.x0.clone();
    let chans = sim.chans.clone();
    let k = chans.len();
    let mut r = Runner { sim, res: Results { chans, y: vec![vec![]; k], ..Default::default() }, ctl, on_major };
    let st: Stats = solve::drive(&mut r, &mut x, &opts)?;
    let mut res = r.res;
    res.stats = RunStats { steps: st.steps, rejected: st.rejected, fevals: st.fevals, events: st.events, stopped: st.stopped, ms: started.elapsed().as_secs_f64() * 1e3 };
    Ok(res)
}

/// Compile and run in one go (tests, sweeps, the command line).
pub fn simulate(m: &Model, base: Option<&Path>, over: &BTreeMap<String, f64>) -> Result<Results, Vec<Issue>> {
    let sim = compile(m, base, over)?;
    run(sim, None, None).map_err(|e| vec![err(0, e)])
}

/// Write the channels of a sink as CSV (time first).
pub fn csv(res: &Results, cols: &[usize]) -> String {
    let mut s = String::from("time");
    for c in cols {
        s.push(',');
        s.push_str(&res.chans[*c].name.replace(',', " "));
    }
    s.push('\n');
    for (i, t) in res.t.iter().enumerate() {
        s.push_str(&format!("{t}"));
        for c in cols {
            s.push_str(&format!(",{}", res.y[*c][i]));
        }
        s.push('\n');
    }
    s
}

/// Reduce a series for drawing: per bucket of time, keep the first, min, max and last points, so
/// peaks survive at any zoom.
pub fn decimate(t: &[f64], y: &[f64], t0: f64, t1: f64, buckets: usize) -> (Vec<f64>, Vec<f64>) {
    let i0 = t.partition_point(|v| *v < t0).saturating_sub(1);
    let i1 = (t.partition_point(|v| *v <= t1) + 1).min(t.len());
    if i1 <= i0 {
        return (vec![], vec![]);
    }
    if i1 - i0 <= buckets * 4 {
        return (t[i0..i1].to_vec(), y[i0..i1].to_vec());
    }
    let (mut ot, mut oy) = (Vec::with_capacity(buckets * 4), Vec::with_capacity(buckets * 4));
    let span = (t1 - t0).max(1e-300);
    let mut i = i0;
    for b in 0..buckets {
        let end_t = t0 + span * (b + 1) as f64 / buckets as f64;
        let st = i;
        while i < i1 && (t[i] <= end_t || b == buckets - 1) {
            i += 1;
        }
        if i == st {
            continue;
        }
        let seg = st..i;
        let (mut lo, mut hi) = (st, st);
        for k in seg.clone() {
            if y[k] < y[lo] {
                lo = k;
            }
            if y[k] > y[hi] {
                hi = k;
            }
        }
        let mut idx = vec![st, lo, hi, i - 1];
        idx.sort_unstable();
        idx.dedup();
        for k in idx {
            ot.push(t[k]);
            oy.push(y[k]);
        }
    }
    (ot, oy)
}

/// A block's linear system, for analysis (Bode, step, poles and zeros, margins).
#[derive(Clone, Debug, Serialize)]
pub struct LinSys {
    pub name: String,
    pub a: Vec<Vec<f64>>,
    pub b: Vec<Vec<f64>>,
    pub c: Vec<Vec<f64>>,
    pub d: Vec<Vec<f64>>,
}

fn rows(m: &Mat) -> Vec<Vec<f64>> {
    (0..m.r).map(|i| (0..m.c).map(|j| m.at(i, j)).collect()).collect()
}

/// The linear model of one block: transfer function, zero-pole-gain, state space, PID, gain,
/// integrator, or the DC motor (voltage to speed).
pub fn linear(m: &Model, id: u32) -> Result<LinSys, String> {
    let b = m.block(id).ok_or("Select a block first.")?.clone();
    let vars = eval_vars(m, &BTreeMap::new()).map_err(|e| e.into_iter().map(|i| i.msg).collect::<Vec<_>>().join("; "))?;
    let f = Flat { id, name: b.name.clone(), kind: b.kind.clone(), block: b.clone(), inputs: vec![], nout: 1, out_names: vec![], parent: String::new() };
    let p = params(&f, &vars, None).map_err(|e| format!("{}: {e}", b.name))?;
    let s = |k: &str| p.m.get(k).and_then(|v| v.s().ok()).ok_or_else(|| format!("{}: {k} must be a single number", b.name));
    let lin = match b.kind.as_str() {
        "TransferFcn" | "ZPK" | "StateSpace" => {
            // Build once to get the block's own checks and messages.
            rt::build(&b.kind, &p, &[p.m.get("B").map_or(1, |m| m.c)]).map_err(|e| format!("{}: {e}", b.name))?;
            match b.kind.as_str() {
                "TransferFcn" => rt::tf2ss(&p.m["num"].d, &p.m["den"].d)?,
                "ZPK" => {
                    let z = rt::poly(&rt::parse_roots(&p.t["zeros"])?)?;
                    let k = s("k")?;
                    rt::tf2ss(&z.iter().map(|v| v * k).collect::<Vec<_>>(), &rt::poly(&rt::parse_roots(&p.t["poles"])?)?)?
                }
                _ => {
                    let mut d = p.m["D"].clone();
                    let (bm, cm) = (p.m["B"].clone(), p.m["C"].clone());
                    if d.is_scalar() && d.d[0] == 0.0 {
                        d = Mat { r: cm.r, c: bm.c, d: vec![0.0; cm.r * bm.c] };
                    }
                    rt::Lin { a: p.m["A"].clone(), b: bm, c: cm, d, x0: vec![] }
                }
            }
        }
        "PID" => {
            let (kp, ki, kd, n) = (s("P")?, s("I")?, s("D")?, s("N")?);
            if kd != 0.0 {
                rt::tf2ss(&[kp + kd * n, kp * n + ki, ki * n], &[1.0, n, 0.0])?
            } else if ki != 0.0 {
                rt::tf2ss(&[kp, ki], &[1.0, 0.0])?
            } else {
                rt::tf2ss(&[kp], &[1.0])?
            }
        }
        "Gain" => rt::tf2ss(&[s("k")?], &[1.0])?,
        "Integrator" => rt::tf2ss(&[1.0], &[1.0, 0.0])?,
        "DCMotor" => {
            let (r, l, ke, kt, j, bf) = (s("R")?, s("L")?, s("Ke")?, s("Kt")?, s("J")?, s("b")?);
            rt::Lin {
                a: Mat { r: 2, c: 2, d: vec![-r / l, -ke / l, kt / j, -bf / j] },
                b: Mat { r: 2, c: 1, d: vec![1.0 / l, 0.0] },
                c: Mat { r: 1, c: 2, d: vec![0.0, 1.0] },
                d: Mat::scalar(0.0),
                x0: vec![],
            }
        }
        k => return Err(format!("Linear analysis works on transfer functions, zero-pole-gain, state space, PID, gain, integrator and DC motor blocks, not on '{k}'. Linearising whole models arrives in 0.3.")),
    };
    Ok(LinSys { name: b.name, a: rows(&lin.a), b: rows(&lin.b), c: rows(&lin.c), d: rows(&lin.d) })
}

/// Reduce several series sampled at the same times, keeping each one's peaks: the union of the
/// points `decimate` would keep for every series, so they still share one time axis.
pub fn decimate_multi(t: &[f64], ys: &[&[f64]], t0: f64, t1: f64, buckets: usize) -> (Vec<f64>, Vec<Vec<f64>>) {
    let i0 = t.partition_point(|v| *v < t0).saturating_sub(1);
    let i1 = (t.partition_point(|v| *v <= t1) + 1).min(t.len());
    if i1 <= i0 {
        return (vec![], vec![vec![]; ys.len()]);
    }
    let idx: Vec<usize> = if i1 - i0 <= buckets * 4 {
        (i0..i1).collect()
    } else {
        let span = (t1 - t0).max(1e-300);
        let mut keep = Vec::with_capacity(buckets * (2 + 2 * ys.len()));
        let mut i = i0;
        for b in 0..buckets {
            let end_t = t0 + span * (b + 1) as f64 / buckets as f64;
            let st = i;
            while i < i1 && (t[i] <= end_t || b == buckets - 1) {
                i += 1;
            }
            if i == st {
                continue;
            }
            keep.push(st);
            keep.push(i - 1);
            for y in ys {
                let (mut lo, mut hi) = (st, st);
                for k in st..i {
                    if y[k] < y[lo] {
                        lo = k;
                    }
                    if y[k] > y[hi] {
                        hi = k;
                    }
                }
                keep.push(lo);
                keep.push(hi);
            }
        }
        keep.sort_unstable();
        keep.dedup();
        keep
    };
    (idx.iter().map(|i| t[*i]).collect(), ys.iter().map(|y| idx.iter().map(|i| y[*i]).collect()).collect())
}

#[cfg(test)]
mod decimate_tests {
    #[test]
    fn keeps_peaks_on_a_shared_axis() {
        let t: Vec<f64> = (0..100_000).map(|i| i as f64 * 1e-3).collect();
        let a: Vec<f64> = t.iter().map(|x| if (*x - 37.0).abs() < 1e-9 { 9.0 } else { x.sin() }).collect();
        let b: Vec<f64> = t.iter().map(|x| if (*x - 61.5).abs() < 1e-9 { -7.0 } else { x.cos() }).collect();
        let (tt, ys) = super::decimate_multi(&t, &[&a, &b], 0.0, 100.0, 200);
        assert!(tt.len() < 2000 && ys[0].len() == tt.len() && ys[1].len() == tt.len());
        assert!(ys[0].contains(&9.0) && ys[1].contains(&-7.0), "peaks of both series are kept");
        assert!(tt.windows(2).all(|w| w[0] < w[1]));
    }
}
