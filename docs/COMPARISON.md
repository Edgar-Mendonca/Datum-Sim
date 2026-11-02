# Datum Sim: comparison with industry-standard tools

Written 2026-11-02, at **Datum Sim 0.2.0**. This is an internal planning document, in the family pattern (Datum Plot's PLAN-0.1 §1–2, Datum Plan's BRAINSTORM §3): how the tools think → feature tables → where we lead and trail → what it means for the roadmap. Product names appear here only to describe features. They never appear in the app's public words (README, CHANGELOG, site.json, help; FAMILY §13).

**Compared with:**
- **Simulink**, with its usual add-ons: Simscape, Stateflow, Simulink Control Design, Embedded Coder. It is the industry standard for block-diagram simulation and model-based design.
- **PLECS**: the reference tool for power electronics and motor drives. That is the closest match to Datum Sim's motor focus.
- **Scilab Xcos** and **OpenModelica**: the two established open-source tools.

Facts about the other tools come from their public documentation and common knowledge. Exact features depend on version and licence.

---

## 1. Where Datum Sim is (0.2.0)

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
  - 69 engine tests against closed forms (equivalent circuits, power balance, difference equations).
  - 60 UI checks and real-app checks under Xvfb.
  - A dated verification record per release.
  - Cargo deny on every dependency.
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

✓ full · ◐ partial · — none. The **Datum Sim** column says what we have and the release it came in, or the planned release (0.3 control design, 0.4 physical components, 0.5 exchange, 0.6 studies, 1.0 validated), or "not planned".

### 3.1 Editor and modelling

| Feature | Simulink | PLECS | Xcos | OpenModelica | **Datum Sim** |
| --- | --- | --- | --- | --- | --- |
| Block diagram editor | ✓ | ✓ | ✓ | ✓ | ✓ 0.1.0 |
| Wires that route themselves | ✓ | ◐ | ◐ | ◐ | ✓ 0.1.0 |
| Branches, segment editing, signal names on wires | ✓ | ✓ | ◐ | ◐ | ✓ 0.1.6 |
| Subsystems, nested | ✓ | ✓ | ✓ | ✓ | ✓ 0.1.0 |
| Masks (own parameters, icon, help) | ✓ | ✓ | ◐ | ✓ (models are parameterised) | ✓ 0.2.0 (text icon only) |
| User libraries | ✓ (linked) | ✓ | ✓ (palettes) | ✓ (packages) | ✓ 0.2.0 (copied, by choice) |
| Variants (switchable subsystems) | ✓ | ✓ | — | ✓ (replaceable) | — not planned yet |
| Buses (named signal groups) | ✓ | ◐ | — | ✓ (connectors) | 0.3 (question 1) |
| Goto/From tags | ✓ | ✓ | ✓ | n/a | ✓ 0.1.4 |
| Notes, colours, Find | ✓ | ✓ | ◐ | ◐ | ✓ 0.1.6 |
| Model checker (unconnected, unused, suspicious settings) | ✓ (Model Advisor) | ◐ | — | ◐ | ◐ (errors and warnings, Issues page) |
| Model comparison and merge (diff) | ✓ | — | — | ◐ | — (idea in PLAN-0.1) |
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
| Python / script blocks | ✓ (MATLAB, Python) | ✓ (C) | ✓ (Scilab, C) | ✓ (external functions) | 0.6 (Python block) |
| State machines | ✓ (Stateflow) | ✓ (state machine block) | ◐ | ✓ (state graphs) | 0.6 |
| Physical networks: mechanical, electrical, hydraulic, thermal | ✓ (Simscape) | ✓ electrical, thermal, mechanical | ◐ (via Modelica) | ✓ (core strength) | 0.4 |
| Total size of the library | thousands | hundreds | hundreds | thousands (MSL) | 85 |

### 3.3 Simulation engine

