//! # Instrumental variables (2SLS)
//!
//! Two-stage least squares with classical and robust covariance, first-stage
//! diagnostics for each endogenous regressor, and order-condition checking.

use crate::linalg::{Matrix, QR};
use crate::ols::leverage_diagonal;
use crate::vcov::{vcov, VcovSpec};
use numeris_core::dist;
use numeris_core::error::{NumerisError, Result};
use serde::{Deserialize, Serialize};

/// A fitted 2SLS model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IvResult {
    pub terms: Vec<String>,
    pub coef: Vec<f64>,
    pub se: Vec<f64>,
    pub z: Vec<f64>,
    pub p: Vec<f64>,
    pub ci_lo: Vec<f64>,
    pub ci_hi: Vec<f64>,
    pub n: usize,
    pub k: usize,
    pub rmse: f64,
    /// First-stage F statistic for each endogenous regressor.
    pub first_stage_f: Vec<f64>,
    pub first_stage_partial_r2: Vec<f64>,
    pub first_stage_terms: Vec<String>,
    pub residuals: Vec<f64>,
    pub vcov_label: String,
}

/// Fit 2SLS.
///
/// * `y`          outcome (n)
/// * `xendog`     endogenous regressors (n × e)
/// * `xexog`      included exogenous controls (n × c)
/// * `instruments` excluded instruments (n × z)
pub fn fit_2sls(
    y: &[f64],
    xendog: &Matrix,
    xexog: &Matrix,
    instruments: &Matrix,
    names: &[String],
    vcov_spec: &VcovSpec,
) -> Result<IvResult> {
    let n = y.len();
    if xendog.rows != n || xexog.rows != n || instruments.rows != n {
        return Err(NumerisError::numerical_failure(
            "Instrumental-variables inputs are misaligned.",
            "Internal error: outcome, regressors and instruments have different row counts.",
            "Please report this as a bug with the command that triggered it.",
        ));
    }
    let e = xendog.cols;
    let c = xexog.cols;
    let z = instruments.cols;
    if e == 0 {
        return Err(NumerisError::invalid_request(
            "The model specifies no endogenous regressors.",
            "ivregress requires at least one endogenous variable with instruments, in the form: ivregress 2sls y (endog = inst1 inst2) controls.",
            "Rewrite the command with an instrumented variable in parentheses.",
        ));
    }
    // Order condition: z >= e.
    if z < e {
        return Err(NumerisError::not_identified(
            "The model fails the order condition: there are fewer instruments than endogenous variables.",
            format!("There are {z} excluded instruments but {e} endogenous regressors; the parameters cannot be identified."),
            "Add at least as many instruments as endogenous variables, or drop an endogenous regressor.",
        ));
    }

    // Z = [instruments, exog controls, constant]
    let zfull = assemble_z(instruments, xexog);
    let zk = zfull.cols;
    if n <= zk {
        return Err(NumerisError::insufficient_data(
            format!("The estimation sample ({n} observations) is too small for {zk} instruments and controls."),
            "Instrumental-variables estimation requires more observations than first-stage parameters.",
            "Use a larger sample or fewer instruments.",
        ));
    }

    // Full design X = [endog, exog, constant].
    let mut x_rows: Vec<Vec<f64>> = Vec::with_capacity(n);
    for i in 0..n {
        let mut row = Vec::with_capacity(e + c + 1);
        row.extend_from_slice(xendog.row(i));
        row.extend_from_slice(xexog.row(i));
        row.push(1.0);
        x_rows.push(row);
    }
    let xfull = Matrix::from_rows(&x_rows)?;

    // X̂ = projection of X onto span(Z).
    let zqr = QR::decompose(&zfull)?;
    if !zqr.rank_deficient_columns().is_empty() {
        return Err(NumerisError::not_identified(
            "The instrument set is collinear with the included controls.",
            "Some instruments are exact linear combinations of the other instruments or controls, so the first stage cannot be estimated.",
            "Remove redundant instruments or controls and rerun.",
        ));
    }
    let mut xhat_rows: Vec<Vec<f64>> = Vec::with_capacity(n);
    for j in 0..xfull.cols {
        let col: Vec<f64> = (0..n).map(|i| xfull.get(i, j)).collect();
        let gamma = zqr.solve(&col);
        let fitted = zfull.mul_vec(&gamma);
        for (i, &f) in fitted.iter().enumerate() {
            if i == xhat_rows.len() {
                xhat_rows.push(Vec::with_capacity(xfull.cols));
            }
            xhat_rows[i].push(f);
        }
    }
    let xhat = Matrix::from_rows(&xhat_rows)?;

    // Second stage: OLS of y on X̂ (no extra constant — it is inside X̂).
    let qr = QR::decompose(&xhat)?;
    let deficient = qr.rank_deficient_columns();
    if !deficient.is_empty() {
        return Err(NumerisError::not_identified(
            "The structural equation is not identified.",
            "The fitted values of the regressors are collinear, which happens when instruments are weak or collinear with controls.",
            "Inspect the first-stage F statistics; if they are small, the instruments are too weak for this specification.",
        ));
    }
    let beta = qr.solve(y);
    let residuals: Vec<f64> = y
        .iter()
        .zip(xfull.mul_vec(&beta).iter())
        .map(|(a, b)| a - b)
        .collect();
    let xtx_inv = qr.xtx_inv();
    let k = xfull.cols;

    // Robust covariance uses X̂ in the bread (standard 2SLS sandwich).
    // HC-style meat with X̂ columns.
    let mut se;
    let vcov_label;
    match vcov_spec {
        VcovSpec::Classical => {
            let df_r = n.saturating_sub(zk);
            if df_r == 0 {
                return Err(NumerisError::insufficient_data(
                    "The model has no residual degrees of freedom.",
                    format!("With {n} observations and {zk} first-stage parameters there are no degrees of freedom left."),
                    "Use a larger sample or fewer instruments.",
                ));
            }
            let sigma2 = residuals.iter().map(|u| u * u).sum::<f64>() / df_r as f64;
            let mut v = xtx_inv.clone();
            v.scale_mut(sigma2);
            se = diag_sqrt(&v);
            vcov_label = "Classical";
        }
        _ => {
            let lev = leverage_diagonal(&xhat, &xtx_inv);
            let v = vcov(&xhat, &residuals, &xtx_inv, &lev, None, vcov_spec)?;
            se = diag_sqrt(&v);
            vcov_label = vcov_spec.label();
        }
    }

    // First-stage diagnostics per endogenous regressor.
    let mut fs_f = Vec::with_capacity(e);
    let mut fs_r2 = Vec::with_capacity(e);
    for j in 0..e {
        let col: Vec<f64> = (0..n).map(|i| xendog.get(i, j)).collect();
        let (f, pr2) = first_stage_stats(&col, &zfull, c)?;
        fs_f.push(f);
        fs_r2.push(pr2);
    }

    let df_r = (n - zk).max(1);
    let tcrit = dist::t_quantile(0.975, df_r as f64)?;
    let mut zs = Vec::with_capacity(k);
    let mut ps = Vec::with_capacity(k);
    let mut lo = Vec::with_capacity(k);
    let mut hi = Vec::with_capacity(k);
    for j in 0..k {
        let zval = beta[j] / se[j];
        zs.push(zval);
        ps.push(dist::t_two_sided_p(zval, df_r as f64));
        lo.push(beta[j] - tcrit * se[j]);
        hi.push(beta[j] + tcrit * se[j]);
    }

    let mut terms: Vec<String> = names.to_vec();
    terms.push("_cons".to_string());

    let rmse = (residuals.iter().map(|u| u * u).sum::<f64>() / n as f64).sqrt();

    Ok(IvResult {
        terms,
        coef: beta,
        se: std::mem::take(&mut se),
        z: zs,
        p: ps,
        ci_lo: lo,
        ci_hi: hi,
        n,
        k,
        rmse,
        first_stage_f: fs_f,
        first_stage_partial_r2: fs_r2,
        first_stage_terms: (0..e).map(|j| names[j].clone()).collect(),
        residuals,
        vcov_label: vcov_label.to_string(),
    })
}

