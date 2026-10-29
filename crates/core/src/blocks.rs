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
    /// Typical parameter values for common ratings (motors).
    pub presets: Vec<crate::motors::Preset>,
}

const fn n(key: &'static str, label: &'static str, def: &'static str, unit: &'static str, hint: &'static str) -> P {
    P { key, label, def, unit, hint, kind: Pk::Num, choices: &[] }
}
const fn t(key: &'static str, label: &'static str, def: &'static str, hint: &'static str) -> P {
    P { key, label, def, unit: "", hint, kind: Pk::Text, choices: &[] }
}
const RESET: &[&str] = &["none", "rising", "falling", "either"];

const fn c(key: &'static str, label: &'static str, choices: &'static [&'static str], hint: &'static str) -> P {
    P { key, label, def: choices[0], unit: "", hint, kind: Pk::Choice, choices }
}

pub const CATS: &[&str] = &[
    "Sources", "Math", "Continuous", "Discontinuities", "Discrete", "Routing", "Lookup tables", "Sinks", "Subsystems",
    "Motors", "Drives", "Physical (0.4)",
];

fn d(kind: &'static str, name: &'static str, cat: &'static str, what: &'static str, how: &'static str, params: Vec<P>) -> Def {
    let (w, h) = size(kind);
    Def { kind, name, cat, what, how, params, w, h, planned: "", presets: crate::motors::presets(kind) }
}

