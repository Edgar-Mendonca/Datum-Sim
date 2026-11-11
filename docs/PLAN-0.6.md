# Plan 0.6: physical components (stage C, 0.5.1 → 0.6.0)

Written 2026-11-11 (session 9), after robotics (0.5.0). Status: **agreed (the defaults in §8), being built**. 0.5.1 is done.

## 1. What changes for the user

Today every wire is a **signal**: it carries a number one way, from an output to an input. A motor, a gearbox or a circuit has to be written as equations with blocks: integrators, gains, sums.

With physical components you **draw the system as it is built**, and the app writes the equations:
- an inertia joined by a shaft to a gear and a load;
- a resistor, a capacitor and a switch;
- a pump, a valve and a cylinder.

Physical connections have no direction. A node where several parts meet shares one value (a speed, a voltage, a temperature, a pressure), and what flows into it adds to zero (torque, current, heat flow, volume flow). These are the same rules for every domain, the "across and through" variables.

## 2. Domains and parts

| Domain | Across (shared at a node) | Through (sums to zero) | Parts |
| --- | --- | --- | --- |
| Rotational | speed ω | torque τ | inertia, spring, damper, gear (with efficiency), friction (stick–slip), backlash, fixed frame, torque source, speed source, sensors |
| Translational | velocity v | force F | mass, spring, damper, hard stop, friction, fixed frame, force source, velocity source, sensors |
| Electrical | voltage V | current i | resistor, capacitor, inductor, ground, voltage and current sources (constant, or driven by a signal), ideal switch, diode, transformer, op-amp, sensors |
| Thermal | temperature T | heat flow Q̇ | heat capacity, conduction, convection, radiation, ambient (temperature source), heat-flow source, sensors |
| Hydraulic (basic) | pressure p | volume flow q | pump (fixed displacement), orifice and valve (driven by a signal), pipe (resistance), cylinder (double-acting), tank, accumulator, relief valve, sensors |

**Between domains:**
- the DC motor (electrical ↔ rotational);
- the PMSM in d–q (electrical ↔ rotational);
- a rack and pinion or a lead screw (rotational ↔ translational);
- the cylinder (hydraulic ↔ translational) and the pump (rotational ↔ hydraulic);
- the heat port on resistors, motors and switches (any domain → thermal), so losses heat things up.

**Between parts and signals:**
- **sensors** turn a physical value into a signal;
- **driven sources** turn a signal into a physical effort.

A PID can then drive a physical motor in a physical circuit, and scopes read sensors as before.

## 3. How the engine does it (my recommendation)

1. **Networks, not a general equation compiler.** Each connected group of physical parts becomes one network. Every part contributes its standard branch equation; the nodes give the "sum to zero" rules (the method used by circuit simulators, applied to every domain).
2. **The network becomes one block in the signal engine.**
   - Its states are the parts' stored energy: capacitor voltages, inductor currents, inertia speeds, spring deflections, heat-capacity temperatures, fluid volumes.
   - Its other unknowns are solved at each evaluation by Newton's method, as algebraic loops already are.
   - So the five solvers, linearisation, trim, live tuning, sweeps and the command line all work with physical parts **without changes**. The stiff solver handles stiff networks.
3. **Problems are caught when the model is checked, with plain messages:**
   - two inertias joined rigidly are merged into one;
   - a loop of voltage sources or capacitors, or a cut set of inductors, is reported with the parts involved and how to fix it (add a small resistance);
   - floating nodes are reported too.
4. **Switches and diodes are ideal.** They switch at exact instants (the zero-crossing location the engine already has), so converters are accurate and fast. A smooth diode model is an option for those who want one.
5. **Units and parameters** are checked as now. Every part has a ? card with its equation.
6. **The robot arms join in.** A Robot arm's joints can take rotational ports (0.5.5), so the joint motors, gears and friction can be drawn as physical parts.

**Why not translate to Modelica, or use an open Modelica tool?**
- The open compilers are AGPL or GPL, which our licence rules exclude.
- A general equation compiler with index reduction is a much larger project.
- The network approach covers these five domains well. Exchange with Modelica tools stays in stage D (0.6.x), through FMU and an external compiler.

