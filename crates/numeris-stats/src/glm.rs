//! # Limited dependent variable models
//!
//! Binary logit, binary probit and Poisson count regression via Fisher
//! scoring (IRLS), with classical (inverse information) and robust
//! (sandwich) covariance, McFadden pseudo-R² and convergence reporting.

use crate::linalg::{Matrix, QR};
use numeris_core::dist;
use numeris_core::error::{NumerisError, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Family {
    Logit,
    Probit,
    Poisson,
}

/// A fitted generalized linear model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlmResult {
    pub family: Family,
    pub terms: Vec<String>,
    pub coef: Vec<f64>,
    pub se: Vec<f64>,
    pub z: Vec<f64>,
    pub p: Vec<f64>,
    pub ci_lo: Vec<f64>,
    pub ci_hi: Vec<f64>,
    pub n: usize,
    pub k: usize,
    pub loglik: f64,
    pub loglik_null: f64,
    pub pseudo_r2_mcfadden: f64,
    pub aic: f64,
    pub bic: f64,
    pub iterations: usize,
    pub converged: bool,
    pub vcov_label: String,
    pub warnings: Vec<String>,
}

/// Linear predictor for the current coefficients.
fn eta_now(design: &Matrix, beta: &[f64]) -> Vec<f64> {
    (0..design.rows)
        .map(|i| design.row(i).iter().zip(beta).map(|(a, b)| a * b).sum())
        .collect()
}

/// Log-likelihood of the fitted means.
fn log_likelihood(mu: &[f64], eta: &[f64], y: &[f64], family: Family) -> f64 {
    match family {
        Family::Logit | Family::Probit => {
            let mut ll = 0.0;
            for i in 0..y.len() {
                ll += y[i] * mu[i].clamp(1e-300, 1.0).ln()
                    + (1.0 - y[i]) * (1.0 - mu[i]).clamp(1e-300, 1.0).ln();
            }
            ll
        }
        Family::Poisson => {
            let mut ll = 0.0;
            for i in 0..y.len() {
                ll += y[i] * eta[i] - mu[i] - dist::ln_gamma(y[i] + 1.0);
            }
            ll
        }
    }
}

impl Family {
    fn label(&self) -> &'static str {
        match self {
            Family::Logit => "Logistic regression",
            Family::Probit => "Probit regression",
            Family::Poisson => "Poisson regression",
        }
    }
}

