//! # Variance–covariance estimators
//!
//! Classical (Gauss–Markov), heteroskedasticity-robust (HC0–HC3), one-way
//! cluster-robust, and HAC (Newey–West with Bartlett weights).
//!
//! All estimators share the sandwich form
//! `V = A · meat · A` with `A = (X'X)^{-1}` and differ only in the meat.

use crate::linalg::Matrix;
use numeris_core::error::{NumerisError, Result};
use serde::{Deserialize, Serialize};

/// Variance estimator selection for a linear model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum VcovSpec {
    /// Classical homoskedastic covariance: σ² (X'X)^{-1}.
    Classical,
    /// HC0 sandwich without small-sample correction.
    Hc0,
    /// HC1 sandwich, HC0 scaled by n/(n-k). This is what the plain
    /// `robust` option selects (the applied-econometrics default).
    Hc1,
    /// HC2 sandwich with leverage adjustment u²/(1-h).
    Hc2,
    /// HC3 sandwich with squared leverage adjustment u²/(1-h)².
    Hc3,
    /// One-way cluster-robust with the standard finite-sample
    /// correction c = G/(G-1) · (n-1)/(n-k).
    Cluster,
    /// Newey–West HAC with Bartlett kernel and user-specified lag order.
    Hac { lags: usize },
}

impl VcovSpec {
    /// Human-readable label shown in results.
    pub fn label(&self) -> &'static str {
        match self {
            VcovSpec::Classical => "Classical",
            VcovSpec::Hc0 => "HC0 robust",
            VcovSpec::Hc1 => "HC1 robust",
            VcovSpec::Hc2 => "HC2 robust",
            VcovSpec::Hc3 => "HC3 robust",
            VcovSpec::Cluster => "Cluster-robust",
            VcovSpec::Hac { .. } => "Newey–West HAC",
        }
    }
}

/// Cluster identifiers as small integers (encoded upstream).
pub type ClusterId = u32;

