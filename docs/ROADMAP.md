# Datum Sim: plan and roadmap from 0.2.1

Written 2026-11-07 (session 9), after the comparison (`docs/COMPARISON.md`). Status: **agreed 2026-11-10** ("go ahead, A and B only"); stages A (0.2.2 → 0.3.0) and B (0.3.1 → 0.4.0) built and released 2026-11-10, stage R (robotics, 0.4.1 → 0.5.0) on 2026-11-11; stage C (0.5.x) next, with its own plan and questions. Defaults used for the questions in §6: all items, a release per patch, sliders for tunable variables, two-slider PID tuner with Ziegler–Nichols and SIMC, drive tuning, the DC motor, tank and PMSM as examples.

## 1. Where we are

- **Released:** 0.2.1 (2026-11-02), through the in-app updater. It includes the editor, 85 blocks, 14 motor types with LV–HV ratings, discrete control, five solvers, scopes with cursors and run comparison, masks and My blocks, the command line and the Python API.
- **Strong:** ease and install size, motors and drives, public verification, scripting.
- **Weak:** control design is limited to one block at a time. There are no physical (port-based) components, no state machines, no FMU exchange, and the installer is Windows only.
- **Not verified yet on a real Windows PC:** the installed command line, `pip install` from GitHub, and most features since 0.1.1.

## 2. How we plan

1. **Small numbered steps, no jumps.** Each patch (0.2.2, 0.2.3, …) is one focused piece that works on its own, is released through the updater, and has its tests and a verification record. A minor number (0.3.0, 0.5.0, …) closes a stage. It is a milestone, not a jump.
2. **No race to 1.0.** 1.0 means "validated and documented for everyday professional use". It comes when the stages below make it true, not on a date.
3. **Fixes take the next number.** If a bug or a user request comes in, it ships as the next patch, and the planned items move down one number. The order stays; the numbers are only labels.
4. **One stage at a time.** Each stage gets a short plan with questions before it starts (as PLAN-0.3 does), and the user's answers decide its details.
5. **Every step keeps the house rules:** engine logic in `crates/core` with tests against known answers; a UI check for everything visible; a real-app check; CHANGELOG, site.json and MEMORY kept true.
6. **The goal** (COMPARISON §6): the first free tool people reach for to learn, sketch and check a dynamic system or a drive, working beside the paid tools. Code generation, hardware in the loop and certification are **out of scope**.

## 3. The stages

### Stage A: control design (0.2.2 → 0.3.0)
Details are in `docs/PLAN-0.3.md` (its release numbers are replaced by these). It makes the weakest everyday area (≈ 25 %) the strongest (≈ 75 %).

| Release | What it adds | Done when |
| --- | --- | --- |
| 0.2.2 | **Analysis points** on wires (input, output, loop opening). **Linearise the whole model** at the initial state or at a time in the last run. **Bode and margins of the real loop.** | A mass–spring–damper's A, B, C, D come out exactly, and the DC motor loop's margins match a hand calculation |
| 0.2.3 | **Nyquist** (with the stability verdict), **root locus**, **sensitivity** (S, T, peak Ms), step and impulse of the open and closed loop | They match python-control on textbook loops |
| 0.2.4 | **Trim**: find operating points (the tank at 2 m, the motor at 1500 rpm). Save and name operating points. | The known equilibria are found |
| 0.2.5 | **PID tuner**: two sliders (speed and robustness), before-and-after plots, Ziegler–Nichols and SIMC, one undoable Apply | The tuned loop meets its targets |
| 0.2.6 | **Drive tuning**: one click tunes a whole drive's current loop and speed loop (from L, R and inertia), for DC and FOC drives | The cascade meets the set bandwidths on the DC and PMSM examples |
| 0.2.7 | **Live tuning**: sliders for tunable variables while the model runs; real-time pacing (×0.1 to ×10) | A gain changed at 1 s matches a Step block at 1 s |
| 0.2.8 | **Buses** (named signal groups) and **algebraic loops solved** instead of reported | The bus and loop examples run and their results match the unbussed or hand-solved versions |
| **0.3.0** | **Milestone:** control-design examples (DC motor, tank, PMSM drive), help pages, and the stage's verification record | Every check passes; the stage is documented |

### Stage B: drives in depth (0.3.1 → 0.4.0)
Where we are already strong, made stronger. This is what drive engineers check daily.

