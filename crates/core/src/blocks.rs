//! The block library: one definition per kind (name, category, parameters with units and
//! defaults, size, help card). The single owner of what a block is; `rt.rs` owns how it runs.

use crate::model::{Block, Diagram};
use serde::Serialize;

#[derive(Clone, Copy, Debug, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Pk {
    /// An expression (number, vector or matrix) that may use model variables.
    Num,
    /// Plain text, used as written (signs, file names, expressions of signals).
    Text,
    /// One of a list of choices.
    Choice,
}

#[derive(Clone, Debug, Serialize)]
pub struct P {
    pub key: &'static str,
    pub label: &'static str,
    pub def: &'static str,
    pub unit: &'static str,
    pub hint: &'static str,
    pub kind: Pk,
    pub choices: &'static [&'static str],
}

#[derive(Clone, Debug, Serialize)]
pub struct Def {
    pub kind: &'static str,
    pub name: &'static str,
    pub cat: &'static str,
    pub what: &'static str,
    pub how: &'static str,
    pub params: Vec<P>,
    pub w: f64,
    pub h: f64,
    /// Shown dimmed in the library with the release it arrives in.
    pub planned: &'static str,
}

const fn n(key: &'static str, label: &'static str, def: &'static str, unit: &'static str, hint: &'static str) -> P {
    P { key, label, def, unit, hint, kind: Pk::Num, choices: &[] }
}
const fn t(key: &'static str, label: &'static str, def: &'static str, hint: &'static str) -> P {
    P { key, label, def, unit: "", hint, kind: Pk::Text, choices: &[] }
}
const fn c(key: &'static str, label: &'static str, choices: &'static [&'static str], hint: &'static str) -> P {
    P { key, label, def: choices[0], unit: "", hint, kind: Pk::Choice, choices }
}

pub const CATS: &[&str] = &[
    "Sources", "Math", "Continuous", "Discontinuities", "Discrete", "Routing", "Lookup tables", "Sinks", "Subsystems",
    "Motors", "Physical (0.4)",
];

fn d(kind: &'static str, name: &'static str, cat: &'static str, what: &'static str, how: &'static str, params: Vec<P>) -> Def {
    let (w, h) = size(kind);
    Def { kind, name, cat, what, how, params, w, h, planned: "" }
}

fn planned(kind: &'static str, name: &'static str, cat: &'static str, what: &'static str, when: &'static str) -> Def {
    Def { kind, name, cat, what, how: "", params: vec![], w: 60.0, h: 40.0, planned: when }
}

/// Default block size on the canvas.
pub fn size(kind: &str) -> (f64, f64) {
    match kind {
        "Sum" => (36.0, 36.0),
        "Product" | "Divide" => (40.0, 40.0),
        "Inport" | "Outport" => (36.0, 24.0),
        "Scope" | "XY" => (50.0, 50.0),
        "Display" => (80.0, 30.0),
        "Mux" | "Demux" => (12.0, 60.0),
        "Subsystem" => (100.0, 70.0),
        "DCMotor" | "PMSM" => (90.0, 70.0),
        "TransferFcn" | "ZPK" | "StateSpace" | "Fcn" | "PID" => (90.0, 44.0),
        "Lookup2D" | "Switch" => (60.0, 60.0),
        _ => (60.0, 40.0),
    }
}