fn diag_sqrt(v: &Matrix) -> Vec<f64> {
    (0..v.rows).map(|i| v.get(i, i).max(0.0).sqrt()).collect()
}

/// First-stage F (excluded instruments) and partial R² for one endogenous
/// regressor against the full instrument matrix Z (which includes `c` controls).
fn first_stage_stats(endog: &[f64], zfull: &Matrix, c: usize) -> Result<(f64, f64)> {
    let n = endog.len();
    let q_full = QR::decompose(zfull)?;
    let gamma_full = q_full.solve(endog);
    let fit_full = zfull.mul_vec(&gamma_full);
    let ssr_full: f64 = endog
        .iter()
        .zip(&fit_full)
        .map(|(a, b)| (a - b) * (a - b))
        .sum();
    // Restricted: controls + constant only = first c+1 columns of zfull.
    let z_restricted = Matrix::from_rows(
        &(0..n)
            .map(|i| zfull.row(i)[..(c + 1)].to_vec())
            .collect::<Vec<_>>(),
    )?;
    let q_r = QR::decompose(&z_restricted)?;
    let gamma_r = q_r.solve(endog);
    let fit_r = z_restricted.mul_vec(&gamma_r);
    let ssr_r: f64 = endog
        .iter()
        .zip(&fit_r)
        .map(|(a, b)| (a - b) * (a - b))
        .sum();
    let df1 = zfull.cols - z_restricted.cols;
    let df2 = n - zfull.cols;
    if df2 == 0 || ssr_r <= 0.0 {
        return Ok((f64::NAN, f64::NAN));
    }
    let f = ((ssr_r - ssr_full) / df1 as f64) / (ssr_full / df2 as f64);
    // Partial R² of excluded instruments.
    let partial_r2 = (ssr_r - ssr_full) / ssr_r;
    Ok((f, partial_r2))
}

