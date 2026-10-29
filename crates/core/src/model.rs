//! The model: a diagram of blocks and wires (subsystems hold their own diagram), model variables,
//! and the run settings. Saved as `.dsim`: pretty JSON with a format version and migrations.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// 2 (0.2.0): masks on subsystems. Format 1 files open unchanged.
pub const FORMAT: u32 = 2;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Model {
    pub format: u32,
    pub name: String,
    pub notes: String,
    /// Model variables, evaluated in order; block parameters may use them (and sweeps vary them).
    pub vars: Vec<Var>,
    pub sim: SimSettings,
    pub root: Diagram,
    /// The next free id for blocks and wires (ids are unique in the whole model).
    pub next: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Var {
    pub name: String,
    pub expr: String,
    pub unit: String,
    pub note: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SimSettings {
    pub start: f64,
    pub stop: f64,
    /// "rk45" (Dormand–Prince), "stiff" (Rosenbrock 2(3)), "rk4", "heun", "euler".
    pub solver: String,
    /// Fixed step for the fixed-step solvers.
    pub step: f64,
    /// Largest step for the variable-step solvers (0 = automatic: span / 500).
    pub max_step: f64,
    pub rtol: f64,
    pub atol: f64,
    /// Log every signal, so any wire can be read after a run (the signal probe).
    pub log_all: bool,
}

impl Default for SimSettings {
    fn default() -> Self {
        SimSettings { start: 0.0, stop: 10.0, solver: "rk45".into(), step: 0.01, max_step: 0.0, rtol: 1e-6, atol: 1e-8, log_all: false }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Diagram {
    pub blocks: Vec<Block>,
    pub wires: Vec<Wire>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Block {
    pub id: u32,
    pub kind: String,
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    /// Inputs on the right and outputs on the left (for feedback paths).
    pub flip: bool,
    /// Parameter texts (expressions), by key; missing keys take the block's default.
    pub params: BTreeMap<String, String>,
    /// The inside of a subsystem.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub: Option<Diagram>,
    /// A fill colour the user chose ("" = none): one of the names in `blocks::COLOURS`.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub color: String,
    /// A subsystem's mask: its own parameters, picture text, help and typical ratings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mask: Option<Mask>,
}

/// A mask makes a subsystem a block of its own: parameters (set like any block's, stored in the
/// subsystem's `params`) that the blocks inside use by name.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Mask {
    pub params: Vec<MaskParam>,
    /// Text drawn on the block ("" = the subsystem's name).
    pub icon: String,
    /// What the block is and how to use it: its ? card.
    pub help: String,
    /// Typical values, as for the motors.
    pub presets: Vec<MaskPreset>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MaskParam {
    /// The name the blocks inside use (R, Kt, ratio …).
    pub key: String,
    pub label: String,
    /// The default value (an expression in the model's variables and outer masks).
    pub def: String,
    pub unit: String,
    pub hint: String,
    /// When not empty: a choice, and the value is one of these (as text, not evaluated).
    pub choices: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MaskPreset {
    pub name: String,
    pub values: Vec<(String, String)>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Wire {
    pub id: u32,
    /// (block id, output port index)
    pub from: (u32, usize),
    /// (block id, input port index)
    pub to: (u32, usize),
    /// Bend points the user placed; the router goes through them.
    pub pts: Vec<(f64, f64)>,
    /// The signal's name, shown on the wire and in scope legends ("" = none).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub name: String,
}

impl Diagram {
    pub fn block(&self, id: u32) -> Option<&Block> {
        self.blocks.iter().find(|b| b.id == id)
    }
}

impl Model {
    pub fn new() -> Model {
        Model { format: FORMAT, name: "Untitled".into(), next: 1, ..Default::default() }
    }

    pub fn id(&mut self) -> u32 {
        let i = self.next.max(1);
        self.next = i + 1;
        i
    }

    /// The diagram with this id: 0 is the top level, otherwise a subsystem block's id.
    pub fn diagram(&self, id: u32) -> Option<&Diagram> {
        if id == 0 {
            return Some(&self.root);
        }
        fn find(d: &Diagram, id: u32) -> Option<&Diagram> {
            for b in &d.blocks {
                if let Some(s) = &b.sub {
                    if b.id == id {
                        return Some(s);
                    }
                    if let Some(r) = find(s, id) {
                        return Some(r);
                    }
                }
            }
            None
        }
        find(&self.root, id)
    }

    pub fn diagram_mut(&mut self, id: u32) -> Option<&mut Diagram> {
        if id == 0 {
            return Some(&mut self.root);
        }
        fn find(d: &mut Diagram, id: u32) -> Option<&mut Diagram> {
            for b in &mut d.blocks {
                if b.id == id {
                    return b.sub.as_mut();
                }
                if let Some(s) = &mut b.sub {
                    if let Some(r) = find(s, id) {
                        return Some(r);
                    }
                }
            }
            None
        }
        find(&mut self.root, id)
    }

    /// The id of the diagram that holds this block or wire (0 = top level).
    pub fn parent_of(&self, id: u32) -> Option<u32> {
        fn find(d: &Diagram, me: u32, id: u32) -> Option<u32> {
            if d.blocks.iter().any(|b| b.id == id) || d.wires.iter().any(|w| w.id == id) {
                return Some(me);
            }
            d.blocks.iter().find_map(|b| b.sub.as_ref().and_then(|s| find(s, b.id, id)))
        }
        find(&self.root, 0, id)
    }

    /// The chain of subsystem names from the top to this diagram (for the breadcrumb).
    pub fn path_of(&self, id: u32) -> Vec<(u32, String)> {
        let mut out = vec![(0, self.name.clone())];
        let mut chain = Vec::new();
        let mut cur = id;
        while cur != 0 {
            let Some(p) = self.parent_of(cur) else { break };
            let name = self.diagram(p).and_then(|d| d.block(cur)).map(|b| b.name.clone()).unwrap_or_default();
            chain.push((cur, name));
            cur = p;
        }
        chain.reverse();
        out.extend(chain);
        out
    }

    pub fn block(&self, id: u32) -> Option<&Block> {
        self.diagram(self.parent_of(id)?)?.block(id)
    }

    pub fn block_mut(&mut self, id: u32) -> Option<&mut Block> {
        let p = self.parent_of(id)?;
        self.diagram_mut(p)?.blocks.iter_mut().find(|b| b.id == id)
    }

    /// The largest id used anywhere (to repair `next` in damaged files).
    pub fn max_id(&self) -> u32 {
        fn walk(d: &Diagram) -> u32 {
            let b = d.blocks.iter().map(|b| b.id.max(b.sub.as_ref().map_or(0, walk))).max().unwrap_or(0);
            b.max(d.wires.iter().map(|w| w.id).max().unwrap_or(0))
        }
        walk(&self.root)
    }

    /// Every block name used in a diagram (for unique names).
    pub fn unique_name(&self, parent: u32, base: &str) -> String {
        let used: Vec<&str> = self.diagram(parent).map_or(vec![], |d| d.blocks.iter().map(|b| b.name.as_str()).collect());
        if !used.contains(&base) {
            return base.to_string();
        }
        let stem = base.trim_end_matches(|c: char| c.is_ascii_digit());
        (1..).map(|i| format!("{stem}{i}")).find(|n| !used.contains(&n.as_str())).unwrap()
    }
}

/// Read a `.dsim` file's text. Damaged or hostile files give an error, never a panic.
pub fn from_json(text: &str) -> Result<Model, String> {
    let v: serde_json::Value = serde_json::from_str(text).map_err(|e| format!("This is not a Datum Sim model: {e}"))?;
    let v = migrate(v)?;
    let mut m: Model = serde_json::from_value(v).map_err(|e| format!("The model file is damaged: {e}"))?;
    repair(&mut m)?;
    Ok(m)
}

pub fn to_json(m: &Model) -> String {
    serde_json::to_string_pretty(m).unwrap_or_default()
}

/// Bring older formats up to date. Format 1 is the first; format 2 only adds masks, which
/// format 1 files do not have, so they need no change.
fn migrate(v: serde_json::Value) -> Result<serde_json::Value, String> {
    let f = v.get("format").and_then(|f| f.as_u64()).unwrap_or(1);
    if f > FORMAT as u64 {
        return Err(format!(
            "This model was saved by a newer Datum Sim (file format {f}). Update the app to open it."
        ));
    }
    if !v.is_object() {
        return Err("This is not a Datum Sim model.".into());
    }
    Ok(v)
}

/// Make a read model consistent: unique ids, wires between existing blocks, sane numbers.
fn repair(m: &mut Model) -> Result<(), String> {
    m.format = FORMAT;
    fn depth(d: &Diagram) -> usize {
        1 + d.blocks.iter().filter_map(|b| b.sub.as_ref()).map(depth).max().unwrap_or(0)
    }
    if depth(&m.root) > 64 {
        return Err("The model file is damaged: subsystems are nested too deeply.".into());
    }
    let mut seen = std::collections::HashSet::new();
    let mut next = m.max_id() + 1;
    fn fix(d: &mut Diagram, seen: &mut std::collections::HashSet<u32>, next: &mut u32) {
        for b in &mut d.blocks {
            if b.id == 0 || !seen.insert(b.id) {
                b.id = *next;
                *next += 1;
                seen.insert(b.id);
            }
            if !(b.x.is_finite() && b.y.is_finite()) {
                b.x = 0.0;
                b.y = 0.0;
            }
            if !(b.w.is_finite() && b.w > 4.0 && b.w < 4000.0) || !(b.h.is_finite() && b.h > 4.0 && b.h < 4000.0) {
                let (w, h) = crate::blocks::size(&b.kind);
                b.w = w;
                b.h = h;
            }
            if let Some(s) = &mut b.sub {
                fix(s, seen, next);
            }
        }
        let ids: Vec<u32> = d.blocks.iter().map(|b| b.id).collect();
        let mut taken = std::collections::HashSet::new();
        d.wires.retain(|w| ids.contains(&w.from.0) && ids.contains(&w.to.0) && w.from.1 < 64 && w.to.1 < 64 && taken.insert(w.to));
        d.wires.retain(|w| w.pts.iter().all(|p| p.0.is_finite() && p.1.is_finite()));
        for w in &mut d.wires {
            if w.id == 0 || !seen.insert(w.id) {
                w.id = *next;
                *next += 1;
                seen.insert(w.id);
            }
        }
    }
    fix(&mut m.root, &mut seen, &mut next);
    m.next = next.max(m.next);
    let s = &mut m.sim;
    let d = SimSettings::default();
    if !(s.start.is_finite() && s.stop.is_finite() && s.stop > s.start) {
        s.start = d.start;
        s.stop = d.stop;
    }
    for (v, dv) in [(&mut s.step, d.step), (&mut s.rtol, d.rtol), (&mut s.atol, d.atol)] {
        if !(v.is_finite() && *v > 0.0) {
            *v = dv;
        }
    }
    if !(s.max_step.is_finite() && s.max_step >= 0.0) {
        s.max_step = 0.0;
    }
    Ok(())
}
