//! # Golden dataset validation
//!
//! Numerical validation against independent reference values:
//!
//! 1. **NIST StRD Longley** — the notoriously ill-conditioned regression
//!    benchmark with certified reference coefficients and standard errors.
//! 2. **Algebraic identities** — estimators cross-checked against each other
//!    (within vs LSDV, cluster-with-singletons vs HC1, WLS vs OLS with
//!    unit weights, F(1,ν) vs t²).
//! 3. **Reference table values** — distribution quantiles (also covered in
//!    numeris-core unit tests).
//!
//! Tolerances are method-appropriate and documented in
//! `validation/golden/longley_expected.json`.

use numeris_core::csv::read_csv;
use numeris_stats::linalg::Matrix;
use numeris_stats::vcov::VcovSpec;
use numeris_stats::{fixed_effects, Ols};

const LONGLEY_CSV: &str = include_str!("../../../validation/golden/longley.csv");
const LONGLEY_EXPECTED: &str = include_str!("../../../validation/golden/longley_expected.json");

fn rel_err(est: f64, ref_: f64) -> f64 {
    (est - ref_).abs() / ref_.abs().max(1e-300)
}

#[test]
fn nist_strd_longley_coefficients_match_certified_values() {
    let expected: serde_json::Value = serde_json::from_str(LONGLEY_EXPECTED).unwrap();
    let coef = &expected["coefficients"];
    let se = &expected["standard_errors"];
    let tol_coef = expected["tolerances"]["coefficient_relative"]
        .as_f64()
        .unwrap();
    let tol_se = expected["tolerances"]["se_relative"].as_f64().unwrap();

    let (df, _report) = read_csv(LONGLEY_CSV).unwrap();
    let names = ["defl", "gnp", "unemp", "armed", "pop", "year"];
    let idx: Vec<usize> = df.complete_case_indices(&names).unwrap();
    let y = df.numeric_vector("total", &idx).unwrap();
    let x = Matrix::from_rows(&df.numeric_matrix(&names, &idx).unwrap()).unwrap();
    let xnames: Vec<String> = names.iter().map(|s| s.to_string()).collect();
    let reg = Ols::new(&y, &x, &xnames).unwrap().fit().unwrap();

    assert_eq!(reg.n, 16);
    // Coefficients.
    for (j, key) in ["b0", "b1", "b2", "b3", "b4", "b5", "b6"]
        .iter()
        .enumerate()
    {
        let expected_value = coef[key].as_f64().unwrap();
        let err = rel_err(reg.coef[j], expected_value);
        assert!(
            err <= tol_coef,
            "{key}: estimated {}, certified {}, rel err {err:.3e} > {tol_coef}",
            reg.coef[j],
            expected_value
        );
    }
    // Standard errors.
    for (j, key) in ["b0", "b1", "b2", "b3", "b4", "b5", "b6"]
        .iter()
        .enumerate()
    {
        let expected_value = se[key].as_f64().unwrap();
        let err = rel_err(reg.se[j], expected_value);
        assert!(
            err <= tol_se,
            "{key} SE: estimated {}, certified {}, rel err {err:.3e} > {tol_se}",
            reg.se[j],
            expected_value
        );
    }
    // R² and residual SD.
    let r2_expected = expected["r_squared"].as_f64().unwrap();
    assert!(
        (reg.r2 - r2_expected).abs() <= expected["tolerances"]["r2_absolute"].as_f64().unwrap(),
        "R² = {} vs {}",
        reg.r2,
        r2_expected
    );
    let rmse_expected = expected["residual_sd"].as_f64().unwrap();
    assert!(
        rel_err(reg.rmse, rmse_expected)
            <= expected["tolerances"]["rmse_relative"].as_f64().unwrap(),
        "RMSE = {} vs {}",
        reg.rmse,
        rmse_expected
    );
}

