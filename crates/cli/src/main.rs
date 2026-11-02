//! datum-sim-cli: Datum Sim without the window. Run, check and sweep models from scripts, batch
//! files and CI; `serve` speaks JSON lines for the Python API (py/datum_sim/api.py).
//! Exit codes: 0 done, 1 the model has errors, 2 the command line is wrong.

use datum_core::api::{self, Session};
use datum_core::compile;
use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::path::Path;
use std::process::ExitCode;

const HELP: &str = "Datum Sim from the command line

USAGE
  datum-sim-cli run MODEL [options]       run a model; write results or print a summary
  datum-sim-cli sweep MODEL --var NAME --values 1,2,3 [options]
                                          run once per value of a model variable
  datum-sim-cli check MODEL               report errors and warnings (exit 1 on errors)
  datum-sim-cli info MODEL                blocks, parameters, variables, signals (JSON)
  datum-sim-cli examples                  list the worked examples
  datum-sim-cli example ID --out FILE     save a worked example as a .dsim file
  datum-sim-cli serve                     JSON requests on standard input, one per line
  datum-sim-cli --version

MODEL is a .dsim file, or example:ID for a worked example.

OPTIONS
  --set NAME=VALUE          set a model variable (repeat for more)
  --param BLOCK:KEY=VALUE   set a block parameter; BLOCK is its name or path (Motor/Gain)
  --start T  --stop T  --solver rk45|stiff|rk4|heun|euler  --step H  --rtol X  --atol X
  --all                     log every signal (not only those into scopes)
  --signals A,B             the signals to write (names as in the scope legends)
  --points N                reduce the results to about N time points (peaks kept)
  --out FILE                .csv (time first) or .json; for sweep, a folder
  --save FILE               save the model with the changes made by --set and --param

EXAMPLES
  datum-sim-cli run motor.dsim --set Kp=120 --stop 5 --out speed.csv
  datum-sim-cli sweep example:msd --var zeta --values 0.1,0.5,1 --out runs
";

struct Args {
    pos: Vec<String>,
    set: Vec<(String, String)>,
    param: Vec<(String, String, String)>,
    sim: serde_json::Map<String, Value>,
    signals: Vec<String>,
    points: usize,
    out: Option<String>,
    save: Option<String>,
    var: Option<String>,
    values: Vec<f64>,
}

fn parse(argv: &[String]) -> Result<Args, String> {
    let mut a = Args { pos: vec![], set: vec![], param: vec![], sim: Default::default(), signals: vec![], points: 0, out: None, save: None, var: None, values: vec![] };
    let mut i = 0;
    let val = |i: &mut usize, flag: &str| -> Result<String, String> {
        *i += 1;
        argv.get(*i).cloned().ok_or_else(|| format!("{flag} needs a value"))
    };
    while i < argv.len() {
        let f = argv[i].as_str();
        match f {
            "--set" => {
                let v = val(&mut i, f)?;
                let (k, x) = v.split_once('=').ok_or("--set takes NAME=VALUE")?;
                a.set.push((k.trim().into(), x.trim().into()));
            }
            "--param" => {
                let v = val(&mut i, f)?;
                let (bk, x) = v.split_once('=').ok_or("--param takes BLOCK:KEY=VALUE")?;
                let (b, k) = bk.rsplit_once(':').ok_or("--param takes BLOCK:KEY=VALUE")?;
                a.param.push((b.trim().into(), k.trim().into(), x.trim().into()));
            }
            "--start" | "--stop" | "--step" | "--rtol" | "--atol" => {
                let v = val(&mut i, f)?;
                let x: f64 = v.parse().map_err(|_| format!("{f} needs a number, not '{v}'"))?;
                a.sim.insert(f[2..].into(), json!(x));
            }
            "--solver" => {
                let v = val(&mut i, f)?;
                a.sim.insert("solver".into(), json!(v));
            }
            "--all" => {
                a.sim.insert("log_all".into(), json!(true));
            }
            "--signals" => a.signals = val(&mut i, f)?.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect(),
            "--points" => a.points = val(&mut i, f)?.parse().map_err(|_| "--points needs a whole number")?,
            "--out" => a.out = Some(val(&mut i, f)?),
            "--save" => a.save = Some(val(&mut i, f)?),
            "--var" => a.var = Some(val(&mut i, f)?),
            "--values" => {
                a.values = val(&mut i, f)?.split(',').map(|s| s.trim().parse::<f64>().map_err(|_| format!("--values: '{s}' is not a number"))).collect::<Result<_, _>>()?;
            }
            "--version" | "--help" | "-h" => a.pos.push(f.to_string()),
            _ if f.starts_with("--") => return Err(format!("Unknown option {f}. Try datum-sim-cli help.")),
            _ => a.pos.push(f.to_string()),
        }
        i += 1;
    }
    Ok(a)
}