fn planned(kind: &'static str, name: &'static str, cat: &'static str, what: &'static str, when: &'static str) -> Def {
    Def { kind, name, cat, what, how: "", params: vec![], w: 60.0, h: 40.0, planned: when, presets: vec![] }
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
        "DCMotor" | "PMSM" | "DCSepEx" | "DCShunt" | "DCSeries" | "DCCompound" | "InductionSC" | "InductionDC" | "PMSM3" | "SynRM" | "BLDC" | "Stepper" | "SRM" => (90.0, 80.0),
        "DCLink" | "InverterDC" => (80.0, 50.0),
        "InductionWR" | "SyncField" => (100.0, 100.0),
        "Clarke" | "InvClarke" | "Park" | "InvPark" | "Encoder" => (60.0, 44.0),
        "ThreePhase" | "Inverter" | "PWM" => (70.0, 50.0),
        "TransferFcn" | "ZPK" | "StateSpace" | "Fcn" | "PID" | "DiscreteTF" | "DiscretePID" | "DiscreteIntegrator" => (90.0, 44.0),
        "Goto" | "From" => (60.0, 30.0),
        "Terminator" => (24.0, 24.0),
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
        d("Square", "Square wave", "Sources", "A square wave between bias − amplitude and bias + amplitude.", "High for duty % of each period. The edges are hit exactly by every solver.",
            vec![n("amp", "Amplitude", "1", "", ""), n("freq", "Frequency", "1", "Hz", ""), n("duty", "Duty cycle", "50", "%", "Percent of the period"), n("bias", "Bias", "0", "", "")]),
        d("Triangle", "Triangle wave", "Sources", "A symmetrical triangle wave from bias − amplitude to bias + amplitude, starting at the bias and rising.", "The corners are hit exactly.",
            vec![n("amp", "Amplitude", "1", "", ""), n("freq", "Frequency", "1", "Hz", ""), n("bias", "Bias", "0", "", "")]),
        d("Chirp", "Chirp (frequency sweep)", "Sources", "A sine wave whose frequency rises linearly from f0 to f1 over a sweep time, then stays at f1.", "Use it to find resonances and the frequency response of a model from a single run.",
            vec![n("amp", "Amplitude", "1", "", ""), n("f0", "Start frequency f0", "0.1", "Hz", ""), n("f1", "End frequency f1", "10", "Hz", ""), n("T", "Sweep time", "10", "s", "")]),
        d("RepSeq", "Repeating sequence", "Sources", "A pattern of times and values repeated for ever: a sawtooth, a drive cycle, a test profile.", "Times start at 0 and increase; the last time is the period. Values are interpolated linearly and every point is hit exactly.",
            vec![n("times", "Times", "[0 1 2]", "s", "From 0; the last is the period"), n("values", "Values", "[0 1 0]", "", "One per time")]),
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
            vec![n("ic", "Initial condition", "0", "", ""), n("upper", "Upper limit", "inf", "", ""), n("lower", "Lower limit", "-inf", "", ""),
                 c("reset", "External reset", RESET, "Adds a reset input; on that edge the state goes back to the initial condition")]),
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
        d("DiscreteTF", "Discrete transfer function", "Discrete", "A linear system in z, updated every sample time: numerator and denominator coefficients in z, highest power first.", "Example: [0.5] and [1 -0.5] is 0.5/(z − 0.5). The numerator degree must not exceed the denominator's. The output holds between samples.",
            vec![n("num", "Numerator", "[1]", "", "Coefficients in z"), n("den", "Denominator", "[1 -0.5]", "", "Coefficients in z"), n("ts", "Sample time", "0.1", "s", "")]),
        d("DiscreteSS", "Discrete state space", "Discrete", "x[k+1] = A x[k] + B u[k], y[k] = C x[k] + D u[k], every sample time.", "Matrices as [a b; c d]. D may be 0.",
            vec![n("A", "A", "[0.5]", "", ""), n("B", "B", "[1]", "", ""), n("C", "C", "[1]", "", ""), n("D", "D", "0", "", ""), n("x0", "Initial state", "0", "", ""), n("ts", "Sample time", "0.1", "s", "")]),
        d("DiscreteIntegrator", "Discrete integrator", "Discrete", "Accumulates K·Ts·u each sample: the discrete form of an integrator, with limits and an optional reset input.", "Forward Euler: y[k] = x[k] (no direct feedthrough, so it breaks loops). Backward Euler: y[k] = x[k] + K·Ts·u[k]. Trapezoidal: the average of the two.",
            vec![n("k", "Gain K", "1", "", ""), n("ts", "Sample time", "0.1", "s", ""), c("method", "Method", &["forward Euler", "backward Euler", "trapezoidal"], ""), n("ic", "Initial condition", "0", "", ""),
                 n("upper", "Upper limit", "inf", "", ""), n("lower", "Lower limit", "-inf", "", ""), c("reset", "External reset", RESET, "Adds a reset input, sampled each sample time")]),
        d("DiscretePID", "Discrete PID", "Discrete", "A parallel PID that runs at a sample time, as in a microcontroller: P·e + I·Ts·Σe + a filtered derivative, with output limits and anti-windup.", "The integral is forward Euler and the derivative filter backward Euler: d[k] = (d[k−1] + D·N·(e[k] − e[k−1]))/(1 + N·Ts). Clamping stops the integral while the output is limited; back-calculation bleeds it back with gain Kb.",
            vec![n("P", "Proportional P", "1", "", ""), n("I", "Integral I", "0", "1/s", ""), n("D", "Derivative D", "0", "s", ""), n("N", "Filter N", "100", "rad/s", ""), n("ts", "Sample time", "0.001", "s", ""),
                 n("upper", "Output upper limit", "inf", "", ""), n("lower", "Output lower limit", "-inf", "", ""), c("aw", "Anti-windup", &["clamping", "back-calculation", "none"], ""),
                 n("kb", "Back-calculation gain Kb", "1", "1/s", "Back-calculation only"), n("ic", "Integral initial value", "0", "", "")]),
        d("RateTransition", "Rate transition", "Discrete", "Passes a signal from one sample time to another.", "Hold samples the input at the new sample time (fast to slow, or continuous to discrete). Delay one sample gives a deterministic hand-over from slow to fast, as a real controller would.",
            vec![n("ts", "Output sample time", "0.1", "s", ""), c("mode", "Mode", &["hold", "delay one sample"], ""), n("ic", "Initial output", "0", "", "Delay mode only")]),
        // Routing
        d("Mux", "Mux", "Routing", "Combines its inputs into one vector signal.", "", vec![n("n", "Inputs", "2", "", "")]),
        d("Demux", "Demux", "Routing", "Splits a vector signal into its parts.", "", vec![n("n", "Outputs", "2", "", "")]),
        d("Switch", "Switch", "Routing", "Passes input 1 when the middle input meets the criterion, otherwise input 3.", "",
            vec![c("crit", "Pass input 1 when", &["u2 >= threshold", "u2 > threshold", "u2 != 0"], ""), n("th", "Threshold", "0", "", "")]),
        d("Selector", "Selector", "Routing", "Picks elements of a vector signal.", "Indices start at 1, e.g. [1 3].", vec![n("idx", "Indices", "[1]", "", "")]),
        d("Goto", "Goto", "Routing", "Sends its input to every From block with the same tag, without a wire.", "Local: only Froms in the same diagram. Global: Froms anywhere in the model, inside subsystems too.",
            vec![t("tag", "Tag", "A", "A short name"), c("scope", "Visible to", &["local", "global"], "")]),
        d("From", "From", "Routing", "Outputs the signal sent by the Goto with the same tag.", "", vec![t("tag", "Tag", "A", "The Goto's tag")]),
        d("Terminator", "Terminator", "Sinks", "Ends an output you do not need, so it is not reported as unconnected.", "", vec![]),
        planned("Bus", "Bus", "Routing", "Group named signals into one wire.", "0.2"),
        // Lookup
        d("Lookup1D", "1-D lookup table", "Lookup tables", "Interpolates y at the input from a table of x and y.", "x must increase. Outside the table: hold the end value (clip) or extend the end line (linear).",
            vec![n("x", "Breakpoints x", "[0 1 2]", "", ""), n("y", "Values y", "[0 1 4]", "", ""), c("extrap", "Outside the table", &["clip", "linear"], "")]),
        d("Lookup2D", "2-D lookup table", "Lookup tables", "Bilinear interpolation in a table: input 1 picks the row, input 2 the column.", "Table has one row per row breakpoint and one column per column breakpoint. Inputs are clipped to the table.",
            vec![n("x", "Row breakpoints", "[0 1]", "", ""), n("y", "Column breakpoints", "[0 1]", "", ""), n("table", "Table", "[0 1; 1 2]", "", "")]),
        // Sinks
        d("Scope", "Scope", "Sinks", "Plots its inputs against time while the model runs.", "Double-click to open it. Each input can be a vector; every element is a line. On the scope: two measuring cursors, step-response marks, earlier runs to compare, and pictures as PNG or SVG. Drag across the plot to zoom (sideways for time, up or down for y); double-click to see the whole run.",
            vec![n("n", "Inputs", "1", "", ""), c("layout", "Layout", &["overlaid", "stacked"], "Stacked: one plot per input"), t("ymin", "Y lower limit", "auto", "A number, or auto"), t("ymax", "Y upper limit", "auto", "A number, or auto")]),
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
        d("DCSepEx", "DC motor, separately excited", "Motors", "A wound-field DC machine with its own field supply. Inputs: armature voltage V_a, field voltage V_f, load torque T_L. Outputs: speed ω, armature current i_a, field current i_f, torque T_e, angle θ.",
            "La·di_a/dt = V_a − Ra·i_a − Laf·i_f·ω;  Lf·di_f/dt = V_f − Rf·i_f;  T_e = Laf·i_f·i_a;  J·dω/dt = T_e − b·ω − T_L. Typical ratings set every value at once; replace them with nameplate data.",
            dc_params(false)),
        d("DCShunt", "DC motor, shunt", "Motors", "A DC machine whose field is fed from the same supply as the armature. Inputs: supply voltage V, load torque T_L. Outputs: speed ω, armature current i_a, field current i_f, torque T_e, angle θ.",
            "As the separately excited motor with V_f = V. The supply current is i_a + i_f.",
            dc_params(false)),
        d("DCSeries", "DC motor, series", "Motors", "A DC machine with its field in series with the armature: high starting torque, speed falls with load (traction, hoists). Inputs: V, T_L. Outputs: ω, current i, T_e, θ.",
            "(La + Lf)·di/dt = V − (Ra + Rf)·i − Laf·i·ω;  T_e = Laf·i². Never run it without load: the no-load speed is very high.",
            vec![n("Ra", "Armature resistance Ra", "0.4", "Ω", ""), n("La", "Armature inductance La", "0.01", "H", ""), n("Rf", "Series field resistance Rf", "0.2", "Ω", ""),
                 n("Lf", "Series field inductance Lf", "0.01", "H", ""), n("Laf", "Mutual inductance Laf", "0.052", "H", "Torque = Laf·i²"), n("J", "Inertia J", "0.15", "kg·m²", ""),
                 n("b", "Viscous friction b", "0.01", "N·m·s/rad", ""), n("w0", "Initial speed", "0", "rad/s", "")]),
        d("DCCompound", "DC motor, compound", "Motors", "A DC machine with a shunt field and a series field (cumulative when Lafse > 0, differential when < 0). Inputs: V, T_L. Outputs: ω, i_a, shunt field current i_f, T_e, θ.",
            "(La + Lse)·di_a/dt = V − (Ra + Rse)·i_a − (Laf·i_f + Lafse·i_a)·ω;  Lf·di_f/dt = V − Rf·i_f;  T_e = (Laf·i_f + Lafse·i_a)·i_a.",
            vec![n("Ra", "Armature resistance Ra", "0.5", "Ω", ""), n("La", "Armature inductance La", "0.01", "H", ""), n("Rse", "Series field resistance Rse", "0.1", "Ω", ""),
                 n("Lse", "Series field inductance Lse", "0.002", "H", ""), n("Lafse", "Series mutual inductance Lafse", "0.005", "H", ""), n("Rf", "Shunt field resistance Rf", "220", "Ω", ""),
                 n("Lf", "Shunt field inductance Lf", "50", "H", ""), n("Laf", "Shunt mutual inductance Laf", "1.25", "H", ""), n("J", "Inertia J", "0.15", "kg·m²", ""),
                 n("b", "Viscous friction b", "0.01", "N·m·s/rad", ""), n("w0", "Initial speed", "0", "rad/s", "")]),
        d("InductionSC", "Induction motor, squirrel-cage", "Motors", "A three-phase asynchronous motor. Inputs: stator phase voltages v_abc (width 3) and load torque T_L. Outputs: speed ω, torque T_e, stator currents i_abc (width 3), angle θ.",
            "Fifth-order model in the stationary frame, the equivalent circuit Rs, Lls, Lm, Llr, Rr (rotor values referred to the stator). Feed it from a Three-phase source or an Inverter. Typical ratings cover low, medium and high voltage machines.",
            im_params(false)),
        d("InductionDC", "Induction motor, double-cage", "Motors", "A squirrel-cage motor with two rotor cages: a high-resistance outer cage for starting torque and a low-resistance inner cage for running. Inputs: v_abc, T_L. Outputs: ω, T_e, i_abc, θ.",
            "Cage 1 (Rr, Llr) is the outer, starting cage; cage 2 (Rr2, Llr2) the inner, running cage. Both share the magnetising inductance Lm.",
            im_params(true)),
        d("InductionWR", "Induction motor, wound-rotor", "Motors", "An induction motor with its rotor windings brought out (slip rings), for rotor resistance starting or a rotor-side drive. Inputs: v_abc, rotor voltages vr_abc (in the rotor frame, referred to the stator), T_L. Outputs: ω, T_e, i_abc, θ, rotor currents ir_abc.",
            "With the rotor short-circuited (vr_abc = 0) it behaves as the squirrel-cage motor. For a starting resistor, feed vr_abc = −R_ext·ir_abc through a Gain.",
            im_params(false)),
        d("PMSM3", "PMSM, three-phase", "Motors", "A permanent-magnet synchronous motor fed with three-phase voltages (surface or interior magnets, Ld ≠ Lq). Inputs: v_abc, T_L. Outputs: speed ω, torque T_e, phase currents i_abc, angle θ.",
            "The same d–q model as the PMSM (d–q) block, with the Park transform inside at the electrical angle p·θ. θ = 0 puts the magnet on phase a.",
            vec![n("Rs", "Stator resistance Rs", "0.15", "Ω", ""), n("Ld", "d-axis inductance Ld", "0.0025", "H", ""), n("Lq", "q-axis inductance Lq", "0.004", "H", ""),
                 n("psi", "Magnet flux ψ", "0.4", "Wb", "Peak flux linkage per phase"), n("p", "Pole pairs p", "4", "", ""), n("J", "Inertia J", "0.03", "kg·m²", ""),
                 n("b", "Viscous friction b", "0.001", "N·m·s/rad", ""), n("w0", "Initial speed", "0", "rad/s", "")]),
        d("SynRM", "Synchronous reluctance motor", "Motors", "A synchronous motor with no magnets or field: torque comes from the difference of Ld and Lq. Inputs: v_abc, T_L. Outputs: ω, T_e, i_abc, θ.",
            "T_e = 1.5·p·(Ld − Lq)·i_d·i_q. It needs a drive that knows the rotor angle; it does not start from the mains.",
            vec![n("Rs", "Stator resistance Rs", "0.8", "Ω", ""), n("Ld", "d-axis inductance Ld", "0.12", "H", ""), n("Lq", "q-axis inductance Lq", "0.02", "H", ""),
                 n("p", "Pole pairs p", "2", "", ""), n("J", "Inertia J", "0.02", "kg·m²", ""), n("b", "Viscous friction b", "0.001", "N·m·s/rad", ""), n("w0", "Initial speed", "0", "rad/s", "")]),
        d("SyncField", "Synchronous motor, wound field", "Motors", "A salient-pole synchronous machine with a DC field winding and damper windings, as used for large medium and high voltage drives. Inputs: v_abc, field voltage v_f (referred to the stator), T_L. Outputs: ω, T_e, i_abc, field current i_f, θ.",
            "Seventh-order model in the rotor d–q frame. The dampers let it start as an induction motor and damp hunting. Start it at synchronous speed (w0 = 2πf/p) with if0 and v_f = Rf·if0 to run on the grid; θ0 sets the rotor's starting angle.",
            vec![n("Rs", "Stator resistance Rs", "0.1", "Ω", ""), n("Ll", "Stator leakage Ll", "0.004", "H", ""), n("Lmd", "d-axis magnetising Lmd", "0.035", "H", ""),
                 n("Lmq", "q-axis magnetising Lmq", "0.02", "H", ""), n("Rf", "Field resistance Rf", "0.03", "Ω", "Referred to the stator"), n("Llf", "Field leakage Llf", "0.006", "H", ""),
                 n("Rkd", "d damper resistance Rkd", "0.5", "Ω", ""), n("Llkd", "d damper leakage Llkd", "0.004", "H", ""), n("Rkq", "q damper resistance Rkq", "0.7", "Ω", ""),
                 n("Llkq", "q damper leakage Llkq", "0.007", "H", ""), n("p", "Pole pairs p", "2", "", ""), n("J", "Inertia J", "20", "kg·m²", ""),
                 n("b", "Viscous friction b", "0", "N·m·s/rad", ""), n("if0", "Initial field current", "0", "A", "Referred to the stator"), n("w0", "Initial speed", "0", "rad/s", ""),
                 n("th0", "Initial rotor angle", "0", "rad", "Mechanical")]),
        d("BLDC", "BLDC motor", "Motors", "A brushless DC motor with ideal six-step commutation. Inputs: DC voltage V, load torque T_L. Outputs: speed ω, current i, torque T_e, angle θ, Hall signals (width 3).",
            "Two phases conduct at a time with flat back-EMF:  2L·di/dt = V − 2R·i − 2Ke·ω;  T_e = 2Ke·i. R, L and Ke are per phase. Commutation ripple is not modelled.",
            vec![n("R", "Phase resistance R", "0.05", "Ω", ""), n("L", "Phase inductance L", "0.0001", "H", ""), n("Ke", "Phase back-EMF constant Ke", "0.045", "V·s/rad", ""),
                 n("p", "Pole pairs p", "4", "", "For the Hall signals"), n("J", "Inertia J", "0.0003", "kg·m²", ""), n("b", "Viscous friction b", "0.00001", "N·m·s/rad", "")]),
        d("Stepper", "Stepper motor, hybrid", "Motors", "A two-phase hybrid stepper motor. Inputs: phase voltages v_A, v_B and load torque T_L. Outputs: ω, T_e, θ, phase currents i_A, i_B.",
            "T_e = −Km·i_A·sin(Nr·θ) + Km·i_B·cos(Nr·θ) − Td·sin(4·Nr·θ). One full step turns the rotor π/(2·Nr): 1.8° with Nr = 50 teeth. Drive it with Pulse or Table sources.",
            vec![n("R", "Phase resistance R", "1.65", "Ω", ""), n("L", "Phase inductance L", "0.0028", "H", ""), n("Km", "Torque constant Km", "0.25", "N·m/A", ""),
                 n("Nr", "Rotor teeth Nr", "50", "", ""), n("Td", "Detent torque Td", "0.01", "N·m", ""), n("J", "Inertia J", "0.0000054", "kg·m²", ""),
                 n("b", "Viscous friction b", "0.00001", "N·m·s/rad", "")]),
        d("SRM", "Switched reluctance motor", "Motors", "A three-phase switched reluctance motor (6/4, 12/8 …): no magnets or windings on the rotor; torque from the change of phase inductance with angle. Inputs: phase voltages v_abc, T_L. Outputs: ω, T_e, i_abc, θ.",
            "L_k(θ) = L0 − L1·cos(Nr·θ − k·120°) between the unaligned Lmin and aligned Lmax; T_e = Σ ½·i_k²·dL_k/dθ. Phase a is unaligned at θ = 0. With an asymmetric bridge (the default) phase currents cannot reverse. No saturation.",
            vec![n("R", "Phase resistance R", "1.3", "Ω", ""), n("Lmax", "Aligned inductance Lmax", "0.06", "H", ""), n("Lmin", "Unaligned inductance Lmin", "0.008", "H", ""),
                 n("Nr", "Rotor poles Nr", "4", "", "4 for a 6/4 motor, 8 for 12/8"), n("J", "Inertia J", "0.008", "kg·m²", ""), n("b", "Viscous friction b", "0.001", "N·m·s/rad", ""),
                 n("w0", "Initial speed", "0", "rad/s", ""), n("th0", "Initial angle", "0", "rad", ""), c("currents", "Phase currents", &["one way (asymmetric bridge)", "both ways"], "")]),
        // Drives
        d("ThreePhase", "Three-phase source", "Drives", "A balanced three-phase voltage: v_abc (width 3).", "Line-to-line RMS voltage V_LL; each phase peak is V_LL·√2/√3. Phase a is a cosine; b and c lag by 120° and 240°.",
            vec![n("Vll", "Line voltage (RMS)", "400", "V", ""), n("f", "Frequency", "50", "Hz", ""), n("phase", "Phase of a", "0", "rad", "")]),
        d("Inverter", "Inverter, three-phase", "Drives", "A two-level voltage-source inverter. Inputs: modulation m_abc (−1…1, width 3), DC link voltage V_dc. Output: phase voltages v_abc.",
            "Average model: v_k = V_dc/2·(m_k − mean of m). Switched: each leg compares m_k with a triangle carrier at the switching frequency; the switching instants are located exactly. The average model runs much faster.",
            vec![c("model", "Model", &["average", "switched (PWM)"], ""), n("fc", "Switching frequency", "5000", "Hz", "Switched model only")]),
        d("PWM", "PWM", "Drives", "Sine-triangle pulse width modulation: compares each reference (−1…1) with a triangle carrier.", "Output +1 / −1 (or 1 / 0) per element. Switching instants are located exactly by the variable-step solvers.",
            vec![n("fc", "Carrier frequency", "1000", "Hz", ""), c("levels", "Output levels", &["−1 / +1", "0 / 1"], "")]),
        d("DCLink", "DC link", "Drives", "The DC link capacitor of a drive, charged from a source through a resistance. Inputs: source voltage V_s, load current i_load. Outputs: link voltage V_dc, source current i_s.",
            "C·dV_dc/dt = i_s − i_load, i_s = (V_s − V_dc)/R_s. A diode rectifier only charges the link (i_s ≥ 0), so braking energy raises V_dc. Feed i_load from Inverter DC current.",
            vec![n("C", "Capacitance C", "0.002", "F", ""), n("Rs", "Source resistance R_s", "0.1", "Ω", ""), n("V0", "Initial voltage", "600", "V", ""),
                 c("source", "Source", &["two-way", "diode rectifier (charges only)"], "")]),
        d("InverterDC", "Inverter DC current", "Drives", "The current an average-model inverter draws from its DC link, from the modulation and the phase currents (power balance). Inputs: m_abc, i_abc. Output: i_dc.",
            "i_dc = ½·(Σ m_k·i_k − mean(m)·Σ i_k), so V_dc·i_dc equals the power into the motor.", vec![]),
        d("Sensor", "Current sensor", "Drives", "A measurement: gain, offset, bandwidth and noise. Works for any signal and width.", "Bandwidth 0 means an ideal, instant sensor. The noise is normally distributed, held for each sample time and repeatable with the same seed.",
            vec![n("k", "Gain", "1", "", ""), n("offset", "Offset", "0", "", ""), n("bw", "Bandwidth", "0", "Hz", "0 = ideal"), n("sd", "Noise (standard deviation)", "0", "", ""),
                 n("ts", "Noise sample time", "0.0001", "s", ""), n("seed", "Seed", "1", "", "")]),
        d("Clarke", "abc → αβ", "Drives", "Clarke transform: three-phase abc (width 3) to the stationary αβ frame (width 2).", "Amplitude-invariant: a balanced set of peak A gives α, β of amplitude A.", vec![]),
        d("InvClarke", "αβ → abc", "Drives", "Inverse Clarke transform: αβ (width 2) to three-phase abc (width 3).", "", vec![]),
        d("Park", "αβ → dq", "Drives", "Park transform: αβ (width 2) to the rotating dq frame at angle θ.", "θ is the electrical angle: pole pairs × the motor's angle θ. d = α·cos θ + β·sin θ; q = −α·sin θ + β·cos θ.", vec![]),
        d("InvPark", "dq → αβ", "Drives", "Inverse Park transform: dq (width 2) at angle θ to αβ (width 2).", "θ is the electrical angle: pole pairs × the motor's angle θ.", vec![]),
        d("Encoder", "Encoder", "Drives", "An incremental encoder: the measured angle in steps of 2π/(4·PPR) and a speed estimate from the count change over each sample time.", "Input: the motor angle θ. Outputs: measured angle, estimated speed.",
            vec![n("ppr", "Pulses per revolution", "1024", "", "Four counts per pulse"), n("ts", "Speed sample time", "0.001", "s", "")]),
        // Physical (planned)
        planned("Mass", "Mass, spring, damper", "Physical (0.4)", "Mechanical components you connect by ports.", "0.4"),
        planned("Resistor", "R, L, C and sources", "Physical (0.4)", "Electrical components you connect by ports.", "0.4"),
        planned("Pipe", "Pipe, pump, valve, tank", "Physical (0.4)", "Hydraulic components you connect by ports.", "0.4"),
        planned("HeatCap", "Heat capacity, conduction", "Physical (0.4)", "Thermal components you connect by ports.", "0.4"),
        planned("FMU", "FMU", "Subsystems", "Run a model exported by another tool (FMI standard).", "0.5"),
        planned("PyBlock", "Python block", "Math", "A block written in Python.", "0.6"),
    ]
}

