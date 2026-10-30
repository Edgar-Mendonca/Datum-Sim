# Plan 0.3: control design

Written 2026-10-30 (session 7), after 0.2.0. Status: **waiting for the user's answers (§7)**. Nothing in this plan is built yet.

## 1. Where we start

Today the analyses (Bode, step response, poles and zeros, margins) work on **one linear block at a time**: a transfer function, a state space, a PID, the DC motor, a gain or an integrator. Python-control computes them from that block's A, B, C, D. Step-response figures come from a logged signal.

0.3 takes this to **whole models**. You mark where to look in the diagram (an input, an output, a loop to open). The engine finds the model's linear behaviour there, at an operating point that it can also search for. From that come Bode, Nyquist, root locus and margins of the real loop, a PID tuner, and parameters you can tune while the model runs.

## 2. What is in it

### A. Linearising whole models
| Item | What it does |
| --- | --- |
| Analysis points | Mark a wire as an **input** (where a test signal goes in), an **output** (what is measured), or a **loop opening** (the loop is cut there for open-loop analysis). They are drawn on the wire and saved with the model, and they do not change a run. |
| Linearise | The engine linearises the whole model between the marked points: the system's A, B, C, D around an operating point. It uses central differences on the compiled model, so it works with motors, lookup tables and nonlinear blocks too. Linear blocks come out exact. |
| Operating points | Linearise at the model's initial state, at a **time in the last run** (the states at t = 2 s, say), or at a trimmed point (B). Operating points can be saved and named in the model. |
| Discrete parts | A loop where every sample time is the same is linearised in z, and the analyses work in z. Mixed rates are converted to one rate (ZOH), and the app says so. |

### B. Trim: finding operating points
| Item | What it does |
| --- | --- |
| Trim | Find the states and inputs where the model is at rest (all derivatives zero), with chosen outputs held at chosen values. Examples: the tank at 2 m, the motor at 1500 rpm under its load, the cruise control at 25 m/s. |
| How | Pick which inputs are free and which outputs are fixed, in a dialog. The engine solves with Newton–Levenberg–Marquardt in Rust and reports what it could not meet. |

### C. Analyses of the linearised model
| Item | What it does |
| --- | --- |
| Bode and margins | The loop, not only one block: gain and phase margins with their frequencies, marked on the plot |
| Nyquist | With the −1 point, the number of encirclements, and the stability verdict |
| Root locus | Poles as a gain varies, with the gain read under the pointer, and the damping and natural-frequency grid |
| Step, impulse, poles and zeros | Of the open or the closed loop |
| Sensitivity | S and T over frequency: peak sensitivity Ms as a robustness figure |

The plots use the family chart style. The numbers come from the engine (Rust) and python-control. Each analysis can be saved as CSV and as a picture.

### D. PID tuning
| Item | What it does |
| --- | --- |
| Tune a PID | Select a PID (continuous or discrete) and press Tune. The loop is opened at the PID and the plant it sees is linearised. You choose **response speed (bandwidth)** and **robustness (phase margin)** with two sliders, and the gains are designed to meet them. Before and after step responses and the loop's Bode plot update as you move the sliders. |
| Classic rules too | Ziegler–Nichols (from the ultimate gain the engine finds) and SIMC (from a first-order-plus-dead-time fit), for teaching and comparison |
| Apply | One undoable edit sets P, I, D and N. A discrete PID's gains are designed for its sample time. |

### E. Live tuning
| Item | What it does |
| --- | --- |
| Tunable variables | Mark a model variable as tunable with a range. A slider panel appears while the model runs, and moving a slider changes the running model at once: the scopes show the effect. |
| What can change live | Gains, PID gains, constants, limits, set points, lookup-table values: parameters that do not change the model's structure. A change that would (a transfer function's order, a vector's width) waits for the next run, and the app says so. |
| Real time | An option to slow a run to real time (or ×0.1 to ×10), so tuning by hand is possible. |
| Keep the result | "Keep these values" writes the slider values back to the variables as one undoable edit. |

