# Datum Sim: project memory

> **Paste this file at the start of each new session**, or just open the repo: `CLAUDE.md` tells Claude to read it.
> It records what exists, why, and what comes next. Update it at the end of every session.

| | |
| --- | --- |
| **Product** | Datum Sim: model and simulate dynamic systems (block diagrams, control design, physical components) |
| **Current version** | none yet (planning; 0.1.0 is next) |
| **Repo** | `Edgar-Mendonca/Datum-Sim` · branch `claude/dreamy-babbage-dyf9nr` |
| **Stack** | Rust engine (`crates/core`), Tauri 2, SolidJS + strict TypeScript, uPlot for scopes (proposed); bundled CPython for analysis from 0.3 (proposed) |
| **Identifier** | `com.edgarmendonca.datumsim` · files `.dsim` · installer `DatumSim-x.y.z-setup.exe` |
| **Accent** | Orange `#C2410C` (proposed, question 1) |
| **Code size** | 0 lines |
| **Installer** | none yet |
| **Reference repos** | `Edgar-Mendonca/Datum` (FAMILY, MEMORY, brain), `Edgar-Mendonca/Datum-Plan` (reference code), `Edgar-Mendonca/Datum-Plot` (bundled Python), `morluto/rea` (how a hard tool is built and tested) |

---

## 1. Architecture

The full design is in [PLAN-0.1.md](PLAN-0.1.md) §4. In short:
- `crates/core` holds the model, ops and snapshot undo, `.dsim`, the expression parser, the block registry, the compiler, the solvers, results, sweeps, the wire router and the verification cases.
- `src-tauri` holds the commands, the run thread that streams to the scopes, the updater and options.
- `ui` holds Plan's window plus the block canvas, library, inspector, run bar and scopes.

### Files

| File | What it does |
| --- | --- |
| `CLAUDE.md` | Notes for Claude: read MEMORY, preferences, checks, release steps |
| `docs/MEMORY.md` | This file |
| `docs/PLAN-0.1.md` | Brainstorm, architecture, 0.1 scope, release plan, risks, questions |
| `docs/briefs/DATUM-SIM.md` | The brief from the hub (session 9 of DATUM) |
| `docs/FAMILY.md` | Link to the family design guide in the hub |
| `docs/brand/datum-sim-icon.svg` | App icon (FAMILY §5): a feedback loop with a step response in the block; amber diamond on the output |
| `docs/brand/accent-options.png` | The icon in the four candidate accents, beside the family icons |
| `README.md`, `LICENSE` | Short readme (planning status); MIT licence |

### How we work

1. The user reviews a release and sends screenshots.
2. Reproduce first.
3. Big asks get a plan file and questions; build after the answers.
4. Engine first (with a test), then the UI.
5. Check every change in the real app (Xvfb screenshots).
6. Add a UI test for every bug.
7. Run all checks: clippy `-D warnings`, `cargo test` (with the hardening and verification tests), the UI type-check and UI tests, `cargo deny check`.
8. Release.
9. Report in plain language: what changed, what was checked, what is not done.

## 2. Decisions

| Version | Decision | Why |
| --- | --- | --- |
| plan | Plan's Network Diagram gives only the viewport (pan, zoom, minimap, dot grid, print); the block editor's interaction layer and the wire router are new | Plan's diagram is read-only with fixed wires through boxes (found in session 1) |
| plan | Undo with whole-model snapshots and ops in, as Plan's `doc.rs` does | Simple, and a failed edit leaves the model exactly as it was (FAMILY §11) |
| plan | Own expression parser (`expr.rs`), not evalexpr | evalexpr is AGPL-3.0 from v12; meval/fasteval unmaintained; we need symbolic derivatives later anyway |
| plan | Own Euler, Heun, RK4 and Dormand–Prince; diffsol (MIT) for BDF, behind our own `Solver` trait | Full control of major/minor steps, events and streaming; diffsol is pure Rust but its API still moves; SUNDIALS needs CMake + clang on Windows |
| plan | Block interface: outputs, derivatives, update (major steps), zero-crossings; feedthrough per input; sample time; execution order by Tarjan SCC on feedthrough edges (petgraph); non-trivial SCC = algebraic loop | How open simulators are built; ideas only, no GPL/AGPL code copied |
| plan | Own orthogonal wire router in Rust (A* on a sparse grid, bend and crossing penalties) | libavoid is LGPL, elkjs EPL; a router in the engine is testable and fast |
| plan | Take rea's practices: inward layers, test depths (module, boundary, acceptance, solver conformance), goldens from closed-form or published answers, proptest, cargo-mutants, dated verification records; never use rea against another simulator | The user asked for a strong way of building; reverse-engineering other simulators would break their licences and FAMILY §13 |
| plan | Proposed (awaiting answers): orange accent, Rust-only 0.1 with Python in 0.3, uPlot scopes, 0.1 in four releases, own physical components in 0.4 | PLAN-0.1 §9 |