| Feature | Simulink | PLECS | Xcos | OpenModelica | **Datum Sim** |
| --- | --- | --- | --- | --- | --- |
| Fixed-step solvers | ✓ (many) | ✓ | ✓ | ✓ | ✓ Euler, Heun, RK4 |
| Variable-step explicit | ✓ | ✓ | ✓ | ✓ | ✓ Dormand–Prince 5(4) |
| Stiff solvers | ✓ (ode15s, ode23s, ode23tb…) | ✓ | ✓ (LSodar, IDA) | ✓ (DASSL, IDA…) | ✓ Rosenbrock 2(3); BDF/IDA with DAEs in 0.4 |
| DAEs (algebraic constraints) | ✓ (Simscape) | ✓ | ✓ | ✓ | 0.4 |
| Exact event location (zero crossings), hits at discontinuities | ✓ | ✓ (switching events) | ✓ | ✓ | ✓ 0.1.0 |
| Algebraic loops solved | ✓ | ✓ | ◐ | ✓ (tearing) | — reported only; 0.3 (question 1) |
| Multirate discrete, sample-time colours | ✓ | ✓ | ◐ | ◐ | ✓ 0.1.4 |
| Vector signals, width inference | ✓ (and matrices, buses, fixed point) | ✓ | ✓ | ✓ | ✓ 0.1.0 (vectors) |
| Fixed-point and integer data types | ✓ | — | ◐ | ◐ | — not planned |
| Speed: large models, accelerator / compiled mode | ✓ (accelerator, rapid accelerator) | ✓ (fast) | ◐ | ✓ (compiled C) | ◐ (Rust engine, interpreted blocks; no compiled mode) |
| Run in real time, pause, step | ✓ (pacing, step back) | ✓ | ◐ | ◐ | ◐ pause and step ✓; real-time pacing 0.3 |

### 3.4 Motors, power electronics and drives

| Feature | Simulink (Simscape Electrical) | PLECS | Xcos | OpenModelica (MSL Electrical) | **Datum Sim** |
| --- | --- | --- | --- | --- | --- |
| DC machines (PM, separately excited, shunt, series, compound) | ✓ | ✓ | ◐ | ✓ | ✓ 0.1.2 |
| Induction (cage, double cage, wound rotor) | ✓ | ✓ | — | ✓ | ✓ 0.1.2 |
| Synchronous (PMSM surface/interior, reluctance, wound field with dampers) | ✓ | ✓ | — | ✓ | ✓ 0.1.2 |
| BLDC, stepper, switched reluctance | ✓ | ✓ | — | ◐ | ✓ 0.1.2, SRM 0.1.4 |
| Saturation, iron losses, temperature effects in machines | ✓ | ✓ | — | ◐ | — later (not yet planned) |
| Typical ratings LV/MV/HV with one click | ◐ (parameter sets per block) | ◐ | — | — | ✓ 0.1.2 (a strength) |
| Inverter: average model and switched with exact switching | ✓ | ✓ (core strength) | — | ◐ | ✓ 0.1.2 |
| Semiconductor-level switching (diodes, IGBT models, dead time, snubbers) | ✓ | ✓ | — | ✓ | 0.4 (with electrical components) |
| Converter losses and thermal models | ✓ | ✓ (core strength) | — | ◐ | — later |
| Clarke/Park, PWM, SVPWM, encoder, sensors | ✓ | ✓ | — | ◐ | ✓ (SVPWM — later) |
| Ready drive examples (DOL, V/f, FOC, BLDC, grid synchronous, SRM) | ✓ | ✓ | — | ◐ | ✓ 0.1.2–0.1.4 |
| Machine parameters from nameplate / test data (parameter estimation) | ✓ | ◐ | — | — | — 0.6 (fitting) |
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
| FFT / spectrum, THD | ✓ | ✓ (core for drives) | ◐ | ◐ | — later (with Datum Plot hand-over today) |
| Export: CSV / MAT / HDF5 | ✓ | ✓ | ✓ | ✓ | ◐ CSV ✓; HDF5 planned |
| Hand-over to a plotting app | ✓ (MATLAB) | ◐ | ✓ (Scilab) | ✓ | ✓ Datum Plot |

### 3.6 Control design and analysis

