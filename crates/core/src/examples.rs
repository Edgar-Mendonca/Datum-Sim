//! Worked examples and templates, built in code so they always match the current block library.

use crate::model::{Block, Diagram, Model, Var, Wire};
use serde::Serialize;

#[derive(Serialize)]
pub struct Example {
    pub id: &'static str,
    pub name: &'static str,
    pub what: &'static str,
}

pub fn list() -> Vec<Example> {
    vec![
        Example { id: "blank", name: "Blank model", what: "An empty diagram." },
        Example { id: "msd", name: "Mass–spring–damper", what: "A mass on a spring and damper pushed by a step force: the classic second-order response." },
        Example { id: "dcmotor", name: "DC motor speed control (PID)", what: "A DC motor built from blocks inside a subsystem, held at speed by a PID controller while a load torque is applied." },
        Example { id: "dcpos", name: "DC motor position control", what: "The DC motor block with a PID position loop and voltage limits." },
        Example { id: "pmsm", name: "PMSM speed control (field-oriented)", what: "A permanent-magnet synchronous motor in the d–q frame with a speed loop and two current loops." },
        Example { id: "imdol", name: "Induction motor: direct-on-line start", what: "A 400 V, 11 kW squirrel-cage motor switched straight onto the mains, starting a fan: the inrush current, the torque and the run-up." },
        Example { id: "imvf", name: "Induction motor: V/f speed control", what: "A variable-frequency supply built from blocks (angle integrator and inverse transforms) ramps the motor up with constant volts per hertz." },
        Example { id: "pmsmfoc", name: "PMSM: field-oriented control with an inverter", what: "A 15 kW interior-magnet motor fed by a three-phase inverter: currents measured in abc, Clarke and Park transforms, current and speed loops." },
        Example { id: "bldc", name: "BLDC motor speed control", what: "A 48 V brushless DC motor held at speed by a PI controller through a load step." },
        Example { id: "syncmotor", name: "Synchronous motor on the grid (3.3 kV)", what: "A 1 MVA wound-field salient-pole motor on a 3.3 kV supply takes a load step: the rotor swings and the dampers settle it back to synchronous speed." },
        Example { id: "dcseries", name: "DC series motor: traction start", what: "A 600 V series motor starts a heavy load as its voltage is ramped up: high torque at low speed." },
        Example { id: "stepper", name: "Stepper motor: full steps", what: "A hybrid stepper driven with two-phase-on full steps at 200 steps per second." },
        Example { id: "pmsmdigital", name: "PMSM: digital drive with a DC link", what: "Field-oriented control as a microcontroller runs it: a 10 kHz current loop and a 1 kHz speed loop, Goto/From tags, and a DC link fed by a diode rectifier that rises while the motor brakes." },
        Example { id: "dcdigital", name: "DC motor: digital and continuous PID", what: "The same DC motor speed loop twice, with a continuous PID and a discrete PID at 5 ms: compare the responses and the staircase of the digital controller." },
        Example { id: "srm", name: "Switched reluctance motor speed control", what: "A 6/4 switched reluctance motor with angle-based phase switching from an asymmetric bridge and a PI speed loop." },
        Example { id: "tank", name: "Pump and tank level control", what: "A pump fills a tank that drains through a valve (flow ∝ √level); a PI controller holds the level." },
        Example { id: "cruise", name: "Vehicle cruise control", what: "A car with aerodynamic drag and rolling resistance; a PI controller sets the drive force to hold the speed." },
        Example { id: "thermostat", name: "Room thermostat (relay)", what: "A heater switched by a relay with hysteresis keeps a room warm; switching times are located exactly." },
    ]
}

struct B {
    m: Model,
    stack: Vec<Diagram>,
}

impl B {
    fn new(name: &str, notes: &str, stop: f64) -> B {
        let mut m = Model::new();
        m.name = name.into();
        m.notes = notes.into();
        m.sim.stop = stop;
        B { m, stack: vec![Diagram::default()] }
    }
    fn var(&mut self, name: &str, expr: &str, unit: &str, note: &str) {
        self.m.vars.push(Var { name: name.into(), expr: expr.into(), unit: unit.into(), note: note.into() });
    }
    fn d(&mut self) -> &mut Diagram {
        self.stack.last_mut().unwrap()
    }
    fn blk(&mut self, kind: &str, name: &str, x: f64, y: f64, params: &[(&str, &str)]) -> u32 {
        let id = self.m.id();
        let (w, h) = crate::blocks::size(kind);
        let mut b = Block { id, kind: kind.into(), name: name.into(), x, y, w, h, ..Default::default() };
        for (k, v) in params {
            b.params.insert((*k).into(), (*v).into());
        }
        self.d().blocks.push(b);
        id
    }
    /// A motor block with a typical-ratings preset, plus overrides.
    fn motor(&mut self, kind: &str, preset: usize, name: &str, x: f64, y: f64, extra: &[(&str, &str)]) -> u32 {
        let pr = crate::motors::presets(kind).swap_remove(preset).values;
        let mut v: Vec<(&str, &str)> = pr.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        v.extend_from_slice(extra);
        self.blk(kind, name, x, y, &v)
    }
    fn flip(&mut self, id: u32) {
        if let Some(b) = self.d().blocks.iter_mut().find(|b| b.id == id) {
            b.flip = true;
        }
    }
    fn size(&mut self, id: u32, w: f64, h: f64) {
        if let Some(b) = self.d().blocks.iter_mut().find(|b| b.id == id) {
            b.w = w;
            b.h = h;
        }
    }
    fn w(&mut self, from: u32, fp: usize, to: u32, tp: usize) {
        let id = self.m.id();
        self.d().wires.push(Wire { id, from: (from, fp), to: (to, tp), pts: vec![] });
    }
    fn begin(&mut self) {
        self.stack.push(Diagram::default());
    }
    fn end(&mut self, name: &str, x: f64, y: f64, w: f64, h: f64) -> u32 {
        let sub = self.stack.pop().unwrap();
        let id = self.m.id();
        self.d().blocks.push(Block { id, kind: "Subsystem".into(), name: name.into(), x, y, w, h, sub: Some(sub), ..Default::default() });
        id
    }
    fn done(mut self) -> Model {
        self.m.root = self.stack.pop().unwrap();
        self.m
    }
}

pub fn get(id: &str) -> Option<Model> {
    Some(match id {
        "blank" => Model::new(),
        "msd" => msd(),
        "dcmotor" => dc_motor_speed(),
        "dcpos" => dc_motor_position(),
        "pmsm" => pmsm(),
        "imdol" => im_dol(),
        "imvf" => im_vf(),
        "pmsmfoc" => pmsm_foc(),
        "bldc" => bldc(),
        "syncmotor" => sync_motor(),
        "dcseries" => dc_series(),
        "stepper" => stepper(),
        "pmsmdigital" => pmsm_digital(),
        "dcdigital" => dc_digital(),
        "srm" => srm(),
        "tank" => tank(),
        "cruise" => cruise(),
        "thermostat" => thermostat(),
        _ => return None,
    })
}

