//! Datum Sim desktop shell: owns the open model, files, options and recovery, runs simulations on
//! a worker thread (streaming to the scopes), and talks to the window.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod py;
mod runtime;
mod update;

use datum_core::compile::{self, Control, Issue, Results};
use datum_core::doc::{Doc, Op};
use datum_core::model::{self, Model, SimSettings, Var};
use datum_core::{blocks, examples, sweep, view};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, State, WindowEvent};
use tauri_plugin_dialog::DialogExt;

#[derive(Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct Options {
    /// Minutes between recovery saves; 0 turns them off.
    autosave: u32,
    beta: bool,
    /// auto, light or dark.
    theme: String,
    open_last: bool,
    /// Defaults for new models.
    solver: String,
    stop: f64,
    /// Snap blocks to the grid while dragging.
    snap: bool,
    grid: bool,
    paper: String,
    landscape: bool,
    company: String,
}

impl Default for Options {
    fn default() -> Self {
        Self { autosave: 5, beta: false, theme: "auto".into(), open_last: false, solver: "rk45".into(), stop: 10.0, snap: true, grid: true, paper: "A4".into(), landscape: true, company: String::new() }
    }
}

/// The last run, shared with the run thread.
#[derive(Default)]
struct RunState {
    ctl: Arc<Control>,
    running: bool,
    results: Option<Arc<Results>>,
    sweep: Option<(String, Vec<f64>, Vec<Arc<Results>>)>,
    /// Bumped when a run starts, so a late finish of an older run is ignored.
    gen: u64,
}

struct App {
    doc: Doc,
    opts: Options,
    clip: String,
    run: RunState,
}

struct St(Mutex<App>);

impl St {
    /// A command that panicked must not lock every later one out, so a poisoned lock is reused.
    fn lock(&self) -> std::sync::MutexGuard<'_, App> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }
}
type R<T> = Result<T, String>;

/// What the window needs after every change.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Out {
    name: String,
    notes: String,
    vars: Vec<Var>,
    sim: SimSettings,
    path: Option<String>,
    dirty: bool,
    undo: bool,
    redo: bool,
    clip: bool,
    running: bool,
    rev: u64,
    created: Vec<u32>,
}

fn out(a: &App) -> Out {
    let m = &a.doc.m;
    Out {
        name: m.name.clone(),
        notes: m.notes.clone(),
        vars: m.vars.clone(),
        sim: m.sim.clone(),
        path: a.doc.path.clone(),
        dirty: a.doc.dirty,
        undo: a.doc.can_undo(),
        redo: a.doc.can_redo(),
        clip: !a.clip.is_empty(),
        running: a.run.running,
        rev: m.next as u64 * 7919 + a.doc.can_undo() as u64,
        created: vec![],
    }
}

fn dir(app: &AppHandle, data: bool) -> Option<PathBuf> {
    let d = if data { app.path().app_data_dir() } else { app.path().app_config_dir() }.ok()?;
    std::fs::create_dir_all(&d).ok()?;
    Some(d)
}

fn base_dir(a: &App) -> Option<PathBuf> {
    a.doc.path.as_ref().and_then(|p| Path::new(p).parent().map(Path::to_path_buf))
}

fn fresh(a: &App, mut m: Model) -> Model {
    if m.root.blocks.is_empty() {
        m.sim.solver = a.opts.solver.clone();
        if a.opts.stop > 0.0 {
            m.sim.stop = a.opts.stop;
        }
    }
    m
}

// ---------------------------------------------------------------- the model

#[tauri::command]
fn state(st: State<St>) -> Out {
    out(&st.lock())
}

#[derive(Serialize)]
struct Library {
    cats: &'static [&'static str],
    blocks: Vec<blocks::Def>,
}

#[tauri::command]
fn library() -> Library {
    Library { cats: blocks::CATS, blocks: blocks::library() }
}

#[tauri::command]
fn diagram(st: State<St>, parent: u32) -> R<view::View> {
    view::view(&st.lock().doc.m, parent).ok_or_else(|| "That diagram no longer exists.".into())
}

#[tauri::command]
fn apply(st: State<St>, op: Op) -> R<Out> {
    let mut a = st.lock();
    let created = a.doc.apply(op)?;
    let mut o = out(&a);
    o.created = created;
    Ok(o)
}