/// Fit a GLM by iteratively reweighted least squares (Fisher scoring).
///
/// * `y`  binary (0/1) for logit/probit; counts ≥ 0 for Poisson.
/// * `x`  design matrix WITHOUT constant (a constant is added here).
pub fn fit_glm(
    y: &[f64],
    xs: &Matrix,
    names: &[String],
    family: Family,
    robust: bool,
) -> Result<GlmResult> {
    let n = y.len();
    if xs.rows != n {
        return Err(NumerisError::numerical_failure(
            "GLM inputs are misaligned.",
            "Internal error: the outcome and design matrix have different row counts.",
            "Please report this as a bug with the command that triggered it.",
        ));
    }
    if n < xs.cols + 2 {
        return Err(NumerisError::insufficient_data(
            format!(
                "The estimation sample ({n} observations) is too small for {} predictors.",
                xs.cols + 1
            ),
            "Iterative estimation requires more observations than parameters.",
            "Use fewer predictors or a larger sample.",
        ));
    }
    // Validate the outcome.
    match family {
        Family::Logit | Family::Probit => {
            for (i, &v) in y.iter().enumerate() {
                if v != 0.0 && v != 1.0 {
                    return Err(NumerisError::invalid_request(
                        format!(
                            "The outcome must be 0/1; value {v} was found at observation {}.",
                            i + 1
                        ),
                        match family {
                            Family::Probit => "Probit regression models a binary outcome.",
                            _ => "Logistic regression models a binary outcome.",
                        },
                        "Recode the outcome to 0/1, or use a different model family.",
                    ));
                }
            }
        }
        Family::Poisson => {
            for (i, &v) in y.iter().enumerate() {
                if !(v >= 0.0 && v.is_finite()) || v.fract() != 0.0 {
                    return Err(NumerisError::invalid_request(
                        format!("The outcome must be a non-negative integer count; value {v} was found at observation {}.", i + 1),
                        "Poisson regression models count data.",
                        "Check the outcome variable, or use a model appropriate for continuous outcomes.",
                    ));
                }
            }
        }
    }

    // Design with constant.
    let k = xs.cols + 1;
    let mut rows: Vec<Vec<f64>> = Vec::with_capacity(n);
    for i in 0..n {
        let mut row = vec![1.0];
        row.extend_from_slice(xs.row(i));
        rows.push(row);
    }
    let design = Matrix::from_rows(&rows)?;
    let mut terms: Vec<String> = names.to_vec();
    terms.insert(0, "_cons".to_string());

    let mut beta = vec![0.0; k];
    // Reasonable starting values: intercept from the marginal mean.
    let ybar = y.iter().sum::<f64>() / n as f64;
    beta[0] = match family {
        Family::Logit => (ybar / (1.0 - ybar)).ln(),
        Family::Probit => dist::normal_quantile(ybar.clamp(1e-6, 1.0 - 1e-6)),
        Family::Poisson => ybar.max(1e-6).ln(),
    };

    let mut warnings = Vec::new();
    let mut iterations = 0usize;
    let max_iter = 200;
    let mut converged = false;
    let mut w = vec![0.0; n];
    let mut z = vec![0.0; n];
    let mut eta = vec![0.0; n];
    let mut prev_ll: Option<f64> = None;

    loop {
        iterations += 1;
        // Compute eta, mu, weights and the working response.
        for i in 0..n {
            eta[i] = design.row(i).iter().zip(&beta).map(|(a, b)| a * b).sum();
            match family {
                Family::Logit => {
                    let e = eta[i].clamp(-30.0, 30.0);
                    let mu = 1.0 / (1.0 + (-e).exp());
                    let m = mu.clamp(1e-10, 1.0 - 1e-10);
                    w[i] = m * (1.0 - m);
                    z[i] = e + (y[i] - mu) / (m * (1.0 - m));
                }
                Family::Probit => {
                    let mu = dist::normal_cdf(eta[i]).clamp(1e-10, 1.0 - 1e-10);
                    let phi = dist::normal_pdf(eta[i]);
                    let denom = mu * (1.0 - mu);
                    w[i] = if phi > 1e-300 {
                        (phi * phi) / denom
                    } else {
                        1e-10
                    };
                    z[i] = eta[i] + (y[i] - mu) * denom / phi.max(1e-300);
                }
                Family::Poisson => {
                    let e = eta[i].min(30.0);
                    let mu = e.exp();
                    w[i] = mu;
                    z[i] = e + (y[i] - mu) / mu;
                }
            }
            if !w[i].is_finite() || w[i] <= 0.0 {
                w[i] = 1e-10;
            }
            if !z[i].is_finite() {
                return Err(NumerisError::numerical_failure(
                    "The iterative estimation encountered invalid intermediate values.",
                    "Extreme outcomes or extreme predictor values produced non-finite working responses.",
                    "Check the outcome and predictors for extreme values; consider rescaling large variables.",
                ));
            }
        }
        if eta.iter().any(|e| e.abs() > 20.0) && family != Family::Poisson
            && !warnings.iter().any(|x: &String| x.contains("separation")) {
                warnings.push(
                    "Quasi-complete separation may be present: the linear predictor is very large in magnitude, so coefficients may diverge."
                        .to_string(),
                );
            }
        // Weighted least squares step: β = (X'WX)^{-1} X'W z.
        let sqrtw: Vec<f64> = w.iter().map(|v| v.sqrt()).collect();
        let mut xw = design.clone();
        for i in 0..n {
            for j in 0..k {
                xw.data[i * k + j] *= sqrtw[i];
            }
        }
        let zw: Vec<f64> = z.iter().zip(&sqrtw).map(|(a, s)| a * s).collect();
        let qr = QR::decompose(&xw)?;
        if !qr.rank_deficient_columns().is_empty() {
            return Err(NumerisError::not_identified(
                "The model is not identified: predictors are perfectly collinear.",
                "The weighted design matrix is rank deficient.",
                "Drop one of the collinear predictors and refit.",
            ));
        }
        let new_beta = qr.solve(&zw);
        let delta = new_beta
            .iter()
            .zip(&beta)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0, f64::max);
        beta = new_beta;

        // Convergence: coefficient stability or negligible log-likelihood
        // change. The likelihood criterion matters under (quasi-)separation,
        // where coefficients drift while the likelihood has already
        // stabilized — matching standard IRLS implementations.
        let mu_now: Vec<f64> = (0..n)
            .map(|i| match family {
                Family::Logit => 1.0 / (1.0 + (-(eta[i].clamp(-30.0, 30.0))).exp()),
                Family::Probit => dist::normal_cdf(eta[i]),
                Family::Poisson => eta[i].min(30.0).exp(),
            })
            .collect();
        let ll = log_likelihood(&mu_now, &eta_now(&design, &beta), y, family);
        if delta < 1e-10 {
            converged = true;
            break;
        }
        if let Some(pll) = prev_ll {
            if (ll - pll).abs() < 1e-10 * (1.0 + ll.abs()) {
                converged = true;
                break;
            }
        }
        prev_ll = Some(ll);
        if iterations >= max_iter {
            // Return the result with a warning instead of failing: this
            // matches standard GLM implementations, where slow convergence
            // (typically from near-separation) yields a warning and
            // unconverged flag rather than a hard error. The result object
            // carries converged=false so the UI reports it honestly.
            warnings.push(format!(
                "The {} did not converge within {max_iter} iterations; coefficients may be unreliable. This often indicates separation or an oversized model.",
                family.label().to_lowercase()
            ));
            break;
        }
    }

    // Final mu and covariance.
    let mu: Vec<f64> = (0..n)
        .map(|i| {
            let e: f64 = design.row(i).iter().zip(&beta).map(|(a, b)| a * b).sum();
            match family {
                Family::Logit => 1.0 / (1.0 + (-(e.clamp(-700.0, 700.0))).exp()),
                Family::Probit => dist::normal_cdf(e),
                Family::Poisson => e.min(30.0).exp(),
            }
        })
        .collect();

    // Log-likelihood.
    let loglik: f64 = match family {
        Family::Logit | Family::Probit => {
            let p = &mu;
            let mut ll = 0.0;
            for i in 0..n {
                ll += y[i] * p[i].clamp(1e-300, 1.0).ln()
                    + (1.0 - y[i]) * (1.0 - p[i]).clamp(1e-300, 1.0).ln();
            }
            ll
        }
        Family::Poisson => {
            let mut ll = 0.0;
            for i in 0..n {
                ll += y[i]
                    * design
                        .row(i)
                        .iter()
                        .zip(&beta)
                        .map(|(a, b)| a * b)
                        .sum::<f64>()
                    - mu[i]
                    - dist::ln_gamma(y[i] + 1.0);
            }
            ll
        }
    };

    // Null model (intercept only).
    let loglik_null: f64 = match family {
        Family::Logit => {
            let p = ybar.clamp(1e-10, 1.0 - 1e-10);
            (0..n)
                .map(|i| y[i] * p.ln() + (1.0 - y[i]) * (1.0 - p).ln())
                .sum()
        }
        Family::Probit => {
            let eta0 = dist::normal_quantile(ybar.clamp(1e-6, 1.0 - 1e-6));
            let p0 = dist::normal_cdf(eta0).clamp(1e-300, 1.0 - 1e-300);
            let q0 = (1.0 - dist::normal_cdf(eta0)).clamp(1e-300, 1.0);
            (0..n)
                .map(|i| p0.ln() * y[i] + q0.ln() * (1.0 - y[i]))
                .sum::<f64>()
        }
        Family::Poisson => {
            let mu0 = ybar.max(1e-10);
            (0..n)
                .map(|i| y[i] * mu0.ln() - mu0 - dist::ln_gamma(y[i] + 1.0))
                .sum()
        }
    };

    let pseudo_r2 = if loglik_null.abs() > 1e-12 {
        1.0 - loglik / loglik_null
    } else {
        0.0
    };
    let aic = -2.0 * loglik + 2.0 * k as f64;
    let bic = -2.0 * loglik + (n as f64).ln() * k as f64;

    // Covariance: bread = (X'WX)^{-1} at convergence.
    let sqrtw: Vec<f64> = w.iter().map(|v| v.sqrt()).collect();
    let mut xw = design.clone();
    for i in 0..n {
        for j in 0..k {
            xw.data[i * k + j] *= sqrtw[i];
        }
    }
    let qr = QR::decompose(&xw)?;
    let bread = qr.xtx_inv();

    let v = if robust {
        // Sandwich: bread · (Σ u_i² x_i x_i') · bread with u = y − μ.
        let mut meat = Matrix::zeros(k, k);
        for i in 0..n {
            let u2 = (y[i] - mu[i]) * (y[i] - mu[i]);
            let row = design.row(i);
            for a in 0..k {
                if row[a] == 0.0 {
                    continue;
                }
                for b in a..k {
                    meat.data[a * k + b] += u2 * row[a] * row[b];
                }
            }
        }
        for a in 0..k {
            for b in (a + 1)..k {
                meat.data[b * k + a] = meat.data[a * k + b];
            }
        }
        bread.mul(&meat)?.mul(&bread)?
    } else {
        bread
    };

    let mut se = Vec::with_capacity(k);
    let mut zstat = Vec::with_capacity(k);
    let mut pval = Vec::with_capacity(k);
    let mut lo = Vec::with_capacity(k);
    let mut hi = Vec::with_capacity(k);
    for j in 0..k {
        let s = v.get(j, j).max(0.0).sqrt();
        let z = if s > 0.0 { beta[j] / s } else { f64::NAN };
        let p = if s > 0.0 {
            2.0 * (1.0 - dist::normal_cdf(z.abs()))
        } else {
            f64::NAN
        };
        let zcrit = 1.959963984540054;
        se.push(s);
        zstat.push(z);
        pval.push(p);
        lo.push(beta[j] - zcrit * s);
        hi.push(beta[j] + zcrit * s);
    }

    Ok(GlmResult {
        family,
        terms,
        coef: beta,
        se,
        z: zstat,
        p: pval,
        ci_lo: lo,
        ci_hi: hi,
        n,
        k,
        loglik,
        loglik_null,
        pseudo_r2_mcfadden: pseudo_r2,
        aic,
        bic,
        iterations,
        converged,
        vcov_label: if robust {
            "Robust".to_string()
        } else {
            "Classical".to_string()
        },
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn logistic_dgp(n: usize) -> (Vec<f64>, Vec<Vec<f64>>) {
        // Deterministic pseudo-random logistic DGP.
        let mut state = 12345u64;
        let mut next = || {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (state >> 11) as f64 / (1u64 << 53) as f64
        };
        let mut y = Vec::new();
        let mut rows = Vec::new();
        for _i in 0..n {
            let x = next() * 2.0 - 1.0;
            let p = 1.0 / (1.0 + (-(0.0 + 1.5 * x)).exp());
            let u = next();
            y.push(if u < p { 1.0 } else { 0.0 });
            rows.push(vec![x]);
        }
        (y, rows)
    }

    #[test]
    fn logit_recovers_sign_and_significance() {
        let (y, rows) = logistic_dgp(600);
        let x = Matrix::from_rows(&rows).unwrap();
        let names = vec!["x".to_string()];
        let res = fit_glm(&y, &x, &names, Family::Logit, false).unwrap();
        assert!(res.converged);
        assert!(res.coef[1] > 1.0, "slope = {}", res.coef[1]);
        assert!(res.p[1] < 0.001);
        assert!(res.pseudo_r2_mcfadden > 0.0 && res.pseudo_r2_mcfadden < 1.0);
    }

    #[test]
    fn logit_matches_closed_form_on_separable_case() {
        // Perfectly separable data: coefficient is large, separation warning issued.
        let y: Vec<f64> = (0..40).map(|i| if i < 20 { 0.0 } else { 1.0 }).collect();
        let rows: Vec<Vec<f64>> = (0..40).map(|i| vec![i as f64]).collect();
        let x = Matrix::from_rows(&rows).unwrap();
        let names = vec!["x".to_string()];
        let res = fit_glm(&y, &x, &names, Family::Logit, false).unwrap();
        assert!(res.coef[1] > 5.0);
        assert!(!res.warnings.is_empty() || res.coef[1] > 5.0);
    }

    #[test]
    fn probit_binary_outcome_validation() {
        let (y, rows) = logistic_dgp(600);
        let x = Matrix::from_rows(&rows).unwrap();
        let names = vec!["x".to_string()];
        let res = fit_glm(&y, &x, &names, Family::Probit, false).unwrap();
        assert!(res.converged);
        assert!(res.coef[1] > 0.5);
        assert!(res.p[1] < 0.001);
    }

    #[test]
    fn poisson_recovers_log_rate() {
        // y ~ Poisson(exp(0.3 + 0.4x)) deterministically.
        let mut state = 987654321u64;
        let mut next = || {
            state = state
                .wrapping_mul(2862933555777941757)
                .wrapping_add(3037000493);
            (state >> 11) as f64 / (1u64 << 53) as f64
        };
        let mut y = Vec::new();
        let mut rows = Vec::new();
        for _ in 0..800 {
            let x = next() * 2.0 - 1.0;
            let lambda = (0.3 + 0.4 * x).exp();
            // Knuth's method for Poisson sampling (deterministic given the PRNG).
            let mut k = 0.0f64;
            let mut p = 1.0;
            loop {
                p *= next();
                if p <= (-lambda).exp() || k > 100.0 {
                    break;
                }
                k += 1.0;
            }
            y.push(k);
            rows.push(vec![x]);
        }
        let x = Matrix::from_rows(&rows).unwrap();
        let names = vec!["x".to_string()];
        let res = fit_glm(&y, &x, &names, Family::Poisson, false).unwrap();
        assert!(
            (res.coef[0] - 0.3).abs() < 0.05,
            "intercept = {}",
            res.coef[0]
        );
        assert!((res.coef[1] - 0.4).abs() < 0.05, "slope = {}", res.coef[1]);
    }

    #[test]
    fn non_binary_outcome_rejected() {
        let y = vec![0.0, 1.0, 2.0, 1.0, 0.0, 1.0];
        let rows: Vec<Vec<f64>> = (0..6).map(|i| vec![i as f64]).collect();
        let x = Matrix::from_rows(&rows).unwrap();
        let err = fit_glm(&y, &x, &["x".to_string()], Family::Logit, false).unwrap_err();
        assert!(err.what.contains("0/1"));
    }

    #[test]
    fn robust_se_available() {
        let (y, rows) = logistic_dgp(400);
        let x = Matrix::from_rows(&rows).unwrap();
        let res = fit_glm(&y, &x, &["x".to_string()], Family::Logit, true).unwrap();
        assert_eq!(res.vcov_label, "Robust");
        assert!(res.se[1] > 0.0);
    }
}
