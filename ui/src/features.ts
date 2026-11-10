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
  f("drivetune", "control", "Tuning", "0.2.7", "Drive Tuning", "motor", "One click tunes a drive's current loop and speed loop from the motor's resistance, inductance and inertia.",
    [{ k: "pick", l: "Motor", what: "a motor block" }, { k: "num", l: "Current-loop bandwidth", v: "2000", u: "rad/s" }, { k: "num", l: "Speed-loop bandwidth", v: "200", u: "rad/s" }]),
  f("live", "control", "Live", "0.2.8", "Live Tuning", "var", "Sliders for variables marked as tunable: move one while the model runs and the scopes show the effect at once.",
    [{ k: "table", l: "Tunable", cols: ["Variable", "Min", "Max"] }]),
  f("pace", "simulate", "Pacing", "0.2.8", "Real Time", "run", "Slow a run to real time, or to a chosen speed (×0.1 to ×10), to tune by hand.", [{ k: "num", l: "Speed", v: "1", u: "× real time" }]),
  f("bus", "model", "Signals", "0.2.9", "Buses", "wire", "Group named signals into one wire (Bus Creator) and pick them out by name (Bus Selector)."),
  f("algloop", "simulate", "Solver", "0.2.10", "Algebraic Loops", "solver", "A loop without a state is solved at each step (Newton iteration) instead of being reported as an error."),

  // ---- Stage B: drives and modules (0.3.x → 0.4.0)
  f("fft", "analyze", "Signals", "0.3.1", "Spectrum (FFT)", "bode", "The spectrum of a signal over whole cycles of its fundamental, with the harmonics and the THD.",
    [{ k: "pick", l: "Signal", what: "a signal" }, { k: "num", l: "Fundamental", v: "50", u: "Hz" }, { k: "num", l: "Cycles", v: "4" }, { k: "num", l: "Harmonics up to", v: "40" }]),
  f("svpwm", "model", "Drives", "0.3.2", "Space-Vector PWM", "motor", "Space-vector modulation and dead time in the PWM and inverter blocks.", [{ k: "choice", l: "Modulation", o: ["Sine", "Space vector"] }, { k: "num", l: "Dead time", v: "2e-6", u: "s" }]),
  f("losses", "model", "Drives", "0.3.3", "Inverter Losses", "motor", "Conduction and switching losses of the inverter from data-sheet values, and its efficiency.",
    [{ k: "num", l: "On-state voltage", v: "1.1", u: "V" }, { k: "num", l: "On-state resistance", v: "0.01", u: "Ω" }, { k: "num", l: "Switching energy", v: "2e-3", u: "J" }]),
  f("thermal", "model", "Drives", "0.3.4", "Thermal Network", "motor", "Heat from the losses into a thermal network: junction, case and heat sink.", [{ k: "table", l: "Stages", cols: ["R (K/W)", "τ (s)"] }]),
  f("sat", "model", "Drives", "0.3.5", "Saturation", "motor", "Magnetic saturation and iron losses in the induction machine and PMSM."),
  f("nameplate", "model", "Drives", "0.3.6", "From Nameplate", "motor", "The machine's equivalent circuit estimated from its nameplate: power, voltage, current, speed, efficiency, power factor.",
    [{ k: "num", l: "Power", v: "11", u: "kW" }, { k: "num", l: "Voltage", v: "400", u: "V" }, { k: "num", l: "Frequency", v: "50", u: "Hz" }, { k: "num", l: "Speed", v: "1460", u: "rpm" }, { k: "num", l: "Efficiency", v: "0.9" }, { k: "num", l: "Power factor", v: "0.85" }]),
  f("robot", "modules", "Modules", "0.3.7", "Robot Arm", "motor", "A two-link robot arm with joint motors, a trajectory planner, joint controllers and an animated view of the arm."),

  // ---- Stage C: physical components (0.4.x → 0.5.0)
  f("phys-rot", "modules", "Physical", "0.4.1", "Rotational", "block", "Inertia, spring, damper, gear and friction, connected by ports (the equations come from the connections)."),
  f("phys-trans", "modules", "Physical", "0.4.2", "Translational", "block", "Mass, spring, damper and hard stop, connected by ports."),
  f("phys-elec", "modules", "Physical", "0.4.3", "Electrical", "block", "Resistor, inductor, capacitor, sources, switch, diode and transformer."),
  f("phys-therm", "modules", "Physical", "0.4.4", "Thermal", "block", "Heat capacity, conduction and convection."),
  f("phys-motor", "modules", "Physical", "0.4.5", "Motors on Ports", "motor", "The motor blocks connected to electrical and mechanical networks."),
  f("phys-hyd", "modules", "Physical", "0.4.6", "Hydraulics", "block", "Pump, valve, cylinder and pipe."),

  // ---- Stage D: exchange (0.5.x)
  f("fmuexp", "analyze", "Exchange", "0.5.1", "FMU Export", "export", "Export the model as an FMU (co-simulation) to run inside other tools."),
  f("fmuimp", "model", "Exchange", "0.5.2", "FMU Import", "import", "Run another tool's FMU as a block."),
  f("hdf5", "analyze", "Results", "0.5.3", "MAT and HDF5", "export", "Save results as MAT or HDF5 files."),
  f("modelica", "model", "Exchange", "0.5.4", "Modelica", "import", "Modelica models through an external compiler."),

  // ---- Stage E: studies and logic (0.6.x)
  f("chart", "model", "Logic", "0.6.1", "State Machine", "sub", "States, transitions and actions drawn as a chart."),
  f("pyblock", "model", "Logic", "0.6.2", "Python Block", "py", "A block whose behaviour is a Python function."),
  f("mc", "simulate", "Studies", "0.6.3", "Monte Carlo", "sweep", "Run many times with parameters drawn from distributions; spread of the results."),
  f("sensi", "simulate", "Studies", "0.6.3", "Sensitivity", "specs", "Which parameters change a result most."),
  f("optim", "simulate", "Studies", "0.6.4", "Optimise and Fit", "plot", "Find parameters that meet goals, or that make the model match measured data."),
  f("report", "analyze", "Results", "0.6.5", "Report", "report", "The model, its runs and its plots in one printable report."),

  // ---- Stage F and modules later
  f("os", "view", "Reach", "0.7.1", "Linux and macOS", "info", "Installers for Linux and macOS."),
  f("fast", "simulate", "Solver", "0.7.2", "Fast Mode", "run", "A compiled run mode for big models."),
  f("variants", "model", "Signals", "0.7.3", "Variants", "sub", "Switchable subsystems, and a comparison of two model files."),
  f("flight", "modules", "Modules", "later", "Flight", "motor", "A module for aircraft and drones: rigid-body dynamics, aerodynamics, autopilot loops."),
  f("vehicle", "modules", "Modules", "later", "Vehicle", "motor", "A module for electric vehicles: drive cycle, battery, motor and driveline."),
  f("bench", "modules", "Hardware", "later", "Hardware Bench", "run", "Connect the model to real controllers and sensors, with the planned Datum bench app."),
];

/** The planned features for a ribbon tab. */
export const planned = (_a: unknown, q: { tab?: string }) => FEATS.filter(x => !q.tab || x.tab === q.tab);
