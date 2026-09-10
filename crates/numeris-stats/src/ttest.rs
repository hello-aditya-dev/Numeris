//! # Classical hypothesis tests
//!
//! One-sample, paired, two-sample pooled and Welch t-tests, one-way ANOVA,
//! and Pearson/Spearman correlation tests.

use crate::linalg::Matrix;
use numeris_core::describe::{mean, sd};
use numeris_core::dist;
use numeris_core::error::{NumerisError, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TTestKind {
    OneSample,
    Paired,
    TwoSamplePooled,
    Welch,
}

/// A t-test result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TTestResult {
    pub kind: TTestKind,
    pub variable: String,
    pub by: Option<String>,
    /// Null-hypothesis value of the mean (or mean difference).
    pub mu0: f64,
    pub n1: usize,
    pub n2: Option<usize>,
    pub mean1: f64,
    pub mean2: Option<f64>,
    pub sd1: f64,
    pub sd2: Option<f64>,
    /// Estimated difference (mean1 − mean2) or mean (one-sample/paired).
    pub diff: f64,
    pub se: f64,
    pub t: f64,
    pub df: f64,
    pub p_two_sided: f64,
    pub ci_lo: f64,
    pub ci_hi: f64,
}

/// One-sample t-test of H0: mean = mu0.
pub fn t_test_one_sample(xs: &[f64], mu0: f64, variable: &str) -> Result<TTestResult> {
    let n = xs.len();
    if n < 2 {
        return Err(NumerisError::insufficient_data(
            format!("A t-test requires at least two observations; '{variable}' has {n} non-missing values."),
            "The sample standard deviation is undefined with fewer than two observations.",
            "Inspect the variable for missing values, then rerun the test.",
        ));
    }
    let m = mean(xs).ok_or_else(|| {
        NumerisError::insufficient_data(
            "Could not compute the sample mean.",
            "The input contains no valid observations.",
            "Inspect the variable for missing values, then rerun the test.",
        )
    })?;
    let s = sd(xs).ok_or_else(|| {
        NumerisError::not_identified(
            format!("The t-test is undefined for '{variable}' because the sample variance is zero."),
            "With no variation in the data, the standard error of the mean is zero and the t statistic cannot be computed.",
            "Check whether the variable is constant in this sample; a t-test of a constant is not meaningful.",
        )
    })?;
    let se = s / (n as f64).sqrt();
    let t = (m - mu0) / se;
    let df = (n - 1) as f64;
    let p = dist::t_two_sided_p(t, df);
    let tcrit = dist::t_quantile(0.975, df)?;
    Ok(TTestResult {
        kind: TTestKind::OneSample,
        variable: variable.to_string(),
        by: None,
        mu0,
        n1: n,
        n2: None,
        mean1: m,
        mean2: None,
        sd1: s,
        sd2: None,
        diff: m,
        se,
        t,
        df,
        p_two_sided: p,
        ci_lo: m - tcrit * se,
        ci_hi: m + tcrit * se,
    })
}

/// Paired t-test of the row-wise differences against mu0.
pub fn t_test_paired(
    a: &[f64],
    b: &[f64],
    mu0: f64,
    var_a: &str,
    var_b: &str,
) -> Result<TTestResult> {
    if a.len() != b.len() {
        return Err(NumerisError::numerical_failure(
            "Paired t-test inputs have different lengths.",
            "Internal error: the paired variables were not aligned by row.",
            "Please report this as a bug with the command that triggered it.",
        ));
    }
    let d: Vec<f64> = a.iter().zip(b).map(|(x, y)| x - y).collect();
    let mut r = t_test_one_sample(&d, mu0, &format!("{var_a} - {var_b}"))?;
    r.kind = TTestKind::Paired;
    r.mean2 = mean(b);
    r.sd2 = sd(b);
    r.n2 = Some(b.len());
    r.variable = format!("{var_a} - {var_b}");
    Ok(r)
}

