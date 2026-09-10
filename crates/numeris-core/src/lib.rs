//! # numeris-core
//!
//! Core data structures and numerical foundations of Numeris:
//!
//! - [`frame`]: the columnar `DataFrame`, variables, and metadata
//! - [`csv`]: independent RFC 4180 CSV import/export with type inference
//! - [`describe`]: descriptive statistics (moments, quantiles, frequencies)
//! - [`dist`]: Normal / t / F / chi-square distributions and special functions
//! - [`error`]: the structured what/why/action error model

pub mod csv;
pub mod describe;
pub mod dist;
pub mod error;
pub mod frame;

pub use error::{ErrorKind, NumerisError, Result};

/// The product version of the engine (recorded in analysis records and the
/// environment manifest for reproducibility).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
