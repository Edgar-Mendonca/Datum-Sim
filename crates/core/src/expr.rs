//! Expressions in block parameters and the Function block: numbers, variables, + - * / ^,
//! comparisons, functions, and matrices written `[1 2; 3 4]`. Our own parser (no AGPL crates).
//! Values are matrices; a scalar is 1×1 and a vector is 1×n.

use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct Mat {
    pub r: usize,
    pub c: usize,
    pub d: Vec<f64>,
}

impl Mat {
    pub fn scalar(v: f64) -> Mat {
        Mat { r: 1, c: 1, d: vec![v] }
    }
    pub fn row(d: Vec<f64>) -> Mat {
        Mat { r: 1, c: d.len(), d }
    }
    pub fn len(&self) -> usize {
        self.d.len()
    }
    pub fn is_empty(&self) -> bool {
        self.d.is_empty()
    }
    pub fn is_scalar(&self) -> bool {
        self.d.len() == 1
    }
    pub fn at(&self, i: usize, j: usize) -> f64 {
        self.d[i * self.c + j]
    }
    /// The value as a scalar, if it is one.
    pub fn s(&self) -> Result<f64, String> {
        if self.is_scalar() {
            Ok(self.d[0])
        } else {
            Err(format!("expected a single number, got {}×{}", self.r, self.c))
        }
    }
}

impl fmt::Display for Mat {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.is_scalar() {
            return write!(f, "{}", self.d[0]);
        }
        write!(f, "[")?;
        for i in 0..self.r {
            if i > 0 {
                write!(f, "; ")?;
            }
            for j in 0..self.c {
                if j > 0 {
                    write!(f, " ")?;
                }
                write!(f, "{}", self.at(i, j))?;
            }
        }
        write!(f, "]")
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    Num(f64),
    Var(String),
    /// A variable resolved to a slot (fast evaluation in the Function block).
    Slot(usize),
    Neg(Box<Node>),
    Bin(char, Box<Node>, Box<Node>),
    Cmp(&'static str, Box<Node>, Box<Node>),
    Call(String, Vec<Node>),
    Matrix(Vec<Vec<Node>>),
}

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Num(f64),
    Id(String),
    Op(&'static str),
}

fn lex(s: &str) -> Result<Vec<(Tok, bool)>, String> {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    let mut space = false;
    while i < b.len() {
        let ch = b[i] as char;
        if ch.is_whitespace() {
            space = true;
            i += 1;
            continue;
        }
        if ch.is_ascii_digit() || (ch == '.' && i + 1 < b.len() && (b[i + 1] as char).is_ascii_digit()) {
            let st = i;
            while i < b.len() && ((b[i] as char).is_ascii_digit() || b[i] == b'.') {
                i += 1;
            }
            if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
                let mut j = i + 1;
                if j < b.len() && (b[j] == b'+' || b[j] == b'-') {
                    j += 1;
                }
                if j < b.len() && (b[j] as char).is_ascii_digit() {
                    i = j;
                    while i < b.len() && (b[i] as char).is_ascii_digit() {
                        i += 1;
                    }
                }
            }
            let v: f64 = s[st..i].parse().map_err(|_| format!("bad number '{}'", &s[st..i]))?;
            out.push((Tok::Num(v), space));
        } else if ch.is_alphabetic() || ch == '_' {
            let st = i;
            while i < b.len() && ((b[i] as char).is_alphanumeric() || b[i] == b'_') {
                i += 1;
            }
            out.push((Tok::Id(s[st..i].to_string()), space));
        } else {
            let two = if i + 1 < b.len() { &s[i..i + 2] } else { "" };
            let op: &'static str = match two {
                "<=" => "<=",
                ">=" => ">=",
                "==" => "==",
                "!=" => "!=",
                "**" => "^",
                _ => match ch {
                    '+' => "+",
                    '-' => "-",
                    '*' => "*",
                    '/' => "/",
                    '^' => "^",
                    '(' => "(",
                    ')' => ")",
                    '[' => "[",
                    ']' => "]",
                    ',' => ",",
                    ';' => ";",
                    '<' => "<",
                    '>' => ">",
                    _ => return Err(format!("unexpected '{}'", ch)),
                },
            };
            i += if op.len() == 2 || two == "**" { 2 } else { 1 };
            out.push((Tok::Op(op), space));
        }
        space = false;
    }
    Ok(out)
}