fn msd() -> Model {
    let mut b = B::new("Mass–spring–damper", "m·x'' + c·x' + k·x = F. With these values ζ = 0.2 and ωn = 3.16 rad/s: about 53 % overshoot.", 10.0);
    b.var("m", "1", "kg", "Mass");
    b.var("k", "10", "N/m", "Spring stiffness");
    b.var("zeta", "0.2", "", "Damping ratio");
    b.var("c", "2*zeta*sqrt(k*m)", "N·s/m", "Damping coefficient");
    let f = b.blk("Step", "Force", 40.0, 100.0, &[("time", "0.5"), ("final", "10")]);
    let s = b.blk("Sum", "Sum", 160.0, 102.0, &[("signs", "+--")]);
    let g = b.blk("Gain", "1/m", 250.0, 100.0, &[("k", "1/m")]);
    let v = b.blk("Integrator", "Velocity", 360.0, 100.0, &[]);
    let x = b.blk("Integrator", "Position", 480.0, 100.0, &[]);
    let kc = b.blk("Gain", "Damper", 360.0, 200.0, &[("k", "c")]);
    let kk = b.blk("Gain", "Spring", 480.0, 270.0, &[("k", "k")]);
    b.flip(kc);
    b.flip(kk);
    let sc = b.blk("Scope", "Scope", 640.0, 95.0, &[("n", "2")]);
    b.w(f, 0, s, 0);
    b.w(s, 0, g, 0);
    b.w(g, 0, v, 0);
    b.w(v, 0, x, 0);
    b.w(x, 0, sc, 0);
    b.w(v, 0, sc, 1);
    b.w(v, 0, kc, 0);
    b.w(kc, 0, s, 1);
    b.w(x, 0, kk, 0);
    b.w(kk, 0, s, 2);
    b.done()
}

/// The DC motor built from blocks inside a subsystem (inputs V and T_L; outputs ω and i).
fn motor_sub(b: &mut B, x: f64, y: f64) -> u32 {
    b.begin();
    let v = b.blk("Inport", "V", 20.0, 60.0, &[("port", "1")]);
    let tl = b.blk("Inport", "T_L", 20.0, 240.0, &[("port", "2")]);
    let se = b.blk("Sum", "Voltage balance", 120.0, 62.0, &[("signs", "+--")]);
    let gl = b.blk("Gain", "1/L", 200.0, 60.0, &[("k", "1/L")]);
    let ii = b.blk("Integrator", "Current", 300.0, 60.0, &[]);
    let kt = b.blk("Gain", "Kt", 400.0, 60.0, &[("k", "Kt")]);
    let sm = b.blk("Sum", "Torque balance", 500.0, 62.0, &[("signs", "+--")]);
    let gj = b.blk("Gain", "1/J", 580.0, 60.0, &[("k", "1/J")]);
    let wi = b.blk("Integrator", "Speed", 680.0, 60.0, &[]);
    let r = b.blk("Gain", "R", 300.0, 150.0, &[("k", "R")]);
    let ke = b.blk("Gain", "Ke", 560.0, 300.0, &[("k", "Ke")]);
    let bf = b.blk("Gain", "Friction b", 680.0, 160.0, &[("k", "b")]);
    for id in [r, ke, bf] {
        b.flip(id);
    }
    let ow = b.blk("Outport", "ω", 820.0, 68.0, &[("port", "1")]);
    let oi = b.blk("Outport", "i", 820.0, 140.0, &[("port", "2")]);
    b.w(v, 0, se, 0);
    b.w(se, 0, gl, 0);
    b.w(gl, 0, ii, 0);
    b.w(ii, 0, kt, 0);
    b.w(kt, 0, sm, 0);
    b.w(sm, 0, gj, 0);
    b.w(gj, 0, wi, 0);
    b.w(wi, 0, ow, 0);
    b.w(ii, 0, oi, 0);
    b.w(ii, 0, r, 0);
    b.w(r, 0, se, 1);
    b.w(wi, 0, ke, 0);
    b.w(ke, 0, se, 2);
    b.w(wi, 0, bf, 0);
    b.w(bf, 0, sm, 1);
    b.w(tl, 0, sm, 2);
    b.end("DC motor", x, y, 110.0, 70.0)
}

fn motor_vars(b: &mut B) {
    b.var("R", "1", "Ω", "Armature resistance");
    b.var("L", "0.5", "H", "Armature inductance");
    b.var("Ke", "0.01", "V·s/rad", "Back-EMF constant");
    b.var("Kt", "0.01", "N·m/A", "Torque constant");
    b.var("J", "0.01", "kg·m²", "Rotor inertia");
    b.var("b", "0.1", "N·m·s/rad", "Viscous friction");
}

fn dc_motor_speed() -> Model {
    let mut b = B::new(
        "DC motor speed control",
        "A permanent-magnet DC motor (open the DC motor subsystem to see its equations) with a PID speed controller. The reference steps to 1 rad/s; at 3 s a load torque is applied and the controller recovers.",
        5.0,
    );
    motor_vars(&mut b);
    b.var("Kp", "100", "", "Proportional gain");
    b.var("Ki", "200", "1/s", "Integral gain");
    b.var("Kd", "10", "s", "Derivative gain");
    let r = b.blk("Step", "Speed reference", 30.0, 80.0, &[("time", "0"), ("final", "1")]);
    let e = b.blk("Sum", "Error", 150.0, 82.0, &[("signs", "+-")]);
    let c = b.blk("PID", "PID", 230.0, 78.0, &[("P", "Kp"), ("I", "Ki"), ("D", "Kd"), ("upper", "24"), ("lower", "-24")]);
    let tl = b.blk("Step", "Load torque", 230.0, 180.0, &[("time", "3"), ("final", "0.05")]);
    let m = motor_sub(&mut b, 380.0, 90.0);
    let s1 = b.blk("Scope", "Speed", 580.0, 40.0, &[("n", "2")]);
    let s2 = b.blk("Scope", "Current and voltage", 580.0, 180.0, &[("n", "2")]);
    b.w(r, 0, e, 0);
    b.w(e, 0, c, 0);
    b.w(c, 0, m, 0);
    b.w(tl, 0, m, 1);
    b.w(m, 0, e, 1);
    b.w(m, 0, s1, 0);
    b.w(r, 0, s1, 1);
    b.w(m, 1, s2, 0);
    b.w(c, 0, s2, 1);
    b.done()
}

