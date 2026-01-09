//! Execution tracing
//!
//! This module provides tracing capabilities for debugging, allowing users to
//! trace function calls, variable changes, and all statements.

use crate::bytecode::Value;
use crate::lexer::SourceLocation;
use super::inspector::ValueFormatter;
use std::fs::File;
use std::io::{self, Write, BufWriter};

/// Trace output mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceMode {
    /// Tracing disabled
    Off,
    /// Trace function entry/exit
    Calls,
    /// Trace variable changes
    Variables,
    /// Trace all statements
    All,
}

impl Default for TraceMode {
    fn default() -> Self {
        TraceMode::Off
    }
}

/// Where to send trace output
enum TraceOutput {
    Stdout,
    File(BufWriter<File>),
}

/// Execution tracer
pub struct Tracer {
    /// Current trace mode
    mode: TraceMode,
    /// Output destination
    output: TraceOutput,
    /// Current indentation level (for nested calls)
    indent: usize,
    /// Variables being specifically traced (empty = all)
    traced_vars: Vec<String>,
}

impl Default for Tracer {
    fn default() -> Self {
        Self::new()
    }
}

impl Tracer {
    /// Create a new tracer (disabled by default)
    pub fn new() -> Self {
        Self {
            mode: TraceMode::Off,
            output: TraceOutput::Stdout,
            indent: 0,
            traced_vars: Vec::new(),
        }
    }

    /// Get current trace mode
    pub fn mode(&self) -> TraceMode {
        self.mode
    }

    /// Set trace mode
    pub fn set_mode(&mut self, mode: TraceMode) {
        self.mode = mode;
    }

    /// Set output to a file
    pub fn set_output_file(&mut self, path: &str) -> io::Result<()> {
        let file = File::create(path)?;
        self.output = TraceOutput::File(BufWriter::new(file));
        Ok(())
    }

    /// Set output to stdout
    pub fn set_output_stdout(&mut self) {
        self.output = TraceOutput::Stdout;
    }

    /// Add a variable to trace
    pub fn add_traced_var(&mut self, name: &str) {
        let name_upper = name.to_uppercase();
        if !self.traced_vars.contains(&name_upper) {
            self.traced_vars.push(name_upper);
        }
    }

    /// Remove a variable from trace list
    pub fn remove_traced_var(&mut self, name: &str) -> bool {
        let name_upper = name.to_uppercase();
        if let Some(pos) = self.traced_vars.iter().position(|v| v == &name_upper) {
            self.traced_vars.remove(pos);
            true
        } else {
            false
        }
    }

    /// Clear traced variables list
    pub fn clear_traced_vars(&mut self) {
        self.traced_vars.clear();
    }

    /// Get list of traced variables
    pub fn traced_vars(&self) -> &[String] {
        &self.traced_vars
    }

    /// Check if a specific variable should be traced
    fn should_trace_var(&self, name: &str) -> bool {
        if self.traced_vars.is_empty() {
            true // Trace all if no specific vars set
        } else {
            self.traced_vars.iter().any(|v| v == &name.to_uppercase())
        }
    }

    /// Write output with indentation
    fn write(&mut self, msg: &str) {
        let indent = "  ".repeat(self.indent);
        let line = format!("{}{}\n", indent, msg);

        match &mut self.output {
            TraceOutput::Stdout => {
                print!("{}", line);
            }
            TraceOutput::File(f) => {
                let _ = f.write_all(line.as_bytes());
                let _ = f.flush();
            }
        }
    }

    /// Trace function entry
    pub fn trace_call(&mut self, name: &str, args: &[Value]) {
        if !matches!(self.mode, TraceMode::Calls | TraceMode::All) {
            return;
        }

        let args_str = args.iter()
            .map(|v| ValueFormatter::format(v))
            .collect::<Vec<_>>()
            .join(", ");

        self.write(&format!("-> {}({})", name, args_str));
        self.indent += 1;
    }

    /// Trace function return
    pub fn trace_return(&mut self, name: &str, result: Option<&Value>) {
        if !matches!(self.mode, TraceMode::Calls | TraceMode::All) {
            return;
        }

        if self.indent > 0 {
            self.indent -= 1;
        }

        let result_str = result
            .map(|v| format!(" = {}", ValueFormatter::format(v)))
            .unwrap_or_default();

        self.write(&format!("<- {}{}", name, result_str));
    }

    /// Trace variable change
    pub fn trace_variable(&mut self, name: &str, old: Option<&Value>, new: &Value) {
        if !matches!(self.mode, TraceMode::Variables | TraceMode::All) {
            return;
        }

        if !self.should_trace_var(name) {
            return;
        }

        let old_str = old
            .map(|v| ValueFormatter::format(v))
            .unwrap_or_else(|| "<undefined>".to_string());

        let new_str = ValueFormatter::format(new);

        self.write(&format!("{}: {} -> {}", name, old_str, new_str));
    }

    /// Trace instruction execution
    pub fn trace_instruction(&mut self, _ip: usize, location: &SourceLocation, source_line: &str) {
        if !matches!(self.mode, TraceMode::All) {
            return;
        }

        let trimmed = source_line.trim();
        if !trimmed.is_empty() {
            self.write(&format!("[{}:{}] {}", location.line, location.column, trimmed));
        }
    }

    /// Increase indent (entering a scope)
    pub fn enter_scope(&mut self) {
        self.indent += 1;
    }

    /// Decrease indent (leaving a scope)
    pub fn leave_scope(&mut self) {
        if self.indent > 0 {
            self.indent -= 1;
        }
    }

    /// Flush any buffered output
    pub fn flush(&mut self) {
        if let TraceOutput::File(f) = &mut self.output {
            let _ = f.flush();
        }
    }
}

impl std::fmt::Debug for Tracer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tracer")
            .field("mode", &self.mode)
            .field("indent", &self.indent)
            .field("traced_vars", &self.traced_vars)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracer_default() {
        let tracer = Tracer::new();
        assert_eq!(tracer.mode(), TraceMode::Off);
    }

    #[test]
    fn test_set_mode() {
        let mut tracer = Tracer::new();
        tracer.set_mode(TraceMode::Calls);
        assert_eq!(tracer.mode(), TraceMode::Calls);
    }

    #[test]
    fn test_traced_vars() {
        let mut tracer = Tracer::new();

        tracer.add_traced_var("x");
        tracer.add_traced_var("Y");
        assert_eq!(tracer.traced_vars().len(), 2);

        assert!(tracer.remove_traced_var("x"));
        assert_eq!(tracer.traced_vars().len(), 1);

        tracer.clear_traced_vars();
        assert!(tracer.traced_vars().is_empty());
    }

    #[test]
    fn test_should_trace_var() {
        let mut tracer = Tracer::new();

        // Empty = trace all
        assert!(tracer.should_trace_var("any_var"));

        tracer.add_traced_var("x");
        assert!(tracer.should_trace_var("X")); // Case insensitive
        assert!(tracer.should_trace_var("x"));
        assert!(!tracer.should_trace_var("y"));
    }
}