struct P {
    t: Vec<(Tok, bool)>,
    i: usize,
    depth: usize,
    in_matrix: bool,
}

impl P {
    fn peek(&self) -> Option<&Tok> {
        self.t.get(self.i).map(|x| &x.0)
    }
    fn is(&self, op: &str) -> bool {
        matches!(self.peek(), Some(Tok::Op(o)) if *o == op)
    }
    fn eat(&mut self, op: &str) -> bool {
        if self.is(op) {
            self.i += 1;
            true
        } else {
            false
        }
    }
    fn expect(&mut self, op: &str) -> Result<(), String> {
        if self.eat(op) {
            Ok(())
        } else {
            Err(format!("expected '{}'", op))
        }
    }
    fn expr(&mut self) -> Result<Node, String> {
        self.depth += 1;
        if self.depth > 200 {
            return Err("expression is nested too deeply".into());
        }
        let a = self.add()?;
        for op in ["<=", ">=", "==", "!=", "<", ">"] {
            if self.eat(op) {
                let b = self.add()?;
                self.depth -= 1;
                return Ok(Node::Cmp(op, Box::new(a), Box::new(b)));
            }
        }
        self.depth -= 1;
        Ok(a)
    }
    /// In a matrix, `[1 -2]` is two elements: a sign with a space before and none after.
    fn sep_in_matrix(&self) -> bool {
        self.in_matrix && matches!(self.t.get(self.i), Some((Tok::Op("+" | "-"), true)))
            && matches!(self.t.get(self.i + 1), Some((_, false)))
    }
    fn add(&mut self) -> Result<Node, String> {
        let mut a = self.mul()?;
        loop {
            if self.sep_in_matrix() {
                return Ok(a);
            }
            if self.eat("+") {
                a = Node::Bin('+', Box::new(a), Box::new(self.mul()?));
            } else if self.eat("-") {
                a = Node::Bin('-', Box::new(a), Box::new(self.mul()?));
            } else {
                return Ok(a);
            }
        }
    }
    fn mul(&mut self) -> Result<Node, String> {
        let mut a = self.unary()?;
        loop {
            if self.eat("*") {
                a = Node::Bin('*', Box::new(a), Box::new(self.unary()?));
            } else if self.eat("/") {
                a = Node::Bin('/', Box::new(a), Box::new(self.unary()?));
            } else {
                return Ok(a);
            }
        }
    }
    fn unary(&mut self) -> Result<Node, String> {
        if self.eat("-") {
            return Ok(Node::Neg(Box::new(self.unary()?)));
        }
        if self.eat("+") {
            return self.unary();
        }
        let a = self.atom()?;
        if self.eat("^") {
            // Right-associative; -2^2 = -4 as usual.
            return Ok(Node::Bin('^', Box::new(a), Box::new(self.unary()?)));
        }
        Ok(a)
    }
    fn atom(&mut self) -> Result<Node, String> {
        match self.peek().cloned() {
            Some(Tok::Num(v)) => {
                self.i += 1;
                Ok(Node::Num(v))
            }
            Some(Tok::Id(name)) => {
                self.i += 1;
                if self.eat("(") {
                    let saved = self.in_matrix;
                    self.in_matrix = false;
                    let mut args = Vec::new();
                    if !self.eat(")") {
                        loop {
                            args.push(self.expr()?);
                            if self.eat(")") {
                                break;
                            }
                            self.expect(",")?;
                        }
                    }
                    self.in_matrix = saved;
                    Ok(Node::Call(name, args))
                } else {
                    Ok(Node::Var(name))
                }
            }
            Some(Tok::Op("(")) => {
                self.i += 1;
                let saved = self.in_matrix;
                self.in_matrix = false;
                let e = self.expr()?;
                self.in_matrix = saved;
                self.expect(")")?;
                Ok(e)
            }
            Some(Tok::Op("[")) => {
                self.i += 1;
                let saved = self.in_matrix;
                self.in_matrix = true;
                let mut rows = vec![Vec::new()];
                loop {
                    if self.eat("]") {
                        break;
                    }
                    if self.eat(";") {
                        rows.push(Vec::new());
                        continue;
                    }
                    if self.eat(",") {
                        continue;
                    }
                    if self.peek().is_none() {
                        return Err("missing ']'".into());
                    }
                    let e = self.expr()?;
                    rows.last_mut().unwrap().push(e);
                }
                self.in_matrix = saved;
                if rows.last().is_some_and(|r| r.is_empty()) && rows.len() > 1 {
                    rows.pop();
                }
                Ok(Node::Matrix(rows))
            }
            Some(Tok::Op(o)) => Err(format!("unexpected '{}'", o)),
            None => Err("the expression ends too early".into()),
        }
    }
}

