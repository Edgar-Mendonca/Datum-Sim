fn main() {
    let id = std::env::args().nth(1).unwrap();
    let m = datum_core::examples::get(&id).unwrap();
    let r = datum_core::compile::simulate(&m, None, &Default::default()).unwrap();
    let n = r.t.len();
    for i in (0..n).step_by((n / 25).max(1)) {
        let row: Vec<String> = r.y.iter().map(|y| format!("{:9.3}", y[i])).collect();
        println!("{:.4} {}", r.t[i], row.join(" "));
    }
    println!("{:?}", r.stats);
}
