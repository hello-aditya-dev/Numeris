//! # numeris-project
//!
//! The reproducible-research layer of Numeris:
//!
//! - [`project`]: the `.numeris` project directory format
//! - [`registry`]: the append-only analysis record (research registry)
//! - [`replay`]: the reproducibility engine — re-runs recorded commands and
//!   verifies results against stored references under documented tolerances
//! - [`environment`]: the environment manifest

pub mod environment;
pub mod project;
pub mod registry;
pub mod replay;

pub use environment::EnvironmentManifest;
pub use project::{DatasetRecord, Discipline, Project, ProjectMetadata};
pub use replay::{replay, ReplayStep, ReproducibilityReport, Tolerances};