#[tauri::command]
fn history(st: State<St>, redo: bool) -> Out {
    let mut a = st.lock();
    if redo {
        a.doc.redo();
    } else {
        a.doc.undo();
    }
    out(&a)
}

#[tauri::command]
fn copy(st: State<St>, ids: Vec<u32>) -> bool {
    let mut a = st.lock();
    let c = a.doc.copy(&ids);
    let ok = !c.is_empty();
    if ok {
        a.clip = c;
    }
    ok
}

#[tauri::command]
fn paste(st: State<St>, parent: u32, dx: f64, dy: f64) -> R<Out> {
    let mut a = st.lock();
    let clip = a.clip.clone();
    if clip.is_empty() {
        return Err("Nothing to paste.".into());
    }
    let created = a.doc.apply(Op::Paste { parent, clip, dx, dy })?;
    let mut o = out(&a);
    o.created = created;
    Ok(o)
}

#[tauri::command]
fn examples_list() -> Vec<examples::Example> {
    examples::list()
}

#[tauri::command]
fn new_model(st: State<St>, example: Option<String>) -> R<Out> {
    let mut a = st.lock();
    if a.run.running {
        return Err("Stop the running simulation first.".into());
    }
    // Model::new (named, ids from 1), not Model::default.
    let m = match example.as_deref().and_then(examples::get) {
        Some(m) => m,
        None => Model::new(),
    };
    let m = fresh(&a, m);
    a.doc = Doc::new(m);
    a.run.results = None;
    a.run.sweep = None;
    Ok(out(&a))
}

/// Check the model without running it: errors and warnings, each pointing at its block.
#[tauri::command]
fn check(st: State<St>) -> Vec<Issue> {
    let a = st.lock();
    match compile::compile(&a.doc.m, base_dir(&a).as_deref(), &BTreeMap::new()) {
        Ok(s) => s.warnings,
        Err(e) => e,
    }
}

/// Each block's sample-time rate, for the sample-time colours (empty while the model has errors).
#[tauri::command]
fn rates(st: State<St>) -> Vec<compile::RateView> {
    let a = st.lock();
    compile::compile(&a.doc.m, base_dir(&a).as_deref(), &BTreeMap::new()).map(|s| compile::rate_view(&a.doc.m, &s)).unwrap_or_default()
}

// ---------------------------------------------------------------- files

fn pick(app: &AppHandle, name: &str, exts: &[&str]) -> Option<PathBuf> {
    app.dialog().file().add_filter(name, exts).blocking_pick_file().and_then(|f| f.into_path().ok())
}

#[tauri::command]
async fn open(app: AppHandle, st: State<'_, St>, path: Option<String>) -> R<Option<Out>> {
    let Some(path) = path.map(PathBuf::from).or_else(|| pick(&app, "Datum Sim model", &["dsim"])) else { return Ok(None) };
    let text = std::fs::read_to_string(&path).map_err(|e| format!("Could not open {}: {e}", path.display()))?;
    let m = model::from_json(&text)?;
    remember(&app, &path);
    let mut a = st.lock();
    if a.run.running {
        return Err("Stop the running simulation first.".into());
    }
    a.doc = Doc::new(m);
    a.doc.path = Some(path.display().to_string());
    a.run.results = None;
    a.run.sweep = None;
    Ok(Some(out(&a)))
}

#[tauri::command]
async fn save(app: AppHandle, st: State<'_, St>, as_new: bool) -> R<Option<Out>> {
    let (known, name) = {
        let a = st.lock();
        (a.doc.path.clone().filter(|_| !as_new), a.doc.m.name.clone())
    };
    let path = match known {
        Some(p) => PathBuf::from(p),
        None => match app.dialog().file().add_filter("Datum Sim model", &["dsim"]).set_file_name(format!("{name}.dsim")).blocking_save_file() {
            Some(f) => f.into_path().map_err(|e| e.to_string())?,
            None => return Ok(None),
        },
    };
    let mut a = st.lock();
    std::fs::write(&path, model::to_json(&a.doc.m)).map_err(|e| format!("Could not save {}: {e}", path.display()))?;
    a.doc.path = Some(path.display().to_string());
    a.doc.dirty = false;
    remember(&app, &path);
    clear_recovery(&app);
    Ok(Some(out(&a)))
}

