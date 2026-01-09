//! Unified diagnostic system for collecting and converting errors
//!
//! This module provides:
//! - DiagnosticBag for collecting multiple diagnostics
//! - Conversions from LexerError, ParseError, SemanticError, CompileError, RuntimeError

use crate::lexer::{LexerError, SourceLocation, Span};
use crate::parser::ParseError;
use crate::semantic::SemanticError;
use crate::bytecode::CompileError;
use crate::vm::RuntimeError;

use super::render::{Diagnostic, Severity};
use super::codes::*;

/// A bag for collecting multiple diagnostics
#[derive(Debug, Default)]
pub struct DiagnosticBag {
    diagnostics: Vec<Diagnostic>,
    error_count: usize,
    warning_count: usize,
}

impl DiagnosticBag {
    /// Create a new empty diagnostic bag
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a diagnostic
    pub fn add(&mut self, diagnostic: Diagnostic) {
        match diagnostic.severity {
            Severity::Error => self.error_count += 1,
            Severity::Warning => self.warning_count += 1,
            _ => {}
        }
        self.diagnostics.push(diagnostic);
    }

    /// Add an error diagnostic
    pub fn error(&mut self, message: impl Into<String>) {
        self.add(Diagnostic::error(message));
    }

    /// Add a warning diagnostic
    pub fn warning(&mut self, message: impl Into<String>) {
        self.add(Diagnostic::warning(message));
    }

    /// Check if there are any errors
    pub fn has_errors(&self) -> bool {
        self.error_count > 0
    }

    /// Check if there are any warnings
    pub fn has_warnings(&self) -> bool {
        self.warning_count > 0
    }

    /// Get the error count
    pub fn error_count(&self) -> usize {
        self.error_count
    }

    /// Get the warning count
    pub fn warning_count(&self) -> usize {
        self.warning_count
    }

    /// Get all diagnostics
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Take all diagnostics, consuming the bag
    pub fn into_diagnostics(self) -> Vec<Diagnostic> {
        self.diagnostics
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }

    /// Merge another bag into this one
    pub fn merge(&mut self, other: DiagnosticBag) {
        self.error_count += other.error_count;
        self.warning_count += other.warning_count;
        self.diagnostics.extend(other.diagnostics);
    }

    /// Clear all diagnostics
    pub fn clear(&mut self) {
        self.diagnostics.clear();
        self.error_count = 0;
        self.warning_count = 0;
    }
}

/// Create a span from a SourceLocation (for legacy error types)
fn span_from_location(loc: SourceLocation) -> Span {
    Span::from_location(loc)
}

// ============================================================================
// Conversions from LexerError
// ============================================================================

impl From<LexerError> for Diagnostic {
    fn from(err: LexerError) -> Self {
        match err {
            LexerError::UnexpectedCharacter(ch, loc) => {
                Diagnostic::error(format!("unexpected character '{}'", ch))
                    .with_code(E0001_UNEXPECTED_CHARACTER.code_string())
                    .with_span(span_from_location(loc))
                    .with_label("unexpected character")
            }
            LexerError::InvalidNumber(value, loc) => {
                Diagnostic::error(format!("invalid number '{}'", value))
                    .with_code(E0003_INVALID_NUMBER.code_string())
                    .with_span(span_from_location(loc))
                    .with_label("invalid numeric literal")
                    .with_help("Check the number format. Examples: 42, 3.14, 1.0E10")
            }
            LexerError::UnterminatedString(loc) => {
                Diagnostic::error("unterminated string literal")
                    .with_code(E0002_UNTERMINATED_STRING.code_string())
                    .with_span(span_from_location(loc))
                    .with_label("string starts here")
                    .with_help("Add a closing quote character")
            }
            LexerError::InvalidEscape(ch, loc) => {
                Diagnostic::error(format!("invalid escape sequence '\\{}'", ch))
                    .with_code(E0004_INVALID_ESCAPE.code_string())
                    .with_span(span_from_location(loc))
                    .with_label("invalid escape")
            }
        }
    }
}

impl From<&LexerError> for Diagnostic {
    fn from(err: &LexerError) -> Self {
        err.clone().into()
    }
}

// ============================================================================
// Conversions from ParseError
// ============================================================================

impl From<ParseError> for Diagnostic {
    fn from(err: ParseError) -> Self {
        match err {
            ParseError::UnexpectedToken { expected, found, location } => {
                Diagnostic::error(format!("expected {} but found {}", expected, found))
                    .with_code(E0101_UNEXPECTED_TOKEN.code_string())
                    .with_span(span_from_location(location))
                    .with_label(format!("expected {}", expected))
            }
            ParseError::UnexpectedEof { expected, location } => {
                Diagnostic::error(format!("expected {} but reached end of file", expected))
                    .with_code(E0102_EXPECTED_END_OF_STATEMENT.code_string())
                    .with_span(span_from_location(location))
                    .with_label("unexpected end of file")
                    .with_help(format!("Add {} before the end of the file", expected))
            }
            ParseError::InvalidNumber { value, location } => {
                Diagnostic::error(format!("invalid number '{}'", value))
                    .with_code(E0003_INVALID_NUMBER.code_string())
                    .with_span(span_from_location(location))
                    .with_label("invalid numeric literal")
            }
        }
    }
}

