# Datum Sim: plan for the 0.1.x releases (and the road to 0.2)

> Written in session 4 (2026-10-28), after 0.1.0 was released. Shape (FAMILY §10): what we know → brainstorm → release steps → questions. Each step is one small release through the in-app updater. **Build after the user's answers.**

## 1. Where 0.1.0 stands

- Works and is tested: the editor, 45 blocks, motors, subsystems, five solvers, live scopes, sweeps, Python analyses, the family window. CI green on three systems; installer 85.6 MB.
- Not yet seen on a real Windows PC: installer, first Python start, print to PDF, updater. **The user's Windows test feeds 0.1.1 first**, as in Plan.
- Rough edges found while using it under Xvfb (session 2), not fixed yet:
  - wires: no branch dots when one output feeds several inputs; feedback wires can bunch up; bend points can only be added, not moved;
  - when the scopes dock opens on the first run, the bottom of the diagram is hidden;
  - scopes: no measuring cursors, no y-zoom, no auto-scale button;
  - a bad parameter shows only when you press Run;
  - the hover tooltip on blocks covers the diagram;
  - print preview not checked page by page.

## 2. Brainstorm

### A. Comfort while building (small, high value)
1. **Wire branch points**: a dot where a wire splits; drag a branch from the middle of a wire to a new input (as people expect).
2. **Move wire segments**: drag a straight segment sideways; double-click a bend to remove it.
3. **Signal names on wires**: name a signal once ("speed"); the name shows on the wire and in scope legends.
4. **Live parameter checks**: the inspector shows "unknown name Kx" under the field as you type, before Run.
5. **Text notes on the diagram**: a note box for explanations (teaching, reviews).
6. **Find a block** (Ctrl+F) across subsystems; **distribute** blocks evenly; **align to grid**.
7. Block colour (to mark plant, controller, sensors).

### B. Seeing results
1. **Scope cursors**: two vertical cursors with t, y and Δt, Δy readouts; peak and value markers.
2. **Scope settings**: y-range or auto-scale, one plot per input (stacked), line styles, legend on or off.
3. **Signal probe**: after a run, hover any wire to read its value at the cursor time (no scope needed). Needs the engine to log every signal (an option: "log all signals").
4. **Keep and compare runs**: the last few runs kept; overlay "run 3 vs run 5" in a scope.
5. **Publication export**: a scope as SVG or a high-resolution PNG at a chosen size.
6. **Step-response lines on the scope**: rise time, overshoot and settling marked on the plot.

### C. More blocks (pulled forward from 0.2 where they are small)
- **Signal sources**: square, triangle, chirp (frequency sweep), PWM, repeating sequence.
- **Discrete**: discrete transfer function, discrete integrator, discrete PID, rate transition; sample-time colours on blocks.
- **Integrator reset** (external reset input), dynamic saturation, Coulomb and viscous friction, hit crossing.
- **Goto / From** (send a signal by name without a wire).
- **Math function** (exp, log, sqrt, square, reciprocal) and trigonometry as blocks, not only in Function.

### D. Making subsystems reusable
- **Masks**: a subsystem gets its own parameters (R, L, Kt …) shown in the inspector, an icon or text on the block, and a ? card. The inside uses the mask parameters.
- **My blocks (user library)**: save a subsystem to a library file; it appears in the Library panel for every model.
- **Linked or copied**: decide whether library blocks update when the library changes (linked) or stay as copied.

### E. Motors and drives (the user's interest)
- **Clarke and Park transforms** (abc ↔ αβ ↔ dq) and their inverses.
- **Induction motor** (d–q model) with a speed or V/f example.
- **BLDC motor** with six-step commutation, and Hall sensors.
- **Three-phase inverter (average model)** and a **PWM** source; DC link.
- **Encoder** (quantised angle, speed estimate) and a **current sensor** with noise and delay.
- An example pack: FOC of a PMSM with Park/Clarke on real three-phase signals; V/f induction motor; BLDC six-step.

### F. Working faster
- **Live tuning**: drag a slider on a gain or PID while the model runs; the scopes respond.
- **Command line**: `datum-sim run model.dsim --out results.csv` for batch runs and CI.
- **Faster big models**: draw only the visible blocks; route wires only around moved blocks.

