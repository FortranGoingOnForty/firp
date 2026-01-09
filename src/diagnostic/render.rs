//! Diagnostic rendering for rich error display
//!
//! Renders error messages with source context, colors, and helpful annotations.

use std::io::{self, Write};
use std::sync::Arc;

use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

use crate::lexer::Span;
use super::source::SourceMap;

/// Severity level of a diagnostic
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Note,
    Help,
}

impl Severity {
    /// Get the color for this severity
    pub fn color(&self) -> Color {
        match self {
            Severity::Error => Color::Red,
            Severity::Warning => Color::Yellow,
            Severity::Note => Color::Blue,
            Severity::Help => Color::Green,
        }
    }

    /// Get the label for this severity
    pub fn label(&self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Note => "note",
            Severity::Help => "help",
        }
    }
}

/// A diagnostic message with source context
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// Severity level
    pub severity: Severity,
    /// Error code (e.g., E0201)
    pub code: Option<String>,
    /// Main error message
    pub message: String,
    /// Primary source span (where the error occurred)
    pub primary_span: Option<Span>,
    /// Primary span label (shown under the ^^^)
    pub primary_label: Option<String>,
    /// Secondary spans with labels (related locations)
    pub secondary_spans: Vec<(Span, String)>,
    /// Additional notes
    pub notes: Vec<String>,
    /// Help suggestions
    pub help: Option<String>,
}

impl Diagnostic {
    /// Create a new error diagnostic
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            code: None,
            message: message.into(),
            primary_span: None,
            primary_label: None,
            secondary_spans: Vec::new(),
            notes: Vec::new(),
            help: None,
        }
    }

    /// Create a new warning diagnostic
    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            code: None,
            message: message.into(),
            primary_span: None,
            primary_label: None,
            secondary_spans: Vec::new(),
            notes: Vec::new(),
            help: None,
        }
    }

    /// Set the error code
    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    /// Set the primary span
    pub fn with_span(mut self, span: Span) -> Self {
        self.primary_span = Some(span);
        self
    }

    /// Set the primary span label
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.primary_label = Some(label.into());
        self
    }

    /// Add a secondary span with label
    pub fn with_secondary(mut self, span: Span, label: impl Into<String>) -> Self {
        self.secondary_spans.push((span, label.into()));
        self
    }

    /// Add a note
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    /// Set the help message
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }
}

/// Output style for diagnostics
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderStyle {
    /// Rich output with source context and carets
    Rich,
    /// Compact single-line output
    Compact,
    /// JSON output for IDE integration
    Json,
}

/// Renders diagnostics to terminal or other output
pub struct DiagnosticRenderer {
    source_map: Arc<SourceMap>,
    color_enabled: bool,
    style: RenderStyle,
}

impl DiagnosticRenderer {
    /// Create a new renderer with a source map
    pub fn new(source_map: Arc<SourceMap>) -> Self {
        Self {
            source_map,
            color_enabled: true,
            style: RenderStyle::Rich,
        }
    }

    /// Enable or disable color output
    pub fn with_color(mut self, enabled: bool) -> Self {
        self.color_enabled = enabled;
        self
    }

    /// Set the output style
    pub fn with_style(mut self, style: RenderStyle) -> Self {
        self.style = style;
        self
    }

    /// Render a diagnostic to a string
    pub fn render(&self, diagnostic: &Diagnostic) -> String {
        match self.style {
            RenderStyle::Rich => self.render_rich(diagnostic),
            RenderStyle::Compact => self.render_compact(diagnostic),
            RenderStyle::Json => self.render_json(diagnostic),
        }
    }

    /// Render a diagnostic to stderr with colors
    pub fn emit(&self, diagnostic: &Diagnostic) -> io::Result<()> {
        let choice = if self.color_enabled {
            ColorChoice::Auto
        } else {
            ColorChoice::Never
        };
        let mut stderr = StandardStream::stderr(choice);

        match self.style {
            RenderStyle::Rich => self.emit_rich(&mut stderr, diagnostic),
            RenderStyle::Compact => self.emit_compact(&mut stderr, diagnostic),
            RenderStyle::Json => {
                writeln!(stderr, "{}", self.render_json(diagnostic))
            }
        }
    }

