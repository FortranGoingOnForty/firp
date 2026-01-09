//! Error and warning codes for diagnostics
//!
//! This module defines standardized error codes for all compiler phases:
//! - E0001-E0099: Lexer errors
//! - E0101-E0199: Parser errors
//! - E0201-E0299: Semantic errors
//! - E0301-E0399: Compile errors (bytecode generation)
//! - E0401-E0499: Runtime errors
//! - W0001-W0099: General warnings
//! - W0101-W0199: Style warnings

use std::fmt;

/// Error code representing a specific type of error or warning
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ErrorCode {
    /// Whether this is a warning (W) or error (E)
    pub is_warning: bool,
    /// Numeric code
    pub number: u16,
}

impl ErrorCode {
    /// Create an error code
    pub const fn error(number: u16) -> Self {
        Self { is_warning: false, number }
    }

    /// Create a warning code
    pub const fn warning(number: u16) -> Self {
        Self { is_warning: true, number }
    }

    /// Get the code string (e.g., "E0201" or "W0001")
    pub fn code_string(&self) -> String {
        let prefix = if self.is_warning { 'W' } else { 'E' };
        format!("{}{:04}", prefix, self.number)
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code_string())
    }
}

// ============================================================================
// Lexer Errors (E0001-E0099)
// ============================================================================

/// Unexpected character in source
pub const E0001_UNEXPECTED_CHARACTER: ErrorCode = ErrorCode::error(1);
/// Unterminated string literal
pub const E0002_UNTERMINATED_STRING: ErrorCode = ErrorCode::error(2);
/// Invalid numeric literal
pub const E0003_INVALID_NUMBER: ErrorCode = ErrorCode::error(3);
/// Invalid escape sequence in string
pub const E0004_INVALID_ESCAPE: ErrorCode = ErrorCode::error(4);
/// Numeric literal overflow
pub const E0005_NUMERIC_OVERFLOW: ErrorCode = ErrorCode::error(5);
/// Invalid character in identifier
pub const E0006_INVALID_IDENTIFIER: ErrorCode = ErrorCode::error(6);
/// Unterminated block comment
pub const E0007_UNTERMINATED_COMMENT: ErrorCode = ErrorCode::error(7);
/// Invalid operator
pub const E0008_INVALID_OPERATOR: ErrorCode = ErrorCode::error(8);

// ============================================================================
// Parser Errors (E0101-E0199)
// ============================================================================

/// Unexpected token
pub const E0101_UNEXPECTED_TOKEN: ErrorCode = ErrorCode::error(101);
/// Expected end of statement
pub const E0102_EXPECTED_END_OF_STATEMENT: ErrorCode = ErrorCode::error(102);
/// Missing closing parenthesis
pub const E0103_MISSING_CLOSE_PAREN: ErrorCode = ErrorCode::error(103);
/// Missing closing bracket
pub const E0104_MISSING_CLOSE_BRACKET: ErrorCode = ErrorCode::error(104);
/// Expected expression
pub const E0105_EXPECTED_EXPRESSION: ErrorCode = ErrorCode::error(105);
/// Expected type specifier
pub const E0106_EXPECTED_TYPE: ErrorCode = ErrorCode::error(106);
/// Expected identifier
pub const E0107_EXPECTED_IDENTIFIER: ErrorCode = ErrorCode::error(107);
/// Invalid statement
pub const E0108_INVALID_STATEMENT: ErrorCode = ErrorCode::error(108);
/// Mismatched END statement
pub const E0109_MISMATCHED_END: ErrorCode = ErrorCode::error(109);
/// Missing THEN after IF condition
pub const E0110_MISSING_THEN: ErrorCode = ErrorCode::error(110);
/// Expected DO loop specification
pub const E0111_EXPECTED_DO_SPEC: ErrorCode = ErrorCode::error(111);
/// Invalid procedure definition
pub const E0112_INVALID_PROCEDURE_DEF: ErrorCode = ErrorCode::error(112);
/// Expected PROGRAM, MODULE, SUBROUTINE, or FUNCTION
pub const E0113_EXPECTED_PROGRAM_UNIT: ErrorCode = ErrorCode::error(113);
/// Invalid array specification
pub const E0114_INVALID_ARRAY_SPEC: ErrorCode = ErrorCode::error(114);
/// Expected comma or closing delimiter
pub const E0115_EXPECTED_COMMA_OR_CLOSE: ErrorCode = ErrorCode::error(115);
/// Invalid intent specifier
pub const E0116_INVALID_INTENT: ErrorCode = ErrorCode::error(116);
/// Invalid attribute
pub const E0117_INVALID_ATTRIBUTE: ErrorCode = ErrorCode::error(117);
/// Duplicate attribute
pub const E0118_DUPLICATE_ATTRIBUTE: ErrorCode = ErrorCode::error(118);
/// Expected equals sign for assignment
pub const E0119_EXPECTED_EQUALS: ErrorCode = ErrorCode::error(119);
/// Invalid format specification
pub const E0120_INVALID_FORMAT: ErrorCode = ErrorCode::error(120);
/// Missing required clause
pub const E0121_MISSING_REQUIRED_CLAUSE: ErrorCode = ErrorCode::error(121);

