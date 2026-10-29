//! The open document: the model, its file, and undo/redo. The window sends edit operations;
//! each one runs on a copy, and a failed edit leaves the model exactly as it was (Plan's pattern:
//! ops in, whole-model snapshots for undo).

use crate::blocks;
use crate::model::{Block, Diagram, Model, SimSettings, Var, Wire};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Op {
    /// Add a block of a kind to a diagram (0 = top level, else a subsystem's id).
    Add { parent: u32, kind: String, x: f64, y: f64 },
    Move { ids: Vec<u32>, dx: f64, dy: f64 },
    Resize { id: u32, x: f64, y: f64, w: f64, h: f64 },
    SetParam { id: u32, key: String, value: String },
    Rename { id: u32, name: String },
    Flip { ids: Vec<u32> },
    /// Delete blocks and wires (with the wires attached to deleted blocks).
    Delete { ids: Vec<u32> },
    Connect { from: (u32, usize), to: (u32, usize), pts: Vec<(f64, f64)> },
    SetPts { id: u32, pts: Vec<(f64, f64)> },
    /// Name the signal a wire carries ("" removes the name). Every wire from the same output
    /// carries the same signal, so they all take the name.
    NameWire { id: u32, name: String },
    /// Fill colour of blocks ("" = none).
    SetColor { ids: Vec<u32>, color: String },
    /// Paste a clip made by `copy` into a diagram, shifted by (dx, dy).
    Paste { parent: u32, clip: String, dx: f64, dy: f64 },
    /// Put these blocks into a new subsystem; wires crossing the edge get In and Out ports.
    Group { ids: Vec<u32> },
    /// Replace a subsystem by its contents.
    Ungroup { id: u32 },
    SetVars { vars: Vec<Var> },
    SetSim { sim: SimSettings },
    SetMeta { name: String, notes: String },
    Batch(Vec<Op>),
}

#[derive(Clone, Debug, Default)]
pub struct Doc {
    pub m: Model,
    pub path: Option<String>,
    pub dirty: bool,
    undo: Vec<Model>,
    redo: Vec<Model>,
}

const UNDO_LIMIT: usize = 200;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct Clip {
    blocks: Vec<Block>,
    wires: Vec<Wire>,
}

