//! Debugger trait for VM callbacks
//!
//! This module defines the Debugger trait that the VM uses to communicate
//! with an attached debugger. The trait provides hooks for various execution
//! events that the debugger can intercept.

use crate::bytecode::Value;
use crate::lexer::SourceLocation;
use super::state::DebugState;

/// Debugger callback trait - implemented by InteractiveDebugger
///
/// The VM calls these methods at various points during execution to allow
/// the debugger to inspect state and control execution flow.
pub trait Debugger: Send {
    /// Called before each instruction executes
    ///
    /// # Arguments
    /// * `ip` - Current instruction pointer
    /// * `location` - Source location of the instruction (if available)
    ///
    /// # Returns
    /// `true` to continue execution, `false` to pause
    fn on_instruction(&mut self, ip: usize, location: Option<SourceLocation>) -> bool;

    /// Called when entering a function/subroutine
    ///
    /// # Arguments
    /// * `name` - Name of the procedure being called
    /// * `args` - Arguments passed to the procedure
    fn on_function_call(&mut self, name: &str, args: &[Value]);

    /// Called when returning from a function/subroutine
    ///
    /// # Arguments
    /// * `name` - Name of the procedure returning
    /// * `result` - Return value (if any)
    fn on_function_return(&mut self, name: &str, result: Option<&Value>);

    /// Called when a variable is modified
    ///
    /// # Arguments
    /// * `name` - Name of the variable
    /// * `old` - Previous value (None if uninitialized)
    /// * `new` - New value
    fn on_variable_changed(&mut self, name: &str, old: Option<&Value>, new: &Value);

    /// Check if debugger wants to pause execution
    fn should_pause(&self) -> bool;

    /// Get current debug state
    fn state(&self) -> DebugState;

    /// Set the current source location (for display purposes)
    fn set_current_location(&mut self, location: Option<SourceLocation>);

    /// Get the current call depth (for step over/out tracking)
    fn call_depth(&self) -> usize;

    /// Increment call depth when entering a function
    fn enter_function(&mut self);

    /// Decrement call depth when leaving a function
    fn leave_function(&mut self);

    /// Called when entering a loop (for profiling)
    ///
    /// # Arguments
    /// * `line` - Source line of the loop
    fn on_loop_start(&mut self, _line: usize) {}

    /// Called on each loop iteration (for profiling)
    ///
    /// # Arguments
    /// * `line` - Source line of the loop
    fn on_loop_iteration(&mut self, _line: usize) {}

    /// Called when exiting a loop (for profiling)
    ///
    /// # Arguments
    /// * `line` - Source line of the loop
    fn on_loop_end(&mut self, _line: usize) {}

    /// Called when memory is allocated (arrays, derived types)
    ///
    /// # Arguments
    /// * `line` - Source line of the allocation
    /// * `size` - Estimated size in bytes
    /// * `name` - Variable name if known
    fn on_allocation(&mut self, _line: usize, _size: usize, _name: Option<&str>) {}
}

/// A no-op debugger that does nothing (for when debugging is disabled)
pub struct NoOpDebugger;

impl Debugger for NoOpDebugger {
    fn on_instruction(&mut self, _ip: usize, _location: Option<SourceLocation>) -> bool {
        true // Always continue
    }

    fn on_function_call(&mut self, _name: &str, _args: &[Value]) {}

    fn on_function_return(&mut self, _name: &str, _result: Option<&Value>) {}

    fn on_variable_changed(&mut self, _name: &str, _old: Option<&Value>, _new: &Value) {}

    fn should_pause(&self) -> bool {
        false
    }

    fn state(&self) -> DebugState {
        DebugState::Running
    }

    fn set_current_location(&mut self, _location: Option<SourceLocation>) {}

    fn call_depth(&self) -> usize {
        0
    }

    fn enter_function(&mut self) {}

    fn leave_function(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noop_debugger_always_continues() {
        let mut debugger = NoOpDebugger;
        assert!(debugger.on_instruction(0, None));
        assert!(!debugger.should_pause());
        assert_eq!(debugger.state(), DebugState::Running);
    }
}