fn dc_motor_position() -> Model {
    let mut b = B::new(
        "DC motor position control",
        "The DC motor library block turns to 1 rad. The PID output (the armature voltage) is limited to ±12 V; the integrator stops winding up while it is limited.",
        3.0,
    );
    let r = b.blk("Step", "Angle reference", 30.0, 80.0, &[("time", "0.1"), ("final", "1")]);
    let e = b.blk("Sum", "Error", 150.0, 82.0, &[("signs", "+-")]);
    let c = b.blk("PID", "PID", 230.0, 78.0, &[("P", "50"), ("I", "5"), ("D", "5"), ("upper", "12"), ("lower", "-12")]);
    let tl = b.blk("Constant", "No load", 230.0, 160.0, &[("value", "0")]);
    let m = b.blk("DCMotor", "Motor", 380.0, 80.0, &[("R", "2"), ("L", "0.002"), ("Ke", "0.05"), ("Kt", "0.05"), ("J", "1e-4"), ("b", "1e-5")]);
    let s1 = b.blk("Scope", "Angle", 580.0, 40.0, &[("n", "2")]);
    let s2 = b.blk("Scope", "Voltage and current", 580.0, 170.0, &[("n", "2")]);
    b.w(r, 0, e, 0);
    b.w(e, 0, c, 0);
    b.w(c, 0, m, 0);
    b.w(tl, 0, m, 1);
    b.w(m, 2, e, 1);
    b.w(m, 2, s1, 0);
    b.w(r, 0, s1, 1);
    b.w(c, 0, s2, 0);
    b.w(m, 1, s2, 1);
    b.done()
}

fn pmsm() -> Model {
    let mut b = B::new(
        "PMSM speed control",
        "Field-oriented control of a permanent-magnet synchronous motor: the speed loop asks for q-axis current (torque); i_d is held at zero; two PI current loops set v_d and v_q. A load torque is applied at 0.1 s.",
        0.2,
    );
    b.var("Rs", "0.5", "Ω", "Stator resistance");
    b.var("Ls", "1e-3", "H", "Stator inductance (Ld = Lq)");
    b.var("psi", "0.05", "Wb", "Magnet flux");
    b.var("pp", "4", "", "Pole pairs");
    b.var("Jm", "5e-4", "kg·m²", "Inertia");
    b.var("wc", "2000", "rad/s", "Current loop bandwidth");
    b.var("Kt", "1.5*pp*psi", "N·m/A", "Torque constant");
    b.var("ws", "300", "rad/s", "Speed loop bandwidth");
    let wr = b.blk("Step", "Speed reference", 20.0, 60.0, &[("time", "0.005"), ("final", "100")]);
    let ew = b.blk("Sum", "Speed error", 130.0, 62.0, &[("signs", "+-")]);
    let pw = b.blk("PID", "Speed PI", 200.0, 58.0, &[("P", "Jm*ws/Kt"), ("I", "Jm*ws/Kt*ws/4"), ("upper", "15"), ("lower", "-15")]);
    let idr = b.blk("Constant", "i_d reference", 200.0, 160.0, &[("value", "0")]);
    let eq = b.blk("Sum", "i_q error", 340.0, 62.0, &[("signs", "+-")]);
    let ed = b.blk("Sum", "i_d error", 340.0, 162.0, &[("signs", "+-")]);
    let pq = b.blk("PID", "i_q PI", 410.0, 58.0, &[("P", "Ls*wc"), ("I", "Rs*wc"), ("upper", "48"), ("lower", "-48")]);
    let pd = b.blk("PID", "i_d PI", 410.0, 158.0, &[("P", "Ls*wc"), ("I", "Rs*wc"), ("upper", "48"), ("lower", "-48")]);
    let tl = b.blk("Step", "Load torque", 410.0, 260.0, &[("time", "0.1"), ("final", "1")]);
    let m = b.blk("PMSM", "Motor", 580.0, 120.0, &[("Rs", "Rs"), ("Ld", "Ls"), ("Lq", "Ls"), ("psi", "psi"), ("p", "pp"), ("J", "Jm"), ("b", "1e-4")]);
    b.size(m, 100.0, 110.0);
    let s1 = b.blk("Scope", "Speed", 760.0, 40.0, &[("n", "2")]);
    let s2 = b.blk("Scope", "Currents", 760.0, 140.0, &[("n", "2")]);
    let s3 = b.blk("Scope", "Torque", 760.0, 240.0, &[]);
    b.w(wr, 0, ew, 0);
    b.w(ew, 0, pw, 0);
    b.w(pw, 0, eq, 0);
    b.w(idr, 0, ed, 0);
    b.w(eq, 0, pq, 0);
    b.w(ed, 0, pd, 0);
    b.w(pd, 0, m, 0);
    b.w(pq, 0, m, 1);
    b.w(tl, 0, m, 2);
    b.w(m, 0, ew, 1);
    b.w(m, 1, ed, 1);
    b.w(m, 2, eq, 1);
    b.w(m, 0, s1, 0);
    b.w(wr, 0, s1, 1);
    b.w(m, 1, s2, 0);
    b.w(m, 2, s2, 1);
    b.w(m, 3, s3, 0);
    b.done()
}

fn tank() -> Model {
    let mut b = B::new(
        "Pump and tank level control",
        "A·dh/dt = q_in − q_out, with q_out = Cv·√h through the drain valve. The pump responds with a 2 s lag; the PI controller's output (the pump command) is limited to 0…3 m³/s. The set point rises from 1 m to 2 m at 10 s.",
        60.0,
    );
    b.var("A", "2", "m²", "Tank cross-section");
    b.var("Cv", "0.5", "m^2.5/s", "Drain valve coefficient");
    let sp = b.blk("Step", "Level set point", 20.0, 80.0, &[("time", "10"), ("initial", "1"), ("final", "2")]);
    let e = b.blk("Sum", "Error", 140.0, 82.0, &[("signs", "+-")]);
    let c = b.blk("PID", "PI", 210.0, 78.0, &[("P", "2"), ("I", "0.2"), ("upper", "3"), ("lower", "0"), ("ic", "2.5")]);
    let p = b.blk("TransferFcn", "Pump", 340.0, 78.0, &[("num", "[1]"), ("den", "[2 1]")]);
    let s = b.blk("Sum", "Flow balance", 470.0, 82.0, &[("signs", "+-")]);
    let ga = b.blk("Gain", "1/A", 540.0, 80.0, &[("k", "1/A")]);
    let h = b.blk("Integrator", "Level", 640.0, 80.0, &[("ic", "1"), ("lower", "0")]);
    let v = b.blk("Fcn", "Drain valve", 520.0, 190.0, &[("expr", "Cv*sqrt(max(u, 0))")]);
    b.flip(v);
    let sc = b.blk("Scope", "Level", 780.0, 40.0, &[("n", "2")]);
    let sf = b.blk("Scope", "Flows", 780.0, 170.0, &[("n", "2")]);
    b.w(sp, 0, e, 0);
    b.w(e, 0, c, 0);
    b.w(c, 0, p, 0);
    b.w(p, 0, s, 0);
    b.w(s, 0, ga, 0);
    b.w(ga, 0, h, 0);
    b.w(h, 0, v, 0);
    b.w(v, 0, s, 1);
    b.w(h, 0, e, 1);
    b.w(h, 0, sc, 0);
    b.w(sp, 0, sc, 1);
    b.w(p, 0, sf, 0);
    b.w(v, 0, sf, 1);
    b.done()
}