pub fn library() -> Vec<Def> {
    let interp: &'static [&'static str] = &["linear", "hold"];
    vec![
        // Sources
        d("Constant", "Constant", "Sources", "A fixed value: a number, or a vector such as [1 2 3].", "Set Value. A vector gives a signal of that width.",
            vec![n("value", "Value", "1", "", "Number or vector")]),
        d("Step", "Step", "Sources", "Jumps from an initial to a final value at a set time.", "The jump is hit exactly by every solver.",
            vec![n("time", "Step time", "1", "s", ""), n("initial", "Initial value", "0", "", ""), n("final", "Final value", "1", "", "")]),
        d("Ramp", "Ramp", "Sources", "A straight line that starts rising at a set time.", "Output = initial + slope · (t − start) after the start time.",
            vec![n("slope", "Slope", "1", "1/s", ""), n("start", "Start time", "0", "s", ""), n("initial", "Initial output", "0", "", "")]),
        d("Sine", "Sine wave", "Sources", "A sine wave: amplitude · sin(2π · frequency · t + phase) + bias.", "Frequency is in hertz (cycles per second).",
            vec![n("amp", "Amplitude", "1", "", ""), n("freq", "Frequency", "1", "Hz", ""), n("phase", "Phase", "0", "rad", ""), n("bias", "Bias", "0", "", "")]),
        d("Pulse", "Pulse", "Sources", "A rectangular pulse train.", "High for width % of each period, starting after the delay. Edges are hit exactly.",
            vec![n("amp", "Amplitude", "1", "", ""), n("period", "Period", "1", "s", ""), n("width", "Pulse width", "50", "%", "Percent of the period"), n("delay", "Delay", "0", "s", "")]),
        d("Clock", "Clock", "Sources", "Outputs the simulation time t.", "Use it with a Function block to build signals of time.", vec![]),
        d("Table", "From table", "Sources", "A signal given as a table of times and values.", "Between points the value is interpolated (linear) or held (hold). Before the first time and after the last, the end values hold.",
            vec![n("times", "Times", "[0 1 2]", "s", "Increasing"), n("values", "Values", "[0 1 0]", "", "Same length as Times"), c("interp", "Between points", interp, "")]),
        d("FromFile", "From file (CSV)", "Sources", "Plays back measured data from a CSV file: the first column is time, the others are signals.", "The path may be relative to the model file. A header row is skipped. Values are interpolated linearly.",
            vec![t("file", "CSV file", "data.csv", "Path to the file")]),
        d("Noise", "Random noise", "Sources", "Normally distributed random numbers, held for each sample time.", "Same seed, same sequence: runs are repeatable.",
            vec![n("mean", "Mean", "0", "", ""), n("sd", "Standard deviation", "1", "", ""), n("ts", "Sample time", "0.1", "s", ""), n("seed", "Seed", "1", "", "")]),
        // Math
        d("Gain", "Gain", "Math", "Multiplies the input by a gain.", "A number scales every element; a vector multiplies element by element; a matrix K gives K·u.",
            vec![n("k", "Gain", "1", "", "Number, vector or matrix")]),
        d("Sum", "Sum", "Math", "Adds and subtracts its inputs.", "Signs lists one + or − per input, e.g. +- or ++-. Inputs of width 1 are added to every element.",
            vec![t("signs", "Signs", "+-", "One + or − per input")]),
        d("Product", "Product", "Math", "Multiplies and divides its inputs element by element.", "Operations lists one * or / per input, e.g. ** or */.",
            vec![t("ops", "Operations", "**", "One * or / per input")]),
        d("Divide", "Divide", "Math", "Divides the first input by the second.", "Division by zero gives infinity; guard it with Saturation if needed.", vec![]),
        d("Abs", "Abs", "Math", "The absolute value of the input.", "", vec![]),
        d("Sign", "Sign", "Math", "1 for positive input, −1 for negative, 0 at zero.", "", vec![]),
        d("MinMax", "Min / Max", "Math", "The smallest or largest of its inputs, element by element.", "",
            vec![c("fn", "Function", &["min", "max"], ""), n("n", "Inputs", "2", "", "")]),
        d("Fcn", "Function", "Math", "Any expression of the inputs u1, u2, … (or u) and time t.", "Example: u1*sin(t) + if(u2 > 0, 1, -1). Model variables can be used too. Functions: sin cos tan exp log sqrt abs min max sat if atan2 …",
            vec![t("expr", "Expression", "u^2", "Uses u (or u1, u2, …) and t"), n("n", "Inputs", "1", "", "")]),
        d("Saturation", "Saturation", "Math", "Limits the input between a lower and an upper bound.", "",
            vec![n("upper", "Upper limit", "1", "", ""), n("lower", "Lower limit", "-1", "", "")]),
        // Continuous
        d("Integrator", "Integrator", "Continuous", "Integrates its input over time: the output is the state x with dx/dt = u.", "Limits hold the state inside [lower, upper] (use inf for none). Integrators break algebraic loops.",
            vec![n("ic", "Initial condition", "0", "", ""), n("upper", "Upper limit", "inf", "", ""), n("lower", "Lower limit", "-inf", "", "")]),
        d("Derivative", "Derivative", "Continuous", "The rate of change of the input, du/dt, from the last two accepted steps.", "Noisy and step inputs give spikes; prefer a filtered derivative (PID with N, or s/(τs+1)).", vec![]),
        d("TransferFcn", "Transfer function", "Continuous", "A linear system given by numerator and denominator coefficients in s, highest power first.", "Example: [1] and [1 2 1] is 1/(s² + 2s + 1). The numerator degree must not exceed the denominator's.",
            vec![n("num", "Numerator", "[1]", "", "Coefficients in s"), n("den", "Denominator", "[1 1]", "", "Coefficients in s")]),
        d("ZPK", "Zero-pole-gain", "Continuous", "A linear system given by its zeros, poles and gain.", "List values separated by spaces or commas; complex ones as -1+2j and -1-2j (in conjugate pairs).",
            vec![t("zeros", "Zeros", "", "e.g. -1"), t("poles", "Poles", "-1 -2", "e.g. -1+2j -1-2j"), n("k", "Gain", "1", "", "")]),
        d("StateSpace", "State space", "Continuous", "dx/dt = A x + B u, y = C x + D u.", "Matrices as [a b; c d]. D may be 0.",
            vec![n("A", "A", "[-1]", "", ""), n("B", "B", "[1]", "", ""), n("C", "C", "[1]", "", ""), n("D", "D", "0", "", ""), n("x0", "Initial state", "0", "", "")]),
        d("PID", "PID controller", "Continuous", "Parallel PID: P·e + I·∫e + D·N·(e − filtered e), with output limits and anti-windup.", "N is the derivative filter (rad/s); larger N is closer to a pure derivative. When the output is limited, the integral stops winding up.",
            vec![n("P", "Proportional P", "1", "", ""), n("I", "Integral I", "0", "1/s", ""), n("D", "Derivative D", "0", "s", ""), n("N", "Filter N", "100", "rad/s", ""),
                 n("upper", "Output upper limit", "inf", "", ""), n("lower", "Output lower limit", "-inf", "", ""), n("ic", "Integral initial value", "0", "", "")]),
        d("Delay", "Transport delay", "Continuous", "Delays the input by a fixed time.", "Before the delay has passed the output is the initial value. The history is interpolated linearly.",
            vec![n("delay", "Delay", "1", "s", ""), n("initial", "Initial output", "0", "", "")]),
        // Discontinuities
        d("DeadZone", "Dead zone", "Discontinuities", "Zero output inside a band; outside it, the input minus the band edge.", "",
            vec![n("start", "Start of zone", "-0.5", "", ""), n("end", "End of zone", "0.5", "", "")]),
        d("RateLimiter", "Rate limiter", "Discontinuities", "Limits how fast the output can rise and fall.", "Falling rate is negative, e.g. −1.",
            vec![n("rising", "Rising rate", "1", "1/s", ""), n("falling", "Falling rate", "-1", "1/s", "")]),
        d("Relay", "Relay", "Discontinuities", "Switches on when the input reaches the on point and off when it falls to the off point (hysteresis).", "Switching times are located exactly by the variable-step solvers.",
            vec![n("on", "Switch on point", "0.5", "", ""), n("off", "Switch off point", "-0.5", "", ""), n("onv", "Output when on", "1", "", ""), n("offv", "Output when off", "0", "", "")]),
        d("Backlash", "Backlash", "Discontinuities", "Play between input and output, as in gears: the output follows only after the input crosses the dead band.", "",
            vec![n("width", "Dead band width", "1", "", ""), n("ic", "Initial output", "0", "", "")]),
        d("Quantizer", "Quantiser", "Discontinuities", "Rounds the input to multiples of an interval.", "",
            vec![n("q", "Interval", "0.5", "", "")]),
        // Discrete
        d("ZOH", "Zero-order hold", "Discrete", "Samples the input every sample time and holds it.", "", vec![n("ts", "Sample time", "0.1", "s", "")]),
        d("UnitDelay", "Unit delay", "Discrete", "Outputs the input from the previous sample.", "", vec![n("ts", "Sample time", "0.1", "s", ""), n("ic", "Initial output", "0", "", "")]),
        planned("DiscreteTF", "Discrete transfer function", "Discrete", "A linear system in z.", "0.2"),
        planned("DiscretePID", "Discrete PID", "Discrete", "A PID controller that runs at a sample time.", "0.2"),
        // Routing
        d("Mux", "Mux", "Routing", "Combines its inputs into one vector signal.", "", vec![n("n", "Inputs", "2", "", "")]),
        d("Demux", "Demux", "Routing", "Splits a vector signal into its parts.", "", vec![n("n", "Outputs", "2", "", "")]),
        d("Switch", "Switch", "Routing", "Passes input 1 when the middle input meets the criterion, otherwise input 3.", "",
            vec![c("crit", "Pass input 1 when", &["u2 >= threshold", "u2 > threshold", "u2 != 0"], ""), n("th", "Threshold", "0", "", "")]),
        d("Selector", "Selector", "Routing", "Picks elements of a vector signal.", "Indices start at 1, e.g. [1 3].", vec![n("idx", "Indices", "[1]", "", "")]),
        planned("Goto", "Goto / From", "Routing", "Connect signals by name without wires.", "0.2"),
        planned("Bus", "Bus", "Routing", "Group named signals into one wire.", "0.2"),
        // Lookup
        d("Lookup1D", "1-D lookup table", "Lookup tables", "Interpolates y at the input from a table of x and y.", "x must increase. Outside the table: hold the end value (clip) or extend the end line (linear).",
            vec![n("x", "Breakpoints x", "[0 1 2]", "", ""), n("y", "Values y", "[0 1 4]", "", ""), c("extrap", "Outside the table", &["clip", "linear"], "")]),
        d("Lookup2D", "2-D lookup table", "Lookup tables", "Bilinear interpolation in a table: input 1 picks the row, input 2 the column.", "Table has one row per row breakpoint and one column per column breakpoint. Inputs are clipped to the table.",
            vec![n("x", "Row breakpoints", "[0 1]", "", ""), n("y", "Column breakpoints", "[0 1]", "", ""), n("table", "Table", "[0 1; 1 2]", "", "")]),
        // Sinks
        d("Scope", "Scope", "Sinks", "Plots its inputs against time while the model runs.", "Double-click to open it. Each input can be a vector; every element is a line.", vec![n("n", "Inputs", "1", "", "")]),
        d("XY", "XY graph", "Sinks", "Plots input 2 against input 1 (a phase plane or a trajectory).", "", vec![]),
        d("Display", "Display", "Sinks", "Shows the latest value of its input on the block.", "", vec![]),
        d("ToFile", "To file (CSV)", "Sinks", "Writes its inputs and time to a CSV file at the end of the run.", "The path may be relative to the model file.",
            vec![t("file", "CSV file", "results.csv", ""), n("n", "Inputs", "1", "", "")]),
        d("ToPlot", "To Datum Plot", "Sinks", "Sends its inputs to Datum Plot for analysis and publication graphs.", "After a run, use Analyze › Send to Datum Plot.", vec![n("n", "Inputs", "1", "", "")]),
        // Subsystems
        d("Subsystem", "Subsystem", "Subsystems", "A group of blocks shown as one block. Double-click to open it.", "Inside, In and Out blocks become the subsystem's ports, ordered by their number. Select blocks and use Model › Create subsystem to group them.", vec![]),
        d("Inport", "In", "Subsystems", "An input port of the subsystem it sits in.", "The number sets the port order.", vec![n("port", "Port number", "1", "", "")]),
        d("Outport", "Out", "Subsystems", "An output port of the subsystem it sits in.", "The number sets the port order.", vec![n("port", "Port number", "1", "", "")]),
        // Motors
        d("DCMotor", "DC motor", "Motors", "A permanent-magnet DC motor. Inputs: armature voltage V and load torque T_L. Outputs: speed ω, current i, angle θ.",
            "L·di/dt = V − R·i − Ke·ω;  J·dω/dt = Kt·i − b·ω − T_L;  dθ/dt = ω. In SI units Ke = Kt.",
            vec![n("R", "Armature resistance R", "1", "Ω", ""), n("L", "Armature inductance L", "0.5", "H", ""), n("Ke", "Back-EMF constant Ke", "0.01", "V·s/rad", ""),
                 n("Kt", "Torque constant Kt", "0.01", "N·m/A", ""), n("J", "Rotor inertia J", "0.01", "kg·m²", ""), n("b", "Viscous friction b", "0.1", "N·m·s/rad", ""),
                 n("w0", "Initial speed", "0", "rad/s", "")]),
        d("PMSM", "PMSM (d–q)", "Motors", "A permanent-magnet synchronous motor in the rotor (d–q) frame. Inputs: v_d, v_q, load torque T_L. Outputs: speed ω (mechanical), i_d, i_q, torque T_e, angle θ.",
            "Ld·di_d/dt = v_d − Rs·i_d + p·ω·Lq·i_q;  Lq·di_q/dt = v_q − Rs·i_q − p·ω·(Ld·i_d + ψ);  T_e = 1.5·p·(ψ·i_q + (Ld − Lq)·i_d·i_q);  J·dω/dt = T_e − b·ω − T_L.",
            vec![n("Rs", "Stator resistance Rs", "0.5", "Ω", ""), n("Ld", "d-axis inductance Ld", "0.001", "H", ""), n("Lq", "q-axis inductance Lq", "0.001", "H", ""),
                 n("psi", "Magnet flux ψ", "0.05", "Wb", ""), n("p", "Pole pairs p", "4", "", ""), n("J", "Inertia J", "0.0005", "kg·m²", ""), n("b", "Viscous friction b", "0.0001", "N·m·s/rad", "")]),
        planned("Induction", "Induction motor", "Motors", "A squirrel-cage induction motor in the d–q frame.", "0.2"),
        // Physical (planned)
        planned("Mass", "Mass, spring, damper", "Physical (0.4)", "Mechanical components you connect by ports.", "0.4"),
        planned("Resistor", "R, L, C and sources", "Physical (0.4)", "Electrical components you connect by ports.", "0.4"),
        planned("Pipe", "Pipe, pump, valve, tank", "Physical (0.4)", "Hydraulic components you connect by ports.", "0.4"),
        planned("HeatCap", "Heat capacity, conduction", "Physical (0.4)", "Thermal components you connect by ports.", "0.4"),
        planned("FMU", "FMU", "Subsystems", "Run a model exported by another tool (FMI standard).", "0.5"),
        planned("PyBlock", "Python block", "Math", "A block written in Python.", "0.6"),
    ]
}