/// Two-sample t-test. `welch` selects the Satterthwaite approximation;
/// otherwise the pooled-variance (equal-variance) form is used.
pub fn t_test_two_sample(
    a: &[f64],
    b: &[f64],
    welch: bool,
    by: &str,
    variable: &str,
) -> Result<TTestResult> {
    if a.len() < 2 || b.len() < 2 {
        return Err(NumerisError::insufficient_data(
            format!("A two-sample t-test requires at least two observations per group; group sizes are {} and {}.", a.len(), b.len()),
            "The group standard deviations are undefined with fewer than two observations.",
            "Check the grouping variable and the outcome for missing values, then rerun.",
        ));
    }
    let n1 = a.len();
    let n2 = b.len();
    let m1 = mean(a).unwrap();
    let m2 = mean(b).unwrap();
    let s1 = sd(a).unwrap();
    let s2 = sd(b).unwrap();
    let (se, df) = if welch {
        let v1 = s1 * s1 / n1 as f64;
        let v2 = s2 * s2 / n2 as f64;
        let se = (v1 + v2).sqrt();
        let df =
            (v1 + v2) * (v1 + v2) / (v1 * v1 / (n1 as f64 - 1.0) + v2 * v2 / (n2 as f64 - 1.0));
        (se, df)
    } else {
        let sp2 = ((n1 - 1) as f64 * s1 * s1 + (n2 - 1) as f64 * s2 * s2) / ((n1 + n2 - 2) as f64);
        let se = sp2.sqrt() * (1.0 / n1 as f64 + 1.0 / n2 as f64).sqrt();
        (se, (n1 + n2 - 2) as f64)
    };
    if se == 0.0 || !se.is_finite() {
        return Err(NumerisError::not_identified(
            format!("The t-test for '{variable}' by '{by}' is undefined because the pooled standard error is zero."),
            "At least one group is constant, so there is no within-group variation to estimate the standard error.",
            "Check the outcome values within each group; a t-test requires variation in both groups.",
        ));
    }
    let diff = m1 - m2;
    let t = diff / se;
    let p = dist::t_two_sided_p(t, df);
    let tcrit = dist::t_quantile(0.975, df)?;
    Ok(TTestResult {
        kind: if welch {
            TTestKind::Welch
        } else {
            TTestKind::TwoSamplePooled
        },
        variable: variable.to_string(),
        by: Some(by.to_string()),
        mu0: 0.0,
        n1,
        n2: Some(n2),
        mean1: m1,
        mean2: Some(m2),
        sd1: s1,
        sd2: Some(s2),
        diff,
        se,
        t,
        df,
        p_two_sided: p,
        ci_lo: diff - tcrit * se,
        ci_hi: diff + tcrit * se,
    })
}

/// One-way ANOVA result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupSummary {
    pub label: String,
    pub n: usize,
    pub mean: f64,
    pub sd: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnovaResult {
    pub variable: String,
    pub by: String,
    pub groups: Vec<GroupSummary>,
    pub n: usize,
    pub df_between: usize,
    pub df_within: usize,
    pub f: f64,
    pub p: f64,
    pub eta_squared: f64,
    pub grand_mean: f64,
}

/// One-way ANOVA of `values` across groups defined by `labels`.
pub fn one_way_anova(
    values: &[f64],
    labels: &[String],
    by: &str,
    variable: &str,
) -> Result<AnovaResult> {
    if values.len() != labels.len() {
        return Err(NumerisError::numerical_failure(
            "ANOVA inputs are misaligned.",
            "Internal error: the outcome and grouping variable have different lengths.",
            "Please report this as a bug with the command that triggered it.",
        ));
    }
    let mut order: Vec<String> = labels.to_vec();
    order.sort();
    order.dedup();
    let g = order.len();
    if g < 2 {
        return Err(NumerisError::insufficient_data(
            format!("ANOVA requires at least two groups; '{by}' defines {g} group."),
            "With a single group there is no between-group variation to test.",
            "Choose a grouping variable that defines at least two groups.",
        ));
    }
    let n = values.len();
    let grand = mean(values).unwrap();
    let mut groups = Vec::with_capacity(g);
    let mut ssb = 0.0;
    let mut ssw = 0.0;
    for label in &order {
        let ys: Vec<f64> = labels
            .iter()
            .zip(values)
            .filter(|(l, _)| l.as_str() == label.as_str())
            .map(|(_, v)| *v)
            .collect();
        let gm = mean(&ys).unwrap();
        let gsd = sd(&ys);
        ssb += ys.len() as f64 * (gm - grand) * (gm - grand);
        ssw += ys.iter().map(|v| (v - gm) * (v - gm)).sum::<f64>();
        groups.push(GroupSummary {
            label: label.clone(),
            n: ys.len(),
            mean: gm,
            sd: gsd,
        });
    }
    let df_b = g - 1;
    let df_w = n.saturating_sub(g);
    if df_w == 0 {
        return Err(NumerisError::insufficient_data(
            "ANOVA has no within-group degrees of freedom.",
            format!("Each of the {g} groups contains a single observation, so the error variance cannot be estimated."),
            "Use a sample with more than one observation per group.",
        ));
    }
    let msb = ssb / df_b as f64;
    let msw = ssw / df_w as f64;
    if msw <= 0.0 {
        return Err(NumerisError::not_identified(
            "ANOVA is undefined because the within-group variance is zero.",
            "All observations within each group are identical, so the F statistic cannot be computed.",
            "Check the outcome variable; ANOVA requires within-group variation.",
        ));
    }
    let f = msb / msw;
    let p = 1.0 - dist::f_cdf(f, df_b as f64, df_w as f64);
    let sst = ssb + ssw;
    let eta2 = if sst > 0.0 { ssb / sst } else { 0.0 };
    Ok(AnovaResult {
        variable: variable.to_string(),
        by: by.to_string(),
        groups,
        n,
        df_between: df_b,
        df_within: df_w,
        f,
        p,
        eta_squared: eta2,
        grand_mean: grand,
    })
}