/// Compute the covariance matrix of β̂.
///
/// * `x`          design matrix (n×k)
/// * `residuals`  u = y − Xβ̂ (length n)
/// * `xtx_inv`    (X'X)^{-1} (k×k)
/// * `leverage`   hat diagonal h_ii (length n)
/// * `clusters`   cluster id per row (required for `Cluster`)
pub fn vcov(
    x: &Matrix,
    residuals: &[f64],
    xtx_inv: &Matrix,
    leverage: &[f64],
    clusters: Option<&[ClusterId]>,
    spec: &VcovSpec,
) -> Result<Matrix> {
    let n = x.rows;
    let k = x.cols;
    if residuals.len() != n || leverage.len() != n {
        return Err(NumerisError::numerical_failure(
            "Covariance inputs have inconsistent lengths.",
            "Internal error: residual and leverage vectors do not match the design matrix.",
            "Please report this as a bug with the command that triggered it.",
        ));
    }
    match spec {
        VcovSpec::Classical => {
            let df_r = n.saturating_sub(k);
            if df_r == 0 {
                return Err(NumerisError::insufficient_data(
                    "The model has no residual degrees of freedom.",
                    format!("With {n} observations and {k} parameters there are no degrees of freedom left to estimate the error variance."),
                    "Use fewer predictors or a larger sample.",
                ));
            }
            let sigma2 = residuals.iter().map(|u| u * u).sum::<f64>() / df_r as f64;
            let mut v = xtx_inv.clone();
            v.scale_mut(sigma2);
            Ok(v)
        }
        VcovSpec::Hc0 | VcovSpec::Hc1 | VcovSpec::Hc2 | VcovSpec::Hc3 => {
            // meat = Σ w_i u_i² x_i x_i'
            let mut meat = Matrix::zeros(k, k);
            for i in 0..n {
                let h = leverage[i].clamp(0.0, 0.999_999);
                let w = match spec {
                    VcovSpec::Hc0 => 1.0,
                    VcovSpec::Hc1 => 1.0, // applied as a global factor below
                    VcovSpec::Hc2 => 1.0 / (1.0 - h),
                    VcovSpec::Hc3 => 1.0 / ((1.0 - h) * (1.0 - h)),
                    _ => unreachable!(),
                };
                let wu2 = w * residuals[i] * residuals[i];
                let row = x.row(i);
                for a in 0..k {
                    if row[a] == 0.0 {
                        continue;
                    }
                    for b in a..k {
                        let s = wu2 * row[a] * row[b];
                        meat.data[a * k + b] += s;
                    }
                }
            }
            for a in 0..k {
                for b in (a + 1)..k {
                    meat.data[b * k + a] = meat.data[a * k + b];
                }
            }
            if matches!(spec, VcovSpec::Hc1) {
                let f = n as f64 / (n - k) as f64;
                meat.scale_mut(f);
            }
            sandwich(xtx_inv, &meat)
        }
        VcovSpec::Cluster => {
            let clusters = clusters.ok_or_else(|| {
                NumerisError::invalid_request(
                    "Clustered standard errors require a cluster variable.",
                    "The cluster() option was not given a variable to cluster on.",
                    "Add the cluster option, for example: regress y x, cluster(firm).",
                )
            })?;
            if clusters.len() != n {
                return Err(NumerisError::numerical_failure(
                    "Cluster vector length does not match the estimation sample.",
                    "Internal error: cluster identifiers were not aligned with the design rows.",
                    "Please report this as a bug with the command that triggered it.",
                ));
            }
            let g = clusters
                .iter()
                .copied()
                .collect::<std::collections::BTreeSet<_>>()
                .len();
            if g < 2 {
                return Err(NumerisError::insufficient_data(
                    "Clustered standard errors require at least two clusters.",
                    "All estimation rows belong to a single cluster, so between-cluster variation cannot be estimated.",
                    "Choose a cluster variable with more groups, or use robust standard errors instead.",
                ));
            }
            let df_r = n.saturating_sub(k);
            if df_r == 0 {
                return Err(NumerisError::insufficient_data(
                    "The model has no residual degrees of freedom.",
                    format!("With {n} observations and {k} parameters there are no degrees of freedom left."),
                    "Use fewer predictors or a larger sample.",
                ));
            }
            // meat = Σ_g (X_g' u_g)(X_g' u_g)'
            let mut group_scores: std::collections::BTreeMap<ClusterId, Vec<f64>> =
                std::collections::BTreeMap::new();
            for i in 0..n {
                let row = x.row(i);
                let u = residuals[i];
                let score = group_scores
                    .entry(clusters[i])
                    .or_insert_with(|| vec![0.0; k]);
                for (a, &xv) in row.iter().enumerate() {
                    score[a] += xv * u;
                }
            }
            let mut meat = Matrix::zeros(k, k);
            for scores in group_scores.values() {
                for a in 0..k {
                    for b in a..k {
                        meat.data[a * k + b] += scores[a] * scores[b];
                    }
                }
            }
            for a in 0..k {
                for b in (a + 1)..k {
                    meat.data[b * k + a] = meat.data[a * k + b];
                }
            }
            // Standard small-sample correction.
            let c = g as f64 / (g as f64 - 1.0) * ((n as f64 - 1.0) / df_r as f64);
            meat.scale_mut(c);
            sandwich(xtx_inv, &meat)
        }
        VcovSpec::Hac { lags } => {
            if residuals.len() != n {
                return Err(NumerisError::numerical_failure(
                    "Residual vector length does not match the design matrix.",
                    "Internal error in HAC covariance assembly.",
                    "Please report this as a bug with the command that triggered it.",
                ));
            }
            let df_r = n.saturating_sub(k);
            if df_r == 0 {
                return Err(NumerisError::insufficient_data(
                    "The model has no residual degrees of freedom.",
                    format!("With {n} observations and {k} parameters there are no degrees of freedom left."),
                    "Use fewer predictors or a larger sample.",
                ));
            }
            let l = *lags;
            if l + 1 >= n {
                return Err(NumerisError::invalid_request(
                    format!("HAC lag order {l} is too large for {n} observations."),
                    "The Bartlett kernel requires the lag order to be smaller than the sample size.",
                    "Choose a smaller lag order, for example floor(n^(1/4)) or T/10 as a starting point.",
                ));
            }
            // Γ_0.
            let mut meat = Matrix::zeros(k, k);
            add_gamma(x, residuals, 0, &mut meat);
            // Γ_l + Γ_l' weighted by w_l = 1 - l/(L+1).
            for lag in 1..=l {
                let w = 1.0 - lag as f64 / (l as f64 + 1.0);
                let mut gamma = Matrix::zeros(k, k);
                add_gamma(x, residuals, lag, &mut gamma);
                for idx in 0..meat.data.len() {
                    meat.data[idx] += w * (gamma.data[idx] + gamma.t_index(idx));
                }
            }
            // Small-sample factor n/(n-k), matching common practice.
            meat.scale_mut(n as f64 / df_r as f64);
            sandwich(xtx_inv, &meat)
        }
    }
}

fn sandwich(a: &Matrix, meat: &Matrix) -> Result<Matrix> {
    let left = a.mul(meat)?;
    left.mul(a)
}

impl Matrix {
    /// Transposed index helper: element (i,j) -> (j,i) for a square matrix.
    fn t_index(&self, flat: usize) -> f64 {
        let j = flat % self.cols;
        let i = flat / self.cols;
        self.data[j * self.cols + i]
    }
}