impl From<&ParseError> for Diagnostic {
    fn from(err: &ParseError) -> Self {
        err.clone().into()
    }
}

// ============================================================================
// Conversions from SemanticError
// ============================================================================

impl From<SemanticError> for Diagnostic {
    fn from(err: SemanticError) -> Self {
        match err {
            SemanticError::UndeclaredVariable { name, location, suggestions } => {
                let mut diag = Diagnostic::error(format!("undeclared variable '{}'", name))
                    .with_code(E0201_UNDECLARED_VARIABLE.code_string())
                    .with_span(span_from_location(location))
                    .with_label("not found in this scope");

                if let Some(sugg) = suggestions {
                    if let Some(help_text) = crate::diagnostic::suggest::format_suggestions(&sugg) {
                        diag = diag.with_help(help_text);
                    }
                } else {
                    diag = diag.with_help("Declare the variable before use, or check for typos");
                }
                diag
            }
            SemanticError::DuplicateDeclaration { name, first_location, second_location } => {
                Diagnostic::error(format!("duplicate declaration of '{}'", name))
                    .with_code(E0202_DUPLICATE_DECLARATION.code_string())
                    .with_span(span_from_location(second_location))
                    .with_label("redeclared here")
                    .with_secondary(span_from_location(first_location), "first declared here".to_string())
                    .with_help("Remove the duplicate declaration or rename one of them")
            }
            SemanticError::TypeMismatch { expected, found, location } => {
                Diagnostic::error(format!("type mismatch: expected {}, found {}", expected, found))
                    .with_code(E0203_TYPE_MISMATCH.code_string())
                    .with_span(span_from_location(location))
                    .with_label(format!("expected {}", expected))
            }
            SemanticError::InvalidOperation { operation, left_type, right_type, location } => {
                Diagnostic::error(format!("invalid operation '{}' for {} and {}", operation, left_type, right_type))
                    .with_code(E0216_INVALID_OPERATION.code_string())
                    .with_span(span_from_location(location))
                    .with_label("invalid operation")
            }
            SemanticError::AssignmentToConstant { name, location } => {
                Diagnostic::error(format!("cannot assign to constant '{}'", name))
                    .with_code(E0209_ASSIGN_TO_CONSTANT.code_string())
                    .with_span(span_from_location(location))
                    .with_label("cannot modify")
                    .with_note("This variable was declared with PARAMETER attribute")
            }
            SemanticError::AssignmentToIntentIn { name, location } => {
                Diagnostic::error(format!("cannot assign to INTENT(IN) argument '{}'", name))
                    .with_code(E0210_ASSIGN_TO_INTENT_IN.code_string())
                    .with_span(span_from_location(location))
                    .with_label("cannot modify")
                    .with_note("Arguments with INTENT(IN) are read-only within the procedure")
            }
        }
    }
}

impl From<&SemanticError> for Diagnostic {
    fn from(err: &SemanticError) -> Self {
        err.clone().into()
    }
}

// ============================================================================
// Conversions from SemanticWarning
// ============================================================================

impl From<crate::semantic::SemanticWarning> for Diagnostic {
    fn from(warn: crate::semantic::SemanticWarning) -> Self {
        Diagnostic::warning(warn.message)
            .with_code(warn.code.code_string())
            .with_span(span_from_location(warn.location))
            .with_label("declared here but never used")
            .with_help("If this is intentional, prefix the name with an underscore: _name")
    }
}

impl From<&crate::semantic::SemanticWarning> for Diagnostic {
    fn from(warn: &crate::semantic::SemanticWarning) -> Self {
        warn.clone().into()
    }
}

// ============================================================================
// Conversions from CompileError
// ============================================================================

impl From<CompileError> for Diagnostic {
    fn from(err: CompileError) -> Self {
        match err {
            CompileError::UndeclaredVariable { name, location } => {
                Diagnostic::error(format!("undeclared variable '{}'", name))
                    .with_code(E0201_UNDECLARED_VARIABLE.code_string())
                    .with_span(span_from_location(location))
                    .with_label("not found")
            }
            CompileError::InternalError { message, location } => {
                Diagnostic::error(format!("internal compiler error: {}", message))
                    .with_code(E0305_INVALID_INSTRUCTION.code_string())
                    .with_span(span_from_location(location))
                    .with_note("This is likely a bug in the compiler. Please report it.")
            }
            CompileError::InvalidOperation { message, location } => {
                Diagnostic::error(message)
                    .with_code(E0216_INVALID_OPERATION.code_string())
                    .with_span(span_from_location(location))
                    .with_label("invalid operation")
            }
        }
    }
}

