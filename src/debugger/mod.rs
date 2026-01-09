//! Interactive Debugger for FIRP
//!
//! This module provides interactive debugging capabilities for the FIRP
//! Fortran interpreter. Features include:
//!
//! - Breakpoints (line-based and function-based)
//! - Execution control (continue, step, next, finish)
//! - Variable inspection
//! - Source display
//! - Call stack navigation
//!
//! # Usage
//!
//! The debugger integrates with the VM through the `Debugger` trait. When
//! debugging is enabled, the VM calls debugger hooks at various points
//! during execution.
//!
//! ```ignore
//! use firp::debugger::InteractiveDebugger;
//! use firp::vm::VM;
//!
//! let mut vm = VM::new();
//! let debugger = InteractiveDebugger::new();
//! vm.set_debugger(Some(Box::new(debugger)));
//! ```

pub mod state;
pub mod traits;
pub mod breakpoint;
pub mod inspector;
pub mod commands;
pub mod display;
pub mod interactive;
pub mod session;
pub mod eval;
pub mod watch;
pub mod trace;

// Re-export commonly used types
pub use state::{DebugAction, DebugState, PauseReason, StepMode};
pub use traits::{Debugger, NoOpDebugger};
pub use breakpoint::{Breakpoint, BreakpointLocation, BreakpointManager};
pub use inspector::{DebugInspector, ValueFormatter, VariableInfo};
pub use commands::{parse_command, help_text, DebugCommand, ParseError};
pub use display::SourceDisplay;
pub use interactive::InteractiveDebugger;
pub use session::{DebugSession, DebugError};