### G. Trust
- **Verification report** inside the app (Help › Verification), printable, from `docs/verification`.
- More published problems (HIRES, Andrews' squeezer, transistor amplifier from the Bari test set).
- Mutation testing of the solvers (cargo-mutants), nightly.

## 3. Proposed steps

| Release | Theme | Contents | Size |
| --- | --- | --- | --- |
| **0.1.1** | First-use fixes | Fixes from the Windows test; dock opening re-fits the diagram; tooltips only after a pause; live parameter checks in the inspector; wire branch dots; print preview checked | Small |
| **0.1.2** | Wires and diagram | Drag wire segments, remove bends, branch from the middle of a wire; signal names on wires and in legends; text notes; find block; distribute | Small–medium |
| **0.1.3** | Scopes | Two cursors with Δ readouts, y-range and auto-scale, stacked plots, keep and compare runs, SVG / high-resolution PNG, step-response lines | Medium |
| **0.1.4** | Blocks | Square, triangle, chirp, PWM; discrete TF, integrator and PID; rate transition; integrator reset; Goto/From; math function; friction | Medium |
| **0.1.5** | Motors and drives | Clarke/Park, induction motor, BLDC with Hall sensors, average inverter, encoder and current sensor; three new motor examples | Medium |
| **0.1.6** | Live tuning and command line | Sliders on parameters while running; `datum-sim run` from a terminal | Medium |
| **0.2.0** | Reusable models | Masks, My blocks (user libraries), buses, algebraic-loop solving, signal probe ("log all signals"), in-app verification report | Large |

Then 0.3 control design (linearise whole models, Nyquist, root locus, PID auto-tuning), 0.4 physical components, 0.5 FMU, 0.6 studies, as in the roadmap.

**Each step** follows FAMILY §11: engine first with tests against known answers, UI checks for every new feature and bug, a real-app check under Xvfb, then a release. CHANGELOG in plain words.

## 4. Questions for the user

1. **Windows test first?** Shall 0.1.1 wait for your screenshots from a Windows PC, or start now with the known fixes and add your findings to 0.1.2?
2. **Order after 0.1.1**: the plan puts wires (0.1.2) and scopes (0.1.3) before new blocks and motors. Would you rather have **motors and drives (0.1.5) earlier**?
3. **Motors**: which matter most to you: induction motor, BLDC six-step, the three-phase inverter with PWM, or full FOC with Park/Clarke on three-phase signals?
4. **Live tuning** (sliders while running): worth it in 0.1.x, or later?
5. **Masks and user libraries**: should library blocks be **linked** (change the library, every model updates) or **copied** (each model keeps its own copy)? Linked is more powerful; copied is simpler and safer.
6. **Discrete blocks** (0.1.4): do you need them soon for real controller work (sample times, discrete PID), or can they wait for 0.2?

---

## 5. Answers (session 4, 2026-10-28) and the new order

| Question | Answer | Effect |
| --- | --- | --- |
| 1 Windows test | "It works, very simple, nothing much to do, but there is no drag and drop from the library to the workspace" | 0.1.1 now: the library drag fixed (see below) plus the known first-use fixes |
| 2 Order | "As you please" | Motors move up (the user's main interest), then discrete blocks (needed) |
| 3 Motors | "All motors: LV, MV, HV, synchronous, asynchronous, AC, DC, all types" | 0.1.2 becomes a full motor library (§6) |
| 4 Live tuning | Later | Moves after 0.2 |
| 5 User libraries | "Whichever is simpler, safer and faster" | **Copied**: a library block is copied into the model; changing the library later does not change saved models |
| 6 Discrete blocks | Needed | 0.1.3 |

**Cause of the drag bug**: on Windows, Tauri's window catches drag-and-drop for dropping files (`dragDropEnabled`, on by default), so HTML drag-and-drop from the library never reached the diagram. Linux (where 0.1.0 was checked) does not do this. Fixed both ways: the setting is off, and the library drag now uses plain mouse events with a ghost block, which behaves the same on every system and is covered by a UI test.

| Release | Theme |
| --- | --- |
| **0.1.1** | Library drag and drop fixed; parameter errors shown while editing; the diagram re-fits when the scopes dock opens; dots where wires split |
| **0.1.2** | Motor library (§6) with examples |
| **0.1.3** | Discrete control: discrete transfer function, integrator, PID, rate transition, sample-time colours; integrator reset; Goto/From |
| **0.1.4** | Wires and diagram (segments, signal names, notes, find) |
| **0.1.5** | Scopes (cursors, y-zoom, stacked plots, compare runs, SVG) |
| **0.2.0** | Masks and copied user libraries, buses, algebraic loops, signal probe |

## 6. The motor library (0.1.2), first draft (built in session 5; see MEMORY for what differs)

All motors as d–q (or equivalent) models with mechanical outputs and a common load-torque input, parameters in SI or per unit, with **rating presets** (low, medium and high voltage examples, e.g. 400 V / 11 kW, 3.3 kV / 500 kW, 11 kV / 5 MW) that fill in typical values the user can change.

| Family | Blocks |
| --- | --- |
| DC | Permanent-magnet (exists), separately excited (field circuit), shunt, series, compound |
| Induction (asynchronous) | Squirrel-cage (d–q, with saturation-free magnetising branch), double-cage, wound-rotor / doubly fed (rotor voltages as inputs) |
| Synchronous | Surface and interior PMSM (exists, add Ld ≠ Lq presets), wound-field salient-pole with damper windings (MV/HV machines, field voltage input), synchronous reluctance |
| Other | BLDC (trapezoidal back-EMF, Hall outputs), stepper (hybrid), switched reluctance (later) |
| Drive pieces | Clarke and Park transforms (and inverses), three-phase sine source, average-model inverter, PWM, DC link, encoder, current sensor |
| Examples | V/f induction motor start, induction motor direct-on-line start (current and torque transients), PMSM FOC with three-phase signals, BLDC six-step, a large synchronous motor on a stiff grid with a load step |

Verification: steady-state torque–speed curves against the equivalent-circuit formulas, no-load speed, locked-rotor current, and power balance (input electrical power = mechanical power + losses) for every machine.
