//! # Descriptive statistics
//!
//! Deterministic univariate summaries: count, missing, mean, variance, SD,
//! min/max, quantiles (type 7), skewness (bias-corrected G1), kurtosis
//! (bias-corrected excess, G2), unique count, and frequency tables.
//!
//! Conventions:
//! - variance / SD: sample (n-1) denominators;
//! - skewness: G1 = (n * m3) / ((n-1)(n-2)) * (s^3)^-1 with s the sample SD
//!   (bias-corrected, "type 2" — matches common statistical software);
//! - kurtosis: excess G2 = ((n+1)/n * m4/m2^2) - 3(n-1)^2/((n-2)(n-3)).

use crate::error::{NumerisError, Result};
use crate::frame::DataFrame;
use serde::{Deserialize, Serialize};

/// Summary statistics for one numeric variable.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SummaryStats {
    pub name: String,
    pub n: usize,
    pub missing: usize,
    pub mean: Option<f64>,
    pub variance: Option<f64>,
    pub sd: Option<f64>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub median: Option<f64>,
    pub q1: Option<f64>,
    pub q3: Option<f64>,
    pub skewness: Option<f64>,
    pub kurtosis: Option<f64>,
    pub unique: usize,
    pub sum: Option<f64>,
}

/// A frequency table row for a categorical variable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrequencyRow {
    pub value: String,
    pub count: usize,
    pub percent: f64,
    pub cumulative_percent: f64,
}

/// Quantile of sorted values with linear interpolation (type 7, R/NumPy default).
/// `p` must be in [0, 1].
pub fn quantile_sorted(sorted: &[f64], p: f64) -> Option<f64> {
    if sorted.is_empty() {
        return None;
    }
    let p = p.clamp(0.0, 1.0);
    let n = sorted.len();
    if n == 1 {
        return Some(sorted[0]);
    }
    let h = (n as f64 - 1.0) * p;
    let lo = h.floor() as usize;
    let hi = (lo + 1).min(n - 1);
    let frac = h - lo as f64;
    Some(sorted[lo] + frac * (sorted[hi] - sorted[lo]))
}

/// Mean; None when no observations.
pub fn mean(xs: &[f64]) -> Option<f64> {
    if xs.is_empty() {
        return None;
    }
    // Kahan compensated summation for reproducible accuracy.
    let mut sum = 0.0;
    let mut c = 0.0;
    for &x in xs {
        let y = x - c;
        let t = sum + y;
        c = (t - sum) - y;
        sum = t;
    }
    Some(sum / xs.len() as f64)
}

/// Sample variance (n-1 denominator).
pub fn variance(xs: &[f64]) -> Option<f64> {
    let n = xs.len();
    if n < 2 {
        return None;
    }
    let m = mean(xs)?;
    let mut ss = 0.0;
    let mut c = 0.0;
    for &x in xs {
        let d = x - m;
        let y = d * d - c;
        let t = ss + y;
        c = (t - ss) - y;
        ss = t;
    }
    Some(ss / (n as f64 - 1.0))
}

/// Sample standard deviation.
pub fn sd(xs: &[f64]) -> Option<f64> {
    variance(xs).map(|v| v.sqrt())
}

/// Bias-corrected skewness (G1). Requires n >= 3 and nonzero variance.
pub fn skewness(xs: &[f64]) -> Option<f64> {
    let n = xs.len();
    if n < 3 {
        return None;
    }
    let m = mean(xs)?;
    let mut m2 = 0.0;
    let mut m3 = 0.0;
    for &x in xs {
        let d = x - m;
        let d2 = d * d;
        m2 += d2;
        m3 += d2 * d;
    }
    m2 /= n as f64;
    m3 /= n as f64;
    let m2_32 = m2 * m2.sqrt();
    if m2_32 <= 0.0 || !m2_32.is_finite() {
        return None;
    }
    let nn = n as f64;
    Some((nn * m3) / ((nn - 1.0) * (nn - 2.0) * m2_32))
}

/// Bias-corrected excess kurtosis (G2). Requires n >= 4.
pub fn kurtosis(xs: &[f64]) -> Option<f64> {
    let n = xs.len();
    if n < 4 {
        return None;
    }
    let m = mean(xs)?;
    let mut m2 = 0.0;
    let mut m4 = 0.0;
    for &x in xs {
        let d = x - m;
        let d2 = d * d;
        m2 += d2;
        m4 += d2 * d2;
    }
    m2 /= n as f64;
    m4 /= n as f64;
    if m2 <= 0.0 || !m2.is_finite() {
        return None;
    }
    let nn = n as f64;
    // Adjusted Fisher–Pearson coefficient:
    // G2 = ((n-1)/((n-2)(n-3))) * ((n+1) * (m4/m2^2 - 3) + 6)
    let g2_raw = m4 / (m2 * m2) - 3.0;
    Some(((nn - 1.0) / ((nn - 2.0) * (nn - 3.0))) * ((nn + 1.0) * g2_raw + 6.0))
}