// ============================================================================
// Semantic Errors (E0201-E0299)
// ============================================================================

/// Undeclared variable
pub const E0201_UNDECLARED_VARIABLE: ErrorCode = ErrorCode::error(201);
/// Variable already declared
pub const E0202_DUPLICATE_DECLARATION: ErrorCode = ErrorCode::error(202);
/// Type mismatch
pub const E0203_TYPE_MISMATCH: ErrorCode = ErrorCode::error(203);
/// Invalid array subscript
pub const E0204_INVALID_SUBSCRIPT: ErrorCode = ErrorCode::error(204);
/// Array rank mismatch
pub const E0205_RANK_MISMATCH: ErrorCode = ErrorCode::error(205);
/// Undefined procedure
pub const E0206_UNDEFINED_PROCEDURE: ErrorCode = ErrorCode::error(206);
/// Wrong number of arguments
pub const E0207_ARGUMENT_COUNT_MISMATCH: ErrorCode = ErrorCode::error(207);
/// Invalid argument type
pub const E0208_INVALID_ARGUMENT_TYPE: ErrorCode = ErrorCode::error(208);
/// Cannot assign to constant
pub const E0209_ASSIGN_TO_CONSTANT: ErrorCode = ErrorCode::error(209);
/// Cannot assign to INTENT(IN) argument
pub const E0210_ASSIGN_TO_INTENT_IN: ErrorCode = ErrorCode::error(210);
/// Undefined type
pub const E0211_UNDEFINED_TYPE: ErrorCode = ErrorCode::error(211);
/// Invalid type conversion
pub const E0212_INVALID_CONVERSION: ErrorCode = ErrorCode::error(212);
/// Missing required argument
pub const E0213_MISSING_REQUIRED_ARG: ErrorCode = ErrorCode::error(213);
/// Duplicate keyword argument
pub const E0214_DUPLICATE_KEYWORD_ARG: ErrorCode = ErrorCode::error(214);
/// Unknown keyword argument
pub const E0215_UNKNOWN_KEYWORD_ARG: ErrorCode = ErrorCode::error(215);
/// Invalid operation for type
pub const E0216_INVALID_OPERATION: ErrorCode = ErrorCode::error(216);
/// Undefined component/member
pub const E0217_UNDEFINED_COMPONENT: ErrorCode = ErrorCode::error(217);
/// Cannot use before declaration
pub const E0218_USE_BEFORE_DECLARATION: ErrorCode = ErrorCode::error(218);
/// Undefined module
pub const E0219_UNDEFINED_MODULE: ErrorCode = ErrorCode::error(219);
/// Symbol not exported from module
pub const E0220_SYMBOL_NOT_EXPORTED: ErrorCode = ErrorCode::error(220);
/// Circular module dependency
pub const E0221_CIRCULAR_DEPENDENCY: ErrorCode = ErrorCode::error(221);
/// Invalid ALLOCATABLE usage
pub const E0222_INVALID_ALLOCATABLE: ErrorCode = ErrorCode::error(222);
/// Invalid POINTER usage
pub const E0223_INVALID_POINTER: ErrorCode = ErrorCode::error(223);
/// Shape mismatch in array operation
pub const E0224_SHAPE_MISMATCH: ErrorCode = ErrorCode::error(224);
/// Invalid DO loop variable
pub const E0225_INVALID_DO_VARIABLE: ErrorCode = ErrorCode::error(225);
/// Branch target not found (GO TO label)
pub const E0226_UNDEFINED_LABEL: ErrorCode = ErrorCode::error(226);
/// Duplicate label
pub const E0227_DUPLICATE_LABEL: ErrorCode = ErrorCode::error(227);
/// EXIT/CYCLE outside of loop
pub const E0228_OUTSIDE_LOOP: ErrorCode = ErrorCode::error(228);
/// RETURN outside of procedure
pub const E0229_RETURN_OUTSIDE_PROCEDURE: ErrorCode = ErrorCode::error(229);
/// Function result not set
pub const E0230_RESULT_NOT_SET: ErrorCode = ErrorCode::error(230);
/// Invalid SELECT CASE type
pub const E0231_INVALID_SELECT_TYPE: ErrorCode = ErrorCode::error(231);
/// Overlapping CASE ranges
pub const E0232_OVERLAPPING_CASE: ErrorCode = ErrorCode::error(232);
/// Invalid PURE procedure violation
pub const E0233_PURE_VIOLATION: ErrorCode = ErrorCode::error(233);
/// Invalid ELEMENTAL procedure usage
pub const E0234_ELEMENTAL_VIOLATION: ErrorCode = ErrorCode::error(234);

