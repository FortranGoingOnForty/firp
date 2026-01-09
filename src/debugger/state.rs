//! Debug state types for the interactive debugger
//!
//! This module defines the core state management types used by the debugger
//! to track execution state, pause reasons, and stepping modes.

/// Debugger execution state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugState {
    /// Normal execution - running without debugger intervention
    Running,
    /// Stopped, waiting for user command
    Paused(PauseReason),
    /// Single-stepping through code
    Stepping(StepMode),
    /// Program has finished execution
    Finished,
}

impl Default for DebugState {
    fn default() -> Self {
        DebugState::Paused(PauseReason::Initial)
    }
}

/// Reason why the debugger paused execution
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PauseReason {
    /// Initial pause before program starts
    Initial,
    /// Hit a breakpoint (stores breakpoint ID)
    Breakpoint(usize),
    /// Step completed
    Step,
    /// Watch expression triggered (stores watchpoint ID)
    Watchpoint(usize),
    /// User requested interrupt (Ctrl+C)
    UserInterrupt,
    /// Runtime error occurred
    Error,
}

impl std::fmt::Display for PauseReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PauseReason::Initial => write!(f, "initial pause"),
            PauseReason::Breakpoint(id) => write!(f, "breakpoint {}", id),
            PauseReason::Step => write!(f, "step completed"),
            PauseReason::Watchpoint(id) => write!(f, "watchpoint {}", id),
            PauseReason::UserInterrupt => write!(f, "user interrupt"),
            PauseReason::Error => write!(f, "error"),
        }
    }
}

/// Stepping mode for execution control
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepMode {
    /// Step into function calls (execute one statement, entering functions)
    Into,
    /// Step over function calls (execute one statement, skipping over function internals)
    Over,
    /// Step out (run until current function returns)
    Out,
}

impl std::fmt::Display for StepMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StepMode::Into => write!(f, "step into"),
            StepMode::Over => write!(f, "step over"),
            StepMode::Out => write!(f, "step out"),
        }
    }
}

/// Debug action that can be requested by the debugger
#[derive(Debug, Clone, PartialEq)]
pub enum DebugAction {
    /// Continue normal execution
    Continue,
    /// Step with specified mode
    Step(StepMode),
    /// Stop execution
    Stop,
    /// Restart the program
    Restart,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_state_default() {
        let state = DebugState::default();
        assert_eq!(state, DebugState::Paused(PauseReason::Initial));
    }

    #[test]
    fn test_pause_reason_display() {
        assert_eq!(format!("{}", PauseReason::Initial), "initial pause");
        assert_eq!(format!("{}", PauseReason::Breakpoint(1)), "breakpoint 1");
        assert_eq!(format!("{}", PauseReason::Step), "step completed");
    }

    #[test]
    fn test_step_mode_display() {
        assert_eq!(format!("{}", StepMode::Into), "step into");
        assert_eq!(format!("{}", StepMode::Over), "step over");
        assert_eq!(format!("{}", StepMode::Out), "step out");
    }
}
