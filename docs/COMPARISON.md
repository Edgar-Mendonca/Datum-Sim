# Datum Sim: comparison with industry-standard tools

Written 2026-11-02 at 0.2.0; brought up to date 2026-11-07 at **Datum Sim 0.2.1** (automation added, open-source tools added in §4, and an honest answer to "do we stand a chance?" in §6). The roadmap that follows from it is `docs/ROADMAP.md`. This is an internal planning document, in the family pattern (Datum Plot's PLAN-0.1 §1–2, Datum Plan's BRAINSTORM §3): how the tools think → feature tables → where we lead and trail → what it means for the roadmap. Product names appear here only to describe features. They never appear in the app's public words (README, CHANGELOG, site.json, help; FAMILY §13).

**Compared with:**
- **Simulink**, with its usual add-ons: Simscape, Stateflow, Simulink Control Design, Embedded Coder. It is the industry standard for block-diagram simulation and model-based design.
- **PLECS**: the reference tool for power electronics and motor drives. That is the closest match to Datum Sim's motor focus.
- **Scilab Xcos** and **OpenModelica**: the two established open-source tools. Smaller open-source projects (PathSim/PathView, bdsim, Hopsan, MatCraft) are in §4.

Facts about the other tools come from their public documentation and common knowledge. Exact features depend on version and licence.

---

## 1. Where Datum Sim is (0.2.1)

- **Editor:**
  - Block diagram editor with wires that route themselves; segments can be moved, wires branched and signals named.
  - Notes, Find, block colours, subsystems.
  - **Masks** (parameters, help, typical ratings) and **My blocks**, a library of copied subsystems.
- **Blocks:** 85, from sources to motors:
  - Discrete control, with sample-time colours.
  - Goto/From tags.
  - **14 motor types** with LV/MV/HV typical ratings.
  - Drive pieces: inverter (average or switched), PWM, Clarke and Park transforms, DC link, encoder, sensor.
- **Engine:** five solvers (Euler, Heun, RK4, Dormand–Prince 5(4), Rosenbrock 2(3) for stiff systems). Hits and zero-crossings are located exactly. Signals are vectors with inferred widths. Subsystem and mask scopes.
- **Results:** live scopes with cursors, step-response marks, comparison of the last five runs, stacked plots and PNG/SVG export. A probe that reads any wire after a run. Parameter sweeps and CSV export.
- **Analysis:** built-in Python (python-control) gives Bode, margins, step and pole-zero plots of **single linear blocks**, plus step-response figures of any signal.
- **Trust:**
  - 71 engine tests against closed forms, 4 command-line tests, 10 Python tests (equivalent circuits, power balance, difference equations).
  - 61 UI checks and real-app checks under Xvfb.
  - A dated verification record per release.
  - Cargo deny on every dependency.
- **Automation (0.2.1):** the `datum-sim-cli` command line (run, sweep, check, info, examples) and a Python API (`import datum_sim`: open, set variables and parameters, run, sweep, results as NumPy arrays). Models cannot yet be *built* from code.
- **Delivery:** one Windows installer (≈ 90 MB, Python included) that updates itself. MIT licence.

---

## 2. How the tools think

| | Simulink (+ add-ons) | PLECS | Scilab Xcos | OpenModelica | **Datum Sim** |
| --- | --- | --- | --- | --- | --- |
| **Who uses it** | Control, automotive, aerospace, embedded teams; universities | Power-electronics and drive engineers | Students and teachers; budget-limited labs | Physical-system modellers; researchers | Control, electrical, mechanical and mechatronics engineers; students |
| **Core idea** | Signal block diagrams, plus physical networks (Simscape) and state machines (Stateflow). Everything feeds code generation and verification. | Electrical circuits (switches, machines) mixed with control blocks. Fast piecewise-linear switching simulation. | Signal block diagrams inside Scilab, with Modelica blocks | Equation-based, acausal physical models (Modelica) | Signal block diagrams with a strong, checked motor library. Small, fast, honest about its numbers. |
| **Strength** | Breadth and depth, toolchain, code generation, industry adoption | Switching accuracy and speed, thermal losses, drive libraries | Free, scriptable, complete enough to teach | Physical modelling power, open standard, FMI | Ease and speed to a first result, modern UI, motor presets LV–HV, public verification |
| **Weakness** | Price, size (GBs), learning curve; licences per add-on | Narrow focus; paid; control features are thinner | Dated UI, crashes on bigger models, little polish | Steep curve, slow compiles for big models, editor rough | Young: no physical networks, no state machines, no code generation, whole-model analysis still to come |
| **Files** | `.slx` (zip of XML), `.mdl` (text) | `.plecs` (text) | `.zcos` (zip of XML) | `.mo` (Modelica text) | `.dsim` (JSON, format 2) |
| **Price** | High, per seat and add-on | Paid | Free (GPL) | Free (OSMC-PL/AGPL/GPL) | Free (MIT) |