// ============================================================================
// Compile Errors (E0301-E0399) - Bytecode generation
// ============================================================================

/// Too many local variables
pub const E0301_TOO_MANY_LOCALS: ErrorCode = ErrorCode::error(301);
/// Too many constants
pub const E0302_TOO_MANY_CONSTANTS: ErrorCode = ErrorCode::error(302);
/// Jump target out of range
pub const E0303_JUMP_OUT_OF_RANGE: ErrorCode = ErrorCode::error(303);
/// Stack overflow during compilation
pub const E0304_COMPILE_STACK_OVERFLOW: ErrorCode = ErrorCode::error(304);
/// Invalid bytecode instruction
pub const E0305_INVALID_INSTRUCTION: ErrorCode = ErrorCode::error(305);
/// Procedure too large
pub const E0306_PROCEDURE_TOO_LARGE: ErrorCode = ErrorCode::error(306);

// ============================================================================
// Runtime Errors (E0401-E0499)
// ============================================================================

/// Division by zero
pub const E0401_DIVISION_BY_ZERO: ErrorCode = ErrorCode::error(401);
/// Array index out of bounds
pub const E0402_INDEX_OUT_OF_BOUNDS: ErrorCode = ErrorCode::error(402);
/// Stack overflow
pub const E0403_STACK_OVERFLOW: ErrorCode = ErrorCode::error(403);
/// Null pointer dereference
pub const E0404_NULL_POINTER: ErrorCode = ErrorCode::error(404);
/// Unallocated array access
pub const E0405_UNALLOCATED_ARRAY: ErrorCode = ErrorCode::error(405);
/// Invalid memory access
pub const E0406_INVALID_MEMORY: ErrorCode = ErrorCode::error(406);
/// Arithmetic overflow
pub const E0407_ARITHMETIC_OVERFLOW: ErrorCode = ErrorCode::error(407);
/// Invalid type at runtime
pub const E0408_RUNTIME_TYPE_ERROR: ErrorCode = ErrorCode::error(408);
/// I/O error
pub const E0409_IO_ERROR: ErrorCode = ErrorCode::error(409);
/// Format error
pub const E0410_FORMAT_ERROR: ErrorCode = ErrorCode::error(410);
/// Undefined procedure at runtime
pub const E0411_UNDEFINED_PROCEDURE_RUNTIME: ErrorCode = ErrorCode::error(411);
/// Recursion limit exceeded
pub const E0412_RECURSION_LIMIT: ErrorCode = ErrorCode::error(412);
/// Invalid argument at runtime
pub const E0413_INVALID_RUNTIME_ARGUMENT: ErrorCode = ErrorCode::error(413);
/// Assertion failure (STOP with code)
pub const E0414_ASSERTION_FAILURE: ErrorCode = ErrorCode::error(414);
/// Allocation failure
pub const E0415_ALLOCATION_FAILURE: ErrorCode = ErrorCode::error(415);
/// Deallocation failure
pub const E0416_DEALLOCATION_FAILURE: ErrorCode = ErrorCode::error(416);

// ============================================================================
// General Warnings (W0001-W0099)
// ============================================================================

/// Unused variable
pub const W0001_UNUSED_VARIABLE: ErrorCode = ErrorCode::warning(1);
/// Unused parameter
pub const W0002_UNUSED_PARAMETER: ErrorCode = ErrorCode::warning(2);
/// Variable shadowing
pub const W0003_SHADOWED_VARIABLE: ErrorCode = ErrorCode::warning(3);
/// Implicit type conversion
pub const W0004_IMPLICIT_CONVERSION: ErrorCode = ErrorCode::warning(4);
/// Unreachable code
pub const W0005_UNREACHABLE_CODE: ErrorCode = ErrorCode::warning(5);
/// Deprecated feature
pub const W0006_DEPRECATED: ErrorCode = ErrorCode::warning(6);
/// Comparison of different types
pub const W0007_MIXED_TYPE_COMPARISON: ErrorCode = ErrorCode::warning(7);
/// Unused import
pub const W0008_UNUSED_IMPORT: ErrorCode = ErrorCode::warning(8);
/// Result of operation discarded
pub const W0009_DISCARDED_RESULT: ErrorCode = ErrorCode::warning(9);
/// Integer division may lose precision
pub const W0010_INTEGER_DIVISION: ErrorCode = ErrorCode::warning(10);

