//! Breakpoint management for the interactive debugger
//!
//! This module provides breakpoint types and a manager for setting,
//! removing, and checking breakpoints during program execution.

use crate::lexer::SourceLocation;
use super::eval::{EvalContext, evaluate_condition};

/// A breakpoint definition
#[derive(Debug, Clone)]
pub struct Breakpoint {
    /// Unique breakpoint identifier
    pub id: usize,
    /// Where the breakpoint is set
    pub location: BreakpointLocation,
    /// Optional condition expression (break only if true)
    pub condition: Option<String>,
    /// Whether this breakpoint is enabled
    pub enabled: bool,
    /// Number of times this breakpoint has been hit
    pub hit_count: usize,
    /// Only break after this many hits (None = always break)
    pub target_hits: Option<usize>,
    /// Delete after first hit
    pub temporary: bool,
}

impl Breakpoint {
    /// Create a new line breakpoint
    pub fn line(id: usize, line: usize) -> Self {
        Self {
            id,
            location: BreakpointLocation::Line { file: None, line },
            condition: None,
            enabled: true,
            hit_count: 0,
            target_hits: None,
            temporary: false,
        }
    }

    /// Create a new function breakpoint
    pub fn function(id: usize, name: String) -> Self {
        Self {
            id,
            location: BreakpointLocation::Function { name },
            condition: None,
            enabled: true,
            hit_count: 0,
            target_hits: None,
            temporary: false,
        }
    }

    /// Create a temporary breakpoint (deleted after first hit)
    pub fn temporary_line(id: usize, line: usize) -> Self {
        let mut bp = Self::line(id, line);
        bp.temporary = true;
        bp
    }

    /// Add a condition to this breakpoint
    pub fn with_condition(mut self, condition: String) -> Self {
        self.condition = Some(condition);
        self
    }

    /// Set target hit count (break after N hits)
    pub fn with_hit_count(mut self, count: usize) -> Self {
        self.target_hits = Some(count);
        self
    }

    /// Check if breakpoint matches the given source location
    pub fn matches(&self, loc: &SourceLocation) -> bool {
        if !self.enabled {
            return false;
        }

        match &self.location {
            BreakpointLocation::Line { file: _, line } => {
                // Check line number (file matching not yet supported in SourceLocation)
                loc.line == *line
            }
            BreakpointLocation::Function { .. } => {
                // Function breakpoints are checked separately
                false
            }
        }
    }

    /// Check if breakpoint should trigger based on hit count
    pub fn should_trigger(&self) -> bool {
        if let Some(target) = self.target_hits {
            self.hit_count >= target
        } else {
            true
        }
    }

    /// Format breakpoint for display
    pub fn display(&self) -> String {
        let status = if self.enabled { "" } else { " [disabled]" };
        let condition = self.condition.as_ref()
            .map(|c| format!(" if {}", c))
            .unwrap_or_default();
        let hits = if self.hit_count > 0 {
            format!(" (hit {} times)", self.hit_count)
        } else {
            String::new()
        };
        let temp = if self.temporary { " [temporary]" } else { "" };

        match &self.location {
            BreakpointLocation::Line { file, line } => {
                let file_str = file.as_ref()
                    .map(|f| format!("{}:", f))
                    .unwrap_or_default();
                format!("Breakpoint {} at {}line {}{}{}{}{}",
                    self.id, file_str, line, condition, status, hits, temp)
            }
            BreakpointLocation::Function { name } => {
                format!("Breakpoint {} at function '{}'{}{}{}{}",
                    self.id, name, condition, status, hits, temp)
            }
        }
    }
}

/// Where a breakpoint is located
#[derive(Debug, Clone, PartialEq)]
pub enum BreakpointLocation {
    /// Breakpoint at a specific line
    Line {
        /// Optional file name (None = any file)
        file: Option<String>,
        /// Line number (1-indexed)
        line: usize,
    },
    /// Breakpoint at function entry
    Function {
        /// Function/subroutine name
        name: String,
    },
}

/// Manages breakpoints during debugging
#[derive(Debug, Default)]
pub struct BreakpointManager {
    /// All breakpoints
    breakpoints: Vec<Breakpoint>,
    /// Next breakpoint ID to assign
    next_id: usize,
}