impl From<&CompileError> for Diagnostic {
    fn from(err: &CompileError) -> Self {
        err.clone().into()
    }
}

// ============================================================================
// Conversions from RuntimeError
// ============================================================================

impl From<RuntimeError> for Diagnostic {
    fn from(err: RuntimeError) -> Self {
        match err {
            RuntimeError::StackOverflow { location } => {
                Diagnostic::error("stack overflow")
                    .with_code(E0403_STACK_OVERFLOW.code_string())
                    .with_span(span_from_location(location))
                    .with_label("stack overflow here")
                    .with_help("Check for infinite recursion or reduce stack usage")
            }
            RuntimeError::StackUnderflow { location } => {
                Diagnostic::error("stack underflow")
                    .with_code(E0403_STACK_OVERFLOW.code_string())
                    .with_span(span_from_location(location))
                    .with_note("This is likely a compiler bug")
            }
            RuntimeError::DivisionByZero { location } => {
                Diagnostic::error("division by zero")
                    .with_code(E0401_DIVISION_BY_ZERO.code_string())
                    .with_span(span_from_location(location))
                    .with_label("divisor is zero")
                    .with_help("Check that the divisor is not zero before dividing")
            }
            RuntimeError::TypeError { message, location } => {
                Diagnostic::error(format!("type error: {}", message))
                    .with_code(E0408_RUNTIME_TYPE_ERROR.code_string())
                    .with_span(span_from_location(location))
            }
            RuntimeError::InvalidVariable { index, location } => {
                Diagnostic::error(format!("invalid variable access (index {})", index))
                    .with_code(E0406_INVALID_MEMORY.code_string())
                    .with_span(span_from_location(location))
            }
            RuntimeError::InvalidConstant { index, location } => {
                Diagnostic::error(format!("invalid constant access (index {})", index))
                    .with_code(E0406_INVALID_MEMORY.code_string())
                    .with_span(span_from_location(location))
            }
            RuntimeError::InvalidJump { target, location } => {
                Diagnostic::error(format!("invalid jump to address {}", target))
                    .with_code(E0406_INVALID_MEMORY.code_string())
                    .with_span(span_from_location(location))
            }
            RuntimeError::CallStackOverflow { location } => {
                Diagnostic::error("call stack overflow")
                    .with_code(E0412_RECURSION_LIMIT.code_string())
                    .with_span(span_from_location(location))
                    .with_help("Check for infinite recursion")
            }
            RuntimeError::InvalidProcedure { index, location } => {
                Diagnostic::error(format!("invalid procedure call (index {})", index))
                    .with_code(E0411_UNDEFINED_PROCEDURE_RUNTIME.code_string())
                    .with_span(span_from_location(location))
            }
            RuntimeError::InvalidInstruction { message, location } => {
                Diagnostic::error(format!("invalid instruction: {}", message))
                    .with_code(E0305_INVALID_INSTRUCTION.code_string())
                    .with_span(span_from_location(location))
                    .with_note("This is likely a compiler bug")
            }
            RuntimeError::IndexOutOfBounds { message, location } => {
                Diagnostic::error(format!("array index out of bounds: {}", message))
                    .with_code(E0402_INDEX_OUT_OF_BOUNDS.code_string())
                    .with_span(span_from_location(location))
                    .with_help("Check that array indices are within bounds")
            }
            RuntimeError::IoError { message, location } => {
                Diagnostic::error(format!("I/O error: {}", message))
                    .with_code(E0409_IO_ERROR.code_string())
                    .with_span(span_from_location(location))
            }
            RuntimeError::MathError { message, location } => {
                Diagnostic::error(format!("math error: {}", message))
                    .with_code(E0407_ARITHMETIC_OVERFLOW.code_string())
                    .with_span(span_from_location(location))
            }
        }
    }
}

impl From<&RuntimeError> for Diagnostic {
    fn from(err: &RuntimeError) -> Self {
        err.clone().into()
    }
}

// ============================================================================
// Unified Error Type
// ============================================================================

/// A unified error type that can hold any compiler error
#[derive(Debug, Clone)]
pub enum FirpError {
    Lexer(LexerError),
    Parse(ParseError),
    Semantic(SemanticError),
    Compile(CompileError),
    Runtime(RuntimeError),
}

impl From<LexerError> for FirpError {
    fn from(err: LexerError) -> Self {
        FirpError::Lexer(err)
    }
}

