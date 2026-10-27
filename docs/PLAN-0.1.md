# Datum Sim: plan for 0.1

> Written in session 1 (2026-10-27), before any code (FAMILY §10, §12). Shape: brief → what we learned → brainstorm → architecture → 0.1 scope → release plan → risks → questions.
> **Nothing is built until the user answers §9.**

---

## 1. The brief in one paragraph

A free, open-source (MIT) Windows desktop app to **model and simulate dynamic systems**. You draw a block diagram, press Run and watch scopes fill as the system evolves. Later releases add control design (linearise, Bode, root locus, PID tuning), physical components (mechanical, electrical, hydraulic, thermal), sweeps and Monte Carlo, fitting to measured data and FMU exchange. It is part of the Datum family and follows `FAMILY.md` for stack, look, docs, updater and releases. The full brief is in [`briefs/DATUM-SIM.md`](briefs/DATUM-SIM.md).

## 2. What we learned before planning

Read in session 1: the hub (`CLAUDE.md`, `MEMORY.md`, `FAMILY.md`, `brand/BRAND.md`, `brain/`), Datum Plan (the reference code), Datum Plot (bundled Python), the `morluto/rea` repo (as an example of building and testing a hard tool), and a web survey of solver crates, simulator engines, diagram editors, FMI tooling and validation problems.

### Corrections to the brief

| Brief says | What we found | Consequence |
| --- | --- | --- |
| Start the block editor from Plan's Network Diagram | Plan's diagram (`ui/src/views.tsx` 13–144, `crates/core/src/network.rs`) is **read-only**: automatic col/row layout, fixed 3-segment wires that pass through boxes, click-select only. No dragging, ports, wire drawing, routing or multi-select. | Reuse its **viewport** (scroll pan, zoom, minimap, dot grid, `<marker>` arrows, print path). The **editing layer is new** (about 1–1.5k lines of UI plus a router in Rust). The biggest UI job in 0.1. |
| Undo/redo "as ops, like Plan's `doc.rs`" | Plan sends **ops** (`enum Op`, `Batch`) but undoes with **whole-document snapshots** (`undo: Vec<Project>`, limit 200). | Copy exactly that. A model is small; snapshots are simple and safe (the hardening rule "a failed edit leaves the document exactly as it was" falls out for free). |
| `evalexpr` for block expressions | `evalexpr` became **AGPL-3.0 from v12** (Oct 2024). `meval`/`fasteval` are unmaintained. | Write our own small expression parser (≈300–400 lines, `expr.rs`). We need it anyway: units, symbolic derivatives for linearisation (0.3) and equations for physical components (0.4). |
| Scopes in the family chart style with Chart.js | Chart.js struggles with 10⁵–10⁶ points updated live. Datum Plot does not use Chart.js at all (matplotlib SVG). | Propose **uPlot** (MIT, ≈50 kB, built for large live time series) for scopes, styled exactly like the family charts (white plot, thin axes, Okabe-Ito, crosshair, PNG button). Chart.js stays for small charts. **Question 7.** |
| Python "as in Datum Plot" | Plot's worker is one request → one answer over JSON lines: no request ids, no progress, no cancel, 60 s timeout, arrays as JSON/base64. Its installer is 113 MB and every update re-downloads the runtime. | Reuse Plot's runtime pack and process handling; **extend the protocol** (ids, progress, cancel, binary arrays) when Python arrives. **Question 2** decides when. |
| `diffsol`, SUNDIALS for solvers | `diffsol` 0.18 (MIT, pure Rust, active, BDF/SDIRK, mass-matrix DAEs, root finding), but its API still changes between minor versions. `sundials-sys` is BSD but needs CMake + clang on Windows. | Hand-write Euler, Heun, RK4 and Dormand–Prince 5(4) (full control of steps, events and streaming); use **diffsol for BDF** behind our own `Solver` trait. SUNDIALS (IDA) only if 0.4 needs fully implicit DAEs. |
| OpenModelica as an optional compiler | OpenModelica is OSMC-PL / **AGPL-3**. The full ModelingToolkit (Julia) is now AGPL too. | Study ideas, never copy code. External-process use only (0.5, optional). See question 4. |

### What `rea` teaches us (the way of building)

`morluto/rea` (MIT) has nothing about simulation, but it is a good model of **how to build a hard tool so it stays right**. We take these practices; we do not use rea against any other simulator (that would break their licences and FAMILY §13).