---

## 3. Feature comparison

✓ full · ◐ partial · — none. The **Datum Sim** column says what we have and the release it came in, or the stage where it is planned in `docs/ROADMAP.md` (0.2.x–0.3.0 control design, 0.3.x drives, 0.4.x physical components, 0.5.x exchange, 0.6.x studies and logic, 0.7.x reach and speed), or "not planned".

### 3.1 Editor and modelling

| Feature | Simulink | PLECS | Xcos | OpenModelica | **Datum Sim** |
| --- | --- | --- | --- | --- | --- |
| Block diagram editor | ✓ | ✓ | ✓ | ✓ | ✓ 0.1.0 |
| Wires that route themselves | ✓ | ◐ | ◐ | ◐ | ✓ 0.1.0 |
| Branches, segment editing, signal names on wires | ✓ | ✓ | ◐ | ◐ | ✓ 0.1.6 |
| Subsystems, nested | ✓ | ✓ | ✓ | ✓ | ✓ 0.1.0 |
| Masks (own parameters, icon, help) | ✓ | ✓ | ◐ | ✓ (models are parameterised) | ✓ 0.2.0 (text icon only) |
| User libraries | ✓ (linked) | ✓ | ✓ (palettes) | ✓ (packages) | ✓ 0.2.0 (copied, by choice) |
| Variants (switchable subsystems) | ✓ | ✓ | — | ✓ (replaceable) | 0.7.3 |
| Buses (named signal groups) | ✓ | ◐ | — | ✓ (connectors) | 0.2.8 |
| Goto/From tags | ✓ | ✓ | ✓ | n/a | ✓ 0.1.4 |
| Notes, colours, Find | ✓ | ✓ | ◐ | ◐ | ✓ 0.1.6 |
| Model checker (unconnected, unused, suspicious settings) | ✓ (Model Advisor) | ◐ | — | ◐ | ◐ (errors and warnings, Issues page) |
| Model comparison and merge (diff) | ✓ | — | — | ◐ | 0.7.3 |
| Undo, recovery, autosave | ✓ | ✓ | ◐ | ◐ | ✓ 0.1.0 |

### 3.2 Block library

| Area | Simulink | PLECS | Xcos | OpenModelica | **Datum Sim** |
| --- | --- | --- | --- | --- | --- |
| Sources, math, routing, sinks | ✓ (hundreds) | ✓ | ✓ | ✓ | ✓ (≈ 40) |
| Continuous: TF, ZPK, state space, PID, delay | ✓ | ✓ | ✓ | ✓ | ✓ 0.1.0 |
| Discontinuities: saturation, dead zone, relay, backlash, quantiser, rate limiter | ✓ | ✓ | ✓ | ✓ | ✓ 0.1.0 |
| Discrete: TF, SS, integrator, PID, rate transition | ✓ | ✓ | ✓ | ✓ | ✓ 0.1.4 |
| Lookup tables 1-D, 2-D | ✓ (n-D) | ✓ | ✓ | ✓ | ✓ 0.1.0 (1-D, 2-D) |
| Function block (expression) | ✓ (and MATLAB Function) | ✓ (C-Script) | ✓ (Scilab) | ✓ | ✓ 0.1.0 (expression) |
| Python / script blocks | ✓ (MATLAB, Python) | ✓ (C) | ✓ (Scilab, C) | ✓ (external functions) | 0.6.2 (Python block) |
| State machines | ✓ (Stateflow) | ✓ (state machine block) | ◐ | ✓ (state graphs) | 0.6.1 |
| Physical networks: mechanical, electrical, hydraulic, thermal | ✓ (Simscape) | ✓ electrical, thermal, mechanical | ◐ (via Modelica) | ✓ (core strength) | 0.4.1–0.4.6 |
| Total size of the library | thousands | hundreds | hundreds | thousands (MSL) | 85 |