/// Open the model and apply --set, --param and the run settings.
fn session(a: &Args) -> Result<Session, (u8, String)> {
    let model = a.pos.get(1).ok_or((2, "Give a model: a .dsim file or example:ID".to_string()))?;
    let mut s = Session::new();
    if let Some(id) = model.strip_prefix("example:") {
        s.handle(&json!({ "cmd": "new", "example": id })).map_err(|e| (2, e))?;
    } else {
        s.open(Path::new(model)).map_err(|e| (2, e))?;
    }
    for (k, v) in &a.set {
        s.set_var(k, v).map_err(|e| (2, e))?;
    }
    for (b, k, v) in &a.param {
        s.set_param(b, k, v).map_err(|e| (2, format!("--param {b}:{k}: {e}")))?;
    }
    if !a.sim.is_empty() {
        s.set_sim(&Value::Object(a.sim.clone())).map_err(|e| (2, e))?;
    }
    if let Some(p) = &a.save {
        s.save(Path::new(p)).map_err(|e| (2, e))?;
    }
    Ok(s)
}

fn write_results(r: &compile::Results, a: &Args, out: &Path) -> Result<(), String> {
    let json_out = out.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("json"));
    let text = if json_out {
        api::results_json(r, &a.signals, a.points)?.to_string()
    } else {
        let cols = api::pick(r, &a.signals)?;
        if a.points > 0 {
            // CSV of the reduced results.
            let v = api::results_json(r, &a.signals, a.points)?;
            let t = v["t"].as_array().cloned().unwrap_or_default();
            let sig = v["signals"].as_array().cloned().unwrap_or_default();
            let mut s = String::from("time");
            for x in &sig {
                s.push(',');
                s.push_str(&csv_field(x["name"].as_str().unwrap_or("")));
            }
            s.push('\n');
            for (i, tt) in t.iter().enumerate() {
                s.push_str(&tt.to_string());
                for x in &sig {
                    s.push(',');
                    s.push_str(&x["y"][i].to_string());
                }
                s.push('\n');
            }
            s
        } else {
            compile::csv(r, &cols)
        }
    };
    std::fs::write(out, text).map_err(|e| format!("Could not write {}: {e}", out.display()))
}

fn csv_field(s: &str) -> String {
    if s.contains([',', '"', '\n']) { format!("\"{}\"", s.replace('"', "\"\"")) } else { s.to_string() }
}

fn summary(r: &compile::Results, a: &Args) -> Result<(), String> {
    let cols = api::pick(r, &a.signals)?;
    println!("{} time points, {} steps, {} events, {:.0} ms", r.t.len(), r.stats.steps, r.stats.events, r.stats.ms);
    for c in cols {
        let y = &r.y[c];
        let (lo, hi) = y.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), v| (a.min(*v), b.max(*v)));
        println!("  {:<40} final {:<14.6} min {:<14.6} max {:.6}", r.chans[c].name, y.last().cloned().unwrap_or(f64::NAN), lo, hi);
    }
    Ok(())
}