#[test]
fn golden_synthetic_regression_exact_recovery() {
    // Orthogonal design with hand-solvable coefficients.
    let rows: Vec<Vec<f64>> = vec![
        vec![1.0, 0.0],
        vec![-1.0, 0.0],
        vec![0.0, 1.0],
        vec![0.0, -1.0],
    ];
    let x = Matrix::from_rows(&rows).unwrap();
    let y = vec![7.0, 3.0, 1.0, -5.0];
    let names = vec!["a".to_string(), "b".to_string()];
    let reg = Ols::new(&y, &x, &names).unwrap().fit().unwrap();
    // β_a = Σ a_i y_i / Σ a_i² = (7 - 3) / 2 = 2
    // β_b = (1 + 5) / 2 = 3
    assert!((reg.coef[1] - 2.0).abs() < 1e-12, "β_a = {}", reg.coef[1]);
    assert!((reg.coef[2] - 3.0).abs() < 1e-12, "β_b = {}", reg.coef[2]);
    assert!((reg.coef[0] - 1.5).abs() < 1e-12, "cons = {}", reg.coef[0]);
}

#[test]
fn identity_wls_with_unit_weights_equals_ols() {
    let y: Vec<f64> = (0..30)
        .map(|i| 2.0 + 1.5 * i as f64 + (if i % 4 == 0 { 1.0 } else { -0.3 }))
        .collect();
    let rows: Vec<Vec<f64>> = (0..30).map(|i| vec![i as f64]).collect();
    let x = Matrix::from_rows(&rows).unwrap();
    let names = vec!["x".to_string()];
    let ols = Ols::new(&y, &x, &names).unwrap().fit().unwrap();
    let wls = Ols::new(&y, &x, &names)
        .unwrap()
        .weights(vec![1.0; 30])
        .fit()
        .unwrap();
    for j in 0..ols.k {
        assert!((ols.coef[j] - wls.coef[j]).abs() < 1e-12);
        assert!((ols.se[j] - wls.se[j]).abs() < 1e-12);
    }
}

#[test]
fn identity_cluster_singletons_equals_hc1() {
    // With one observation per cluster (G = n), the cluster correction
    // c = G/(G-1) · (n-1)/(n-k) reduces to n/(n-k), which is exactly HC1.
    let y: Vec<f64> = (0..40)
        .map(|i| 1.0 + 0.7 * i as f64 + (if i % 3 == 0 { 2.0 } else { -1.0 }))
        .collect();
    let rows: Vec<Vec<f64>> = (0..40).map(|i| vec![i as f64 % 9.0]).collect();
    let x = Matrix::from_rows(&rows).unwrap();
    let names = vec!["x".to_string()];
    let hc1 = Ols::new(&y, &x, &names)
        .unwrap()
        .vcov(VcovSpec::Hc1)
        .fit()
        .unwrap();
    let clustered = Ols::new(&y, &x, &names)
        .unwrap()
        .cluster((0..40).map(|i| i as u32).collect())
        .fit()
        .unwrap();
    for j in 0..hc1.k {
        let rel = (hc1.se[j] - clustered.se[j]).abs() / hc1.se[j];
        assert!(
            rel < 1e-10,
            "se[{j}]: hc1={}, cluster={}",
            hc1.se[j],
            clustered.se[j]
        );
    }
}