- **Layers that point inward**: engine (`crates/core`) ← app commands (`src-tauri`) ← window (`ui`). One owner per meaning: units are parsed once, block definitions live in one registry, sizes are worked out once in the compiler. Expected failures are typed errors, never panics; "unknown" is never treated as "absent".
- **Test depths**, each with a purpose:
  - *module*: colocated unit tests per block and solver;
  - *boundary*: one real seam per test (the `.dsim` reader, a Tauri command, later the Python worker);
  - *acceptance*: the built UI driving a real model end to end (Plan's `tests_ui/run.cjs` pattern);
  - *conformance*: **every solver passes one shared contract** (order of convergence, Dahlquist, oscillator energy, events), parameterised by what it supports (stiff, events, DAE).
- **Goldens from real producers**: reference trajectories come from closed-form answers or published test sets, stored with tolerances and the source they came from.
- **Property tests** (`proptest`): save → open is identity; block sorting is a valid order; halving the step shrinks the error by about 2^p.
- **Mutation testing** (`cargo-mutants`) on the solvers, step control, event location and the compiler, nightly and informational at first: it shows which tests would not notice a broken solver.
- **Dated verification records**: `docs/verification/YYYY-MM-DD.md` with commit, test counts, accuracy and speed numbers, and **what was not verified**. These grow into the verification report that ships with the app (§5 of the brief).
- **Report only checks that ran.** Already Datum's habit; written into `CLAUDE.md`.

What we skip from rea: Turbo, the CLI/MCP dual adapter, plugin packaging and its very long testing guide. Cargo, npm scripts and FAMILY §11 are enough.

## 3. Brainstorm

### Who uses it and for what
- **Students and lecturers**: step and Bode of a second-order system, PID loops, a worked example per lecture. They need clear help, examples and printouts.
- **Control engineers**: tuning loops before commissioning (anti-surge, level, temperature, speed). They need trustworthy solvers, step specifications, sweeps and Datum Plot export.
- **Mechanical, electrical and process engineers**: physical components (0.4) without writing equations.
- **Researchers**: reproducible models, verified solvers, scripting (Python blocks, 0.6), FMUs (0.5).

### What makes it good (beyond the feature list)
1. **Trust.** Every block and solver is tested against a known answer, and the verification report ships in the app. "Is this number right?" always has an answer.
2. **Instant feedback.** Scopes fill live; editing a parameter and pressing Run again takes under a second for typical models. Later: "live tuning" (drag a gain while it runs).
3. **Explains itself.** A `?` card on every block, with the equation, ports, parameters and a tiny example. Errors in plain words that point at the block: "Algebraic loop: Sum1 → Gain2 → Sum1. Add a delay or an integrator in the loop."
4. **Units on every parameter** (0.1 shows and checks them; conversion arrives with Calc's units work).
5. **Family connections.** Results to Datum Plot in one click; materials from Atlas (0.4); the same printing, help and updater as every other app.
6. **Files you can read.** `.dsim` is pretty JSON, versioned with migrations, diff-friendly in git.

### Ideas parked for later (not 0.1)
Live parameter tuning while running · signal logging to HDF5 · model comparison (diff two `.dsim` files) · "explain this result" (energy balance, which block saturated) · a command line (`datumsim run model.dsim --out results.csv`) for batch work and CI · a lecture pack of examples · a model checker (unconnected ports, unused parameters, suspicious step sizes) · code generation (C) for embedded targets · co-simulation with Datum Field later.

## 4. Architecture

```
crates/core           pure Rust, no Tauri, unit-tested
  model.rs            Model, Block, Port, Wire, Param{expr, unit}, Subsystem; ids
  doc.rs              Doc{model, path, dirty, undo, redo}; enum Op + Batch; snapshot undo (Plan's pattern)
  file.rs             .dsim read/write: JSON, "version" field, migrations, hostile-file safe
  expr.rs             our expression parser → AST → fast closure; variables = model parameters
  units.rs            unit strings on parameters (0.1: parse and show; check dimensions in 0.2)
  blocks/             the block registry: one BlockDef per kind (ports, params, states,
                      direct feedthrough per input, sample time, zero-crossings, help key)
    sources.rs math.rs continuous.rs discont.rs routing.rs lookup.rs sinks.rs
  compile.rs          flatten → check sizes → feedthrough graph → Tarjan SCC (petgraph)
                      → execution order; any cycle without a state = algebraic loop error
  sim/                Solver trait; euler, heun, rk4, dopri5 (own); bdf (diffsol)
    run.rs            the loop: major/minor steps, output decimation, stop flag, step mode
    events.rs         zero-crossings: sign change → bracket on dense output → reset (0.2 in full)
  results.rs          per-signal time series; decimation for scopes; CSV writer
  sweep.rs            parameter sweep (run N copies on threads, rayon-free std::thread)
  route.rs            orthogonal wire router: A* on a sparse grid built from block boxes
  verify/             the verification cases (closed-form answers) → tests and the report
  hardening.rs        FAMILY §11: failed edits, random models, damaged files, speed gate
src-tauri             Tauri 2 shell: commands, the run thread, events to the UI, update.rs, options
ui/src                SolidJS + strict TS (Plan's layout): ribbon, backstage, options, help,
                      print, roadmap + canvas.tsx (block editor), library.tsx, inspector.tsx,
                      runbar.tsx, scope.tsx (uPlot), store.ts, api.ts
py/                   (0.3, or 0.1 if question 2 says so) worker.py, datum_sim/…, tests
```

**The block interface** (taken from how open simulators are built: Scicos-style job flags, Drake/Collimator-style declarations; ideas only, no code):

```rust
trait Block {
    fn outputs(&self, t: f64, x: &[f64], u: &[&[f64]], y: &mut [&mut [f64]]);
    fn derivatives(&self, t: f64, x: &[f64], u: &[&[f64]], dx: &mut [f64]) {}
    fn update(&mut self, t: f64, x: &mut [f64], u: &[&[f64]]) {}        // major steps only (discrete, resets)
    fn zero_crossings(&self, t: f64, x: &[f64], u: &[&[f64]], z: &mut [f64]) {}
}
// plus the declaration in BlockDef: n_states, initial state, feedthrough[in], sample time
// (continuous | discrete{period, offset} | inherited | constant), number of zero-crossings.
```

- **Signals** are f64 vectors whose width the compiler infers (most are width 1); Mux/Demux change width.
- **Execution order**: a graph with only direct-feedthrough edges, sorted with Tarjan SCC. Integrators, delays and unit delays have no feedthrough, so they break loops. A non-trivial SCC is reported as an algebraic loop in 0.1; it is solved with Newton and tearing in 0.2.
- **Major and minor steps**: the solver calls `outputs`/`derivatives` at its stages (minor). Logging, discrete updates and mode changes (saturation, relay) happen only at accepted steps (major), so a variable-step solver never sees a block change its mind mid-step.
- **Transport delay** keeps a history buffer and reads it with interpolation.
- **Running**: `src-tauri` starts the simulation on a worker thread. Every ~50 ms it sends the new decimated samples to the UI as an event, and the scopes append them. Stop sets an `AtomicBool`; Step runs one major step. Full-resolution results stay in Rust; CSV and Plot export read them there.
- **Wires that route around blocks**: `route.rs` in Rust (fast, testable). A* on a sparse orthogonal grid built from block edges and port stubs, penalising bends and crossings, then nudging parallel segments apart. The UI asks for the routes after each move; a user-placed bend point is kept as a waypoint. No LGPL/EPL routing library is needed.
- **The window never calculates** (FAMILY §2): the UI sends ops and draws what the engine returns.

**Python, when it arrives** (question 2): Plot's runtime pack (`release.yml` runtime step, `runtime.rs`, `runtime.txt`) and process handling (`py.rs`) are copied. The protocol is extended:
- request `id`s;
- `progress` lines before the answer;
- `cancel` messages;
- per-command timeouts;
- **binary arrays**: a JSON header followed by raw little-endian f64, or a memory-mapped `.npy` in the cache folder, never base64.

Python starts **lazily**, on the first analysis. Heavy modules are imported inside their handlers. Results come back as arrays that the UI draws; matplotlib is used only for publication figures, if at all.

**Dependencies for 0.1** (Rust): serde, serde_json, petgraph (MIT/Apache), diffsol (MIT; the nalgebra backend), nalgebra (Apache-2.0), tauri 2, tauri-plugin-dialog, ureq, sha2 (as Plan). **UI**: solid-js, @fontsource IBM Plex, uPlot (MIT). All recorded in `NOTICE`; `deny.toml` from Plan.

## 5. Scope of 0.1

The brief's 0.1 list is large. To keep the family rule "small, consecutive releases" and have the updater working from day one, 0.1 is split into steps. Each step is a release through the updater.

| Release | What ships |
| --- | --- |
| **0.1.0** shell | FAMILY §12 shell copied from Plan and renamed: installer, updater, title bar, ribbon (Model · Simulate · Analyze · View · Help), File page (Home, New, Open, Save, Save As, Export, Print, Options, Updates, Roadmap), Options, Help with `?` cards, Roadmap page reading `docs/site.json`. **The whole UI laid out with planned features dimmed** (as Plot did). `.dsim` open/save with undo. The canvas shows a model (pan, zoom, minimap), with a first working run: Step → Transfer function → Scope with **RK4**. Hardening tests, `deny.toml`, CI on Linux, Windows and macOS. |
| **0.1.1** editor | Place blocks from the searchable library, drag with snap, draw wires port to port, orthogonal routing around blocks, multi-select (marquee, Shift/Ctrl), copy/paste, delete, align, inspector with parameters and units, a `?` card per block. |
| **0.1.2** blocks and solvers | About 25 core blocks (below). Euler, Heun, RK4, Dormand–Prince with error control, BDF (diffsol). Run bar: start, stop time, solver, step or tolerance, Run/Stop/Step, progress. Algebraic-loop and size errors pointing at the block. Live scopes with cursors and a PNG button; XY graph; display. |
| **0.1.3** results and examples | Parameter sweep (scope overlays); export to CSV and Datum Plot; Print to PDF (diagram, scopes, results report); worked examples; the first verification record. |

**The ~25 blocks of 0.1**:
- **Sources**: Step, Ramp, Sine, Pulse, Constant, Clock, From table, From file (CSV).
- **Math**: Gain, Sum, Product, Divide, Abs, Sign, Min/Max, Function (expression), Saturation.
- **Continuous**: Integrator (with limits; reset arrives with events in 0.2), Derivative, Transfer function, Zero-pole-gain, State-space, PID, Transport delay.
- **Discontinuities**: Dead zone, Rate limiter, Relay, Backlash, Quantiser. These use major-step modes; exact zero-crossing location arrives in 0.2.
- **Routing**: Mux, Demux, Switch, Selector.
- **Lookup tables**: 1-D, 2-D.
- **Sinks**: Scope, XY graph, Display, To file, To Datum Plot.

That is a little over 25. The rest of the brief's list follows its own release plan: discrete blocks, goto/from, subsystems and buses in 0.2.

**Worked examples for 0.1** (question 3): mass–spring–damper; DC motor speed with PID; pump–pipe–tank level (signal-block version); vehicle cruise control. Compressor anti-surge needs a compressor map, valve dynamics and events, so it suits 0.2 (events) or 0.4 (physical). The heat exchanger suits 0.4 (thermal).

**Definition of done for 0.1.x**: every block and solver has a test against a known answer; CI is green on three OSes; real-app screenshots (Xvfb) have been checked; and a 500-block model edits smoothly and runs within the measured speed gate.

## 6. Release plan after 0.1 (from the brief, unchanged unless the answers say so)

| Version | Scope |
| --- | --- |
| 0.2 | Subsystems and masks, buses, discrete and multi-rate blocks, events and zero-crossings (bouncing ball), integrator reset, algebraic-loop solving, goto/from |
| 0.3 | Control design: operating points, linearise, Bode, Nyquist, root locus, PID auto-tuning, step-response specifications (Python arrives here if question 2 says so) |
| 0.4 | Physical modelling (mechanical, rotational, electrical, hydraulic, thermal), DAE assembly, index reduction (Pantelides, BLT via Tarjan, tearing), diffsol mass-matrix DAEs or IDA |
| 0.5 | FMU import and export (FMI 2/3; Model Exchange maps 1:1 onto our block interface; the Reference-FMUs are the test suite); optional Modelica through OpenModelica as an external process |
| 0.6 | State machines, Python blocks, Monte Carlo and sensitivity, fitting to measured data, report builder |
| 1.0 | Validation suite and report, user guide, signed installer |

## 7. Verification plan (grows every release)

- **Solvers (conformance contract)**:
  - Dahlquist `y' = λy`;
  - order of convergence (halve h → error / 2^p);
  - harmonic oscillator (energy and phase);
  - Van der Pol at μ = 1 and μ = 1000;
  - Robertson to 1e11 (mass conserved; BDF only);
  - Lotka–Volterra (invariant);
  - bouncing ball (0.2: impact times `t1 = sqrt(2h/g)`, later intervals × e).
- **Blocks**:
  - first-order step `1 − e^(−t/τ)`;
  - second-order step (peak `exp(−ζπ/√(1−ζ²))`, settling ≈ `4/(ζωₙ)`);
  - PID loop against its closed form;
  - transport delay exactness;
  - lookup interpolation;
  - saturation and dead-zone algebra.
- **Published sets**: the Bari *Test Set for IVP Solvers* (Robertson, HIRES and others, with reference solutions); Hairer–Wanner problems.
- **Cross-checks** (once Python is in): SciPy `solve_ivp` and python-control `step_response`/`step_info`. FMI Reference-FMUs in 0.5.
- **Output**: `docs/verification/YYYY-MM-DD.md` (what ran, numbers, what was not verified) and, from 0.1.3, a printable verification report under Help.

## 8. Risks

| Risk | Size | What we do |
| --- | --- | --- |
| The block editor (dragging, ports, routing, 500 blocks smooth) | Large | Router in Rust with tests; SVG with viewport culling; a 500-block speed gate in the UI tests from 0.1.1 |
| Solver correctness, events and stiffness | Large | Conformance contract from day one; mutation testing on solvers; diffsol behind our trait |
| diffsol API changes | Medium | Pin the version; wrap it; its tests are part of the conformance contract |
| Python size and start-up (113 MB installers in Plot) | Medium | Rust-only until needed (question 2); lazy start; question 8 on updates |
| Scope performance with long runs | Medium | Decimate in Rust (min/max per pixel bucket); uPlot; full data stays in Rust |
| Scope creep (the brief is a multi-year product) | Large | Small releases; dimmed planned features; PLAN file before each big release |
| Licences (AGPL/GPL/LGPL nearby: evalexpr ≥12, OpenModelica, ModelingToolkit, Xcos, libavoid) | Medium | `cargo deny` licence allow-list; own parser and router; external tools only as separate processes |
| Unsigned installer (SmartScreen warnings, antivirus with a Python runtime) | Small now, larger with Python | As the family does today; signing at 1.0 (question 9) |

## 9. Questions for the user

1. **Accent colour.** The spares are Amber `#A8650F` (too close to the amber diamond and the warning colour) and Crimson `#B83250` (close to the error red). **Proposed: a new Orange `#C2410C`**, which is distinct from Plan's blue, Plot's teal and Atlas's violet, and passes 4.5:1 for text on white. Its accent set would be: `--blue #C2410C`, `--blue2 #A3360A`, `--sky #F6C3A5`, `--pale #FCEEE6`, `--sel #F9E0D1`, `--sel2 #F0C2A6`, dark `--blue #F08A5D`, sidebar `#5C220A`. The icon in all four candidates is in `docs/brand/accent-options.png`.
2. **Python.** Bundled from 0.1, like Plot: a ~110 MB installer, but analysis is ready from the start. Or **(proposed) a Rust-only 0.1** (installer about 8–10 MB) with Python arriving in 0.3 for control design, with the improved protocol.
3. **Worked examples for 0.1.** Proposed: mass–spring–damper, DC motor PID, pump–tank level, cruise control. Anti-surge and the heat exchanger come later (they need events or physical components). Is that the right order, and do you have real parameters (for example from your anti-surge work) you would like used?
4. **Physical modelling (0.4).** **Proposed: our own equation-based components in Rust.** This keeps everything MIT, is fast, and works with our solvers and FMU export. OpenModelica would be an optional external compiler later; it is AGPL/OSMC, so it runs as a separate process only.
5. **Priority after 0.1.** **Proposed: 0.2 (events, subsystems, discrete) first**, because both control design and physical models need it. Then control design (0.3) before physical modelling (0.4), as the brief says. Agree?
6. **Ribbon tabs.** Model · Simulate · Analyze · View · Help, with a contextual Format tab when blocks or wires are selected. **Proposed: yes**, with the File button first as in every family app.
7. **Scope charts.** Use **uPlot** (MIT, fast with large live data) for scopes, in the family chart style, instead of Chart.js. OK to add it as a family-approved chart library for large time series?
8. **Updates with Python (when it arrives).** Plot re-downloads its whole runtime (~100 MB) on every update. Keep that simple rule, or allow a small app-only update when the runtime has not changed?
9. **Default branch and deployment.** The repo is empty. Our first push (`claude/dreamy-babbage-dyf9nr`) becomes the default branch, as in the other apps. Is that fine, or would you like a `main` branch? Code signing waits for 1.0, as in the other apps. Agree?
10. **0.1 split.** Ship 0.1 as four small releases (0.1.0 shell → 0.1.1 editor → 0.1.2 blocks and solvers → 0.1.3 results and examples), as in §5?