| Feature | Simulink (+ Control Design) | PLECS | Xcos | OpenModelica | **Datum Sim** |
| --- | --- | --- | --- | --- | --- |
| Bode, margins, step, poles/zeros of a block | ✓ | ✓ | ✓ (Scilab) | ◐ | ✓ 0.1.0 |
| Linearise a whole model between analysis points | ✓ | ✓ (small-signal analysis) | ◐ | ✓ | 0.3 |
| Trim / operating points | ✓ | ◐ (steady-state analysis) | ◐ | ◐ | 0.3 |
| Nyquist, root locus, sensitivity | ✓ | ◐ | ✓ (Scilab) | ◐ | 0.3 |
| PID auto-tuning | ✓ | — | — | — | 0.3 |
| Live tuning while running | ✓ | ◐ | — | — | 0.3 |
| Parameter sweeps | ✓ | ✓ | ◐ | ✓ | ✓ 0.1.0 |
| Monte Carlo, sensitivity, optimisation | ✓ (Design Optimization) | ◐ | ◐ | ◐ | 0.6 |
| Parameter estimation from measured data | ✓ | — | ◐ | ◐ | 0.6 |

### 3.7 Exchange, automation, deployment

| Feature | Simulink | PLECS | Xcos | OpenModelica | **Datum Sim** |
| --- | --- | --- | --- | --- | --- |
| FMU import and export (FMI) | ✓ | ✓ | ◐ | ✓ | 0.5 |
| Modelica models | ◐ | — | ✓ | ✓ | 0.5 (external compiler) |
| Scripting / API (build, run, analyse models from code) | ✓ (MATLAB) | ✓ (RPC, scripting) | ✓ (Scilab) | ✓ (OMPython, scripts) | — **gap: not on the roadmap yet** |
| Command line / batch runs, CI | ✓ | ✓ | ✓ | ✓ | — (idea in PLAN-0.1; suggest adding) |
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

## 4. Where we stand, in one table

A rough coverage of the industry standard's **everyday** feature set, by area (my estimate):

| Area | Coverage at 0.2.0 | After 0.3 | Main gap after that |
| --- | --- | --- | --- |
| Editor and modelling | ≈ 75 % | ≈ 80 % (buses) | Variants, model diff |
| Signal block library | ≈ 55 % | ≈ 60 % | Breadth, n-D tables, data types |
| Simulation engine | ≈ 65 % | ≈ 70 % (algebraic loops, real-time pacing) | DAEs (0.4), compiled speed |
| Motors and drives | ≈ 70 % (presets ahead) | ≈ 70 % | Semiconductor switching, losses and thermal, saturation |
| Results and scopes | ≈ 75 % | ≈ 75 % | FFT/THD, HDF5 |
| Control design and analysis | ≈ 25 % | **≈ 75 %** | Optimisation, estimation (0.6) |
| Physical modelling | 0 % | 0 % | 0.4 |
| Exchange and automation | ≈ 10 % | ≈ 10 % | **Scripting and command line**, FMU (0.5) |
| Code generation, HIL, verification tools | 0 % | 0 % | Not planned |

**Where we lead today:**
- Time to first result.
- Install size.
- Price (free, MIT).
- One-click machine ratings from LV to HV.
- Published checks of every machine and solver.
- A modern, consistent family UI.
- Python inside without setup.

**Where we trail most:**
1. Physical networks (0.4).
2. State machines (0.6).
3. **Scripting and batch runs**, which every tool in the table has and our roadmap does not.
4. Code generation and hardware, a separate world we should not chase before 1.0.

---

## 5. What this suggests for the roadmap

- **0.3 control design is the right next step.** It moves the weakest everyday area (≈ 25 %) to about 75 %, and it is what our target users (control and drive engineers) do daily.
- **Add a small "automation" release**, either before 0.3 or alongside it:
  - A command line (`datum-sim run model.dsim --set Kp=2 --out results.csv`).
  - A Python API to open, change, run and read models, sharing the engine through the bundled Python.
  - This closes the one gap every compared tool covers, and it enables CI and batch studies.
- **For drives**, after 0.3:
  - FFT and THD on scopes.
  - SVPWM.
  - Dead time and simple conduction and switching losses on the inverter.
  - This is a smaller step than full semiconductor models (0.4) and matters most to drive engineers.
- **Keep code generation, HIL and certification out of scope** until 1.0. State this honestly in the README's "what it does" (without naming any product).

These are proposals: nothing changes in PLAN-0.3 until the user decides.
