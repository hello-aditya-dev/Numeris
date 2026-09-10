//! # numeris-command
//!
//! The Numeris command language:
//!
//! - lexer: tokenizer
//! - parser: recursive-descent parser producing the Command AST
//! - render: canonical command rendering (GUI ⇄ command equivalence)
//! - exec: the single execution path over the statistical engine
//!
//! GUI forms build the same AST the parser produces; both run through
//! `Executor::execute_command`. There is no second execution path.

pub mod ast;
pub mod exec;
pub mod lexer;
pub mod parser;
pub mod render;

pub use exec::{
    ComparisonResult, EstimateResult, EstimateSummary, ExecOutput, Executor, StoredEstimate,
};
pub use parser::parse;
pub use render::render;