fn cruise() -> Model {
    let mut b = B::new(
        "Vehicle cruise control",
        "m·dv/dt = F − ½·ρ·Cd·A·v² − Crr·m·g. The PI controller sets the drive force (limited to −5 kN…5 kN); the set speed rises from 20 to 25 m/s at 5 s.",
        40.0,
    );
    b.var("mass", "1200", "kg", "Vehicle mass");
    b.var("rho", "1.2", "kg/m³", "Air density");
    b.var("CdA", "0.3*2.2", "m²", "Drag coefficient × frontal area");
    b.var("Crr", "0.015", "", "Rolling resistance coefficient");
    let sp = b.blk("Step", "Set speed", 20.0, 80.0, &[("time", "5"), ("initial", "20"), ("final", "25")]);
    let e = b.blk("Sum", "Error", 140.0, 82.0, &[("signs", "+-")]);
    let c = b.blk("PID", "PI", 210.0, 78.0, &[("P", "1000"), ("I", "100"), ("upper", "5000"), ("lower", "-5000"), ("ic", "3.4")]);
    let s = b.blk("Sum", "Forces", 360.0, 82.0, &[("signs", "+-")]);
    let gm = b.blk("Gain", "1/m", 430.0, 80.0, &[("k", "1/mass")]);
    let v = b.blk("Integrator", "Speed", 530.0, 80.0, &[("ic", "20")]);
    let r = b.blk("Fcn", "Resistance", 400.0, 190.0, &[("expr", "0.5*rho*CdA*u*abs(u) + Crr*mass*9.81")]);
    b.flip(r);
    let sc = b.blk("Scope", "Speed", 680.0, 40.0, &[("n", "2")]);
    let sf = b.blk("Scope", "Drive force", 680.0, 170.0, &[]);
    b.w(sp, 0, e, 0);
    b.w(e, 0, c, 0);
    b.w(c, 0, s, 0);
    b.w(s, 0, gm, 0);
    b.w(gm, 0, v, 0);
    b.w(v, 0, r, 0);
    b.w(r, 0, s, 1);
    b.w(v, 0, e, 1);
    b.w(v, 0, sc, 0);
    b.w(sp, 0, sc, 1);
    b.w(c, 0, sf, 0);
    b.done()
}

fn thermostat() -> Model {
    let mut b = B::new(
        "Room thermostat",
        "C·dT/dt = Q − (T − T_out)/R. The relay switches the 3 kW heater on below 19.5 °C and off above 20.5 °C. Every switching time is located exactly.",
        20000.0,
    );
    b.var("C", "2e6", "J/K", "Heat capacity of the room");
    b.var("Rth", "0.01", "K/W", "Thermal resistance to outside");
    b.var("Tout", "5", "°C", "Outside temperature");
    let sp = b.blk("Constant", "Set point", 20.0, 80.0, &[("value", "20")]);
    let e = b.blk("Sum", "Error", 140.0, 82.0, &[("signs", "+-")]);
    let r = b.blk("Relay", "Thermostat", 210.0, 80.0, &[("on", "0.5"), ("off", "-0.5"), ("onv", "3000"), ("offv", "0")]);
    let s = b.blk("Sum", "Heat balance", 340.0, 82.0, &[("signs", "+-")]);
    let gc = b.blk("Gain", "1/C", 410.0, 80.0, &[("k", "1/C")]);
    let t = b.blk("Integrator", "Room temperature", 520.0, 80.0, &[("ic", "15")]);
    let l = b.blk("Fcn", "Heat loss", 400.0, 190.0, &[("expr", "(u - Tout)/Rth")]);
    b.flip(l);
    let sc = b.blk("Scope", "Temperature", 680.0, 40.0, &[]);
    let sq = b.blk("Scope", "Heater", 680.0, 170.0, &[]);
    b.w(sp, 0, e, 0);
    b.w(e, 0, r, 0);
    b.w(r, 0, s, 0);
    b.w(s, 0, gc, 0);
    b.w(gc, 0, t, 0);
    b.w(t, 0, l, 0);
    b.w(l, 0, s, 1);
    b.w(t, 0, e, 1);
    b.w(t, 0, sc, 0);
    b.w(r, 0, sq, 0);
    b.done()
}

fn im_dol() -> Model {
    let mut b = B::new(
        "Induction motor: direct-on-line start",
        "A 400 V, 50 Hz, 11 kW, 4-pole squirrel-cage motor (typical values) switched onto the mains at t = 0 with a fan on its shaft (torque ∝ speed²). Watch the starting current of several times rated, the torque pulsations while the flux builds, and the run-up to just below 1500 rpm (157 rad/s).",
        0.6,
    );
    b.var("Tn", "72", "N·m", "Fan torque at synchronous speed");
    b.var("wsync", "2*pi*50/2", "rad/s", "Synchronous speed");
    let src = b.blk("ThreePhase", "Mains 400 V", 20.0, 60.0, &[("Vll", "400"), ("f", "50")]);
    let m = b.motor("InductionSC", 0, "Motor 11 kW", 200.0, 60.0, &[]);
    let fan = b.blk("Fcn", "Fan load", 200.0, 200.0, &[("expr", "Tn*(u/wsync)^2")]);
    b.flip(fan);
    let s1 = b.blk("Scope", "Speed", 400.0, 20.0, &[]);
    let s2 = b.blk("Scope", "Torque", 400.0, 100.0, &[("n", "2")]);
    let s3 = b.blk("Scope", "Stator currents", 400.0, 180.0, &[]);
    b.w(src, 0, m, 0);
    b.w(fan, 0, m, 1);
    b.w(m, 0, fan, 0);
    b.w(m, 0, s1, 0);
    b.w(m, 1, s2, 0);
    b.w(fan, 0, s2, 1);
    b.w(m, 2, s3, 0);
    b.done()
}

