# Changelog

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