    /// Render in rich format (with source context)
    fn render_rich(&self, diagnostic: &Diagnostic) -> String {
        let mut output = String::new();

        // Header: error[E0001]: message
        output.push_str(&format!(
            "{}",
            diagnostic.severity.label()
        ));
        if let Some(code) = &diagnostic.code {
            output.push_str(&format!("[{}]", code));
        }
        output.push_str(&format!(": {}\n", diagnostic.message));

        // Location: --> file:line:col
        if let Some(span) = &diagnostic.primary_span {
            let location = if let Some(file) = span.file() {
                format!("{}:{}:{}", file, span.start_line, span.start_column)
            } else {
                format!("{}:{}", span.start_line, span.start_column)
            };
            output.push_str(&format!("  --> {}\n", location));

            // Source context
            if let Some(line) = self.get_source_line(span) {
                let line_num = span.start_line;
                let gutter_width = line_num.to_string().len().max(3);

                // Empty line with pipe
                output.push_str(&format!("{:>width$} |\n", "", width = gutter_width));

                // Source line
                output.push_str(&format!(
                    "{:>width$} | {}\n",
                    line_num,
                    line,
                    width = gutter_width
                ));

                // Caret line
                let caret_start = span.start_column.saturating_sub(1);
                let caret_len = if span.end_line == span.start_line {
                    (span.end_column - span.start_column).max(1)
                } else {
                    line.len().saturating_sub(caret_start).max(1)
                };

                let padding = " ".repeat(caret_start);
                let carets = "^".repeat(caret_len);

                let label = diagnostic.primary_label.as_deref().unwrap_or("");
                output.push_str(&format!(
                    "{:>width$} | {}{} {}\n",
                    "",
                    padding,
                    carets,
                    label,
                    width = gutter_width
                ));
            }
        }

        // Notes
        for note in &diagnostic.notes {
            output.push_str(&format!("   = note: {}\n", note));
        }

        // Help
        if let Some(help) = &diagnostic.help {
            output.push_str(&format!("   = help: {}\n", help));
        }

        output
    }

