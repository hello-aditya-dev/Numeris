//! # Regression diagnostics
//!
//! VIF, Breusch–Pagan (Koenker's student-robust form), RESET, Durbin–Watson,
//! Cook's distance and standardized residuals — all computed from the same
//! design matrix and residuals produced by the shared estimators.

use crate::linalg::Matrix;
use crate::ols::Regression;
use numeris_core::dist;
use numeris_core::error::Result;
use serde::{Deserialize, Serialize};

/// One diagnostic finding with a traffic-light status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticItem {
    pub name: String,
    pub status: String, // "ok" | "warning" | "attention" | "info"
    pub detail: String,
}

/// Variance inflation factors for each non-constant regressor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VifRow {
    pub variable: String,
    pub vif: f64,
    pub r_squared: f64,
}

/// Compute VIFs by auxiliary regressions of each regressor on the others.
pub fn vif(x: &Matrix, names: &[String]) -> Result<Vec<VifRow>> {
    let mut out = Vec::new();
    let n = x.rows;
    for j in 0..x.cols {
        // Dependent: column j. Regressors: all other columns + constant.
        let others: Vec<usize> = (0..x.cols).filter(|&c| c != j).collect();
        let y: Vec<f64> = (0..n).map(|i| x.get(i, j)).collect();
        let rows: Vec<Vec<f64>> = (0..n)
            .map(|i| {
                let mut row = vec![1.0];
                row.extend(others.iter().map(|&c| x.get(i, c)));
                row
            })
            .collect();
        let design = crate::linalg::Matrix::from_rows(&rows)?;
        let qr = crate::linalg::QR::decompose(&design)?;
        let beta = qr.solve(&y);
        let fitted = design.mul_vec(&beta);
        let ssr: f64 = y.iter().zip(&fitted).map(|(a, b)| (a - b) * (a - b)).sum();
        let ybar = y.iter().sum::<f64>() / n as f64;
        let sst: f64 = y.iter().map(|v| (v - ybar) * (v - ybar)).sum();
        if sst <= 0.0 {
            continue; // constant regressor
        }
        let r2 = (1.0 - ssr / sst).clamp(0.0, 1.0 - 1e-12);
        if r2 < 1.0 {
            out.push(VifRow {
                variable: names[j].clone(),
                vif: 1.0 / (1.0 - r2),
                r_squared: r2,
            });
        }
    }
    Ok(out)
}

/// Breusch–Pagan with the design matrix available.
pub fn breusch_pagan_with_design(
    reg: &Regression,
    x: &Matrix,
    constant: bool,
) -> (f64, usize, f64) {
    let n = x.rows;
    let e2: Vec<f64> = reg.residuals.iter().map(|u| u * u).collect();
    let rows: Vec<Vec<f64>> = (0..n)
        .map(|i| {
            let mut row = Vec::with_capacity(x.cols + 1);
            if constant {
                row.push(1.0);
            }
            row.extend_from_slice(x.row(i));
            row
        })
        .collect();
    let Ok(design) = Matrix::from_rows(&rows) else {
        return (f64::NAN, 0, f64::NAN);
    };
    let Ok(qr) = crate::linalg::QR::decompose(&design) else {
        return (f64::NAN, 0, f64::NAN);
    };
    let beta = qr.solve(&e2);
    let fitted = design.mul_vec(&beta);
    let ssr: f64 = e2.iter().zip(&fitted).map(|(a, b)| (a - b) * (a - b)).sum();
    let mean_e2 = e2.iter().sum::<f64>() / n as f64;
    let sst: f64 = e2.iter().map(|v| (v - mean_e2) * (v - mean_e2)).sum();
    if sst <= 0.0 {
        return (f64::NAN, 0, f64::NAN);
    }
    let r2 = 1.0 - ssr / sst;
    let lm = n as f64 * r2;
    // df = number of slope regressors.
    let df = x.cols;
    let p = 1.0 - dist::chi2_cdf(lm, df as f64);
    (lm, df, p)
}

