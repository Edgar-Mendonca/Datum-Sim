//! The command line as a user runs it: the real binary, real files, exit codes.

use std::process::Command;

fn cli(args: &[&str]) -> (i32, String, String) {
    let o = Command::new(env!("CARGO_BIN_EXE_datum-sim-cli")).args(args).output().expect("the binary runs");
    (o.status.code().unwrap_or(-1), String::from_utf8_lossy(&o.stdout).into(), String::from_utf8_lossy(&o.stderr).into())
}

fn tmp(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("datum-sim-cli-test-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d.join(name)
}

/// Save an example, run it with a variable set, and read the CSV: the mass–spring–damper,
/// critically damped, settles at 1 without overshoot.
#[test]
fn run_writes_csv() {
    let model = tmp("msd.dsim");
    let (c, _, e) = cli(&["example", "msd", "--out", model.to_str().unwrap()]);
    assert_eq!(c, 0, "{e}");
    let out = tmp("msd.csv");
    let (c, _, e) = cli(&["run", model.to_str().unwrap(), "--set", "zeta=1", "--stop", "20", "--rtol", "1e-9", "--out", out.to_str().unwrap()]);
    assert_eq!(c, 0, "{e}");
    let text = std::fs::read_to_string(&out).unwrap();
    let rows: Vec<Vec<f64>> = text.lines().skip(1).map(|l| l.split(',').map(|x| x.parse().unwrap()).collect()).collect();
    assert!(text.lines().next().unwrap().starts_with("time"));
    let peak = rows.iter().map(|r| r[1]).fold(0.0, f64::max);
    assert!(peak <= 1.0 + 1e-6 && (rows.last().unwrap()[1] - 1.0).abs() < 1e-3, "peak {peak}");
}

/// Reduced JSON output, a summary to the terminal, and a sweep into a folder.
#[test]
fn json_summary_and_sweep() {
    let out = tmp("dc.json");
    let (c, _, e) = cli(&["run", "example:dcmotor", "--points", "100", "--out", out.to_str().unwrap()]);
    assert_eq!(c, 0, "{e}");
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
    assert!(v["t"].as_array().unwrap().len() <= 100 && !v["signals"].as_array().unwrap().is_empty());
    let (c, so, e) = cli(&["run", "example:dcmotor"]);
    assert_eq!(c, 0, "{e}");
    assert!(so.contains("final") && so.contains("time points"), "{so}");
    let dir = tmp("sweep");
    let (c, so, e) = cli(&["sweep", "example:msd", "--var", "zeta", "--values", "0.2,1", "--out", dir.to_str().unwrap()]);
    assert_eq!(c, 0, "{e}");
    assert!(dir.join("zeta=0.2.csv").exists() && dir.join("zeta=1.csv").exists(), "{so}");
}

/// Exit codes: 2 for a wrong command line, 1 for a model with errors; check and info.
#[test]
fn errors_and_exit_codes() {
    assert_eq!(cli(&["fly"]).0, 2);
    assert_eq!(cli(&["run"]).0, 2);
    assert_eq!(cli(&["run", "example:msd", "--bogus"]).0, 2);
    assert_eq!(cli(&["run", "example:nope"]).0, 2);
    let (c, _, e) = cli(&["run", "example:dcmotor", "--param", "PID:P=Kq"]);
    assert_eq!(c, 1, "an unknown name in a parameter is a model error: {e}");
    assert!(e.contains("Kq"), "{e}");
    let (c, so, _) = cli(&["check", "example:dcmotor"]);
    assert_eq!(c, 0);
    assert!(so.contains("ready to run"));
    let (c, so, _) = cli(&["info", "example:dcmotor"]);
    assert_eq!(c, 0);
    let v: serde_json::Value = serde_json::from_str(&so).unwrap();
    assert!(v["blocks"].as_array().unwrap().iter().any(|b| b["path"] == "PID"));
    let (c, so, _) = cli(&["--version"]);
    assert!(c == 0 && so.starts_with("datum-sim-cli "));
}

/// The JSON session Python uses: requests in, answers with ids out, errors as answers.
#[test]
fn serve_answers_each_line() {
    use std::io::Write;
    let mut p = Command::new(env!("CARGO_BIN_EXE_datum-sim-cli")).arg("serve").stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).spawn().unwrap();
    {
        let i = p.stdin.as_mut().unwrap();
        writeln!(i, r#"{{"id": 1, "cmd": "new", "example": "msd"}}"#).unwrap();
        writeln!(i, r#"{{"id": 2, "cmd": "run", "points": 50}}"#).unwrap();
        writeln!(i, r#"{{"id": 3, "cmd": "fly"}}"#).unwrap();
    }
    drop(p.stdin.take());
    let out = p.wait_with_output().unwrap();
    let lines: Vec<serde_json::Value> = String::from_utf8_lossy(&out.stdout).lines().map(|l| serde_json::from_str(l).unwrap()).collect();
    assert_eq!(lines.len(), 3);
    assert_eq!((lines[0]["id"].as_i64(), lines[0]["ok"].as_bool()), (Some(1), Some(true)));
    assert!(lines[1]["result"]["t"].as_array().unwrap().len() <= 50);
    assert_eq!((lines[2]["id"].as_i64(), lines[2]["ok"].as_bool()), (Some(3), Some(false)));
}
