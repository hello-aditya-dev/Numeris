//! # CSV import (RFC 4180)
//!
//! Independent implementation of a CSV reader: quoted fields, escaped quotes,
//! embedded newlines/commas, BOM handling, delimiter detection, numeric type
//! inference, and ragged-row reporting. No third-party parser is used.

use crate::error::{NumerisError, Result};
use crate::frame::{Column, DataFrame, SemanticType, StorageType, Variable};

/// A cell value as read from the file.
#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    /// Empty field (treated as missing).
    Empty,
    /// Unquoted text or quoted text.
    Text(String),
}

/// Issues found during import. Warnings never abort the import; they are
/// surfaced to the user in the import report.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct ImportReport {
    pub rows: usize,
    pub columns: usize,
    pub delimiter: char,
    pub issues: Vec<String>,
}

/// Parse a CSV document into cells. RFC 4180 with lenient newline handling.
fn parse_csv(input: &str, delimiter: char) -> Vec<Vec<Cell>> {
    let mut rows: Vec<Vec<Cell>> = Vec::new();
    let mut row: Vec<Cell> = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut field_started = false;
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if in_quotes {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    field.push('"');
                } else {
                    in_quotes = false;
                }
            } else {
                field.push(c);
            }
        } else if c == '"' && field.is_empty() && !field_started {
            in_quotes = true;
            field_started = true;
        } else if c == delimiter {
            push_field(&mut row, &mut field, &mut field_started);
        } else if c == '\r' {
            // Handle \r\n and lone \r as record separators.
            if chars.peek() == Some(&'\n') {
                chars.next();
            }
            push_field(&mut row, &mut field, &mut field_started);
            finish_row(&mut rows, &mut row);
        } else if c == '\n' {
            push_field(&mut row, &mut field, &mut field_started);
            finish_row(&mut rows, &mut row);
        } else {
            field_started = true;
            field.push(c);
        }
    }
    // Final field/row without trailing newline.
    if field_started || !field.is_empty() {
        push_field(&mut row, &mut field, &mut field_started);
    }
    finish_row(&mut rows, &mut row);
    rows
}

fn push_field(row: &mut Vec<Cell>, field: &mut String, started: &mut bool) {
    if *started || !field.is_empty() {
        row.push(Cell::Text(std::mem::take(field)));
    } else {
        row.push(Cell::Empty);
    }
    *started = false;
}

fn finish_row(rows: &mut Vec<Vec<Cell>>, row: &mut Vec<Cell>) {
    // A single empty cell means a blank line, and an empty row is the
    // trailing artifact after the final newline: skip both.
    let is_blank = row.is_empty() || (row.len() == 1 && matches!(row[0], Cell::Empty));
    if !is_blank {
        rows.push(std::mem::take(row));
    }
}

/// Detect the delimiter by counting candidate separators in the first line.
fn detect_delimiter(input: &str) -> char {
    let first_line = input.split(['\n', '\r']).next().unwrap_or("");
    let quoted = first_line.starts_with('"');
    let line = if quoted {
        // Take up to the closing quote of the first field to avoid counting
        // separators inside quoted fields.
        first_line.split('"').nth(1).unwrap_or(first_line)
    } else {
        first_line
    };
    let comma = line.matches(',').count();
    let semi = line.matches(';').count();
    let tab = line.matches('\t').count();
    if tab > comma && tab > semi {
        '\t'
    } else if semi > comma {
        ';'
    } else {
        ','
    }
}

fn normalize_name(raw: &str, used: &mut Vec<String>, issues: &mut Vec<String>) -> String {
    let mut name = raw.trim().to_lowercase();
    let mut cleaned = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            cleaned.push(ch);
        } else if ch == ' ' || ch == '-' || ch == '.' {
            cleaned.push('_');
        }
        // drop anything else
    }
    if cleaned.is_empty() {
        cleaned = format!("v{}", used.len() + 1);
    }
    if cleaned
        .chars()
        .next()
        .map(|c| c.is_ascii_digit())
        .unwrap_or(false)
    {
        cleaned = format!("v{cleaned}");
    }
    name = cleaned;
    let mut candidate = name.clone();
    let mut suffix = 1;
    while used.contains(&candidate) {
        suffix += 1;
        candidate = format!("{name}_{suffix}");
    }
    if candidate != name {
        issues.push(format!(
            "Duplicate column name '{}' was renamed to '{}'.",
            name, candidate
        ));
    }
    used.push(candidate.clone());
    candidate
}