## 4. The window

- **Physical ports** are drawn as small squares in each domain's colour: rotational grey, translational green, electrical blue, thermal red, hydraulic teal. Signal ports keep their arrows.
- **Physical connections** have no arrowhead and take the domain's colour. Several can meet at one port, or at a node dot on a line.
  - Only ports of the same domain connect; a wrong connection is refused with a reason.
- **Library:** new groups Rotational, Translational, Electrical, Thermal and Hydraulic, each with a ? card per part.
- **Probe:** after a run, hovering a physical connection shows its across and through values (V and i, ω and τ …).
- Physical parts work inside **subsystems** and **masks**, and in **My blocks**, like any block.
- **Files** become format 3. Older files open unchanged. A file with physical parts needs 0.5.1 or later to open.

## 5. Releases

| Release | What it adds | Checked by |
| --- | --- | --- |
| 0.5.1 | The network engine and the window's physical ports and connections; **rotational** parts | Two inertias and a spring: frequency √(k(J₁+J₂)/(J₁J₂)); a gear's speed and torque ratios with efficiency; stick–slip friction against its break-away torque; energy without damping conserved |
| 0.5.2 | **Translational** parts; the rack and pinion and the lead screw | The mass–spring–damper against the signal-block example (the same curve); a hard stop's rebound with its restitution |
| 0.5.3 | **Electrical** parts with ideal switches and diodes | RC and RLC step responses (closed form); a half-wave and a bridge rectifier (diode switching instants exact); a buck converter's average output D·V_in; the transformer ratio |
| 0.5.4 | **Thermal** parts and heat ports | An RC thermal network (closed form, and the same as the 0.3.3 Thermal network block); a resistor heating up by its I²R loss |
| 0.5.5 | **Motors on ports**: the DC motor and the PMSM (d–q) with electrical, rotational and heat ports; robot joints with rotational ports | The physical DC motor against the signal DC motor block (the same speed and current); the motor's heat equal to its I²R loss |
| 0.5.6 | **Hydraulics**: pump, valves, pipe, cylinder, tank, accumulator, relief valve | A cylinder's steady speed q/A; an orifice's flow against its pressure-drop law; the relief valve's set pressure |
| 0.5.7 | **Examples**: a DC motor driving a gearbox and a load from an H-bridge; a buck converter; a motor heating up; a hydraulic press; a robot joint with a physical gearbox | All run; within tolerance of their targets |
| **0.6.0** | **Milestone**: verification record, docs | |

Each is about the size of a robotics step. 0.5.1 is the largest, as it carries the engine and the editor changes.

## 6. Risks

- **Higher-index networks** (rigid connections, loops of sources and capacitors). The common case, rigid inertias, is merged automatically; the rest are reported with the parts and a fix. Nothing is silently wrong.
- **Stiffness:** stiff contact, small inductances and fast switches. The stiff solver is there, and the ideal switch avoids the stiffest smooth models.
- **Editor changes:** undirected connections touch the diagram editor, the most-used part of the window. Every new behaviour gets UI checks, and old models are unaffected.

## 7. What stays out (for now)

- Two-phase or compressible fluids, and fluid temperature.
- Detailed semiconductor models (beyond ideal switches and diodes with an on-resistance).
- 3D multibody contact.
- Electromagnetic field solutions.
- SPICE netlist import (could come later if wanted).

## 8. Questions for the user

1. **Domains and order:** rotational → translational → electrical → thermal → motors on ports → hydraulics. Is that right, or should electrical come first (for power electronics)?
2. **Hydraulics:** is basic incompressible oil with a compliance (bulk modulus) enough for now? My default is yes.
3. **Switches:** ideal switches and diodes switching at exact instants (my default), with a smooth diode as an option?
4. **Motors on ports:** the DC motor and PMSM first, with the induction motor later? Robot joints with rotational ports too?
5. **Look:** domain-coloured squares for physical ports, and lines without arrows. Is that fine?
6. **Releases:** one per step, as before?