### 3.3 Simulation engine

| Feature | Simulink | PLECS | Xcos | OpenModelica | **Datum Sim** |
| --- | --- | --- | --- | --- | --- |
| Fixed-step solvers | ✓ (many) | ✓ | ✓ | ✓ | ✓ Euler, Heun, RK4 |
| Variable-step explicit | ✓ | ✓ | ✓ | ✓ | ✓ Dormand–Prince 5(4) |
| Stiff solvers | ✓ (ode15s, ode23s, ode23tb…) | ✓ | ✓ (LSodar, IDA) | ✓ (DASSL, IDA…) | ✓ Rosenbrock 2(3); BDF/IDA with DAEs in 0.4 |
| DAEs (algebraic constraints) | ✓ (Simscape) | ✓ | ✓ | ✓ | 0.4.1 |
| Exact event location (zero crossings), hits at discontinuities | ✓ | ✓ (switching events) | ✓ | ✓ | ✓ 0.1.0 |
| Algebraic loops solved | ✓ | ✓ | ◐ | ✓ (tearing) | — reported only; 0.2.8 |
| Multirate discrete, sample-time colours | ✓ | ✓ | ◐ | ◐ | ✓ 0.1.4 |
| Vector signals, width inference | ✓ (and matrices, buses, fixed point) | ✓ | ✓ | ✓ | ✓ 0.1.0 (vectors) |
| Fixed-point and integer data types | ✓ | — | ◐ | ◐ | — not planned |
| Speed: large models, accelerator / compiled mode | ✓ (accelerator, rapid accelerator) | ✓ (fast) | ◐ | ✓ (compiled C) | ◐ (Rust engine, interpreted blocks; compiled mode 0.7.2) |
| Run in real time, pause, step | ✓ (pacing, step back) | ✓ | ◐ | ◐ | ◐ pause and step ✓; real-time pacing 0.2.7 |

### 3.4 Motors, power electronics and drives

| Feature | Simulink (Simscape Electrical) | PLECS | Xcos | OpenModelica (MSL Electrical) | **Datum Sim** |
| --- | --- | --- | --- | --- | --- |
| DC machines (PM, separately excited, shunt, series, compound) | ✓ | ✓ | ◐ | ✓ | ✓ 0.1.2 |
| Induction (cage, double cage, wound rotor) | ✓ | ✓ | — | ✓ | ✓ 0.1.2 |
| Synchronous (PMSM surface/interior, reluctance, wound field with dampers) | ✓ | ✓ | — | ✓ | ✓ 0.1.2 |
| BLDC, stepper, switched reluctance | ✓ | ✓ | — | ◐ | ✓ 0.1.2, SRM 0.1.4 |
| Saturation, iron losses, temperature effects in machines | ✓ | ✓ | — | ◐ | 0.3.5 |
| Typical ratings LV/MV/HV with one click | ◐ (parameter sets per block) | ◐ | — | — | ✓ 0.1.2 (a strength) |
| Inverter: average model and switched with exact switching | ✓ | ✓ (core strength) | — | ◐ | ✓ 0.1.2 |
| Semiconductor-level switching (diodes, IGBT models, dead time, snubbers) | ✓ | ✓ | — | ✓ | 0.4.3 (with electrical components) |
| Converter losses and thermal models | ✓ | ✓ (core strength) | — | ◐ | 0.3.3–0.3.4 |
| Clarke/Park, PWM, SVPWM, encoder, sensors | ✓ | ✓ | — | ◐ | ✓ (SVPWM and dead time 0.3.2) |
| Ready drive examples (DOL, V/f, FOC, BLDC, grid synchronous, SRM) | ✓ | ✓ | — | ◐ | ✓ 0.1.2–0.1.4 |
| Machine parameters from nameplate / test data (parameter estimation) | ✓ | ◐ | — | — | 0.3.6 (from the nameplate) |
| Power balance checks published | — | — | — | — | ✓ 0.1.4 (verification record) |

