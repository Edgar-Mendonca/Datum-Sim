# Plan 0.2: a bigger build

Written 2026-10-29 (session 6), after 0.1.3. Status: **waiting for the user's answers (§6)**. Nothing in this plan is built yet.

## 1. The idea

The roadmap has three small releases next: discrete control (0.1.4), wires and diagram (0.1.5) and scopes (0.1.6). The user asked for "a little bigger build". This plan merges them into one release, **0.2.0**, and adds the two features that make models reusable: **masks** and **My blocks**. That joins the "Reusable models" item already planned for 0.2.

Theme: **build real controllers, read the results properly, reuse what you build.**

Releases stay numbered and small inside the build. Each part is pushed as it is done, and patch releases (0.1.4, 0.1.5, …) can still go out on the way if you want them (question 2).

## 2. What is in it

### A. Discrete control (was 0.1.4)
| Item | What it does |
| --- | --- |
| Discrete transfer function | num/den in z (or z⁻¹), sample time; states updated at each sample |
| Discrete integrator | forward Euler, backward Euler or trapezoidal; limits; reset |
| Discrete PID | parallel form, filtered D, anti-windup (clamping and back-calculation), output limits |
| Discrete state space | x[k+1] = A x[k] + B u[k] |
| Rate transition | moves a signal safely between two sample times (hold or delay) |
| Sample-time colours | each block coloured by its rate (continuous black, fastest red, next green…), with a legend; the engine works out each block's rate |
| Integrator reset | an optional reset input on the continuous integrator (rising, falling or either edge), located exactly |
| Goto / From | send a signal by name without a wire, with a tag visibility (local or global) |
| Signal sources | square, triangle, chirp, repeating sequence |
| Example | a PMSM drive with a 10 kHz discrete current loop and a 1 kHz speed loop; a digital PID of the DC motor against the continuous one |

Verification: discrete TF against the difference-equation answer; a discrete PID loop against the closed form of its z-domain system; the rate transition's values at both rates; the discrete integrator's three methods on a ramp.

### B. Wires and diagram (was 0.1.5)
| Item | What it does |
| --- | --- |
| Branch from a wire | drag from the middle of a wire to a new input |
| Move wire segments | drag a straight segment sideways; double-click a bend to remove it |
| Signal names | name a wire ("speed"); the name shows on the wire and in scope legends |
| Text notes | a note box on the diagram (teaching, reviews), printed with it |
| Find | Ctrl+F finds a block or signal across subsystems and opens it |
| Distribute, block colours | space blocks evenly; colour blocks to mark plant, controller, sensors |

### C. Scopes (was 0.1.6)
| Item | What it does |
| --- | --- |
| Two cursors | t, y, Δt, Δy readouts; snap to peaks |
| Y-zoom and auto-scale | per scope; fixed range as an option |
| Stacked plots | one plot per input, shared time axis |
| Keep and compare runs | the last runs kept; overlay any of them |
| Signal probe | after a run, hover a wire to read its value at the cursor time ("log all signals" option) |
| Step-response marks | rise time, overshoot and settling shown on the plot |
| Pictures | SVG and high-resolution PNG at a chosen size |

### D. Reusable models (from 0.2)
| Item | What it does |
| --- | --- |
| Masks | a subsystem gets its own parameters (with units and hints), an icon text, a help text and its own typical-ratings list; the inside uses those parameters |
| My blocks | save a masked subsystem to your library; it shows in the Library for every model. Copied into the model when used (your answer in PLAN-0.1.x §5) |
| Look under mask | open the inside; edits ask whether to change only this copy |

### E. Left for later (not in 0.2.0)
- Buses, solving algebraic loops, live tuning, linearising whole models (0.3).
- Switched reluctance motor, DC link and current sensor blocks, motor power-balance checks: these could join 0.2.0 if you want (question 4).

## 3. Engine changes behind it

- **Sample times**: each block gets a rate (continuous, discrete Ts with offset, inherited, constant). Inherited rates are worked out by fixed-point iteration, as the widths are today. The solver already lands on every `next_hit`; discrete blocks share one timetable instead of each having its own.
- **Masks**: a subsystem's mask parameters are evaluated in its own scope: inner names first, then the model's variables. This needs a scope stack in parameter evaluation. Today there is one flat variable table.
- **Signal logging**: an option to log every signal, decimated with peaks kept, for the probe and for comparing runs.
- **File format**: `.dsim` gets version 2 (masks, notes, signal names, run history off by default). Version 1 files open and are upgraded; a test keeps every 0.1 example and a damaged file opening.

## 4. Order of work and size

| Step | Part | Size |
| --- | --- | --- |
| 1 | Sample times in the engine + discrete blocks + verification | medium |
| 2 | Integrator reset, Goto/From, sources, discrete examples | small |
| 3 | Scopes: cursors, y-zoom, stacked, pictures, step marks | medium |
| 4 | Signal logging, probe, compare runs | medium |
| 5 | Wires: branch, segments, names, notes, find, distribute, colours | medium |
| 6 | Masks (engine scopes, Inspector, mask editor, file v2) | large |
| 7 | My blocks (save, library panel, copy into models) | medium |
| 8 | Docs, help cards, verification record, site.json, release 0.2.0 | small |

About three times the size of 0.1.2. Every step keeps the CLAUDE.md checks green, adds UI checks for what you can see, and is checked in the real app.

## 5. Risks

- Masks touch parameter evaluation everywhere: the largest change since 0.1.0. Mitigation: build them last, behind the same tests, with the file upgrade tested on every example.
- Logging every signal can use a lot of memory on long runs. Mitigation: it is off by default, decimated, and has a cap (as `MAX_POINTS` today).
- One big release delays everything in it. Mitigation: push each step as it is done, and release patches on the way if you prefer (question 2).

## 6. Questions for the user

1. **Scope**: all four parts (A–D) in 0.2.0, or A–C now and masks and My blocks in a following build?
2. **Releases on the way**: one release at the end (0.2.0), or a patch release after each part (0.1.4 discrete, 0.1.5 scopes, …) and 0.2.0 when masks land?
3. **Order**: the plan starts with discrete control, then scopes, then wires, then masks. Is any part more urgent for you?
4. **Motor extras**: add the switched reluctance motor, DC link and current sensor, and the motor power-balance checks to this build?
5. **Examples**: are there models you want as worked examples, for example a model from your own work, a textbook problem, or a particular drive?
6. **My blocks location**: keep your library in a folder you choose (it can then be shared on a network drive), or only in the app's own data folder?