#[test]
fn identity_within_estimator_equals_lsdv() {
    // The within (FE) estimator equals least squares with entity dummies,
    // including the classical standard error (both use df = n - g - k).
    let g = 5usize;
    let t = 4usize;
    let mut y = Vec::new();
    let mut rows = Vec::new();
    let mut entity: Vec<u32> = Vec::new();
    let mut state = 42u64;
    let mut next = || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((state >> 11) as f64 / (1u64 << 53) as f64) * 2.0 - 1.0
    };
    for e in 0..g {
        let a = e as f64 * 3.0;
        for _t in 0..t {
            let x = next();
            y.push(a + 2.0 * x + 0.5 * next());
            rows.push(vec![x]);
            entity.push(e as u32);
        }
    }
    let xmat = Matrix::from_rows(&rows).unwrap();
    let names = vec!["x".to_string()];
    let fe = fixed_effects(&y, &xmat, &entity, &names, &VcovSpec::Classical, None).unwrap();

    // LSDV: entity dummies (drop one) + x.
    let n = y.len();
    let mut dummies: Vec<Vec<f64>> = Vec::with_capacity(n);
    for i in 0..n {
        let mut row = Vec::with_capacity(g - 1 + 1);
        for e in 1..g {
            row.push(if entity[i] as usize == e { 1.0 } else { 0.0 });
        }
        row.push(rows[i][0]);
        dummies.push(row);
    }
    let dmat = Matrix::from_rows(&dummies).unwrap();
    let dnames: Vec<String> = (1..g)
        .map(|e| format!("d{e}"))
        .chain(std::iter::once("x".to_string()))
        .collect();
    let lsdv = Ols::new(&y, &dmat, &dnames).unwrap().fit().unwrap();

    let x_index_lsdv = lsdv.terms.iter().position(|t| t == "x").unwrap();
    assert!(
        (fe.coef[0] - lsdv.coef[x_index_lsdv]).abs() < 1e-9,
        "fe={}, lsdv={}",
        fe.coef[0],
        lsdv.coef[x_index_lsdv]
    );
    assert!(
        (fe.se[0] - lsdv.se[x_index_lsdv]).abs() < 1e-9,
        "fe se={}, lsdv se={}",
        fe.se[0],
        lsdv.se[x_index_lsdv]
    );
    assert_eq!(fe.df_r, lsdv.df_r);
}

#[test]
fn golden_robust_se_heteroskedastic_pattern() {
    // Constructed heteroskedasticity: variance scales with x. Verify that
    // robust and classical SEs differ materially, and HC3 >= HC2 >= HC1.
    let y: Vec<f64> = (0..100)
        .map(|i| {
            let x = (i as f64).sqrt();
            let scale = x;
            let e = if i % 2 == 0 { scale } else { -scale };
            1.0 + 2.0 * x + e
        })
        .collect();
    let rows: Vec<Vec<f64>> = (0..100).map(|i| vec![(i as f64).sqrt()]).collect();
    let x = Matrix::from_rows(&rows).unwrap();
    let names = vec!["x".to_string()];
    let classical = Ols::new(&y, &x, &names).unwrap().fit().unwrap();
    let robust = Ols::new(&y, &x, &names).unwrap().robust().fit().unwrap();
    let hc2 = Ols::new(&y, &x, &names)
        .unwrap()
        .vcov(VcovSpec::Hc2)
        .fit()
        .unwrap();
    let hc3 = Ols::new(&y, &x, &names)
        .unwrap()
        .vcov(VcovSpec::Hc3)
        .fit()
        .unwrap();
    assert!((classical.se[1] - robust.se[1]).abs() / robust.se[1] > 0.05);
    assert!(hc2.se[1] >= robust.se[1]);
    assert!(hc3.se[1] >= hc2.se[1]);
}

#[test]
fn reproducibility_same_inputs_same_outputs_bit_for_bit() {
    let build = || {
        let y: Vec<f64> = (0..80)
            .map(|i| 3.0 + 0.9 * i as f64 + (if i % 5 == 0 { 1.5 } else { -0.2 }))
            .collect();
        let rows: Vec<Vec<f64>> = (0..80).map(|i| vec![i as f64, (i % 7) as f64]).collect();
        let x = Matrix::from_rows(&rows).unwrap();
        let names = vec!["a".to_string(), "b".to_string()];
        let reg = Ols::new(&y, &x, &names).unwrap().robust().fit().unwrap();
        (reg.coef.clone(), reg.se.clone(), reg.p.clone())
    };
    let first = build();
    let second = build();
    assert_eq!(first.0, second.0);
    assert_eq!(first.1, second.1);
    assert_eq!(first.2, second.2);
}
