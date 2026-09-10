//! # Columnar data frame
//!
//! Numeris's in-memory dataset. Columns are stored as contiguous vectors
//! (`Vec<Option<f64>>` / `Vec<Option<String>>`) so that statistical kernels
//! can scan them without pointer chasing. Missing values are explicit `None`.

use crate::error::{NumerisError, Result};
use serde::{Deserialize, Serialize};

/// Storage type of a column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageType {
    Numeric,
    Text,
}

/// Semantic role of a variable, used by the UI and the Data Quality Lab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticType {
    /// Continuous numeric measurement.
    Continuous,
    /// Categorical (few distinct text or numeric values).
    Categorical,
    /// Row identifier.
    Identifier,
    /// Date/time or time index.
    Temporal,
    /// Not yet classified.
    Unknown,
}

/// Variable metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Variable {
    pub name: String,
    #[serde(default)]
    pub label: String,
    pub storage: StorageType,
    pub semantic: SemanticType,
}

/// A single column of data. Missing values are `None`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Column {
    Numeric(Vec<Option<f64>>),
    Text(Vec<Option<String>>),
}

impl Column {
    pub fn storage(&self) -> StorageType {
        match self {
            Column::Numeric(_) => StorageType::Numeric,
            Column::Text(_) => StorageType::Text,
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Column::Numeric(v) => v.len(),
            Column::Text(v) => v.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// The dataset: variables + columns, column-major.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DataFrame {
    pub variables: Vec<Variable>,
    pub columns: Vec<Column>,
    pub(crate) n_rows: usize,
}

impl DataFrame {
    /// Set the row count when constructing a frame column-by-column
    /// (used by the CSV importer).
    pub fn set_row_count(&mut self, n: usize) {
        self.n_rows = n;
    }
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a numeric column with optional missing values.
    pub fn add_numeric(&mut self, name: &str, values: Vec<Option<f64>>) -> Result<()> {
        self.push_variable(
            Variable {
                name: name.to_string(),
                label: String::new(),
                storage: StorageType::Numeric,
                semantic: SemanticType::Continuous,
            },
            Column::Numeric(values),
        )
    }

    /// Add a text column with optional missing values.
    pub fn add_text(&mut self, name: &str, values: Vec<Option<String>>) -> Result<()> {
        self.push_variable(
            Variable {
                name: name.to_string(),
                label: String::new(),
                storage: StorageType::Text,
                semantic: SemanticType::Unknown,
            },
            Column::Text(values),
        )
    }

    fn push_variable(&mut self, variable: Variable, column: Column) -> Result<()> {
        if self.variables.is_empty() {
            self.n_rows = column.len();
        } else if column.len() != self.n_rows {
            return Err(NumerisError::invalid_request(
                format!(
                    "Column '{}' has {} values but the dataset has {} rows.",
                    variable.name,
                    column.len(),
                    self.n_rows
                ),
                "All columns in a dataset must have the same number of rows.",
                "Check the source data for a ragged column and re-import.",
            ));
        }
        if self.var_index(&variable.name).is_some() {
            return Err(NumerisError::invalid_request(
                format!(
                    "A variable named '{}' already exists in the dataset.",
                    variable.name
                ),
                "Variable names must be unique within a dataset.",
                "Choose a different name or rename the existing variable first.",
            ));
        }
        self.variables.push(variable);
        self.columns.push(column);
        Ok(())
    }

    pub fn n_rows(&self) -> usize {
        self.n_rows
    }

    pub fn n_cols(&self) -> usize {
        self.variables.len()
    }

    pub fn var_names(&self) -> Vec<String> {
        self.variables.iter().map(|v| v.name.clone()).collect()
    }

    pub fn var_index(&self, name: &str) -> Option<usize> {
        self.variables.iter().position(|v| v.name == name)
    }

    pub fn variable(&self, name: &str) -> Result<&Variable> {
        self.variables
            .iter()
            .find(|v| v.name == name)
            .ok_or_else(|| self.unknown_variable(name))
    }

    fn unknown_variable(&self, name: &str) -> NumerisError {
        let mut known: Vec<String> = self.var_names();
        known.sort();
        NumerisError::invalid_request(
            format!("Variable '{name}' was not found in the dataset."),
            format!(
                "The dataset loaded in this session contains {} variables: {}.",
                known.len(),
                if known.is_empty() {
                    "(none)".to_string()
                } else {
                    known.join(", ")
                }
            ),
            "Check the spelling of the variable name or run 'describe' to list variables.",
        )
    }

    pub fn column(&self, name: &str) -> Result<&Column> {
        let idx = self
            .var_index(name)
            .ok_or_else(|| self.unknown_variable(name))?;
        Ok(&self.columns[idx])
    }

    pub fn column_mut(&mut self, name: &str) -> Result<&mut Column> {
        let idx = self
            .var_index(name)
            .ok_or_else(|| self.unknown_variable(name))?;
        Ok(&mut self.columns[idx])
    }

    /// Numeric column as a slice of optional values.
    pub fn numeric_col(&self, name: &str) -> Result<&[Option<f64>]> {
        let col = self.column(name)?;
        match col {
            Column::Numeric(v) => Ok(v),
            Column::Text(_) => Err(NumerisError::invalid_request(
                format!("Variable '{name}' is a text variable."),
                "This operation requires a numeric variable.",
                "Convert the variable to numeric first (for example: 'destring' or a recode), then rerun.",
            )),
        }
    }

    /// Text column as a slice of optional values.
    pub fn text_col(&self, name: &str) -> Result<&[Option<String>]> {
        let col = self.column(name)?;
        match col {
            Column::Text(v) => Ok(v),
            Column::Numeric(_) => Err(NumerisError::invalid_request(
                format!("Variable '{name}' is a numeric variable."),
                "This operation requires a text (string) variable.",
                "Choose a text variable, or convert the numeric variable to text first.",
            )),
        }
    }

    /// Rows where every listed variable is non-missing (listwise deletion).
    /// Returns row indices sorted ascending.
    pub fn complete_case_indices(&self, names: &[&str]) -> Result<Vec<usize>> {
        if names.is_empty() {
            return Ok((0..self.n_rows).collect());
        }
        let mut cols: Vec<&Column> = Vec::with_capacity(names.len());
        for name in names {
            cols.push(self.column(name)?);
        }
        let mut idx = Vec::with_capacity(self.n_rows);
        'row: for i in 0..self.n_rows {
            for col in &cols {
                let present = match col {
                    Column::Numeric(v) => v[i].is_some(),
                    Column::Text(v) => v[i].as_deref().map(|s| !s.is_empty()).unwrap_or(false),
                };
                if !present {
                    continue 'row;
                }
            }
            idx.push(i);
        }
        Ok(idx)
    }

    /// Gather a numeric vector over the given row indices.
    pub fn numeric_vector(&self, name: &str, indices: &[usize]) -> Result<Vec<f64>> {
        let col = self.numeric_col(name)?;
        let mut out = Vec::with_capacity(indices.len());
        for &i in indices {
            out.push(col[i].ok_or_else(|| {
                NumerisError::numerical_failure(
                    "Internal error: missing value escaped row filtering.",
                    "The complete-case filter should have removed this row.",
                    "Please report this as a bug with the command and dataset.",
                )
            })?);
        }
        Ok(out)
    }

    /// Gather a dense numeric matrix (rows = indices, cols = names).
    pub fn numeric_matrix(&self, names: &[&str], indices: &[usize]) -> Result<Vec<Vec<f64>>> {
        let mut cols = Vec::with_capacity(names.len());
        for name in names {
            cols.push(self.numeric_vector(name, indices)?);
        }
        let mut out = Vec::with_capacity(indices.len());
        for r in 0..indices.len() {
            let row: Vec<f64> = cols.iter().map(|c| c[r]).collect();
            out.push(row);
        }
        Ok(out)
    }

    /// Gather a text vector over the given row indices (missing -> error context upstream).
    pub fn text_vector(&self, name: &str, indices: &[usize]) -> Result<Vec<String>> {
        let col = self.text_col(name)?;
        let mut out = Vec::with_capacity(indices.len());
        for &i in indices {
            out.push(col[i].clone().ok_or_else(|| {
                NumerisError::numerical_failure(
                    "Internal error: missing value escaped row filtering.",
                    "The complete-case filter should have removed this row.",
                    "Please report this as a bug with the command and dataset.",
                )
            })?);
        }
        Ok(out)
    }

    /// Rename a variable.
    pub fn rename(&mut self, old: &str, new: &str) -> Result<()> {
        if self.var_index(new).is_some() {
            return Err(NumerisError::invalid_request(
                format!(
                    "Cannot rename '{old}' to '{new}': a variable named '{new}' already exists."
                ),
                "Variable names must be unique within a dataset.",
                "Choose another name.",
            ));
        }
        let err = self.unknown_variable(old);
        let v = self
            .variables
            .iter_mut()
            .find(|v| v.name == old)
            .ok_or(err)?;
        v.name = new.to_string();
        Ok(())
    }

    /// Drop variables by name.
    pub fn drop_variables(&mut self, names: &[&str]) -> Result<()> {
        for name in names {
            self.variable(name)?;
        }
        let drop: Vec<String> = names.iter().map(|s| s.to_string()).collect();
        let mut i = 0;
        while i < self.variables.len() {
            if drop.contains(&self.variables[i].name) {
                self.variables.remove(i);
                self.columns.remove(i);
            } else {
                i += 1;
            }
        }
        Ok(())
    }

    /// Keep only the named variables.
    pub fn keep_variables(&mut self, names: &[&str]) -> Result<()> {
        for name in names {
            self.variable(name)?;
        }
        let keep: Vec<String> = names.iter().map(|s| s.to_string()).collect();
        let mut i = 0;
        while i < self.variables.len() {
            if !keep.contains(&self.variables[i].name) {
                self.variables.remove(i);
                self.columns.remove(i);
            } else {
                i += 1;
            }
        }
        Ok(())
    }

    /// Stable sort rows by a variable (missing values last).
    pub fn sort_by(&mut self, name: &str, descending: bool) -> Result<()> {
        let idx = self
            .var_index(name)
            .ok_or_else(|| self.unknown_variable(name))?;
        let n = self.n_rows;
        let mut order: Vec<usize> = (0..n).collect();
        {
            let key = &self.columns[idx];
            order.sort_by(|&a, &b| {
                let av = key_present(key, a);
                let bv = key_present(key, b);
                // Missing values always last regardless of direction.
                match (av, bv) {
                    (false, true) => return std::cmp::Ordering::Greater,
                    (true, false) => return std::cmp::Ordering::Less,
                    (false, false) => return std::cmp::Ordering::Equal,
                    (true, true) => {}
                }
                let ord = column_cmp(key, a, b);
                if descending {
                    ord.reverse()
                } else {
                    ord
                }
            });
        }
        for col in self.columns.iter_mut() {
            reorder_column(col, &order);
        }
        Ok(())
    }
}

fn key_present(col: &Column, i: usize) -> bool {
    match col {
        Column::Numeric(v) => v[i].is_some(),
        Column::Text(v) => v[i].is_some(),
    }
}

fn column_cmp(col: &Column, a: usize, b: usize) -> std::cmp::Ordering {
    match col {
        Column::Numeric(v) => v[a].partial_cmp(&v[b]).unwrap_or(std::cmp::Ordering::Equal),
        Column::Text(v) => v[a].cmp(&v[b]),
    }
}

fn reorder_column(col: &mut Column, order: &[usize]) {
    match col {
        Column::Numeric(v) => {
            let old = std::mem::take(v);
            *v = order.iter().map(|&i| old[i]).collect();
        }
        Column::Text(v) => {
            let old = std::mem::take(v);
            *v = order.iter().map(|&i| old[i].clone()).collect();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame() -> DataFrame {
        let mut df = DataFrame::new();
        df.add_numeric("x", vec![Some(3.0), None, Some(1.0), Some(2.0)])
            .unwrap();
        df.add_text(
            "g",
            vec![Some("b".into()), Some("a".into()), None, Some("a".into())],
        )
        .unwrap();
        df
    }

    #[test]
    fn complete_case_filter_works() {
        let df = frame();
        let idx = df.complete_case_indices(&["x", "g"]).unwrap();
        assert_eq!(idx, vec![0, 3]);
    }

    #[test]
    fn unknown_variable_error_lists_known_variables() {
        let df = frame();
        let err = df.variable("nope").unwrap_err();
        assert!(err.what.contains("nope"));
        // Known variables are listed in sorted order for determinism.
        assert!(err.why.contains("g, x"));
    }

    #[test]
    fn sort_numeric_desc_missing_last() {
        let mut df = frame();
        df.sort_by("x", true).unwrap();
        let x: Vec<Option<f64>> = df.numeric_col("x").unwrap().to_vec();
        assert_eq!(x, vec![Some(3.0), Some(2.0), Some(1.0), None]);
        let g: Vec<Option<String>> = df.text_col("g").unwrap().to_vec();
        assert_eq!(g[0].as_deref(), Some("b"));
        // The x-missing row (g = "a") is moved last, after all present rows.
        assert_eq!(g[3].as_deref(), Some("a"));
    }

    #[test]
    fn rename_and_drop() {
        let mut df = frame();
        df.rename("x", "wage").unwrap();
        assert!(df.var_index("wage").is_some());
        df.drop_variables(&["wage"]).unwrap();
        assert_eq!(df.var_names(), vec!["g".to_string()]);
    }
}