impl P {
    fn new(t: Vec<(Tok, bool)>) -> P {
        P { t, i: 0, depth: 0, in_matrix: false }
    }
}

pub fn parse(s: &str) -> Result<Node, String> {
    let t = lex(s)?;
    if t.is_empty() {
        return Err("empty expression".into());
    }
    let mut p = P::new(t);
    let e = p.expr()?;
    if p.i < p.t.len() {
        return Err(format!("unexpected text after position {}", p.i + 1));
    }
    Ok(e)
}

pub const FUNCS: &[&str] = &[
    "sin", "cos", "tan", "asin", "acos", "atan", "atan2", "sinh", "cosh", "tanh", "exp", "log", "ln", "log10",
    "log2", "sqrt", "abs", "sign", "floor", "ceil", "round", "min", "max", "pow", "hypot", "mod", "rem", "if",
    "sat", "deg", "rad", "zeros", "ones", "eye", "linspace",
];

fn f1(name: &str, x: f64) -> Option<f64> {
    Some(match name {
        "sin" => x.sin(),
        "cos" => x.cos(),
        "tan" => x.tan(),
        "asin" => x.asin(),
        "acos" => x.acos(),
        "atan" => x.atan(),
        "sinh" => x.sinh(),
        "cosh" => x.cosh(),
        "tanh" => x.tanh(),
        "exp" => x.exp(),
        "log" | "ln" => x.ln(),
        "log10" => x.log10(),
        "log2" => x.log2(),
        "sqrt" => x.sqrt(),
        "abs" => x.abs(),
        "sign" => {
            if x > 0.0 {
                1.0
            } else if x < 0.0 {
                -1.0
            } else {
                0.0
            }
        }
        "floor" => x.floor(),
        "ceil" => x.ceil(),
        "round" => x.round(),
        "deg" => x.to_degrees(),
        "rad" => x.to_radians(),
        _ => return None,
    })
}

fn f2(name: &str, a: f64, b: f64) -> Option<f64> {
    Some(match name {
        "atan2" => a.atan2(b),
        "min" => a.min(b),
        "max" => a.max(b),
        "pow" => a.powf(b),
        "hypot" => a.hypot(b),
        "mod" => a - b * (a / b).floor(),
        "rem" => a % b,
        _ => return None,
    })
}

fn constant(name: &str) -> Option<f64> {
    match name {
        "pi" => Some(std::f64::consts::PI),
        "e" => Some(std::f64::consts::E),
        "inf" | "Inf" => Some(f64::INFINITY),
        _ => None,
    }
}

fn map(a: Mat, f: impl Fn(f64) -> f64) -> Mat {
    Mat { r: a.r, c: a.c, d: a.d.into_iter().map(f).collect() }
}