/// Correlation matrix (listwise deletion) with pairwise p-values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrResult {
    pub variables: Vec<String>,
    /// values[i][j] correlation; p_values[i][j] two-sided p.
    pub values: Vec<Vec<Option<f64>>>,
    pub p_values: Vec<Vec<Option<f64>>>,
    pub n: usize,
    pub method: String,
}

/// Pearson or Spearman correlation matrix over complete rows.
pub fn corr_matrix(columns: &Matrix, names: &[String], spearman: bool) -> Result<CorrResult> {
    let n = columns.rows;
    let k = columns.cols;
    let mut work = columns.clone();
    if spearman {
        for j in 0..k {
            let col: Vec<f64> = (0..n).map(|i| columns.get(i, j)).collect();
            let ranked = rank_average(&col);
            for i in 0..n {
                work.data[i * k + j] = ranked[i];
            }
        }
    }
    let mut values = vec![vec![None; k]; k];
    let mut p_values = vec![vec![None; k]; k];
    for a in 0..k {
        values[a][a] = Some(1.0);
        for b in (a + 1)..k {
            let xs: Vec<f64> = (0..n).map(|i| work.get(i, a)).collect();
            let ys: Vec<f64> = (0..n).map(|i| work.get(i, b)).collect();
            let r = pearson(&xs, &ys);
            values[a][b] = r;
            values[b][a] = r;
            if let Some(r) = r {
                if r.abs() < 1.0 && n > 2 {
                    let t = r * ((n as f64 - 2.0) / (1.0 - r * r)).sqrt();
                    let p = dist::t_two_sided_p(t, n as f64 - 2.0);
                    p_values[a][b] = Some(p);
                    p_values[b][a] = Some(p);
                } else if (r.abs() - 1.0).abs() < 1e-12 {
                    p_values[a][b] = Some(0.0);
                    p_values[b][a] = Some(0.0);
                }
            }
        }
    }
    Ok(CorrResult {
        variables: names.to_vec(),
        values,
        p_values,
        n,
        method: if spearman {
            "Spearman".to_string()
        } else {
            "Pearson".to_string()
        },
    })
}

/// Pearson correlation; None when either vector has zero variance or n < 2.
pub fn pearson(xs: &[f64], ys: &[f64]) -> Option<f64> {
    if xs.len() != ys.len() || xs.len() < 2 {
        return None;
    }
    let mx = mean(xs)?;
    let my = mean(ys)?;
    let mut sxy = 0.0;
    let mut sxx = 0.0;
    let mut syy = 0.0;
    for (&x, &y) in xs.iter().zip(ys) {
        let dx = x - mx;
        let dy = y - my;
        sxy += dx * dy;
        sxx += dx * dx;
        syy += dy * dy;
    }
    if sxx <= 0.0 || syy <= 0.0 {
        return None;
    }
    Some((sxy / (sxx * syy).sqrt()).clamp(-1.0, 1.0))
}