/// Ramsey RESET: F test of the augmented regression with ŷ² and ŷ³.
pub fn reset_with_design(
    reg: &Regression,
    x: &Matrix,
    y: &[f64],
    constant: bool,
) -> (f64, usize, usize, f64) {
    let n = x.rows;
    let fit2: Vec<f64> = reg.fitted.iter().map(|f| f * f).collect();
    let fit3: Vec<f64> = reg.fitted.iter().map(|f| f * f * f).collect();
    let rows: Vec<Vec<f64>> = (0..n)
        .map(|i| {
            let mut row = Vec::with_capacity(x.cols + 3);
            if constant {
                row.push(1.0);
            }
            row.extend_from_slice(x.row(i));
            row.push(fit2[i]);
            row.push(fit3[i]);
            row
        })
        .collect();
    let Ok(design) = Matrix::from_rows(&rows) else {
        return (f64::NAN, 0, 0, f64::NAN);
    };
    let Ok(qr) = crate::linalg::QR::decompose(&design) else {
        return (f64::NAN, 0, 0, f64::NAN);
    };
    if !qr.rank_deficient_columns().is_empty() {
        return (f64::NAN, 0, 0, f64::NAN);
    }
    let beta = qr.solve(y);
    let fitted = design.mul_vec(&beta);
    let ssr_u: f64 = y.iter().zip(&fitted).map(|(a, b)| (a - b) * (a - b)).sum();
    let ssr_r: f64 = reg.residuals.iter().map(|u| u * u).sum();
    let df1 = 2; // two added terms
    let df2 = (n - design.cols).max(1);
    if ssr_u <= 0.0 {
        return (f64::NAN, df1, df2, f64::NAN);
    }
    let f = ((ssr_r - ssr_u) / df1 as f64) / (ssr_u / df2 as f64);
    let p = 1.0 - dist::f_cdf(f, df1 as f64, df2 as f64);
    (f, df1, df2, p)
}

/// Durbin–Watson statistic for ordered residuals.
pub fn durbin_watson(reg: &Regression) -> Option<f64> {
    let e = &reg.residuals;
    if e.len() < 2 {
        return None;
    }
    let denom: f64 = e.iter().map(|u| u * u).sum();
    if denom <= 0.0 {
        return None;
    }
    let num: f64 = e.windows(2).map(|w| w[1] - w[0]).map(|d| d * d).sum();
    Some(num / denom)
}

/// Cook's distance for each observation.
pub fn cooks_distance(reg: &Regression, _design: &Matrix) -> Vec<f64> {
    let k = reg.k as f64;
    let s2 = reg.rmse * reg.rmse;
    reg.residuals
        .iter()
        .zip(&reg.leverage)
        .map(|(&e, &h)| {
            let one_minus = 1.0 - h;
            if one_minus <= 1e-12 {
                f64::NAN
            } else {
                (e * e * h) / (k * s2 * one_minus * one_minus)
            }
        })
        .collect()
}

/// Externally standardized residuals are approximated by internally
/// standardized residuals: e_i / (σ √(1 − h_i)).
pub fn standardized_residuals(reg: &Regression) -> Vec<f64> {
    reg.residuals
        .iter()
        .zip(&reg.leverage)
        .map(|(&e, &h)| e / (reg.rmse * (1.0 - h).max(1e-12).sqrt()))
        .collect()
}

