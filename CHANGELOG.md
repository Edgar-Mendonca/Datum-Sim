# Changelog

## 0.2.9 — 2026-11-10

Buses: many signals on one wire.

- **Bus Creator** (Library › Routing) groups named signals into one wire. Name them, or leave the names empty to use the incoming signals' names (wire names or block names). Signals may be vectors.
- **Bus Selector** picks signals out of the bus by name, with one output per name, in any order. The bus may pass through subsystem ports and Goto/From tags on the way.
- A name that is not on the bus is a model error that lists the names the bus has.

Checked: three signals, one of them a vector, go through a bus and Goto/From tags and come out unchanged in a different order.

## 0.2.8 — 2026-11-10

Live tuning and real-time pacing.

- **Control › Live Tuning** opens a panel of sliders. Add any model variable: it gets a slider with a range around its value, and you can edit the ends.
- **Move a slider while the model runs** and the scopes respond at once. The blocks that use the variable are rebuilt with the new value at the next step, and the states are kept, so nothing jumps except what you changed.
- **Keep these values** writes the slider values to the variables, as one undoable edit.
- **Simulate › Pacing** runs the model in real time, or at ×0.1 to ×10, so you can follow it and tune by hand. It can change while the model runs.
- Tunable variables and the pace are saved with the model.

Checked: a gain changed live at 1 s gives exactly the same result as a Step block that changes at 1 s. A 2 s run paced at ×50 takes about 40 ms.

## 0.2.7 — 2026-11-10

One-click drive tuning.

- **Control › Drive Tuning** tunes a motor drive's cascade from the motor's own data: resistance, d and q inductances, torque constant and inertia. It works with DC, BLDC, PMSM and three-phase PMSM blocks.
- **Current PIs**, for the q and d axes or the armature, cancel the winding's pole, so the current loop crosses at the bandwidth you choose with 90° of margin.
- **Speed PI**: designed on the motor's mechanics as seen through the closed current loop. The speed PI can give a current or a torque reference.
- Say which PID is the speed loop and which are the current loops. **Apply** sets them in one undoable edit, changing the model variables they use where they use them.

Checked: for a 4-pole-pair PMSM, the current loop crosses at exactly 2000 rad/s with 90°. The speed loop crosses within 10 % of 200 rad/s with more than 60° of margin.

## 0.2.6 — 2026-11-10

The PID Tuner.

- **Control › PID Tuner** tunes a PID block in its real loop. The model is linearised with the loop cut at the PID's output, to give the plant the PID sees.
- **Two sliders.** **Speed** sets the loop's crossover frequency and **Robustness** sets its phase margin. The gains follow as you move them.
  - You see the gains now and after, the margins before and after, the closed-loop step response and |L|.
  - It designs a PI when the plant allows one and a PID when it needs one, or you can choose which.
- **Classic rules** alongside: Ziegler–Nichols (from the ultimate gain and period) and SIMC (from a first-order-plus-dead-time fit).
- **Apply** sets P, I, D and N in one undoable edit. If a gain is a model variable (Kp), the variable changes.
- Loop Analysis now says when the gain is zero because a block is at a limit.
- The DC motor speed example's reference now steps at 0.2 s, so its starting point is not at the PID's limits.

Checked: the DC motor's PID was tuned for 20 rad/s and 60°. Linearising the real model again with the new gains gives a 20 rad/s crossover and a 60° phase margin.

## 0.2.5 — 2026-11-10

Operating points: find where the model rests, and analyse it there.

- **Trim** (Control tab) finds the states where every derivative is zero. It holds the analysis-point outputs you choose at their values and changes the model variables you allow, for example a set point or a load. If it cannot meet everything, it says which equations are unmet.
- **Save** the result as a named operating point, as one undoable edit, kept in the model file.
- **Operating Points** lists the saved points. Loop Analysis can linearise around any of them.
- The pump and tank example is ready: its level is an output and its loop opens at the PI.

Checked: the tank held at 2 m with its set point free gives a set point of 2, a pump flow of Cv·√2, and a PI integral of Cv·√2 / I.

## 0.2.4 — 2026-11-10

