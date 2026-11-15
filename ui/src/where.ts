// "Where is…?" (0.6.1): everyday tasks in plain words, and where their tool lives. Help ›
// Where Is…? lists them; Tell Me (Ctrl+K) finds them by these words too. Choosing one opens the
// tab and highlights the button, so people learn where things are instead of only running them.
import { setTab, setBackstage } from "./store";

export type Task = { task: string; words: string; tab: string; tabName: string; group: string; button: string };

const t = (task: string, words: string, tab: string, group: string, button: string): Task =>
  ({ task, words, tab, tabName: TAB_NAMES[tab] ?? tab, group, button });

export const TAB_NAMES: Record<string, string> = { model: "Model", simulate: "Simulate", analyze: "Analyze", results: "Results", modules: "Modules", view: "View", help: "Help" };

export const TASKS: Task[] = [
  // Build
  t("Add a block", "insert new block library part", "model", "Blocks", "Add Block"),
  t("Open an example", "sample demo tutorial start", "model", "Examples", "Examples"),
  t("Group blocks into a subsystem", "subsystem group nest", "model", "Subsystems", "Create Subsystem"),
  t("Give a subsystem its own parameters", "mask parameters icon", "model", "Subsystems", "Mask"),
  t("Set model variables (Kp, R, L …)", "variables parameters workspace constants", "model", "Variables", "Variables"),
  t("Find a block or a signal", "search find locate", "model", "Arrange", "Find"),
  t("Tidy the diagram", "align arrange snap grid fit", "model", "Arrange", "Align Left"),
  t("Add a motor or a drive", "motor drive inverter pmsm induction dc", "modules", "Motors and Drives", "Motors"),
  t("Add a robot arm", "robot arm urdf joints manipulator", "modules", "Robotics", "Arms"),
  t("Draw a circuit (R, L, C, switches, diodes)", "electrical circuit resistor capacitor inductor diode switch converter", "modules", "Physical Parts", "Electrical"),
  t("Draw mechanics (inertias, gears, masses, springs)", "mechanical gear inertia mass spring damper friction rotational translational", "modules", "Physical Parts", "Rotational"),
  t("Draw heat flow (heat capacity, convection)", "thermal heat temperature cooling", "modules", "Physical Parts", "Thermal"),
  t("Draw hydraulics (pump, valve, cylinder)", "hydraulic oil pump cylinder valve pressure", "modules", "Physical Parts", "Hydraulic"),
  // Run
  t("Run the model", "simulate start play go", "simulate", "Run", "Run"),
  t("Set the stop time or the solver", "time stop solver step tolerance stiff", "simulate", "Solver", "Solver"),
  t("Run once for each value of a variable", "sweep parameter study batch", "simulate", "Studies", "Parameter Sweep"),
  t("Run in real time, or slower", "pace real time speed slow", "simulate", "Pacing", "Speed"),
  t("Change gains while the model runs", "live tuning sliders tune online", "simulate", "Live", "Live Tuning"),
  // Analyse
  t("Bode plot and margins of a control loop", "bode margins gain phase loop frequency response linearise", "analyze", "Loop (Whole Model)", "Loop Analysis"),
  t("Nyquist plot or root locus", "nyquist root locus stability poles", "analyze", "Loop (Whole Model)", "Nyquist"),
  t("Mark where to open a loop", "analysis point loop opening input output", "analyze", "Analysis Points", "Loop Opening"),
  t("Bode or step response of one block", "bode step transfer function block poles zeros", "analyze", "Single Block", "Bode"),
  t("Find an operating point (trim)", "trim equilibrium steady state operating point", "analyze", "Operating Points", "Trim"),
  t("Tune a PID controller", "pid tune tuner gains controller", "analyze", "Tuning", "PID Tuner"),
  t("Tune a motor drive's loops", "drive tuning current loop speed loop foc", "analyze", "Tuning", "Drive Tuning"),
  t("Rise time, overshoot and settling", "step specs rise overshoot settling", "analyze", "Signals", "Step Specs"),
  t("Harmonics and THD", "spectrum fft harmonics thd frequency", "analyze", "Signals", "Spectrum"),
  // Results
  t("Show the scopes", "scope plot graph signals curves", "results", "Scopes", "Scopes"),
  t("Read any wire after a run", "probe log all signals hover", "results", "Scopes", "Log all signals"),
  t("See the results as numbers", "results table values data", "results", "Results", "Results"),
  t("Save results as CSV", "export csv save data excel", "results", "Export", "Save as CSV"),
  t("Send results to Datum Plot", "datum plot export figure", "results", "Export", "Send to Datum Plot"),
  // View and help
  t("Zoom the diagram", "zoom in out fit", "view", "Zoom", "Fit"),
  t("Colour blocks by sample time", "sample time rates colours discrete", "view", "Show", "Sample-time colours"),
  t("Hide the planned (coming) features", "planned coming future dimmed", "view", "Planned", "Planned features"),
  t("Run models from Python or the command line", "python automation command line cli script", "help", "Learn", "Automation"),
  t("Check for updates", "update version new", "help", "Updates", "Check for Updates"),
];

/** Open a tab and highlight one of its buttons (or fields) for a moment. */
export const goTo = (w: Task) => {
  setBackstage(null);
  setTab(w.tab);
  setTimeout(() => {
    const els = [...document.querySelectorAll<HTMLElement>(".ribbon .rb, .ribbon label, .ribbon .grp .cap")];
    const el = els.find(e => e.textContent?.trim().startsWith(w.button)) ?? els.find(e => e.classList.contains("cap") && e.textContent?.startsWith(w.group));
    const box = el?.closest<HTMLElement>(".rbw, label, .grp") ?? el;
    if (!box) return;
    box.classList.add("flash");
    box.scrollIntoView({ block: "nearest", inline: "nearest" });
    setTimeout(() => box.classList.remove("flash"), 2400);
  }, 60);
};
