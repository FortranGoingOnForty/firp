//! Source display for the interactive debugger
//!
//! This module provides utilities for displaying source code
//! and current execution location during debugging.

use crate::lexer::SourceLocation;

/// Number of context lines to show around current line
const DEFAULT_CONTEXT: usize = 5;

/// Source display utilities
pub struct SourceDisplay {
    /// Source code lines (stored for display)
    source_lines: Vec<String>,
    /// File name (if any)
    file_name: Option<String>,
}

impl SourceDisplay {
    /// Create a new source display from source code
    pub fn new(source: &str) -> Self {
        Self {
            source_lines: source.lines().map(|s| s.to_string()).collect(),
            file_name: None,
        }
    }

    /// Create a source display with a file name
    pub fn with_file(source: &str, file_name: &str) -> Self {
        Self {
            source_lines: source.lines().map(|s| s.to_string()).collect(),
            file_name: Some(file_name.to_string()),
        }
    }

    /// Get the number of source lines
    pub fn line_count(&self) -> usize {
        self.source_lines.len()
    }

    /// Get the full source code
    pub fn source(&self) -> String {
        self.source_lines.join("\n")
    }

    /// Get a specific source line (1-indexed)
    pub fn get_line(&self, line: usize) -> Option<&str> {
        if line > 0 && line <= self.source_lines.len() {
            Some(&self.source_lines[line - 1])
        } else {
            None
        }
    }

    /// Format the current location for display
    pub fn format_location(&self, location: &SourceLocation) -> String {
        let file = self.file_name.as_deref().unwrap_or("<input>");
        let line_content = self.get_line(location.line)
            .map(|s| s.trim())
            .unwrap_or("<unknown>");

        format!("Stopped at {}:{}:{}\n    {}",
            file, location.line, location.column, line_content)
    }

    /// Format source lines with line numbers
    ///
    /// # Arguments
    /// * `start` - Start line (1-indexed, inclusive)
    /// * `end` - End line (1-indexed, inclusive)
    /// * `current` - Current line to highlight (optional)
    pub fn format_lines(&self, start: usize, end: usize, current: Option<usize>) -> String {
        let mut result = String::new();
        let max_line = self.source_lines.len();

        // Clamp to valid range
        let start = start.max(1);
        let end = end.min(max_line);

        if start > end {
            return "  <no source available>".to_string();
        }

        // Calculate padding for line numbers
        let width = format!("{}", end).len();

        for line_num in start..=end {
            let line = self.source_lines.get(line_num - 1)
                .map(|s| s.as_str())
                .unwrap_or("");

            // Mark current line with arrow
            let marker = if Some(line_num) == current { "=>" } else { "  " };

            result.push_str(&format!("{} {:>width$} | {}\n",
                marker, line_num, line, width = width));
        }

        result
    }

    /// List source around the current line
    pub fn list_around(&self, line: usize, context: Option<usize>) -> String {
        let ctx = context.unwrap_or(DEFAULT_CONTEXT);
        let start = line.saturating_sub(ctx);
        let end = line.saturating_add(ctx);

        self.format_lines(start, end, Some(line))
    }

    /// List source from start to end
    pub fn list_range(&self, start: usize, end: usize) -> String {
        self.format_lines(start, end, None)
    }

    /// Show where we are (current location)
    pub fn where_am_i(&self, location: &SourceLocation) -> String {
        let mut result = self.format_location(location);
        result.push('\n');
        result.push_str(&self.list_around(location.line, Some(2)));
        result
    }
}

/// Format a call stack entry for display
pub fn format_stack_entry(index: usize, name: &str, location: Option<&SourceLocation>) -> String {
    match location {
        Some(loc) => format!("#{} {} at line {}:{}", index, name, loc.line, loc.column),
        None => format!("#{} {} at <unknown>", index, name),
    }
}

/// Format a backtrace from a list of (name, location) pairs
pub fn format_backtrace(frames: &[(String, Option<SourceLocation>)], full: bool) -> String {
    if frames.is_empty() {
        return "  <no call stack>".to_string();
    }

    let mut result = String::new();
    for (i, (name, loc)) in frames.iter().enumerate() {
        result.push_str(&format_stack_entry(i, name, loc.as_ref()));
        result.push('\n');

        // If full, we could show local variables here
        if full {
            // TODO: Show locals for each frame
            result.push_str("    <locals not yet implemented>\n");
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_SOURCE: &str = r#"PROGRAM test
  INTEGER :: x, y
  x = 10
  y = 20
  PRINT *, x + y
END PROGRAM"#;

    #[test]
    fn test_line_count() {
        let display = SourceDisplay::new(SAMPLE_SOURCE);
        assert_eq!(display.line_count(), 6);
    }

    #[test]
    fn test_get_line() {
        let display = SourceDisplay::new(SAMPLE_SOURCE);
        assert_eq!(display.get_line(1), Some("PROGRAM test"));
        assert_eq!(display.get_line(3), Some("  x = 10"));
        assert_eq!(display.get_line(0), None);
        assert_eq!(display.get_line(100), None);
    }

    #[test]
    fn test_format_location() {
        let display = SourceDisplay::with_file(SAMPLE_SOURCE, "test.f90");
        let loc = SourceLocation { line: 3, column: 3 };
        let formatted = display.format_location(&loc);

        assert!(formatted.contains("test.f90"));
        assert!(formatted.contains("3:3"));
        assert!(formatted.contains("x = 10"));
    }

    #[test]
    fn test_format_lines() {
        let display = SourceDisplay::new(SAMPLE_SOURCE);
        let output = display.format_lines(1, 3, Some(2));

        assert!(output.contains("1 | PROGRAM test"));
        assert!(output.contains("=> 2 | ")); // Current line marker
        assert!(output.contains("3 | "));
    }

    #[test]
    fn test_list_around() {
        let display = SourceDisplay::new(SAMPLE_SOURCE);
        let output = display.list_around(3, Some(1));

        assert!(output.contains("2 | "));
        assert!(output.contains("=> 3 | "));
        assert!(output.contains("4 | "));
    }

    #[test]
    fn test_format_stack_entry() {
        let loc = SourceLocation { line: 10, column: 5 };
        let entry = format_stack_entry(0, "compute", Some(&loc));
        assert!(entry.contains("#0"));
        assert!(entry.contains("compute"));
        assert!(entry.contains("line 10"));
    }

    #[test]
    fn test_format_backtrace() {
        let frames = vec![
            ("inner".to_string(), Some(SourceLocation { line: 20, column: 1 })),
            ("outer".to_string(), Some(SourceLocation { line: 10, column: 1 })),
            ("main".to_string(), None),
        ];

        let bt = format_backtrace(&frames, false);
        assert!(bt.contains("#0 inner"));
        assert!(bt.contains("#1 outer"));
        assert!(bt.contains("#2 main"));
    }

    #[test]
    fn test_empty_backtrace() {
        let frames: Vec<(String, Option<SourceLocation>)> = vec![];
        let bt = format_backtrace(&frames, false);
        assert!(bt.contains("no call stack"));
    }
}