impl BreakpointManager {
    /// Create a new breakpoint manager
    pub fn new() -> Self {
        Self {
            breakpoints: Vec::new(),
            next_id: 1,
        }
    }

    /// Add a line breakpoint
    pub fn add_line(&mut self, line: usize, condition: Option<String>) -> usize {
        let id = self.next_id;
        self.next_id += 1;

        let mut bp = Breakpoint::line(id, line);
        if let Some(cond) = condition {
            bp = bp.with_condition(cond);
        }

        self.breakpoints.push(bp);
        id
    }

    /// Add a line breakpoint in a specific file
    pub fn add_line_in_file(&mut self, file: &str, line: usize, condition: Option<String>) -> usize {
        let id = self.next_id;
        self.next_id += 1;

        let mut bp = Breakpoint {
            id,
            location: BreakpointLocation::Line {
                file: Some(file.to_string()),
                line,
            },
            condition: None,
            enabled: true,
            hit_count: 0,
            target_hits: None,
            temporary: false,
        };

        if let Some(cond) = condition {
            bp = bp.with_condition(cond);
        }

        self.breakpoints.push(bp);
        id
    }

    /// Add a function breakpoint
    pub fn add_function(&mut self, name: &str) -> usize {
        let id = self.next_id;
        self.next_id += 1;

        self.breakpoints.push(Breakpoint::function(id, name.to_string()));
        id
    }

    /// Add a temporary breakpoint (deleted after first hit)
    pub fn add_temporary(&mut self, line: usize) -> usize {
        let id = self.next_id;
        self.next_id += 1;

        self.breakpoints.push(Breakpoint::temporary_line(id, line));
        id
    }

    /// Remove a breakpoint by ID
    pub fn remove(&mut self, id: usize) -> bool {
        if let Some(pos) = self.breakpoints.iter().position(|b| b.id == id) {
            self.breakpoints.remove(pos);
            true
        } else {
            false
        }
    }

    /// Enable a breakpoint by ID
    pub fn enable(&mut self, id: usize) -> bool {
        if let Some(bp) = self.breakpoints.iter_mut().find(|b| b.id == id) {
            bp.enabled = true;
            true
        } else {
            false
        }
    }

    /// Disable a breakpoint by ID
    pub fn disable(&mut self, id: usize) -> bool {
        if let Some(bp) = self.breakpoints.iter_mut().find(|b| b.id == id) {
            bp.enabled = false;
            true
        } else {
            false
        }
    }

    /// Get all breakpoints
    pub fn list(&self) -> &[Breakpoint] {
        &self.breakpoints
    }

    /// Get a breakpoint by ID
    pub fn get(&self, id: usize) -> Option<&Breakpoint> {
        self.breakpoints.iter().find(|b| b.id == id)
    }

    /// Get a mutable breakpoint by ID
    pub fn get_mut(&mut self, id: usize) -> Option<&mut Breakpoint> {
        self.breakpoints.iter_mut().find(|b| b.id == id)
    }

    /// Check if execution should stop at this location
    /// Returns the breakpoint ID if we should stop
    pub fn should_break(&mut self, location: &SourceLocation) -> Option<usize> {
        let mut triggered_id = None;
        let mut to_remove = Vec::new();

        for bp in &mut self.breakpoints {
            if bp.matches(location) {
                bp.hit_count += 1;

                if bp.should_trigger() {
                    triggered_id = Some(bp.id);

                    // Mark temporary breakpoints for removal
                    if bp.temporary {
                        to_remove.push(bp.id);
                    }
                    break;
                }
            }
        }

        // Remove temporary breakpoints that were hit
        for id in to_remove {
            self.remove(id);
        }

        triggered_id
    }

    /// Check if execution should stop at this location with context evaluation
    /// Returns the breakpoint ID if we should stop
    pub fn should_break_with_context(&mut self, location: &SourceLocation, ctx: &EvalContext) -> Option<usize> {
        let mut triggered_id = None;
        let mut to_remove = Vec::new();

        for bp in &mut self.breakpoints {
            if bp.matches(location) {
                bp.hit_count += 1;

                if !bp.should_trigger() {
                    continue;
                }

                // Evaluate condition if present
                if let Some(ref condition) = bp.condition {
                    match evaluate_condition(condition, ctx) {
                        Ok(true) => { /* condition met, proceed to break */ }
                        Ok(false) => continue, // condition not met, skip
                        Err(e) => {
                            eprintln!("Warning: breakpoint {} condition error: {}", bp.id, e);
                            continue;
                        }
                    }
                }

                triggered_id = Some(bp.id);

                // Mark temporary breakpoints for removal
                if bp.temporary {
                    to_remove.push(bp.id);
                }
                break;
            }
        }

        // Remove temporary breakpoints that were hit
        for id in to_remove {
            self.remove(id);
        }

        triggered_id
    }

