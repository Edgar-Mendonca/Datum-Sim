// The register of planned features (0.2.2, the pattern of Datum Mesh): every feature not built yet,
// on the ribbon tab it will live on, with the release that brings it and the dialog it will have.
// When a feature is built, its entry leaves this list and its real button takes the place.
// Keep it in step with docs/ROADMAP.md.

export type PField =
  | { k: "num"; l: string; v?: string; u?: string }
  | { k: "choice"; l: string; o: string[] }
  | { k: "check"; l: string; on?: boolean }
  | { k: "text"; l: string; v?: string }
  | { k: "pick"; l: string; what: string }
  | { k: "table"; l: string; cols: string[]; rows?: string[][] };

export interface Feat { id: string; label: string; icon: string; tab: string; group: string; v: string; what: string; fields: PField[] }

const f = (id: string, tab: string, group: string, v: string, label: string, icon: string, what: string, fields: PField[] = []): Feat => ({ id, label, icon, tab, group, v, what, fields });

export const FEATS: Feat[] = [
  // ---- Stage A: control design (0.2.x → 0.3.0)

  // ---- Stage B: drives and modules (0.3.x → 0.4.0)

  // ---- Stage R: robotics (0.4.x → 0.5.0), before physical components
  f("r-arc", "modules", "Robotics", "later", "Arc Tool Paths", "solver", "Tool paths along arcs and with orientation changes."),

  // ---- Stage C: physical components (0.4.x → 0.5.0)

  // ---- Stage S: scripting, the Datum way (0.6.x → 0.7.0)
  f("console", "analyze", "Scripting", "0.6.1", "Python Console", "py", "Type Python beside the diagram: the open model is ready as `model` (variables, run, results as arrays), with the control commands tf, step, bode and lqr.", [{ k: "text", l: ">>>", v: "model.vars['Kp'] = 3; r = model.run()" }]),
  f("scripts", "analyze", "Scripting", "0.6.2", "Scripts", "report", "Python scripts kept with the model; a setup script runs before every simulation, so variables can come from a calculation.", [{ k: "pick", l: "Setup script", what: "script" }, { k: "check", l: "Run before every simulation", on: true }]),
  f("buildcode", "analyze", "Scripting", "0.6.3", "Build from Code", "sub", "Add blocks and wires from Python, with undo like any edit."),
  f("pyblock", "model", "Logic", "0.6.4", "Python Block", "py", "A block whose behaviour is a Python function."),
  f("mimport", "model", "Exchange", "0.6.5", "Import Parameters (.m)", "import", "Read plain assignments (J = 0.01; wn = 2*pi*50;) from a parameter file into model variables. The file is read, never run.", [{ k: "pick", l: "Parameter file", what: "file" }]),

  // ---- Stage U: easy to find your way (0.7.x → 0.8.0), its own plan first

  // ---- Stage D: exchange (0.8.x → 0.9.0)
  f("fmuexp", "analyze", "Exchange", "0.8.1", "FMU Export", "export", "Export the model as an FMU (co-simulation) to run inside other tools."),
  f("fmuimp", "model", "Exchange", "0.8.2", "FMU Import", "import", "Run another tool's FMU as a block."),
  f("hdf5", "analyze", "Results", "0.8.3", "MAT and HDF5", "export", "Save results as MAT or HDF5 files."),
  f("modelica", "model", "Exchange", "0.8.4", "Modelica", "import", "Modelica models through an external compiler."),

  // ---- Stage E: studies and logic (0.9.x → 0.10.0)
  f("chart", "model", "Logic", "0.9.1", "State Machine", "sub", "States, transitions and actions drawn as a chart."),
  f("mc", "simulate", "Studies", "0.9.2", "Monte Carlo", "sweep", "Run many times with parameters drawn from distributions; spread of the results."),
  f("sensi", "simulate", "Studies", "0.9.2", "Sensitivity", "specs", "Which parameters change a result most."),
  f("optim", "simulate", "Studies", "0.9.3", "Optimise and Fit", "plot", "Find parameters that meet goals, or that make the model match measured data."),
  f("report", "analyze", "Results", "0.9.4", "Report", "report", "The model, its runs and its plots in one printable report."),

  // ---- Stage F and modules later
  f("os", "view", "Reach", "0.10.1", "Linux and macOS", "info", "Installers for Linux and macOS."),
  f("fast", "simulate", "Solver", "0.10.2", "Fast Mode", "run", "A compiled run mode for big models."),
  f("variants", "model", "Signals", "0.10.3", "Variants", "sub", "Switchable subsystems, and a comparison of two model files."),
  f("flight", "modules", "Modules", "later", "Flight", "motor", "A module for aircraft and drones: rigid-body dynamics, aerodynamics, autopilot loops."),
  f("vehicle", "modules", "Modules", "later", "Vehicle", "motor", "A module for electric vehicles: drive cycle, battery, motor and driveline."),
  f("bench", "modules", "Hardware", "later", "Hardware Bench", "run", "Connect the model to real controllers and sensors, with the planned Datum bench app."),
];

/** The planned features for a ribbon tab. */
export const planned = (_a: unknown, q: { tab?: string }) => FEATS.filter(x => !q.tab || x.tab === q.tab);