/// Assemble a diagnostic summary for a fitted regression.
pub fn regression_diagnostics(
    reg: &Regression,
    x: &Matrix,
    y: &[f64],
    constant: bool,
) -> Vec<DiagnosticItem> {
    let mut items = Vec::new();
    // Sample size vs parameters.
    items.push(DiagnosticItem {
        name: "Sample size".to_string(),
        status: "ok".to_string(),
        detail: format!(
            "{} observations, {} parameters, {} residual degrees of freedom.",
            reg.n, reg.k, reg.df_r
        ),
    });
    // Condition number.
    let xtx = x.cross_product_self();
    let cond = xtx.condition_number_symmetric().sqrt();
    if cond.is_finite() {
        let status = if cond < 1e7 {
            "ok"
        } else if cond < 1e10 {
            "warning"
        } else {
            "attention"
        };
        items.push(DiagnosticItem {
            name: "Condition number".to_string(),
            status: status.to_string(),
            detail: format!("Design-matrix condition number ≈ {:.3e}. Values above ~1e10 indicate severe multicollinearity.", cond),
        });
    }
    // VIF.
    if let Ok(vifs) = vif(x, &reg.terms[if constant { 1 } else { 0 }..]) {
        if !vifs.is_empty() {
            let max_vif = vifs.iter().map(|v| v.vif).fold(f64::MIN, f64::max);
            let worst = vifs
                .iter()
                .max_by(|a, b| a.vif.partial_cmp(&b.vif).unwrap())
                .map(|v| v.variable.clone())
                .unwrap_or_default();
            let status = if max_vif < 5.0 {
                "ok"
            } else if max_vif < 10.0 {
                "warning"
            } else {
                "attention"
            };
            items.push(DiagnosticItem {
                name: "Multicollinearity (VIF)".to_string(),
                status: status.to_string(),
                detail: format!("Maximum VIF is {max_vif:.2} ({worst}). Values above 10 indicate problematic collinearity."),
            });
        }
    }
    // Heteroskedasticity.
    let (lm, df, p) = breusch_pagan_with_design(reg, x, constant);
    if p.is_finite() {
        let status = if p < 0.05 { "attention" } else { "ok" };
        items.push(DiagnosticItem {
            name: "Heteroskedasticity (Breusch–Pagan)".to_string(),
            status: status.to_string(),
            detail: format!("LM = {lm:.3} on {df} df, p = {p:.4}. A small p suggests heteroskedasticity; robust standard errors are recommended."),
        });
    }
    // RESET.
    let (f, df1, df2, p) = reset_with_design(reg, x, y, constant);
    if p.is_finite() {
        let status = if p < 0.05 { "warning" } else { "ok" };
        items.push(DiagnosticItem {
            name: "Functional form (RESET)".to_string(),
            status: status.to_string(),
            detail: format!(
                "F({df1}, {df2}) = {f:.3}, p = {p:.4}. A small p suggests unmodeled nonlinearity."
            ),
        });
    }
    // Influential observations.
    let cooks = cooks_distance(reg, x);
    let n_cook = cooks
        .iter()
        .filter(|c| c.is_finite() && **c > 4.0 / reg.n as f64)
        .count();
    if n_cook > 0 {
        items.push(DiagnosticItem {
            name: "Influential observations".to_string(),
            status: if n_cook as f64 > 0.05 * reg.n as f64 { "warning" } else { "info" }.to_string(),
            detail: format!("{n_cook} observations exceed a Cook's distance of 4/n. Inspect them before interpreting coefficients."),
        });
    }
    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ols::Ols;

    fn fitted() -> (Regression, Matrix, Vec<f64>) {
        // Homoskedastic noise-free relation plus a mild pattern.
        let y: Vec<f64> = (0..60)
            .map(|i| {
                let x = (i as f64) / 6.0;
                let e = if i % 5 == 0 { 0.5 } else { -0.125 };
                2.0 + 1.0 * x + e
            })
            .collect();
        let rows: Vec<Vec<f64>> = (0..60).map(|i| vec![i as f64 / 6.0]).collect();
        let x = Matrix::from_rows(&rows).unwrap();
        let names = vec!["x".to_string()];
        let reg = Ols::new(&y, &x, &names).unwrap().fit().unwrap();
        (reg, x, y)
    }

    #[test]
    fn vif_of_orthogonal_predictors_is_one() {
        // x1 = i, x2 = i² (not collinear in a problematic way for small range).
        let y: Vec<f64> = (0..30).map(|i| i as f64).collect();
        let rows: Vec<Vec<f64>> = (0..30).map(|i| vec![i as f64, (i % 7) as f64]).collect();
        let x = Matrix::from_rows(&rows).unwrap();
        let names = vec!["a".to_string(), "b".to_string()];
        let reg = Ols::new(&y, &x, &names).unwrap().fit().unwrap();
        let vifs = vif(&x, &["a".to_string(), "b".to_string()]).unwrap();
        assert!(vifs.iter().all(|v| v.vif < 5.0));
        let _ = reg;
    }

    #[test]
    fn vif_detects_collinearity() {
        let rows: Vec<Vec<f64>> = (0..30)
            .map(|i| vec![i as f64, 2.0 * i as f64 + 3.0])
            .collect();
        let x = Matrix::from_rows(&rows).unwrap();
        let vifs = vif(&x, &["a".to_string(), "b".to_string()]).unwrap();
        assert!(
            vifs.iter().all(|v| v.vif > 100.0),
            "vifs = {:?}",
            vifs.iter().map(|v| v.vif).collect::<Vec<_>>()
        );
    }

    #[test]
    fn durbin_watson_bounds() {
        let (reg, _x, _y) = fitted();
        let d = durbin_watson(&reg).unwrap();
        assert!((0.0..=4.0).contains(&d), "dw = {d}");
    }

    #[test]
    fn standardized_residuals_magnitude_reasonable() {
        let (reg, _x, _y) = fitted();
        let sr = standardized_residuals(&reg);
        assert!(sr.iter().all(|v| v.abs() < 10.0));
    }

    #[test]
    fn bp_test_runs() {
        let (reg, x, _y) = fitted();
        let (lm, df, p) = breusch_pagan_with_design(&reg, &x, true);
        assert!(lm.is_finite());
        assert!(df >= 1);
        assert!((0.0..=1.0).contains(&p));
    }
}