fn zip(op: char, a: Mat, b: Mat, f: impl Fn(f64, f64) -> f64) -> Result<Mat, String> {
    if a.is_scalar() {
        let x = a.d[0];
        return Ok(map(b, |y| f(x, y)));
    }
    if b.is_scalar() {
        let y = b.d[0];
        return Ok(map(a, |x| f(x, y)));
    }
    if a.r != b.r || a.c != b.c {
        return Err(format!("sizes do not match for '{}': {}×{} and {}×{}", op, a.r, a.c, b.r, b.c));
    }
    Ok(Mat { r: a.r, c: a.c, d: a.d.iter().zip(&b.d).map(|(x, y)| f(*x, *y)).collect() })
}

impl Node {
    /// Evaluate with a variable lookup.
    pub fn eval(&self, env: &dyn Fn(&str) -> Option<Mat>) -> Result<Mat, String> {
        Ok(match self {
            Node::Num(v) => Mat::scalar(*v),
            Node::Var(n) => match env(n) {
                Some(m) => m,
                None => Mat::scalar(constant(n).ok_or_else(|| format!("unknown name '{}'", n))?),
            },
            Node::Slot(_) => return Err("internal: unresolved slot".into()),
            Node::Neg(a) => map(a.eval(env)?, |x| -x),
            Node::Bin(op, a, b) => {
                let (a, b) = (a.eval(env)?, b.eval(env)?);
                match op {
                    '+' => zip('+', a, b, |x, y| x + y)?,
                    '-' => zip('-', a, b, |x, y| x - y)?,
                    '/' => zip('/', a, b, |x, y| x / y)?,
                    '^' => zip('^', a, b, f64::powf)?,
                    _ => {
                        if a.is_scalar() || b.is_scalar() {
                            zip('*', a, b, |x, y| x * y)?
                        } else if a.c == b.r {
                            let mut d = vec![0.0; a.r * b.c];
                            for i in 0..a.r {
                                for j in 0..b.c {
                                    d[i * b.c + j] = (0..a.c).map(|k| a.at(i, k) * b.at(k, j)).sum();
                                }
                            }
                            Mat { r: a.r, c: b.c, d }
                        } else {
                            return Err(format!("cannot multiply {}×{} by {}×{}", a.r, a.c, b.r, b.c));
                        }
                    }
                }
            }
            Node::Cmp(op, a, b) => {
                let (a, b) = (a.eval(env)?, b.eval(env)?);
                let op = *op;
                zip('=', a, b, move |x, y| cmp(op, x, y))?
            }
            Node::Call(name, args) => {
                let v: Vec<Mat> = args.iter().map(|a| a.eval(env)).collect::<Result<_, _>>()?;
                call(name, v)?
            }
            Node::Matrix(rows) => {
                let mut out: Vec<Vec<f64>> = Vec::new();
                for row in rows {
                    let mut r = Vec::new();
                    for e in row {
                        let m = e.eval(env)?;
                        if m.r > 1 {
                            return Err("nested matrices with several rows are not supported".into());
                        }
                        r.extend(m.d);
                    }
                    out.push(r);
                }
                let c = out.first().map_or(0, |r| r.len());
                if out.iter().any(|r| r.len() != c) {
                    return Err("matrix rows have different lengths".into());
                }
                let r = if c == 0 { 0 } else { out.len() };
                Mat { r: r.max(if c == 0 { 1 } else { 0 }), c, d: out.concat() }
            }
        })
    }

    /// Replace named variables by slots (for fast scalar evaluation).
    pub fn bind(&mut self, names: &[&str]) -> Result<(), String> {
        match self {
            Node::Var(n) => {
                if let Some(i) = names.iter().position(|x| x == n) {
                    *self = Node::Slot(i);
                } else if let Some(v) = constant(n) {
                    *self = Node::Num(v);
                } else {
                    return Err(format!("unknown name '{}'", n));
                }
            }
            Node::Neg(a) => a.bind(names)?,
            Node::Bin(_, a, b) | Node::Cmp(_, a, b) => {
                a.bind(names)?;
                b.bind(names)?;
            }
            Node::Call(name, args) => {
                if !FUNCS.contains(&name.as_str()) {
                    return Err(format!("unknown function '{}'", name));
                }
                for a in args {
                    a.bind(names)?;
                }
            }
            Node::Matrix(_) => return Err("matrices are not allowed here".into()),
            Node::Num(_) | Node::Slot(_) => {}
        }
        Ok(())
    }