More views of the loop in Control › Loop Analysis.

- **Nyquist**: the loop L(jω) with the −1 point. It shows the number of encirclements, the open-loop poles in the right half-plane, and the stability verdict.
- **Root locus**: the closed-loop poles as the loop gain is scaled from 0.001 to 1000 times. It marks the open-loop poles and the poles at today's gain.
- **Sensitivity**: |S| and |T| over frequency, and the peak sensitivity Ms as a robustness figure.
- Each view also has its own button in the Control tab.

Checked against textbook loops: K/(s+1)³ has its phase crossover at √3 rad/s and is stable for K < 8. At K = 10 it has two right-half-plane closed-loop poles and two encirclements.

## 0.2.3 — 2026-11-10

Control design starts: analyse the whole model, not one block.

- **Analysis points**: mark a wire's signal as a **loop opening**, an **input** or an **output** (Control tab, right-click a wire › Analysis point, or the Inspector). A small mark shows on the wire. Points are saved with the model and runs ignore them.
- **Loop Analysis** (Control tab) linearises the whole model between its points. It works around the initial state or the state at a time in the last run, and with motors and nonlinear blocks too.
- From a loop opening to itself you get the loop transfer: a **Bode plot with gain and phase margins**, whether the closed loop is stable, and the closed loop's **step and impulse** responses with rise time, overshoot and settling time.
- From an input to an output you get that transfer's Bode plot, step response and poles.
- **Copy A, B, C, D** puts the linear model on the clipboard.
- The DC motor speed example has a loop opening at its PID already: open it and press Loop Analysis.

Checked against closed forms: the mass–spring–damper's transfer function, and the DC motor position loop against the PID times the motor's voltage-to-angle transfer.

## 0.2.2 — 2026-11-10

The whole road ahead, in the window.

- Every feature still to come now shows on its ribbon tab, dimmed, with the release that brings it. Click one to see the dialog it will have.
- Two new ribbon tabs: **Control** (control design, arriving from 0.2.3) and **Modules** (ready-made systems such as the robot arm, and physical components).
- File › Roadmap lists every planned feature by release.
- View › Planned features hides them when you want the working tools only. When the ribbon is narrow, planned features move under More ▾.

## 0.2.1 — 2026-11-02

Automation: run models from the command line and from Python.

**Command line**: `datum-sim-cli`, installed next to the app.
- `run` a model and write the results as CSV or JSON, or print a summary. Set variables (`--set Kp=120`), block parameters (`--param PID:D=5`) and run settings (`--stop`, `--solver` …), pick signals, and reduce the points.
- `sweep` a variable and write one file per value; `check` a model (exit code 1 on errors, for build servers); `info` lists blocks, parameters, variables and signals; `examples`.

**Python**: `import datum_sim`. Open a model or an example, set variables and parameters, run, and get the results as NumPy arrays (or a pandas table). Sweep a variable, check, save. Install with `pip install "git+https://github.com/Edgar-Mendonca/Datum-Sim#subdirectory=py"`.

**Help › Automation** shows the commands for the open model, ready to copy.

The command line and Python use the app's own engine, so their results are the same as the window's.

## 0.2.0 — 2026-10-29

Reusable models: masks and My blocks.

**Masks**
- Give a subsystem a **mask** (Model › Mask, or the Inspector): parameters of its own with labels, units and defaults, a text on the block, its own help card, and typical ratings saved from the block's values.
- The blocks inside use the parameters by name; a parameter's value can use the model's variables and the parameters of masks around it. A masked subsystem is set in the Inspector like any block, and errors point at it.
- Look under the mask (right-click or the Inspector) to see and change the blocks inside.

**My blocks**
- Save any subsystem to **My blocks**; it appears at the top of the Library for every model. Drag it in or double-click it: each model gets its own copy, so changing a saved block later never changes models that already use it.
- Options › My blocks chooses the folder, so a team can share one on a network drive.

**Worked example**: a masked DC motor block used for two different motors in one model.

**Files**: models are saved in file format 2 (it can hold masks). Files from earlier versions open as before.

## 0.1.6 — 2026-10-29

