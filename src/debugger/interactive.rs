//! Interactive debugger implementation
//!
//! This module provides the main InteractiveDebugger struct that implements
//! the Debugger trait and orchestrates all debugging functionality.

use crate::bytecode::Value;
use crate::lexer::SourceLocation;

use super::breakpoint::BreakpointManager;
use super::display::SourceDisplay;
use super::state::{DebugState, PauseReason, StepMode};
use super::traits::Debugger;

/// The interactive debugger implementation
pub struct InteractiveDebugger {
    /// Current debug state
    state: DebugState,
    /// Breakpoint manager
    breakpoints: BreakpointManager,
    /// Source display
    source_display: Option<SourceDisplay>,
    /// Current source location
    current_location: Option<SourceLocation>,
    /// Current call depth (for step over/out)
    current_call_depth: usize,
    /// Call depth when step over started
    step_over_depth: Option<usize>,
    /// Call depth when step out started
    step_out_depth: Option<usize>,
    /// Last line we stopped at (for detecting line changes)
    last_stopped_line: Option<usize>,
}

impl InteractiveDebugger {
    /// Create a new interactive debugger
    pub fn new() -> Self {
        Self {
            state: DebugState::default(),
            breakpoints: BreakpointManager::new(),
            source_display: None,
            current_location: None,
            current_call_depth: 0,
            step_over_depth: None,
            step_out_depth: None,
            last_stopped_line: None,
        }
    }

    /// Set the source code for display
    pub fn set_source(&mut self, source: &str) {
        self.source_display = Some(SourceDisplay::new(source));
    }

    /// Set source code with file name
    pub fn set_source_with_file(&mut self, source: &str, file_name: &str) {
        self.source_display = Some(SourceDisplay::with_file(source, file_name));
    }

    /// Get the breakpoint manager
    pub fn breakpoints(&mut self) -> &mut BreakpointManager {
        &mut self.breakpoints
    }

    /// Get the source display
    pub fn source_display(&self) -> Option<&SourceDisplay> {
        self.source_display.as_ref()
    }

    /// Get current location
    pub fn current_location(&self) -> Option<&SourceLocation> {
        self.current_location.as_ref()
    }

    /// Get current debug state
    pub fn get_state(&self) -> DebugState {
        self.state
    }

    /// Update current location
    pub fn update_location(&mut self, location: Option<SourceLocation>) {
        self.current_location = location;
    }

    /// Continue execution
    pub fn continue_execution(&mut self) {
        self.state = DebugState::Running;
        self.step_over_depth = None;
        self.step_out_depth = None;
    }

    /// Step into next statement
    pub fn step_into(&mut self) {
        self.state = DebugState::Stepping(StepMode::Into);
        self.step_over_depth = None;
        self.step_out_depth = None;
        self.last_stopped_line = self.current_location.as_ref().map(|l| l.line);
    }

    /// Step over (skip function calls)
    pub fn step_over(&mut self) {
        self.state = DebugState::Stepping(StepMode::Over);
        self.step_over_depth = Some(self.current_call_depth);
        self.step_out_depth = None;
        self.last_stopped_line = self.current_location.as_ref().map(|l| l.line);
    }

    /// Step out (run until function returns)
    pub fn step_out(&mut self) {
        if self.current_call_depth > 0 {
            self.state = DebugState::Stepping(StepMode::Out);
            self.step_over_depth = None;
            self.step_out_depth = Some(self.current_call_depth - 1);
        } else {
            // Already at top level, just continue
            self.continue_execution();
        }
    }

    /// Run the program
    pub fn run(&mut self) {
        self.state = DebugState::Running;
    }

    /// Stop execution
    pub fn stop(&mut self) {
        self.state = DebugState::Finished;
    }

    /// Pause at current location
    pub fn pause(&mut self, reason: PauseReason) {
        self.state = DebugState::Paused(reason);
    }

    /// Check if we're currently paused
    pub fn is_paused(&self) -> bool {
        matches!(self.state, DebugState::Paused(_))
    }

    /// Check if execution is finished
    pub fn is_finished(&self) -> bool {
        matches!(self.state, DebugState::Finished)
    }

    /// Format where we are for display
    pub fn format_where(&self) -> String {
        if let (Some(display), Some(loc)) = (&self.source_display, &self.current_location) {
            display.where_am_i(loc)
        } else if let Some(loc) = &self.current_location {
            format!("Stopped at line {}:{}", loc.line, loc.column)
        } else {
            "Location unknown".to_string()
        }
    }

    /// List source code around current location or specified range
    pub fn list_source(&self, start: Option<usize>, end: Option<usize>) -> String {
        if let Some(display) = &self.source_display {
            match (start, end) {
                (Some(s), Some(e)) => display.list_range(s, e),
                (Some(s), None) => display.list_around(s, None),
                (None, None) => {
                    if let Some(loc) = &self.current_location {
                        display.list_around(loc.line, None)
                    } else {
                        display.list_range(1, 10)
                    }
                }
                (None, Some(e)) => display.list_range(1, e),
            }
        } else {
            "  <no source available>".to_string()
        }
    }
}

impl Default for InteractiveDebugger {
    fn default() -> Self {
        Self::new()
    }
}