### 3.5 Results and scopes

| Feature | Simulink | PLECS | Xcos | OpenModelica | **Datum Sim** |
| --- | --- | --- | --- | --- | --- |
| Live scopes while running | ✓ | ✓ | ✓ | ◐ | ✓ 0.1.0 |
| Cursors, measurements, step marks | ✓ | ✓ | ◐ | ◐ | ✓ 0.1.5 |
| Compare runs | ✓ (Data Inspector) | ✓ | — | ✓ | ✓ 0.1.5 (last five) |
| Log any signal, read it on the wire | ✓ | ✓ (probes) | ◐ | ✓ (all variables) | ✓ 0.1.5 |
| Stacked plots, y-zoom, pictures (PNG/SVG) | ✓ | ✓ | ◐ | ✓ | ✓ 0.1.5 |
| XY graph, display | ✓ | ✓ | ✓ | ✓ | ✓ 0.1.0 |
| FFT / spectrum, THD | ✓ | ✓ (core for drives) | ◐ | ◐ | 0.3.1 (Datum Plot hand-over today) |
| Export: CSV / MAT / HDF5 | ✓ | ✓ | ✓ | ✓ | ◐ CSV and JSON ✓; MAT and HDF5 0.5.x |
| Hand-over to a plotting app | ✓ (MATLAB) | ◐ | ✓ (Scilab) | ✓ | ✓ Datum Plot |

### 3.6 Control design and analysis

| Feature | Simulink (+ Control Design) | PLECS | Xcos | OpenModelica | **Datum Sim** |
| --- | --- | --- | --- | --- | --- |
| Bode, margins, step, poles/zeros of a block | ✓ | ✓ | ✓ (Scilab) | ◐ | ✓ 0.1.0 |
| Linearise a whole model between analysis points | ✓ | ✓ (small-signal analysis) | ◐ | ✓ | 0.2.2 |
| Trim / operating points | ✓ | ◐ (steady-state analysis) | ◐ | ◐ | 0.2.4 |
| Nyquist, root locus, sensitivity | ✓ | ◐ | ✓ (Scilab) | ◐ | 0.2.3 |
| PID auto-tuning | ✓ | — | — | — | 0.2.5 |
| Live tuning while running | ✓ | ◐ | — | — | 0.2.7 |
| Parameter sweeps | ✓ | ✓ | ◐ | ✓ | ✓ 0.1.0 |
| Monte Carlo, sensitivity, optimisation | ✓ (Design Optimization) | ◐ | ◐ | ◐ | 0.6.3–0.6.4 |
| Parameter estimation from measured data | ✓ | — | ◐ | ◐ | 0.6.4 |

### 3.7 Exchange, automation, deployment

| Feature | Simulink | PLECS | Xcos | OpenModelica | **Datum Sim** |
| --- | --- | --- | --- | --- | --- |
| FMU import and export (FMI) | ✓ | ✓ | ◐ | ✓ | 0.5.1–0.5.2 |
| Modelica models | ◐ | — | ✓ | ✓ | 0.5.4 (external compiler) |
| Scripting / API: open, change, run, sweep models from code | ✓ (MATLAB) | ✓ (RPC, scripting) | ✓ (Scilab) | ✓ (OMPython, scripts) | ✓ 0.2.1 (Python API) |
| Scripting / API: build models (add blocks and wires) from code | ✓ | ✓ | ✓ | ✓ | — 0.5.x |
| Command line / batch runs, CI | ✓ | ✓ | ✓ | ✓ | ✓ 0.2.1 (`datum-sim-cli`) |
| C code generation for embedded targets | ✓ (core strength) | ✓ | ◐ | ✓ | — not planned (1.0+) |
| Hardware in the loop, real-time targets | ✓ | ✓ (RT Box) | ◐ | ◐ | — not planned |
| Requirements, test harnesses, coverage | ✓ | — | — | ◐ | — not planned |
| Import of other tools' model files | n/a | ◐ | ✓ (Scicos) | ✓ | — not planned |

