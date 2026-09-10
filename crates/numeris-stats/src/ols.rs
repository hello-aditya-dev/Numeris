//! # Ordinary least squares
//!
//! QR-based OLS with a shared result object used by the Results UI,
//! publication tables, coefficient plots, reports and replication packages.
//!
//! The same `Regression` result is produced regardless of whether the user
//! arrives through the GUI form or the command language.

use crate::linalg::{Matrix, QR};
use crate::vcov::{vcov, ClusterId, VcovSpec};
use numeris_core::dist;
use numeris_core::error::{NumerisError, Result};
use serde::{Deserialize, Serialize};

/// A fitted linear regression with full inferential detail.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Regression {
    /// Term names, starting with `_cons` when a constant is included.
    pub terms: Vec<String>,
    pub coef: Vec<f64>,
    pub se: Vec<f64>,
    pub t: Vec<f64>,
    pub p: Vec<f64>,
    pub ci_lo: Vec<f64>,
    pub ci_hi: Vec<f64>,
    pub n: usize,
    /// Number of estimated parameters (including constant).
    pub k: usize,
    pub df_r: usize,
    pub r2: f64,
    pub adj_r2: f64,
    /// Overall F test of joint significance (present when a constant is included).
    pub f: Option<f64>,
    pub f_p: Option<f64>,
    pub f_df: Option<(usize, usize)>,
    /// Root MSE.
    pub rmse: f64,
    pub loglik: f64,
    pub aic: f64,
    pub bic: f64,
    pub residuals: Vec<f64>,
    pub fitted: Vec<f64>,
    /// Hat diagonal h_ii.
    pub leverage: Vec<f64>,
    /// (X'X)^{-1} for post-estimation.
    pub xtx_inv: Matrix,
    pub vcov_label: String,
    pub estimator_label: String,
}

/// Builder for OLS/WLS estimation.
pub struct Ols {
    y: Vec<f64>,
    x: Matrix,
    names: Vec<String>,
    vcov_spec: VcovSpec,
    clusters: Option<Vec<ClusterId>>,
    weights: Option<Vec<f64>>,
    constant: bool,
    estimator_label: String,
}

impl Ols {
    /// `xs` columns in order; a constant is prepended automatically unless
    /// `no_constant` is set. `names` must match `xs` columns.
    pub fn new(y: &[f64], xs: &Matrix, names: &[String]) -> Result<Self> {
        if y.len() != xs.rows {
            return Err(NumerisError::numerical_failure(
                "Outcome and predictor vectors have different lengths.",
                "Internal error: the design matrix and the outcome were misaligned.",
                "Please report this as a bug with the command that triggered it.",
            ));
        }
        if names.len() != xs.cols {
            return Err(NumerisError::numerical_failure(
                "Predictor names do not match the design matrix.",
                "Internal error in model assembly.",
                "Please report this as a bug with the command that triggered it.",
            ));
        }
        if y.len() < 2 {
            return Err(NumerisError::insufficient_data(
                "The estimation sample has fewer than two observations.",
                "After listwise deletion, too few rows remain to fit a regression.",
                "Inspect missing values in the outcome and predictors, then rerun.",
            ));
        }
        Ok(Self {
            y: y.to_vec(),
            x: xs.clone(),
            names: names.to_vec(),
            vcov_spec: VcovSpec::Classical,
            clusters: None,
            weights: None,
            constant: true,
            estimator_label: "OLS".to_string(),
        })
    }

    pub fn robust(mut self) -> Self {
        self.vcov_spec = VcovSpec::Hc1;
        self
    }

    pub fn vcov(mut self, spec: VcovSpec) -> Self {
        self.vcov_spec = spec;
        self
    }

    pub fn cluster(mut self, ids: Vec<ClusterId>) -> Self {
        self.vcov_spec = VcovSpec::Cluster;
        self.clusters = Some(ids);
        self
    }

    pub fn weights(mut self, w: Vec<f64>) -> Self {
        self.weights = Some(w);
        self
    }

    pub fn no_constant(mut self) -> Self {
        self.constant = false;
        self
    }

    pub fn estimator_label(mut self, label: &str) -> Self {
        self.estimator_label = label.to_string();
        self
    }