impl Debugger for InteractiveDebugger {
    fn on_instruction(&mut self, _ip: usize, location: Option<SourceLocation>) -> bool {
        // Update current location
        if let Some(loc) = &location {
            self.current_location = Some(*loc);
        }

        // Check current state
        match self.state {
            DebugState::Running => {
                // Check for breakpoints
                if let Some(loc) = &location {
                    if let Some(bp_id) = self.breakpoints.should_break(loc) {
                        self.state = DebugState::Paused(PauseReason::Breakpoint(bp_id));
                        return false; // Pause execution
                    }
                }
                true // Continue execution
            }

            DebugState::Stepping(mode) => {
                let current_line = location.as_ref().map(|l| l.line);

                match mode {
                    StepMode::Into => {
                        // Stop on any line change
                        if current_line != self.last_stopped_line {
                            self.state = DebugState::Paused(PauseReason::Step);
                            return false;
                        }
                        true
                    }

                    StepMode::Over => {
                        // Stop when back at same call depth and line changed
                        if let Some(target_depth) = self.step_over_depth {
                            if self.current_call_depth <= target_depth
                                && current_line != self.last_stopped_line
                            {
                                self.state = DebugState::Paused(PauseReason::Step);
                                return false;
                            }
                        }
                        true
                    }

                    StepMode::Out => {
                        // Stop when at lower call depth
                        if let Some(target_depth) = self.step_out_depth {
                            if self.current_call_depth <= target_depth {
                                self.state = DebugState::Paused(PauseReason::Step);
                                return false;
                            }
                        }
                        true
                    }
                }
            }

            DebugState::Paused(_) => {
                // Already paused, stay paused
                false
            }

            DebugState::Finished => {
                // Finished, don't continue
                false
            }
        }
    }

    fn on_function_call(&mut self, _name: &str, _args: &[Value]) {
        // Call tracking is handled by enter_function
    }

    fn on_function_return(&mut self, _name: &str, _result: Option<&Value>) {
        // Return tracking is handled by leave_function
    }

    fn on_variable_changed(&mut self, _name: &str, _old: Option<&Value>, _new: &Value) {
        // Could be used for watchpoints in the future
    }

    fn should_pause(&self) -> bool {
        matches!(self.state, DebugState::Paused(_))
    }

    fn state(&self) -> DebugState {
        self.state
    }

    fn set_current_location(&mut self, location: Option<SourceLocation>) {
        self.current_location = location;
    }

    fn call_depth(&self) -> usize {
        self.current_call_depth
    }

    fn enter_function(&mut self) {
        self.current_call_depth += 1;
    }

    fn leave_function(&mut self) {
        if self.current_call_depth > 0 {
            self.current_call_depth -= 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_debugger_is_paused() {
        let debugger = InteractiveDebugger::new();
        assert!(debugger.is_paused());
        assert_eq!(debugger.state(), DebugState::Paused(PauseReason::Initial));
    }

    #[test]
    fn test_continue_sets_running() {
        let mut debugger = InteractiveDebugger::new();
        debugger.continue_execution();
        assert_eq!(debugger.state(), DebugState::Running);
        assert!(!debugger.is_paused());
    }

    #[test]
    fn test_step_into() {
        let mut debugger = InteractiveDebugger::new();
        debugger.step_into();
        assert_eq!(debugger.state(), DebugState::Stepping(StepMode::Into));
    }

    #[test]
    fn test_step_over() {
        let mut debugger = InteractiveDebugger::new();
        debugger.step_over();
        assert_eq!(debugger.state(), DebugState::Stepping(StepMode::Over));
        assert_eq!(debugger.step_over_depth, Some(0));
    }

    #[test]
    fn test_step_out() {
        let mut debugger = InteractiveDebugger::new();
        // At depth 0, step out just continues
        debugger.step_out();
        assert_eq!(debugger.state(), DebugState::Running);

        // At depth > 0, step out sets target depth
        debugger.current_call_depth = 2;
        debugger.step_out();
        assert_eq!(debugger.state(), DebugState::Stepping(StepMode::Out));
        assert_eq!(debugger.step_out_depth, Some(1));
    }

    #[test]
    fn test_breakpoint_triggers_pause() {
        let mut debugger = InteractiveDebugger::new();
        debugger.breakpoints().add_line(10, None);
        debugger.continue_execution();

        let loc = SourceLocation { line: 10, column: 1 };
        let should_continue = debugger.on_instruction(0, Some(loc));

        assert!(!should_continue);
        assert!(debugger.is_paused());
        if let DebugState::Paused(PauseReason::Breakpoint(id)) = debugger.state() {
            assert_eq!(id, 1);
        } else {
            panic!("Expected breakpoint pause");
        }
    }

    #[test]
    fn test_step_into_stops_on_line_change() {
        let mut debugger = InteractiveDebugger::new();
        debugger.current_location = Some(SourceLocation { line: 5, column: 1 });
        debugger.step_into();

        // Same line - continue
        let loc1 = SourceLocation { line: 5, column: 5 };
        assert!(debugger.on_instruction(0, Some(loc1)));

        // Different line - pause
        let loc2 = SourceLocation { line: 6, column: 1 };
        assert!(!debugger.on_instruction(1, Some(loc2)));
        assert!(debugger.is_paused());
    }

    #[test]
    fn test_call_depth_tracking() {
        let mut debugger = InteractiveDebugger::new();
        assert_eq!(debugger.call_depth(), 0);

        debugger.enter_function();
        assert_eq!(debugger.call_depth(), 1);

        debugger.enter_function();
        assert_eq!(debugger.call_depth(), 2);

        debugger.leave_function();
        assert_eq!(debugger.call_depth(), 1);

        debugger.leave_function();
        assert_eq!(debugger.call_depth(), 0);
    }

    #[test]
    fn test_source_display() {
        let mut debugger = InteractiveDebugger::new();
        debugger.set_source("PROGRAM test\n  x = 10\nEND PROGRAM");

        let display = debugger.source_display().unwrap();
        assert_eq!(display.line_count(), 3);
    }
}
