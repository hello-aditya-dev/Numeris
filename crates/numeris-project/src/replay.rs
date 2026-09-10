//! # Project replay (reproducibility engine)
//!
//! Re-runs every recorded command against the stored dataset and compares
//! the fresh results with the stored references under documented,
//! method-appropriate tolerances.

use crate::registry::load_records;
use numeris_command::Executor;
use numeris_core::error::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// One replayed command's verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayStep {
    pub seq: u64,
    pub command: String,
    pub status: String, // "reproduced" | "error" | "not_comparable"
    pub detail: String,
}

/// The reproducibility report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReproducibilityReport {
    pub project: String,
    pub software_version: String,
    pub commands_replayed: usize,
    pub reproduced: usize,
    pub failed: usize,
    pub steps: Vec<ReplayStep>,
    pub all_reproduced: bool,
}

/// Reproducibility tolerances by statistic class. Method-appropriate, not
/// one universal tolerance (see docs/ARCHITECTURE.md).
#[derive(Debug, Clone, Copy)]
pub struct Tolerances {
    pub coefficient_relative: f64,
    pub se_relative: f64,
    pub statistic_absolute: f64,
}

impl Default for Tolerances {
    fn default() -> Self {
        Self {
            coefficient_relative: 1e-10,
            se_relative: 1e-9,
            statistic_absolute: 1e-9,
        }
    }
}

/// Replay a registry against a dataset and compare with stored results.
pub fn replay(
    project_name: &str,
    registry_path: &Path,
    dataset_text: &str,
    tol: Tolerances,
) -> Result<ReproducibilityReport> {
    let records = load_records(registry_path)?;
    let (df, _) = numeris_core::csv::read_csv(dataset_text)?;
    let mut ex = Executor::with_frame(df);
    let mut steps = Vec::new();
    let mut reproduced = 0usize;
    let mut failed = 0usize;

    for record in &records {
        match ex.execute(&record.command) {
            Ok(output) => {
                let fresh = match &output {
                    numeris_command::ExecOutput::Estimate { result, .. } => {
                        serde_json::to_value(result).ok()
                    }
                    numeris_command::ExecOutput::TTest(r) => serde_json::to_value(&**r).ok(),
                    numeris_command::ExecOutput::Anova(r) => serde_json::to_value(&**r).ok(),
                    numeris_command::ExecOutput::Summary(s) => serde_json::to_value(s).ok(),
                    numeris_command::ExecOutput::Corr(c) => serde_json::to_value(&**c).ok(),
                    _ => None,
                };
                let fresh = match fresh {
                    Some(v) => v,
                    None => {
                        steps.push(ReplayStep {
                            seq: record.seq,
                            command: record.command.clone(),
                            status: "not_comparable".into(),
                            detail: "The command produced no comparable result object.".into(),
                        });
                        continue;
                    }
                };
                let diffs = compare_results(&fresh, &record.result, tol);
                if diffs.is_empty() {
                    reproduced += 1;
                    steps.push(ReplayStep {
                        seq: record.seq,
                        command: record.command.clone(),
                        status: "reproduced".into(),
                        detail: format!(
                            "All {} compared statistics match within tolerance.",
                            count_numbers(&record.result)
                        ),
                    });
                } else {
                    failed += 1;
                    steps.push(ReplayStep {
                        seq: record.seq,
                        command: record.command.clone(),
                        status: "error".into(),
                        detail: format!(
                            "{} statistic(s) differ: {}",
                            diffs.len(),
                            diffs.join("; ")
                        ),
                    });
                }
            }
            Err(e) => {
                failed += 1;
                steps.push(ReplayStep {
                    seq: record.seq,
                    command: record.command.clone(),
                    status: "error".into(),
                    detail: e.what.clone(),
                });
            }
        }
    }

    Ok(ReproducibilityReport {
        project: project_name.to_string(),
        software_version: numeris_core::VERSION.to_string(),
        commands_replayed: records.len(),
        reproduced,
        failed,
        all_reproduced: failed == 0,
        steps,
    })
}

