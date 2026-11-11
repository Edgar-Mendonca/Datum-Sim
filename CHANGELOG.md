# Changelog

## 0.4.7 — 2026-11-11

The task scheduler.

- **Task scheduler** (Library › Robot arm) runs a task as steps. Each row is a step's duration and its target values, such as a tool position and a grip.
  - During a step the values glide smoothly from the last step's to this one's.
  - **Switch columns**, such as the grip, jump at the step's start instead.
  - **Wait for condition**: a step also needs its input ≥ 0.5 to end, for example "the payload is held".
  - **Repeat** starts again after the last step.
  - Outputs: the values and the step number.
- Its default steps are a pick and place: approach, go down, close, lift, carry, go down, open, back away.

Checked:
- Steps change at their times.
- Gliding values are exactly half-way at a step's middle, and switch columns jump.
- A step that waits for its condition ends only once the condition rises.

This is a small sequencer for robot tasks. Full state machines (branches, parallel states) stay on the roadmap for 0.7.1.

## 0.4.6 — 2026-11-11

Gripper and contact: pick things up.

- **Payload and table** (Library › Robot arm): an object on a table that the tool can pick up.
  - The table holds it up by spring–damper contact, and it slides with damping.
  - Closing the grip (input ≥ 0.5) with the tool within the capture distance holds it to the tool until the grip opens.
  - It outputs its position, the force on the tool and whether it is held.
- **Robot arm › Tool force input: yes** adds a second input: a force on the tool, so the arm feels a held payload's weight (Jᵀ·F).
- The Robot arm block has a fourth output: the **tool's velocity**.
- **3D view › Show an object: yes** adds a second input for an object's position, drawn as a box.

Checked:
- On the table, the payload rests at the table height less m·g/k.
- Closed on and lifted, it is carried m·g/k_grip below the tool, and the force on the tool equals its weight.
- Opened, it drops back to the table.

The gripper is simple: it holds by a spring–damper when closed nearby. The 7-joint arm's fingers do not move yet.

## 0.4.5 — 2026-11-11

Controllers for any arm, and motors on the joints.

- **Arm controller (any arm)**: computed torque, which cancels the arm's dynamics so each joint follows e'' + Kd·e' + Kp·e = 0, or PD with gravity compensation. Gains are per joint or one for all, with a torque limit. It takes its model from the Robot arm block you name.
- **Gravity torques** and **Inverse dynamics** blocks for any named arm: g(q), and τ = M(q)·q̈ + h(q, q̇).
- **Joint motors**: DC motors driving the joints through gears (resistance, inductance, Kt, Ke, gear ratio, efficiency). Inputs are the voltages and joint speeds; outputs are the joint torques and motor currents.
- **Armature** on the Robot arm block: each motor rotor's inertia seen at its joint (J_m·N²).
- **Joint trajectory** now takes any number of joints: rows of time, q1, q2, … qn.

Checked:
- The 7-joint arm under computed torque follows a joint trajectory within 1e-5 rad.
- Under PD + gravity it settles within 1e-3 rad of the last way point.
- Joint motors at stall give η·N·Kt·V/R within 1e-6.

## 0.4.4 — 2026-11-11

Inverse kinematics for any arm, and tool paths.

- **Inverse kinematics (any arm)** gives the joint values that put a Robot arm's tool at a position, or at a position and orientation (roll, pitch, yaw).
  - It uses damped least squares on the arm's Jacobian and stays within the joint limits.
  - Each solution starts from the last one, so the arm moves smoothly.
  - It takes the robot from the **Robot arm block you name**, so a robot is described only once.
- **Tool path**: straight tool moves through way points (time, x, y, z), each starting and ending at rest. Feed its position to Inverse kinematics to get joint references.

Checked:
- The Jacobian matches finite differences for both library arms.
- Inverse kinematics recovers a pose of the 6-joint and the 7-joint arm, position and orientation, within 1e-9 from a different start, and keeps every joint within its limits.
- Following the default tool path, the 6-joint arm's tool stays on the path within 1e-8 m.
- An IK block that names no arm is a model error that says so.