Wires and the diagram: easier to draw, read and find your way in.

**Wires**
- Drag a wire's middle segment sideways to move it; the bends stay square.
- Alt-drag (or Ctrl-drag) from a wire to branch its signal to another input.
- Double-click a bend to remove it.
- **Signal names**: double-click a wire (or use the Inspector) to name the signal it carries. The name shows on the wire, on the printed diagram and in scope legends and results, and every wire from the same output carries it.

**Diagram**
- **Notes**: text notes on the diagram for explanations, reviews and teaching (Format › Note). Wires run over them and they are printed with the diagram.
- **Find** (Ctrl+F): find a block, or a named signal, anywhere in the model, inside subsystems too; choosing one opens its diagram and selects it.
- **Block colours**: colour blocks to mark the plant, the controller and the sensors (Format › Block Colour, or right-click).
- **Distribute**: space three or more blocks evenly, across or down.

## 0.1.5 — 2026-10-29

Scopes you can measure with, and a probe that reads any wire.

**Scopes**
- **Cursors**: two lines, A and B, that you drag across the plot; a table gives every signal at A and at B, the difference, Δt and 1/Δt.
- **Marks**: rise time (10–90 %), overshoot, peak and settling time (2 %) of the first signal, drawn on the plot.
- **Compare runs**: the last five runs are kept; overlay any of them, dashed in the same colours.
- **Axes**: drag up or down to zoom the y axis; a fixed y range; one plot per input (stacked) on a shared time axis. Layout and range are saved in the Scope block.
- **Pictures**: PNG as shown, PNG at three times the size for print, and SVG for papers and reports.

**Signal probe**
- Simulate › Log all signals logs every signal of the model. After the run, hover any wire to read its value at cursor A (or at the end of the run), inside subsystems too.

**Fixed**
- With every signal logged, the first run no longer opens an empty scope.

## 0.1.4 — 2026-10-29

Discrete control: controllers that run at a sample time, as in a microcontroller, and more motor and drive blocks.

**Discrete control**
- Discrete transfer function and discrete state space.
- Discrete integrator (forward Euler, backward Euler or trapezoidal) with limits and a reset input.
- Discrete PID with a filtered derivative, output limits and anti-windup (clamping or back-calculation).
- Rate transition, to pass a signal between sample times (hold, or a one-sample delay).
- **Sample-time colours** (View › Sample-time colours): every block is coloured by how often its output changes, with a legend: continuous, each sample time, constant, or several rates.

**Blocks**
- A reset input on the Integrator (rising, falling or either edge), located exactly.
- Goto and From: send a signal by name without a wire, within one diagram or across the whole model.
- Sources: square wave, triangle wave, chirp (frequency sweep), repeating sequence.
- Terminator, to end an output you do not need.

**Motors and drives**
- Switched reluctance motor (6/4, 12/8 …) with an asymmetric bridge, and typical ratings.
- DC link (with a diode rectifier that only charges it), inverter DC current, and a current sensor with gain, offset, bandwidth and noise.

**Worked examples**: a digital PMSM drive (10 kHz current loop, 1 kHz speed loop) whose DC link rises while the motor brakes; a DC motor with a digital and a continuous PID side by side; switched reluctance motor speed control.

**Checked**: the discrete blocks against their difference equations; power balance for the induction, synchronous, DC and BLDC machines; the switched reluctance motor's torque and energy balance. See [the verification record](https://github.com/Edgar-Mendonca/Datum-Sim/blob/HEAD/docs/verification/2026-10-29.md).

**Fixed**
- A signal resting at exactly zero (such as a reset input between pulses) could make the variable-step solvers take endless tiny steps.

## 0.1.3 — 2026-10-29

- Windows now lists the publisher as Datum.

## 0.1.2 — 2026-10-28

The motor library: DC, induction and synchronous motors, BLDC and stepper motors, and the pieces of a drive.

