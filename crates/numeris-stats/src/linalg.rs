//! # Dense linear algebra
//!
//! Row-major `f64` matrices with Householder QR, back-substitution solves,
//! `(X'X)^{-1}` recovery, and a Jacobi eigensolver for symmetric matrices.
//! All implementations are independent, deterministic and dependency-free.

use numeris_core::error::{NumerisError, Result};
use serde::{Deserialize, Serialize};

/// Dense row-major matrix.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Matrix {
    pub rows: usize,
    pub cols: usize,
    pub data: Vec<f64>,
}

impl Matrix {
    pub fn zeros(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            data: vec![0.0; rows * cols],
        }
    }

    pub fn identity(n: usize) -> Self {
        let mut m = Self::zeros(n, n);
        for i in 0..n {
            m.data[i * n + i] = 1.0;
        }
        m
    }

    /// Build from row vectors (each inner Vec must have equal length).
    pub fn from_rows(rows: &[Vec<f64>]) -> Result<Self> {
        if rows.is_empty() {
            return Err(NumerisError::insufficient_data(
                "Cannot build a design matrix with no rows.",
                "The estimation sample is empty after applying listwise deletion.",
                "Check missing values in the model variables and any subset condition.",
            ));
        }
        let cols = rows[0].len();
        for r in rows {
            if r.len() != cols {
                return Err(NumerisError::numerical_failure(
                    "Cannot build a design matrix with ragged rows.",
                    "Internal error: rows of unequal length were passed to the linear algebra layer.",
                    "Please report this as a bug with the command that triggered it.",
                ));
            }
        }
        Ok(Self {
            rows: rows.len(),
            cols,
            data: rows.iter().flatten().copied().collect(),
        })
    }

    #[inline]
    pub fn get(&self, i: usize, j: usize) -> f64 {
        self.data[i * self.cols + j]
    }

    #[inline]
    pub fn set(&mut self, i: usize, j: usize, v: f64) {
        self.data[i * self.cols + j] = v;
    }

    pub fn row(&self, i: usize) -> &[f64] {
        &self.data[i * self.cols..(i + 1) * self.cols]
    }

    /// Transpose.
    pub fn t(&self) -> Matrix {
        let mut out = Self::zeros(self.cols, self.rows);
        for i in 0..self.rows {
            for j in 0..self.cols {
                out.data[j * self.rows + i] = self.get(i, j);
            }
        }
        out
    }

    /// Matrix product A·B.
    pub fn mul(&self, b: &Matrix) -> Result<Matrix> {
        if self.cols != b.rows {
            return Err(NumerisError::numerical_failure(
                "Matrix dimensions do not match for multiplication.",
                "Internal error: incompatible matrices reached the linear algebra layer.",
                "Please report this as a bug with the command that triggered it.",
            ));
        }
        let mut out = Matrix::zeros(self.rows, b.cols);
        for i in 0..self.rows {
            for k in 0..self.cols {
                let a = self.get(i, k);
                if a == 0.0 {
                    continue;
                }
                for j in 0..b.cols {
                    let idx = i * b.cols + j;
                    out.data[idx] += a * b.get(k, j);
                }
            }
        }
        Ok(out)
    }

    /// A · v.
    pub fn mul_vec(&self, v: &[f64]) -> Vec<f64> {
        let mut out = vec![0.0; self.rows];
        for i in 0..self.rows {
            let row = self.row(i);
            out[i] = row.iter().zip(v).map(|(a, b)| a * b).sum();
        }
        out
    }

    /// A' · v.
    pub fn t_mul_vec(&self, v: &[f64]) -> Vec<f64> {
        let mut out = vec![0.0; self.cols];
        for i in 0..self.rows {
            let vi = v[i];
            let row = self.row(i);
            for (j, &x) in row.iter().enumerate() {
                out[j] += x * vi;
            }
        }
        out
    }

    /// Symmetric square of a matrix: A' A (k×k when A is n×k).
    pub fn cross_product_self(&self) -> Matrix {
        let k = self.cols;
        let mut out = Matrix::zeros(k, k);
        for i in 0..self.rows {
            let row = self.row(i);
            for a in 0..k {
                if row[a] == 0.0 {
                    continue;
                }
                for b in a..k {
                    out.data[a * k + b] += row[a] * row[b];
                }
            }
        }
        for a in 0..k {
            for b in (a + 1)..k {
                out.data[b * k + a] = out.data[a * k + b];
            }
        }
        out
    }

    /// Multiply every element by a scalar.
    pub fn scale_mut(&mut self, s: f64) {
        for v in self.data.iter_mut() {
            *v *= s;
        }
    }

    /// Diagonal as a vector.
    pub fn diag(&self) -> Vec<f64> {
        (0..self.rows.min(self.cols))
            .map(|i| self.get(i, i))
            .collect()
    }

    /// Spectral condition number of a symmetric positive-definite matrix
    /// (ratio of largest to smallest eigenvalue).
    pub fn condition_number_symmetric(&self) -> f64 {
        let eigs = jacobi_eigenvalues(self);
        let max = eigs.iter().cloned().fold(f64::MIN, f64::max);
        let min = eigs.iter().cloned().fold(f64::MAX, f64::min);
        if min <= 0.0 || max <= 0.0 {
            f64::INFINITY
        } else {
            max / min
        }
    }
}

