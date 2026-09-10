//! # Command executor
//!
//! The single execution path shared by the command console, the script
//! editor and the GUI. Every command is parsed into an AST and executed
//! against the session state; results are structured and serializable.

use crate::ast::*;
use numeris_core::csv;
use numeris_core::describe::{self, SummaryStats};
use numeris_core::error::{NumerisError, Result};
use numeris_core::frame::{Column, DataFrame, StorageType, Variable};
use numeris_stats::linalg::Matrix;
use numeris_stats::vcov::{ClusterId, VcovSpec};
use numeris_stats::{
    corr_matrix, diagnostics, fit_2sls, fit_glm, fixed_effects, one_way_anova, t_test_one_sample,
    t_test_paired, t_test_two_sample, AnovaResult, CorrResult, DidResult, Family, GlmResult,
    IvResult, Ols, PanelResult, Regression, TTestResult,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Stored estimate (research registry entry).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredEstimate {
    pub id: String,
    pub label: String,
    pub command: String,
    pub result: EstimateResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EstimateResult {
    Ols(Regression),
    Glm(GlmResult),
    Panel(PanelResult),
    Iv(IvResult),
    Did(Box<DidResult>),
}

impl EstimateResult {
    pub fn kind(&self) -> &'static str {
        match self {
            EstimateResult::Ols(_) => "ols",
            EstimateResult::Glm(_) => "glm",
            EstimateResult::Panel(_) => "panel",
            EstimateResult::Iv(_) => "iv",
            EstimateResult::Did(_) => "did",
        }
    }

    pub fn terms(&self) -> &[String] {
        match self {
            EstimateResult::Ols(r) => &r.terms,
            EstimateResult::Glm(r) => &r.terms,
            EstimateResult::Panel(r) => &r.terms,
            EstimateResult::Iv(r) => &r.terms,
            EstimateResult::Did(r) => &r.terms,
        }
    }

    pub fn n(&self) -> usize {
        match self {
            EstimateResult::Ols(r) => r.n,
            EstimateResult::Glm(r) => r.n,
            EstimateResult::Panel(r) => r.n,
            EstimateResult::Iv(r) => r.n,
            EstimateResult::Did(r) => r.n,
        }
    }
}

/// The output of executing one command.
/// Variant sizes differ by design: results are structured payloads while
/// most commands produce short messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
pub enum ExecOutput {
    DatasetChanged {
        message: String,
    },
    DataLoaded {
        message: String,
        report: csv::ImportReport,
        variables: Vec<Variable>,
    },
    Summary(Vec<SummaryStats>),
    Describe(Vec<Variable>),
    Frequency {
        variable: String,
        rows: Vec<describe::FrequencyRow>,
    },
    Corr(Box<CorrResult>),
    TTest(Box<TTestResult>),
    Anova(Box<AnovaResult>),
    Estimate {
        id: String,
        label: String,
        result: EstimateResult,
        diagnostics: Vec<diagnostics::DiagnosticItem>,
    },
    EstimateList(Vec<EstimateSummary>),
    Compare(Box<ComparisonResult>),
    Notes(Vec<String>),
    Message(String),
    Help(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstimateSummary {
    pub id: String,
    pub label: String,
    pub command: String,
    pub kind: String,
    pub n: usize,
    pub fit: String,
}

/// Model comparison output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonResult {
    pub ids: Vec<String>,
    pub labels: Vec<String>,
    pub terms: Vec<String>,
    pub coef: Vec<Vec<Option<f64>>>,
    pub se: Vec<Vec<Option<f64>>>,
    pub p: Vec<Vec<Option<f64>>>,
    pub n: Vec<usize>,
    pub fit: Vec<String>,
}

/// The session: dataset + estimate store + notes.
pub struct Executor {
    pub df: DataFrame,
    pub estimates: Vec<StoredEstimate>,
    pub notes: Vec<String>,
    next_model: usize,
    dataset_loaded: bool,
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}

impl Executor {
    pub fn new() -> Self {
        Self {
            df: DataFrame::new(),
            estimates: Vec::new(),
            notes: Vec::new(),
            next_model: 1,
            dataset_loaded: false,
        }
    }

    pub fn with_frame(df: DataFrame) -> Self {
        Self {
            df,
            estimates: Vec::new(),
            notes: Vec::new(),
            next_model: 1,
            dataset_loaded: true,
        }
    }

    fn require_data(&self) -> Result<()> {
        if !self.dataset_loaded || self.df.n_rows() == 0 {
            return Err(NumerisError::insufficient_data(
                "No dataset is loaded in this session.",
                "Commands that operate on data require a dataset to be loaded first.",
                "Load data with: use \"path/to/file.csv\".",
            ));
        }
        Ok(())
    }

    /// Execute a command string (parse + run).
    pub fn execute(&mut self, source: &str) -> Result<ExecOutput> {
        let cmd = crate::parser::parse(source)?;
        self.execute_command(cmd)
    }

    /// Execute a parsed command. This is the shared path used by the GUI
    /// (which builds the AST from a form) and the command language.
    pub fn execute_command(&mut self, cmd: Command) -> Result<ExecOutput> {
        match cmd {
            Command::Use { path } => self.load_file(&path),
            Command::Summarize {
                variables,
                detail: _,
            } => {
                self.require_data()?;
                let vars = if variables.is_empty() {
                    self.df.var_names()
                } else {
                    variables
                };
                let mut stats = Vec::new();
                for v in vars {
                    stats.push(describe::summarize_variable(&self.df, &v)?);
                }
                Ok(ExecOutput::Summary(stats))
            }
            Command::Describe { variables } => {
                self.require_data()?;
                let vars = if variables.is_empty() {
                    self.df.variables.clone()
                } else {
                    variables
                        .iter()
                        .map(|v| self.df.variable(v).cloned())
                        .collect::<Result<Vec<_>>>()?
                };
                Ok(ExecOutput::Describe(vars))
            }
            Command::Correlate {
                variables,
                spearman,
            } => {
                self.require_data()?;
                let vars = if variables.is_empty() {
                    self.df
                        .variables
                        .iter()
                        .filter(|v| v.storage == StorageType::Numeric)
                        .map(|v| v.name.clone())
                        .collect()
                } else {
                    variables
                };
                if vars.len() < 2 {
                    return Err(NumerisError::invalid_request(
                        "correlate requires at least two variables.",
                        "A correlation matrix needs two or more numeric variables.",
                        "List at least two variables, for example: correlate wage education.",
                    ));
                }
                let idx = self
                    .df
                    .complete_case_indices(&vars.iter().map(|s| s.as_str()).collect::<Vec<_>>())?;
                if idx.len() < 2 {
                    return Err(NumerisError::insufficient_data(
                        "Fewer than two complete observations are available for the requested variables.",
                        "Listwise deletion removed all rows because of missing values.",
                        "Check missingness with 'summarize' and choose variables with more complete data.",
                    ));
                }
                let m = self
                    .df
                    .numeric_matrix(&vars.iter().map(|s| s.as_str()).collect::<Vec<_>>(), &idx)?;
                let matrix = Matrix::from_rows(&m)?;
                let result = corr_matrix(&matrix, &vars, spearman)?;
                Ok(ExecOutput::Corr(Box::new(result)))
            }
            Command::Ttest { variable, form } => {
                self.require_data()?;
                match form {
                    TtestForm::OneSample { mu } => {
                        let col = self.df.numeric_col(&variable)?;
                        let xs: Vec<f64> = col.iter().filter_map(|v| *v).collect();
                        describe::require_observations(&variable, xs.len())?;
                        let r = t_test_one_sample(&xs, mu, &variable)?;
                        Ok(ExecOutput::TTest(Box::new(r)))
                    }
                    TtestForm::ByGroup { group, welch } => {
                        let gcol = self.df.column(&group)?;
                        // Numeric group variable: split on 0/1 (or two values).
                        let (labels, values_g): (Vec<String>, Vec<Option<f64>>) = match gcol {
                            Column::Numeric(v) => {
                                let mut distinct: Vec<f64> = v.iter().filter_map(|x| *x).collect();
                                distinct.sort_by(|a, b| {
                                    a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
                                });
                                distinct.dedup();
                                let keys: Vec<f64> = distinct;
                                if keys.len() != 2 {
                                    return Err(NumerisError::invalid_request(
                                        format!("The grouping variable '{group}' must define exactly two groups."),
                                        format!("{} distinct values were found.", keys.len()),
                                        "Choose a 0/1 indicator or a variable with exactly two values.",
                                    ));
                                }
                                let labels: Vec<String> = v
                                    .iter()
                                    .map(|x| x.map(|k| format!("{k}")).unwrap_or_default())
                                    .collect();
                                (labels, v.clone())
                            }
                            Column::Text(v) => {
                                let labels: Vec<String> =
                                    v.iter().map(|x| x.clone().unwrap_or_default()).collect();
                                let dummy: Vec<Option<f64>> =
                                    v.iter().map(|x| x.as_deref().map(|_| 0.0)).collect();
                                (labels, dummy)
                            }
                        };
                        let ycol = self.df.numeric_col(&variable)?;
                        let mut a = Vec::new();
                        let mut b = Vec::new();
                        let group_keys: Vec<String> = {
                            let mut ks: Vec<String> =
                                labels.iter().filter(|l| !l.is_empty()).cloned().collect();
                            ks.sort();
                            ks.dedup();
                            ks
                        };
                        for i in 0..self.df.n_rows() {
                            if labels[i].is_empty() {
                                continue;
                            }
                            if group_keys.len() == 2 && labels[i] == group_keys[1] {
                                if let Some(v) = ycol[i] {
                                    a.push(v);
                                }
                            } else if let Some(v) = ycol[i] {
                                b.push(v);
                            }
                        }
                        let _ = values_g;
                        let r = t_test_two_sample(&a, &b, welch, &group, &variable)?;
                        Ok(ExecOutput::TTest(Box::new(r)))
                    }
                    TtestForm::TwoVariables { other, paired } => {
                        let idx = self
                            .df
                            .complete_case_indices(&[variable.as_str(), other.as_str()])?;
                        let a = self.df.numeric_vector(&variable, &idx)?;
                        let b = self.df.numeric_vector(&other, &idx)?;
                        let r = if paired {
                            t_test_paired(&a, &b, 0.0, &variable, &other)?
                        } else {
                            t_test_two_sample(&a, &b, false, &other, &variable)?
                        };
                        Ok(ExecOutput::TTest(Box::new(r)))
                    }
                }
            }
            Command::Anova { outcome, group } => {
                self.require_data()?;
                let idx = self
                    .df
                    .complete_case_indices(&[outcome.as_str(), group.as_str()])?;
                let values = self.df.numeric_vector(&outcome, &idx)?;
                let gcol = self.df.column(&group)?;
                let labels: Vec<String> = (0..self.df.n_rows())
                    .filter(|&i| idx.contains(&i))
                    .map(|i| match gcol {
                        Column::Numeric(v) => v[i].map(|x| format!("{x}")).unwrap_or_default(),
                        Column::Text(v) => v[i].clone().unwrap_or_default(),
                    })
                    .collect();
                let r = one_way_anova(&values, &labels, &group, &outcome)?;
                Ok(ExecOutput::Anova(Box::new(r)))
            }
            Command::Regress { spec } => {
                let (id, label, result, diags) = self.run_regression(&spec)?;
                Ok(ExecOutput::Estimate {
                    id,
                    label,
                    result,
                    diagnostics: diags,
                })
            }
            Command::Xtreg {
                outcome,
                predictors,
                entity,
                model,
                vcov,
            } => {
                self.require_data()?;
                let mut vars = vec![outcome.as_str()];
                vars.extend(predictors.iter().map(|s| s.as_str()));
                vars.push(entity.as_str());
                let idx = self.df.complete_case_indices(&vars)?;
                let y = self.df.numeric_vector(&outcome, &idx)?;
                let names = predictors.clone();
                let x = Matrix::from_rows(&self.df.numeric_matrix(
                    &predictors.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
                    &idx,
                )?)?;
                let ids = self.encode_cluster(&entity, &idx)?;
                let (spec, cluster_ids) = self.vcov_to_spec(&vcov, &idx)?;
                let result = match model {
                    PanelModel::Fe => {
                        fixed_effects(&y, &x, &ids, &names, &spec, cluster_ids.as_deref())?
                    }
                    PanelModel::Be => numeris_stats::panel::between(
                        &y,
                        &x,
                        &ids,
                        &names,
                        &spec,
                        cluster_ids.as_deref(),
                    )?,
                    PanelModel::Pooled => {
                        let mut builder = Ols::new(&y, &x, &names)?
                            .vcov(spec.clone())
                            .estimator_label("Pooled OLS");
                        if matches!(vcov, VcovOption::Cluster { .. }) {
                            builder =
                                builder.cluster(cluster_ids.clone().unwrap_or_else(|| ids.clone()));
                        }
                        let reg = builder.fit()?;
                        PanelResult {
                            estimator: "Pooled OLS".into(),
                            terms: reg.terms[1..].to_vec(),
                            coef: reg.coef[1..].to_vec(),
                            se: reg.se[1..].to_vec(),
                            t: reg.t[1..].to_vec(),
                            p: reg.p[1..].to_vec(),
                            ci_lo: reg.ci_lo[1..].to_vec(),
                            ci_hi: reg.ci_hi[1..].to_vec(),
                            n: reg.n,
                            g: ids.iter().copied().collect::<BTreeSet<ClusterId>>().len(),
                            dropped_singletons: 0,
                            k: reg.k - 1,
                            df_r: reg.df_r,
                            r2_within: reg.r2,
                            sigma: reg.rmse,
                            vcov_label: reg.vcov_label,
                        }
                    }
                };
                let command = crate::render::render(&Command::Xtreg {
                    outcome,
                    predictors,
                    entity,
                    model,
                    vcov,
                });
                let id = self.store_estimate(
                    &command,
                    &result.terms.to_vec(),
                    EstimateResult::Panel(result.clone()),
                    &format!("Panel: {}", result.estimator),
                );
                Ok(ExecOutput::Estimate {
                    id,
                    label: format!("Panel: {}", result.estimator),
                    result: EstimateResult::Panel(result),
                    diagnostics: Vec::new(),
                })
            }
            Command::Ivregress {
                estimator,
                outcome,
                endogenous,
                instruments,
                exogenous,
                vcov,
            } => {
                self.require_data()?;
                let mut vars = vec![outcome.as_str()];
                vars.extend(endogenous.iter().map(|s| s.as_str()));
                vars.extend(instruments.iter().map(|s| s.as_str()));
                vars.extend(exogenous.iter().map(|s| s.as_str()));
                let idx = self.df.complete_case_indices(&vars)?;
                let y = self.df.numeric_vector(&outcome, &idx)?;
                let e = Matrix::from_rows(&self.df.numeric_matrix(
                    &endogenous.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
                    &idx,
                )?)?;
                let z = Matrix::from_rows(&self.df.numeric_matrix(
                    &instruments.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
                    &idx,
                )?)?;
                let x = Matrix::from_rows(&self.df.numeric_matrix(
                    &exogenous.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
                    &idx,
                )?)?;
                let mut names = endogenous.clone();
                names.extend(exogenous.clone());
                if let VcovOption::Cluster { .. } = vcov {
                    return Err(NumerisError::invalid_request(
                        "Clustered standard errors are not yet supported for ivregress.",
                        "This version implements classical and robust covariance for 2SLS; the cluster-robust variant is planned.",
                        "Use the robust option for now: ivregress 2sls y (x = z), robust.",
                    ));
                }
                let spec = self.vcov_spec_no_cluster(&vcov)?;
                let result = fit_2sls(&y, &e, &x, &z, &names, &spec)?;
                let command = crate::render::render(&Command::Ivregress {
                    estimator,
                    outcome,
                    endogenous,
                    instruments,
                    exogenous,
                    vcov,
                });
                let id = self.store_estimate(
                    &command,
                    &result.terms.clone(),
                    EstimateResult::Iv(result.clone()),
                    "2SLS",
                );
                Ok(ExecOutput::Estimate {
                    id,
                    label: "2SLS".into(),
                    result: EstimateResult::Iv(result),
                    diagnostics: Vec::new(),
                })
            }
            Command::Did {
                outcome,
                controls,
                treat,
                time,
                entity: _entity,
                vcov,
            } => {
                self.require_data()?;
                let mut vars = vec![outcome.as_str(), treat.as_str(), time.as_str()];
                vars.extend(controls.iter().map(|s| s.as_str()));
                let idx = self.df.complete_case_indices(&vars)?;
                let y = self.df.numeric_vector(&outcome, &idx)?;
                let t = self.df.numeric_vector(&treat, &idx)?;
                let p = self.df.numeric_vector(&time, &idx)?;
                let c = Matrix::from_rows(&self.df.numeric_matrix(
                    &controls.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
                    &idx,
                )?)?;
                let (spec, cluster_ids) = self.vcov_to_spec(&vcov, &idx)?;
                let result =
                    numeris_stats::panel::did(&y, &t, &p, &c, &controls, &spec, cluster_ids)?;
                let command = crate::render::render(&Command::Did {
                    outcome,
                    controls,
                    treat,
                    time,
                    entity: _entity,
                    vcov,
                });
                let id = self.store_estimate(
                    &command,
                    &result.terms.clone(),
                    EstimateResult::Did(Box::new(result.clone())),
                    "DID",
                );
                Ok(ExecOutput::Estimate {
                    id,
                    label: "DID".into(),
                    result: EstimateResult::Did(Box::new(result)),
                    diagnostics: Vec::new(),
                })
            }
            Command::Logit { spec } => self.run_glm(spec, Family::Logit),
            Command::Probit { spec } => self.run_glm(spec, Family::Probit),
            Command::Poisson { spec } => self.run_glm(spec, Family::Poisson),
            Command::Generate { name, expr } => {
                self.require_data()?;
                if self.df.var_index(&name).is_some() {
                    return Err(NumerisError::invalid_request(
                        format!("A variable named '{name}' already exists."),
                        "generate creates a new variable; it never overwrites an existing one.",
                        "Choose a new name, or use 'replace' to modify the existing variable.",
                    ));
                }
                let values = self.eval_column(&expr)?;
                let numeric = values.iter().all(|v| v.is_none() || v.is_some());
                if numeric {
                    self.df.add_numeric(&name, values)?;
                } else {
                    // Values are always f64 here; text handled by replace path.
                    self.df.add_numeric(&name, values)?;
                }
                Ok(ExecOutput::DatasetChanged {
                    message: format!("Variable '{name}' created ({} values).", self.df.n_rows()),
                })
            }
            Command::Replace { name, expr, cond } => {
                self.require_data()?;
                let values = self.eval_column(&expr)?;
                // Compute the condition mask before taking a mutable borrow.
                let n = self.df.n_rows();
                let keep: Vec<bool> = match &cond {
                    Some(c) => {
                        let col = self.df.numeric_col(&c.variable)?;
                        (0..n)
                            .map(|i| match col.get(i) {
                                Some(Some(x)) => cond_holds(c, *x),
                                _ => false,
                            })
                            .collect()
                    }
                    None => vec![true; n],
                };
                let col = self.df.column_mut(&name)?;
                match col {
                    Column::Numeric(v) => {
                        for (i, val) in values.into_iter().enumerate() {
                            if keep[i] {
                                v[i] = val;
                            }
                        }
                    }
                    Column::Text(_) => {
                        return Err(NumerisError::invalid_request(
                            format!(
                                "Cannot replace text variable '{name}' with a numeric expression."
                            ),
                            "replace currently applies to numeric variables.",
                            "Convert the variable to numeric first, or choose a numeric variable.",
                        ));
                    }
                }
                Ok(ExecOutput::DatasetChanged {
                    message: format!("Variable '{name}' updated."),
                })
            }
            Command::Drop { targets } => match targets {
                DropTarget::Variables(vars) => {
                    self.require_data()?;
                    self.df
                        .drop_variables(&vars.iter().map(|s| s.as_str()).collect::<Vec<_>>())?;
                    Ok(ExecOutput::DatasetChanged {
                        message: format!("Dropped {} variable(s).", vars.len()),
                    })
                }
                DropTarget::If(cond) => {
                    self.require_data()?;
                    self.filter_rows(&cond, false)?;
                    Ok(ExecOutput::DatasetChanged {
                        message: "Rows not satisfying the condition were dropped.".to_string(),
                    })
                }
            },
            Command::Keep { targets } => match targets {
                KeepTarget::Variables(vars) => {
                    self.require_data()?;
                    self.df
                        .keep_variables(&vars.iter().map(|s| s.as_str()).collect::<Vec<_>>())?;
                    Ok(ExecOutput::DatasetChanged {
                        message: format!("Kept {} variable(s).", vars.len()),
                    })
                }
                KeepTarget::If(cond) => {
                    self.require_data()?;
                    self.filter_rows(&cond, true)?;
                    Ok(ExecOutput::DatasetChanged {
                        message: "Rows not satisfying the condition were removed.".to_string(),
                    })
                }
            },
            Command::Rename { from, to } => {
                self.require_data()?;
                self.df.rename(&from, &to)?;
                Ok(ExecOutput::DatasetChanged {
                    message: format!("Renamed '{from}' to '{to}'."),
                })
            }
            Command::Label { variable, text } => {
                self.require_data()?;
                let v = self
                    .df
                    .variables
                    .iter_mut()
                    .find(|v| v.name == variable)
                    .ok_or_else(|| {
                        NumerisError::invalid_request(
                            format!("Variable '{variable}' was not found in the dataset."),
                            "Labels attach to existing variables.",
                            "Check the variable name with 'describe'.",
                        )
                    })?;
                v.label = text;
                Ok(ExecOutput::DatasetChanged {
                    message: format!("Label set for '{variable}'."),
                })
            }
            Command::Sort {
                variable,
                descending,
            } => {
                self.require_data()?;
                self.df.sort_by(&variable, descending)?;
                Ok(ExecOutput::DatasetChanged {
                    message: format!(
                        "Sorted by '{variable}'{}.",
                        if descending { " (descending)" } else { "" }
                    ),
                })
            }
            Command::EstimateList => {
                let list = self
                    .estimates
                    .iter()
                    .map(|e| EstimateSummary {
                        id: e.id.clone(),
                        label: e.label.clone(),
                        command: e.command.clone(),
                        kind: e.result.kind().to_string(),
                        n: e.result.n(),
                        fit: fit_line(&e.result),
                    })
                    .collect();
                Ok(ExecOutput::EstimateList(list))
            }
            Command::EstimateCompare { ids } => self.compare(&ids),
            Command::Note { text } => {
                self.notes.push(text);
                Ok(ExecOutput::Message("Note recorded.".to_string()))
            }
            Command::Notes => Ok(ExecOutput::Notes(self.notes.clone())),
            Command::ExportTable { path, format, ids } => {
                let content = self.table_export(&format, &ids)?;
                std::fs::write(&path, content.as_bytes()).map_err(|e| {
                    NumerisError::io(
                        format!("Could not write the table to '{path}': {e}"),
                        "The file could not be created at the requested location.",
                        "Check that the directory exists and that you have write permission.",
                    )
                })?;
                Ok(ExecOutput::Message(format!("Table exported to '{path}'.")))
            }
            Command::ExportData { path } => {
                self.require_data()?;
                let content = csv::write_csv(&self.df);
                std::fs::write(&path, content.as_bytes()).map_err(|e| {
                    NumerisError::io(
                        format!("Could not write the dataset to '{path}': {e}"),
                        "The file could not be created at the requested location.",
                        "Check that the directory exists and that you have write permission.",
                    )
                })?;
                Ok(ExecOutput::Message(format!(
                    "Dataset exported to '{path}'."
                )))
            }
            Command::Help { command } => Ok(ExecOutput::Help(help_text(command.as_deref()))),
        }
    }

    fn load_file(&mut self, path: &str) -> Result<ExecOutput> {
        let lower = path.to_lowercase();
        if !lower.ends_with(".csv") {
            return Err(NumerisError::invalid_request(
                format!("The file '{path}' is not a CSV file."),
                "V1 reads CSV directly; other formats (Excel, Stata, SPSS, SAS, Parquet) are planned but not yet supported.",
                "Save the data as CSV and use: use \"file.csv\".",
            ));
        }
        let text = std::fs::read_to_string(path).map_err(|e| {
            NumerisError::io(
                format!("Could not read the file '{path}': {e}"),
                "The file does not exist, is not readable, or is not a text CSV.",
                "Check the path and permissions, then try again.",
            )
        })?;
        let (df, report) = csv::read_csv(&text)?;
        let variables = df.variables.clone();
        let message = format!(
            "Loaded {} observations and {} variables from {}.",
            report.rows,
            report.columns,
            path.rsplit('/').next().unwrap_or(path)
        );
        self.df = df;
        self.dataset_loaded = true;
        Ok(ExecOutput::DataLoaded {
            message,
            report,
            variables,
        })
    }

    fn run_regression(
        &mut self,
        spec: &ModelSpec,
    ) -> Result<(
        String,
        String,
        EstimateResult,
        Vec<diagnostics::DiagnosticItem>,
    )> {
        self.require_data()?;
        if spec.predictors.is_empty() {
            return Err(NumerisError::invalid_request(
                "The regression has no predictors.",
                "regress requires at least one predictor besides the constant.",
                "Add one or more predictor variables to the command.",
            ));
        }
        let mut vars = vec![spec.outcome.as_str()];
        vars.extend(spec.predictors.iter().map(|s| s.as_str()));
        // Note: the cluster variable is deliberately NOT part of the
        // complete-case filter. Rows whose cluster identifier is missing
        // must raise an actionable error (see encode_cluster), not be
        // silently dropped from the estimation sample.
        let idx = self.df.complete_case_indices(&vars)?;
        let y = self.df.numeric_vector(&spec.outcome, &idx)?;
        let x = Matrix::from_rows(
            &self.df.numeric_matrix(
                &spec
                    .predictors
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>(),
                &idx,
            )?,
        )?;
        let (vcov_spec, cluster_ids) = self.vcov_to_spec(&spec.vcov, &idx)?;
        let mut builder = Ols::new(&y, &x, &spec.predictors)?.vcov(vcov_spec);
        if spec.no_constant {
            builder = builder.no_constant();
        }
        if let Some(ids) = cluster_ids {
            builder = builder.cluster(ids);
        }
        if let Some(w) = &spec.weights {
            let wv = self.df.numeric_vector(w, &idx)?;
            builder = builder.weights(wv);
        }
        let reg = builder.fit()?;
        let diags = diagnostics::regression_diagnostics(&reg, &x, &y, !spec.no_constant);
        let command = crate::render::render(&Command::Regress { spec: spec.clone() });
        let label = format!("OLS ({})", reg.vcov_label);
        let id = self.store_estimate(
            &command,
            &reg.terms.clone(),
            EstimateResult::Ols(reg.clone()),
            &label,
        );
        Ok((id, label, EstimateResult::Ols(reg), diags))
    }

    fn run_glm(&mut self, spec: GlmSpec, family: Family) -> Result<ExecOutput> {
        self.require_data()?;
        let vars = {
            let mut v = vec![spec.outcome.as_str()];
            v.extend(spec.predictors.iter().map(|s| s.as_str()));
            v
        };
        let idx = self.df.complete_case_indices(&vars)?;
        let y = self.df.numeric_vector(&spec.outcome, &idx)?;
        let x = Matrix::from_rows(
            &self.df.numeric_matrix(
                &spec
                    .predictors
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>(),
                &idx,
            )?,
        )?;
        let result = fit_glm(&y, &x, &spec.predictors, family, spec.robust)?;
        let command = crate::render::render(&match family {
            Family::Logit => Command::Logit { spec: spec.clone() },
            Family::Probit => Command::Probit { spec: spec.clone() },
            Family::Poisson => Command::Poisson { spec: spec.clone() },
        });
        let label = match family {
            Family::Logit => "Logit".to_string(),
            Family::Probit => "Probit".to_string(),
            Family::Poisson => "Poisson".to_string(),
        };
        let id = self.store_estimate(
            &command,
            &result.terms.clone(),
            EstimateResult::Glm(result.clone()),
            &label,
        );
        let diags = result
            .warnings
            .iter()
            .map(|w| diagnostics::DiagnosticItem {
                name: "Estimation warning".to_string(),
                status: "warning".to_string(),
                detail: w.clone(),
            })
            .collect();
        Ok(ExecOutput::Estimate {
            id,
            label,
            result: EstimateResult::Glm(result),
            diagnostics: diags,
        })
    }

    fn store_estimate(
        &mut self,
        command: &str,
        _terms: &[String],
        result: EstimateResult,
        label: &str,
    ) -> String {
        let id = format!("M{}", self.next_model);
        self.next_model += 1;
        self.estimates.push(StoredEstimate {
            id: id.clone(),
            label: label.to_string(),
            command: command.to_string(),
            result,
        });
        id
    }

    /// Map the command-level vcov option to an engine spec, returning the
    /// cluster ids separately when clustering was requested.
    fn vcov_to_spec(
        &self,
        vcov: &VcovOption,
        idx: &[usize],
    ) -> Result<(VcovSpec, Option<Vec<ClusterId>>)> {
        Ok(match vcov {
            VcovOption::Classical => (VcovSpec::Classical, None),
            VcovOption::Robust => (VcovSpec::Hc1, None),
            VcovOption::Hc2 => (VcovSpec::Hc2, None),
            VcovOption::Hc3 => (VcovSpec::Hc3, None),
            VcovOption::Cluster { variable } => {
                let ids = self.encode_cluster(variable, idx)?;
                (VcovSpec::Cluster, Some(ids))
            }
            VcovOption::Hac { lags } => (VcovSpec::Hac { lags: *lags }, None),
        })
    }

    /// Vcov spec without cluster support (used where clustering is handled
    /// separately or unsupported).
    fn vcov_spec_no_cluster(&self, vcov: &VcovOption) -> Result<VcovSpec> {
        Ok(match vcov {
            VcovOption::Classical => VcovSpec::Classical,
            VcovOption::Robust => VcovSpec::Hc1,
            VcovOption::Hc2 => VcovSpec::Hc2,
            VcovOption::Hc3 => VcovSpec::Hc3,
            VcovOption::Cluster { variable } => {
                return Err(NumerisError::invalid_request(
                    format!("Clustered standard errors by '{variable}' are not supported for this estimator yet."),
                    "This estimator currently supports classical, robust, HC2 and HC3 covariance.",
                    "Use the robust option for now.",
                ));
            }
            VcovOption::Hac { lags } => VcovSpec::Hac { lags: *lags },
        })
    }

    /// Encode a cluster/entity variable into u32 ids over the row indices.
    fn encode_cluster(&self, variable: &str, idx: &[usize]) -> Result<Vec<ClusterId>> {
        let col = self.df.column(variable)?;
        let mut mapping: BTreeMap<String, ClusterId> = BTreeMap::new();
        let mut ids = Vec::with_capacity(idx.len());
        for &i in idx {
            let key = match col {
                Column::Numeric(v) => match v[i] {
                    Some(x) => format!("num:{x}"),
                    None => return Err(missing_cluster_error(variable)),
                },
                Column::Text(v) => match &v[i] {
                    Some(s) if !s.is_empty() => format!("txt:{s}"),
                    _ => return Err(missing_cluster_error(variable)),
                },
            };
            let next = mapping.len() as ClusterId;
            let id = *mapping.entry(key).or_insert(next);
            ids.push(id);
        }
        Ok(ids)
    }

    fn eval_column(&self, expr: &Expr) -> Result<Vec<Option<f64>>> {
        let n = self.df.n_rows();
        let mut out = vec![Some(0.0); n];
        self.eval_into(expr, &mut out)?;
        Ok(out)
    }

    fn eval_into(&self, expr: &Expr, out: &mut [Option<f64>]) -> Result<()> {
        let n = out.len();
        match expr {
            Expr::Number(v) => {
                for slot in out.iter_mut() {
                    *slot = Some(*v);
                }
                Ok(())
            }
            Expr::Var(name) => {
                let col = self.df.numeric_col(name)?;
                for (i, slot) in out.iter_mut().enumerate() {
                    *slot = col[i];
                }
                Ok(())
            }
            Expr::Neg(inner) => {
                let mut tmp = vec![Some(0.0); n];
                self.eval_into(inner, &mut tmp)?;
                for (slot, t) in out.iter_mut().zip(tmp) {
                    *slot = t.map(|v| -v);
                }
                Ok(())
            }
            Expr::BinOp { op, left, right } => {
                let mut l = vec![Some(0.0); n];
                let mut r = vec![Some(0.0); n];
                self.eval_into(left, &mut l)?;
                self.eval_into(right, &mut r)?;
                for (i, slot) in out.iter_mut().enumerate() {
                    *slot = match (l[i], r[i]) {
                        (Some(a), Some(b)) => match op {
                            BinOpKind::Add => Some(a + b),
                            BinOpKind::Sub => Some(a - b),
                            BinOpKind::Mul => Some(a * b),
                            BinOpKind::Div => {
                                if b == 0.0 {
                                    None
                                } else {
                                    Some(a / b)
                                }
                            }
                            BinOpKind::Pow => Some(a.powf(b)),
                        },
                        _ => None,
                    };
                }
                Ok(())
            }
            Expr::Call { function, arg } => match function {
                FnKind::Rank => {
                    // Rank of the argument column (average ties).
                    let mut vals = vec![Some(0.0); n];
                    self.eval_into(arg, &mut vals)?;
                    let dense: Vec<f64> = vals.iter().filter_map(|v| *v).collect();
                    let ranks = numeris_stats::ttest::rank_average(&dense);
                    // Map back to rows.
                    let mut it = ranks.into_iter();
                    for (i, slot) in out.iter_mut().enumerate() {
                        *slot = if vals[i].is_some() { it.next() } else { None };
                    }
                    Ok(())
                }
                FnKind::Standardize => {
                    let mut vals = vec![Some(0.0); n];
                    self.eval_into(arg, &mut vals)?;
                    let dense: Vec<f64> = vals.iter().filter_map(|v| *v).collect();
                    let m = numeris_core::describe::mean(&dense);
                    let s = numeris_core::describe::sd(&dense);
                    for (i, slot) in out.iter_mut().enumerate() {
                        *slot = match (vals[i], m, s) {
                            (Some(v), Some(m), Some(s)) if s > 0.0 => Some((v - m) / s),
                            _ => None,
                        };
                    }
                    Ok(())
                }
                _ => {
                    let mut vals = vec![Some(0.0); n];
                    self.eval_into(arg, &mut vals)?;
                    for (i, slot) in out.iter_mut().enumerate() {
                        *slot = vals[i].and_then(|v| {
                            let r = match function {
                                FnKind::Ln => v.ln(),
                                FnKind::Log => v.log10(),
                                FnKind::Exp => v.exp(),
                                FnKind::Sqrt => v.sqrt(),
                                FnKind::Abs => v.abs(),
                                FnKind::Round => v.round(),
                                _ => unreachable!(),
                            };
                            if r.is_finite() {
                                Some(r)
                            } else {
                                None
                            }
                        });
                    }
                    Ok(())
                }
            },
        }
    }

    fn filter_rows(&mut self, cond: &Cond, keep_matching: bool) -> Result<()> {
        let col = self.df.numeric_col(&cond.variable)?;
        let mut keep = vec![false; self.df.n_rows()];
        for i in 0..self.df.n_rows() {
            let holds = match col[i] {
                Some(x) => cond_holds(cond, x),
                None => false,
            };
            keep[i] = if keep_matching { holds } else { !holds };
        }
        for c in self.df.columns.iter_mut() {
            match c {
                Column::Numeric(v) => {
                    let old = std::mem::take(v);
                    *v = old
                        .into_iter()
                        .zip(&keep)
                        .filter(|(_, k)| **k)
                        .map(|(x, _)| x)
                        .collect();
                }
                Column::Text(v) => {
                    let old = std::mem::take(v);
                    *v = old
                        .into_iter()
                        .zip(&keep)
                        .filter(|(_, k)| **k)
                        .map(|(x, _)| x)
                        .collect();
                }
            }
        }
        let kept = keep.iter().filter(|k| **k).count();
        self.df.set_row_count(kept);
        Ok(())
    }

    fn compare(&self, ids: &[String]) -> Result<ExecOutput> {
        let selected: Vec<&StoredEstimate> = if ids.is_empty() {
            self.estimates.iter().collect()
        } else {
            let mut found = Vec::new();
            for id in ids {
                let est = self
                    .estimates
                    .iter()
                    .find(|e| e.id.eq_ignore_ascii_case(id))
                    .ok_or_else(|| {
                        NumerisError::invalid_request(
                            format!("Estimate '{id}' was not found."),
                            "The stored estimates have different ids.",
                            "Run 'estimate list' to see available estimates.",
                        )
                    })?;
                found.push(est);
            }
            found
        };
        if selected.is_empty() {
            return Err(NumerisError::insufficient_data(
                "No estimates are available to compare.",
                "estimate compare requires at least one stored model.",
                "Run a model first, then compare.",
            ));
        }
        // Union of terms in order of first appearance.
        let mut terms: Vec<String> = Vec::new();
        for e in &selected {
            for t in e.result.terms() {
                if !terms.contains(t) {
                    terms.push(t.clone());
                }
            }
        }
        let mut coef = vec![];
        let mut se = vec![];
        let mut p = vec![];
        for e in &selected {
            let (c, s, pv, _lo, _hi) = table_triple(&e.result);
            let mut row_c = vec![None; terms.len()];
            let mut row_s = vec![None; terms.len()];
            let mut row_p = vec![None; terms.len()];
            for (j, t) in terms.iter().enumerate() {
                if let Some(pos) = e.result.terms().iter().position(|x| x == t) {
                    row_c[j] = Some(c[pos]);
                    row_s[j] = Some(s[pos]);
                    row_p[j] = Some(pv[pos]);
                }
            }
            coef.push(row_c);
            se.push(row_s);
            p.push(row_p);
        }
        let n = selected.iter().map(|e| e.result.n()).collect();
        let fit = selected.iter().map(|e| fit_line(&e.result)).collect();
        Ok(ExecOutput::Compare(Box::new(ComparisonResult {
            ids: selected.iter().map(|e| e.id.clone()).collect(),
            labels: selected.iter().map(|e| e.label.clone()).collect(),
            terms,
            coef,
            se,
            p,
            n,
            fit,
        })))
    }

    /// Build a publication table in the requested format.
    fn table_export(&self, format: &TableFormat, ids: &[String]) -> Result<String> {
        let selected: Vec<&StoredEstimate> = if ids.is_empty() {
            self.estimates.iter().collect()
        } else {
            ids.iter()
                .map(|id| {
                    self.estimates
                        .iter()
                        .find(|e| e.id.eq_ignore_ascii_case(id))
                        .ok_or_else(|| {
                            NumerisError::invalid_request(
                                format!("Estimate '{id}' was not found."),
                                "The requested model is not in the registry.",
                                "Run 'estimate list' to see available estimates.",
                            )
                        })
                })
                .collect::<Result<Vec<_>>>()?
        };
        if selected.is_empty() {
            return Err(NumerisError::insufficient_data(
                "No stored estimates to export.",
                "The table exporter needs at least one fitted model.",
                "Run a model first, then export the table.",
            ));
        }
        let cmp = match self.compare(ids)? {
            ExecOutput::Compare(c) => *c,
            _ => unreachable!(),
        };
        Ok(match format {
            TableFormat::Markdown => table_markdown(&cmp),
            TableFormat::Csv => table_csv(&cmp),
            TableFormat::Latex => table_latex(&cmp),
            TableFormat::Html => table_html(&cmp),
        })
    }
}

fn missing_cluster_error(variable: &str) -> NumerisError {
    NumerisError::invalid_request(
        format!("The cluster variable '{variable}' contains missing values."),
        "Clustered standard errors require every estimation row to have a cluster identifier; missing identifiers cannot be assigned to a cluster.",
        "Resolve the missing cluster identifiers (fill, drop, or choose a different variable) before estimating clustered standard errors.",
    )
}

fn cond_holds(cond: &Cond, x: f64) -> bool {
    match cond.op {
        CondOp::Eq => x == cond.value,
        CondOp::Ne => x != cond.value,
        CondOp::Gt => x > cond.value,
        CondOp::Ge => x >= cond.value,
        CondOp::Lt => x < cond.value,
        CondOp::Le => x <= cond.value,
    }
}

/// Coefficient table columns: (coef, se, p, ci_lo, ci_hi).
type TableColumns = (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>);

fn table_triple(result: &EstimateResult) -> TableColumns {
    match result {
        EstimateResult::Ols(r) => (
            r.coef.clone(),
            r.se.clone(),
            r.p.clone(),
            r.ci_lo.clone(),
            r.ci_hi.clone(),
        ),
        EstimateResult::Glm(r) => (
            r.coef.clone(),
            r.se.clone(),
            r.p.clone(),
            r.ci_lo.clone(),
            r.ci_hi.clone(),
        ),
        EstimateResult::Panel(r) => (
            r.coef.clone(),
            r.se.clone(),
            r.p.clone(),
            r.ci_lo.clone(),
            r.ci_hi.clone(),
        ),
        EstimateResult::Iv(r) => (
            r.coef.clone(),
            r.se.clone(),
            r.p.clone(),
            r.ci_lo.clone(),
            r.ci_hi.clone(),
        ),
        EstimateResult::Did(r) => (
            r.coef.clone(),
            r.se.clone(),
            r.p_values.clone(),
            r.regression.ci_lo.clone(),
            r.regression.ci_hi.clone(),
        ),
    }
}

fn fit_line(result: &EstimateResult) -> String {
    match result {
        EstimateResult::Ols(r) => format!("N = {}, R² = {:.3}", r.n, r.r2),
        EstimateResult::Glm(r) => format!("N = {}, pseudo-R² = {:.3}", r.n, r.pseudo_r2_mcfadden),
        EstimateResult::Panel(r) => format!(
            "N = {}, entities = {}, R²(within) = {:.3}",
            r.n, r.g, r.r2_within
        ),
        EstimateResult::Iv(r) => format!("N = {}", r.n),
        EstimateResult::Did(r) => format!("N = {}", r.n),
    }
}

fn stars(p: f64) -> &'static str {
    if p < 0.001 {
        "***"
    } else if p < 0.01 {
        "**"
    } else if p < 0.05 {
        "*"
    } else {
        ""
    }
}

fn fmt_cell(v: f64) -> String {
    format!("{:.3}", v)
}

fn table_markdown(c: &ComparisonResult) -> String {
    let mut s = String::new();
    s.push_str("| Term |");
    for id in &c.ids {
        s.push_str(&format!(" {id} |"));
    }
    s.push('\n');
    s.push_str("|---|");
    for _ in &c.ids {
        s.push_str("---:|");
    }
    s.push('\n');
    for (j, term) in c.terms.iter().enumerate() {
        s.push_str(&format!("| {term} |"));
        for m in 0..c.ids.len() {
            match (c.coef[m][j], c.p[m][j]) {
                (Some(co), Some(p)) => s.push_str(&format!(" {}{} |", fmt_cell(co), stars(p))),
                _ => s.push_str("  |"),
            }
        }
        s.push('\n');
        s.push_str("|  |");
        for m in 0..c.ids.len() {
            match c.se[m][j] {
                Some(se) => s.push_str(&format!(" ({}) |", fmt_cell(se))),
                None => s.push_str("  |"),
            }
        }
        s.push('\n');
    }
    s.push_str("| N |");
    for &n in &c.n {
        s.push_str(&format!(" {n} |"));
    }
    s.push('\n');
    for (m, fit) in c.fit.iter().enumerate() {
        if m == 0 {
            s.push_str("| Fit |");
        } else {
            s.push_str("|  |");
        }
        s.push_str(&format!(" {fit} |"));
    }
    s.push_str("\n\n*Significance: \\*\\*\\* p<0.001, \\*\\* p<0.01, \\* p<0.05.*\n");
    s
}

fn table_csv(c: &ComparisonResult) -> String {
    let mut s = String::new();
    s.push_str("term");
    for id in &c.ids {
        s.push_str(&format!(",{id}_coef,{id}_se,{id}_p"));
    }
    s.push('\n');
    for (j, term) in c.terms.iter().enumerate() {
        s.push_str(term);
        for m in 0..c.ids.len() {
            let co = c.coef[m][j].map(|v| format!("{v}")).unwrap_or_default();
            let se = c.se[m][j].map(|v| format!("{v}")).unwrap_or_default();
            let p = c.p[m][j].map(|v| format!("{v}")).unwrap_or_default();
            s.push_str(&format!(",{co},{se},{p}"));
        }
        s.push('\n');
    }
    s.push('N');
    for &n in &c.n {
        s.push_str(&format!(",{n},,"));
    }
    s.push('\n');
    s
}

fn table_latex(c: &ComparisonResult) -> String {
    let mut s = String::new();
    s.push_str("% Numeris exported comparison table\n\\begin{tabular}{l");
    for _ in &c.ids {
        s.push('c');
    }
    s.push_str("}\n\\toprule\n");
    s.push_str(" & ");
    let heads: Vec<String> = c.ids.iter().map(|i| format!("({i})")).collect();
    s.push_str(&heads.join(" & "));
    s.push_str(" \\\\\n\\midrule\n");
    for (j, term) in c.terms.iter().enumerate() {
        let name = latex_escape(term);
        s.push_str(&name);
        for m in 0..c.ids.len() {
            match (c.coef[m][j], c.p[m][j]) {
                (Some(co), Some(p)) => s.push_str(&format!(" & {}{}", fmt_cell(co), stars(p))),
                _ => s.push_str(" & "),
            }
        }
        s.push_str(" \\\\\n");
        s.push_str(" & ");
        for m in 0..c.ids.len() {
            match c.se[m][j] {
                Some(se) => s.push_str(&format!(" & ({})", fmt_cell(se))),
                None => s.push_str(" & "),
            }
        }
        s.push_str(" \\\\\n");
    }
    s.push_str("\\midrule\nN");
    for &n in &c.n {
        s.push_str(&format!(" & {n}"));
    }
    s.push_str(" \\\\\n");
    s.push_str("\\bottomrule\n\\end{tabular}\n");
    s
}

fn table_html(c: &ComparisonResult) -> String {
    let mut s = String::from("<table class=\"numeris-table\">\n<thead><tr><th>Term</th>");
    for id in &c.ids {
        s.push_str(&format!("<th>{id}</th>"));
    }
    s.push_str("</tr></thead>\n<tbody>\n");
    for (j, term) in c.terms.iter().enumerate() {
        s.push_str(&format!("<tr><th>{}</th>", html_escape(term)));
        for m in 0..c.ids.len() {
            match (c.coef[m][j], c.p[m][j]) {
                (Some(co), Some(p)) => {
                    s.push_str(&format!("<td>{}{}</td>", fmt_cell(co), stars(p)))
                }
                _ => s.push_str("<td></td>"),
            }
        }
        s.push_str("</tr>\n<tr><td></td>");
        for m in 0..c.ids.len() {
            match c.se[m][j] {
                Some(se) => s.push_str(&format!("<td>({})</td>", fmt_cell(se))),
                None => s.push_str("<td></td>"),
            }
        }
        s.push_str("</tr>\n");
    }
    s.push_str("</tbody>\n</table>\n");
    s
}

fn latex_escape(s: &str) -> String {
    s.replace('_', "\\_")
        .replace('&', "\\&")
        .replace('%', "\\%")
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn help_text(command: Option<&str>) -> String {
    let general = "Numeris command language — quick reference\n\n\
Data:      use \"file.csv\" · describe · summarize [vars] · correlate vars\n\
Tests:     ttest x == 20 · ttest x, by(g) · ttest a == b, paired · anova y g\n\
Models:    regress y x1 x2, [robust|hc2|hc3|cluster(v)|vce(hac L)]\n\
           xtreg y x, fe entity(v) · ivregress 2sls y (endog = z) [controls]\n\
           did y [controls], treat(t) time(p) [cluster(v)]\n\
           logit y xs, robust · probit y xs · poisson y xs\n\
Data ops:  generate name = expr · replace name = expr [if cond]\n\
           drop/keep vars | if cond · rename a b · label variable v \"text\" · sort v [desc]\n\
Research:  estimate list · estimate compare m1 m2 · note \"text\" · notes\n\
Export:    export table \"file.md\" [ids] · export data \"file.csv\"\n\n\
Functions in generate/replace: ln log exp sqrt abs round rank standardize\n\
Comments: // or lines starting with *";
    match command {
        None => general.to_string(),
        Some(c) => match c.to_lowercase().as_str() {
            "regress" => "regress depvar indepvars, [robust|hc2|hc3|cluster(varname)|vce(hac L)|noconstant]\n\nFits OLS (or WLS with weights) by QR decomposition. Default standard errors are classical; 'robust' selects HC1; cluster(varname) computes one-way cluster-robust standard errors with the G/(G-1)·(n-1)/(n-k) correction.".into(),
            "xtreg" => "xtreg depvar indepvars, fe|be|pooled entity(panelvar) [robust|cluster(varname)]\n\nPanel estimators. fe = within (fixed effects, singletons dropped); be = between (entity means); pooled = pooled OLS.".into(),
            "ivregress" => "ivregress 2sls depvar (endogenous = instruments) [exogenous], [robust]\n\nTwo-stage least squares. Reports first-stage F and partial R² for each endogenous regressor; the order condition is checked before estimation.".into(),
            "did" => "did depvar [controls], treat(tvar) time(pvar) [robust|cluster(varname)]\n\nDifference-in-differences with the interaction term treat#post. treat and time must be 0/1.".into(),
            "generate" => "generate newvar = expression\n\nExpression operators: + - * / ^, parentheses, and functions ln log exp sqrt abs round rank standardize. Missing values propagate.".into(),
            "summarize" => "summarize [varlist], [detail]\n\nDescriptive statistics: N, missing, mean, SD, min, quartiles, median, max, skewness, kurtosis. Without a varlist, summarizes every variable.".into(),
            other => format!("No help entry for '{other}'.\n\n{general}"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_frame() -> DataFrame {
        let text = "wage,education,experience,female,firm,year\n\
20.0,12,3,1,1,2018\n\
25.5,14,5,0,1,2018\n\
18.0,12,2,1,1,2019\n\
22.0,13,4,0,2,2018\n\
30.0,16,8,0,2,2019\n\
28.0,15,7,1,2,2019\n\
19.5,12,2,1,3,2018\n\
26.0,14,6,0,3,2019\n\
24.0,13,4,1,3,2019\n\
32.0,17,9,0,3,2019\n";
        let (df, _) = csv::read_csv(text).unwrap();
        df
    }

    #[test]
    fn gui_and_console_produce_identical_models() {
        // The specification's central equivalence requirement.
        let mut ex1 = Executor::with_frame(test_frame());
        let mut ex2 = Executor::with_frame(test_frame());
        // GUI path: build the AST directly (as the GUI form does).
        let gui_cmd = Command::Regress {
            spec: ModelSpec {
                outcome: "wage".into(),
                predictors: vec!["education".into(), "experience".into(), "female".into()],
                vcov: VcovOption::Robust,
                no_constant: false,
                weights: None,
            },
        };
        let out1 = ex1.execute_command(gui_cmd).unwrap();
        // Console path: the equivalent command text.
        let out2 = ex2
            .execute("regress wage education experience female, robust")
            .unwrap();
        let (id1, r1) = match out1 {
            ExecOutput::Estimate {
                id,
                result: EstimateResult::Ols(r),
                ..
            } => (id, r),
            _ => panic!("expected estimate"),
        };
        let (id2, r2) = match out2 {
            ExecOutput::Estimate {
                id,
                result: EstimateResult::Ols(r),
                ..
            } => (id, r),
            _ => panic!("expected estimate"),
        };
        assert_eq!(id1, id2);
        assert_eq!(r1.coef, r2.coef);
        assert_eq!(r1.se, r2.se);
        assert_eq!(r1.p, r2.p);
        assert_eq!(r1.r2, r2.r2);
    }

    #[test]
    fn full_workflow_via_console() {
        let mut ex = Executor::with_frame(test_frame());
        // summarize
        match ex.execute("summarize wage education").unwrap() {
            ExecOutput::Summary(stats) => {
                assert_eq!(stats.len(), 2);
                assert_eq!(stats[0].n, 10);
                assert!(stats[0].mean.unwrap() > 0.0);
            }
            _ => panic!(),
        }
        // correlate
        assert!(matches!(
            ex.execute("correlate wage education").unwrap(),
            ExecOutput::Corr(_)
        ));
        // ttest
        assert!(matches!(
            ex.execute("ttest wage == 20").unwrap(),
            ExecOutput::TTest(_)
        ));
        assert!(matches!(
            ex.execute("ttest wage, by(female)").unwrap(),
            ExecOutput::TTest(_)
        ));
        // anova
        assert!(matches!(
            ex.execute("anova wage firm").unwrap(),
            ExecOutput::Anova(_)
        ));
        // regress + robust + cluster
        assert!(matches!(
            ex.execute("regress wage education experience female")
                .unwrap(),
            ExecOutput::Estimate { .. }
        ));
        assert!(matches!(
            ex.execute("regress wage education experience female, robust")
                .unwrap(),
            ExecOutput::Estimate { .. }
        ));
        assert!(matches!(
            ex.execute("regress wage education experience, cluster(firm)")
                .unwrap(),
            ExecOutput::Estimate { .. }
        ));
        // xtreg
        assert!(matches!(
            ex.execute("xtreg wage education, fe entity(firm)").unwrap(),
            ExecOutput::Estimate { .. }
        ));
        // estimate list/compare
        assert!(
            matches!(ex.execute("estimate list").unwrap(), ExecOutput::EstimateList(l) if l.len() >= 4)
        );
        assert!(matches!(
            ex.execute("estimate compare").unwrap(),
            ExecOutput::Compare(_)
        ));
        // notes
        ex.execute("note \"Baseline models estimated.\"").unwrap();
        assert!(matches!(ex.execute("notes").unwrap(), ExecOutput::Notes(n) if n.len() == 1));
    }

    #[test]
    fn generate_and_filter_work() {
        let mut ex = Executor::with_frame(test_frame());
        ex.execute("generate logwage = ln(wage)").unwrap();
        let logw = ex.df.numeric_col("logwage").unwrap().to_vec();
        assert!((logw[0].unwrap() - 20.0f64.ln()).abs() < 1e-12);
        // replace with condition.
        ex.execute("replace logwage = 0 if wage < 20").unwrap();
        assert_eq!(ex.df.numeric_col("logwage").unwrap()[2], Some(0.0));
        // drop if
        let n_before = ex.df.n_rows();
        ex.execute("drop if wage > 30").unwrap();
        assert!(ex.df.n_rows() < n_before);
        // keep if
        ex.execute("keep if education >= 12").unwrap();
        assert!(ex.df.n_rows() > 0);
    }

    #[test]
    fn missing_cluster_variable_error_is_actionable() {
        let text = "y,x,cl\n1,2,1\n2,3,\n3,4,2\n4,5,1\n";
        let (df, _) = csv::read_csv(text).unwrap();
        let mut ex = Executor::with_frame(df);
        let err = ex.execute("regress y x, cluster(cl)").unwrap_err();
        assert!(err.what.contains("missing values"));
        assert!(err.action.contains("Resolve"));
    }

    #[test]
    fn no_dataset_error_guides_to_use() {
        let mut ex = Executor::new();
        let err = ex.execute("summarize wage").unwrap_err();
        assert!(err.action.contains("use"));
    }

    #[test]
    fn export_table_markdown_round_trip() {
        let mut ex = Executor::with_frame(test_frame());
        ex.execute("regress wage education").unwrap();
        ex.execute("regress wage education experience, robust")
            .unwrap();
        let out = ex.table_export(&TableFormat::Markdown, &[]).unwrap();
        assert!(out.contains("| Term |"));
        assert!(out.contains("M1"));
        assert!(out.contains("M2"));
        assert!(out.contains("***"));
    }

    #[test]
    fn glm_commands_execute() {
        let text = "employed,education,experience\n1,16,4\n0,12,2\n1,15,6\n0,11,1\n1,14,5\n0,13,2\n1,17,8\n0,12,3\n1,16,7\n0,10,2\n";
        let (df, _) = csv::read_csv(text).unwrap();
        let mut ex = Executor::with_frame(df);
        assert!(matches!(
            ex.execute("logit employed education, robust").unwrap(),
            ExecOutput::Estimate { .. }
        ));
        assert!(matches!(
            ex.execute("probit employed education").unwrap(),
            ExecOutput::Estimate { .. }
        ));
    }

    #[test]
    fn did_command_executes() {
        let text = "y,treat,post,firm\n10,0,0,1\n12,0,1,1\n10,1,0,2\n15,1,1,2\n11,0,0,3\n13,0,1,3\n10,1,0,4\n16,1,1,4\n";
        let (df, _) = csv::read_csv(text).unwrap();
        let mut ex = Executor::with_frame(df);
        let out = ex.execute("did y, treat(treat) time(post)").unwrap();
        match out {
            ExecOutput::Estimate {
                result: EstimateResult::Did(r),
                ..
            } => {
                // Hand-verified: treated diff 5.5 − control diff 2.0 = 3.5.
                assert!((r.att - 3.5).abs() < 1e-9, "att = {}", r.att);
            }
            _ => panic!(),
        }
    }
}
