// "?" help: a small button beside a tool; a click opens a short card saying what the tool is for
// and how to use it. One place for every explanation, so the whole app teaches itself. Blocks get
// their card from the block library (the engine's definitions), so the help never drifts.
import { For, Show } from "solid-js";
import { openDialog, defOf } from "./store";
import { Dialog } from "./ui";

type Topic = { t: string; what: string; how?: string[]; tip?: string };
export const HELP: Record<string, Topic> = {
  // Model tab
  clipboard: { t: "Clipboard", what: "Cut, copy and paste blocks with the wires between them.", how: ["Select blocks, Ctrl+C, then Ctrl+V: the copies appear a little to the right.", "Paste works inside subsystems too."] },
  blocks: { t: "Blocks", what: "Add, delete, flip and rename blocks.",
    how: ["Drag a block from the Library panel on the left onto the diagram, or double-click it in the library.", "Flip turns a block round so its inputs are on the right: handy for feedback paths.", "Delete (Del) removes the selected blocks and wires."] },
  subsystems: { t: "Subsystems", what: "A subsystem is a group of blocks shown as one block, with its own diagram inside. Its In and Out blocks are its ports.",
    how: ["Select blocks and click Create Subsystem: wires that cross the edge become In and Out ports.", "Double-click a subsystem to open it; the path above the diagram takes you back.", "Ungroup puts the blocks back into the diagram."], tip: "Subsystems keep big models readable: a motor, a tank or a controller each in its own box." },
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
  show: { t: "Show", what: "The grid dots, snapping blocks to the grid while dragging, and the scopes dock under the diagram." },
  // Panels
  library: { t: "Library", what: "Every block, by category. Planned blocks are dimmed with the release they arrive in.",
    how: ["Type to search by name.", "Drag a block onto the diagram, or double-click it to add it in the middle.", "Click ? on a block for its equations and how to use it."] },
  canvas: { t: "Diagram", what: "The model as blocks joined by wires. Signals flow along the wires from outputs (right) to inputs (left).",
    how: ["Drag from an output port to an input port to draw a wire; drop on a wire's end to reconnect it.", "Drag blocks to move them; drag on empty space to select an area; Shift or Ctrl adds to the selection.", "Double-click a block to edit it; double-click a scope to open it, a subsystem to go inside.", "Space-drag or the middle mouse button pans; Ctrl + wheel zooms."] },
  inspector: { t: "Inspector", what: "The selected block's parameters. Values can be numbers, vectors [1 2 3], matrices [1 2; 3 4] or expressions using model variables." },
  scopes: { t: "Scopes", what: "Plots of the signals going into each Scope block, live while the model runs.",
    how: ["Drag on a plot to zoom in time; double-click to zoom out.", "Hover to read the values under the cursor.", "PNG saves a picture; CSV saves the data."] },
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
