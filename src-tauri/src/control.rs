//! The Control tab (0.2.3 on): linearising the open model between its analysis points and the
//! analyses of the result. The engine (datum_core::linearise, lti) does the work.

use crate::{base_dir, St, R};
use datum_core::linearise::{self, At};
use serde_json::Value;
use std::collections::BTreeMap;
use tauri::State;

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