pub fn def(kind: &str) -> Option<Def> {
    library().into_iter().find(|d| d.kind == kind && d.planned.is_empty())
}

/// The text of a parameter: the block's own, else the default.
pub fn param(b: &Block, key: &str) -> String {
    if let Some(v) = b.params.get(key) {
        return v.clone();
    }
    def(&b.kind).and_then(|d| d.params.into_iter().find(|p| p.key == key)).map(|p| p.def.to_string()).unwrap_or_default()
}

fn count(b: &Block, key: &str) -> usize {
    param(b, key).trim().parse::<f64>().ok().filter(|v| v.is_finite()).map_or(1, |v| (v.round() as i64).clamp(1, 32) as usize)
}

/// The subsystem's ports: (In blocks, Out blocks) ordered by their port number, then position.
pub fn sub_ports(d: &Diagram) -> (Vec<u32>, Vec<u32>) {
    let order = |kind: &str| {
        let mut v: Vec<(f64, f64, u32)> = d.blocks.iter().filter(|b| b.kind == kind)
            .map(|b| (param(b, "port").trim().parse::<f64>().unwrap_or(1e9), b.y, b.id)).collect();
        v.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
        v.into_iter().map(|x| x.2).collect::<Vec<_>>()
    };
    (order("Inport"), order("Outport"))
}