### F. From the old 0.3 list (question 1)
- **Buses**: group named signals into one wire and pick them out by name.
- **Algebraic loops solved**: a loop with no state is solved by Newton iteration at each step, instead of being reported as an error.

## 3. Engine changes behind it

- **Linearisation** (`linearise.rs`): perturb each state and each marked input on the compiled model, and read the derivatives and marked outputs. Use central differences with steps scaled to each variable. A loop opening becomes an extra input and output pair at the cut. For discrete parts the same is done on the update map.
- **Trim** (`trim.rs`): a damped Newton (Levenberg–Marquardt) solve for the derivatives = 0 and outputs = targets, with the Jacobian from the same differences. LU is already in `linalg.rs`.
- **Live parameters**: runtimes get `retune(&Pv)` for the blocks whose parameters can change live. The run thread takes slider messages between steps, re-evaluates the affected blocks' parameters and retunes them. States are kept, so nothing jumps except what changed.
- **Analysis points** are a new field on wires (`point: in | out | open`), with a format 3 file. Older files open unchanged.
- **Python**: Nyquist, root locus and margins use python-control as Bode does now. The PID design maths (loop shaping to bandwidth and phase margin, Ziegler–Nichols, SIMC, the fit) live in Rust, so they are fast enough to follow the sliders and can be unit-tested.

## 4. How it is checked
- Linearisation against **closed forms**: the mass–spring–damper's A, B, C, D exactly; the DC motor block's transfer function; a PMSM at an operating point against its d–q equations by hand; a Gain → Integrator loop opened at the gain.
- Trim against **known equilibria**: the tank level (√ law), the cruise force balance, the motor speed under load.
- Nyquist, margins and root locus against python-control on textbook loops (and against hand results for a type-1 second-order loop).
- PID tuning: after tuning, the margins and crossover the engine computes for the new loop must match the targets within a tolerance; Ziegler–Nichols against the classic table on a known plant.
- Live tuning: a run where a gain is changed at t = 1 s must match a run where the same change is made by a Step block at 1 s.
- As always: UI checks for everything you can see, the real app under Xvfb, and a dated verification record.

## 5. Order and releases (my proposal)

| Release | Contents | Size |
| --- | --- | --- |
| 0.2.1 | Analysis points, linearising whole models at the initial state or a time in a run, Bode and margins of the loop | medium |
| 0.2.2 | Nyquist, root locus, sensitivity, step and impulse of the loop | medium |
| 0.2.3 | Trim and saved operating points | medium |
| 0.2.4 | PID tuning | medium |
| 0.2.5 | Live tuning and real-time pacing | medium |
| 0.3.0 | Buses and algebraic loops (if wanted, question 1), docs and the 0.3 verification record | medium |

About the size of the 0.2 build. As before, each step is pushed as it is done and released when it passes every check.

## 6. Risks

- **Linearising at kinks** (saturation edges, relays, dead zones) gives a derivative that depends on the side. The engine reports blocks sitting at a kink at the operating point, so a result is never silently wrong.
- **Live tuning** touches the run loop, the most delicate part. Mitigation: only blocks that declare `retune` can change, changes apply between steps, and the test compares against an equivalent Step.
- **Trim** may not converge on hard models. Mitigation: clear reports of which equations are unmet, starting from the last run's state, and limits on the free inputs.

## 7. Questions for the user

1. **Scope**: all of A–E? And should buses and algebraic-loop solving (F) be in this build, or later?
2. **Releases**: patch releases as each part lands (0.2.1 … 0.2.5, then 0.3.0), as in 0.2? Or one release at the end?
3. **Live tuning**: sliders for **model variables marked as tunable** (my proposal: one place, works for every block that uses them)? Or sliders right on any block's parameters too?
4. **PID tuning style**: is the two-slider design (speed and robustness) what you want, with Ziegler–Nichols and SIMC alongside? Are there other rules you use (for example IMC or lambda tuning, Cohen–Coon)?
5. **Motors and drives**: should the tuner also offer the usual drive cascade (current loop from motor L and R, then speed loop from inertia), one click for a whole FOC or DC drive?
6. **Examples**: any plant you want as a worked control-design example? My default would be the DC motor, the tank and a PMSM drive.