fn real_main() -> Result<(), (u8, String)> {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let a = parse(&argv).map_err(|e| (2, e))?;
    match a.pos.first().map(String::as_str) {
        None | Some("help") | Some("-h") | Some("--help") => {
            print!("{HELP}");
            Ok(())
        }
        Some("--version") | Some("version") => {
            println!("datum-sim-cli {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Some("examples") => {
            for e in datum_core::examples::list() {
                println!("{:<12} {}", e.id, e.name);
            }
            Ok(())
        }
        Some("example") => {
            let id = a.pos.get(1).ok_or((2, "Give an example id (see: datum-sim-cli examples)".to_string()))?;
            let out = a.out.as_ref().ok_or((2, "example needs --out FILE.dsim".to_string()))?;
            let mut s = Session::new();
            s.handle(&json!({ "cmd": "new", "example": id })).map_err(|e| (2, e))?;
            s.save(Path::new(out)).map_err(|e| (1, e))
        }
        Some("check") => {
            let mut s = session(&a)?;
            let v = s.handle(&json!({ "cmd": "check" })).map_err(|e| (1, e))?;
            for i in v["issues"].as_array().into_iter().flatten() {
                println!("{} {}", if i["warn"].as_bool() == Some(true) { "warning:" } else { "error:  " }, i["msg"].as_str().unwrap_or(""));
            }
            if v["ok"].as_bool() == Some(true) {
                println!("The model is ready to run.");
                Ok(())
            } else {
                Err((1, "The model has errors.".into()))
            }
        }
        Some("info") => {
            let mut s = session(&a)?;
            let v = s.handle(&json!({ "cmd": "info" })).map_err(|e| (1, e))?;
            println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
            Ok(())
        }
        Some("run") => {
            let s = session(&a)?;
            let r = s.run(&Default::default()).map_err(|e| (1, e))?;
            match &a.out {
                Some(o) => {
                    write_results(&r, &a, Path::new(o)).map_err(|e| (1, e))?;
                    eprintln!("Wrote {o} ({} time points)", r.t.len());
                    Ok(())
                }
                None => summary(&r, &a).map_err(|e| (1, e)),
            }
        }
        Some("sweep") => {
            let s = session(&a)?;
            let var = a.var.clone().ok_or((2, "sweep needs --var NAME".to_string()))?;
            if a.values.is_empty() {
                return Err((2, "sweep needs --values 1,2,3".into()));
            }
            let runs = datum_core::sweep::sweep(&s.doc.m, s.base().as_deref(), &var, &a.values).map_err(|e| (1, e.iter().map(|i| i.msg.clone()).collect::<Vec<_>>().join("\n")))?;
            for (v, r) in a.values.iter().zip(&runs) {
                println!("{var} = {v}");
                match &a.out {
                    Some(dir) => {
                        std::fs::create_dir_all(dir).map_err(|e| (1, format!("Could not make {dir}: {e}")))?;
                        let f = Path::new(dir).join(format!("{var}={v}.csv"));
                        write_results(r, &a, &f).map_err(|e| (1, e))?;
                        println!("  wrote {}", f.display());
                    }
                    None => summary(r, &a).map_err(|e| (1, e))?,
                }
            }
            Ok(())
        }
        Some("serve") => {
            let mut s = Session::new();
            let stdin = std::io::stdin();
            let mut out = std::io::stdout().lock();
            for line in stdin.lock().lines() {
                let Ok(line) = line else { break };
                if line.trim().is_empty() {
                    continue;
                }
                let _ = writeln!(out, "{}", s.answer(&line));
                let _ = out.flush();
            }
            Ok(())
        }
        Some(c) => Err((2, format!("Unknown command '{c}'. Try datum-sim-cli help."))),
    }
}

fn main() -> ExitCode {
    match real_main() {
        Ok(()) => ExitCode::SUCCESS,
        Err((code, msg)) => {
            eprintln!("{msg}");
            ExitCode::from(code)
        }
    }
}
