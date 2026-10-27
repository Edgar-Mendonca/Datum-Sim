# Datum Sim: project memory

> **Paste this file at the start of each new session**, or just open the repo: `CLAUDE.md` tells Claude to read it.
> It records what exists, why, and what comes next. Update it at the end of every session.

| | |
| --- | --- |
| **Product** | Datum Sim: model and simulate dynamic systems (block diagrams, control design, motors; physical components later) |
| **Current version** | 0.1.0 (built and tested; not released yet) |
| **Repo** | `Edgar-Mendonca/Datum-Sim` · branch `claude/dreamy-babbage-dyf9nr` (the first branch pushed, so the default branch) |
| **Stack** | Rust engine (`crates/core`), Tauri 2, SolidJS + strict TypeScript, uPlot for scopes; bundled CPython 3.13 with numpy, SciPy and python-control for analysis |
| **Identifier** | `com.edgarmendonca.datumsim` · files `.dsim` · installer `DatumSim-x.y.z-setup.exe` |
| **Accent** | Orange `#C2410C` (`--blue2` `#A3360A`, `--sky` `#F6C3A5`, `--pale` `#FCEEE6`, `--sel` `#F9E0D1`, `--sel2` `#F0C2A6`; dark `#F08A5D`; dark File sidebar `#5C220A`) |
| **Code size** | about 9 500 lines: engine 5 300 (Rust), shell 1 150 (Rust), window 2 000 (TS), Python 250, tests in each |
| **Installer** | built by `release.yml` with the Python runtime inside (expect about 70–90 MB; not built yet) |
| **Reference repos** | `Edgar-Mendonca/Datum` (FAMILY, MEMORY, brain), `Edgar-Mendonca/Datum-Plan` (reference code), `Edgar-Mendonca/Datum-Plot` (bundled Python), `morluto/rea` (how a hard tool is built and tested) |

---

## 1. Architecture

`crates/core` is the engine (pure Rust, no Tauri): the model and its edits, the block library, the compiler, the solvers, results, wire routing and sweeps. `src-tauri` is the shell: commands, files, the run thread, the Python engine and the updater. `ui` is the window: it draws what the engine returns and sends edits as operations. `py` is the Python analysis engine.

### How a run works