### 3.8 Product

| | Simulink | PLECS | Xcos | OpenModelica | **Datum Sim** |
| --- | --- | --- | --- | --- | --- |
| Install size, time to first run | GBs, licence setup | Moderate | Moderate | Large | **≈ 90 MB, minutes** |
| Look and feel | Mature, dense | Clean | Dated | Functional | **Modern family ribbon, light/dark** |
| Help: ? cards on every tool and block, worked examples | ✓ (documentation) | ✓ | ◐ | ◐ | ✓ (18 worked examples) |
| Platforms | Win/Mac/Linux | Win/Mac/Linux | Win/Mac/Linux | Win/Mac/Linux | Windows (CI also on Linux and macOS) |
| Price | High | Paid | Free | Free | **Free, MIT** |
| Public verification of results | — (certification kits, paid) | — | — | ◐ (test suites) | ✓ (records per release) |
| Updates inside the app | ✓ | ✓ | — | — | ✓ |

---

## 4. The open-source field

What else is free, and where each one sits. Checked 2026-11-02 from each project's public repository and documentation.

| | **Datum Sim** | Scilab Xcos | OpenModelica | PathSim / PathView | bdsim | Hopsan | MatCraft |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Kind | Block diagrams, desktop app | Block diagrams inside Scilab | Equation-based physical modelling | Python block-diagram library + browser editor | Python block-diagram library | Fluid power and mechatronics | A MATLAB-style language environment (no block diagrams) |
| Graphical editor | Modern | Dated | OMEdit, functional | New (2026) | Basic | Yes | n/a |
| Motor and drive library | **Strong** (14 kinds, LV–HV ratings, checked) | Weak | Good (Modelica libraries) | Little | Little | Some | n/a |
| Physical (port-based) models | 0.4.x | Via Modelica | **Core strength** | Partial | No | **Core strength** | n/a |
| Control design | Single blocks; whole models 0.2.x | Basic (Scilab) | Linearisation | Limited | Basic | Limited | Toolbox-style functions |
| Scripting | CLI + Python (0.2.1) | Scilab | Modelica, Python | Python is the interface | Python is the interface | API | Its own language |
| Install and first run | **≈ 90 MB, minutes** | Moderate | Large | pip / browser | pip | Moderate | Alpha |
| Public test evidence | **Records per release** | Little | Test suites | Paper and tests | Some | Academic | — |
| Licence | MIT | GPL-2 | OSMC-PL / AGPL / GPL | MIT | MIT | Apache-2.0 | (see repo) |

**What it shows.** The free tools are either powerful but hard to start with (OpenModelica, Xcos) or good engines with little or no window (the Python libraries). **No free tool is at the same time easy to install, modern to use, strong on motors and drives, and open about how its numbers were checked.** That gap is Datum Sim's place.

---

## 5. Where we stand, in one table

A rough coverage of the industry standard's **everyday** feature set, by area (my estimate; the stages are those of `docs/ROADMAP.md`):