/// Parse CSV text into a DataFrame. First row is the header.
pub fn read_csv(text: &str) -> Result<(DataFrame, ImportReport)> {
    // Strip BOM.
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let delimiter = detect_delimiter(text);
    let rows = parse_csv(text, delimiter);
    let mut issues = Vec::new();

    if rows.is_empty() {
        return Err(NumerisError::invalid_request(
            "The file contains no data rows.",
            "No comma-separated values could be parsed from the file.",
            "Check that the file is a valid CSV with a header row.",
        ));
    }

    let header: Vec<Cell> = rows[0].clone();
    let n_cols = header.len();
    let mut used_names: Vec<String> = Vec::new();
    let names: Vec<String> = header
        .iter()
        .map(|c| match c {
            Cell::Text(t) => normalize_name(t, &mut used_names, &mut issues),
            Cell::Empty => normalize_name("", &mut used_names, &mut issues),
        })
        .collect();

    let data = &rows[1..];
    let mut issues_pad = 0usize;
    let mut numeric: Vec<Vec<Option<f64>>> = (0..n_cols)
        .map(|_| Vec::with_capacity(data.len()))
        .collect();
    let mut is_numeric: Vec<bool> = vec![true; n_cols];
    let mut text: Vec<Vec<Option<String>>> = (0..n_cols)
        .map(|_| Vec::with_capacity(data.len()))
        .collect();

    for (r, row) in data.iter().enumerate() {
        if row.len() != n_cols {
            issues_pad += 1;
            if issues_pad <= 5 {
                issues.push(format!(
                    "Row {} has {} fields but the header has {}; extra fields were dropped or missing fields were left empty.",
                    r + 2,
                    row.len(),
                    n_cols
                ));
            }
        }
        for c in 0..n_cols {
            let cell = row.get(c);
            match cell {
                Some(Cell::Text(t)) => {
                    let t = t.trim();
                    if is_missing_token(t) {
                        numeric[c].push(None);
                        text[c].push(None);
                    } else if is_numeric[c] {
                        match parse_f64_lenient(t) {
                            Some(v) => {
                                numeric[c].push(Some(v));
                                text[c].push(None);
                            }
                            None => {
                                // First non-numeric value: this column is text.
                                is_numeric[c] = false;
                                text[c].push(Some(t.to_string()));
                            }
                        }
                    } else {
                        text[c].push(Some(t.to_string()));
                    }
                }
                _ => {
                    numeric[c].push(None);
                    text[c].push(None);
                }
            }
        }
    }

    if issues_pad > 5 {
        issues.push(format!(
            "…and {} more rows with a different field count.",
            issues_pad - 5
        ));
    }

    let mut df = DataFrame::new();
    for c in 0..n_cols {
        if is_numeric[c] {
            let mut variable = Variable {
                name: names[c].clone(),
                label: String::new(),
                storage: StorageType::Numeric,
                semantic: SemanticType::Continuous,
            };
            let mut sorted_vals: Vec<f64> = numeric[c].iter().filter_map(|v| *v).collect();
            sorted_vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let uniq = if sorted_vals.is_empty() {
                0
            } else {
                1 + (1..sorted_vals.len())
                    .filter(|i| sorted_vals[*i] != sorted_vals[*i - 1])
                    .count()
            };
            let non_missing = numeric[c].iter().filter(|v| v.is_some()).count();
            if uniq <= 10 && uniq > 0 && non_missing > 0 && all_integer(&numeric[c]) {
                variable.semantic = SemanticType::Categorical;
            }
            df.variables.push(variable);
            df.columns
                .push(Column::Numeric(std::mem::take(&mut numeric[c])));
        } else {
            // Non-numeric: keep text only.
            let column_text: Vec<Option<String>> = text[c].clone();
            let uniq = column_text
                .iter()
                .filter(|v| v.is_some())
                .collect::<std::collections::BTreeSet<_>>()
                .len();
            let variable = Variable {
                name: names[c].clone(),
                label: String::new(),
                storage: StorageType::Text,
                semantic: if uniq <= 10 {
                    SemanticType::Categorical
                } else {
                    SemanticType::Identifier
                },
            };
            df.variables.push(variable);
            df.columns.push(Column::Text(column_text));
        }
    }
    df.set_row_count(data.len());

    let report = ImportReport {
        rows: data.len(),
        columns: n_cols,
        delimiter,
        issues,
    };
    Ok((df, report))
}