impl From<ParseError> for FirpError {
    fn from(err: ParseError) -> Self {
        FirpError::Parse(err)
    }
}

impl From<SemanticError> for FirpError {
    fn from(err: SemanticError) -> Self {
        FirpError::Semantic(err)
    }
}

impl From<CompileError> for FirpError {
    fn from(err: CompileError) -> Self {
        FirpError::Compile(err)
    }
}

impl From<RuntimeError> for FirpError {
    fn from(err: RuntimeError) -> Self {
        FirpError::Runtime(err)
    }
}

impl From<FirpError> for Diagnostic {
    fn from(err: FirpError) -> Self {
        match err {
            FirpError::Lexer(e) => e.into(),
            FirpError::Parse(e) => e.into(),
            FirpError::Semantic(e) => e.into(),
            FirpError::Compile(e) => e.into(),
            FirpError::Runtime(e) => e.into(),
        }
    }
}

impl std::fmt::Display for FirpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FirpError::Lexer(e) => write!(f, "{}", e),
            FirpError::Parse(e) => write!(f, "{}", e),
            FirpError::Semantic(e) => write!(f, "{}", e),
            FirpError::Compile(e) => write!(f, "{}", e),
            FirpError::Runtime(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for FirpError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostic_bag_counts() {
        let mut bag = DiagnosticBag::new();
        assert_eq!(bag.error_count(), 0);
        assert_eq!(bag.warning_count(), 0);
        assert!(!bag.has_errors());
        assert!(!bag.has_warnings());

        bag.error("test error");
        assert_eq!(bag.error_count(), 1);
        assert!(bag.has_errors());

        bag.warning("test warning");
        assert_eq!(bag.warning_count(), 1);
        assert!(bag.has_warnings());
    }

    #[test]
    fn test_diagnostic_bag_merge() {
        let mut bag1 = DiagnosticBag::new();
        bag1.error("error 1");
        bag1.warning("warning 1");

        let mut bag2 = DiagnosticBag::new();
        bag2.error("error 2");

        bag1.merge(bag2);
        assert_eq!(bag1.error_count(), 2);
        assert_eq!(bag1.warning_count(), 1);
        assert_eq!(bag1.diagnostics().len(), 3);
    }

    #[test]
    fn test_lexer_error_conversion() {
        let err = LexerError::UnexpectedCharacter('@', SourceLocation::new(1, 5));
        let diag: Diagnostic = err.into();

        assert_eq!(diag.severity, Severity::Error);
        assert_eq!(diag.code, Some("E0001".to_string()));
        assert!(diag.message.contains("unexpected character"));
    }

    #[test]
    fn test_parse_error_conversion() {
        use crate::lexer::TokenType;

        let err = ParseError::UnexpectedToken {
            expected: "identifier".to_string(),
            found: TokenType::Integer,
            location: SourceLocation::new(2, 10),
        };
        let diag: Diagnostic = err.into();

        assert_eq!(diag.severity, Severity::Error);
        assert_eq!(diag.code, Some("E0101".to_string()));
    }

    #[test]
    fn test_semantic_error_conversion() {
        let err = SemanticError::UndeclaredVariable {
            name: "xyz".to_string(),
            location: SourceLocation::new(5, 3),
            suggestions: None,
        };
        let diag: Diagnostic = err.into();

        assert_eq!(diag.severity, Severity::Error);
        assert_eq!(diag.code, Some("E0201".to_string()));
        assert!(diag.message.contains("undeclared variable"));
        assert!(diag.message.contains("xyz"));
    }

    #[test]
    fn test_semantic_error_with_suggestions() {
        let err = SemanticError::UndeclaredVariable {
            name: "counte".to_string(),
            location: SourceLocation::new(5, 3),
            suggestions: Some(vec!["counter".to_string()]),
        };
        let diag: Diagnostic = err.into();

        assert_eq!(diag.severity, Severity::Error);
        assert!(diag.help.is_some());
        assert!(diag.help.unwrap().contains("counter"));
    }

    #[test]
    fn test_runtime_error_conversion() {
        let err = RuntimeError::DivisionByZero {
            location: SourceLocation::new(10, 15),
        };
        let diag: Diagnostic = err.into();

        assert_eq!(diag.severity, Severity::Error);
        assert_eq!(diag.code, Some("E0401".to_string()));
        assert!(diag.help.is_some());
    }

    #[test]
    fn test_firp_error_unified() {
        let lexer_err = FirpError::from(LexerError::UnterminatedString(SourceLocation::new(1, 1)));
        let parse_err = FirpError::from(ParseError::InvalidNumber {
            value: "1.2.3".to_string(),
            location: SourceLocation::new(1, 1),
        });

        // Both can be converted to Diagnostic
        let _d1: Diagnostic = lexer_err.into();
        let _d2: Diagnostic = parse_err.into();
    }
}