// ============================================================================
// Style Warnings (W0101-W0199)
// ============================================================================

/// Identifier too long
pub const W0101_IDENTIFIER_TOO_LONG: ErrorCode = ErrorCode::warning(101);
/// Mixed case style
pub const W0102_MIXED_CASE: ErrorCode = ErrorCode::warning(102);
/// Deeply nested code
pub const W0103_DEEP_NESTING: ErrorCode = ErrorCode::warning(103);
/// Procedure too long
pub const W0104_PROCEDURE_TOO_LONG: ErrorCode = ErrorCode::warning(104);
/// Missing IMPLICIT NONE
pub const W0105_MISSING_IMPLICIT_NONE: ErrorCode = ErrorCode::warning(105);

/// Get a description for an error code
pub fn describe_error(code: ErrorCode) -> &'static str {
    match (code.is_warning, code.number) {
        // Lexer errors
        (false, 1) => "unexpected character in source code",
        (false, 2) => "unterminated string literal",
        (false, 3) => "invalid numeric literal",
        (false, 4) => "invalid escape sequence",
        (false, 5) => "numeric literal overflow",
        (false, 6) => "invalid character in identifier",
        (false, 7) => "unterminated block comment",
        (false, 8) => "invalid operator",

        // Parser errors
        (false, 101) => "unexpected token",
        (false, 102) => "expected end of statement",
        (false, 103) => "missing closing parenthesis",
        (false, 104) => "missing closing bracket",
        (false, 105) => "expected expression",
        (false, 106) => "expected type specifier",
        (false, 107) => "expected identifier",
        (false, 108) => "invalid statement",
        (false, 109) => "mismatched END statement",
        (false, 110) => "missing THEN after IF condition",
        (false, 111) => "expected DO loop specification",
        (false, 112) => "invalid procedure definition",
        (false, 113) => "expected PROGRAM, MODULE, SUBROUTINE, or FUNCTION",
        (false, 114) => "invalid array specification",
        (false, 115) => "expected comma or closing delimiter",
        (false, 116) => "invalid intent specifier",
        (false, 117) => "invalid attribute",
        (false, 118) => "duplicate attribute",
        (false, 119) => "expected equals sign for assignment",
        (false, 120) => "invalid format specification",
        (false, 121) => "missing required clause",

        // Semantic errors
        (false, 201) => "undeclared variable",
        (false, 202) => "duplicate declaration",
        (false, 203) => "type mismatch",
        (false, 204) => "invalid array subscript",
        (false, 205) => "array rank mismatch",
        (false, 206) => "undefined procedure",
        (false, 207) => "wrong number of arguments",
        (false, 208) => "invalid argument type",
        (false, 209) => "cannot assign to constant",
        (false, 210) => "cannot assign to INTENT(IN) argument",
        (false, 211) => "undefined type",
        (false, 212) => "invalid type conversion",
        (false, 213) => "missing required argument",
        (false, 214) => "duplicate keyword argument",
        (false, 215) => "unknown keyword argument",
        (false, 216) => "invalid operation for type",
        (false, 217) => "undefined component/member",
        (false, 218) => "cannot use before declaration",
        (false, 219) => "undefined module",
        (false, 220) => "symbol not exported from module",
        (false, 221) => "circular module dependency",
        (false, 222) => "invalid ALLOCATABLE usage",
        (false, 223) => "invalid POINTER usage",
        (false, 224) => "shape mismatch in array operation",
        (false, 225) => "invalid DO loop variable",
        (false, 226) => "undefined label (GO TO target)",
        (false, 227) => "duplicate label",
        (false, 228) => "EXIT/CYCLE outside of loop",
        (false, 229) => "RETURN outside of procedure",
        (false, 230) => "function result not set",
        (false, 231) => "invalid SELECT CASE type",
        (false, 232) => "overlapping CASE ranges",
        (false, 233) => "PURE procedure violation",
        (false, 234) => "ELEMENTAL procedure violation",

        // Compile errors
        (false, 301) => "too many local variables",
        (false, 302) => "too many constants",
        (false, 303) => "jump target out of range",
        (false, 304) => "stack overflow during compilation",
        (false, 305) => "invalid bytecode instruction",
        (false, 306) => "procedure too large",

        // Runtime errors
        (false, 401) => "division by zero",
        (false, 402) => "array index out of bounds",
        (false, 403) => "stack overflow",
        (false, 404) => "null pointer dereference",
        (false, 405) => "unallocated array access",
        (false, 406) => "invalid memory access",
        (false, 407) => "arithmetic overflow",
        (false, 408) => "runtime type error",
        (false, 409) => "I/O error",
        (false, 410) => "format error",
        (false, 411) => "undefined procedure at runtime",
        (false, 412) => "recursion limit exceeded",
        (false, 413) => "invalid argument at runtime",
        (false, 414) => "assertion failure",
        (false, 415) => "allocation failure",
        (false, 416) => "deallocation failure",

        // General warnings
        (true, 1) => "unused variable",
        (true, 2) => "unused parameter",
        (true, 3) => "variable shadowing",
        (true, 4) => "implicit type conversion",
        (true, 5) => "unreachable code",
        (true, 6) => "deprecated feature",
        (true, 7) => "comparison of different types",
        (true, 8) => "unused import",
        (true, 9) => "result of operation discarded",
        (true, 10) => "integer division may lose precision",

        // Style warnings
        (true, 101) => "identifier too long",
        (true, 102) => "mixed case style",
        (true, 103) => "deeply nested code",
        (true, 104) => "procedure too long",
        (true, 105) => "missing IMPLICIT NONE",

        _ => "unknown error",
    }
}