fn im_vf() -> Model {
    let mut b = B::new(
        "Induction motor: V/f speed control",
        "Open-loop volts-per-hertz control. The frequency ramps to 50 Hz in 2 s; its integral is the supply angle; the voltage rises in proportion to the frequency (with a small boost at low speed). The inverse Park and Clarke transforms turn amplitude and angle into three phase voltages. A load torque of 50 N·m is applied at 3 s.",
        4.0,
    );
    b.var("Vpk", "400*sqrt(2/3)", "V", "Rated phase peak voltage");
    b.var("boost", "0.03", "", "Voltage boost at zero frequency (fraction of rated)");
    let fr = b.blk("Ramp", "Frequency ramp", 20.0, 40.0, &[("slope", "25")]);
    let sat = b.blk("Saturation", "Up to 50 Hz", 110.0, 40.0, &[("upper", "50"), ("lower", "0")]);
    let we = b.blk("Gain", "2π", 200.0, 40.0, &[("k", "2*pi")]);
    let th = b.blk("Integrator", "Supply angle", 290.0, 40.0, &[]);
    let vf = b.blk("Fcn", "V/f", 200.0, 120.0, &[("expr", "Vpk*(boost + (1 - boost)*u/50)")]);
    let z = b.blk("Constant", "q = 0", 200.0, 190.0, &[("value", "0")]);
    let mx = b.blk("Mux", "", 300.0, 120.0, &[]);
    let ip = b.blk("InvPark", "", 360.0, 110.0, &[]);
    let ic = b.blk("InvClarke", "", 450.0, 110.0, &[]);
    let tl = b.blk("Step", "Load torque", 450.0, 200.0, &[("time", "3"), ("final", "50")]);
    let m = b.motor("InductionSC", 0, "Motor 11 kW", 560.0, 120.0, &[]);
    let s1 = b.blk("Scope", "Speed", 740.0, 40.0, &[]);
    let s2 = b.blk("Scope", "Torque", 740.0, 120.0, &[]);
    let s3 = b.blk("Scope", "Stator currents", 740.0, 200.0, &[]);
    b.w(fr, 0, sat, 0);
    b.w(sat, 0, we, 0);
    b.w(we, 0, th, 0);
    b.w(sat, 0, vf, 0);
    b.w(vf, 0, mx, 0);
    b.w(z, 0, mx, 1);
    b.w(mx, 0, ip, 0);
    b.w(th, 0, ip, 1);
    b.w(ip, 0, ic, 0);
    b.w(ic, 0, m, 0);
    b.w(tl, 0, m, 1);
    b.w(m, 0, s1, 0);
    b.w(m, 1, s2, 0);
    b.w(m, 2, s3, 0);
    b.done()
}

fn pmsm_foc() -> Model {
    let mut b = B::new(
        "PMSM: field-oriented control with an inverter",
        "A 400 V, 15 kW interior-magnet motor fed by a 600 V three-phase inverter (average model; set the Inverter to switched to see the PWM). The phase currents are measured in abc, turned into d and q with the Clarke and Park transforms at the electrical angle p·θ, and held by two PI loops; the speed loop asks for q-axis current. Speed reference 150 rad/s; a 50 N·m load at 0.3 s.",
        0.5,
    );
    b.var("pp", "4", "", "Pole pairs");
    b.var("Jm", "0.03", "kg·m²", "Inertia");
    b.var("Kt", "1.5*pp*0.4", "N·m/A", "Torque constant 1.5·p·ψ");
    b.var("wc", "2000", "rad/s", "Current loop bandwidth");
    b.var("ws", "100", "rad/s", "Speed loop bandwidth");
    b.var("Vdc", "600", "V", "DC link voltage");
    let wr = b.blk("Step", "Speed reference", 20.0, 40.0, &[("time", "0.01"), ("final", "150")]);
    let ew = b.blk("Sum", "", 120.0, 42.0, &[("signs", "+-")]);
    let pw = b.blk("PID", "Speed PI", 180.0, 38.0, &[("P", "Jm*ws/Kt"), ("I", "Jm*ws/Kt*ws/4"), ("upper", "40"), ("lower", "-40")]);
    let idr = b.blk("Constant", "i_d reference", 180.0, 130.0, &[("value", "0")]);
    let eq = b.blk("Sum", "", 300.0, 42.0, &[("signs", "+-")]);
    let ed = b.blk("Sum", "", 300.0, 132.0, &[("signs", "+-")]);
    let pq = b.blk("PID", "i_q PI", 360.0, 38.0, &[("P", "0.004*wc"), ("I", "0.15*wc"), ("upper", "Vdc/2"), ("lower", "-Vdc/2")]);
    let pd = b.blk("PID", "i_d PI", 360.0, 128.0, &[("P", "0.0025*wc"), ("I", "0.15*wc"), ("upper", "Vdc/2"), ("lower", "-Vdc/2")]);
    let mx = b.blk("Mux", "", 480.0, 60.0, &[]);
    let ip = b.blk("InvPark", "", 520.0, 60.0, &[]);
    let ic = b.blk("InvClarke", "", 600.0, 60.0, &[]);
    let gm = b.blk("Gain", "Modulation", 680.0, 60.0, &[("k", "2/Vdc")]);
    let dc = b.blk("Constant", "DC link", 680.0, 140.0, &[("value", "Vdc")]);
    let inv = b.blk("Inverter", "Inverter", 780.0, 70.0, &[("model", "average"), ("fc", "10000")]);
    let tl = b.blk("Step", "Load torque", 780.0, 180.0, &[("time", "0.3"), ("final", "50")]);
    let m = b.motor("PMSM3", 1, "Motor 15 kW", 900.0, 70.0, &[]);
    let gp = b.blk("Gain", "p", 900.0, 220.0, &[("k", "pp")]);
    b.flip(gp);
    let cl = b.blk("Clarke", "", 700.0, 260.0, &[]);
    b.flip(cl);
    let pk = b.blk("Park", "", 600.0, 260.0, &[]);
    b.flip(pk);
    let dm = b.blk("Demux", "", 540.0, 250.0, &[]);
    b.flip(dm);
    let s1 = b.blk("Scope", "Speed", 1080.0, 20.0, &[("n", "2")]);
    let s2 = b.blk("Scope", "Torque", 1080.0, 100.0, &[]);
    let s3 = b.blk("Scope", "Phase currents", 1080.0, 180.0, &[]);
    let s4 = b.blk("Scope", "i_d, i_q", 420.0, 260.0, &[("n", "2")]);
    b.w(wr, 0, ew, 0);
    b.w(ew, 0, pw, 0);
    b.w(pw, 0, eq, 0);
    b.w(idr, 0, ed, 0);
    b.w(eq, 0, pq, 0);
    b.w(ed, 0, pd, 0);
    b.w(pd, 0, mx, 0);
    b.w(pq, 0, mx, 1);
    b.w(mx, 0, ip, 0);
    b.w(gp, 0, ip, 1);
    b.w(ip, 0, ic, 0);
    b.w(ic, 0, gm, 0);
    b.w(gm, 0, inv, 0);
    b.w(dc, 0, inv, 1);
    b.w(inv, 0, m, 0);
    b.w(tl, 0, m, 1);
    b.w(m, 3, gp, 0);
    b.w(m, 2, cl, 0);
    b.w(cl, 0, pk, 0);
    b.w(gp, 0, pk, 1);
    b.w(pk, 0, dm, 0);
    b.w(dm, 0, ed, 1);
    b.w(dm, 1, eq, 1);
    b.w(m, 0, ew, 1);
    b.w(m, 0, s1, 0);
    b.w(wr, 0, s1, 1);
    b.w(m, 1, s2, 0);
    b.w(m, 2, s3, 0);
    b.w(dm, 0, s4, 0);
    b.w(dm, 1, s4, 1);
    b.done()
}