fn dc_params(_series: bool) -> Vec<P> {
    vec![n("Ra", "Armature resistance Ra", "0.6", "Ω", ""), n("La", "Armature inductance La", "0.012", "H", ""), n("Rf", "Field resistance Rf", "220", "Ω", ""),
         n("Lf", "Field inductance Lf", "50", "H", ""), n("Laf", "Mutual inductance Laf", "1.3", "H", "e = Laf·i_f·ω"), n("J", "Inertia J", "0.15", "kg·m²", ""),
         n("b", "Viscous friction b", "0.01", "N·m·s/rad", ""), n("w0", "Initial speed", "0", "rad/s", "")]
}

fn im_params(double: bool) -> Vec<P> {
    let mut v = vec![n("Rs", "Stator resistance Rs", "0.37", "Ω", ""), n("Lls", "Stator leakage Lls", "0.0039", "H", ""), n("Lm", "Magnetising inductance Lm", "0.117", "H", ""),
                     n("Rr", "Rotor resistance Rr", "0.31", "Ω", "Referred to the stator"), n("Llr", "Rotor leakage Llr", "0.0039", "H", "")];
    if double {
        v[3] = n("Rr", "Outer cage resistance Rr", "0.93", "Ω", "Starting cage");
        v[4] = n("Llr", "Outer cage leakage Llr", "0.0012", "H", "");
        v.push(n("Rr2", "Inner cage resistance Rr2", "0.15", "Ω", "Running cage"));
        v.push(n("Llr2", "Inner cage leakage Llr2", "0.0058", "H", ""));
    }
    v.extend([n("p", "Pole pairs p", "2", "", ""), n("J", "Inertia J", "0.07", "kg·m²", ""), n("b", "Viscous friction b", "0", "N·m·s/rad", ""), n("w0", "Initial speed", "0", "rad/s", "")]);
    v
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
        "Integrator" | "DiscreteIntegrator" => (if param(b, "reset") == "none" || param(b, "reset").is_empty() { 1 } else { 2 }, 1),
        "Square" | "Triangle" | "Chirp" | "RepSeq" | "From" => (0, 1),
        "Goto" | "Terminator" => (1, 0),
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
        "DCSepEx" | "SyncField" => (3, 5),
        "DCShunt" | "DCCompound" => (2, 5),
        "DCSeries" | "InductionSC" | "InductionDC" | "PMSM3" | "SynRM" => (2, 4),
        "InductionWR" => (3, 5),
        "BLDC" => (2, 5),
        "Stepper" => (3, 5),
        "ThreePhase" => (0, 1),
        "Inverter" | "Park" | "InvPark" => (2, 1),
        "Encoder" => (1, 2),
        "SRM" => (2, 4),
        "DCLink" | "InverterDC" => (2, if b.kind == "DCLink" { 2 } else { 1 }),
        _ => (1, 1),
    }
}