    /// Emit rich format with colors
    fn emit_rich(&self, w: &mut StandardStream, diagnostic: &Diagnostic) -> io::Result<()> {
        // Header: error[E0001]: message
        w.set_color(ColorSpec::new().set_fg(Some(diagnostic.severity.color())).set_bold(true))?;
        write!(w, "{}", diagnostic.severity.label())?;
        if let Some(code) = &diagnostic.code {
            write!(w, "[{}]", code)?;
        }
        w.reset()?;
        w.set_color(ColorSpec::new().set_bold(true))?;
        writeln!(w, ": {}", diagnostic.message)?;
        w.reset()?;

        // Location: --> file:line:col
        if let Some(span) = &diagnostic.primary_span {
            w.set_color(ColorSpec::new().set_fg(Some(Color::Blue)).set_bold(true))?;
            write!(w, "  --> ")?;
            w.reset()?;

            let location = if let Some(file) = span.file() {
                format!("{}:{}:{}", file, span.start_line, span.start_column)
            } else {
                format!("{}:{}", span.start_line, span.start_column)
            };
            writeln!(w, "{}", location)?;

            // Source context
            if let Some(line) = self.get_source_line(span) {
                let line_num = span.start_line;
                let gutter_width = line_num.to_string().len().max(3);

                // Empty line with pipe
                w.set_color(ColorSpec::new().set_fg(Some(Color::Blue)).set_bold(true))?;
                writeln!(w, "{:>width$} |", "", width = gutter_width)?;

                // Source line
                write!(w, "{:>width$} | ", line_num, width = gutter_width)?;
                w.reset()?;
                writeln!(w, "{}", line)?;

                // Caret line
                w.set_color(ColorSpec::new().set_fg(Some(Color::Blue)).set_bold(true))?;
                write!(w, "{:>width$} | ", "", width = gutter_width)?;

                let caret_start = span.start_column.saturating_sub(1);
                let caret_len = if span.end_line == span.start_line {
                    (span.end_column - span.start_column).max(1)
                } else {
                    line.len().saturating_sub(caret_start).max(1)
                };

                let padding = " ".repeat(caret_start);
                w.set_color(ColorSpec::new().set_fg(Some(diagnostic.severity.color())).set_bold(true))?;
                write!(w, "{}{}", padding, "^".repeat(caret_len))?;

                if let Some(label) = &diagnostic.primary_label {
                    write!(w, " {}", label)?;
                }
                writeln!(w)?;
                w.reset()?;
            }
        }

        // Notes
        for note in &diagnostic.notes {
            w.set_color(ColorSpec::new().set_fg(Some(Color::Blue)).set_bold(true))?;
            write!(w, "   = ")?;
            w.reset()?;
            w.set_color(ColorSpec::new().set_bold(true))?;
            write!(w, "note")?;
            w.reset()?;
            writeln!(w, ": {}", note)?;
        }

        // Help
        if let Some(help) = &diagnostic.help {
            w.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))?;
            write!(w, "   = ")?;
            w.reset()?;
            w.set_color(ColorSpec::new().set_bold(true))?;
            write!(w, "help")?;
            w.reset()?;
            writeln!(w, ": {}", help)?;
        }

        Ok(())
    }

    /// Render in compact format (single line)
    fn render_compact(&self, diagnostic: &Diagnostic) -> String {
        let mut output = String::new();

        // file:line:col: severity[code]: message
        if let Some(span) = &diagnostic.primary_span {
            if let Some(file) = span.file() {
                output.push_str(&format!("{}:", file));
            }
            output.push_str(&format!("{}:{}: ", span.start_line, span.start_column));
        }

        output.push_str(diagnostic.severity.label());
        if let Some(code) = &diagnostic.code {
            output.push_str(&format!("[{}]", code));
        }
        output.push_str(&format!(": {}", diagnostic.message));

        output
    }

    /// Emit compact format with colors
    fn emit_compact(&self, w: &mut StandardStream, diagnostic: &Diagnostic) -> io::Result<()> {
        // file:line:col:
        if let Some(span) = &diagnostic.primary_span {
            if let Some(file) = span.file() {
                write!(w, "{}:", file)?;
            }
            write!(w, "{}:{}: ", span.start_line, span.start_column)?;
        }

        // severity[code]:
        w.set_color(ColorSpec::new().set_fg(Some(diagnostic.severity.color())).set_bold(true))?;
        write!(w, "{}", diagnostic.severity.label())?;
        if let Some(code) = &diagnostic.code {
            write!(w, "[{}]", code)?;
        }
        w.reset()?;

        // message
        writeln!(w, ": {}", diagnostic.message)?;

        Ok(())
    }

    /// Render in JSON format
    fn render_json(&self, diagnostic: &Diagnostic) -> String {
        use serde_json::json;

        let value = json!({
            "severity": diagnostic.severity.label(),
            "code": diagnostic.code,
            "message": diagnostic.message,
            "span": diagnostic.primary_span.as_ref().map(|s| json!({
                "file": s.file(),
                "start_line": s.start_line,
                "start_column": s.start_column,
                "end_line": s.end_line,
                "end_column": s.end_column,
            })),
            "label": diagnostic.primary_label,
            "notes": diagnostic.notes,
            "help": diagnostic.help,
        });

        serde_json::to_string_pretty(&value).unwrap_or_else(|_| "{}".to_string())
    }

    /// Get the source line for a span
    fn get_source_line(&self, span: &Span) -> Option<String> {
        if let Some(file_name) = span.file() {
            if let Some(line) = self.source_map.get_line(file_name, span.start_line) {
                return Some(line.to_string());
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostic_builder() {
        let diag = Diagnostic::error("test error")
            .with_code("E0001")
            .with_help("try this instead");

        assert_eq!(diag.severity, Severity::Error);
        assert_eq!(diag.code, Some("E0001".to_string()));
        assert_eq!(diag.message, "test error");
        assert_eq!(diag.help, Some("try this instead".to_string()));
    }

    #[test]
    fn test_render_compact() {
        let mut source_map = SourceMap::new();
        let file = source_map.add_file("test.f90", "x = 1\n");

        let span = Span::new(Some(file), 1, 1, 0, 1, 2, 1);
        let diag = Diagnostic::error("undeclared variable")
            .with_code("E0201")
            .with_span(span);

        let renderer = DiagnosticRenderer::new(Arc::new(source_map))
            .with_color(false)
            .with_style(RenderStyle::Compact);

        let output = renderer.render(&diag);
        assert!(output.contains("test.f90:1:1"));
        assert!(output.contains("error[E0201]"));
        assert!(output.contains("undeclared variable"));
    }

    #[test]
    fn test_render_rich() {
        let mut source_map = SourceMap::new();
        let file = source_map.add_file("test.f90", "  x = y + 1\n");

        let span = Span::new(Some(file), 1, 7, 6, 1, 8, 7);
        let diag = Diagnostic::error("undeclared variable 'y'")
            .with_code("E0201")
            .with_span(span)
            .with_label("not found in this scope")
            .with_help("did you mean 'x'?");

        let renderer = DiagnosticRenderer::new(Arc::new(source_map))
            .with_color(false)
            .with_style(RenderStyle::Rich);

        let output = renderer.render(&diag);
        assert!(output.contains("error[E0201]: undeclared variable 'y'"));
        assert!(output.contains("--> test.f90:1:7"));
        assert!(output.contains("x = y + 1"));
        assert!(output.contains("^"));
        assert!(output.contains("not found in this scope"));
        assert!(output.contains("help: did you mean 'x'?"));
    }

    #[test]
    fn test_render_json() {
        let source_map = SourceMap::new();

        let diag = Diagnostic::error("test error")
            .with_code("E0001");

        let renderer = DiagnosticRenderer::new(Arc::new(source_map))
            .with_style(RenderStyle::Json);

        let output = renderer.render(&diag);
        assert!(output.contains("\"severity\": \"error\""));
        assert!(output.contains("\"code\": \"E0001\""));
        assert!(output.contains("\"message\": \"test error\""));
    }
}
