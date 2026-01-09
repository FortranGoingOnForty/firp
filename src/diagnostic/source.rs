//! Source file storage for error display
//!
//! The SourceMap stores source files and provides efficient line retrieval
//! for displaying source context in error messages.

use std::collections::HashMap;
use std::sync::Arc;

use crate::lexer::{SourceLocation, Span};

/// A source file with line index for efficient line lookup
#[derive(Debug, Clone)]
pub struct SourceFile {
    /// File name
    pub name: Arc<str>,
    /// Full source content
    pub content: Arc<str>,
    /// Byte offsets where each line starts (0-indexed lines)
    line_starts: Vec<usize>,
}

impl SourceFile {
    /// Create a new source file
    pub fn new(name: &str, content: &str) -> Self {
        let name = Arc::from(name);
        let content_arc = Arc::from(content);

        // Build line start index
        let mut line_starts = vec![0];
        for (i, ch) in content.char_indices() {
            if ch == '\n' {
                line_starts.push(i + 1);
            }
        }

        Self {
            name,
            content: content_arc,
            line_starts,
        }
    }

    /// Get a line by 1-indexed line number
    pub fn get_line(&self, line: usize) -> Option<&str> {
        if line == 0 || line > self.line_starts.len() {
            return None;
        }

        let start = self.line_starts[line - 1];
        let end = self.line_starts
            .get(line)
            .copied()
            .unwrap_or(self.content.len());

        // Trim trailing newline if present
        let line_content = &self.content[start..end];
        Some(line_content.trim_end_matches('\n').trim_end_matches('\r'))
    }

    /// Get the number of lines in the file
    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    /// Get byte offset for a line/column (both 1-indexed)
    pub fn get_offset(&self, line: usize, column: usize) -> Option<usize> {
        if line == 0 || line > self.line_starts.len() {
            return None;
        }

        let line_start = self.line_starts[line - 1];
        Some(line_start + column.saturating_sub(1))
    }

    /// Get line and column from byte offset
    pub fn get_location(&self, offset: usize) -> SourceLocation {
        let line = self.line_starts
            .iter()
            .rposition(|&start| start <= offset)
            .map(|i| i + 1)
            .unwrap_or(1);

        let line_start = self.line_starts.get(line - 1).copied().unwrap_or(0);
        let column = offset - line_start + 1;

        SourceLocation::new(line, column)
    }

    /// Create a span from two locations in this file
    pub fn make_span(&self, start: SourceLocation, end: SourceLocation) -> Span {
        let start_offset = self.get_offset(start.line, start.column).unwrap_or(0);
        let end_offset = self.get_offset(end.line, end.column).unwrap_or(start_offset);

        Span::new(
            Some(self.name.clone()),
            start.line,
            start.column,
            start_offset,
            end.line,
            end.column,
            end_offset,
        )
    }

    /// Create a span from a single location (point span)
    pub fn make_point_span(&self, loc: SourceLocation) -> Span {
        self.make_span(loc, loc)
    }
}

/// A map of source files for error display
#[derive(Debug, Clone, Default)]
pub struct SourceMap {
    files: HashMap<Arc<str>, SourceFile>,
}

impl SourceMap {
    /// Create a new empty source map
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a source file and return its name for reference
    pub fn add_file(&mut self, name: &str, content: &str) -> Arc<str> {
        let file = SourceFile::new(name, content);
        let name = file.name.clone();
        self.files.insert(name.clone(), file);
        name
    }

    /// Get a source file by name
    pub fn get_file(&self, name: &str) -> Option<&SourceFile> {
        self.files.get(name)
    }

    /// Get a line from a file
    pub fn get_line(&self, file: &str, line: usize) -> Option<&str> {
        self.get_file(file)?.get_line(line)
    }

    /// Get a snippet of source code for a span
    pub fn get_snippet(&self, span: &Span) -> Option<String> {
        let file_name = span.file()?;
        let file = self.get_file(file_name)?;

        if span.start_line == span.end_line {
            // Single line span
            let line = file.get_line(span.start_line)?;
            Some(line.to_string())
        } else {
            // Multi-line span
            let mut lines = Vec::new();
            for line_num in span.start_line..=span.end_line {
                if let Some(line) = file.get_line(line_num) {
                    lines.push(line.to_string());
                }
            }
            Some(lines.join("\n"))
        }
    }

    /// Create a span from a SourceLocation, looking up file info if available
    pub fn make_span(&self, loc: SourceLocation, file_name: Option<&str>) -> Span {
        if let Some(name) = file_name {
            if let Some(file) = self.get_file(name) {
                return file.make_point_span(loc);
            }
        }
        Span::from_location(loc)
    }

    /// Check if any files have been added
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Get the number of files
    pub fn file_count(&self) -> usize {
        self.files.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_file_get_line() {
        let source = "line one\nline two\nline three";
        let file = SourceFile::new("test.f90", source);

        assert_eq!(file.get_line(1), Some("line one"));
        assert_eq!(file.get_line(2), Some("line two"));
        assert_eq!(file.get_line(3), Some("line three"));
        assert_eq!(file.get_line(0), None);
        assert_eq!(file.get_line(4), None);
    }

    #[test]
    fn test_source_file_line_count() {
        let source = "a\nb\nc";
        let file = SourceFile::new("test.f90", source);
        assert_eq!(file.line_count(), 3);
    }

    #[test]
    fn test_source_file_get_offset() {
        let source = "abc\ndefg\nhi";
        let file = SourceFile::new("test.f90", source);

        // Line 1, column 1 = offset 0
        assert_eq!(file.get_offset(1, 1), Some(0));
        // Line 1, column 3 = offset 2
        assert_eq!(file.get_offset(1, 3), Some(2));
        // Line 2, column 1 = offset 4 (after "abc\n")
        assert_eq!(file.get_offset(2, 1), Some(4));
        // Line 2, column 4 = offset 7
        assert_eq!(file.get_offset(2, 4), Some(7));
    }

    #[test]
    fn test_source_file_get_location() {
        let source = "abc\ndefg\nhi";
        let file = SourceFile::new("test.f90", source);

        assert_eq!(file.get_location(0), SourceLocation::new(1, 1));
        assert_eq!(file.get_location(2), SourceLocation::new(1, 3));
        assert_eq!(file.get_location(4), SourceLocation::new(2, 1));
        assert_eq!(file.get_location(7), SourceLocation::new(2, 4));
    }

    #[test]
    fn test_source_map_add_and_get() {
        let mut map = SourceMap::new();
        let name = map.add_file("test.f90", "line one\nline two");

        assert_eq!(name.as_ref(), "test.f90");
        assert!(map.get_file("test.f90").is_some());
        assert_eq!(map.get_line("test.f90", 1), Some("line one"));
        assert_eq!(map.get_line("test.f90", 2), Some("line two"));
    }

    #[test]
    fn test_source_map_snippet() {
        let mut map = SourceMap::new();
        let name = map.add_file("test.f90", "line one\nline two\nline three");

        let span = Span::new(
            Some(name),
            1, 1, 0,
            1, 8, 8,
        );

        assert_eq!(map.get_snippet(&span), Some("line one".to_string()));
    }

    #[test]
    fn test_make_span() {
        let mut map = SourceMap::new();
        map.add_file("test.f90", "line one\nline two");

        let loc = SourceLocation::new(2, 5);
        let span = map.make_span(loc, Some("test.f90"));

        assert_eq!(span.file(), Some("test.f90"));
        assert_eq!(span.start_line, 2);
        assert_eq!(span.start_column, 5);
    }
}
