//! Prints engine output for the DC motor example (and the induction motor start's diagram) as JSON fixtures for the UI tests
//! (cargo run -p datum-core --example dump > tests_ui/fixture.json).
use datum_core::{blocks, compile, examples, view};
use std::collections::BTreeMap;

fn main() {
    let m = examples::get("dcmotor").unwrap();
    let sub = m.root.blocks.iter().find(|b| b.kind == "Subsystem").unwrap().id;
    let sim = compile::compile(&m, None, &BTreeMap::new()).unwrap();
    let chans = sim.chans.clone();
    let r = compile::run(sim, None, None).unwrap();
    let all: Vec<&[f64]> = r.y.iter().map(|y| y.as_slice()).collect();
    let (t, y) = compile::decimate_multi(&r.t, &all, r.t[0], *r.t.last().unwrap(), 400);
    println!("{}", serde_json::json!({
        "state": { "name": m.name, "notes": m.notes, "vars": m.vars, "sim": m.sim, "path": null, "dirty": false, "undo": false, "redo": false, "clip": false, "running": false, "rev": 1, "created": [] },
        "library": { "cats": blocks::CATS, "blocks": blocks::library() },
        "examples_list": examples::list(),
        "diagram": view::view(&m, 0).unwrap(),
        "subdiagram": view::view(&m, sub).unwrap(),
        "motor": view::view(&examples::get("imdol").unwrap(), 0).unwrap(),
        "started": { "gen": 1, "chans": chans, "warnings": [], "start": m.sim.start, "stop": m.sim.stop },
        "rates": compile::rate_view(&m, &compile::compile(&m, None, &BTreeMap::new()).unwrap()),
        "results": { "chans": r.chans, "stats": r.stats, "n": r.t.len(), "t0": r.t[0], "t1": r.t.last(), "last": r.y.iter().map(|y| y.last().cloned().unwrap_or(0.0)).collect::<Vec<_>>(), "sweep": null },
        "series": { "t": t, "y": y },
        "sub": sub,
    }));
}
