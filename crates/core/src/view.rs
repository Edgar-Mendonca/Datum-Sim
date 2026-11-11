//! What the window draws for one diagram: blocks with their port positions, labels and
//! parameters, and the routed wires. The engine owns this geometry so routing and drawing agree.

use crate::blocks;
use crate::model::Model;
use crate::route::{self, Rect, Req};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Serialize)]
pub struct BlockView {
    pub id: u32,
    pub kind: String,
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub flip: bool,
    pub label: String,
    pub ins: Vec<(f64, f64)>,
    pub outs: Vec<(f64, f64)>,
    /// Physical ports (0.5.1): positions, domains and names.
    pub phys: Vec<(f64, f64)>,
    pub phys_dom: Vec<String>,
    pub phys_names: Vec<String>,
    pub in_names: Vec<String>,
    pub out_names: Vec<String>,
    /// Every parameter's text (the block's own or the default).
    pub params: BTreeMap<String, String>,
    pub sub: bool,
    pub color: String,
    /// A masked subsystem's mask (its parameters are in `params`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mask: Option<crate::model::Mask>,
}

#[derive(Serialize)]
pub struct WireView {
    pub id: u32,
    pub from: (u32, usize),
    pub to: (u32, usize),
    pub pts: Vec<(f64, f64)>,
    pub path: Vec<(f64, f64)>,
    pub name: String,
    pub point: String,
    pub phys: bool,
}

#[derive(Serialize)]
pub struct View {
    pub parent: u32,
    pub path: Vec<(u32, String)>,
    pub blocks: Vec<BlockView>,
    pub wires: Vec<WireView>,
}

/// A block found by name or kind anywhere in the model, with the diagram it sits in.
#[derive(Serialize, Debug)]
pub struct Hit {
    pub id: u32,
    pub name: String,
    pub kind: String,
    /// The diagram to open (0 = the top level).
    pub parent: u32,
    /// Where it is, as "Subsystem/Inner" ("" at the top level).
    pub path: String,
}

/// Blocks whose name, kind or kind's library name contains the text (case does not matter).
/// Named signals are found too, by the block their wire leaves from.
pub fn find(m: &Model, q: &str) -> Vec<Hit> {
    let q = q.trim().to_lowercase();
    let mut out = vec![];
    if q.is_empty() {
        return out;
    }
    fn walk(d: &crate::model::Diagram, parent: u32, path: &str, q: &str, out: &mut Vec<Hit>) {
        for b in &d.blocks {
            let lib = blocks::def(&b.kind).map(|d| d.name.to_lowercase()).unwrap_or_default();
            let signal = d.wires.iter().any(|w| w.from.0 == b.id && w.name.to_lowercase().contains(q));
            if b.name.to_lowercase().contains(q) || b.kind.to_lowercase().contains(q) || lib.contains(q) || signal {
                out.push(Hit { id: b.id, name: b.name.clone(), kind: b.kind.clone(), parent, path: path.to_string() });
            }
            if let Some(s) = &b.sub {
                let p = if path.is_empty() { b.name.clone() } else { format!("{path}/{}", b.name) };
                walk(s, b.id, &p, q, out);
            }
        }
    }
    walk(&m.root, 0, "", &q, &mut out);
    out.truncate(200);
    out
}

pub fn view(m: &Model, parent: u32) -> Option<View> {
    let d = m.diagram(parent)?;
    let mut blocks_v = Vec::with_capacity(d.blocks.len());
    for b in &d.blocks {
        let (ins, outs) = blocks::port_xy(b);
        let (in_names, out_names) = blocks::port_names(b);
        let params = match &b.mask {
            Some(mk) => mk.params.iter().map(|p| (p.key.clone(), blocks::param(b, &p.key))).collect(),
            None => blocks::def(&b.kind).map(|def| def.params.iter().map(|p| (p.key.to_string(), blocks::param(b, p.key))).collect()).unwrap_or_default(),
        };
        blocks_v.push(BlockView {
            id: b.id,
            kind: b.kind.clone(),
            name: b.name.clone(),
            x: b.x,
            y: b.y,
            w: b.w,
            h: b.h,
            flip: b.flip,
            label: blocks::label(b),
            ins,
            outs,
            phys: blocks::phys_xy(b),
            phys_dom: crate::physical::ports_of(&b.kind).map(|s| s.iter().map(|p| p.1.name().to_string()).collect()).unwrap_or_default(),
            phys_names: crate::physical::ports_of(&b.kind).map(|s| s.iter().map(|p| p.0.to_string()).collect()).unwrap_or_default(),
            in_names,
            out_names,
            params,
            sub: b.sub.is_some(),
            color: b.color.clone(),
            mask: b.mask.clone(),
        });
    }
    // Wires may run over notes: they are only text.
    let rects: Vec<Rect> = d.blocks.iter().filter(|b| b.kind != "Note").map(|b| Rect { x: b.x, y: b.y, w: b.w, h: b.h }).collect();
    let by_id: BTreeMap<u32, &BlockView> = blocks_v.iter().map(|b| (b.id, b)).collect();
    let mut reqs = Vec::new();
    let mut ok = Vec::new();
    for w in d.wires.iter().filter(|w| !w.phys) {
        let (Some(a), Some(b)) = (by_id.get(&w.from.0), by_id.get(&w.to.0)) else { continue };
        let (Some(pa), Some(pb)) = (a.outs.get(w.from.1), b.ins.get(w.to.1)) else { continue };
        reqs.push(Req {
            a: *pa,
            dir_a: if a.flip { -1.0 } else { 1.0 },
            b: *pb,
            dir_b: if b.flip { -1.0 } else { 1.0 },
            pts: w.pts.clone(),
            group: ((w.from.0 as u64) << 8) | w.from.1 as u64,
        });
        ok.push(w);
    }
    let paths = route::route(&rects, &reqs);
    let mut wires: Vec<WireView> = ok.into_iter().zip(paths).map(|(w, path)| WireView { id: w.id, from: w.from, to: w.to, pts: w.pts.clone(), path, name: w.name.clone(), point: w.point.clone(), phys: false }).collect();
    // Physical connections: straight square paths between their ports (no direction).
    for w in d.wires.iter().filter(|w| w.phys) {
        let (Some(a), Some(b)) = (by_id.get(&w.from.0), by_id.get(&w.to.0)) else { continue };
        let (Some(&pa), Some(&pb)) = (a.phys.get(w.from.1), b.phys.get(w.to.1)) else { continue };
        let mut path = vec![pa];
        if w.pts.is_empty() {
            let bottom = |bv: &BlockView, p: (f64, f64)| (p.1 - (bv.y + bv.h)).abs() < 0.5;
            if bottom(a, pa) || bottom(b, pb) {
                let y = pa.1.max(pb.1) + 14.0;
                path.extend([(pa.0, y), (pb.0, y)]);
            } else {
                let mx = ((pa.0 + pb.0) / 2.0).round();
                path.extend([(mx, pa.1), (mx, pb.1)]);
            }
        } else {
            for p in &w.pts {
                let last = *path.last().unwrap();
                path.extend([(p.0, last.1), *p]);
            }
            let last = *path.last().unwrap();
            path.push((pb.0, last.1));
        }
        path.push(pb);
        path.dedup();
        wires.push(WireView { id: w.id, from: w.from, to: w.to, pts: w.pts.clone(), path, name: String::new(), point: String::new(), phys: true });
    }
    Some(View { parent, path: m.path_of(parent), blocks: blocks_v, wires })
}