fn bldc() -> Model {
    let mut b = B::new(
        "BLDC motor speed control",
        "A 48 V, 750 W brushless DC motor (ideal six-step commutation) with a PI speed loop that sets the DC voltage, limited to 0…48 V. Speed reference 300 rad/s; a 1.5 N·m load at 0.3 s.",
        0.6,
    );
    let wr = b.blk("Step", "Speed reference", 20.0, 40.0, &[("time", "0.01"), ("final", "300")]);
    let e = b.blk("Sum", "", 120.0, 42.0, &[("signs", "+-")]);
    let c = b.blk("PID", "Speed PI", 180.0, 38.0, &[("P", "0.5"), ("I", "10"), ("upper", "48"), ("lower", "0")]);
    let tl = b.blk("Step", "Load torque", 180.0, 130.0, &[("time", "0.3"), ("final", "1.5")]);
    let m = b.motor("BLDC", 1, "Motor 750 W", 320.0, 50.0, &[]);
    let s1 = b.blk("Scope", "Speed", 500.0, 20.0, &[("n", "2")]);
    let s2 = b.blk("Scope", "Current", 500.0, 100.0, &[]);
    let s3 = b.blk("Scope", "Hall signals", 500.0, 180.0, &[]);
    b.w(wr, 0, e, 0);
    b.w(e, 0, c, 0);
    b.w(c, 0, m, 0);
    b.w(tl, 0, m, 1);
    b.w(m, 0, e, 1);
    b.w(m, 0, s1, 0);
    b.w(wr, 0, s1, 1);
    b.w(m, 1, s2, 0);
    b.w(m, 4, s3, 0);
    b.done()
}

fn sync_motor() -> Model {
    let mut b = B::new(
        "Synchronous motor on the grid (3.3 kV)",
        "A 3.3 kV, 1 MVA, 4-pole wound-field salient-pole motor (typical values) runs at synchronous speed (157 rad/s) on a stiff supply with its field excited for rated voltage. At 1 s a 3000 N·m load is applied: the rotor falls back in angle, swings, and the damper windings settle it. The field voltage is Rf·if0, referred to the stator.",
        6.0,
    );
    let src = b.blk("ThreePhase", "Supply 3.3 kV", 20.0, 40.0, &[("Vll", "3300"), ("f", "50")]);
    let pr = crate::motors::presets("SyncField").swap_remove(0).values;
    let g = |k: &str| pr.iter().find(|(a, _)| a == k).map(|x| x.1.clone()).unwrap_or_default();
    let vf = format!("{}*{}", g("Rf"), g("if0"));
    let fv = b.blk("Constant", "Field voltage", 20.0, 120.0, &[("value", &vf)]);
    let tl = b.blk("Step", "Load torque", 20.0, 200.0, &[("time", "1"), ("final", "3000")]);
    let m = b.motor("SyncField", 0, "Motor 1 MVA", 200.0, 80.0, &[("w0", "2*pi*50/2"), ("th0", "-pi/4")]);
    let s1 = b.blk("Scope", "Speed", 400.0, 20.0, &[]);
    let s2 = b.blk("Scope", "Torque", 400.0, 100.0, &[]);
    let s3 = b.blk("Scope", "Stator currents", 400.0, 180.0, &[]);
    let s4 = b.blk("Scope", "Field current", 400.0, 260.0, &[]);
    b.w(src, 0, m, 0);
    b.w(fv, 0, m, 1);
    b.w(tl, 0, m, 2);
    b.w(m, 0, s1, 0);
    b.w(m, 1, s2, 0);
    b.w(m, 2, s3, 0);
    b.w(m, 3, s4, 0);
    b.done()
}

fn dc_series() -> Model {
    let mut b = B::new(
        "DC series motor: traction start",
        "A 600 V, 300 kW series motor (typical values) starts a heavy load; the inertia J includes the vehicle referred to the motor shaft. The supply voltage is ramped up over 10 s, as a chopper would, which keeps the current near its limit. The load is a constant rolling resistance plus drag ∝ speed².",
        150.0,
    );
    let v = b.blk("Ramp", "Voltage ramp", 20.0, 40.0, &[("slope", "60")]);
    let sat = b.blk("Saturation", "Up to 600 V", 110.0, 40.0, &[("upper", "600"), ("lower", "0")]);
    let load = b.blk("Fcn", "Train resistance", 200.0, 160.0, &[("expr", "300 + 0.02*u^2")]);
    b.flip(load);
    let m = b.motor("DCSeries", 1, "Motor 300 kW", 220.0, 40.0, &[("J", "400")]);
    let s1 = b.blk("Scope", "Speed", 420.0, 20.0, &[]);
    let s2 = b.blk("Scope", "Current", 420.0, 100.0, &[]);
    let s3 = b.blk("Scope", "Torque", 420.0, 180.0, &[("n", "2")]);
    b.w(v, 0, sat, 0);
    b.w(sat, 0, m, 0);
    b.w(load, 0, m, 1);
    b.w(m, 0, load, 0);
    b.w(m, 0, s1, 0);
    b.w(m, 1, s2, 0);
    b.w(m, 2, s3, 0);
    b.w(load, 0, s3, 1);
    b.done()
}

fn stepper() -> Model {
    let mut b = B::new(
        "Stepper motor: full steps",
        "A 1.8° NEMA 17 hybrid stepper driven with two-phase-on full steps: the phase voltages are square waves a quarter period apart, at 200 steps per second (one revolution per second). Each step moves the rotor 1.8° (π/100 rad), with a small overshoot.",
        0.1,
    );
    b.var("Vs", "1.65*1.7", "V", "Phase voltage for 1.7 A");
    b.var("fs", "200", "steps/s", "Step rate");
    let c = b.blk("Clock", "", 20.0, 60.0, &[]);
    let va = b.blk("Fcn", "Phase A", 100.0, 20.0, &[("expr", "Vs*if(cos(pi/2*fs*u + pi/4) >= 0, 1, -1)")]);
    let vb = b.blk("Fcn", "Phase B", 100.0, 100.0, &[("expr", "Vs*if(sin(pi/2*fs*u + pi/4) >= 0, 1, -1)")]);
    let tl = b.blk("Constant", "Load torque", 100.0, 180.0, &[("value", "0")]);
    let m = b.motor("Stepper", 0, "Stepper NEMA 17", 260.0, 60.0, &[]);
    let s1 = b.blk("Scope", "Angle", 440.0, 20.0, &[]);
    let s2 = b.blk("Scope", "Phase currents", 440.0, 100.0, &[("n", "2")]);
    b.w(c, 0, va, 0);
    b.w(c, 0, vb, 0);
    b.w(va, 0, m, 0);
    b.w(vb, 0, m, 1);
    b.w(tl, 0, m, 2);
    b.w(m, 2, s1, 0);
    b.w(m, 3, s2, 0);
    b.w(m, 4, s2, 1);
    b.done()
}