fn assemble_z(instruments: &Matrix, xexog: &Matrix) -> Matrix {
    let n = instruments.rows;
    let cols = instruments.cols + xexog.cols + 1;
    let mut z = Matrix::zeros(n, cols);
    for i in 0..n {
        let mut col = 0;
        for j in 0..instruments.cols {
            z.data[i * cols + col] = instruments.get(i, j);
            col += 1;
        }
        for j in 0..xexog.cols {
            z.data[i * cols + col] = xexog.get(i, j);
            col += 1;
        }
        z.data[i * cols + col] = 1.0;
    }
    z
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovers_known_dgp_with_valid_instruments() {
        // DGP: z -> x (strong), x -> y with coefficient 2, e independent.
        let n = 400;
        let mut z1 = Vec::new();
        let mut x = Vec::new();
        let mut y = Vec::new();
        let mut idx = 0u64;
        for i in 0..n {
            let zi = ((i * 37) % 100) as f64 / 10.0; // quasi-random
            let u1 = (((idx.wrapping_mul(6364136223846793005).wrapping_add(1)) >> 33) % 100) as f64
                / 50.0
                - 1.0;
            let u2 = (((idx
                .wrapping_mul(2862933555777941757)
                .wrapping_add(4)
                .wrapping_add(idx))
                >> 33)
                % 100) as f64
                / 50.0
                - 1.0;
            idx = idx.wrapping_add(1);
            let xi = 0.5 + 0.9 * zi + 0.25 * u1;
            let yi = 1.0 + 2.0 * xi + 0.5 * u2;
            z1.push(zi);
            x.push(xi);
            y.push(yi);
        }
        let xmat = Matrix::from_rows(&x.iter().map(|&v| vec![v]).collect::<Vec<_>>()).unwrap();
        let zmat = Matrix::from_rows(&z1.iter().map(|&v| vec![v]).collect::<Vec<_>>()).unwrap();
        let empty_rows: Vec<Vec<f64>> = (0..n).map(|_| vec![]).collect();
        let empty = Matrix::from_rows(&empty_rows).unwrap();
        let names = vec!["x".to_string()];
        let res = fit_2sls(&y, &xmat, &empty, &zmat, &names, &VcovSpec::Classical).unwrap();
        // The 2SLS estimate should be close to 2 with a strong instrument.
        assert!((res.coef[0] - 2.0).abs() < 0.6, "coef = {}", res.coef[0]);
        assert!(res.first_stage_f[0] > 30.0, "F = {}", res.first_stage_f[0]);
    }

    #[test]
    fn order_condition_violation_is_reported() {
        let y: Vec<f64> = (0..50).map(|i| i as f64).collect();
        // Two endogenous, one instrument.
        let rows_x: Vec<Vec<f64>> = (0..50).map(|i| vec![i as f64, (i * 2) as f64]).collect();
        let rows_z: Vec<Vec<f64>> = (0..50).map(|i| vec![(i * 3) as f64 % 17.0]).collect();
        let xmat = Matrix::from_rows(&rows_x).unwrap();
        let zmat = Matrix::from_rows(&rows_z).unwrap();
        let empty_rows: Vec<Vec<f64>> = (0..50).map(|_| vec![]).collect();
        let empty = Matrix::from_rows(&empty_rows).unwrap();
        let names = vec!["a".to_string(), "b".to_string()];
        let err = fit_2sls(&y, &xmat, &empty, &zmat, &names, &VcovSpec::Classical).unwrap_err();
        assert!(err.what.contains("order condition"));
    }

    #[test]
    fn no_endogenous_variables_is_an_error() {
        let y: Vec<f64> = (0..30).map(|i| i as f64).collect();
        let empty_rows: Vec<Vec<f64>> = (0..30).map(|_| vec![]).collect();
        let empty = Matrix::from_rows(&empty_rows).unwrap();
        let rows_z: Vec<Vec<f64>> = (0..30).map(|i| vec![i as f64 % 7.0]).collect();
        let zmat = Matrix::from_rows(&rows_z).unwrap();
        let err = fit_2sls(&y, &empty, &empty, &zmat, &[], &VcovSpec::Classical).unwrap_err();
        assert!(err.what.contains("no endogenous regressors"));
    }
}