/// Recursively compare numeric leaves of two JSON results.
fn compare_results(
    fresh: &serde_json::Value,
    reference: &serde_json::Value,
    tol: Tolerances,
) -> Vec<String> {
    let mut diffs = Vec::new();
    compare_walk(fresh, reference, "", tol, &mut diffs);
    diffs
}

fn compare_walk(
    fresh: &serde_json::Value,
    reference: &serde_json::Value,
    path: &str,
    tol: Tolerances,
    diffs: &mut Vec<String>,
) {
    match (fresh, reference) {
        (serde_json::Value::Number(a), serde_json::Value::Number(b)) => {
            if let (Some(x), Some(y)) = (a.as_f64(), b.as_f64()) {
                let is_se = path.contains("se") || path.contains("standard");
                let tol_here = if is_se {
                    tol.se_relative
                } else {
                    tol.coefficient_relative
                };
                let denom = y.abs().max(1e-300);
                if (x - y).abs() > tol_here.max(tol.statistic_absolute) * denom.max(1.0) {
                    diffs.push(format!("{path}: {x} vs reference {y}"));
                }
            }
        }
        (serde_json::Value::Object(a), serde_json::Value::Object(b)) => {
            for (k, v) in b {
                if let Some(fv) = a.get(k) {
                    let child = if path.is_empty() {
                        k.clone()
                    } else {
                        format!("{path}.{k}")
                    };
                    compare_walk(fv, v, &child, tol, diffs);
                }
            }
        }
        (serde_json::Value::Array(a), serde_json::Value::Array(b)) => {
            for (i, v) in b.iter().enumerate() {
                if let Some(fv) = a.get(i) {
                    compare_walk(fv, v, &format!("{path}[{i}]"), tol, diffs);
                }
            }
        }
        _ => {}
    }
}

fn count_numbers(v: &serde_json::Value) -> usize {
    match v {
        serde_json::Value::Number(_) => 1,
        serde_json::Value::Object(m) => m.values().map(count_numbers).sum(),
        serde_json::Value::Array(a) => a.iter().map(count_numbers).sum(),
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DATA: &str = "wage,education\n20,12\n25,14\n18,12\n22,13\n30,16\n28,15\n19,12\n26,14\n";

    #[test]
    fn replay_reproduces_stored_results() {
        let dir = std::env::temp_dir().join(format!("numeris-replay-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let registry = dir.join("registry.jsonl");
        let _ = std::fs::remove_file(&registry);

        // Record two models.
        let (df, _) = numeris_core::csv::read_csv(DATA).unwrap();
        let mut ex = Executor::with_frame(df);
        for (seq, cmd) in [
            (1u64, "regress wage education"),
            (2, "regress wage education, robust"),
        ] {
            let out = ex.execute(cmd).unwrap();
            let record =
                crate::registry::record_from_output(seq, Some("ds1"), 1, cmd, &out).unwrap();
            crate::registry::append_record(&registry, &record).unwrap();
        }

        // Replay: fresh executor on the same data.
        let report = replay("Wage Study", &registry, DATA, Tolerances::default()).unwrap();
        assert_eq!(report.commands_replayed, 2);
        assert!(report.all_reproduced, "steps: {:#?}", report.steps);
        assert_eq!(report.reproduced, 2);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn replay_detects_changed_data() {
        let dir = std::env::temp_dir().join(format!("numeris-replay2-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let registry = dir.join("registry.jsonl");
        let _ = std::fs::remove_file(&registry);

        let (df, _) = numeris_core::csv::read_csv(DATA).unwrap();
        let mut ex = Executor::with_frame(df);
        let out = ex.execute("regress wage education").unwrap();
        let record =
            crate::registry::record_from_output(1, Some("ds1"), 1, "regress wage education", &out)
                .unwrap();
        crate::registry::append_record(&registry, &record).unwrap();

        // Mutated data: same shape, different values.
        let mutated = DATA.replace("20,", "24,");
        let report = replay("Wage Study", &registry, &mutated, Tolerances::default()).unwrap();
        assert!(!report.all_reproduced);
        assert_eq!(report.failed, 1);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
