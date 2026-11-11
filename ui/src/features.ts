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
  f("r-ctl", "modules", "Robotics", "0.4.5", "Arm Controllers", "specs", "Computed torque and PD + gravity for any arm; mass, gravity and velocity torque blocks; motors on joints through gears."),
  f("r-grip", "modules", "Robotics", "0.4.6", "Gripper and Contact", "block", "A gripper that grasps and releases objects, with simple contact between the robot, objects and a table."),
  f("r-task", "modules", "Robotics", "0.4.7", "Task Scheduler", "sub", "A small state machine for pick and place: states, conditions and outputs."),

  // ---- Stage C: physical components (0.4.x → 0.5.0)
  f("phys-rot", "modules", "Physical", "0.5.1", "Rotational", "block", "Inertia, spring, damper, gear and friction, connected by ports (the equations come from the connections)."),
  f("phys-trans", "modules", "Physical", "0.5.2", "Translational", "block", "Mass, spring, damper and hard stop, connected by ports."),
  f("phys-elec", "modules", "Physical", "0.5.3", "Electrical", "block", "Resistor, inductor, capacitor, sources, switch, diode and transformer."),
  f("phys-therm", "modules", "Physical", "0.5.4", "Thermal", "block", "Heat capacity, conduction and convection."),
  f("phys-motor", "modules", "Physical", "0.5.5", "Motors on Ports", "motor", "The motor blocks connected to electrical and mechanical networks."),
  f("phys-hyd", "modules", "Physical", "0.5.6", "Hydraulics", "block", "Pump, valve, cylinder and pipe."),

  // ---- Stage D: exchange (0.5.x)
  f("fmuexp", "analyze", "Exchange", "0.6.1", "FMU Export", "export", "Export the model as an FMU (co-simulation) to run inside other tools."),
  f("fmuimp", "model", "Exchange", "0.6.2", "FMU Import", "import", "Run another tool's FMU as a block."),
  f("hdf5", "analyze", "Results", "0.6.3", "MAT and HDF5", "export", "Save results as MAT or HDF5 files."),
  f("modelica", "model", "Exchange", "0.6.4", "Modelica", "import", "Modelica models through an external compiler."),

  // ---- Stage E: studies and logic (0.6.x)
  f("chart", "model", "Logic", "0.7.1", "State Machine", "sub", "States, transitions and actions drawn as a chart."),
  f("pyblock", "model", "Logic", "0.7.2", "Python Block", "py", "A block whose behaviour is a Python function."),
  f("mc", "simulate", "Studies", "0.7.3", "Monte Carlo", "sweep", "Run many times with parameters drawn from distributions; spread of the results."),
  f("sensi", "simulate", "Studies", "0.7.3", "Sensitivity", "specs", "Which parameters change a result most."),
  f("optim", "simulate", "Studies", "0.7.4", "Optimise and Fit", "plot", "Find parameters that meet goals, or that make the model match measured data."),
  f("report", "analyze", "Results", "0.7.5", "Report", "report", "The model, its runs and its plots in one printable report."),

  // ---- Stage F and modules later
  f("os", "view", "Reach", "0.8.1", "Linux and macOS", "info", "Installers for Linux and macOS."),
  f("fast", "simulate", "Solver", "0.8.2", "Fast Mode", "run", "A compiled run mode for big models."),
  f("variants", "model", "Signals", "0.8.3", "Variants", "sub", "Switchable subsystems, and a comparison of two model files."),
  f("flight", "modules", "Modules", "later", "Flight", "motor", "A module for aircraft and drones: rigid-body dynamics, aerodynamics, autopilot loops."),
  f("vehicle", "modules", "Modules", "later", "Vehicle", "motor", "A module for electric vehicles: drive cycle, battery, motor and driveline."),
  f("bench", "modules", "Hardware", "later", "Hardware Bench", "run", "Connect the model to real controllers and sensors, with the planned Datum bench app."),
];

/** The planned features for a ribbon tab. */
export const planned = (_a: unknown, q: { tab?: string }) => FEATS.filter(x => !q.tab || x.tab === q.tab);
