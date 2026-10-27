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