/// Port labels, shown beside the ports when a block has named ones.
pub fn port_names(b: &Block) -> (Vec<String>, Vec<String>) {
    let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
    match b.kind.as_str() {
        "DCMotor" => (s(&["V", "T_L"]), s(&["ω", "i", "θ"])),
        "PMSM" => (s(&["v_d", "v_q", "T_L"]), s(&["ω", "i_d", "i_q", "T_e", "θ"])),
        "DCSepEx" => (s(&["V_a", "V_f", "T_L"]), s(&["ω", "i_a", "i_f", "T_e", "θ"])),
        "DCShunt" | "DCCompound" => (s(&["V", "T_L"]), s(&["ω", "i_a", "i_f", "T_e", "θ"])),
        "DCSeries" => (s(&["V", "T_L"]), s(&["ω", "i", "T_e", "θ"])),
        "InductionSC" | "InductionDC" | "PMSM3" | "SynRM" => (s(&["v_abc", "T_L"]), s(&["ω", "T_e", "i_abc", "θ"])),
        "InductionWR" => (s(&["v_abc", "vr_abc", "T_L"]), s(&["ω", "T_e", "i_abc", "θ", "ir_abc"])),
        "SyncField" => (s(&["v_abc", "v_f", "T_L"]), s(&["ω", "T_e", "i_abc", "i_f", "θ"])),
        "BLDC" => (s(&["V", "T_L"]), s(&["ω", "i", "T_e", "θ", "hall"])),
        "Stepper" => (s(&["v_A", "v_B", "T_L"]), s(&["ω", "T_e", "θ", "i_A", "i_B"])),
        "Inverter" => (s(&["m_abc", "V_dc"]), s(&["v_abc"])),
        "Park" | "InvPark" => (s(&["", "θ"]), vec![]),
        "Encoder" => (vec![], s(&["θ_m", "ω_m"])),
        "SRM" => (s(&["v_abc", "T_L"]), s(&["ω", "T_e", "i_abc", "θ"])),
        "DCLink" => (s(&["V_s", "i_load"]), s(&["V_dc", "i_s"])),
        "InverterDC" => (s(&["m_abc", "i_abc"]), vec![]),
        "Subsystem" => {
            let Some(sd) = &b.sub else { return (vec![], vec![]) };
            let (i, o) = sub_ports(sd);
            let nm = |ids: Vec<u32>| ids.iter().map(|id| sd.block(*id).map(|b| b.name.clone()).unwrap_or_default()).collect();
            (nm(i), nm(o))
        }
        "Switch" => (s(&["", "u2", ""]), vec![]),
        "Integrator" | "DiscreteIntegrator" if ports(b).0 == 2 => (s(&["", "reset"]), vec![]),
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
        "DiscreteTF" => format!("{}\n{}", p("num"), p("den")),
        "DiscreteSS" => "x[k+1]".into(),
        "DiscreteIntegrator" => "K·Ts/(z−1)".into(),
        "DiscretePID" => format!("P {} · I {} · D {}", p("P"), p("I"), p("D")),
        "RateTransition" => format!("→ {} s", p("ts")),
        "Goto" => format!("[{}]", p("tag")),
        "From" => format!("[{}]", p("tag")),
        "Step" => format!("{} → {}", p("initial"), p("final")),
        "Table" | "Lookup1D" => "f(x)".into(),
        "FromFile" | "ToFile" => p("file"),
        "ThreePhase" => format!("{} V {} Hz", p("Vll"), p("f")),
        "PWM" => format!("{} Hz", p("fc")),
        "Clarke" => "abc→αβ".into(),
        "InvClarke" => "αβ→abc".into(),
        "Park" => "αβ→dq".into(),
        "InvPark" => "dq→αβ".into(),
        _ => String::new(),
    }
}