fn pmsm_digital() -> Model {
    let mut b = B::new(
        "PMSM: digital drive with a DC link",
        "The 15 kW interior-magnet motor of the field-oriented control example, now with discrete controllers: the current loops run at 10 kHz and the speed loop at 1 kHz (see View › Sample-time colours). Goto/From tags carry the electrical angle and the phase currents without long wires. The inverter draws its current from a DC link charged by a diode rectifier: when the speed reference falls to zero at 0.3 s the motor brakes, returns energy and the link voltage rises.",
        0.5,
    );
    b.var("pp", "4", "", "Pole pairs");
    b.var("Jm", "0.03", "kg·m²", "Inertia");
    b.var("Kt", "1.5*pp*0.4", "N·m/A", "Torque constant 1.5·p·ψ");
    b.var("wc", "2000", "rad/s", "Current loop bandwidth");
    b.var("ws", "100", "rad/s", "Speed loop bandwidth");
    b.var("Tc", "1e-4", "s", "Current loop sample time (10 kHz)");
    b.var("Tw", "1e-3", "s", "Speed loop sample time (1 kHz)");
    b.var("Vdc", "600", "V", "Rated DC link voltage");
    let wr = b.blk("Step", "Speed up", 20.0, 40.0, &[("time", "0.01"), ("final", "150")]);
    let wd = b.blk("Step", "Speed down", 20.0, 110.0, &[("time", "0.3"), ("final", "-150")]);
    let wsum = b.blk("Sum", "Speed reference", 110.0, 60.0, &[("signs", "++")]);
    let ew = b.blk("Sum", "", 170.0, 62.0, &[("signs", "+-")]);
    let pw = b.blk("DiscretePID", "Speed PI 1 kHz", 230.0, 58.0, &[("P", "Jm*ws/Kt"), ("I", "Jm*ws/Kt*ws/4"), ("ts", "Tw"), ("upper", "40"), ("lower", "-40")]);
    let idr = b.blk("Constant", "i_d reference", 230.0, 150.0, &[("value", "0")]);
    let eq = b.blk("Sum", "", 360.0, 62.0, &[("signs", "+-")]);
    let ed = b.blk("Sum", "", 360.0, 152.0, &[("signs", "+-")]);
    let pq = b.blk("DiscretePID", "i_q PI 10 kHz", 420.0, 58.0, &[("P", "0.004*wc"), ("I", "0.15*wc"), ("ts", "Tc"), ("upper", "Vdc/2"), ("lower", "-Vdc/2")]);
    let pd = b.blk("DiscretePID", "i_d PI 10 kHz", 420.0, 148.0, &[("P", "0.0025*wc"), ("I", "0.15*wc"), ("ts", "Tc"), ("upper", "Vdc/2"), ("lower", "-Vdc/2")]);
    let mx = b.blk("Mux", "", 540.0, 70.0, &[]);
    let ip = b.blk("InvPark", "", 580.0, 70.0, &[]);
    let fth = b.blk("From", "", 480.0, 230.0, &[("tag", "theta_e")]);
    let ic = b.blk("InvClarke", "", 670.0, 70.0, &[]);
    let gm = b.blk("Gain", "Modulation", 750.0, 70.0, &[("k", "2/Vdc")]);
    let inv = b.blk("Inverter", "Inverter", 850.0, 80.0, &[]);
    let tl = b.blk("Step", "Load torque", 850.0, 190.0, &[("time", "0.15"), ("final", "30")]);
    let m = b.motor("PMSM3", 1, "Motor 15 kW", 970.0, 80.0, &[]);
    let gp = b.blk("Gain", "p", 1100.0, 200.0, &[("k", "pp")]);
    let gth = b.blk("Goto", "", 1180.0, 200.0, &[("tag", "theta_e")]);
    let gi = b.blk("Goto", "", 1180.0, 260.0, &[("tag", "i_abc")]);
    let fi = b.blk("From", "", 600.0, 300.0, &[("tag", "i_abc")]);
    let fth2 = b.blk("From", "", 600.0, 360.0, &[("tag", "theta_e")]);
    let cl = b.blk("Clarke", "", 700.0, 300.0, &[]);
    let pk = b.blk("Park", "", 800.0, 310.0, &[]);
    let dm = b.blk("Demux", "i_dq", 900.0, 300.0, &[]);
    let gd = b.blk("Goto", "", 940.0, 290.0, &[("tag", "i_d")]);
    let gq = b.blk("Goto", "", 940.0, 340.0, &[("tag", "i_q")]);
    let fd = b.blk("From", "", 260.0, 220.0, &[("tag", "i_d")]);
    let fq = b.blk("From", "", 260.0, 260.0, &[("tag", "i_q")]);
    let vs = b.blk("Constant", "Mains, rectified", 600.0, 440.0, &[("value", "Vdc")]);
    let link = b.blk("DCLink", "DC link", 720.0, 430.0, &[("C", "0.002"), ("Rs", "0.05"), ("V0", "Vdc"), ("source", "diode rectifier (charges only)")]);
    let fi2 = b.blk("From", "", 480.0, 500.0, &[("tag", "i_abc")]);
    let idc = b.blk("InverterDC", "Inverter DC current", 560.0, 480.0, &[]);
    let s1 = b.blk("Scope", "Speed", 1180.0, 20.0, &[("n", "2")]);
    let s2 = b.blk("Scope", "i_d, i_q", 1180.0, 100.0, &[("n", "2")]);
    let s3 = b.blk("Scope", "DC link voltage", 900.0, 440.0, &[]);
    b.w(wr, 0, wsum, 0);
    b.w(wd, 0, wsum, 1);
    b.w(wsum, 0, ew, 0);
    b.w(ew, 0, pw, 0);
    b.w(pw, 0, eq, 0);
    b.w(idr, 0, ed, 0);
    b.w(eq, 0, pq, 0);
    b.w(ed, 0, pd, 0);
    b.w(fq, 0, eq, 1);
    b.w(fd, 0, ed, 1);
    b.w(pd, 0, mx, 0);
    b.w(pq, 0, mx, 1);
    b.w(mx, 0, ip, 0);
    b.w(fth, 0, ip, 1);
    b.w(ip, 0, ic, 0);
    b.w(ic, 0, gm, 0);
    b.w(gm, 0, inv, 0);
    b.w(link, 0, inv, 1);
    b.w(inv, 0, m, 0);
    b.w(tl, 0, m, 1);
    b.w(m, 3, gp, 0);
    b.w(gp, 0, gth, 0);
    b.w(m, 2, gi, 0);
    b.w(fi, 0, cl, 0);
    b.w(cl, 0, pk, 0);
    b.w(fth2, 0, pk, 1);
    b.w(pk, 0, dm, 0);
    b.w(dm, 0, gd, 0);
    b.w(dm, 1, gq, 0);
    b.w(m, 0, ew, 1);
    b.w(vs, 0, link, 0);
    b.w(gm, 0, idc, 0);
    b.w(fi2, 0, idc, 1);
    b.w(idc, 0, link, 1);
    b.w(m, 0, s1, 0);
    b.w(wsum, 0, s1, 1);
    b.w(dm, 0, s2, 0);
    b.w(dm, 1, s2, 1);
    b.w(link, 0, s3, 0);
    b.done()
}

