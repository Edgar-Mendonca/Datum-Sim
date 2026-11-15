# Datum Sim: roadmap

First written 2026-11-07 after the comparison (`docs/COMPARISON.md`); **rewritten 2026-11-15 at 0.5.1** after the user's review:
- the motto (§2.6);
- a new scripting stage (S);
- a new ease-of-use stage (U);
- the later stages renumbered.

## 1. Where we are (0.6.0)

- **Done (0.6.0, 2026-11-15):**
  - the basics (editor, solvers, scopes, 14 motor types);
  - stage A, control design (0.3.0);
  - stage B, drives in depth (0.4.0);
  - stage R, robotics (0.5.0);
  - stage C, physical parts (0.6.0): rotational, translational, electrical, thermal and hydraulic parts, motors on ports, six examples.
- **Strong:** a first result in minutes; control design built in; motors and drives; robot arms; public verification; scripting from outside (command line and Python API).
- **Weak:**
  - no scripting inside the window;
  - **the app is hard to find your way around** (the user, 2026-11-15);
  - no FMU exchange or state machines;
  - Windows only.

## 2. How we plan

1. **Small numbered steps, no jumps.** Each patch is one focused piece that works on its own and reaches users through the updater, with its tests. A minor number (0.6.0, 0.7.0 …) closes a stage and brings its verification record.
2. **No race to 1.0.** 1.0 means "validated and documented for everyday professional use".
3. **Fixes take the next number;** planned items move down one. The order stays; the numbers are labels.
4. **One stage at a time,** each with a short plan and questions first. The user's answers decide the details.
5. **House rules every step:**
   - engine logic in `crates/core`, tested against known answers;
   - a UI check for everything visible, and a real-app check;
   - CHANGELOG, site.json and MEMORY kept true.
6. **The motto:** *Draw a dynamic system, run it and trust the result: free, open, and no coding needed.*
   - Everything can be done with the mouse. Python is there for those who want it, never required.
   - Built the Datum way from free, open parts. We copy no one and replace no one; we work beside other tools.
   - Every block is checked against a known answer, in public.
   - MIT licence: free for students, teachers, small firms and companies alike.
   - Production code generation, hardware in the loop and certification are **out of scope**.

## 3. The stages

| Stage | Versions | What | Status |
| --- | --- | --- | --- |
| Basics | 0.1.0 → 0.2.1 | Editor, solvers, scopes, motors, masks, command line and Python API | ✅ |
| A | 0.2.2 → 0.3.0 | Control design: analysis points, linearisation, Nyquist, root locus, trim, PID and drive tuning, live tuning, buses, algebraic loops | ✅ |
| B | 0.3.1 → 0.4.0 | Drives in depth: FFT/THD, SVPWM, dead time, losses, thermal, saturation, nameplate, robot arm module | ✅ |
| R | 0.4.1 → 0.5.0 | Robotics: any arm, URDF, 3D view, IK, controllers, gripper, scheduler, pick and place | ✅ |
| C | 0.5.1 → 0.6.0 | Physical parts: rotational, translational, electrical, thermal, motors on ports, hydraulics, examples | ✅ |
| U1 | 0.6.1 | **Easy to find your way, first step**: tabs in working order, one home per tool, Modules by area, the library in families, examples by topic, Where Is…? | ✅ |
| **S** | 0.6.2 → 0.7.0 | **Scripting, the Datum way** | **next** |
| **U** | 0.7.1 → 0.8.0 | **Easy to find your way** | then |
| D | 0.8.1 → 0.9.0 | Exchange with other tools | later |
| E | 0.9.1 → 0.10.0 | Studies and logic | later |
| F | 0.10.1 → 0.11.0 | Reach and speed | later |

### Stage C: physical parts (0.5.1 → 0.6.0)
Draw parts as they are built; the app writes the equations. Plan: `docs/PLAN-0.6.md` (agreed with the defaults).

| Release | What it adds |
| --- | --- |
| 0.5.1 ✅ | The network engine; physical ports and connections in the editor; **rotational** parts |
| 0.5.2 ✅ | **Translational** parts; rack and pinion, lead screw |
| 0.5.3 ✅ | **Electrical** parts with ideal switches and diodes |
| 0.5.4 ✅ | **Thermal** parts and heat ports |
| 0.5.5 ✅ | **Motors on ports** (DC, PMSM) and robot joints with rotational ports |
| 0.5.6 ✅ | **Hydraulics** |
| 0.5.7 ✅ | **Examples**: H-bridge motor and gearbox, buck converter, motor heating, hydraulic press, robot joint |
| **0.6.0** ✅ | **Milestone**: verification record (`docs/verification/2026-11-15-physical.md`), docs |