/// Add Γ_l = Σ_{t>l} u_t u_{t-l} x_t x_{t-l}' into `acc`.
fn add_gamma(x: &Matrix, u: &[f64], lag: usize, acc: &mut Matrix) {
    let k = x.cols;
    for t in lag..x.rows {
        let wu = u[t] * u[t - lag];
        if wu == 0.0 {
            continue;
        }
        let current = x.row(t);
        let past = x.row(t - lag);
        for a in 0..k {
            for b in 0..k {
                acc.data[a * k + b] += wu * current[a] * past[b];
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linalg::{Matrix, QR};

    /// Simple regression dataset: y = 1 + 2x + e with e from a fixed pattern.
    fn setup() -> (Matrix, Vec<f64>, Matrix, Vec<f64>, Vec<f64>, Vec<ClusterId>) {
        let rows: Vec<Vec<f64>> = (0..24)
            .map(|i| {
                let x = (i % 8) as f64;
                vec![1.0, x]
            })
            .collect();
        let y: Vec<f64> = (0..24)
            .map(|i| {
                let x = (i % 8) as f64;
                let e = if i % 3 == 0 { 1.0 } else { -0.5 };
                1.0 + 2.0 * x + e
            })
            .collect();
        let x = Matrix::from_rows(&rows).unwrap();
        let qr = QR::decompose(&x).unwrap();
        let beta = qr.solve(&y);
        let fitted = x.mul_vec(&beta);
        let residuals: Vec<f64> = y.iter().zip(&fitted).map(|(a, b)| a - b).collect();
        let xtx_inv = qr.xtx_inv();
        let leverage: Vec<f64> = (0..x.rows)
            .map(|i| {
                let row = x.row(i);
                let mut h = 0.0;
                for a in 0..2 {
                    for b in 0..2 {
                        h += row[a] * xtx_inv.get(a, b) * row[b];
                    }
                }
                h
            })
            .collect();
        let clusters: Vec<ClusterId> = (0..24).map(|i| (i / 4) as u32).collect();
        (x, y, xtx_inv, residuals, leverage, clusters)
    }

    #[test]
    fn classical_matches_manual_sigma2_formula() {
        let (x, _y, xtx_inv, residuals, _lev, _cl) = setup();
        let v = vcov(
            &x,
            &residuals,
            &xtx_inv,
            &vec![0.0; x.rows],
            None,
            &VcovSpec::Classical,
        )
        .unwrap();
        let ssr: f64 = residuals.iter().map(|u| u * u).sum();
        let df = x.rows - x.cols;
        let s2 = ssr / df as f64;
        assert!((v.get(0, 0) - s2 * xtx_inv.get(0, 0)).abs() < 1e-12);
        assert!((v.get(1, 1) - s2 * xtx_inv.get(1, 1)).abs() < 1e-12);
    }

    #[test]
    fn hc1_scales_hc0() {
        let (x, _y, xtx_inv, residuals, lev, _cl) = setup();
        let v0 = vcov(&x, &residuals, &xtx_inv, &lev, None, &VcovSpec::Hc0).unwrap();
        let v1 = vcov(&x, &residuals, &xtx_inv, &lev, None, &VcovSpec::Hc1).unwrap();
        let factor = x.rows as f64 / (x.rows - x.cols) as f64;
        assert!((v1.get(1, 1) / v0.get(1, 1) - factor).abs() < 1e-10);
    }

    #[test]
    fn hc2_hc3_larger_than_hc0() {
        let (x, _y, xtx_inv, residuals, lev, _cl) = setup();
        let v0 = vcov(&x, &residuals, &xtx_inv, &lev, None, &VcovSpec::Hc0)
            .unwrap()
            .get(1, 1);
        let v2 = vcov(&x, &residuals, &xtx_inv, &lev, None, &VcovSpec::Hc2)
            .unwrap()
            .get(1, 1);
        let v3 = vcov(&x, &residuals, &xtx_inv, &lev, None, &VcovSpec::Hc3)
            .unwrap()
            .get(1, 1);
        assert!(v2 >= v0);
        assert!(v3 >= v2);
    }

    #[test]
    fn cluster_requires_variable() {
        let (x, _y, xtx_inv, residuals, lev, _cl) = setup();
        let err = vcov(&x, &residuals, &xtx_inv, &lev, None, &VcovSpec::Cluster).unwrap_err();
        assert!(err.what.contains("cluster"));
    }

    #[test]
    fn cluster_covariance_finite_and_positive() {
        let (x, _y, xtx_inv, residuals, lev, cl) = setup();
        let v = vcov(
            &x,
            &residuals,
            &xtx_inv,
            &lev,
            Some(&cl),
            &VcovSpec::Cluster,
        )
        .unwrap();
        assert!(v.get(1, 1) > 0.0);
        assert!(v.get(1, 1).is_finite());
    }

    #[test]
    fn hac_finite() {
        let (x, _y, xtx_inv, residuals, lev, _cl) = setup();
        let v = vcov(
            &x,
            &residuals,
            &xtx_inv,
            &lev,
            None,
            &VcovSpec::Hac { lags: 2 },
        )
        .unwrap();
        assert!(v.get(0, 0).is_finite() && v.get(0, 0) > 0.0);
        assert!(v.get(1, 1).is_finite() && v.get(1, 1) > 0.0);
    }
}