fn all_integer(col: &[Option<f64>]) -> bool {
    col.iter().all(|v| match v {
        Some(x) => x.fract() == 0.0 && x.abs() < 9.007199254740992e15,
        None => true,
    })
}

/// Recognized missing-value tokens (case-insensitive).
fn is_missing_token(t: &str) -> bool {
    matches!(
        t.to_ascii_lowercase().as_str(),
        "" | "na" | "n/a" | "nan" | "." | "null" | "none"
    )
}

/// Parse a numeric cell. Returns None when the text is not a number
/// (missing tokens are handled separately by `is_missing_token`).
fn parse_f64_lenient(t: &str) -> Option<f64> {
    let v = t.parse::<f64>().ok()?;
    if v.is_nan() {
        None
    } else {
        Some(v)
    }
}

/// Serialize a DataFrame back to CSV.
pub fn write_csv(df: &DataFrame) -> String {
    let mut out = String::new();
    let names: Vec<String> = df.variables.iter().map(|v| v.name.clone()).collect();
    out.push_str(&names.join(","));
    out.push('\n');
    for i in 0..df.n_rows() {
        let mut row: Vec<String> = Vec::with_capacity(df.n_cols());
        for col in &df.columns {
            let cell = match col {
                Column::Numeric(v) => match v[i] {
                    Some(x) => {
                        let s = format_number(x);
                        if s.contains(',') || s.contains('"') || s.contains('\n') {
                            format!("\"{}\"", s.replace('"', "\"\""))
                        } else {
                            s
                        }
                    }
                    None => String::new(),
                },
                Column::Text(v) => match &v[i] {
                    Some(s) if s.contains(',') || s.contains('"') || s.contains('\n') => {
                        format!("\"{}\"", s.replace('"', "\"\""))
                    }
                    Some(s) => s.clone(),
                    None => String::new(),
                },
            };
            row.push(cell);
        }
        out.push_str(&row.join(","));
        out.push('\n');
    }
    out
}

fn format_number(x: f64) -> String {
    if x == x.trunc() && x.abs() < 1e15 {
        format!("{}", x as i64)
    } else {
        let s = format!("{x}");
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_quoted_fields_and_embedded_separators() {
        let text = "name,age,note\n\"Doe, Jane\",34,\"said \"\"hi\"\" twice\"\nBob,29,plain\n";
        let (df, report) = read_csv(text).unwrap();
        assert_eq!(report.rows, 2);
        let name = df.text_col("name").unwrap();
        assert_eq!(name[0].as_deref(), Some("Doe, Jane"));
        let note = df.text_col("note").unwrap();
        assert_eq!(note[0].as_deref(), Some("said \"hi\" twice"));
        let age = df.numeric_col("age").unwrap();
        assert_eq!(age[1], Some(29.0));
    }

    #[test]
    fn type_inference_numeric_vs_text() {
        let text = "a,b\n1,x\n2,y\n3,z\n";
        let (df, _) = read_csv(text).unwrap();
        assert!(df.column("a").unwrap().storage() == crate::frame::StorageType::Numeric);
        assert!(df.column("b").unwrap().storage() == crate::frame::StorageType::Text);
    }

    #[test]
    fn missing_values_and_ragged_rows() {
        let text = "x,y\n1,\n,2\n3,4,99\n";
        let (df, report) = read_csv(text).unwrap();
        assert_eq!(df.n_rows(), 3);
        let x = df.numeric_col("x").unwrap().to_vec();
        assert_eq!(x[1], None);
        assert!(!report.issues.is_empty());
    }

    #[test]
    fn semicolon_delimiter_detected() {
        let text = "x;y\n1;2\n3;4\n";
        let (_, report) = read_csv(text).unwrap();
        assert_eq!(report.delimiter, ';');
    }

    #[test]
    fn round_trip_csv() {
        let text = "x,g\n1.5,a\n2.5,\"b,c\"\n";
        let (df, _) = read_csv(text).unwrap();
        let out = write_csv(&df);
        let (df2, _) = read_csv(&out).unwrap();
        assert_eq!(df2.numeric_col("x").unwrap()[0], Some(1.5));
        assert_eq!(df2.text_col("g").unwrap()[1].as_deref(), Some("b,c"));
    }

    #[test]
    fn bom_and_na_tokens_are_missing() {
        let text = "\u{feff}x,y\nNA,1\n.,2\n";
        let (df, _) = read_csv(text).unwrap();
        assert_eq!(df.numeric_col("x").unwrap()[0], None);
        assert_eq!(df.numeric_col("x").unwrap()[1], None);
    }
}
