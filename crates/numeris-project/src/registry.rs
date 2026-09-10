//! # Research registry
//!
//! An append-only JSONL file of analysis records. Historical analyses are
//! never silently overwritten: changing a model creates a new record, so
//! lineage is always preserved.

use numeris_command::ExecOutput;
use numeris_core::error::{NumerisError, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// One immutable analysis record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisRecord {
    pub seq: u64,
    pub dataset_id: Option<String>,
    pub dataset_version: u64,
    /// Canonical command text.
    pub command: String,
    /// Full structured result serialized as JSON.
    pub result: serde_json::Value,
    /// Human-readable result kind (ols/glm/panel/iv/did/…).
    pub kind: String,
    pub software_version: String,
}

/// Append a record to `registry.jsonl`.
pub fn append_record(path: &Path, record: &AnalysisRecord) -> Result<()> {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| {
            NumerisError::io(
                format!("Could not open the analysis registry: {e}"),
                "The registry file (registry.jsonl) could not be opened for appending.",
                "Check that the project directory is writable.",
            )
        })?;
    let line = serde_json::to_string(record)?;
    writeln!(file, "{line}")?;
    Ok(())
}

/// Read all records from the registry.
pub fn load_records(path: &Path) -> Result<Vec<AnalysisRecord>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = std::fs::read_to_string(path)?;
    let mut out = Vec::new();
    for (lineno, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let record: AnalysisRecord = serde_json::from_str(line).map_err(|e| {
            NumerisError::io(
                format!(
                    "The analysis registry contains an unreadable record at line {}.",
                    lineno + 1
                ),
                format!("JSON error: {e}"),
                "The registry may be corrupted; restore it from a backup or start a new project.",
            )
        })?;
        out.push(record);
    }
    Ok(out)
}

/// Build a record from an executor output.
pub fn record_from_output(
    seq: u64,
    dataset_id: Option<&str>,
    dataset_version: u64,
    command: &str,
    output: &ExecOutput,
) -> Option<AnalysisRecord> {
    match output {
        ExecOutput::Estimate { id, result, .. } => {
            let value = serde_json::to_value(result).ok()?;
            Some(AnalysisRecord {
                seq,
                dataset_id: dataset_id.map(|s| s.to_string()),
                dataset_version,
                command: command.to_string(),
                result: value,
                kind: format!("{}:{id}", result.kind()),
                software_version: numeris_core::VERSION.to_string(),
            })
        }
        ExecOutput::TTest(r) => Some(AnalysisRecord {
            seq,
            dataset_id: dataset_id.map(|s| s.to_string()),
            dataset_version,
            command: command.to_string(),
            result: serde_json::to_value(&**r).ok()?,
            kind: "ttest".to_string(),
            software_version: numeris_core::VERSION.to_string(),
        }),
        ExecOutput::Anova(r) => Some(AnalysisRecord {
            seq,
            dataset_id: dataset_id.map(|s| s.to_string()),
            dataset_version,
            command: command.to_string(),
            result: serde_json::to_value(&**r).ok()?,
            kind: "anova".to_string(),
            software_version: numeris_core::VERSION.to_string(),
        }),
        ExecOutput::Summary(stats) => Some(AnalysisRecord {
            seq,
            dataset_id: dataset_id.map(|s| s.to_string()),
            dataset_version,
            command: command.to_string(),
            result: serde_json::to_value(stats).ok()?,
            kind: "summarize".to_string(),
            software_version: numeris_core::VERSION.to_string(),
        }),
        ExecOutput::Corr(c) => Some(AnalysisRecord {
            seq,
            dataset_id: dataset_id.map(|s| s.to_string()),
            dataset_version,
            command: command.to_string(),
            result: serde_json::to_value(&**c).ok()?,
            kind: "correlate".to_string(),
            software_version: numeris_core::VERSION.to_string(),
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use numeris_command::Executor;

    #[test]
    fn registry_appends_and_reads_back() {
        let dir = std::env::temp_dir().join(format!("numeris-reg-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("registry.jsonl");
        let _ = std::fs::remove_file(&path);

        let text = "wage,education\n20,12\n25,14\n18,12\n22,13\n30,16\n";
        let (df, _) = numeris_core::csv::read_csv(text).unwrap();
        let mut ex = Executor::with_frame(df);
        let out = ex.execute("regress wage education").unwrap();
        let record = record_from_output(1, Some("ds1"), 1, "regress wage education", &out).unwrap();
        append_record(&path, &record).unwrap();
        let out2 = ex.execute("regress wage education, robust").unwrap();
        let record2 =
            record_from_output(2, Some("ds1"), 1, "regress wage education, robust", &out2).unwrap();
        append_record(&path, &record2).unwrap();

        let records = load_records(&path).unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].seq, 1);
        assert_eq!(records[1].seq, 2);
        assert!(records[1].command.contains("robust"));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