impl Doc {
    pub fn new(m: Model) -> Doc {
        Doc { m, ..Default::default() }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Apply an edit. Returns the ids it created (new blocks, wires).
    pub fn apply(&mut self, op: Op) -> Result<Vec<u32>, String> {
        let mut m = self.m.clone();
        let mut created = vec![];
        exec(&mut m, op, &mut created)?;
        prune(&mut m);
        if m == self.m {
            return Ok(created);
        }
        let old = std::mem::replace(&mut self.m, m);
        self.undo.push(old);
        if self.undo.len() > UNDO_LIMIT {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.dirty = true;
        Ok(created)
    }

    pub fn undo(&mut self) -> bool {
        match self.undo.pop() {
            Some(m) => {
                self.redo.push(std::mem::replace(&mut self.m, m));
                self.dirty = true;
                true
            }
            None => false,
        }
    }

    pub fn redo(&mut self) -> bool {
        match self.redo.pop() {
            Some(m) => {
                self.undo.push(std::mem::replace(&mut self.m, m));
                self.dirty = true;
                true
            }
            None => false,
        }
    }

    /// A clip of these blocks (and the wires between them) for the clipboard.
    pub fn copy(&self, ids: &[u32]) -> String {
        let Some(parent) = ids.first().and_then(|i| self.m.parent_of(*i)) else { return String::new() };
        let Some(d) = self.m.diagram(parent) else { return String::new() };
        let set: HashSet<u32> = ids.iter().cloned().collect();
        let clip = Clip {
            blocks: d.blocks.iter().filter(|b| set.contains(&b.id)).cloned().collect(),
            wires: d.wires.iter().filter(|w| set.contains(&w.from.0) && set.contains(&w.to.0)).cloned().collect(),
        };
        serde_json::to_string(&clip).unwrap_or_default()
    }
}

fn diagram(m: &mut Model, parent: u32) -> Result<&mut Diagram, String> {
    m.diagram_mut(parent).ok_or_else(|| "That diagram no longer exists.".to_string())
}

fn finite(v: &[f64]) -> Result<(), String> {
    if v.iter().all(|x| x.is_finite() && x.abs() < 1e7) {
        Ok(())
    } else {
        Err("Positions must be finite numbers.".into())
    }
}

/// Give every block and wire in a clip fresh ids (subsystems included).
fn reid(m: &mut Model, blocks: &mut [Block], wires: &mut [Wire]) {
    let mut map = HashMap::new();
    for b in blocks.iter_mut() {
        let id = m.id();
        map.insert(b.id, id);
        b.id = id;
        if let Some(s) = &mut b.sub {
            let (mut bs, mut ws) = (std::mem::take(&mut s.blocks), std::mem::take(&mut s.wires));
            reid(m, &mut bs, &mut ws);
            s.blocks = bs;
            s.wires = ws;
        }
    }
    for w in wires.iter_mut() {
        w.id = m.id();
        w.from.0 = map.get(&w.from.0).cloned().unwrap_or(0);
        w.to.0 = map.get(&w.to.0).cloned().unwrap_or(0);
    }
}

fn new_block(m: &mut Model, parent: u32, kind: &str, x: f64, y: f64) -> Result<Block, String> {
    let def = blocks::def(kind).ok_or_else(|| format!("'{kind}' is not a block type."))?;
    let base = match kind {
        "Inport" => "In1",
        "Outport" => "Out1",
        _ => def.name,
    };
    let name = m.unique_name(parent, base);
    let mut b = Block { id: m.id(), kind: kind.into(), name, x: x.round(), y: y.round(), w: def.w, h: def.h, ..Default::default() };
    if kind == "Subsystem" {
        let (i, o) = (m.id(), m.id());
        let mut sub = Diagram::default();
        sub.blocks.push(Block { id: i, kind: "Inport".into(), name: "In1".into(), x: 40.0, y: 60.0, w: 36.0, h: 24.0, ..Default::default() });
        sub.blocks.push(Block { id: o, kind: "Outport".into(), name: "Out1".into(), x: 240.0, y: 60.0, w: 36.0, h: 24.0, ..Default::default() });
        sub.wires.push(Wire { id: m.id(), from: (i, 0), to: (o, 0), ..Default::default() });
        b.sub = Some(sub);
    }
    if kind == "Inport" || kind == "Outport" {
        let k = m.diagram(parent).map_or(0, |d| d.blocks.iter().filter(|x| x.kind == kind).count());
        b.params.insert("port".into(), (k + 1).to_string());
        b.name = m.unique_name(parent, &format!("{}{}", if kind == "Inport" { "In" } else { "Out" }, k + 1));
    }
    Ok(b)
}

fn exec(m: &mut Model, op: Op, created: &mut Vec<u32>) -> Result<(), String> {
    match op {
        Op::Add { parent, kind, x, y } => {
            finite(&[x, y])?;
            let b = new_block(m, parent, &kind, x, y)?;
            created.push(b.id);
            diagram(m, parent)?.blocks.push(b);
        }
        Op::Move { ids, dx, dy } => {
            finite(&[dx, dy])?;
            for id in &ids {
                let b = m.block_mut(*id).ok_or("A block to move no longer exists.")?;
                b.x = (b.x + dx).round();
                b.y = (b.y + dy).round();
            }
            // Bend points of wires whose both ends move go along.
            let set: HashSet<u32> = ids.iter().cloned().collect();
            if let Some(p) = ids.first().and_then(|i| m.parent_of(*i)) {
                for w in &mut diagram(m, p)?.wires {
                    if set.contains(&w.from.0) && set.contains(&w.to.0) {
                        for pt in &mut w.pts {
                            pt.0 += dx;
                            pt.1 += dy;
                        }
                    }
                }
            }
        }
        Op::Resize { id, x, y, w, h } => {
            finite(&[x, y, w, h])?;
            let b = m.block_mut(id).ok_or("The block no longer exists.")?;
            b.x = x.round();
            b.y = y.round();
            b.w = w.round().clamp(10.0, 2000.0);
            b.h = h.round().clamp(10.0, 2000.0);
        }
        Op::SetParam { id, key, value } => {
            let b = m.block_mut(id).ok_or("The block no longer exists.")?;
            let def = blocks::def(&b.kind).ok_or("Unknown block type.")?;
            let p = def.params.iter().find(|p| p.key == key).ok_or_else(|| format!("{} has no parameter '{key}'.", def.name))?;
            if value.len() > 100_000 {
                return Err("The value is too long.".into());
            }
            if !p.choices.is_empty() && !p.choices.contains(&value.as_str()) {
                return Err(format!("'{value}' is not one of the choices."));
            }
            if value == p.def {
                b.params.remove(&key);
            } else {
                b.params.insert(key, value);
            }
        }
        Op::Rename { id, name } => {
            let name = name.trim().to_string();
            if name.is_empty() || name.len() > 200 || name.contains('/') {
                return Err("A block name must not be empty or contain '/'.".into());
            }
            let p = m.parent_of(id).ok_or("The block no longer exists.")?;
            if m.diagram(p).is_some_and(|d| d.blocks.iter().any(|b| b.id != id && b.name == name)) {
                return Err(format!("Another block here is already called '{name}'."));
            }
            m.block_mut(id).ok_or("The block no longer exists.")?.name = name;
        }
        Op::Flip { ids } => {
            for id in ids {
                let b = m.block_mut(id).ok_or("The block no longer exists.")?;
                b.flip = !b.flip;
            }
        }
        Op::Delete { ids } => {
            let set: HashSet<u32> = ids.iter().cloned().collect();
            fn del(d: &mut Diagram, set: &HashSet<u32>) {
                d.blocks.retain(|b| !set.contains(&b.id));
                let ids: HashSet<u32> = d.blocks.iter().map(|b| b.id).collect();
                d.wires.retain(|w| !set.contains(&w.id) && ids.contains(&w.from.0) && ids.contains(&w.to.0));
                for b in &mut d.blocks {
                    if let Some(s) = &mut b.sub {
                        del(s, set);
                    }
                }
            }
            del(&mut m.root, &set);
        }
        Op::Connect { from, to, pts } => {
            finite(&pts.iter().flat_map(|p| [p.0, p.1]).collect::<Vec<_>>())?;
            let pf = m.parent_of(from.0).ok_or("The source block no longer exists.")?;
            let pt = m.parent_of(to.0).ok_or("The target block no longer exists.")?;
            if pf != pt {
                return Err("Wires connect blocks in the same diagram.".into());
            }
            let (bf, bt) = (m.block(from.0).unwrap(), m.block(to.0).unwrap());
            if from.1 >= blocks::ports(bf).1 {
                return Err(format!("{} has no output {}.", bf.name, from.1 + 1));
            }
            if to.1 >= blocks::ports(bt).0 {
                return Err(format!("{} has no input {}.", bt.name, to.1 + 1));
            }
            let id = m.id();
            let d = diagram(m, pf)?;
            d.wires.retain(|w| w.to != to);
            d.wires.push(Wire { id, from, to, pts, ..Default::default() });
            created.push(id);
        }
        Op::SetPts { id, pts } => {
            finite(&pts.iter().flat_map(|p| [p.0, p.1]).collect::<Vec<_>>())?;
            if pts.len() > 200 {
                return Err("Too many bend points.".into());
            }
            let p = m.parent_of(id).ok_or("The wire no longer exists.")?;
            let w = diagram(m, p)?.wires.iter_mut().find(|w| w.id == id).ok_or("The wire no longer exists.")?;
            w.pts = pts;
        }
        Op::NameWire { id, name } => {
            let name = name.trim().to_string();
            if name.len() > 100 || name.contains(['/', '\n']) {
                return Err("A signal name must be one line, without '/'.".into());
            }
            let p = m.parent_of(id).ok_or("The wire no longer exists.")?;
            let d = diagram(m, p)?;
            let from = d.wires.iter().find(|w| w.id == id).ok_or("The wire no longer exists.")?.from;
            for w in d.wires.iter_mut().filter(|w| w.from == from) {
                w.name = name.clone();
            }
        }
        Op::SetColor { ids, color } => {
            if !color.is_empty() && !blocks::COLOURS.iter().any(|(k, _)| *k == color) {
                return Err(format!("Unknown block colour '{color}'."));
            }
            for id in ids {
                m.block_mut(id).ok_or("The block no longer exists.")?.color = color.clone();
            }
        }
        Op::Paste { parent, clip, dx, dy } => {
            finite(&[dx, dy])?;
            let mut c: Clip = serde_json::from_str(&clip).map_err(|_| "The clipboard does not hold Datum Sim blocks.".to_string())?;
            if c.blocks.iter().any(|b| blocks::def(&b.kind).is_none()) {
                return Err("The clipboard holds an unknown block type.".into());
            }
            let (mut bs, mut ws) = (std::mem::take(&mut c.blocks), std::mem::take(&mut c.wires));
            reid(m, &mut bs, &mut ws);
            for b in &mut bs {
                b.x += dx;
                b.y += dy;
                b.name = m.unique_name(parent, &b.name);
                created.push(b.id);
                diagram(m, parent)?.blocks.push(b.clone());
            }
            for w in &mut ws {
                for p in &mut w.pts {
                    p.0 += dx;
                    p.1 += dy;
                }
            }
            diagram(m, parent)?.wires.extend(ws);
        }
        Op::Group { ids } => group(m, &ids, created)?,
        Op::Ungroup { id } => ungroup(m, id)?,
        Op::SetVars { vars } => {
            if vars.len() > 10_000 {
                return Err("Too many variables.".into());
            }
            m.vars = vars;
        }
        Op::SetSim { sim } => {
            if !(sim.start.is_finite() && sim.stop.is_finite() && sim.stop > sim.start) {
                return Err("The stop time must be after the start time.".into());
            }
            if !(sim.step > 0.0 && sim.rtol > 0.0 && sim.atol > 0.0 && sim.max_step >= 0.0) || !sim.step.is_finite() {
                return Err("Step and tolerances must be positive numbers.".into());
            }
            if !["rk45", "stiff", "rk4", "heun", "euler"].contains(&sim.solver.as_str()) {
                return Err("Unknown solver.".into());
            }
            m.sim = sim;
        }
        Op::SetMeta { name, notes } => {
            m.name = name;
            m.notes = notes;
        }
        Op::Batch(ops) => {
            for o in ops {
                exec(m, o, created)?;
            }
        }
    }
    Ok(())
}

fn group(m: &mut Model, ids: &[u32], created: &mut Vec<u32>) -> Result<(), String> {
    let parent = ids.first().and_then(|i| m.parent_of(*i)).ok_or("Select the blocks to group.")?;
    let set: HashSet<u32> = ids.iter().cloned().collect();
    {
        let d = m.diagram(parent).unwrap();
        if ids.iter().any(|i| d.block(*i).is_none()) {
            return Err("Select blocks in one diagram to group them.".into());
        }
        if ids.iter().any(|i| matches!(d.block(*i).unwrap().kind.as_str(), "Inport" | "Outport")) && parent != 0 {
            return Err("In and Out blocks cannot be grouped; they are this subsystem's ports.".into());
        }
    }
    let d = m.diagram(parent).unwrap().clone();
    let inside: Vec<Block> = d.blocks.iter().filter(|b| set.contains(&b.id)).cloned().collect();
    let (x0, y0) = (inside.iter().map(|b| b.x).fold(f64::INFINITY, f64::min), inside.iter().map(|b| b.y).fold(f64::INFINITY, f64::min));
    let (x1, y1) = (inside.iter().map(|b| b.x + b.w).fold(f64::NEG_INFINITY, f64::max), inside.iter().map(|b| b.y + b.h).fold(f64::NEG_INFINITY, f64::max));
    let sid = m.id();
    let mut sub = Diagram::default();
    for b in &inside {
        let mut b = b.clone();
        b.x += 120.0 - x0;
        b.y += 60.0 - y0;
        sub.blocks.push(b);
    }
    let mut outer: Vec<Wire> = vec![];
    // Incoming: one In per distinct source port.
    let mut ins: BTreeMap<(u32, usize), usize> = BTreeMap::new();
    let mut outs: BTreeMap<(u32, usize), usize> = BTreeMap::new();
    let mut nin = 0usize;
    let mut nout = 0usize;
    for w in &d.wires {
        let (a, b) = (set.contains(&w.from.0), set.contains(&w.to.0));
        if a && b {
            let mut w = w.clone();
            w.pts.iter_mut().for_each(|p| {
                p.0 += 120.0 - x0;
                p.1 += 60.0 - y0;
            });
            sub.wires.push(w);
        } else if b {
            let port = *ins.entry(w.from).or_insert_with(|| {
                nin += 1;
                nin
            });
            let pid = match sub.blocks.iter().find(|x| x.kind == "Inport" && blocks::param(x, "port") == port.to_string()) {
                Some(x) => x.id,
                None => {
                    let id = m.id();
                    let y = 60.0 + (port as f64 - 1.0) * 50.0;
                    let mut b = Block { id, kind: "Inport".into(), name: format!("In{port}"), x: 30.0, y, w: 36.0, h: 24.0, ..Default::default() };
                    b.params.insert("port".into(), port.to_string());
                    sub.blocks.push(b);
                    outer.push(Wire { id: m.id(), from: w.from, to: (sid, port - 1), pts: vec![], name: w.name.clone() });
                    id
                }
            };
            sub.wires.push(Wire { id: m.id(), from: (pid, 0), to: w.to, pts: vec![], name: w.name.clone() });
        } else if a {
            let port = *outs.entry(w.from).or_insert_with(|| {
                nout += 1;
                nout
            });
            if !sub.blocks.iter().any(|x| x.kind == "Outport" && blocks::param(x, "port") == port.to_string()) {
                let id = m.id();
                let y = 60.0 + (port as f64 - 1.0) * 50.0;
                let mut b = Block { id, kind: "Outport".into(), name: format!("Out{port}"), x: 160.0 + x1 - x0, y, w: 36.0, h: 24.0, ..Default::default() };
                b.params.insert("port".into(), port.to_string());
                sub.blocks.push(b);
                sub.wires.push(Wire { id: m.id(), from: w.from, to: (id, 0), pts: vec![], name: w.name.clone() });
            }
            outer.push(Wire { id: m.id(), from: (sid, port - 1), to: w.to, pts: vec![], name: w.name.clone() });
        }
    }
    let name = m.unique_name(parent, "Subsystem");
    let (w, h) = (100.0f64, (30.0 * nin.max(nout).max(2) as f64).max(60.0));
    let blk = Block { id: sid, kind: "Subsystem".into(), name, x: ((x0 + x1) / 2.0 - w / 2.0).round(), y: ((y0 + y1) / 2.0 - h / 2.0).round(), w, h, sub: Some(sub), ..Default::default() };
    let dm = diagram(m, parent)?;
    dm.blocks.retain(|b| !set.contains(&b.id));
    dm.wires.retain(|w| !set.contains(&w.from.0) && !set.contains(&w.to.0));
    dm.blocks.push(blk);
    dm.wires.extend(outer);
    created.push(sid);
    Ok(())
}

fn ungroup(m: &mut Model, id: u32) -> Result<(), String> {
    let parent = m.parent_of(id).ok_or("The subsystem no longer exists.")?;
    let b = m.block(id).cloned().ok_or("The subsystem no longer exists.")?;
    let sub = b.sub.clone().ok_or("Only a subsystem can be ungrouped.")?;
    let (pins, pouts) = blocks::sub_ports(&sub);
    let d = m.diagram(parent).unwrap().clone();
    let mut nd = d.clone();
    nd.blocks.retain(|x| x.id != id);
    nd.wires.retain(|w| w.from.0 != id && w.to.0 != id);
    let (sx, sy) = (sub.blocks.iter().map(|b| b.x).fold(f64::INFINITY, f64::min), sub.blocks.iter().map(|b| b.y).fold(f64::INFINITY, f64::min));
    for x in &sub.blocks {
        if pins.contains(&x.id) || pouts.contains(&x.id) {
            continue;
        }
        let mut x = x.clone();
        x.x += b.x - sx;
        x.y += b.y - sy;
        x.name = {
            let used: Vec<&str> = nd.blocks.iter().map(|b| b.name.as_str()).collect();
            if used.contains(&x.name.as_str()) { format!("{}_{}", b.name, x.name) } else { x.name.clone() }
        };
        nd.blocks.push(x);
    }
    for w in &sub.wires {
        let from_in = pins.iter().position(|p| *p == w.from.0);
        let to_out = pouts.iter().position(|p| *p == w.to.0);
        match (from_in, to_out) {
            (None, None) => nd.wires.push(w.clone()),
            (Some(k), None) => {
                if let Some(o) = d.wires.iter().find(|x| x.to == (id, k)) {
                    nd.wires.push(Wire { id: m.id(), from: o.from, to: w.to, pts: vec![], name: o.name.clone() });
                }
            }
            (None, Some(k)) => {
                for o in d.wires.iter().filter(|x| x.from == (id, k)) {
                    nd.wires.push(Wire { id: m.id(), from: w.from, to: o.to, pts: vec![], name: w.name.clone() });
                }
            }
            (Some(a), Some(k)) => {
                if let Some(src) = d.wires.iter().find(|x| x.to == (id, a)) {
                    for o in d.wires.iter().filter(|x| x.from == (id, k)) {
                        nd.wires.push(Wire { id: m.id(), from: src.from, to: o.to, pts: vec![], name: src.name.clone() });
                    }
                }
            }
        }
    }
    *diagram(m, parent)? = nd;
    Ok(())
}

/// Remove wires whose ports no longer exist (after parameter or port changes), and keep one
/// wire per input.
pub fn prune(m: &mut Model) {
    fn walk(d: &mut Diagram) {
        let counts: HashMap<u32, (usize, usize)> = d.blocks.iter().map(|b| (b.id, blocks::ports(b))).collect();
        let mut taken = HashSet::new();
        d.wires.retain(|w| {
            let (Some(a), Some(b)) = (counts.get(&w.from.0), counts.get(&w.to.0)) else { return false };
            w.from.1 < a.1 && w.to.1 < b.0 && taken.insert(w.to)
        });
        for b in &mut d.blocks {
            if let Some(s) = &mut b.sub {
                walk(s);
            }
        }
    }
    walk(&mut m.root);
}
