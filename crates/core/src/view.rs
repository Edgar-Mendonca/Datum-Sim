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
    pub in_names: Vec<String>,
    pub out_names: Vec<String>,
    /// Every parameter's text (the block's own or the default).
    pub params: BTreeMap<String, String>,
    pub sub: bool,
}

#[derive(Serialize)]
pub struct WireView {
    pub id: u32,
    pub from: (u32, usize),
    pub to: (u32, usize),
    pub pts: Vec<(f64, f64)>,
    pub path: Vec<(f64, f64)>,
}

#[derive(Serialize)]
pub struct View {
    pub parent: u32,
    pub path: Vec<(u32, String)>,
    pub blocks: Vec<BlockView>,
    pub wires: Vec<WireView>,
}

pub fn view(m: &Model, parent: u32) -> Option<View> {
    let d = m.diagram(parent)?;
    let mut blocks_v = Vec::with_capacity(d.blocks.len());
    for b in &d.blocks {
        let (ins, outs) = blocks::port_xy(b);
        let (in_names, out_names) = blocks::port_names(b);
        let params = blocks::def(&b.kind)
            .map(|def| def.params.iter().map(|p| (p.key.to_string(), blocks::param(b, p.key))).collect())
            .unwrap_or_default();
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
            in_names,
            out_names,
            params,
            sub: b.sub.is_some(),
        });
    }
    let rects: Vec<Rect> = d.blocks.iter().map(|b| Rect { x: b.x, y: b.y, w: b.w, h: b.h }).collect();
    let by_id: BTreeMap<u32, &BlockView> = blocks_v.iter().map(|b| (b.id, b)).collect();
    let mut reqs = Vec::new();
    let mut ok = Vec::new();
    for w in &d.wires {
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
    let wires = ok.into_iter().zip(paths).map(|(w, path)| WireView { id: w.id, from: w.from, to: w.to, pts: w.pts.clone(), path }).collect();
    Some(View { parent, path: m.path_of(parent), blocks: blocks_v, wires })
}