    /// Fast scalar evaluation after `bind`.
    pub fn eval_s(&self, s: &[f64]) -> f64 {
        match self {
            Node::Num(v) => *v,
            Node::Slot(i) => s[*i],
            Node::Var(_) | Node::Matrix(_) => f64::NAN,
            Node::Neg(a) => -a.eval_s(s),
            Node::Bin(op, a, b) => {
                let (x, y) = (a.eval_s(s), b.eval_s(s));
                match op {
                    '+' => x + y,
                    '-' => x - y,
                    '*' => x * y,
                    '/' => x / y,
                    _ => x.powf(y),
                }
            }
            Node::Cmp(op, a, b) => cmp(op, a.eval_s(s), b.eval_s(s)),
            Node::Call(name, args) => {
                let v: Vec<f64> = args.iter().map(|a| a.eval_s(s)).collect();
                match (name.as_str(), v.as_slice()) {
                    ("if", [c, a, b]) => {
                        if *c != 0.0 {
                            *a
                        } else {
                            *b
                        }
                    }
                    ("sat", [x, lo, hi]) => x.max(*lo).min(*hi),
                    ("min", _) if !v.is_empty() => v.iter().cloned().fold(f64::INFINITY, f64::min),
                    ("max", _) if !v.is_empty() => v.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
                    (n, [x]) => f1(n, *x).unwrap_or(f64::NAN),
                    (n, [x, y]) => f2(n, *x, *y).unwrap_or(f64::NAN),
                    _ => f64::NAN,
                }
            }
        }
    }
}

fn cmp(op: &str, x: f64, y: f64) -> f64 {
    let r = match op {
        "<" => x < y,
        ">" => x > y,
        "<=" => x <= y,
        ">=" => x >= y,
        "==" => x == y,
        _ => x != y,
    };
    if r {
        1.0
    } else {
        0.0
    }
}

fn call(name: &str, v: Vec<Mat>) -> Result<Mat, String> {
    let n = v.len();
    let sc = |i: usize| v[i].s();
    match (name, n) {
        ("if", 3) => return Ok(if sc(0)? != 0.0 { v[1].clone() } else { v[2].clone() }),
        ("sat", 3) => {
            let (lo, hi) = (sc(1)?, sc(2)?);
            return Ok(map(v[0].clone(), |x| x.max(lo).min(hi)));
        }
        ("zeros" | "ones", 1 | 2) => {
            let r = sc(0)? as usize;
            let c = if n == 2 { sc(1)? as usize } else { r };
            if r * c > 1_000_000 {
                return Err("matrix too large".into());
            }
            let x = if name == "ones" { 1.0 } else { 0.0 };
            return Ok(Mat { r, c, d: vec![x; r * c] });
        }
        ("eye", 1) => {
            let k = sc(0)? as usize;
            if k > 1000 {
                return Err("matrix too large".into());
            }
            let mut d = vec![0.0; k * k];
            for i in 0..k {
                d[i * k + i] = 1.0;
            }
            return Ok(Mat { r: k, c: k, d });
        }
        ("linspace", 3) => {
            let (a, b, k) = (sc(0)?, sc(1)?, sc(2)? as usize);
            if !(2..=1_000_000).contains(&k) {
                return Err("linspace needs 2 to 1e6 points".into());
            }
            return Ok(Mat::row((0..k).map(|i| a + (b - a) * i as f64 / (k - 1) as f64).collect()));
        }
        ("min" | "max", 3..) => {
            let s: Vec<f64> = (0..n).map(sc).collect::<Result<_, _>>()?;
            let it = s.into_iter();
            let r = if name == "min" { it.fold(f64::INFINITY, f64::min) } else { it.fold(f64::NEG_INFINITY, f64::max) };
            return Ok(Mat::scalar(r));
        }
        ("min" | "max", 1) => {
            let it = v[0].d.iter().cloned();
            let r = if name == "min" { it.fold(f64::INFINITY, f64::min) } else { it.fold(f64::NEG_INFINITY, f64::max) };
            return Ok(Mat::scalar(r));
        }
        _ => {}
    }
    if n == 1 {
        if f1(name, 0.0).is_some() {
            let a = v.into_iter().next().unwrap();
            return Ok(map(a, |x| f1(name, x).unwrap()));
        }
    } else if n == 2 && f2(name, 0.0, 0.0).is_some() {
        let mut it = v.into_iter();
        let (a, b) = (it.next().unwrap(), it.next().unwrap());
        return zip(',', a, b, |x, y| f2(name, x, y).unwrap());
    }
    if FUNCS.contains(&name) {
        Err(format!("{}() does not take {} argument{}", name, n, if n == 1 { "" } else { "s" }))
    } else {
        Err(format!("unknown function '{}'", name))
    }
}

