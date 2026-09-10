//! # Panel data estimators
//!
//! Fixed effects (within), between, and pooled OLS for panel data.
//! Singletons are dropped with a report count; degrees of freedom follow
//! the standard within-estimator accounting: df = n − g − k.

use crate::linalg::{Matrix, QR};
use crate::ols::leverage_diagonal;
use crate::vcov::{vcov, ClusterId, VcovSpec};
use numeris_core::dist;
use numeris_core::error::{NumerisError, Result};
use serde::{Deserialize, Serialize};

/// A fitted panel model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelResult {
    pub estimator: String,
    pub terms: Vec<String>,
    pub coef: Vec<f64>,
    pub se: Vec<f64>,
    pub t: Vec<f64>,
    pub p: Vec<f64>,
    pub ci_lo: Vec<f64>,
    pub ci_hi: Vec<f64>,
    /// Rows used after dropping singletons.
    pub n: usize,
    /// Entities used.
    pub g: usize,
    /// Singleton entities dropped (FE only).
    pub dropped_singletons: usize,
    pub k: usize,
    pub df_r: usize,
    pub r2_within: f64,
    pub sigma: f64,
    pub vcov_label: String,
}

/// Fixed effects (within) estimator.
/// `cluster_ids` supplies row-level cluster identifiers when a clustered
/// covariance is requested (falls back to entity ids when absent).
pub fn fixed_effects(
    y: &[f64],
    xs: &Matrix,
    entity: &[ClusterId],
    names: &[String],
    vcov_spec: &VcovSpec,
    cluster_ids: Option<&[ClusterId]>,
) -> Result<PanelResult> {
    let n = y.len();
    if entity.len() != n || xs.rows != n {
        return Err(NumerisError::numerical_failure(
            "Panel inputs are misaligned.",
            "Internal error: outcome, regressors and entity identifiers have different lengths.",
            "Please report this as a bug with the command that triggered it.",
        ));
    }
    let k = xs.cols;
    if k == 0 {
        return Err(NumerisError::invalid_request(
            "The fixed-effects model has no predictors.",
            "xtreg requires at least one regressor besides the entity identifier.",
            "Add a predictor to the model.",
        ));
    }
    // Group rows by entity.
    let mut groups: std::collections::BTreeMap<ClusterId, Vec<usize>> =
        std::collections::BTreeMap::new();
    for (i, &e) in entity.iter().enumerate() {
        groups.entry(e).or_default().push(i);
    }
    // Drop singletons: within variation is zero for them.
    let dropped = groups.values().filter(|v| v.len() < 2).count();
    let used_groups = groups.len() - dropped;
    if used_groups < 2 {
        return Err(NumerisError::insufficient_data(
            "The fixed-effects estimator requires at least two entities with two or more observations each.",
            format!("After dropping singleton entities, {used_groups} usable entities remain."),
            "Check the entity variable; xtreg, fe needs repeated observations within entities.",
        ));
    }
    let mut y_dm = Vec::new();
    let mut x_dm_rows: Vec<Vec<f64>> = Vec::new();
    let mut entity_used: Vec<ClusterId> = Vec::new();
    for (e, rows) in groups.iter() {
        if rows.len() < 2 {
            continue;
        }
        let ybar: f64 = rows.iter().map(|&i| y[i]).sum::<f64>() / rows.len() as f64;
        let xbars: Vec<f64> = (0..k)
            .map(|j| rows.iter().map(|&i| xs.get(i, j)).sum::<f64>() / rows.len() as f64)
            .collect();
        for &i in rows {
            y_dm.push(y[i] - ybar);
            let mut row = Vec::with_capacity(k);
            for j in 0..k {
                row.push(xs.get(i, j) - xbars[j]);
            }
            x_dm_rows.push(row);
            entity_used.push(*e);
        }
    }
    let n_used = y_dm.len();
    // Checked subtraction: n_used < used_groups + k would underflow.
    let df_r = match n_used.checked_sub(used_groups + k) {
        Some(df) if df > 0 => df,
        _ => {
            return Err(NumerisError::insufficient_data(
                "The fixed-effects model has no residual degrees of freedom.",
                format!("With {n_used} observations, {used_groups} entities and {k} regressors, no degrees of freedom remain."),
                "Use a larger panel or fewer regressors.",
            ));
        }
    };
    let xdm = Matrix::from_rows(&x_dm_rows)?;
    let qr = QR::decompose(&xdm)?;
    let deficient = qr.rank_deficient_columns();
    if !deficient.is_empty() {
        let bad: Vec<String> = deficient.iter().map(|&i| names[i].clone()).collect();
        return Err(NumerisError::not_identified(
            "The fixed-effects model is not identified: time-invariant regressors are absorbed by the entity effects.",
            format!("After within-transformation, these regressors have no variation: {}.", bad.join(", ")),
            "Remove time-invariant regressors from the model; their effects cannot be separated from the entity fixed effects.",
        ));
    }
    let beta = qr.solve(&y_dm);
    let fitted = xdm.mul_vec(&beta);
    let residuals: Vec<f64> = y_dm.iter().zip(&fitted).map(|(a, b)| a - b).collect();
    let ssr: f64 = residuals.iter().map(|u| u * u).sum();
    let sst: f64 = y_dm.iter().map(|v| v * v).sum();
    let r2_within = if sst > 0.0 {
        (1.0 - ssr / sst).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let sigma2 = ssr / df_r as f64;

    // Covariance: classical uses σ²; robust/cluster use the sandwich on
    // the demeaned design with the entity-level df already accounted.
    let xtx_inv = qr.xtx_inv();
    let lev = leverage_diagonal(&xdm, &xtx_inv);
    let v = match vcov_spec {
        VcovSpec::Classical => {
            let mut m = xtx_inv.clone();
            m.scale_mut(sigma2);
            m
        }
        VcovSpec::Cluster => {
            let ids = cluster_ids.unwrap_or(&entity_used);
            vcov(&xdm, &residuals, &xtx_inv, &lev, Some(ids), vcov_spec)?
        }
        other => vcov(&xdm, &residuals, &xtx_inv, &lev, None, other)?,
    };

    let tcrit = dist::t_quantile(0.975, df_r as f64)?;
    let mut se = Vec::with_capacity(k);
    let mut tstat = Vec::with_capacity(k);
    let mut pval = Vec::with_capacity(k);
    let mut lo = Vec::with_capacity(k);
    let mut hi = Vec::with_capacity(k);
    for j in 0..k {
        let s = v.get(j, j).max(0.0).sqrt();
        let t = if s > 0.0 { beta[j] / s } else { f64::NAN };
        se.push(s);
        tstat.push(t);
        pval.push(if s > 0.0 {
            dist::t_two_sided_p(t, df_r as f64)
        } else {
            f64::NAN
        });
        lo.push(beta[j] - tcrit * s);
        hi.push(beta[j] + tcrit * s);
    }

    Ok(PanelResult {
        estimator: "Fixed effects (within)".to_string(),
        terms: names.to_vec(),
        coef: beta,
        se,
        t: tstat,
        p: pval,
        ci_lo: lo,
        ci_hi: hi,
        n: n_used,
        g: used_groups,
        dropped_singletons: dropped,
        k,
        df_r,
        r2_within,
        sigma: sigma2.sqrt(),
        vcov_label: vcov_spec.label().to_string(),
    })
}

/// Between estimator: OLS on entity means.
pub fn between(
    y: &[f64],
    xs: &Matrix,
    entity: &[ClusterId],
    names: &[String],
    vcov_spec: &VcovSpec,
    cluster_ids: Option<&[ClusterId]>,
) -> Result<PanelResult> {
    let n = y.len();
    if entity.len() != n || xs.rows != n {
        return Err(NumerisError::numerical_failure(
            "Panel inputs are misaligned.",
            "Internal error in the between estimator.",
            "Please report this as a bug with the command that triggered it.",
        ));
    }
    let k = xs.cols;
    let mut groups: std::collections::BTreeMap<ClusterId, Vec<usize>> =
        std::collections::BTreeMap::new();
    for (i, &e) in entity.iter().enumerate() {
        groups.entry(e).or_default().push(i);
    }
    let g = groups.len();
    if g < k + 2 {
        return Err(NumerisError::insufficient_data(
            "The between estimator has too few entities for the model.",
            format!("{g} entities cannot identify {k} regressors plus a constant."),
            "Use a panel with more entities, or fewer regressors.",
        ));
    }
    let mut ym = Vec::with_capacity(g);
    let mut xm: Vec<Vec<f64>> = Vec::with_capacity(g);
    for rows in groups.values() {
        let m = rows.len() as f64;
        ym.push(rows.iter().map(|&i| y[i]).sum::<f64>() / m);
        let row: Vec<f64> = (0..k)
            .map(|j| rows.iter().map(|&i| xs.get(i, j)).sum::<f64>() / m)
            .collect();
        xm.push(row);
    }
    let xmat = Matrix::from_rows(&xm)?;
    // OLS with constant via the shared engine.
    let mut builder = crate::ols::Ols::new(&ym, &xmat, names)?.estimator_label("Between");
    if let VcovSpec::Cluster = vcov_spec {
        // Cluster ids collapse to one per entity in the between transform.
        if let Some(ids) = cluster_ids {
            let mut by_entity: std::collections::BTreeMap<ClusterId, ClusterId> =
                std::collections::BTreeMap::new();
            let collapsed: Vec<ClusterId> = {
                let mut out = Vec::with_capacity(ids.len());
                let mut mapping: std::collections::BTreeMap<ClusterId, ClusterId> =
                    std::collections::BTreeMap::new();
                for &id in ids {
                    let next = mapping.len() as ClusterId;
                    out.push(*mapping.entry(id).or_insert(next));
                }
                out
            };
            for (i, &e) in entity.iter().enumerate() {
                by_entity.insert(e, collapsed[i]);
            }
            let group_ids: Vec<ClusterId> = groups
                .keys()
                .map(|e| by_entity.get(e).copied().unwrap_or(0))
                .collect();
            builder = builder.cluster(group_ids);
        }
    } else {
        builder = builder.vcov(vcov_spec.clone());
    }
    let ols = builder.fit()?;
    let df_r = (g - k - 1).max(1);
    let tcrit = dist::t_quantile(0.975, df_r as f64)?;
    let mut lo = Vec::with_capacity(k);
    let mut hi = Vec::with_capacity(k);
    for j in 0..k {
        lo.push(ols.coef[j + 1] - tcrit * ols.se[j + 1]);
        hi.push(ols.coef[j + 1] + tcrit * ols.se[j + 1]);
    }
    Ok(PanelResult {
        estimator: "Between".to_string(),
        terms: names.to_vec(),
        coef: ols.coef[1..].to_vec(),
        se: ols.se[1..].to_vec(),
        t: ols.t[1..].to_vec(),
        p: ols.p[1..].to_vec(),
        ci_lo: lo,
        ci_hi: hi,
        n: g,
        g,
        dropped_singletons: 0,
        k,
        df_r,
        r2_within: ols.r2,
        sigma: ols.rmse,
        vcov_label: ols.vcov_label,
    })
}

/// Difference-in-differences result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DidResult {
    /// The interaction coefficient (ATT in the 2×2 design).
    pub att: f64,
    pub att_se: f64,
    pub t: f64,
    pub p: f64,
    pub ci_lo: f64,
    pub ci_hi: f64,
    /// Cell means: [(control,pre), (control,post), (treated,pre), (treated,post)].
    pub means: [(f64, usize); 4],
    pub n: usize,
    pub regression: crate::ols::Regression,
    /// Full coefficient table of the underlying regression.
    pub terms: Vec<String>,
    pub coef: Vec<f64>,
    pub se: Vec<f64>,
    pub p_values: Vec<f64>,
}

