// "?" help: a small button beside a tool; a click opens a short card saying what the tool is for
// and how to use it. One place for every explanation, so the whole app teaches itself. Blocks get
// their card from the block library (the engine's definitions), so the help never drifts.
import { For, Show } from "solid-js";
import { openDialog, defOf } from "./store";
import { Dialog } from "./ui";

type Topic = { t: string; what: string; how?: string[]; tip?: string };
export const HELP: Record<string, Topic> = {
  // Model tab
  points: { t: "Analysis points", what: "Mark where to look in the model: a loop opening (the loop is cut there and the loop transfer L is measured), an input (a test signal is added) or an output (the signal is measured).", how: ["Select a wire, then Control › Loop Opening, Input or Output; or right-click the wire › Analysis point.", "A point belongs to the signal: every branch of the wire shows it.", "Runs ignore analysis points; they are saved with the model."] },
  loop: { t: "Loop analysis", what: "The engine linearises the whole model between its analysis points around an operating point (central differences on the compiled model, so motors and nonlinear blocks work too) and analyses the result.", how: ["Choose the operating point: the initial state, or the state at a time in the last run.", "From a loop opening to the same loop opening gives the loop transfer L: Bode with gain and phase margins, and the closed loop's step response.", "From an input to an output gives that transfer: Bode, step and poles.", "Nyquist draws L(jω) with the −1 point; the count of encirclements and the open-loop poles in the right half-plane give the stability verdict (N = Z − P).", "Root locus: the closed-loop poles as the loop gain is scaled from 0.001 to 1000 times (×: open-loop poles, +: the poles at the present gain).", "Sensitivity: |S| = |1/(1 + L)| and |T| = |L/(1 + L)|; the peak Ms of |S| is a robustness figure (below 2 is good).", "Copy A, B, C, D to use the linear model elsewhere."], tip: "Discrete blocks are held at their values, so a digital controller's sampling is not in the linear model." },
  trim: { t: "Trim and operating points", what: "Trim finds an operating point: the states where every derivative is zero, with the analysis-point outputs you choose held at their values, changing the model variables you allow.", how: ["Mark the signals to hold as analysis Outputs (Control › Output).", "Control › Trim: type the values to hold, tick the variables trim may change, and press Find.", "Save the point: Loop Analysis can then linearise around it (Operating point › your point).", "Control › Operating Points lists the saved points."], tip: "A held output needs something free to move it: a variable, or a controller's integrator. If trim cannot meet everything, it says which equations are unmet." },
  pidtune: { t: "PID Tuner", what: "Tunes a PID block in its real loop: the model is linearised with the loop cut at the PID's output, giving the plant the PID sees.", how: ["Select the PID and press Control › PID Tuner.", "Speed sets the loop's crossover frequency; Robustness sets its phase margin. The gains follow as you move the sliders, with the closed-loop step and |L| before and after.", "Ziegler–Nichols uses the plant's ultimate gain and period; SIMC fits a first-order-plus-dead-time model to the plant's step response.", "Apply changes P, I, D and N in one undoable edit; if a gain is a model variable, the variable changes."], tip: "Tune where the PID is not at its output limits: the model's initial state is used." },
  drivetune: { t: "Drive Tuning", what: "Tunes a motor drive's cascade from the motor's data: the current PIs (q and d, or the armature) and the speed PI.", how: ["Select the motor (DC, BLDC or PMSM) and press Control › Drive Tuning.", "Choose the current-loop and speed-loop bandwidths; the speed loop is usually ten times slower.", "Say which PID is which; Apply sets their P and I (D = 0) in one undoable edit."], tip: "The design assumes the current loop follows its reference; limits, the inverter and sampling are not in it, so check with a run." },
  live: { t: "Live tuning and pacing", what: "Change model variables while the model runs and watch the scopes respond; slow the run to real time so you can follow it.", how: ["Control › Live Tuning opens the panel. Add a variable: it gets a slider with a range around its value (edit the ends).", "Run the model and move a slider: the blocks that use the variable are rebuilt with the new value at the next step; states are kept, so nothing jumps except what you changed.", "Keep these values writes the slider values to the variables, as one undoable edit.", "Simulate › Pacing slows the run to real time (or ×0.1 to ×10); it can change while running."], tip: "Blocks rebuilt live start their internal histories again (a delay's buffer, a relay's state); continuous and discrete states are kept." },
  fft: { t: "Spectrum and THD", what: "The harmonics of a signal over the last whole cycles of its fundamental (an FFT of the logged signal), and its total harmonic distortion.", how: ["Run the model to steady state, then Analyze › Spectrum, or a scope's More ▾ › Spectrum and THD.", "Leave the fundamental empty to find it, or type it (50 Hz).", "THD = √(Σ Aₕ²) / A₁ over h = 2 to the highest harmonic you choose."], tip: "A whole number of cycles avoids leakage; log enough points for the harmonics you want (a smaller largest step, or a fixed-step solver)." },
  nameplate: { t: "From nameplate", what: "Estimates an induction motor's equivalent circuit (Rs, Rr, Lls = Llr, Lm, pole pairs) from its rating plate and data sheet.", how: ["Select the motor, then the Inspector's From nameplate….", "Type the rated power, line voltage, frequency, speed, efficiency and power factor, and the starting current ratio I_A/I_N from the data sheet (7 is typical).", "Apply sets the six parameters in one undoable edit."], tip: "The stator copper loss comes from the efficiency and the slip, with iron and friction losses neglected; a very high efficiency for the slip has no matching circuit." },
  robot: { t: "Robot arm module", what: "Blocks for a two-link planar robot arm: the arm's dynamics, a smooth joint trajectory through way points, a joint controller (computed torque or PD with gravity compensation), inverse kinematics, and an animated Arm view.", how: ["Modules › Robot Arm Example opens a ready model: run it, then double-click the Arm view and press Play.", "Change the way points in the Trajectory (rows of time, q1, q2).", "Make the controller's model differ from the arm (another m2) to see how computed torque degrades, or switch to PD + gravity."], tip: "q1 is measured from the horizontal and q2 from link 1; gravity acts downwards." },
  planned: { t: "Planned features", what: "Features still to come show dimmed on their ribbon tab, each with the release that brings it.", how: ["Click one to see the dialog it will have.", "File › Roadmap lists them all by release.", "Untick View › Planned features to hide them."] },
  clipboard: { t: "Clipboard", what: "Cut, copy and paste blocks with the wires between them.", how: ["Select blocks, Ctrl+C, then Ctrl+V: the copies appear a little to the right.", "Paste works inside subsystems too."] },
  blocks: { t: "Blocks", what: "Add, delete, flip and rename blocks.",
    how: ["Drag a block from the Library panel on the left onto the diagram, or double-click it in the library.", "Flip turns a block round so its inputs are on the right: handy for feedback paths.", "Delete (Del) removes the selected blocks and wires."] },
  subsystems: { t: "Subsystems", what: "A subsystem is a group of blocks shown as one block, with its own diagram inside. Its In and Out blocks are its ports.",
    how: ["Select blocks and click Create Subsystem: wires that cross the edge become In and Out ports.", "Double-click a subsystem to open it; the path above the diagram takes you back.", "Ungroup puts the blocks back into the diagram.", "Mask: give a subsystem parameters of its own (the blocks inside use them by name), a text on the block, help and typical ratings. A masked subsystem is set like any block; right-click › Look under the mask to go inside.", "Save to My blocks: keep a subsystem in your library, for every model. A model gets its own copy each time you add one."], tip: "Subsystems keep big models readable: a motor, a tank or a controller each in its own box." },
  arrange: { t: "Arrange", what: "Tidy the diagram: align the selected blocks, snap to the grid, and zoom to fit." },
  vars: { t: "Model variables", what: "Named values (R, L, Kp …) that block parameters use. Change a variable once and every block that uses it follows.",
    how: ["Model › Variables opens the table: a name, a value or expression (e.g. 2*zeta*sqrt(k*m)), a unit and a note.", "Variables may use earlier ones.", "A parameter sweep runs the model for several values of one variable."] },
  // Simulate tab
  run: { t: "Run", what: "Runs the model from the start time to the stop time. The scopes fill while it runs.",
    how: ["Run (F5) starts; Stop ends it early; Pause holds it; Step advances one solver step while paused.", "Errors point at the block that caused them: click one in the Issues view to select the block."] },
  time: { t: "Time", what: "When the simulation starts and stops, in seconds." },
  solver: { t: "Solver", what: "How the equations are stepped through time.",
    how: ["Variable step (Dormand–Prince 4/5): accurate and fast for most models; it shortens its steps where things change quickly.", "Stiff (Rosenbrock 2/3): for models with very fast and very slow parts together (chemistry, electrical circuits, tight control loops).", "Fixed step (RK4, Heun, Euler): the same step every time, as on a real-time controller."],
    tip: "Relative tolerance 1e-6 is a good default. Tighten it if results change when you rerun with a smaller tolerance." },
  sweep: { t: "Parameter sweep", what: "Runs the model once for each value of a model variable and overlays the results in the scopes.",
    how: ["Pick a variable and type values such as 0.1 0.5 1, or linspace(0.1, 1, 5).", "Each run is drawn in its own colour; the legend shows the values."] },
  check: { t: "Check model", what: "Checks the model without running it: parameter errors, sizes that do not fit, unconnected inputs and algebraic loops." },
  // Analyze tab
  linear: { t: "Linear analysis", what: "For a linear block (transfer function, zero-pole-gain, state space, PID, gain, integrator or DC motor): the Bode plot, step response, poles and zeros, and the gain and phase margins.",
    how: ["Select the block, then click Bode, Step response or Poles and zeros.", "The analysis runs in the built-in Python engine (python-control); the first one starts the engine."] },
  specs: { t: "Step-response specifications", what: "Rise time, peak and overshoot, settling time and the final value of a signal from the last run.", how: ["Run the model, then pick the signal."] },
  exportres: { t: "Results", what: "Save the results as CSV for a spreadsheet or Datum Plot, and see them as a table." },
  // View tab
  views: { t: "Views", what: "The model's pages: the diagram, the variables, the scopes, the results table, analyses and issues." },
  zoom: { t: "Zoom", what: "How large the diagram is drawn. Ctrl + wheel zooms; Fit shows the whole diagram." },
  show: { t: "Show", what: "The grid dots, snapping blocks to the grid while dragging, the scopes dock under the diagram, and sample-time colours: every block coloured by how often its output changes." },
  // Panels
  library: { t: "Library", what: "Every block, by category. Planned blocks are dimmed with the release they arrive in.",
    how: ["Type to search by name.", "Drag a block onto the diagram, or double-click it to add it in the middle.", "Click ? on a block for its equations and how to use it."] },
  canvas: { t: "Diagram", what: "The model as blocks joined by wires. Signals flow along the wires from outputs (right) to inputs (left).",
    how: ["Drag from an output port to an input port to draw a wire; drop on a wire's end to reconnect it.", "Alt-drag (or Ctrl-drag) from a wire to branch its signal to another input.", "Drag a wire's middle segment to move it; double-click a bend to remove it; double-click a wire to name its signal.", "Drag blocks to move them; drag on empty space to select an area; Shift or Ctrl adds to the selection.", "Double-click a block to edit it; double-click a scope to open it, a subsystem to go inside.", "Ctrl+F finds a block or a named signal anywhere in the model.", "Space-drag or the middle mouse button pans; Ctrl + wheel zooms."] },
  inspector: { t: "Inspector", what: "The selected block's parameters. Values can be numbers, vectors [1 2 3], matrices [1 2; 3 4] or expressions using model variables." },
  scopes: { t: "Scopes", what: "Plots of the signals going into each Scope block, live while the model runs.",
    how: ["Drag sideways on a plot to zoom in time, up or down to zoom the y axis; double-click to see the whole run.", "Cursors: two lines A and B you drag; the table gives each signal at A and B and the difference.", "Marks: rise time, overshoot, peak and settling time of the first signal, drawn on the plot.", "Compare: overlay any of the last five runs, dashed.", "More: pictures (PNG as shown, PNG at three times the size, SVG), CSV, one plot per input, and a fixed y range. Layout and range are saved in the Scope block.", "Turn on Simulate › Log all signals, run, and hover any wire to read its value at cursor A (or the end of the run)."] },
  issues: { t: "Issues", what: "Errors stop a run; warnings do not. Click one to select its block." },
};

export const blockTopic = (kind: string): Topic | null => {
  const d = defOf(kind);
  if (!d) return null;
  const how = [d.how, ...d.params.map(p => `${p.label}${p.unit ? ` (${p.unit})` : ""}: ${p.hint || `default ${p.def || "empty"}`}`)].filter(Boolean);
  return { t: d.name, what: d.what, how };
};

/** The small "?" beside a tool. */
export const Help = (p: { k: string }) => <button class="help" title={`What is ${HELP[p.k]?.t ?? "this"}?`} onClick={e => { e.stopPropagation(); helpCard(p.k); }}>?</button>;

export const helpCard = (k: string) => openDialog(() => {
  const h = k.startsWith("block:") ? blockTopic(k.slice(6)) : HELP[k];
  return (
    <Dialog title={h?.t ?? "Help"}>
      <div class="helpcard"><p>{h?.what}</p>
        <Show when={h?.how?.length}><h4>How to use it</h4><ul><For each={h!.how}>{x => <li>{x}</li>}</For></ul></Show>
        <Show when={h?.tip}><p class="muted">{h!.tip}</p></Show></div>
    </Dialog>
  );
});