| Area | At 0.2.1 | After 0.3.0 (control design) | After 0.3.x (drives) and 0.4.x (physical) | Main gap after that |
| --- | --- | --- | --- | --- |
| Editor and modelling | ≈ 75 % | ≈ 80 % (buses) | ≈ 80 % | Variants, model diff |
| Signal block library | ≈ 55 % | ≈ 60 % | ≈ 65 % | Breadth, n-D tables, data types |
| Simulation engine | ≈ 65 % | ≈ 70 % (algebraic loops, real-time pacing) | ≈ 80 % (DAEs) | Compiled speed |
| Motors and drives | ≈ 70 % | ≈ 75 % (drive tuning) | ≈ 85 % (FFT/THD, SVPWM, losses, saturation) | Semiconductor detail |
| Results and scopes | ≈ 75 % | ≈ 75 % | ≈ 80 % | MAT/HDF5 |
| Control design and analysis | ≈ 25 % | **≈ 75 %** | ≈ 75 % | Optimisation, estimation (0.6.x) |
| Physical modelling | 0 % | 0 % | ≈ 40 % | Library breadth |
| Exchange and automation | ≈ 35 % (CLI, Python) | ≈ 35 % | ≈ 35 % | FMU, building models from code (0.5.x) |
| Code generation, hardware in the loop, certification | 0 % | 0 % | 0 % | Not planned |

**Where we lead today:** time to first result, install size, price (free, MIT), one-click machine ratings from LV to HV, published checks of every machine and solver, a modern consistent UI, Python inside without setup.

**Where we trail most:** physical networks, state machines, whole-model control design, library breadth, FMU exchange, and the whole code-generation and hardware world.

---

## 6. Do we stand a chance?

**Short answer: not as a replacement for the industry standard, and we should not try to be one. Yes, as the best free tool for a clear group of people.**

### What the industry standard really sells
Its editor is not the hard part to match. What keeps companies on it is:
- **Code generation with certification kits** (ISO 26262 for cars, DO-178C for aircraft, IEC 61508). Teams ship production code from their models. This is decades of work and legal qualification.
- **Hardware in the loop and real-time targets**, sold with partners' hardware.
- **Forty years of libraries and add-ons**, and the MATLAB language around them.
- **People**: every control engineer learns it at university, so companies hire for it and train in it. Support contracts and consultants come with it.

An open-source project with one developer cannot cross those walls, and a cheaper editor will not make an automotive or aerospace company switch. Saying so honestly is part of being trusted.

### Where open source has won in engineering, and why
- **KiCad** (circuit boards) went from hobby to professional use in about 15 years, with institutional backing and a community.
- **Blender** (3D) and the **Python scientific stack** (which took much of academia and data work from MATLAB) won by being free, good enough for most daily work, scriptable, and open.
- **OpenModelica** holds research and teaching through an open standard.

The pattern: they win **where price matters most** (students, universities, small firms, many countries), **where "good enough for 80 % of everyday work" is enough**, **where open formats and scripting let them fit next to the paid tools**, and **once a community forms**.

### Our chance
- The free tools today are hard (Xcos, OpenModelica) or windowless (Python libraries) — §4. A free, modern, easy and *checked* block-diagram simulator is missing.
- Datum Sim already has the unusual parts: a strong motor and drive library with ratings, public verification, a small installer, Python and a command line.
- **Realistic target users:** students and teachers of control, electrical drives and dynamics; small engineering firms and consultants; drive and control engineers doing everyday studies (sizing, tuning, "what if") where production code is not needed; researchers who want Python around their models.
- **Realistic goal:** "the first free tool people reach for to learn, sketch and check a dynamic system or a drive", working *beside* the paid tools (through FMU, CSV and Python), not instead of them.

### The risks, honestly
- **One developer** (with AI help): the project's continuity depends on one person. Contributors and clear docs reduce this.
- **Windows only** as an installer: much of teaching and research uses Linux and macOS.
- **Library breadth**: 85 blocks against thousands.
- **No users yet** giving feedback, and no tutorials or course material.
- **Trust**: engineers need proof before they rely on numbers. Our public verification records are the right answer and must keep growing.

### What would change the odds the most
1. **Finish control design** (whole-model linearisation, tuning): the everyday work of the target users.
2. **Go deeper on drives**, where we are already strong: FFT/THD, SVPWM, losses, saturation.
3. **Physical components**, so mechanical, electrical and thermal parts can be drawn as they are.
4. **FMU exchange**, so Datum Sim fits into teams that use the paid tools.
5. **Linux and macOS installers**, tutorials, and a contributor guide.
6. **A public benchmark suite** comparing our results with closed-form answers and published reference results.

None of these needs code generation or hardware in the loop, which stay out of scope.
