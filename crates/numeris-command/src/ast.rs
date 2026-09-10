//! # Command AST
//!
//! The typed representation of every Numeris command. Both the GUI (which
//! builds an AST from a form) and the command language (which parses text
//! into the same AST) compile to this representation before execution —
//! there is a single execution path.

use serde::{Deserialize, Serialize};

/// A complete parsed command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Command {
    /// Load a data file into the session.
    Use {
        path: String,
    },
    /// Descriptive statistics.
    Summarize {
        variables: Vec<String>,
        detail: bool,
    },
    /// Variable metadata listing.
    Describe {
        variables: Vec<String>,
    },
    /// Correlation matrix (listwise).
    Correlate {
        variables: Vec<String>,
        spearman: bool,
    },
    /// t-tests.
    Ttest {
        variable: String,
        form: TtestForm,
    },
    /// One-way ANOVA.
    Anova {
        outcome: String,
        group: String,
    },
    /// Linear regression (OLS / WLS).
    Regress {
        spec: ModelSpec,
    },
    /// Panel regression.
    Xtreg {
        outcome: String,
        predictors: Vec<String>,
        entity: String,
        model: PanelModel,
        vcov: VcovOption,
    },
    /// Instrumental variables (2SLS).
    Ivregress {
        estimator: String,
        outcome: String,
        endogenous: Vec<String>,
        instruments: Vec<String>,
        exogenous: Vec<String>,
        vcov: VcovOption,
    },
    /// Difference-in-differences.
    Did {
        outcome: String,
        controls: Vec<String>,
        treat: String,
        time: String,
        entity: Option<String>,
        vcov: VcovOption,
    },
    /// Binary/count GLMs.
    Logit {
        spec: GlmSpec,
    },
    Probit {
        spec: GlmSpec,
    },
    Poisson {
        spec: GlmSpec,
    },
    /// Create a new variable from an expression.
    Generate {
        name: String,
        expr: Expr,
    },
    /// Replace an existing variable's values.
    Replace {
        name: String,
        expr: Expr,
        cond: Option<Cond>,
    },
    /// Drop variables or rows.
    Drop {
        targets: DropTarget,
    },
    /// Keep variables or rows.
    Keep {
        targets: KeepTarget,
    },
    /// Rename a variable.
    Rename {
        from: String,
        to: String,
    },
    /// Attach a label to a variable.
    Label {
        variable: String,
        text: String,
    },
    /// Sort rows.
    Sort {
        variable: String,
        descending: bool,
    },
    /// List stored estimates.
    EstimateList,
    /// Compare stored estimates.
    EstimateCompare {
        ids: Vec<String>,
    },
    /// Attach a research note to the project.
    Note {
        text: String,
    },
    /// Show research notes.
    Notes,
    /// Export the active comparison set (or last model) as a table.
    ExportTable {
        path: String,
        format: TableFormat,
        ids: Vec<String>,
    },
    /// Export the dataset.
    ExportData {
        path: String,
    },
    /// Help text.
    Help {
        command: Option<String>,
    },
}

/// Regression specification shared by `regress`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelSpec {
    pub outcome: String,
    pub predictors: Vec<String>,
    pub vcov: VcovOption,
    pub no_constant: bool,
    pub weights: Option<String>,
}

/// GLM specification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GlmSpec {
    pub outcome: String,
    pub predictors: Vec<String>,
    pub robust: bool,
}

/// Variance estimator option for regression-family commands.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VcovOption {
    Classical,
    Robust,
    Hc2,
    Hc3,
    Cluster { variable: String },
    Hac { lags: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PanelModel {
    Fe,
    Be,
    Pooled,
}

/// Forms of the t-test command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TtestForm {
    /// `ttest x == k` (one sample).
    OneSample { mu: f64 },
    /// `ttest x, by(g)` (two-sample; pooled or Welch).
    ByGroup { group: String, welch: bool },
    /// `ttest a == b` (independent two-sample on two variables).
    TwoVariables { other: String, paired: bool },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DropTarget {
    Variables(Vec<String>),
    If(Cond),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum KeepTarget {
    Variables(Vec<String>),
    If(Cond),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TableFormat {
    Markdown,
    Csv,
    Latex,
    Html,
}

/// A single filter condition: `variable op value`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cond {
    pub variable: String,
    pub op: CondOp,
    pub value: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CondOp {
    Eq,
    Ne,
    Gt,
    Ge,
    Lt,
    Le,
}

/// Expressions for `generate` / `replace`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expr {
    Number(f64),
    Var(String),
    BinOp {
        op: BinOpKind,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Neg(Box<Expr>),
    Call {
        function: FnKind,
        arg: Box<Expr>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinOpKind {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FnKind {
    Ln,
    Log,
    Exp,
    Sqrt,
    Abs,
    Round,
    Rank,
    Standardize,
}
