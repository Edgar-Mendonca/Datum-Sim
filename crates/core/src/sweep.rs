//! Parameter sweeps: run the model once per value of a variable, in parallel.

use crate::compile::{simulate, Issue, Results};
use crate::model::Model;
use std::collections::BTreeMap;
use std::path::Path;

/// Values from a text: "1 2 5", "[1, 2, 5]" or "linspace(0.1, 1, 5)" (any expression).
pub fn values(text: &str) -> Result<Vec<f64>, String> {
    let m = crate::expr::eval(text, &|_| None)?;
    if m.is_empty() || m.len() > 1000 {
        return Err("Give between 1 and 1000 values.".into());
    }
    if m.d.iter().any(|v| !v.is_finite()) {
        return Err("Every value must be a finite number.".into());
    }
    Ok(m.d)
}

pub fn sweep(m: &Model, base: Option<&Path>, var: &str, vals: &[f64]) -> Result<Vec<Results>, Vec<Issue>> {
    if !m.vars.iter().any(|v| v.name == var) {
        return Err(vec![Issue { id: 0, msg: format!("There is no model variable called '{var}'."), warn: false }]);
    }
    let threads = std::thread::available_parallelism().map_or(2, |n| n.get()).clamp(1, 8);
    let chunks: Vec<Vec<(usize, f64)>> = (0..threads).map(|k| vals.iter().cloned().enumerate().skip(k).step_by(threads).collect()).collect();
    let mut out: Vec<Option<Result<Results, Vec<Issue>>>> = (0..vals.len()).map(|_| None).collect();
    std::thread::scope(|s| {
        let hs: Vec<_> = chunks.into_iter().map(|ch| {
            s.spawn(move || {
                ch.into_iter().map(|(i, v)| {
                    let mut o = BTreeMap::new();
                    o.insert(var.to_string(), v);
                    (i, simulate(m, base, &o))
                }).collect::<Vec<_>>()
            })
        }).collect();
        for h in hs {
            for (i, r) in h.join().unwrap_or_default() {
                out[i] = Some(r);
            }
        }
    });
    out.into_iter().map(|r| r.unwrap_or_else(|| Err(vec![]))).collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn sweep_damping() {
        let m = crate::examples::get("msd").unwrap();
        let vals = super::values("[0.1 0.5 1]").unwrap();
        let rs = super::sweep(&m, None, "zeta", &vals).unwrap();
        let peaks: Vec<f64> = rs.iter().map(|r| r.y[0].iter().cloned().fold(0.0, f64::max)).collect();
        assert!(peaks[0] > peaks[1] && peaks[1] > peaks[2], "{peaks:?}");
        assert!(super::values("linspace(0, 1, 5)").unwrap().len() == 5);
        assert!(super::sweep(&m, None, "nope", &vals).is_err());
    }
}