/// Average ranks with tie handling (1 = smallest).
pub fn rank_average(xs: &[f64]) -> Vec<f64> {
    let n = xs.len();
    let mut idx: Vec<usize> = (0..n).collect();
    idx.sort_by(|&a, &b| {
        xs[a]
            .partial_cmp(&xs[b])
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut ranks = vec![0.0; n];
    let mut i = 0;
    while i < n {
        let mut j = i;
        while j + 1 < n && xs[idx[j + 1]] == xs[idx[i]] {
            j += 1;
        }
        let avg = (i + j) as f64 / 2.0 + 1.0;
        for &pos in &idx[i..=j] {
            ranks[pos] = avg;
        }
        i = j + 1;
    }
    ranks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_sample_t_test_known_values() {
        // xs: 1..5 => mean 3, sd = sqrt(2.5), se = sqrt(2.5/5)
        let xs: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let r = t_test_one_sample(&xs, 2.5, "x").unwrap();
        assert!((r.mean1 - 3.0).abs() < 1e-12);
        let se = (2.5f64 / 5.0).sqrt();
        assert!((r.se - se).abs() < 1e-12);
        assert!((r.t - (0.5 / se)).abs() < 1e-12);
        assert_eq!(r.df, 4.0);
        // p two-sided for t with df 4
        let expected_p = dist::t_two_sided_p(r.t, 4.0);
        assert!((r.p_two_sided - expected_p).abs() < 1e-15);
    }

    #[test]
    fn paired_t_test_matches_manual_differences() {
        let a: Vec<f64> = vec![10.0, 12.0, 14.0, 16.0];
        let b: Vec<f64> = vec![9.0, 12.0, 13.0, 17.0];
        let r = t_test_paired(&a, &b, 0.0, "a", "b").unwrap();
        let d = vec![1.0, 0.0, 1.0, -1.0];
        let manual = t_test_one_sample(&d, 0.0, "d").unwrap();
        assert!((r.t - manual.t).abs() < 1e-12);
        assert!((r.p_two_sided - manual.p_two_sided).abs() < 1e-15);
    }

    #[test]
    fn two_sample_pooled_t_test_matches_manual() {
        let a: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let b: Vec<f64> = vec![3.0, 4.0, 5.0, 6.0, 7.0];
        let r = t_test_two_sample(&a, &b, false, "g", "x").unwrap();
        let m1 = 3.0;
        let m2 = 5.0;
        let sp2: f64 = (4.0 * 2.5 + 4.0 * 2.5) / 8.0;
        let se = sp2.sqrt() * (0.2f64 + 0.2).sqrt();
        assert!((r.diff - (m1 - m2)).abs() < 1e-12);
        assert!((r.se - se).abs() < 1e-12);
        assert_eq!(r.df, 8.0);
    }

    #[test]
    fn welch_df_computed_correctly() {
        let a: Vec<f64> = vec![1.0, 2.0, 3.0];
        let b: Vec<f64> = vec![
            10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0, 20.0,
        ];
        let r = t_test_two_sample(&a, &b, true, "g", "x").unwrap();
        let v1 = numeris_core::describe::variance(&a).unwrap() / 3.0;
        let v2 = numeris_core::describe::variance(&b).unwrap() / 11.0;
        let expected_df = (v1 + v2).powi(2) / (v1 * v1 / 2.0 + v2 * v2 / 10.0);
        assert!((r.df - expected_df).abs() < 1e-10);
    }

    #[test]
    fn anova_known_result() {
        // Two well-separated groups.
        let values: Vec<f64> = (0..10)
            .map(|i| if i < 5 { i as f64 } else { 100.0 + i as f64 })
            .collect();
        let labels: Vec<String> = (0..10)
            .map(|i| {
                if i < 5 {
                    "a".to_string()
                } else {
                    "b".to_string()
                }
            })
            .collect();
        let r = one_way_anova(&values, &labels, "g", "x").unwrap();
        assert_eq!(r.df_between, 1);
        assert_eq!(r.df_within, 8);
        assert!(r.f > 1000.0);
        assert!(r.p < 1e-9);
        assert!(r.eta_squared > 0.99);
    }

    #[test]
    fn anova_requires_two_groups() {
        let values = vec![1.0, 2.0, 3.0];
        let labels = vec!["a".to_string(), "a".to_string(), "a".to_string()];
        let err = one_way_anova(&values, &labels, "g", "x").unwrap_err();
        assert!(err.what.contains("two groups"));
    }

    #[test]
    fn pearson_perfect_correlation() {
        let xs: Vec<f64> = (0..10).map(|i| i as f64).collect();
        let ys: Vec<f64> = xs.iter().map(|x| 3.0 * x + 1.0).collect();
        let r = pearson(&xs, &ys).unwrap();
        assert!((r - 1.0).abs() < 1e-12);
    }

    #[test]
    fn spearman_ranks_with_ties() {
        let xs = vec![1.0, 2.0, 2.0, 3.0];
        let ranks = rank_average(&xs);
        assert_eq!(ranks, vec![1.0, 2.5, 2.5, 4.0]);
    }

    #[test]
    fn corr_matrix_symmetric_with_ones_on_diagonal() {
        let rows: Vec<Vec<f64>> = (0..30)
            .map(|i| {
                let x = i as f64;
                vec![x, 2.0 * x + 1.0, -x]
            })
            .collect();
        let m = Matrix::from_rows(&rows).unwrap();
        let names = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let r = corr_matrix(&m, &names, false).unwrap();
        assert_eq!(r.n, 30);
        for i in 0..3 {
            assert!((r.values[i][i].unwrap() - 1.0).abs() < 1e-12);
        }
        let ab = r.values[0][1].unwrap();
        assert!((ab - 1.0).abs() < 1e-12);
        let ac = r.values[0][2].unwrap();
        assert!((ac + 1.0).abs() < 1e-12);
        assert!((r.values[1][0].unwrap() - ab).abs() < 1e-12);
    }
}