    /// Fit the model.
    pub fn fit(self) -> Result<Regression> {
        // Weighted least squares: transform y and the FULL design (including
        // the constant column) by sqrt(w). Scaling must happen after the
        // constant is prepended, otherwise the intercept is not weighted.
        let (y, design, is_wls) = match &self.weights {
            Some(w) => {
                if w.len() != self.y.len() {
                    return Err(NumerisError::numerical_failure(
                        "Weights vector length does not match the estimation sample.",
                        "Internal error in WLS assembly.",
                        "Please report this as a bug with the command that triggered it.",
                    ));
                }
                if w.iter().any(|v| !(v.is_finite() && *v > 0.0)) {
                    return Err(NumerisError::invalid_request(
                        "All weights must be positive and finite.",
                        "The weight variable contains zero, negative, or missing values.",
                        "Inspect the weight variable and choose weights that are strictly positive.",
                    ));
                }
                let sw: Vec<f64> = w.iter().map(|v| v.sqrt()).collect();
                let yw: Vec<f64> = self.y.iter().zip(&sw).map(|(a, s)| a * s).collect();
                let full = with_constant(&self.x, self.constant);
                (yw, scale_rows(&full, &sw), true)
            }
            None => (self.y.clone(), with_constant(&self.x, self.constant), false),
        };
        let terms = self.term_names();

        let n = design.rows;
        let k = design.cols;
        if n <= k {
            return Err(NumerisError::insufficient_data(
                format!(
                    "The estimation sample ({n} observations) is too small for {k} parameters."
                ),
                "Linear regression requires more observations than estimated parameters.",
                "Use fewer predictors or a larger sample.",
            ));
        }

        let qr = QR::decompose(&design)?;
        let deficient = qr.rank_deficient_columns();
        if !deficient.is_empty() {
            let bad: Vec<String> = deficient.iter().map(|&i| terms[i].clone()).collect();
            return Err(NumerisError::not_identified(
                "The regression is not identified: some predictors are perfectly collinear with the others.",
                format!(
                    "The following terms are redundant in the design matrix: {}. Exact linear dependence makes the coefficients non-estimable.",
                    bad.join(", ")
                ),
                "Drop one of the collinear variables, or inspect 'estat vif' after removing the constant from the collinear set.",
            ));
        }

        let beta = qr.solve(&y);
        let fitted = design.mul_vec(&beta);
        let residuals: Vec<f64> = y.iter().zip(&fitted).map(|(a, b)| a - b).collect();
        let xtx_inv = qr.xtx_inv();

        // Leverage: h_ii = x_i' (X'X)^{-1} x_i.
        let leverage = leverage_diagonal(&design, &xtx_inv);

        // Fit statistics on the (weighted) estimation sample.
        let ssr: f64 = residuals.iter().map(|u| u * u).sum();
        let df_r = n - k;
        let sigma2 = ssr / df_r as f64;
        let rmse = sigma2.sqrt();

        let ybar = y.iter().sum::<f64>() / n as f64;
        let sst: f64 = y.iter().map(|v| (v - ybar) * (v - ybar)).sum();
        let r2 = if sst > 0.0 {
            (1.0 - ssr / sst).clamp(-1e-12, 1.0)
        } else if ssr < 1e-14 {
            1.0
        } else {
            0.0
        };
        let adj_r2 = if n > 0 && sst > 0.0 {
            1.0 - (1.0 - r2) * (n as f64 - 1.0) / df_r as f64
        } else {
            0.0
        };

        // Overall F test (only meaningful with a constant).
        let (f, f_p, f_df) = if self.constant && k > 1 {
            let df_m = k - 1;
            let f_stat = (r2 / df_m as f64) / ((1.0 - r2) / df_r as f64);
            if f_stat.is_finite() && f_stat >= 0.0 {
                (
                    Some(f_stat),
                    Some(1.0 - dist::f_cdf(f_stat, df_m as f64, df_r as f64)),
                    Some((df_m, df_r)),
                )
            } else {
                (None, None, None)
            }
        } else {
            (None, None, None)
        };

        // Log-likelihood under normality (MLE variance).
        let ln2pi = (2.0 * std::f64::consts::PI).ln();
        let sigma2_mle = if ssr > 0.0 { ssr / n as f64 } else { 1e-300 };
        let loglik = -0.5 * n as f64 * (ln2pi + sigma2_mle.ln() + 1.0);
        let aic = -2.0 * loglik + 2.0 * k as f64;
        let bic = -2.0 * loglik + (n as f64).ln() * k as f64;

        // Covariance of β̂.
        let v = vcov(
            &design,
            &residuals,
            &xtx_inv,
            &leverage,
            self.clusters.as_deref(),
            &self.vcov_spec,
        )?;

        // Inference per coefficient.
        let mut se = Vec::with_capacity(k);
        let mut tstat = Vec::with_capacity(k);
        let mut pval = Vec::with_capacity(k);
        let mut lo = Vec::with_capacity(k);
        let mut hi = Vec::with_capacity(k);
        for j in 0..k {
            let s = v.get(j, j).max(0.0).sqrt();
            let t = if s > 0.0 { beta[j] / s } else { f64::NAN };
            let p = if s > 0.0 {
                dist::t_two_sided_p(t, df_r as f64)
            } else {
                f64::NAN
            };
            let tcrit = dist::t_quantile(0.975, df_r as f64)?;
            se.push(s);
            tstat.push(t);
            pval.push(p);
            lo.push(beta[j] - tcrit * s);
            hi.push(beta[j] + tcrit * s);
        }

        let estimator_label = if is_wls && self.estimator_label == "OLS" {
            "WLS".to_string()
        } else {
            self.estimator_label
        };

        Ok(Regression {
            terms,
            coef: beta,
            se,
            t: tstat,
            p: pval,
            ci_lo: lo,
            ci_hi: hi,
            n,
            k,
            df_r,
            r2,
            adj_r2,
            f,
            f_p,
            f_df,
            rmse,
            loglik,
            aic,
            bic,
            residuals,
            fitted,
            leverage,
            xtx_inv,
            vcov_label: self.vcov_spec.label().to_string(),
            estimator_label,
        })
    }