/// Number of (input, output) ports.
pub fn ports(b: &Block) -> (usize, usize) {
    match b.kind.as_str() {
        "Constant" | "Step" | "Ramp" | "Sine" | "Pulse" | "Clock" | "Table" | "FromFile" | "Noise" | "Inport" => (0, 1),
        "Sum" => (param(b, "signs").chars().filter(|c| *c == '+' || *c == '-').count().clamp(1, 32), 1),
        "Product" => (param(b, "ops").chars().filter(|c| *c == '*' || *c == '/').count().clamp(1, 32), 1),
        "Divide" | "Lookup2D" => (2, 1),
        "MinMax" | "Fcn" | "Mux" => (count(b, "n"), 1),
        "Demux" => (1, count(b, "n")),
        "Switch" => (3, 1),
        "Scope" | "ToFile" | "ToPlot" => (count(b, "n"), 0),
        "XY" => (2, 0),
        "Display" | "Outport" => (1, 0),
        "Subsystem" => b.sub.as_ref().map_or((0, 0), |s| {
            let (i, o) = sub_ports(s);
            (i.len(), o.len())
        }),
        "DCMotor" => (2, 3),
        "PMSM" => (3, 5),
        _ => (1, 1),
    }
}

/// Port labels, shown beside the ports when a block has named ones.
pub fn port_names(b: &Block) -> (Vec<String>, Vec<String>) {
    let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
    match b.kind.as_str() {
        "DCMotor" => (s(&["V", "T_L"]), s(&["ω", "i", "θ"])),
        "PMSM" => (s(&["v_d", "v_q", "T_L"]), s(&["ω", "i_d", "i_q", "T_e", "θ"])),
        "Subsystem" => {
            let Some(sd) = &b.sub else { return (vec![], vec![]) };
            let (i, o) = sub_ports(sd);
            let nm = |ids: Vec<u32>| ids.iter().map(|id| sd.block(*id).map(|b| b.name.clone()).unwrap_or_default()).collect();
            (nm(i), nm(o))
        }
        "Switch" => (s(&["", "u2", ""]), vec![]),
        _ => (vec![], vec![]),
    }
}

