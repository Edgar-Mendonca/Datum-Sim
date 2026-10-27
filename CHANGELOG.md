# Changelog

## 0.1.0 — 2026-10-27

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
