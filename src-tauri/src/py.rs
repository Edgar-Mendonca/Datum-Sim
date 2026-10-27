//! The Python engine for analysis (numpy, SciPy, python-control): one worker process started on
//! the first analysis and kept running. Requests and answers are JSON lines (py/worker.py), each
//! with an id; the worker may send progress lines before the answer. Built from Datum Plot's
//! worker, with ids, progress and a time limit per request. The installed app uses the runtime
//! pack (runtime.rs); a development build uses DATUM_PYTHON or the Python on the PATH.
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{mpsc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

const START: Duration = Duration::from_secs(180);

pub struct Worker {
    child: Child,
    stdin: ChildStdin,
    lines: mpsc::Receiver<String>,
}

#[derive(Default)]
pub struct Py {
    worker: Option<Worker>,
    /// Python, numpy, scipy and python-control versions once the engine has answered.
    pub versions: Option<Value>,
    pub error: Option<String>,
    next: u64,
}

pub struct Engine(pub Mutex<Py>);

fn paths(app: &AppHandle) -> Result<(PathBuf, PathBuf), String> {
    let res = app.path().resource_dir().ok();
    let python = match std::env::var_os("DATUM_PYTHON") {
        Some(p) => p.into(),
        None => crate::runtime::python(app)?.unwrap_or_else(|| (if cfg!(windows) { "python" } else { "python3" }).into()),
    };
    let script = res.map(|r| r.join("py").join("worker.py")).filter(|p| p.exists()).unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../py/worker.py"));
    Ok((python, script))
}

impl Py {
    fn start(&mut self, app: &AppHandle) -> Result<(), String> {
        let (python, script) = paths(app)?;
        let mut cmd = Command::new(&python);
        cmd.arg("-X").arg("utf8").arg(&script).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        }
        let mut child = cmd.spawn().map_err(|e| format!("The analysis engine (Python) could not start: {e}. Looked for {}.", python.display()))?;
        let (stdin, stdout) = (child.stdin.take().unwrap(), BufReader::new(child.stdout.take().unwrap()));
        let (tx, lines) = mpsc::channel();
        std::thread::spawn(move || {
            for l in stdout.lines() {
                let Ok(l) = l else { break };
                if tx.send(l).is_err() {
                    break;
                }
            }
        });
        self.worker = Some(Worker { child, stdin, lines });
        let ready = self.read(None, START, None).map_err(|e| format!("The analysis engine (Python) stopped while starting: {e}"))?;
        if ready["ready"] != true {
            return Err("The analysis engine (Python) did not start properly.".into());
        }
        self.versions = self.ask_once(&json!({ "cmd": "ping" }), Duration::from_secs(30), None).ok();
        self.error = None;
        Ok(())
    }

    /// Read lines until the answer to `id` (progress lines go to the window as events).
    fn read(&mut self, id: Option<u64>, wait: Duration, app: Option<&AppHandle>) -> Result<Value, String> {
        let until = Instant::now() + wait;
        loop {
            let w = self.worker.as_mut().ok_or("not running")?;
            let left = until.saturating_duration_since(Instant::now());
            match w.lines.recv_timeout(left) {
                Ok(line) => {
                    let v: Value = serde_json::from_str(&line).map_err(|e| e.to_string())?;
                    if v.get("progress").is_some() {
                        if let Some(app) = app {
                            let _ = app.emit("analysis-progress", &v);
                        }
                        continue;
                    }
                    if id.is_none() || v["id"].as_u64() == id {
                        return Ok(v);
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    self.stop();
                    return Err(format!("it took longer than {} s and was stopped", wait.as_secs()));
                }
                Err(_) => {
                    self.stop();
                    return Err("the engine closed".into());
                }
            }
        }
    }

    fn ask_once(&mut self, q: &Value, wait: Duration, app: Option<&AppHandle>) -> Result<Value, String> {
        self.next += 1;
        let id = self.next;
        let mut q = q.clone();
        q["id"] = json!(id);
        let w = self.worker.as_mut().ok_or("not running")?;
        if writeln!(w.stdin, "{q}").and_then(|_| w.stdin.flush()).is_err() {
            self.stop();
            return Err("the engine closed".into());
        }
        let r = self.read(Some(id), wait, app)?;
        if r["ok"] == true {
            Ok(r)
        } else {
            Err(r["error"].as_str().unwrap_or("The analysis failed.").to_string())
        }
    }

    fn stop(&mut self) {
        if let Some(mut w) = self.worker.take() {
            let _ = w.child.kill();
            let _ = w.child.wait();
        }
    }

    /// Send a request (starting, or restarting once, the worker if needed).
    pub fn ask(&mut self, app: &AppHandle, q: &Value, wait: Duration) -> Result<Value, String> {
        for _ in 0..2 {
            if self.worker.is_none() {
                if let Err(e) = self.start(app) {
                    self.error = Some(e.clone());
                    return Err(e);
                }
            }
            match self.ask_once(q, wait, Some(app)) {
                Err(e) if e.contains("longer than") => return Err(format!("The analysis stopped: {e}.")),
                Err(e) if self.worker.is_none() => self.error = Some(e),
                r => return r,
            }
        }
        Err(format!("The analysis engine stopped: {}", self.error.clone().unwrap_or_default()))
    }

    pub fn restart(&mut self, app: &AppHandle) -> Result<Value, String> {
        self.stop();
        self.ask(app, &json!({ "cmd": "ping" }), Duration::from_secs(60))
    }
}

impl Drop for Py {
    fn drop(&mut self) {
        self.stop();
    }
}