Arcs in tool paths are not in this release.

## 0.4.3 — 2026-11-11

The robot in 3D.

- **3D view** (Library › Robot arm, or Modules › Add 3D View): feed it the q output of a Robot arm (any joints) block. After a run it shows the robot moving in 3D.
  - **Play**, or drag the time slider.
  - **Turn the camera** by dragging, and zoom with the wheel.
  - The **tool's path** is traced.
- The 6-joint and 7-joint arms are drawn with their own parts. Robots from a DH table or your own URDF file are drawn as a skeleton through their joints.
- The 3D drawing library loads only when a 3D view opens, so the window starts as fast as before.
- **New example: 6-joint arm in 3D, swinging under gravity.**

Checked: the view's frames match the engine's own kinematics. In the window, the 6-joint arm's seven parts all load and draw, and the view opens in the dock with its play bar. In the real app the arm draws with its parts. A UI check now guards the security setting that first stopped them loading.

## 0.4.2 — 2026-11-11

Real robots: a 6-joint industrial arm, a 7-joint arm, and your own from URDF files.

- **Robot arm, any joints › Robot** now offers:
  - **6-joint industrial arm**;
  - **7-joint arm** (its two finger joints are held still until the gripper arrives);
  - **URDF file**: load any robot from its open description file, with an optional tool link.
- Each robot's links, masses, centres of mass, full inertias, joint axes, joint limits and joint damping come from its description file.
- The two arms come from open robot descriptions under BSD and Apache-2.0 licences (see NOTICE). Their light 3D parts are bundled for the 3D view in 0.4.3 (about 0.4 MB).

Checked: both robots load with 6 and 7 moving joints and the limits from their files. For every joint, at a test pose, the engine's gravity torque matches an independent sum of each link's weight times its lever arm within 1e-9. Broken files give clear errors.

## 0.4.1 — 2026-11-11

Robotics begins: robot arms with any number of joints.

- **Robot arm, any joints** (Library › Robot arm, or Modules › Add Arm (Any Joints)) builds an arm from a DH table. Each row is one joint: α, a, d, θ₀, the link's mass, its centre of mass and its inertias. Modified (Craig) and standard DH are both supported.
  - Input: one torque per joint, or one value for all.
  - Outputs: the joint angles, the joint speeds and the tool's position (x, y, z).
  - Gravity, joint friction, a tool offset and initial angles are set on the block.
- The engine computes the dynamics of any chain of rigid links: inverse dynamics, the mass matrix and forward dynamics.
- The default table is a six-joint arm, so you can drop the block in and run it.

Checked:
- As a DH chain, the two-link planar arm gives the closed form's accelerations within 1e-10, and its tip exactly.
- A six-joint arm with full inertias keeps its energy within 1e-8 without torque or friction, in both DH conventions. Its forward and inverse dynamics agree within 1e-9.
- Held by its own gravity torques, the six-joint arm stays still within 1e-6 rad.

Next: robots loaded from description files (0.4.2) and a 3D view (0.4.3).

## 0.4.0 — 2026-11-10

Drives in depth and the robot arm: the second stage of the roadmap is done. 0.4.0 brings together 0.3.1 to 0.3.7:

- **Spectrum and THD** of any signal.
- **Space-vector modulation** and **dead time** in the inverter.
- **Inverter losses** and a **thermal network** for the junction temperature.
- **Saturation and iron losses** in the induction machine.
- **Motor parameters from the nameplate**.
- The **Modules** tab with the **robot arm module**: arm, trajectory, controller, inverse kinematics and an animated Arm view.
- Three new **drive examples**.

New in 0.4.0:
- **Fixed:** an XY graph or the Arm view opened in the scopes dock no longer shows an error.
- The Arm view's Play bar stays visible in a small dock.
- Pictures for the new examples on the File page.
- A verification record for the stage (docs/verification/2026-11-10-drives.md).