/// Where the ports sit on the canvas: inputs on the left (right when flipped), outputs opposite.
pub fn port_xy(b: &Block) -> (Vec<(f64, f64)>, Vec<(f64, f64)>) {
    let (ni, no) = ports(b);
    let col = |k: usize, x: f64| -> Vec<(f64, f64)> {
        (0..k).map(|i| (x, (b.y + b.h * (i as f64 + 0.5) / k as f64).round())).collect()
    };
    let (l, r) = (b.x, b.x + b.w);
    if b.flip {
        (col(ni, r), col(no, l))
    } else {
        (col(ni, l), col(no, r))
    }
}

/// A short text shown on the block (its key parameter).
pub fn label(b: &Block) -> String {
    let p = |k: &str| param(b, k);
    match b.kind.as_str() {
        "Constant" => p("value"),
        "Gain" => p("k"),
        "Sum" => p("signs"),
        "Product" => p("ops"),
        "Fcn" => p("expr"),
        "TransferFcn" => format!("{}\n{}", p("num"), p("den")),
        "ZPK" => format!("{}·({})\n({})", p("k"), p("zeros"), p("poles")),
        "Saturation" => format!("[{}, {}]", p("lower"), p("upper")),
        "Delay" => format!("e^(−{}s)", p("delay")),
        "Inport" | "Outport" => p("port"),
        "MinMax" => p("fn"),
        "PID" => format!("P {} · I {} · D {}", p("P"), p("I"), p("D")),
        "Integrator" => "1/s".into(),
        "Derivative" => "du/dt".into(),
        "ZOH" => format!("ZOH {}", p("ts")),
        "UnitDelay" => "1/z".into(),
        "Step" => format!("{} → {}", p("initial"), p("final")),
        "Table" | "Lookup1D" => "f(x)".into(),
        "FromFile" | "ToFile" => p("file"),
        _ => String::new(),
    }
}
