//! # numeris-stats
//!
//! The deterministic statistical engine of Numeris:
//!
//! - [`linalg`]: dense matrices, Householder QR, Jacobi eigenvalues
//! - [`vcov`]: classical / HC0–HC3 / cluster / Newey–West HAC covariance
//! - [`ols`]: OLS and WLS with a shared structured result
//! - [`iv`]: 2SLS instrumental variables with first-stage diagnostics
//! - [`ttest`]: t-tests, one-way ANOVA, Pearson/Spearman correlation
//! - [`panel`]: fixed effects (within), between, difference-in-differences
//! - [`glm`]: logit, probit, Poisson (IRLS / Fisher scoring)
//! - [`diagnostics`]: VIF, Breusch–Pagan, RESET, Durbin–Watson, Cook's D

pub mod diagnostics;
pub mod glm;
pub mod iv;
pub mod linalg;
pub mod ols;
pub mod panel;
pub mod ttest;
pub mod vcov;

pub use glm::{fit_glm, Family, GlmResult};
pub use iv::{fit_2sls, IvResult};
pub use linalg::{Matrix, QR};
pub use ols::{Ols, Regression};
pub use panel::{between, did, fixed_effects, DidResult, PanelResult};
pub use ttest::{
    corr_matrix, one_way_anova, pearson, rank_average, t_test_one_sample, t_test_paired,
    t_test_two_sample, AnovaResult, CorrResult, GroupSummary, TTestKind, TTestResult,
};
pub use vcov::{ClusterId, VcovSpec};