    /// Check if a function breakpoint should trigger
    pub fn should_break_function(&mut self, name: &str) -> Option<usize> {
        let name_upper = name.to_uppercase();

        for bp in &mut self.breakpoints {
            if let BreakpointLocation::Function { name: bp_name } = &bp.location {
                if bp.enabled && bp_name.to_uppercase() == name_upper {
                    bp.hit_count += 1;
                    if bp.should_trigger() {
                        return Some(bp.id);
                    }
                }
            }
        }
        None
    }

    /// Clear all breakpoints
    pub fn clear(&mut self) {
        self.breakpoints.clear();
    }

    /// Get the number of breakpoints
    pub fn count(&self) -> usize {
        self.breakpoints.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_location(line: usize) -> SourceLocation {
        SourceLocation {
            line,
            column: 1,
        }
    }

    #[test]
    fn test_add_line_breakpoint() {
        let mut manager = BreakpointManager::new();
        let id = manager.add_line(10, None);
        assert_eq!(id, 1);
        assert_eq!(manager.count(), 1);

        let bp = manager.get(id).unwrap();
        assert!(bp.enabled);
        assert_eq!(bp.hit_count, 0);
    }

    #[test]
    fn test_should_break_at_line() {
        let mut manager = BreakpointManager::new();
        manager.add_line(10, None);

        // Should not break at line 5
        assert!(manager.should_break(&make_location(5)).is_none());

        // Should break at line 10
        assert!(manager.should_break(&make_location(10)).is_some());
    }

    #[test]
    fn test_disable_breakpoint() {
        let mut manager = BreakpointManager::new();
        let id = manager.add_line(10, None);

        // Should break when enabled
        assert!(manager.should_break(&make_location(10)).is_some());

        // Disable and check it doesn't break
        manager.disable(id);
        assert!(manager.should_break(&make_location(10)).is_none());

        // Re-enable and check it breaks again
        manager.enable(id);
        assert!(manager.should_break(&make_location(10)).is_some());
    }

    #[test]
    fn test_remove_breakpoint() {
        let mut manager = BreakpointManager::new();
        let id = manager.add_line(10, None);

        assert_eq!(manager.count(), 1);
        assert!(manager.remove(id));
        assert_eq!(manager.count(), 0);
        assert!(!manager.remove(id)); // Already removed
    }

    #[test]
    fn test_temporary_breakpoint() {
        let mut manager = BreakpointManager::new();
        manager.add_temporary(10);

        assert_eq!(manager.count(), 1);

        // First hit removes it
        assert!(manager.should_break(&make_location(10)).is_some());
        assert_eq!(manager.count(), 0);
    }

    #[test]
    fn test_hit_count_breakpoint() {
        let mut manager = BreakpointManager::new();
        let id = manager.add_line(10, None);

        // Set to break after 3 hits
        if let Some(bp) = manager.get_mut(id) {
            bp.target_hits = Some(3);
        }

        // First two hits don't trigger
        assert!(manager.should_break(&make_location(10)).is_none());
        assert!(manager.should_break(&make_location(10)).is_none());

        // Third hit triggers
        assert!(manager.should_break(&make_location(10)).is_some());
    }

    #[test]
    fn test_function_breakpoint() {
        let mut manager = BreakpointManager::new();
        manager.add_function("compute");

        // Case-insensitive matching
        assert!(manager.should_break_function("COMPUTE").is_some());
        assert!(manager.should_break_function("Compute").is_some());
        assert!(manager.should_break_function("other").is_none());
    }

    #[test]
    fn test_breakpoint_display() {
        let bp = Breakpoint::line(1, 42);
        assert!(bp.display().contains("line 42"));

        let bp_func = Breakpoint::function(2, "test_func".to_string());
        assert!(bp_func.display().contains("test_func"));
    }
}