/// Householder QR decomposition of an n×k matrix (n ≥ k).
#[derive(Debug, Clone)]
pub struct QR {
    pub n: usize,
    pub k: usize,
    /// Reflector vectors v_j of length n-j (unit norm), applied in order.
    reflectors: Vec<Vec<f64>>,
    /// Upper triangular R stored row-major k×k.
    pub r: Vec<f64>,
    /// Diagonal of R for rank detection.
    pub r_diag_abs_max: f64,
}

impl QR {
    /// Decompose `a`. Columns are modified in place conceptually; `a` is
    /// consumed as scratch space by copy.
    pub fn decompose(a: &Matrix) -> Result<QR> {
        let n = a.rows;
        let k = a.cols;
        if n < k {
            return Err(NumerisError::insufficient_data(
                "The model has more parameters than observations.",
                format!("The estimation sample has {n} rows but the design matrix has {k} columns."),
                "Use fewer predictors, a larger sample, or check that the model is specified correctly.",
            ));
        }
        let mut work = a.data.clone();
        let mut reflectors: Vec<Vec<f64>> = Vec::with_capacity(k);
        let mut r = vec![0.0; k * k];

        for j in 0..k {
            // Column j below the diagonal (including the diagonal).
            let mut x = vec![0.0; n - j];
            for (i, xi) in x.iter_mut().enumerate() {
                *xi = work[(i + j) * k + j];
            }
            // Householder vector: v = x + sign(x0)||x|| e1, then normalize.
            let norm_x = x.iter().map(|v| v * v).sum::<f64>().sqrt();
            let alpha = if x[0] >= 0.0 { norm_x } else { -norm_x };
            let mut v = x;
            v[0] += alpha;
            let vnorm = v.iter().map(|z| z * z).sum::<f64>().sqrt();
            if vnorm > 0.0 {
                for z in v.iter_mut() {
                    *z /= vnorm;
                }
            }
            reflectors.push(v);

            // Apply H_j = I - 2 v v' to the remaining submatrix [j..n) x [j..k).
            for col in j..k {
                let mut dot = 0.0;
                for i in j..n {
                    dot += reflectors[j][i - j] * work[i * k + col];
                }
                let s = 2.0 * dot;
                for i in j..n {
                    let idx = i * k + col;
                    work[idx] -= s * reflectors[j][i - j];
                }
            }
        }

        // Extract R (upper triangular k×k) from the scratch matrix.
        let mut r_diag_abs_max = 0.0f64;
        for i in 0..k {
            for j in i..k {
                let val = work[i * k + j];
                r[i * k + j] = val;
            }
            let d = r[i * k + i].abs();
            if d > r_diag_abs_max {
                r_diag_abs_max = d;
            }
        }

        Ok(QR {
            n,
            k,
            reflectors,
            r,
            r_diag_abs_max,
        })
    }

    /// Rank check based on the diagonal of R. Returns the indices of
    /// numerically-zero pivots.
    pub fn rank_deficient_columns(&self) -> Vec<usize> {
        let tol = self.r_diag_abs_max * self.k as f64 * f64::EPSILON * 4.0;
        (0..self.k)
            .filter(|&i| self.r[i * self.k + i].abs() <= tol || !self.r[i * self.k + i].is_finite())
            .collect()
    }

    /// Compute Q^T y by applying the reflectors in order.
    pub fn qty(&self, y: &[f64]) -> Vec<f64> {
        let mut v = y.to_vec();
        for j in 0..self.k {
            let refl = &self.reflectors[j];
            let mut dot = 0.0;
            for i in j..self.n {
                dot += refl[i - j] * v[i];
            }
            let s = 2.0 * dot;
            for i in j..self.n {
                v[i] -= s * refl[i - j];
            }
        }
        v
    }

    /// Solve R x = b (b of length k) by back substitution.
    pub fn back_substitute(&self, b: &[f64]) -> Vec<f64> {
        let k = self.k;
        let mut x = vec![0.0; k];
        for i in (0..k).rev() {
            let mut s = b[i];
            for j in (i + 1)..k {
                s -= self.r[i * k + j] * x[j];
            }
            x[i] = s / self.r[i * k + i];
        }
        x
    }

    /// Solve least squares: β = R^{-1} Q^T y.
    pub fn solve(&self, y: &[f64]) -> Vec<f64> {
        let qty = self.qty(y);
        let b: Vec<f64> = (0..self.k).map(|i| qty[i]).collect();
        self.back_substitute(&b)
    }

    /// R^{-1} (k×k) via back substitution against identity columns.
    pub fn r_inv(&self) -> Matrix {
        let k = self.k;
        let mut inv = Matrix::zeros(k, k);
        for j in 0..k {
            let mut e = vec![0.0; k];
            e[j] = 1.0;
            let col = self.back_substitute(&e);
            for i in 0..k {
                inv.data[i * k + j] = col[i];
            }
        }
        inv
    }

