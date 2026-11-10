//! The Control tab (0.2.3 on): linearising the open model between its analysis points and the
//! analyses of the result. The engine (datum_core::linearise, lti) does the work.

use crate::{base_dir, St, R};
use datum_core::linearise::{self, At, TrimSpec, Trimmed};
use serde_json::Value;
use std::collections::BTreeMap;
use tauri::State;

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