| Release | What it adds |
| --- | --- |
| 0.3.1 | **FFT and THD** on scopes (spectrum of a signal over chosen cycles) |
| 0.3.2 | **Space-vector PWM** and **dead time** in the inverter |
| 0.3.3 | **Inverter losses** (conduction and switching from data-sheet values) and a **thermal network** (junction, case, heat sink): one release, as they work together |
| 0.3.4 | **Machine saturation** (magnetising curve) and iron losses for the induction machine |
| 0.3.5 | **Parameters from the nameplate**: estimate the equivalent circuit from rated data |
| 0.3.6 | **Modules** and the **robot arm module**: a two-link arm, trajectory, joint controllers, an animated view (added 2026-11-10 at the user's request) |
| 0.3.7 | **Drive examples**: losses and junction temperature, a harmonics study, a saturated start |
| **0.4.0** | **Milestone:** verification record, docs |

### Stage R: robotics (0.4.1 → 0.5.0)
Added 2026-11-11 at the user's request, before stage C, for both a 6-joint and a 7-joint arm. Details in `docs/PLAN-robotics.md`: any serial arm (0.4.1), URDF import and the robot library (0.4.2), 3D view (0.4.3), inverse kinematics and tool paths (0.4.4), controllers and motors on joints (0.4.5), gripper and contact (0.4.6), task scheduler (0.4.7), pick-and-place examples (0.4.8), milestone 0.5.0.

### Stage C: physical components (0.5.1 → 0.6.0)
Draw mechanical, electrical and thermal parts as they are, connected by ports instead of signals. It is the biggest engine change, so it gets its own plan with questions first.

| Release | What it adds |
| --- | --- |
| 0.5.1 | Engine: **equations from connected components** (DAEs), with a solver for them (BDF), and **rotational mechanics** (inertia, spring, damper, gear, friction) |
| 0.5.2 | **Translational mechanics** (mass, spring, damper, hard stop) |
| 0.5.3 | **Electrical circuits** (R, L, C, sources, ideal switch, diode, transformer) |
| 0.5.4 | **Thermal** (capacity, conduction, convection) |
| 0.5.5 | **Motors on ports**: the motor blocks connect to electrical and mechanical networks |
| 0.5.6 | **Basic hydraulics** (pump, valve, cylinder, pipe) |
| **0.6.0** | **Milestone:** physical examples and verification record |

### Stage D: exchange (0.6.1 → 0.7.0)
Fit beside other tools.

| Release | What it adds |
| --- | --- |
| 0.6.1 | **FMU export** (co-simulation): run a Datum Sim model inside other tools |
| 0.6.2 | **FMU import**: run other tools' models as a block |
| 0.6.3 | Results as **MAT and HDF5**; **build models from Python** (add blocks and wires) |
| 0.6.4 | **Modelica models** through an external compiler (if one with a fitting licence is installed) |
| **0.7.0** | **Milestone** |

### Stage E: studies and logic (0.7.1 → 0.8.0)
| Release | What it adds |
| --- | --- |
| 0.7.1 | **State machines** (states, transitions, actions) |
| 0.7.2 | **Python block**: a block whose behaviour is a Python function |
| 0.7.3 | **Monte Carlo** and **sensitivity** studies |
| 0.7.4 | **Optimisation** and **parameter fitting** to measured data |
| 0.7.5 | **Report builder**: a model, its runs and its plots in one printable report |
| **0.8.0** | **Milestone** |

### Stage F: reach and speed (0.8.1 → 0.9.0)
| Release | What it adds |
| --- | --- |
| 0.8.1 | **Linux and macOS installers** (CI already builds and tests on both) |
| 0.8.2 | **Faster runs** for big models (a compiled run mode) |
| 0.8.3 | **Variants** (switchable subsystems) and **model comparison** (what changed between two files) |
| 0.8.4 | **Tutorials** for teaching (a short course: dynamics, control, drives), and a **contributor guide** |
| **0.9.0** | **Milestone** |

### After that
The way to **1.0**: a public benchmark suite against closed forms and published reference results, a complete user guide, and a signed installer. The order of stages E and F can swap. Tutorials and Linux/macOS (stage F) could also come much earlier if wanted (question 3).

## 4. What is not planned
Production code generation, hardware in the loop and real-time targets, certification kits, fixed-point data types, and requirements management. These belong to a different class of tool (COMPARISON §6).

## 5. Size and pace
- Each patch is one focused piece, about the size of 0.1.4–0.1.6.
- Stage A is 7 patches plus its milestone. Stages B, D and E are similar in size. Stage C is the largest, because of the new engine.
- If a patch turns out larger than planned, it is split (0.2.4 becomes 0.2.4 and 0.2.5) rather than shipped late.

## 6. Questions for the user

1. **Goal:** do you agree with the goal in §2.6 and the target users in COMPARISON §6? These are students and teachers, small firms, and drive and control engineers doing everyday studies, working beside the paid tools rather than replacing them.
2. **Order of stages:** A control design → B drives → C physical → D exchange → E studies → F reach? Or should drives (B) come before control design (A)?
3. **Earlier reach:** should Linux/macOS installers and tutorials (stage F) move up, for example right after 0.3.0?
4. **Stage A details:** PLAN-0.3 §7 has six questions. If you are happy with my defaults, answer "your defaults". They are:
   - all of A–F, including buses and algebraic loops (as 0.2.8);
   - a release per patch;
   - sliders for marked variables;
   - the two-slider PID tuner plus Ziegler–Nichols and SIMC;
   - yes to drive tuning (0.2.6);
   - the DC motor, tank and PMSM drive as examples.
5. **Before 0.2.2:** could you try 0.2.1 on Windows (the command line `datum-sim-cli run example:dcmotor` in the app's folder, and `pip install` of the Python API)? Anything found becomes 0.2.2, and the plan moves down one number.
