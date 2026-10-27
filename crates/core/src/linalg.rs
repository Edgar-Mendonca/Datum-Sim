//! Small dense linear algebra: LU with partial pivoting (for the stiff solver).

pub struct Lu {
    a: Vec<f64>,
    p: Vec<usize>,
    n: usize,
}

impl Lu {
    /// Factorise an n×n row-major matrix; None if it is singular.
    pub fn new(mut a: Vec<f64>, n: usize) -> Option<Lu> {
        let mut p: Vec<usize> = (0..n).collect();
        for k in 0..n {
            let (mut piv, mut big) = (k, a[k * n + k].abs());
            for i in k + 1..n {
                let v = a[i * n + k].abs();
                if v > big {
                    big = v;
                    piv = i;
                }
            }
            if big == 0.0 || !big.is_finite() {
                return None;
            }
            if piv != k {
                for j in 0..n {
                    a.swap(k * n + j, piv * n + j);
                }
                p.swap(k, piv);
            }
            let d = a[k * n + k];
            for i in k + 1..n {
                let f = a[i * n + k] / d;
                a[i * n + k] = f;
                if f != 0.0 {
                    for j in k + 1..n {
                        a[i * n + j] -= f * a[k * n + j];
                    }
                }
            }
        }
        Some(Lu { a, p, n })
    }

    /// Solve A x = b in place.
    pub fn solve(&self, b: &mut [f64]) {
        let n = self.n;
        let mut y: Vec<f64> = self.p.iter().map(|&i| b[i]).collect();
        for i in 0..n {
            for j in 0..i {
                y[i] -= self.a[i * n + j] * y[j];
            }
        }
        for i in (0..n).rev() {
            for j in i + 1..n {
                y[i] -= self.a[i * n + j] * y[j];
            }
            y[i] /= self.a[i * n + i];
        }
        b.copy_from_slice(&y);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn solves() {
        let lu = super::Lu::new(vec![0.0, 2.0, 1.0, 1.0, 1.0, 0.0, 3.0, 0.0, 1.0], 3).unwrap();
        let mut b = vec![5.0, 3.0, 6.0]; // x = [1, 2, 1]... check: [0+4+1, 1+2+0, 3+0+1] = [5,3,4]
        b[2] = 4.0;
        lu.solve(&mut b);
        for (v, w) in b.iter().zip([1.0, 2.0, 1.0]) {
            assert!((v - w).abs() < 1e-12);
        }
        assert!(super::Lu::new(vec![1.0, 2.0, 2.0, 4.0], 2).is_none());
    }
}