    /// (X'X)^{-1} = R^{-1} R^{-T}.
    pub fn xtx_inv(&self) -> Matrix {
        let ri = self.r_inv();
        let k = self.k;
        // (R^{-1}) * (R^{-1})'
        let mut out = Matrix::zeros(k, k);
        for i in 0..k {
            for j in 0..k {
                let mut s = 0.0;
                for l in 0..k {
                    s += ri.get(i, l) * ri.get(j, l);
                }
                out.data[i * k + j] = s;
            }
        }
        out
    }
}

/// Jacobi eigenvalue iteration for a symmetric matrix. Returns eigenvalues
/// (unsorted). Deterministic sweep order makes results reproducible.
pub fn jacobi_eigenvalues(a: &Matrix) -> Vec<f64> {
    let n = a.rows;
    let mut m = a.clone();
    let mut off;
    for _ in 0..100 {
        off = 0.0;
        for i in 0..n {
            for j in (i + 1)..n {
                off += m.get(i, j).abs();
            }
        }
        if off < 1e-14 * (1.0 + m.data.iter().fold(0.0f64, |acc, v| acc + v.abs())) {
            break;
        }
        for p in 0..n {
            for q in (p + 1)..n {
                let apq = m.get(p, q);
                if apq.abs() < 1e-300 {
                    continue;
                }
                let app = m.get(p, p);
                let aqq = m.get(q, q);
                let theta = (aqq - app) / (2.0 * apq);
                let t = theta.signum() / (theta.abs() + (theta * theta + 1.0).sqrt());
                let c = 1.0 / (t * t + 1.0).sqrt();
                let s = t * c;
                // Apply rotation to rows/columns p and q.
                for i in 0..n {
                    let mip = m.get(i, p);
                    let miq = m.get(i, q);
                    m.set(i, p, c * mip - s * miq);
                    m.set(i, q, s * mip + c * miq);
                }
                for i in 0..n {
                    let mpi = m.get(p, i);
                    let mqi = m.get(q, i);
                    m.set(p, i, c * mpi - s * mqi);
                    m.set(q, i, s * mpi + c * mqi);
                }
            }
        }
    }
    m.diag()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qr_solves_well_conditioned_system() {
        // y = 1 + 2x (exact fit).
        let rows: Vec<Vec<f64>> = (0..10)
            .map(|i| {
                let x = i as f64;
                vec![1.0, x]
            })
            .collect();
        let x = Matrix::from_rows(&rows).unwrap();
        let y: Vec<f64> = (0..10).map(|i| 1.0 + 2.0 * i as f64).collect();
        let qr = QR::decompose(&x).unwrap();
        let beta = qr.solve(&y);
        assert!((beta[0] - 1.0).abs() < 1e-12);
        assert!((beta[1] - 2.0).abs() < 1e-12);
        assert!(qr.rank_deficient_columns().is_empty());
    }

    #[test]
    fn qr_detects_exact_collinearity() {
        let rows: Vec<Vec<f64>> = (0..8)
            .map(|i| {
                let x = i as f64;
                vec![1.0, x, 2.0 * x + 1.0] // third column is exact combo
            })
            .collect();
        let x = Matrix::from_rows(&rows).unwrap();
        let qr = QR::decompose(&x).unwrap();
        assert!(!qr.rank_deficient_columns().is_empty());
    }

    #[test]
    fn xtx_inverse_matches_direct_inverse_identity() {
        let rows: Vec<Vec<f64>> = vec![
            vec![1.0, 2.0],
            vec![1.0, 3.0],
            vec![1.0, 5.0],
            vec![1.0, 7.0],
        ];
        let x = Matrix::from_rows(&rows).unwrap();
        let qr = QR::decompose(&x).unwrap();
        let inv = qr.xtx_inv();
        let xtx = x.cross_product_self();
        let product = inv.mul(&xtx).unwrap();
        for i in 0..2 {
            for j in 0..2 {
                let expected = if i == j { 1.0 } else { 0.0 };
                assert!((product.get(i, j) - expected).abs() < 1e-12);
            }
        }
    }

    #[test]
    fn jacobi_eigenvalues_of_diagonal_matrix() {
        let mut m = Matrix::zeros(3, 3);
        m.set(0, 0, 4.0);
        m.set(1, 1, 9.0);
        m.set(2, 2, 1.0);
        let mut eigs = jacobi_eigenvalues(&m);
        eigs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert!((eigs[0] - 1.0).abs() < 1e-10);
        assert!((eigs[1] - 4.0).abs() < 1e-10);
        assert!((eigs[2] - 9.0).abs() < 1e-10);
    }

    #[test]
    fn cross_product_matches_manual() {
        let rows = vec![vec![1.0, 2.0], vec![3.0, 4.0], vec![5.0, 6.0]];
        let x = Matrix::from_rows(&rows).unwrap();
        let cp = x.cross_product_self();
        assert_eq!(cp.get(0, 0), 35.0);
        assert_eq!(cp.get(0, 1), 44.0);
        assert_eq!(cp.get(1, 1), 56.0);
    }
}