1. `compile.rs` flattens subsystems (their In and Out blocks become pass-throughs), evaluates every parameter with the model variables, infers signal widths (iterating to a fixed point), and sorts blocks by direct feedthrough (Kahn's algorithm). A cycle without a state is an algebraic loop error naming its blocks.
2. Each block (`rt.rs`) has `out` (outputs from state and inputs), `deriv` (continuous states), `update` (at accepted steps: discrete states, modes, histories), `next_hit` (times the solver must land on: jumps and samples) and `zc` (zero-crossing surfaces).
3. `solve::drive` steps the system: fixed step (Euler, Heun, RK4) or variable step (Dormand–Prince 5(4) with dense output; Rosenbrock 2(3) for stiff systems). It never steps over a hit, locates zero-crossings by bisection on the dense output, and calls the model at every accepted step (log, update, stop/pause).
4. The shell runs this on a thread and streams new samples to the window (`sim-live`, about 20 a second); `sim-done` ends it. Full results stay in Rust; the scopes ask for each zoom level reduced with peaks kept (`decimate_multi`).

### Files

| File | What it does |
| --- | --- |
| `crates/core/src/expr.rs` | Our expression parser: numbers, variables, + − * / ^, comparisons, functions, matrices `[1 2; 3 4]`; fast slot evaluation for the Function block |
| `crates/core/src/model.rs` | Model, diagrams, blocks, wires, variables, run settings; `.dsim` read (migrations, repair of damaged files) and write |
| `crates/core/src/doc.rs` | Edit operations (add, move, resize, set parameter, rename, flip, delete, connect, bend points, paste, group into a subsystem, ungroup, variables, settings, batch) with snapshot undo; copy |
| `crates/core/src/blocks.rs` | The block library: names, categories, parameters with units and hints, help text, sizes, ports, port positions, labels; planned blocks |
| `crates/core/src/rt.rs` | How each block runs (about 45 kinds, including the DC motor and the PMSM), transfer function → state space, roots → polynomials |
| `crates/core/src/compile.rs` | Flatten, parameters, widths, sorting, signal layout, logged channels; the run loop; CSV; decimation; the linear model of one block for analysis |
| `crates/core/src/solve.rs` | Euler, Heun, RK4, Dormand–Prince 5(4), Rosenbrock 2(3); the stepping loop with hits and zero-crossings; solver conformance tests |
| `crates/core/src/linalg.rs` | Dense LU with pivoting (for the stiff solver) |
| `crates/core/src/route.rs` | Orthogonal wire routing on a sparse grid (bend and shared-segment penalties, user bend points) |
| `crates/core/src/view.rs` | One diagram as the window draws it: blocks with port positions, labels, parameters; routed wires |
| `crates/core/src/sweep.rs` | Parameter sweeps on threads; values from any expression (`linspace(…)`) |
| `crates/core/src/examples.rs` | The worked examples, built in code |
| `crates/core/src/verify.rs` | Verification tests: blocks and models against closed-form answers |
| `crates/core/src/hardening.rs` | FAMILY §11: failed edits, random models, damaged files, speed gate |
| `crates/core/examples/dump.rs` | Writes `tests_ui/fixture.json` (the engine's real output) for the UI tests |
| `src-tauri/src/main.rs` | Commands: state, library, diagram, apply, history, copy/paste, examples, check, open/save, run/control, results/series, sweep, CSV, analysis, options, recovery, recent, updates |
| `src-tauri/src/py.rs` | The Python engine: one worker on demand, JSON lines with ids, progress lines, per-request time limits |
| `src-tauri/src/runtime.rs` | Unpacks the Python runtime pack from the installer (Plot's) |
| `src-tauri/src/update.rs` | The updater (Plan's, `REPO` = Datum-Sim) |
| `ui/src/main.tsx` | Window layout, sidebar, status bar, keys, start-up and recovery |
| `ui/src/store.ts` | Window state: model, diagram, selection, runs, results, menus, dialogs |
| `ui/src/api.ts` | Shell calls and the shapes they return |
| `ui/src/cmds.ts` | Every command (ribbon, menus, keys, Tell Me); run events |
| `ui/src/canvas.tsx` | The block editor: drawing, select, move, resize, wire, bend, pan/zoom, minimap, quick insert, context menu, keys |
| `ui/src/panels.tsx` | Library panel and Inspector (block parameters, or the run settings and variables) |
| `ui/src/scope.tsx` | uPlot scopes (live, zoom, sweep overlays, PNG/CSV), XY graph, the dock, Scopes and Results pages |
| `ui/src/ribbon.tsx` | Title bar, quick access, Tell Me, the ribbon (Model, Simulate, Analyze, View, Help, contextual Format) |
| `ui/src/backstage.tsx` | File page: examples, recent, export, options, updates |
| `ui/src/print.tsx` | Print: diagrams (with subsystems), scopes and a results report as pages |
| `ui/src/dialogs.tsx` | Variables page, Issues page, sweep, analyses, step specifications, help pages, recovery |
| `ui/src/help.tsx` | The `?` cards; block cards come from the library |
| `ui/src/roadmap.tsx` | File › Roadmap from `docs/site.json` |
| `ui/src/styles.css`, `ui/src/ui.tsx`, `ui/index.html` | Plan's styles (orange accent) and components; the icon sprite |
| `py/worker.py`, `py/datum_sim/linear.py` | The Python worker and the linear analyses (python-control) |
| `py/tests/test_worker.py` | Python tests against closed-form answers and the protocol |
| `tests_ui/run.cjs` | 22 UI checks of the built window with a mocked bridge |
| `tests_ui/realapp.sh` | Screenshots of the real app under Xvfb (xdotool steps) |
| `docs/verification/2026-10-27.md` | The first verification record: what was checked, numbers, what was not |
| `docs/PLAN-0.1.md`, `docs/site.json`, `CHANGELOG.md` | The plan, the public summary and roadmap, the change log |

### Release flow

`release.yml` (windows-latest): checks the tag matches the version, builds the newest embeddable CPython 3.13 with `py/requirements.txt`, **runs the Python tests on that runtime**, zips it into `src-tauri/runtime.zip`, builds one NSIS installer with it inside (`tauri.release.conf.json`), writes `SHA256SUMS.txt`, takes the notes from CHANGELOG and publishes the GitHub Release. Run it by pushing a tag `vX.Y.Z` or from the Actions tab. Every update downloads the whole installer (Plot's rule); the runtime is unpacked again only when `py/runtime.txt` changes.

### How we build (from PLAN-0.1 §2; practices taken from morluto/rea)

- Layers point inward: `crates/core` ← `src-tauri` ← `ui`. One owner per meaning: the block library (`blocks.rs`), port geometry (`blocks::port_xy`, drawn by the window), widths (the compiler). Expected failures are typed errors with the block's id, never panics.
- Test depths: module tests in each file; solver conformance in `solve.rs`; closed-form verification in `verify.rs`; boundary tests (file reader, Python protocol); acceptance (UI tests and real-app screenshots).
- Goldens come from closed-form answers or published values (Robertson, Hairer & Wanner).
- Dated verification records in `docs/verification/`, listing what was not verified.
- Report only checks that ran.

### How we work

1. The user reviews a release and sends screenshots.
2. Reproduce first.
3. Big asks get a plan file and questions; build after the answers.
4. Engine first (with a test), then the UI.
5. Check every change in the real app (Xvfb screenshots, `tests_ui/realapp.sh`).
6. Add a UI test for every bug.
7. Run all checks: clippy `-D warnings`, `cargo test` (with the hardening and verification tests), the Python tests, the UI type-check and UI tests, `cargo deny check`.
8. Release.
9. Report in plain language: what changed, what was checked, what is not done.

## 2. Decisions

| Version | Decision | Why |
| --- | --- | --- |
| plan | Plan's Network Diagram gives only the viewport ideas; the block editor's interaction layer and the wire router are new | Plan's diagram is read-only with fixed wires through boxes (found in session 1) |
| plan | Undo with whole-model snapshots and ops in, as Plan's `doc.rs` does | Simple, and a failed edit leaves the model exactly as it was (FAMILY §11) |
| plan | Own expression parser (`expr.rs`), not evalexpr | evalexpr is AGPL-3.0 from v12; meval/fasteval unmaintained; we need symbolic work later anyway |
| plan | Block interface: outputs, derivatives, update (major steps), next hit, zero-crossings; feedthrough; sorting on feedthrough edges; a cycle without state is an algebraic loop error | How open simulators are built; ideas only, no GPL/AGPL code |
| plan | Own orthogonal wire router in Rust | libavoid is LGPL, elkjs EPL; a router in the engine is testable and fast |
| plan | Take rea's practices (inward layers, test depths, goldens, verification records); never use rea against another simulator | The user asked for a strong way of building; reverse-engineering other simulators would break their licences and FAMILY §13 |
| 0.1.0 | **Orange accent `#C2410C`** (a new family accent) | The user's answer 1 (session 2); distinct from blue, teal and violet; 5.2:1 on white |
| 0.1.0 | **Python from the start** (numpy, SciPy, python-control only), started on the first analysis | The user's answer 2: "add Python in the beginning, not too big"; lazy start keeps launch fast |
| 0.1.0 | **uPlot** for scopes in the family chart style (Chart.js stays for small charts) | The user's answer 7; Chart.js is too slow for long live signals |
| 0.1.0 | **Own Rosenbrock 2(3) stiff solver** instead of diffsol's BDF for now | The block system is a black box stepped by our loop (hits, zero-crossings, streaming, stop/pause); a one-step method with a finite-difference Jacobian and dense output fits that loop with no dependency. Robertson and Van der Pol (μ = 1000) pass. diffsol stays the candidate for DAEs in 0.4 |
| 0.1.0 | Signals are vectors with inferred widths; most blocks work element by element | Mux/Demux and vector constants from the start, without a second signal type later |
| 0.1.0 | Piecewise-constant sources use the step's start time inside a step (`Ctx::tseg`) and the solver lands on every jump (`next_hit`) | A step input never smears across a solver step: the step time is hit exactly with every solver |
| 0.1.0 | Subsystems from 0.1 (create, open, ungroup, nest); masks in 0.2 | The user asked for subsystems in the first build |
| 0.1.0 | **Motor blocks**: DC motor and PMSM (d–q) as library blocks, plus a DC motor built from blocks in a subsystem as an example; induction motor in 0.2 | The user asked for motor simulation; a library block is fast and exact, the subsystem shows how it is built |
| 0.1.0 | Ribbon Model · Simulate · Analyze · View · Help with a contextual Format tab; planned tools dimmed with their release | The user agreed (answer 6); Plot's "whole UI laid out" rule |
| 0.1.0 | One installer with Python inside; every update downloads it whole | Datum Plot's rule (the user did not ask otherwise; question 8 left open) |
| 0.1.0 | The branch `claude/dreamy-babbage-dyf9nr` is the default branch, as in the other apps; signing waits for 1.0 | Question 9 left open; the family's way |
| 0.1.0 | Send to Datum Plot = a CSV Plot can open (time first, named columns) | Plot opens only `.dplot` files from the command line; a direct hand-over needs a change in Plot |
| 0.1.0 | `zip` 9 (not 2 as in Plot) in the shell | cargo deny: three advisories in zip 2.x (RUSTSEC-2026-0367/0368/0369) |

## 3. Roadmap

✅ done · 🔜 next · ⬜ later

| Version | Item | Status |
| --- | --- | --- |
| plan | Read the hub, Plan, Plot and rea; web survey; PLAN-0.1, MEMORY, CLAUDE, icon | ✅ |
| plan | The user answers PLAN-0.1 §9 | ✅ (session 2: 1, 2, 3, 6, 7, 10; 4, 5 agreed in principle; 8, 9 left as the family default) |
| 0.1.0 | Engine: model, edits, 45 blocks, compiler, five solvers, events, routing, sweeps, examples, verification and hardening tests | ✅ |
| 0.1.0 | Shell and window: family ribbon, File page, Options, Help, Roadmap, editor, inspector, scopes, subsystems, Python analyses, print, updater | ✅ |
| 0.1.0 | CI, release workflow with the Python runtime, cargo deny, UI tests | ✅ (CI not yet seen green on GitHub) |
| 0.1.0 | First release through the updater | 🔜 (the user starts it: tag `v0.1.0` or run Release) |
| 0.1.x | Check on a Windows PC: installer, Python first start, printing to PDF, updater | 🔜 |
| 0.1.x | Wire branch points and labels; faster drawing of very large diagrams; print preview polish; more examples (heat exchanger, compressor anti-surge) | ⬜ |
| 0.2 | Masks, buses, goto/from, discrete TF and PID, multi-rate, integrator reset, algebraic-loop solving, induction motor | ⬜ |
| 0.3 | Control design: trim, linearise whole models, Nyquist, root locus, PID tuning | ⬜ |
| 0.4 | Physical components (mechanical, rotational, electrical, hydraulic, thermal), DAEs | ⬜ |
| 0.5 | FMU import and export; Modelica through an external compiler | ⬜ |
| 0.6 | State machines, Python blocks, Monte Carlo, sensitivity, fitting, report builder | ⬜ |
| 1.0 | Validation suite and report, user guide, signed installer | ⬜ |

## 4. Session log

### 2026-10-27: Session 1 (planning)
- The user pasted the brief and asked for a complete brainstorm and plan before deciding how to deploy and build 0.1, and pointed at `morluto/rea` for how hard apps are built.
- Read: the hub (CLAUDE, MEMORY, FAMILY, BRAND, the brain), Datum Plan, Datum Plot and rea. Web survey of solver crates, simulator engines, diagram editors, FMI tools and validation problems.
- Found: Plan's diagram is read-only; Plan undoes by snapshots; evalexpr is AGPL from v12; OpenModelica and the full ModelingToolkit are AGPL; Plot's worker has no ids, progress, cancel or binary arrays.
- Wrote: `docs/PLAN-0.1.md`, this MEMORY, `CLAUDE.md`, `docs/FAMILY.md`, `README.md`, `LICENSE`, the icon and an accent comparison image. No code.

### 2026-10-27: Session 2 (0.1.0 built)
- The user's answers: orange accent; Python from the start but small; the worked examples plus motor simulation and subsystems ("a little bigger build"); the family look; uPlot; "just start building, push bit by bit"; update the memory and the brain.
- Built the engine (expression parser, model and file, edits with undo, 45 blocks with a DC motor and a PMSM, compiler, five solvers with exact hits and zero-crossings, router, sweeps, seven examples), the shell (Plan's, with a run thread streaming to the scopes and Plot's Python engine with ids), the window (Plan's family window in orange; block editor; inspector; uPlot scopes; subsystems; analyses; print), CI and the release workflow with the Python runtime.
- Checked: 37 Rust tests (closed-form verification, solver conformance with Robertson, Van der Pol and the bouncing ball, hardening; 2 000 random models once), 7 Python tests, 22 UI checks, clippy, cargo deny, and the real app under Xvfb (every example, live scopes, subsystems, Bode, drag-and-drop building and wiring, dark theme).
- Bugs found and fixed: widths growing forever through a Mux in a loop and a Sum with `*` signs (random-model tests); a run that ended before its start call returned stayed "Running"; the ribbon jumped when the contextual tab appeared; scopes kept light colours in dark theme; the unit delay updated too early; Constant blocks were reset by flattening; three zip advisories (moved to zip 9).
- Not done: no release yet; not run on Windows; brain not regenerated in the hub (this session cannot push there: the user adds Datum Sim to `apps.json` in the hub chat after the first release).

## 5. Known limits and next steps

- Not yet run on Windows: the installer, the embedded Python's first start (unpacking the runtime), printing to PDF and the updater. CI builds and tests on Windows and macOS.
- The Python engine's large-array path (binary frames instead of JSON) is not needed yet: analyses send small data; step specifications send at most 20 000 points.
- Wires have no branch points of their own (a second wire from the same output is drawn separately) and no labels.
- Algebraic loops are reported, not solved (0.2). Discrete blocks are the zero-order hold and unit delay only.
- Linear analysis works on single blocks; linearising whole models arrives in 0.3.
- Datum Plot has the same zip 2.x advisories (fix in Plot's chat: zip 9).
- After the first release: add Datum Sim to the hub's `apps.json` and copy `docs/brand/datum-sim-icon.svg` into the hub's `brand/` (in the hub chat), then regenerate the brain.
