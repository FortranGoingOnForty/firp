//! Diagnostic system for rich error reporting
//!
//! This module provides:
//! - SourceMap for storing source files and retrieving lines
//! - DiagnosticRenderer for rich error display with colors and source context
//! - Error codes for unique error identification
//! - Unified Diagnostic type for all compiler errors

pub mod source;
pub mod render;
pub mod codes;
pub mod unified;
pub mod suggest;
pub mod warning;

pub use source::{SourceMap, SourceFile};
pub use render::{Diagnostic, DiagnosticRenderer, Severity, RenderStyle};
pub use codes::ErrorCode;
pub use unified::{DiagnosticBag, FirpError};
pub use suggest::{find_suggestions, format_suggestions, suggest_similar_identifiers};
pub use warning::{WarningConfig, WarningLevel};
