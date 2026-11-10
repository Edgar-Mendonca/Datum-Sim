//! The Control tab (0.2.3 on): linearising the open model between its analysis points and the
//! analyses of the result. The engine (datum_core::linearise, lti) does the work.

use crate::{base_dir, St, R};
use datum_core::linearise::{self, At, TrimSpec, Trimmed};
use datum_core::lti::{self, Pid, Ss};
use serde_json::Value;
use std::collections::BTreeMap;
use tauri::State;

/// The plant a PID sees and its present gains: {plant, cur, name, report}.
#[tauri::command]
pub async fn pid_plant(st: State<'_, St>, id: u32, at: At) -> R<Value> {
    let (m, base) = {
        let a = st.lock();
        (a.doc.m.clone(), base_dir(&a))
    };
    let (plant, cur, name) = linearise::pid_plant(&m, base.as_deref(), id, &at)?;
    let now = lti::margins(&lti::series(&cur.ss(), &plant));
    Ok(serde_json::json!({ "plant": plant, "cur": cur, "name": name, "wgc": if now.wgc.is_finite() { now.wgc } else { 0.0 } }))
}

/// Gains for a rule ("shape", "zn", "simc"), with the before-and-after report.
#[tauri::command]
pub async fn pid_design(plant: Ss, cur: Pid, rule: String, wc: f64, pm: f64, form: String) -> R<Value> {
    let d = match rule.as_str() {
        "zn" => lti::ziegler_nichols(&plant)?,
        "simc" => lti::simc(&plant)?,
        _ => lti::design(&plant, wc, pm, &form)?,
    };
    let mut pid = d.pid;
    if pid.d != 0.0 && pid.n <= 0.0 {
        pid.n = 100.0;
    }
    if pid.d == 0.0 {
        pid.n = cur.n;
    }
    Ok(serde_json::json!({ "pid": pid, "rule": d.rule, "note": d.note, "report": linearise::pid_report(&plant, &cur, &pid) }))
}

/// A motor's data for drive tuning, and the cascade's gains for two bandwidths.
#[tauri::command]
pub async fn drive_tune(st: State<'_, St>, id: u32, wc_i: f64, wc_s: f64, torque_out: bool) -> R<Value> {
    let m = st.lock().doc.m.clone();
    let (d, name) = linearise::drive_data(&m, id)?;
    let t = lti::drive_tune(&d, wc_i, wc_s, torque_out)?;
    Ok(serde_json::json!({ "data": d, "name": name, "tune": t }))
}

/// The analysis-point outputs trim can hold, and the number of states.
#[tauri::command]
pub async fn trim_info(st: State<'_, St>) -> R<(Vec<String>, usize)> {
    let (m, base) = {
        let a = st.lock();
        (a.doc.m.clone(), base_dir(&a))
    };
    linearise::trim_info(&m, base.as_deref())
}

/// Find an operating point (it is saved by the window with SetOps, as one undoable edit).
#[tauri::command]
pub async fn trim(st: State<'_, St>, spec: TrimSpec) -> R<Trimmed> {
    let (m, base) = {
        let a = st.lock();
        (a.doc.m.clone(), base_dir(&a))
    };
    linearise::trim(&m, base.as_deref(), &spec)
}

/// Linearise at an operating point and analyse one input–output pair.
#[tauri::command]
pub async fn lin_analysis(st: State<'_, St>, at: At, inp: usize, outp: usize) -> R<Value> {
    let (m, base) = {
        let a = st.lock();
        (a.doc.m.clone(), base_dir(&a))
    };
    let l = linearise::linearise(&m, base.as_deref(), &BTreeMap::new(), &at)?;
    linearise::report(&l, inp, outp)
}