/// Get extended help for an error code (for --explain)
pub fn explain_error(code: ErrorCode) -> Option<&'static str> {
    match (code.is_warning, code.number) {
        (false, 201) => Some(
            "An undeclared variable was used. In Fortran, all variables should be declared\n\
             before use. Add a declaration like:\n\n\
             INTEGER :: variable_name\n\
             REAL :: other_variable\n\n\
             Or check if you made a typo in the variable name."
        ),
        (false, 203) => Some(
            "A type mismatch occurred. The expression has a different type than expected.\n\
             For example, assigning a CHARACTER to an INTEGER variable.\n\n\
             Check that both sides of the assignment have compatible types, or use\n\
             explicit type conversion functions like INT(), REAL(), etc."
        ),
        (false, 401) => Some(
            "A division by zero occurred at runtime. Before dividing, check that the\n\
             divisor is not zero:\n\n\
             IF (divisor /= 0) THEN\n\
                 result = numerator / divisor\n\
             ELSE\n\
                 PRINT *, 'Error: Division by zero'\n\
             END IF"
        ),
        (false, 402) => Some(
            "An array index was out of bounds. Fortran arrays are 1-indexed by default.\n\
             An array declared as REAL :: arr(10) has valid indices 1 through 10.\n\n\
             Check your loop bounds and array indices."
        ),
        (true, 1) => Some(
            "A variable was declared but never used. This may indicate dead code or\n\
             a typo elsewhere. Either use the variable or remove the declaration."
        ),
        (true, 3) => Some(
            "A variable in an inner scope has the same name as one in an outer scope.\n\
             This 'shadows' the outer variable, making it inaccessible in the inner scope.\n\
             Consider using a different name to avoid confusion."
        ),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_display() {
        assert_eq!(E0201_UNDECLARED_VARIABLE.to_string(), "E0201");
        assert_eq!(W0001_UNUSED_VARIABLE.to_string(), "W0001");
        assert_eq!(E0001_UNEXPECTED_CHARACTER.to_string(), "E0001");
    }

    #[test]
    fn test_error_code_string() {
        assert_eq!(E0401_DIVISION_BY_ZERO.code_string(), "E0401");
        assert_eq!(W0105_MISSING_IMPLICIT_NONE.code_string(), "W0105");
    }

    #[test]
    fn test_describe_error() {
        assert_eq!(describe_error(E0201_UNDECLARED_VARIABLE), "undeclared variable");
        assert_eq!(describe_error(E0401_DIVISION_BY_ZERO), "division by zero");
        assert_eq!(describe_error(W0001_UNUSED_VARIABLE), "unused variable");
    }

    #[test]
    fn test_explain_error() {
        assert!(explain_error(E0201_UNDECLARED_VARIABLE).is_some());
        assert!(explain_error(E0401_DIVISION_BY_ZERO).is_some());
        assert!(explain_error(W0001_UNUSED_VARIABLE).is_some());
    }
}