fn dc_digital() -> Model {
    let mut b = B::new(
        "DC motor: digital and continuous PID",
        "Two copies of the DC motor speed loop. The upper one has the continuous PID of the DC motor example; the lower one a discrete PID sampled every 5 ms with the same gains. The digital loop follows closely but its voltage is a staircase, and a slower sample time (try Ts = 20 ms) makes it ring.",
        5.0,
    );
    motor_vars(&mut b);
    b.var("Ts", "0.005", "s", "Controller sample time");
    let r = b.blk("Step", "Speed reference", 20.0, 120.0, &[("time", "0"), ("final", "1")]);
    let tl = b.blk("Step", "Load torque", 20.0, 260.0, &[("time", "3"), ("final", "0.05")]);
    let mut speeds = vec![];
    let mut volts = vec![];
    for (k, digital) in [false, true].iter().enumerate() {
        let y = 40.0 + 160.0 * k as f64;
        let e = b.blk("Sum", "", 140.0, y + 2.0, &[("signs", "+-")]);
        let c = if *digital {
            b.blk("DiscretePID", "Discrete PID", 200.0, y, &[("P", "100"), ("I", "200"), ("D", "10"), ("ts", "Ts"), ("upper", "24"), ("lower", "-24")])
        } else {
            b.blk("PID", "Continuous PID", 200.0, y, &[("P", "100"), ("I", "200"), ("D", "10"), ("upper", "24"), ("lower", "-24")])
        };
        let m = b.blk("DCMotor", if *digital { "Motor (digital)" } else { "Motor (continuous)" }, 340.0, y, &[("R", "R"), ("L", "L"), ("Ke", "Ke"), ("Kt", "Kt"), ("J", "J"), ("b", "b")]);
        b.w(r, 0, e, 0);
        b.w(e, 0, c, 0);
        b.w(c, 0, m, 0);
        b.w(tl, 0, m, 1);
        b.w(m, 0, e, 1);
        speeds.push(m);
        volts.push(c);
    }
    let s1 = b.blk("Scope", "Speed", 520.0, 40.0, &[("n", "3")]);
    let s2 = b.blk("Scope", "Voltage", 520.0, 180.0, &[("n", "2")]);
    b.w(speeds[0], 0, s1, 0);
    b.w(speeds[1], 0, s1, 1);
    b.w(r, 0, s1, 2);
    b.w(volts[0], 0, s2, 0);
    b.w(volts[1], 0, s2, 1);
    b.done()
}

fn srm() -> Model {
    let mut b = B::new(
        "Switched reluctance motor speed control",
        "A 400 V, 4 kW 6/4 switched reluctance motor (typical values) on a 540 V asymmetric bridge. Each phase is switched on while its inductance rises (electrical angle Nr·θ − k·120° between θon and θoff) at a voltage set by the speed PI, and off (−540 V, the diodes return the energy) for the rest of the turn. Speed reference 150 rad/s; a 10 N·m load at 0.6 s.",
        1.5,
    );
    b.var("Vdc", "540", "V", "DC link voltage");
    b.var("thon", "0.15*pi", "rad", "Turn-on angle (electrical)");
    b.var("thoff", "0.75*pi", "rad", "Turn-off angle (electrical)");
    let wr = b.blk("Step", "Speed reference", 20.0, 40.0, &[("time", "0"), ("final", "150")]);
    let e = b.blk("Sum", "", 120.0, 42.0, &[("signs", "+-")]);
    let c = b.blk("PID", "Speed PI", 180.0, 38.0, &[("P", "10"), ("I", "60"), ("upper", "Vdc"), ("lower", "0")]);
    let m = b.motor("SRM", 0, "SRM 4 kW 6/4", 620.0, 60.0, &[]);
    let mut ph = vec![];
    for k in 0..3 {
        let a = format!("mod(4*u1 - {k}*2*pi/3, 2*pi)");
        let expr = format!("if({a} >= thon, if({a} < thoff, u2, -Vdc), -Vdc)");
        let f = b.blk("Fcn", &format!("Phase {}", ["a", "b", "c"][k]), 380.0, 20.0 + 70.0 * k as f64, &[("expr", &expr), ("n", "2")]);
        ph.push(f);
    }
    let mx = b.blk("Mux", "", 540.0, 50.0, &[("n", "3")]);
    let tl = b.blk("Step", "Load torque", 540.0, 200.0, &[("time", "0.6"), ("final", "10")]);
    let gth = b.blk("Goto", "", 800.0, 160.0, &[("tag", "theta")]);
    let fth = b.blk("From", "", 260.0, 160.0, &[("tag", "theta")]);
    let s1 = b.blk("Scope", "Speed", 800.0, 20.0, &[("n", "2")]);
    let s2 = b.blk("Scope", "Torque", 800.0, 240.0, &[]);
    let s3 = b.blk("Scope", "Phase currents", 800.0, 320.0, &[]);
    b.w(wr, 0, e, 0);
    b.w(e, 0, c, 0);
    for (k, f) in ph.iter().enumerate() {
        b.w(fth, 0, *f, 0);
        b.w(c, 0, *f, 1);
        b.w(*f, 0, mx, k);
    }
    b.w(mx, 0, m, 0);
    b.w(tl, 0, m, 1);
    b.w(m, 0, e, 1);
    b.w(m, 3, gth, 0);
    b.w(m, 0, s1, 0);
    b.w(wr, 0, s1, 1);
    b.w(m, 1, s2, 0);
    b.w(m, 2, s3, 0);
    b.done()
}