/// Difference-in-differences via interaction regression:
/// y = β0 + β1·treat + β2·post + β3·(treat·post) + controls + ε.
///
/// `treat` and `post` must be 0/1 variables.
pub fn did(
    y: &[f64],
    treat: &[f64],
    post: &[f64],
    controls: &Matrix,
    control_names: &[String],
    vcov_spec: &VcovSpec,
    clusters: Option<Vec<ClusterId>>,
) -> Result<DidResult> {
    let n = y.len();
    if treat.len() != n || post.len() != n || controls.rows != n {
        return Err(NumerisError::numerical_failure(
            "Difference-in-differences inputs are misaligned.",
            "Internal error in DID assembly.",
            "Please report this as a bug with the command that triggered it.",
        ));
    }
    for (i, &t) in treat.iter().enumerate() {
        if t != 0.0 && t != 1.0 {
            return Err(NumerisError::invalid_request(
                format!(
                    "The treat variable must be 0/1; value {t} was found at observation {}.",
                    i + 1
                ),
                "DID compares a treated group (1) against a control group (0).",
                "Recode the treatment indicator to 0/1 before running DID.",
            ));
        }
        if post[i] != 0.0 && post[i] != 1.0 {
            return Err(NumerisError::invalid_request(
                format!(
                    "The time variable must be 0/1; value {} was found at observation {}.",
                    post[i],
                    i + 1
                ),
                "DID compares periods before (0) and after (1) the treatment.",
                "Recode the post indicator to 0/1 before running DID.",
            ));
        }
    }
    let kc = controls.cols;
    let mut rows: Vec<Vec<f64>> = Vec::with_capacity(n);
    let mut names: Vec<String> = Vec::with_capacity(2 + kc);
    names.push("treat".to_string());
    names.push("post".to_string());
    for i in 0..n {
        let mut row = vec![treat[i], post[i], treat[i] * post[i]];
        row.extend_from_slice(controls.row(i));
        rows.push(row);
    }
    names.push("treat#post".to_string());
    names.extend(control_names.iter().cloned());
    let x = Matrix::from_rows(&rows)?;

    let mut builder = crate::ols::Ols::new(y, &x, &names)?.vcov(vcov_spec.clone());
    if let Some(c) = clusters {
        builder = builder.cluster(c);
    }
    let reg = builder.estimator_label("DID").fit()?;

    // Cell means for the 2×2 summary.
    let mut sums = [0.0f64; 4];
    let mut counts = [0usize; 4];
    for i in 0..n {
        let cell = (treat[i] as usize) * 2 + post[i] as usize;
        sums[cell] += y[i];
        counts[cell] += 1;
    }
    let means = [
        (
            if counts[0] > 0 {
                sums[0] / counts[0] as f64
            } else {
                f64::NAN
            },
            counts[0],
        ),
        (
            if counts[1] > 0 {
                sums[1] / counts[1] as f64
            } else {
                f64::NAN
            },
            counts[1],
        ),
        (
            if counts[2] > 0 {
                sums[2] / counts[2] as f64
            } else {
                f64::NAN
            },
            counts[2],
        ),
        (
            if counts[3] > 0 {
                sums[3] / counts[3] as f64
            } else {
                f64::NAN
            },
            counts[3],
        ),
    ];
    for (j, &(_m, c)) in means.iter().enumerate() {
        if c == 0 {
            return Err(NumerisError::insufficient_data(
                "One of the four treatment-by-period cells is empty.",
                format!("The DID design requires observations in every (treatment, period) cell; cell {} has none.", j),
                "Check the treatment and time variables so all four cells contain data.",
            ));
        }
    }

    // Interaction is the 4th term (index 3: treat, post, treat#post, controls…).
    let idx = 3;
    Ok(DidResult {
        att: reg.coef[idx],
        att_se: reg.se[idx],
        t: reg.t[idx],
        p: reg.p[idx],
        ci_lo: reg.ci_lo[idx],
        ci_hi: reg.ci_hi[idx],
        means,
        n: reg.n,
        terms: reg.terms.clone(),
        coef: reg.coef.clone(),
        se: reg.se.clone(),
        p_values: reg.p.clone(),
        regression: reg,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn panel() -> (Vec<f64>, Vec<Vec<f64>>, Vec<ClusterId>) {
        // Two-period panel, 20 entities: y_it = a_i + 0.5 * x_it
        let mut y = Vec::new();
        let mut rows = Vec::new();
        let mut ent = Vec::new();
        for e in 0..20u32 {
            let a = (e as f64) * 2.0;
            for t in 0..2 {
                let x = (e + t) as f64 * 0.1 + 1.0;
                y.push(a + 0.5 * x);
                rows.push(vec![x]);
                ent.push(e);
            }
        }
        (y, rows, ent)
    }

    #[test]
    fn fixed_effects_recovers_slope_with_absorbed_heterogeneity() {
        let (y, rows, ent) = panel();
        let x = Matrix::from_rows(&rows).unwrap();
        let names = vec!["x".to_string()];
        let res = fixed_effects(&y, &x, &ent, &names, &VcovSpec::Classical, None).unwrap();
        assert!((res.coef[0] - 0.5).abs() < 1e-10, "coef={}", res.coef[0]);
        assert_eq!(res.g, 20);
        assert_eq!(res.dropped_singletons, 0);
        assert_eq!(res.df_r, 40 - 20 - 1);
    }

    #[test]
    fn time_invariant_regressor_is_absorbed() {
        // x constant within entity -> within-variation zero -> not identified.
        let mut y = Vec::new();
        let mut rows = Vec::new();
        let mut ent = Vec::new();
        for e in 0..10u32 {
            for t in 0..3 {
                y.push((e + t) as f64);
                rows.push(vec![e as f64]); // constant within entity
                ent.push(e);
            }
        }
        let x = Matrix::from_rows(&rows).unwrap();
        let names = vec!["x".to_string()];
        let err = fixed_effects(&y, &x, &ent, &names, &VcovSpec::Classical, None).unwrap_err();
        assert!(err.why.contains("no variation"));
    }

    #[test]
    fn between_estimator_uses_entity_means() {
        let (y, rows, ent) = panel();
        let x = Matrix::from_rows(&rows).unwrap();
        let names = vec!["x".to_string()];
        let res = between(&y, &x, &ent, &names, &VcovSpec::Classical, None).unwrap();
        assert_eq!(res.n, 20);
        // Between slope on means differs from within slope here.
        assert!(res.coef[0].is_finite());
    }

    #[test]
    fn did_recovers_known_treatment_effect() {
        // DGP: control pre 10, control post 12, treated pre 10, treated post 15 (+5 effect).
        let mut y = Vec::new();
        let mut treat = Vec::new();
        let mut post = Vec::new();
        for i in 0..40 {
            let t = if i < 20 { 0.0 } else { 1.0 };
            let p = (i % 2) as f64;
            let mut base = 10.0 + 2.0 * p;
            if t == 1.0 && p == 1.0 {
                base += 5.0;
            }
            y.push(base);
            treat.push(t);
            post.push(p);
        }
        let empty_rows: Vec<Vec<f64>> = (0..40).map(|_| vec![]).collect();
        let controls = Matrix::from_rows(&empty_rows).unwrap();
        let res = did(
            &y,
            &treat,
            &post,
            &controls,
            &[],
            &VcovSpec::Classical,
            None,
        )
        .unwrap();
        assert!((res.att - 5.0).abs() < 1e-10, "att={}", res.att);
        assert!(res.p < 1e-10);
    }

    #[test]
    fn did_rejects_non_binary_treatment() {
        let y = vec![1.0, 2.0, 3.0, 4.0];
        let treat = vec![0.0, 1.0, 2.0, 1.0];
        let post = vec![0.0, 0.0, 1.0, 1.0];
        let empty_rows: Vec<Vec<f64>> = (0..4).map(|_| vec![]).collect();
        let controls = Matrix::from_rows(&empty_rows).unwrap();
        let err = did(
            &y,
            &treat,
            &post,
            &controls,
            &[],
            &VcovSpec::Classical,
            None,
        )
        .unwrap_err();
        assert!(err.what.contains("0/1"));
    }
}