**Motors**
- DC motors: separately excited, shunt, series and compound, each with its field circuit.
- Induction motors: squirrel-cage, double-cage and wound-rotor (rotor voltages as an input, for starting resistors or a rotor-side drive), fed with three-phase voltages.
- Synchronous motors: permanent-magnet with three-phase inputs (surface or interior magnets), synchronous reluctance, and the wound-field salient-pole motor with damper windings used for large medium and high voltage drives.
- BLDC motor with ideal six-step commutation and Hall signals; two-phase hybrid stepper motor.
- **Typical ratings**: choose a rating in the Inspector (for example 400 V 11 kW, 3.3 kV 500 kW, 11 kV 5 MW) and every value is filled in at once, as one step you can undo. Replace them with nameplate data when you have it.

**Drives** (a new library group)
- Three-phase source, three-phase inverter (average model, or switched with the switching instants located exactly), PWM, encoder.
- Clarke and Park transforms and their inverses.

**Worked examples**: induction motor direct-on-line start, induction motor V/f speed control, PMSM field-oriented control with an inverter, BLDC speed control, a 3.3 kV synchronous motor on the grid taking a load step, a DC series motor starting a heavy load, and a stepper motor making full steps.

**Checked**: the induction motors' torque against the exact equivalent circuit at three slips, the DC motors' and the synchronous motor's steady states, the three-phase PMSM against the d–q PMSM, BLDC no-load speed, the stepper's step angle, the inverter's mean voltages and the encoder. See [the verification record](docs/verification/2026-10-28.md).

**Fixed**
- A zero-crossing that fell exactly on zero at the end of a step could be missed; the variable-step solvers now keep its sign and locate it.

## 0.1.1 — 2026-10-28

Dragging blocks from the library works on Windows, and small fixes from first use.

**Fixed**
- Blocks can be dragged from the Library onto the diagram again on Windows: a ghost of the block follows the pointer and the block lands where you drop it.

**Better**
- A wrong parameter shows its error in the Inspector as soon as you change it, before you press Run.
- When the scopes open under the diagram for the first time, the diagram fits itself into the smaller space.
- Dots show where a wire splits to several inputs.

## 0.1.0 — 2026-10-28

The first version: draw a model as blocks and wires, press Run and watch the scopes.

**Models**
- A block diagram editor: drag blocks from the library, connect outputs to inputs, and the wires find their own way around the blocks. Move, resize, flip, rename, copy and paste, undo and redo, zoom and pan, a minimap for big diagrams.
- About 45 blocks: sources (step, ramp, sine, pulse, clock, table, CSV file, random noise), math (gain, sum, product, divide, min/max, function, saturation), continuous (integrator with limits, derivative, transfer function, zero-pole-gain, state space, PID with anti-windup, transport delay), discontinuities (dead zone, rate limiter, relay, backlash, quantiser), discrete (zero-order hold, unit delay), routing (mux, demux, switch, selector), 1-D and 2-D lookup tables, and sinks (scope, XY graph, display, CSV file).
- Motors: a DC motor block and a permanent-magnet synchronous motor block (d–q model).
- Subsystems: select blocks and make them one block; open it to see inside; ungroup it again.
- Model variables: name a value once (R, L, Kp …) and use it in any parameter.

**Running**
- Five solvers: variable step (Dormand–Prince), stiff (Rosenbrock), and fixed-step RK4, Heun and Euler. Jumps in the inputs are hit exactly; relay switching and integrator limits are located exactly.
- Run, stop, pause and single step, with live scopes.
- Clear errors that point at the block: unknown names, sizes that do not fit, algebraic loops.
- Parameter sweeps: run the model for several values of a variable and compare the runs.

**Results and analysis**
- Scopes with cursors, zoom and PNG pictures; results as CSV, ready for Datum Plot.
- Bode plot, stability margins, step response and poles and zeros of linear blocks, and the rise time, overshoot and settling time of any signal, from the built-in Python engine.

**Worked examples**: mass–spring–damper, DC motor speed control (the motor built from blocks in a subsystem), DC motor position control, PMSM speed control, pump and tank level control, vehicle cruise control and a room thermostat.

**And** the Datum family window: ribbon, File page, options, print to PDF, a ? card beside every tool, light and dark themes, recovery of unsaved work and updates from inside the app.