## 3. Roadmap

✅ done · 🔜 next · ⬜ later

| Version | Item | Status |
| --- | --- | --- |
| plan | Read the hub, Plan, Plot and rea; web survey; PLAN-0.1, MEMORY, CLAUDE, icon | ✅ |
| plan | The user answers PLAN-0.1 §9 | 🔜 |
| 0.1.0 | Shell (installer, updater, ribbon, File page, Options, Help, Roadmap, dimmed planned features), `.dsim` with undo, first run (Step → TF → Scope, RK4), hardening, CI on three OSes | 🔜 |
| 0.1.1 | Block editor: library, drag and snap, wires, routing, multi-select, copy/paste, align, inspector, ? cards | ⬜ |
| 0.1.2 | About 25 blocks; Euler, Heun, RK4, Dormand–Prince, BDF; run bar; live scopes, XY, display | ⬜ |
| 0.1.3 | Parameter sweep; CSV and Datum Plot export; print to PDF; worked examples; first verification record | ⬜ |
| 0.2 | Subsystems and masks, buses, discrete and multi-rate, events and zero-crossings, algebraic loops, goto/from | ⬜ |
| 0.3 | Control design: operating points, linearise, Bode, Nyquist, root locus, PID tuning, step specs | ⬜ |
| 0.4 | Physical components (mechanical, rotational, electrical, hydraulic, thermal), DAEs | ⬜ |
| 0.5 | FMU import and export; optional Modelica via OpenModelica (external process) | ⬜ |
| 0.6 | State machines, Python blocks, Monte Carlo and sensitivity, fitting, report builder | ⬜ |
| 1.0 | Validation suite and report, user guide, signed installer | ⬜ |

## 4. Session log

### 2026-10-27: Session 1 (planning)
- The user pasted the brief and asked for a complete brainstorm and plan before deciding how to deploy and build 0.1, and pointed at `morluto/rea` for how hard apps are built.
- Read: the hub (CLAUDE, MEMORY, FAMILY, BRAND, the brain), Datum Plan (layout, Network Diagram, `doc.rs`, ribbon, backstage, print, updater, CI, UI tests), Datum Plot (runtime pack, worker protocol, tests), rea (AGENTS.md, testing depths, mutation testing, verification records, release automation). Web survey of solver crates, simulator engines, diagram editors, FMI tools and validation problems.
- Found: Plan's diagram is read-only; Plan undoes by snapshots; evalexpr is AGPL from v12; OpenModelica and the full ModelingToolkit are AGPL; Plot's worker has no ids, progress, cancel or binary arrays.
- Wrote: `docs/PLAN-0.1.md`, this MEMORY, `CLAUDE.md`, `docs/FAMILY.md`, `README.md`, `LICENSE`, the icon (`docs/brand/datum-sim-icon.svg`) and an accent comparison image. No code.
- Next: the user answers PLAN-0.1 §9; then build 0.1.0.

## 5. Known limits and next steps

- Nothing is built yet. The accent, Python timing, chart library and 0.1 split await the user's answers.
- After the first release, Datum Sim goes into the hub's `apps.json` (in the hub chat), and its icon is copied into the hub's `brand/`.