    fn term_names(&self) -> Vec<String> {
        let mut terms = Vec::with_capacity(self.x.cols + 1);
        if self.constant {
            terms.push("_cons".to_string());
        }
        terms.extend(self.names.iter().cloned());
        terms
    }
}

fn scale_rows(x: &Matrix, s: &[f64]) -> Matrix {
    let mut out = x.clone();
    for i in 0..out.rows {
        for j in 0..out.cols {
            out.data[i * out.cols + j] *= s[i];
        }
    }
    out
}

fn with_constant(x: &Matrix, constant: bool) -> Matrix {
    if !constant {
        return x.clone();
    }
    let mut out = Matrix::zeros(x.rows, x.cols + 1);
    for i in 0..x.rows {
        out.data[i * out.cols] = 1.0;
        for j in 0..x.cols {
            out.data[i * out.cols + 1 + j] = x.get(i, j);
        }
    }
    out
}

/// Hat diagonal from (X'X)^{-1}.
pub fn leverage_diagonal(x: &Matrix, xtx_inv: &Matrix) -> Vec<f64> {
    let k = x.cols;
    (0..x.rows)
        .map(|i| {
            let row = x.row(i);
            let mut h = 0.0;
            for a in 0..k {
                if row[a] == 0.0 {
                    continue;
                }
                for b in 0..k {
                    h += row[a] * xtx_inv.get(a, b) * row[b];
                }
            }
            h
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn by_rozen() -> (Vec<f64>, Vec<Vec<f64>>, Vec<String>) {
        // Simple exact linear model: y = 3 + 2x
        let y: Vec<f64> = (0..10).map(|i| 3.0 + 2.0 * i as f64).collect();
        let rows: Vec<Vec<f64>> = (0..10).map(|i| vec![i as f64]).collect();
        (y, rows, vec!["x".to_string()])
    }

    #[test]
    fn exact_linear_fit_recovers_coefficients() {
        let (y, rows, names) = by_rozen();
        let x = Matrix::from_rows(&rows).unwrap();
        let reg = Ols::new(&y, &x, &names).unwrap().fit().unwrap();
        assert!((reg.coef[0] - 3.0).abs() < 1e-10);
        assert!((reg.coef[1] - 2.0).abs() < 1e-10);
        assert!((reg.r2 - 1.0).abs() < 1e-12);
        assert!(reg.se.iter().all(|s| *s < 1e-10));
    }

    #[test]
    fn classical_se_matches_manual_formula() {
        // y = 3 + 2x + e with e orthogonal to [1, x] by construction:
        // symmetric-pair noise (e_i = e_{19-i}) gives Σe = 0 and Σe·(x−x̄) = 0,
        // so residuals are exactly the noise and SSR/RMSE/SE are closed-form.
        let e: Vec<f64> = (0..20)
            .map(|i| {
                let pair = i.min(19 - i);
                match pair {
                    0 | 2 | 4 | 6 | 8 => 1.0,
                    _ => -1.0,
                }
            })
            .collect();
        let y: Vec<f64> = (0..20).map(|i| 3.0 + 2.0 * i as f64 + e[i]).collect();
        let rows: Vec<Vec<f64>> = (0..20).map(|i| vec![i as f64]).collect();
        let x = Matrix::from_rows(&rows).unwrap();
        let names = vec!["x".to_string()];
        let reg = Ols::new(&y, &x, &names).unwrap().fit().unwrap();
        assert!(reg
            .residuals
            .iter()
            .enumerate()
            .all(|(i, &r)| (r - e[i]).abs() < 1e-9));
        assert!((reg.coef[0] - 3.0).abs() < 1e-9);
        assert!((reg.coef[1] - 2.0).abs() < 1e-9);
        let ssr: f64 = reg.residuals.iter().map(|e| e * e).sum();
        assert!((ssr - 20.0).abs() < 1e-8, "ssr = {ssr}");
        assert!((reg.rmse - (20.0f64 / 18.0).sqrt()).abs() < 1e-10);
        // Closed-form SE of the slope: se(b1)² = σ² / Σ(x_i − x̄)².
        let xbar = 9.5;
        let sxx: f64 = (0..20).map(|i| (i as f64 - xbar).powi(2)).sum();
        let expected_se1 = (20.0 / 18.0 / sxx).sqrt();
        assert!(
            (reg.se[1] - expected_se1).abs() < 1e-10,
            "se1 = {} expected {expected_se1}",
            reg.se[1]
        );
    }

    #[test]
    fn collinear_predictors_produce_not_identified() {
        let y: Vec<f64> = (0..12).map(|i| i as f64).collect();
        let rows: Vec<Vec<f64>> = (0..12)
            .map(|i| vec![i as f64, 2.0 * i as f64 + 5.0])
            .collect();
        let x = Matrix::from_rows(&rows).unwrap();
        let names = vec!["a".to_string(), "b".to_string()];
        let err = Ols::new(&y, &x, &names).unwrap().fit().unwrap_err();
        assert!(matches!(
            err.kind,
            numeris_core::error::ErrorKind::NotIdentified
        ));
        assert!(err.why.contains("redundant"));
    }

    #[test]
    fn robust_se_differ_from_classical_when_heteroskedastic() {
        // Heteroskedastic pattern: error grows with x.
        let y: Vec<f64> = (0..60)
            .map(|i| {
                let x = (i as f64) / 10.0;
                x + if i % 2 == 0 { 0.5 * x } else { -0.5 * x }
            })
            .collect();
        let rows: Vec<Vec<f64>> = (0..60).map(|i| vec![i as f64 / 10.0]).collect();
        let x = Matrix::from_rows(&rows).unwrap();
        let names = vec!["x".to_string()];
        let classical = Ols::new(&y, &x, &names).unwrap().fit().unwrap();
        let robust = Ols::new(&y, &x, &names).unwrap().robust().fit().unwrap();
        assert!((classical.se[1] - robust.se[1]).abs() > 1e-9);
        assert_eq!(robust.vcov_label, "HC1 robust");
    }

    #[test]
    fn wls_recovers_exact_model_through_noise_scaling() {
        // y = 1 + x with variance proportional to w; WLS should be exact on
        // the transformed problem for a noiseless relation.
        let w: Vec<f64> = (0..20).map(|i| 1.0 + i as f64 * 0.5).collect();
        let rows: Vec<Vec<f64>> = (0..20).map(|i| vec![i as f64]).collect();
        let x = Matrix::from_rows(&rows).unwrap();
        let y: Vec<f64> = (0..20).map(|i| 1.0 + i as f64).collect();
        let names = vec!["x".to_string()];
        let reg = Ols::new(&y, &x, &names).unwrap().weights(w).fit().unwrap();
        assert!((reg.coef[0] - 1.0).abs() < 1e-9);
        assert!((reg.coef[1] - 1.0).abs() < 1e-9);
        assert_eq!(reg.estimator_label, "WLS");
    }

    #[test]
    fn no_constant_fit() {
        let y: Vec<f64> = (0..10).map(|i| 3.0 * i as f64).collect();
        let rows: Vec<Vec<f64>> = (0..10).map(|i| vec![i as f64]).collect();
        let x = Matrix::from_rows(&rows).unwrap();
        let names = vec!["x".to_string()];
        let reg = Ols::new(&y, &x, &names)
            .unwrap()
            .no_constant()
            .fit()
            .unwrap();
        assert_eq!(reg.terms, vec!["x"]);
        assert!((reg.coef[0] - 3.0).abs() < 1e-10);
        assert!(reg.f.is_none());
    }

    #[test]
    fn inference_columns_are_consistent() {
        let y: Vec<f64> = (0..30)
            .map(|i| 1.0 + 0.5 * i as f64 + if i % 3 == 0 { 2.0 } else { 0.0 })
            .collect();
        let rows: Vec<Vec<f64>> = (0..30).map(|i| vec![i as f64]).collect();
        let x = Matrix::from_rows(&rows).unwrap();
        let names = vec!["x".to_string()];
        let reg = Ols::new(&y, &x, &names).unwrap().fit().unwrap();
        for j in 0..reg.k {
            let t = reg.coef[j] / reg.se[j];
            assert!((t - reg.t[j]).abs() < 1e-10);
            assert!(reg.ci_lo[j] <= reg.ci_hi[j]);
            let p = numeris_core::dist::t_two_sided_p(reg.t[j], reg.df_r as f64);
            assert!((p - reg.p[j]).abs() < 1e-12);
        }
    }
}