/// Saves a picture or a file the window made (SVG text, PNG data URL, CSV text).
#[tauri::command]
async fn save_file(app: AppHandle, name: String, data: String) -> R<Option<String>> {
    let ext = name.rsplit('.').next().unwrap_or("txt").to_string();
    let Some(path) = app.dialog().file().add_filter(ext.to_uppercase(), &[&ext]).set_file_name(&name).blocking_save_file().and_then(|f| f.into_path().ok()) else { return Ok(None) };
    let bytes = match data.strip_prefix("data:").and_then(|d| d.split_once(";base64,")) {
        Some((_, b64)) => base64_decode(b64)?,
        None => data.into_bytes(),
    };
    std::fs::write(&path, bytes).map_err(|e| format!("Could not save {}: {e}", path.display()))?;
    Ok(Some(path.display().to_string()))
}

fn base64_decode(s: &str) -> R<Vec<u8>> {
    let val = |c: u8| match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    };
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let (mut acc, mut bits) = (0u32, 0);
    for c in s.bytes().filter(|c| !c.is_ascii_whitespace() && *c != b'=') {
        acc = acc << 6 | val(c).ok_or("The picture data is damaged.")? as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------- running

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Live {
    gen: u64,
    t: Vec<f64>,
    y: Vec<Vec<f64>>,
    progress: f64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Done {
    gen: u64,
    ok: bool,
    error: Option<String>,
    issues: Vec<Issue>,
    stats: Option<compile::RunStats>,
    truncated: bool,
    files: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Started {
    gen: u64,
    chans: Vec<compile::Chan>,
    warnings: Vec<Issue>,
    start: f64,
    stop: f64,
}

/// Compile and start a run on a worker thread. Samples stream to the window as `sim-live`
/// events (about 20 per second); `sim-done` ends it.
#[tauri::command]
fn run(app: AppHandle, st: State<St>) -> Result<Started, Vec<Issue>> {
    let mut a = st.lock();
    if a.run.running {
        return Err(vec![Issue { id: 0, msg: "A simulation is already running.".into(), warn: false }]);
    }
    let base = base_dir(&a);
    let sim = compile::compile(&a.doc.m, base.as_deref(), &BTreeMap::new())?;
    a.run.gen += 1;
    let gen = a.run.gen;
    let ctl = Arc::new(Control::default());
    a.run.ctl = ctl.clone();
    a.run.running = true;
    a.run.results = None;
    let started = Started { gen, chans: sim.chans.clone(), warnings: sim.warnings.clone(), start: sim.settings.start, stop: sim.settings.stop };
    let (t0, t1) = (sim.settings.start, sim.settings.stop);
    let files = sim.files.clone();
    std::thread::spawn(move || {
        let mut sent = 0usize;
        let mut last = Instant::now();
        let app2 = app.clone();
        let mut emit = move |r: &Results| {
            if last.elapsed() < Duration::from_millis(50) {
                return;
            }
            last = Instant::now();
            let n = r.t.len();
            let stride = ((n - sent) / 4000).max(1);
            let idx: Vec<usize> = (sent..n).step_by(stride).collect();
            let live = Live {
                gen,
                t: idx.iter().map(|i| r.t[*i]).collect(),
                y: r.y.iter().map(|ys| idx.iter().map(|i| ys[*i]).collect()).collect(),
                progress: ((r.t.last().cloned().unwrap_or(t0) - t0) / (t1 - t0)).clamp(0.0, 1.0),
            };
            sent = n;
            let _ = app2.emit("sim-live", live);
        };
        let res = compile::run(sim, Some(&ctl), Some(&mut emit));
        let st = app.state::<St>();
        let mut a = st.lock();
        if a.run.gen != gen {
            return;
        }
        a.run.running = false;
        let done = match res {
            Ok(r) => {
                let mut written = vec![];
                let mut issues = vec![];
                for (id, file, cols) in &files {
                    let path = match &base {
                        Some(b) if Path::new(file).is_relative() => b.join(file),
                        _ => PathBuf::from(file),
                    };
                    match std::fs::write(&path, compile::csv(&r, cols)) {
                        Ok(_) => written.push(path.display().to_string()),
                        Err(e) => issues.push(Issue { id: *id, msg: format!("Could not write {}: {e}", path.display()), warn: true }),
                    }
                }
                let d = Done { gen, ok: true, error: None, issues, stats: Some(r.stats.clone()), truncated: r.truncated, files: written };
                a.run.results = Some(Arc::new(r));
                d
            }
            Err(e) => Done { gen, ok: false, error: Some(e), issues: vec![], stats: None, truncated: false, files: vec![] },
        };
        drop(a);
        let _ = app.emit("sim-done", done);
    });
    Ok(started)
}

/// Stop, pause (true), resume (false) or take one step while paused.
#[tauri::command]
fn control(st: State<St>, what: String) {
    let a = st.lock();
    let c = &a.run.ctl;
    match what.as_str() {
        "stop" => c.stop.store(true, Ordering::Relaxed),
        "pause" => c.pause.store(1, Ordering::Relaxed),
        "resume" => c.pause.store(0, Ordering::Relaxed),
        "step" => c.pause.store(2, Ordering::Relaxed),
        _ => {}
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Series {
    t: Vec<f64>,
    y: Vec<f64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ResultsInfo {
    chans: Vec<compile::Chan>,
    stats: compile::RunStats,
    n: usize,
    t0: f64,
    t1: f64,
    last: Vec<f64>,
    sweep: Option<(String, Vec<f64>)>,
}

#[tauri::command]
fn results(st: State<St>) -> Option<ResultsInfo> {
    let a = st.lock();
    let r = a.run.results.as_ref()?;
    Some(ResultsInfo {
        chans: r.chans.clone(),
        stats: r.stats.clone(),
        n: r.t.len(),
        t0: r.t.first().cloned().unwrap_or(0.0),
        t1: r.t.last().cloned().unwrap_or(0.0),
        last: r.y.iter().map(|y| y.last().cloned().unwrap_or(f64::NAN)).collect(),
        sweep: a.run.sweep.as_ref().map(|s| (s.0.clone(), s.1.clone())),
    })
}

/// Channels of the last run reduced for drawing (peaks kept) on one shared time axis.
#[tauri::command]
fn series(st: State<St>, chans: Vec<usize>, t0: f64, t1: f64, buckets: usize) -> Multi {
    let a = st.lock();
    let Some(r) = a.run.results.clone() else { return Multi { t: vec![], y: vec![] } };
    drop(a);
    let ys: Vec<&[f64]> = chans.iter().filter(|c| **c < r.y.len()).map(|c| r.y[*c].as_slice()).collect();
    let (t, y) = compile::decimate_multi(&r.t, &ys, t0, t1, buckets.clamp(10, 5000));
    Multi { t, y }
}

#[derive(Serialize)]
struct Multi {
    t: Vec<f64>,
    y: Vec<Vec<f64>>,
}

/// One channel from every run of the last sweep.
#[tauri::command]
fn sweep_series(st: State<St>, chan: usize, buckets: usize) -> Vec<Series> {
    let a = st.lock();
    let Some((_, _, runs)) = a.run.sweep.clone() else { return vec![] };
    drop(a);
    runs.iter().filter(|r| chan < r.y.len()).map(|r| {
        let (t0, t1) = (r.t.first().cloned().unwrap_or(0.0), r.t.last().cloned().unwrap_or(0.0));
        let (t, y) = compile::decimate(&r.t, &r.y[chan], t0, t1, buckets.clamp(10, 5000));
        Series { t, y }
    }).collect()
}

#[tauri::command]
async fn run_sweep(st: State<'_, St>, var: String, values: String) -> R<usize> {
    let (m, base) = {
        let a = st.lock();
        if a.run.running {
            return Err("Stop the running simulation first.".into());
        }
        (a.doc.m.clone(), base_dir(&a))
    };
    let vals = sweep::values(&values)?;
    let runs = sweep::sweep(&m, base.as_deref(), &var, &vals).map_err(|e| e.into_iter().map(|i| i.msg).collect::<Vec<_>>().join("\n"))?;
    let n = runs.len();
    let mut a = st.lock();
    a.run.results = runs.first().cloned().map(Arc::new);
    a.run.sweep = Some((var, vals, runs.into_iter().map(Arc::new).collect()));
    Ok(n)
}

/// Write the last results as CSV: every channel, or one sink's.
#[tauri::command]
async fn export_csv(app: AppHandle, st: State<'_, St>, sink: Option<u32>) -> R<Option<String>> {
    let (r, name) = {
        let a = st.lock();
        (a.run.results.clone().ok_or("Run the model first.")?, a.doc.m.name.clone())
    };
    let cols: Vec<usize> = (0..r.chans.len()).filter(|i| sink.is_none_or(|s| r.chans[*i].sink == s)).collect();
    let text = compile::csv(&r, &cols);
    save_file(app, format!("{name} results.csv"), text).await
}

// ---------------------------------------------------------------- analysis (Python)

/// The linear system of a block (TF, ZPK, state space, PID, gain, integrator) for analysis.
#[tauri::command]
async fn analyze(app: AppHandle, st: State<'_, St>, id: u32, what: String) -> R<Value> {
    let lin = {
        let a = st.lock();
        compile::linear(&a.doc.m, id)?
    };
    let q = json!({ "cmd": "linear", "what": what, "A": lin.a, "B": lin.b, "C": lin.c, "D": lin.d, "name": lin.name });
    let eng = app.state::<py::Engine>();
    let mut p = eng.0.lock().unwrap_or_else(|e| e.into_inner());
    p.ask(&app, &q, Duration::from_secs(120))
}

/// Step-response specifications of a logged signal (rise time, overshoot, settling time…).
#[tauri::command]
async fn step_info(app: AppHandle, st: State<'_, St>, chan: usize) -> R<Value> {
    let r = st.lock().run.results.clone().ok_or("Run the model first.")?;
    if chan >= r.y.len() {
        return Err("That signal is not in the results.".into());
    }
    let (t, y) = compile::decimate(&r.t, &r.y[chan], r.t[0], *r.t.last().unwrap(), 20_000);
    let q = json!({ "cmd": "stepinfo", "t": t, "y": y, "name": r.chans[chan].name });
    let eng = app.state::<py::Engine>();
    let mut p = eng.0.lock().unwrap_or_else(|e| e.into_inner());
    p.ask(&app, &q, Duration::from_secs(60))
}

#[tauri::command]
async fn engine(app: AppHandle, restart: bool) -> R<Value> {
    let eng = app.state::<py::Engine>();
    let mut p = eng.0.lock().unwrap_or_else(|e| e.into_inner());
    if restart {
        return p.restart(&app);
    }
    p.ask(&app, &json!({ "cmd": "ping" }), Duration::from_secs(60))
}

// ---------------------------------------------------------------- options, recovery, recent

fn options_file(app: &AppHandle) -> Option<PathBuf> {
    dir(app, false).map(|d| d.join("options.json"))
}

#[tauri::command]
fn options(st: State<St>) -> Options {
    st.lock().opts.clone()
}

#[tauri::command]
fn set_options(app: AppHandle, st: State<St>, opts: Options) -> Out {
    if let Some(f) = options_file(&app) {
        let _ = std::fs::write(f, serde_json::to_string(&opts).unwrap_or_default());
    }
    let mut a = st.lock();
    a.opts = opts;
    out(&a)
}

#[derive(Serialize, Deserialize)]
struct Recovery {
    name: String,
    path: Option<String>,
    saved: String,
}

fn recovery_files(app: &AppHandle) -> Option<(PathBuf, PathBuf)> {
    dir(app, true).map(|d| (d.join("recovery.dsim"), d.join("recovery.json")))
}

fn clear_recovery(app: &AppHandle) {
    if let Some((a, b)) = recovery_files(app) {
        let _ = std::fs::remove_file(a);
        let _ = std::fs::remove_file(b);
    }
}

#[tauri::command]
fn autosave(app: AppHandle, st: State<St>) {
    let a = st.lock();
    if let (true, Some((file, meta))) = (a.doc.dirty, recovery_files(&app)) {
        let info = Recovery { name: a.doc.m.name.clone(), path: a.doc.path.clone(), saved: chrono::Local::now().format("%d-%b-%y %H:%M").to_string() };
        let _ = std::fs::write(file, model::to_json(&a.doc.m));
        let _ = std::fs::write(meta, serde_json::to_string(&info).unwrap_or_default());
    }
}

#[tauri::command]
fn recovery(app: AppHandle) -> Option<Recovery> {
    let (file, meta) = recovery_files(&app)?;
    file.exists().then_some(())?;
    serde_json::from_str(&std::fs::read_to_string(meta).ok()?).ok()
}

#[tauri::command]
fn recover(app: AppHandle, st: State<St>, keep: bool) -> R<Option<Out>> {
    let info = recovery(app.clone());
    let files = recovery_files(&app);
    if !keep {
        clear_recovery(&app);
        return Ok(None);
    }
    let (file, _) = files.ok_or("No recovered model was found.")?;
    let m = model::from_json(&std::fs::read_to_string(file).map_err(|e| e.to_string())?)?;
    let mut a = st.lock();
    a.doc = Doc::new(m);
    a.doc.path = info.and_then(|i| i.path);
    a.doc.dirty = true;
    Ok(Some(out(&a)))
}

fn recent_paths(app: &AppHandle) -> Vec<String> {
    options_file(app).map(|f| f.with_file_name("recent.json")).and_then(|f| std::fs::read_to_string(f).ok()).and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

fn remember(app: &AppHandle, path: &Path) {
    let p = path.display().to_string();
    let mut v = recent_paths(app);
    v.retain(|x| *x != p);
    v.insert(0, p);
    v.truncate(12);
    if let Some(f) = options_file(app).map(|f| f.with_file_name("recent.json")) {
        let _ = std::fs::write(f, serde_json::to_string(&v).unwrap_or_default());
    }
}

#[derive(Serialize)]
struct Recent {
    path: String,
    name: String,
    modified: String,
}

#[tauri::command]
fn recent(app: AppHandle) -> Vec<Recent> {
    recent_paths(&app)
        .into_iter()
        .filter_map(|p| {
            let m = std::fs::metadata(&p).ok()?.modified().ok()?;
            let modified = chrono::DateTime::<chrono::Local>::from(m).format("%d-%b-%y %H:%M").to_string();
            let name = Path::new(&p).file_stem()?.to_string_lossy().into_owned();
            Some(Recent { path: p, name, modified })
        })
        .collect()
}

#[tauri::command]
fn startup_file(app: AppHandle, st: State<St>) -> Option<String> {
    let last = || if st.lock().opts.open_last { recent_paths(&app).into_iter().find(|p| Path::new(p).exists()) } else { None };
    std::env::args().skip(1).find(|a| a.to_lowercase().ends_with(".dsim")).or_else(last)
}

/// Opens one of the project's GitHub pages in the browser (only those).
#[tauri::command]
fn open_url(url: String) -> R<()> {
    if !url.starts_with("https://github.com/Edgar-Mendonca/Datum-Sim") && !url.starts_with("https://github.com/Edgar-Mendonca/Datum-Plot") && !url.starts_with("https://edgar-mendonca.github.io/Datum") {
        return Err("Only the Datum GitHub pages can be opened.".into());
    }
    #[cfg(windows)]
    let r = std::process::Command::new("explorer").arg(&url).spawn();
    #[cfg(not(windows))]
    let r = std::process::Command::new("xdg-open").arg(&url).spawn();
    r.map(|_| ()).map_err(|e| e.to_string())
}

#[tauri::command]
fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[tauri::command]
fn quit(app: AppHandle) {
    clear_recovery(&app);
    app.exit(0);
}

#[tauri::command]
async fn check_update(st: State<'_, St>) -> R<update::Info> {
    let beta = st.lock().opts.beta;
    tauri::async_runtime::spawn_blocking(move || update::check(beta)).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn install_update(app: AppHandle, url: String, sums: Option<String>) -> R<()> {
    tauri::async_runtime::spawn_blocking(move || update::install(&app, &url, sums.as_deref())).await.map_err(|e| e.to_string())?
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let opts = options_file(app.handle()).and_then(|f| std::fs::read_to_string(f).ok()).and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
            let a = App { doc: Doc::new(Model::new()), opts, clip: String::new(), run: RunState::default() };
            let m = fresh(&a, Model::new());
            app.manage(St(Mutex::new(App { doc: Doc::new(m), ..a })));
            app.manage(py::Engine(Mutex::new(py::Py::default())));
            Ok(())
        })
        .on_window_event(|w, e| {
            if let WindowEvent::CloseRequested { api, .. } = e {
                if w.state::<St>().lock().doc.dirty {
                    api.prevent_close();
                    let _ = w.emit("close-requested", ());
                } else {
                    clear_recovery(w.app_handle());
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            state, library, diagram, apply, history, copy, paste, examples_list, new_model, check, rates, open, save, save_file,
            run, control, results, series, sweep_series, run_sweep, export_csv, analyze, step_info, engine,
            options, set_options, autosave, recovery, recover, recent, startup_file, open_url, version, quit, check_update, install_update
        ])
        .run(tauri::generate_context!())
        .expect("Datum Sim failed to start");
}