/// Compute the full summary for one variable of a DataFrame.
pub fn summarize_variable(df: &DataFrame, name: &str) -> Result<SummaryStats> {
    let col = df.numeric_col(name)?;
    let xs: Vec<f64> = col.iter().filter_map(|v| *v).collect();
    let mut sorted = xs.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mut stats = SummaryStats {
        name: name.to_string(),
        n: xs.len(),
        missing: col.len() - xs.len(),
        unique: if sorted.is_empty() {
            0
        } else {
            1 + (1..sorted.len())
                .filter(|i| sorted[*i] != sorted[*i - 1])
                .count()
        },
        mean: mean(&xs),
        variance: variance(&xs),
        sd: sd(&xs),
        min: sorted.first().copied(),
        max: sorted.last().copied(),
        median: quantile_sorted(&sorted, 0.5),
        q1: quantile_sorted(&sorted, 0.25),
        q3: quantile_sorted(&sorted, 0.75),
        skewness: skewness(&xs),
        kurtosis: kurtosis(&xs),
        sum: if xs.is_empty() {
            None
        } else {
            Some(xs.iter().sum())
        },
    };
    if stats.variance == Some(0.0) {
        // Zero variance: skewness/kurtosis are undefined.
        stats.skewness = None;
        stats.kurtosis = None;
    }
    if stats.n < 3 {
        stats.skewness = None;
    }
    if stats.n < 4 {
        stats.kurtosis = None;
    }
    Ok(stats)
}

/// Frequency table for a text variable, sorted by count descending then value.
pub fn frequency_table(df: &DataFrame, name: &str) -> Result<Vec<FrequencyRow>> {
    let col = df.text_col(name)?;
    let mut counts: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    let mut missing = 0usize;
    for v in col.iter() {
        match v {
            Some(s) if !s.is_empty() => *counts.entry(s.clone()).or_insert(0) += 1,
            _ => missing += 1,
        }
    }
    let total: usize = counts.values().sum::<usize>() + missing;
    let mut rows: Vec<(String, usize)> = counts.into_iter().collect();
    if missing > 0 {
        rows.push(("(missing)".to_string(), missing));
    }
    rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let mut out = Vec::with_capacity(rows.len());
    let mut cum = 0.0;
    for (value, count) in rows {
        let percent = if total == 0 {
            0.0
        } else {
            100.0 * count as f64 / total as f64
        };
        cum += percent;
        out.push(FrequencyRow {
            value,
            count,
            percent,
            cumulative_percent: cum,
        });
    }
    Ok(out)
}

/// Summarize all numeric variables.
pub fn summarize_all(df: &DataFrame) -> Vec<SummaryStats> {
    df.variables
        .iter()
        .filter(|v| matches!(v.storage, crate::frame::StorageType::Numeric))
        .filter_map(|v| summarize_variable(df, &v.name).ok())
        .collect()
}

/// Errors used by callers when a variable has no usable observations.
pub fn require_observations(name: &str, n: usize) -> Result<()> {
    if n == 0 {
        Err(NumerisError::insufficient_data(
            format!("Variable '{name}' has no non-missing observations."),
            "After applying listwise deletion, no rows with a valid value remain for this variable.",
            "Inspect the variable in the Data Workbench and resolve missing values before continuing.",
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mean_and_variance_known_values() {
        let xs = [1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(mean(&xs), Some(3.0));
        let v = variance(&xs).unwrap();
        assert!((v - 2.5).abs() < 1e-14);
        let s = sd(&xs).unwrap();
        assert!((s - (2.5f64).sqrt()).abs() < 1e-14);
    }

    #[test]
    fn quantiles_type7() {
        let xs: Vec<f64> = (1..=10).map(|i| i as f64).collect();
        assert_eq!(quantile_sorted(&xs, 0.0), Some(1.0));
        assert_eq!(quantile_sorted(&xs, 0.5), Some(5.5));
        assert_eq!(quantile_sorted(&xs, 0.25), Some(3.25));
        assert_eq!(quantile_sorted(&xs, 1.0), Some(10.0));
    }

    #[test]
    fn skewness_symmetric_is_zero() {
        let xs: Vec<f64> = vec![-3.0, -2.0, -1.0, 0.0, 1.0, 2.0, 3.0, 0.5, -0.5];
        let g1 = skewness(&xs).unwrap();
        assert!(g1.abs() < 1e-12, "g1 = {g1}");
    }

    #[test]
    fn skewness_positive_for_right_tail() {
        let xs: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 30.0];
        assert!(skewness(&xs).unwrap() > 0.5);
    }

    #[test]
    fn kurtosis_uniform_is_negative() {
        // The adjusted excess kurtosis of the discrete uniform is ≈ -1.2.
        let xs: Vec<f64> = (1..=1000).map(|i| i as f64).collect();
        let g2 = kurtosis(&xs).unwrap();
        assert!((-1.4..-1.0).contains(&g2), "g2 = {g2}");
    }

    #[test]
    fn empty_and_tiny_inputs_return_none() {
        assert_eq!(mean(&[]), None);
        assert_eq!(variance(&[1.0]), None);
        assert_eq!(skewness(&[1.0, 2.0]), None);
        assert_eq!(kurtosis(&[1.0, 2.0, 3.0]), None);
        assert_eq!(quantile_sorted(&[], 0.5), None);
    }
}