### 0.6.1: easy to find your way, first step (2026-11-15)
The user asked for one small update on ease of use before scripting. Done:
- tabs in working order, Model · Simulate · Analyze · Results · Modules · View · Help, each kind of tool with one home (the Control tab's tools joined Analyze; Live Tuning went to Simulate; scopes, results and export to Results; the Python engine buttons to Help);
- Modules by area (motors and drives, robotics with four menus, physical parts by domain);
- the library in four families with remembered folding;
- examples by topic (menu and File page chips);
- Help › Where Is…? and Tell Me finding everyday tasks and lighting up their button;
- quieter planned features.

### Stage S: scripting, the Datum way (0.6.2 → 0.7.0)
Python for those who want it, inside the window, on the Python already bundled. Plan: `docs/PLAN-script.md`.

| Release | What it adds |
| --- | --- |
| 0.6.2 | **Python console** in the window: the open model is ready to use (`model.vars`, `model.run()`, results as arrays); control commands (`tf`, `step`, `bode`, `lqr`) |
| 0.6.3 | **Script editor** and **setup scripts** saved with the model, run before every simulation |
| 0.6.4 | **Build models from code**: add blocks and wires from Python; scripting examples |
| 0.6.5 | **Python block**: a block whose behaviour is a Python function |
| 0.6.6 | **Import parameter files (.m)**: plain assignments become model variables (read, never run) |
| **0.7.0** | **Milestone**: verification record, docs |

### Stage U: easy to find your way (0.7.1 → 0.8.0)
The user, 2026-11-15: "the interface is okay, but navigating between the tools and understanding where to go is very difficult." This stage gets its own plan with questions (`docs/PLAN-ease.md`), written from a walk through the app and the user's notes. Possible pieces, to be decided in that plan:
- a **start page** organised by task ("simulate a motor", "tune a loop", "move a robot arm", "build a circuit"), each opening the right example and tools;
- a **map of the tools**: where each one lives and when to use it;
- **guided first steps** (a short tour, then hints that stop once known);
- a ribbon that **follows the selection** (select a PID and its tools come forward);
- clearer names and fewer places to look;
- **tutorials** for teaching (moved here from stage F).

### Stage D: exchange (0.8.1 → 0.9.0)
| Release | What it adds |
| --- | --- |
| 0.8.1 | **FMU export** (co-simulation) |
| 0.8.2 | **FMU import** |
| 0.8.3 | Results as **MAT and HDF5** |
| 0.8.4 | **Modelica models** through an external compiler (if one with a fitting licence is installed) |
| **0.9.0** | **Milestone** |

### Stage E: studies and logic (0.9.1 → 0.10.0)
| Release | What it adds |
| --- | --- |
| 0.9.1 | **State machines** |
| 0.9.2 | **Monte Carlo** and **sensitivity** |
| 0.9.3 | **Optimisation** and **parameter fitting** to measured data |
| 0.9.4 | **Report builder** |
| **0.10.0** | **Milestone** |

### Stage F: reach and speed (0.10.1 → 0.11.0)
| Release | What it adds |
| --- | --- |
| 0.10.1 | **Linux and macOS installers** (CI already builds and tests on both; "later, not needed now", the user 2026-11-15) |
| 0.10.2 | **Faster runs** for big models |
| 0.10.3 | **Variants** and **model comparison** |
| 0.10.4 | **Contributor guide** |
| **0.11.0** | **Milestone** |

### After that
The way to **1.0**: a public benchmark suite, a complete user guide and a signed installer.

## 4. What is not planned
Production code generation, hardware in the loop and real-time targets, certification kits, fixed-point data types, and requirements management.

## 5. Size and pace
- Each patch is one focused piece. If one turns out larger, it is split rather than shipped late.
- Stage S has 5 patches plus its milestone. Stage U is sized by its plan.

## 6. Decisions (2026-11-15)
The user agreed:
- the motto (§2.6);
- the scripting stage, built the Datum way from free, open parts (Python), with `.m` parameter-file import as its last, optional item;
- the order: finish stage C, then S, then U;
- Linux and macOS later;
- the bundled Python stays as it is (`docs/PLAN-script.md` §3).