/// Evaluate a text with a variable lookup.
pub fn eval(s: &str, env: &dyn Fn(&str) -> Option<Mat>) -> Result<Mat, String> {
    parse(s)?.eval(env)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn ev(s: &str) -> Mat {
        eval(s, &|n| if n == "k" { Some(Mat::scalar(2.0)) } else { None }).unwrap()
    }
    #[test]
    fn arithmetic() {
        assert_eq!(ev("1+2*3").d, vec![7.0]);
        assert_eq!(ev("-2^2").d, vec![-4.0]);
        assert_eq!(ev("2^3^2").d, vec![512.0]);
        assert_eq!(ev("(1+2)*k").d, vec![6.0]);
        assert_eq!(ev("1e-3*1E3").d, vec![1.0]);
        assert!((ev("sin(pi/2)").d[0] - 1.0).abs() < 1e-15);
        assert_eq!(ev("max(1, 3, 2)").d, vec![3.0]);
        assert_eq!(ev("3 > 2").d, vec![1.0]);
        assert_eq!(ev("if(0, 1, 5)").d, vec![5.0]);
    }
    #[test]
    fn matrices() {
        assert_eq!(ev("[1 2 3]"), Mat::row(vec![1.0, 2.0, 3.0]));
        assert_eq!(ev("[1 -2]"), Mat::row(vec![1.0, -2.0]));
        assert_eq!(ev("[1 - 2]"), Mat::scalar(-1.0));
        assert_eq!(ev("[1, k*2]").d, vec![1.0, 4.0]);
        let m = ev("[1 2; 3 4]");
        assert_eq!((m.r, m.c, m.d.clone()), (2, 2, vec![1.0, 2.0, 3.0, 4.0]));
        let p = ev("[1 2; 3 4] * [1; 1]");
        assert_eq!((p.r, p.c, p.d), (2, 1, vec![3.0, 7.0]));
        assert_eq!(ev("2*[1 2]").d, vec![2.0, 4.0]);
        assert_eq!(ev("[]").len(), 0);
        assert_eq!(ev("eye(2)").d, vec![1.0, 0.0, 0.0, 1.0]);
    }
    #[test]
    fn errors() {
        for bad in ["", "1+", "(1", "[1 2", "foo", "foo(1)", "[1 2; 3]", "sin(1,2)", "1 2", "$", "zeros(1e9)"] {
            assert!(eval(bad, &|_| None).is_err(), "{bad}");
        }
        let deep = "(".repeat(500) + "1" + &")".repeat(500);
        assert!(eval(&deep, &|_| None).is_err());
    }
    #[test]
    fn bound() {
        let mut e = parse("u1*sin(t) + if(u2 > 0, 1, -1)").unwrap();
        e.bind(&["t", "u1", "u2"]).unwrap();
        let v = e.eval_s(&[std::f64::consts::FRAC_PI_2, 3.0, -1.0]);
        assert!((v - 2.0).abs() < 1e-12);
        assert!(parse("x").unwrap().bind(&["t"]).is_err());
    }
}