Next on the roadmap, after your testing: physical components (0.4.1 → 0.5.0).

## 0.3.7 — 2026-11-10

Three new drive examples (File › New):

- **PMSM drive: inverter losses and junction temperature.** The 15 kW field-oriented drive with the inverter's conduction and switching losses heating a junction through a three-stage thermal network. Watch it warm as the load comes on.
- **Inverter harmonics: switched PWM into an induction motor.** A 2 kHz switched inverter feeds an 11 kW motor. Run it, then use Analyze › Spectrum on a stator current. Try space-vector modulation or another switching frequency.
- **Induction motor start with saturation and iron loss.** The direct-on-line start with a saturating magnetising inductance and an iron-loss output. Set ψ_sat to 0 to compare with the linear motor.

Checked: every example runs. The junction ends above the 40 °C ambient. The harmonics example's current has a finite THD with a fundamental above 5 A. The saturated motor runs up to just below synchronous speed.

## 0.3.6 — 2026-11-10

Modules, starting with the robot arm.

- A new **Robot arm** library group, and its own group on the **Modules** tab:
  - **Robot arm, two links**: a planar arm with a motor torque at each joint, gravity, inertia and joint friction. Its outputs are the joint angles, the joint speeds and the tip position.
  - **Joint trajectory**: smooth motion through way points. Each segment starts and ends at rest.
  - **Arm controller**: computed torque (it cancels the arm's dynamics) or PD with gravity compensation, with torque limits. Give it its own idea of the arm's parameters to see what a wrong model does.
  - **Inverse kinematics**: the joint angles that put the tip at (x, y), elbow up or down.
  - **Arm view**: an animated drawing of the arm after a run, with Play, a time slider and the tip's path.
- **Worked example: Robot arm pick and place.** Run it, double-click the Arm view, press Play.
- More modules (flight, vehicle, a hardware bench) show on the Modules tab as planned.

Checked:
- Without torque or friction, the arm's energy stays constant to 1e-9.
- Gravity compensation holds it still at any pose.
- Inverse kinematics returns the tip exactly, elbow up and down.
- In the example, each joint follows its trajectory within 1e-4 rad.

## 0.3.5 — 2026-11-10

Motor parameters from the nameplate.

- **Inspector › From nameplate…** (squirrel-cage and wound-rotor induction motors) estimates the equivalent circuit from the rating plate and data sheet. You give the rated power, line voltage, frequency, speed, efficiency, power factor and the starting current ratio I_A/I_N.
- You get Rs, Rr, Lls = Llr, Lm and the pole pairs, with the rated current and slip. **Apply** sets the six parameters in one undoable edit.
- Inertia and friction are not on a nameplate: set those yourself.

Checked: a known machine's rated point, turned into a nameplate, gives back its own Rs, Rr, leakage and Lm within 1e-6. An efficiency too high for the slip is refused with a reason.

## 0.3.4 — 2026-11-10

Magnetic saturation and iron losses in the induction machine.

- **Saturation flux ψ_sat** (squirrel-cage, double-cage and wound-rotor motors): the magnetising current becomes ψ/Lm·(1 + (ψ/ψ_sat)^k), so the magnetising inductance halves at ψ_sat. A sharpness k sets the knee. 0 keeps the machine linear, as before.
- **Iron-loss resistance R_fe**: above 0, the motor gets one more output, the iron loss P_fe = 1.5·|e_m|²/R_fe from the magnetising EMF. It is an estimate for loss and efficiency studies; the model's currents do not include it.
- Models made before this release run exactly as they did.

Checked: at no load and synchronous speed, the stator current's peak matches the no-load phasor equation within 0.2 %, linear and saturated. Saturation raises it by more than 30 % on 400 V. The iron loss matches 1.5·(ω·ψ_m)²/R_fe within 0.5 %.

Saturation of the PMSM is not in this release.

## 0.3.3 — 2026-11-10

Inverter losses and junction temperature.

- **Inverter losses** (Library › Drives) gives the conduction and switching losses of a two-level inverter from data-sheet values: the on-state voltage and resistance, and the switching energy at a reference current and voltage. Its inputs are the phase currents and the DC link voltage. It outputs the conduction, switching and total loss.
- **Thermal network** turns heat into temperature: a Foster network of R–C stages from junction to ambient, taken from the data sheet's transient thermal impedance. One stage is a simple thermal mass.
- Feed the losses into the thermal network to watch the junction temperature through a load cycle.

Checked: currents of (10, −5, −5) A on 600 V give exactly 23.5 W of conduction loss and 8 W of switching loss. A 100 W step into a three-stage network follows 40 + Σ 100·Rᵢ·(1 − e^(−t/τᵢ)) within 1e-5 °C.

The roadmap moves up one number from here: saturation is now 0.3.4, nameplate 0.3.5, the robot arm 0.3.6 and drive examples 0.3.7.

## 0.3.2 — 2026-11-10

Space-vector modulation and dead time.

- **Inverter › Modulation: space vector** adds the common-mode offset −(max + min)/2 to the references. The linear range then reaches 2/√3 ≈ 1.155 instead of 1, about 15 % more voltage from the same DC link. It works in the average and the switched model.
- **PWM › Modulation: space vector** does the same for three gate references.
- **Inverter › Dead time**: above 0, the inverter takes the phase currents as a third input. Each leg loses V_dc·t_d·f_s against its current, the average effect of the dead time. It fades smoothly near zero current, over a current band you set.

Checked: references of amplitude 1.1 give phase voltages of exactly 1.1·V_dc/2 with space vector and are clipped without it. A dead time of 2 µs at 10 kHz on 600 V shifts the phase voltages by −4/3 and +2/3 of 12 V, as the formula says.

## 0.3.1 — 2026-11-10

Drives in depth begins: spectrum and harmonic distortion.

- **Analyze › Spectrum** (or a scope's More ▾ › Spectrum and THD) shows the harmonics of any logged signal over the last whole cycles of its fundamental, with the **THD**, the fundamental's amplitude and rms value, the DC part and the largest harmonics.
- Leave the fundamental empty to find it, or type it. Choose how many cycles and how many harmonics.

Checked: a 50 Hz signal with 10 % fifth and 5 % seventh harmonics and an offset gives an 11.18 % THD. Each harmonic and the DC part come out within 1e-4, with the fundamental given or found.

## 0.3.0 — 2026-11-10

Control design, complete: the first stage of the roadmap is done. 0.3.0 brings together 0.2.2 to 0.2.10:

- Every planned feature shows in the window, dimmed, with the release that brings it.
- Analysis points on wires and **Loop Analysis** of the whole model: Bode with gain and phase margins, Nyquist, root locus, sensitivity, closed-loop step and impulse, poles.
- **Trim** and saved operating points.
- The **PID Tuner** (speed and robustness sliders, Ziegler–Nichols, SIMC) and one-click **Drive Tuning**.
- **Live tuning** while the model runs, and real-time pacing.
- **Buses** and solved **algebraic loops**.

New in 0.3.0:
- The PMSM speed-control example is ready for Control: its speed loop opens at the speed PI.
- Loop Analysis tells a truly unstable loop from one with a pole at the origin that the loop does not use, such as a rotor angle.
- A verification record for the whole stage (docs/verification/2026-11-10.md).

## 0.2.10 — 2026-11-10

Algebraic loops are solved.

- A loop with no state in it, such as y = u − k·y or y + y³ = 2, used to be an error. It is now **solved at each step** by Newton's method on one block in the loop, with a warning that names the loop. A small delay or filter in the loop avoids the cost.
- If such a loop has no solution, for example y = 1 + y, the run stops with an error that names the loop.
- Linearising and trim work through these loops too.

Checked: y = 3 − 2y gives 1, and y + y³ = 2 gives 1. A loop with a state, x′ = −y with y = x − 2y, decays as e^(−t/3).

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
