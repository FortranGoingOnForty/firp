//! Token types and definitions for the Fortran lexer

use std::fmt;

/// Source location for error reporting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
}

impl fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}, column {}", self.line, self.column)
    }
}

/// Token type enum covering all Fortran tokens
#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    // Keywords - Control Flow
    Program,
    End,
    If,
    Then,
    Else,
    ElseIf,
    Do,
    While,
    Cycle,
    Exit,
    Continue,
    Select,
    Case,
    Default,

    // Keywords - Subprograms
    Subroutine,
    Function,
    Return,
    Call,
    Contains,
    Result,
    Recursive,

    // Keywords - Modules
    Module,
    Use,
    Only,
    Public,
    Private,

    // Keywords - Declarations
    Implicit,
    None,
    Integer,
    Real,
    Double,
    Precision,
    Complex,
    Logical,
    Character,
    Type,
    Class,
    Dimension,
    Parameter,
    Allocatable,
    Allocate,
    Deallocate,
    Pointer,
    Target,
    Intent,
    In,
    Out,
    InOut,
    Save,
    Len,
    Kind,
    Stat,
    Optional,

    // Keywords - I/O
    Print,
    Read,
    Write,
    Open,
    Close,
    Format,
    Stop,
    Unit,
    File,
    Status,
    Action,
    Iostat,
    Iomsg,

    // Keywords - OOP
    Extends,
    Procedure,
    Interface,
    Operator,
    Assignment,

    // Keywords - Parallel
    Concurrent,
    Sync,
    All,
    Images,
    Critical,

    // Keywords - Array operations
    Where,
    Elsewhere,
    Forall,

    // Keywords - Modern constructs
    Associate,
    Block,
    Present,
    Pure,
    Elemental,

    // Literals
    IntegerLiteral(String),
    RealLiteral(String),
    StringLiteral(String),
    True,
    False,

    // Identifiers
    Identifier(String),

    // Operators - Arithmetic
    Plus,         // +
    Minus,        // -
    Star,         // *
    Slash,        // /
    Power,        // **

    // Operators - Relational
    Equal,        // =
    EqualEqual,   // ==
    NotEqual,     // /=
    Less,         // <
    LessEqual,    // <=
    Greater,      // >
    GreaterEqual, // >=

    // Operators - Logical (word-based)
    And,  // .AND.
    Or,   // .OR.
    Not,  // .NOT.
    Eq,   // .EQ.
    Ne,   // .NE.
    Lt,   // .LT.
    Le,   // .LE.
    Gt,   // .GT.
    Ge,   // .GE.
    Eqv,  // .EQV.
    Neqv, // .NEQV.

    // Delimiters
    LeftParen,    // (
    RightParen,   // )
    LeftBracket,  // [
    RightBracket, // ]
    Comma,        // ,
    Colon,        // :
    DoubleColon,  // ::
    Semicolon,    // ;
    Dot,          // .
    Percent,      // %
    Arrow,        // =>

    // Special
    Eof,
}

impl fmt::Display for TokenType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenType::Program => write!(f, "PROGRAM"),
            TokenType::End => write!(f, "END"),
            TokenType::If => write!(f, "IF"),
            TokenType::Then => write!(f, "THEN"),
            TokenType::Else => write!(f, "ELSE"),
            TokenType::ElseIf => write!(f, "ELSEIF"),
            TokenType::Do => write!(f, "DO"),
            TokenType::While => write!(f, "WHILE"),
            TokenType::Cycle => write!(f, "CYCLE"),
            TokenType::Exit => write!(f, "EXIT"),
            TokenType::Continue => write!(f, "CONTINUE"),
            TokenType::Select => write!(f, "SELECT"),
            TokenType::Case => write!(f, "CASE"),
            TokenType::Default => write!(f, "DEFAULT"),
            TokenType::Subroutine => write!(f, "SUBROUTINE"),
            TokenType::Function => write!(f, "FUNCTION"),
            TokenType::Return => write!(f, "RETURN"),
            TokenType::Call => write!(f, "CALL"),
            TokenType::Contains => write!(f, "CONTAINS"),
            TokenType::Result => write!(f, "RESULT"),
            TokenType::Recursive => write!(f, "RECURSIVE"),
            TokenType::Module => write!(f, "MODULE"),
            TokenType::Use => write!(f, "USE"),
            TokenType::Only => write!(f, "ONLY"),
            TokenType::Public => write!(f, "PUBLIC"),
            TokenType::Private => write!(f, "PRIVATE"),
            TokenType::Implicit => write!(f, "IMPLICIT"),
            TokenType::None => write!(f, "NONE"),
            TokenType::Integer => write!(f, "INTEGER"),
            TokenType::Real => write!(f, "REAL"),
            TokenType::Double => write!(f, "DOUBLE"),
            TokenType::Precision => write!(f, "PRECISION"),
            TokenType::Complex => write!(f, "COMPLEX"),
            TokenType::Logical => write!(f, "LOGICAL"),
            TokenType::Character => write!(f, "CHARACTER"),
            TokenType::Type => write!(f, "TYPE"),
            TokenType::Class => write!(f, "CLASS"),
            TokenType::Dimension => write!(f, "DIMENSION"),
            TokenType::Parameter => write!(f, "PARAMETER"),
            TokenType::Allocatable => write!(f, "ALLOCATABLE"),
            TokenType::Allocate => write!(f, "ALLOCATE"),
            TokenType::Deallocate => write!(f, "DEALLOCATE"),
            TokenType::Pointer => write!(f, "POINTER"),
            TokenType::Target => write!(f, "TARGET"),
            TokenType::Intent => write!(f, "INTENT"),
            TokenType::In => write!(f, "IN"),
            TokenType::Out => write!(f, "OUT"),
            TokenType::InOut => write!(f, "INOUT"),
            TokenType::Save => write!(f, "SAVE"),
            TokenType::Len => write!(f, "LEN"),
            TokenType::Kind => write!(f, "KIND"),
            TokenType::Stat => write!(f, "STAT"),
            TokenType::Optional => write!(f, "OPTIONAL"),
            TokenType::Print => write!(f, "PRINT"),
            TokenType::Read => write!(f, "READ"),
            TokenType::Write => write!(f, "WRITE"),
            TokenType::Open => write!(f, "OPEN"),
            TokenType::Close => write!(f, "CLOSE"),
            TokenType::Format => write!(f, "FORMAT"),
            TokenType::Stop => write!(f, "STOP"),
            TokenType::Unit => write!(f, "UNIT"),
            TokenType::File => write!(f, "FILE"),
            TokenType::Status => write!(f, "STATUS"),
            TokenType::Action => write!(f, "ACTION"),
            TokenType::Iostat => write!(f, "IOSTAT"),
            TokenType::Iomsg => write!(f, "IOMSG"),
            TokenType::Extends => write!(f, "EXTENDS"),
            TokenType::Procedure => write!(f, "PROCEDURE"),
            TokenType::Interface => write!(f, "INTERFACE"),
            TokenType::Operator => write!(f, "OPERATOR"),
            TokenType::Assignment => write!(f, "ASSIGNMENT"),
            TokenType::Concurrent => write!(f, "CONCURRENT"),
            TokenType::Sync => write!(f, "SYNC"),
            TokenType::All => write!(f, "ALL"),
            TokenType::Images => write!(f, "IMAGES"),
            TokenType::Critical => write!(f, "CRITICAL"),
            TokenType::Where => write!(f, "WHERE"),
            TokenType::Elsewhere => write!(f, "ELSEWHERE"),
            TokenType::Forall => write!(f, "FORALL"),
            TokenType::IntegerLiteral(s) => write!(f, "Integer({})", s),
            TokenType::RealLiteral(s) => write!(f, "Real({})", s),
            TokenType::StringLiteral(s) => write!(f, "String(\"{}\")", s),
            TokenType::True => write!(f, ".TRUE."),
            TokenType::False => write!(f, ".FALSE."),
            TokenType::Identifier(s) => write!(f, "Identifier({})", s),
            TokenType::Plus => write!(f, "+"),
            TokenType::Minus => write!(f, "-"),
            TokenType::Star => write!(f, "*"),
            TokenType::Slash => write!(f, "/"),
            TokenType::Power => write!(f, "**"),
            TokenType::Equal => write!(f, "="),
            TokenType::EqualEqual => write!(f, "=="),
            TokenType::NotEqual => write!(f, "/="),
            TokenType::Less => write!(f, "<"),
            TokenType::LessEqual => write!(f, "<="),
            TokenType::Greater => write!(f, ">"),
            TokenType::GreaterEqual => write!(f, ">="),
            TokenType::And => write!(f, ".AND."),
            TokenType::Or => write!(f, ".OR."),
            TokenType::Not => write!(f, ".NOT."),
            TokenType::Eq => write!(f, ".EQ."),
            TokenType::Ne => write!(f, ".NE."),
            TokenType::Lt => write!(f, ".LT."),
            TokenType::Le => write!(f, ".LE."),
            TokenType::Gt => write!(f, ".GT."),
            TokenType::Ge => write!(f, ".GE."),
            TokenType::Eqv => write!(f, ".EQV."),
            TokenType::Neqv => write!(f, ".NEQV."),
            TokenType::LeftParen => write!(f, "("),
            TokenType::RightParen => write!(f, ")"),
            TokenType::LeftBracket => write!(f, "["),
            TokenType::RightBracket => write!(f, "]"),
            TokenType::Comma => write!(f, ","),
            TokenType::Colon => write!(f, ":"),
            TokenType::DoubleColon => write!(f, "::"),
            TokenType::Semicolon => write!(f, ";"),
            TokenType::Dot => write!(f, "."),
            TokenType::Percent => write!(f, "%"),
            TokenType::Arrow => write!(f, "=>"),
            TokenType::Associate => write!(f, "ASSOCIATE"),
            TokenType::Block => write!(f, "BLOCK"),
            TokenType::Present => write!(f, "PRESENT"),
            TokenType::Pure => write!(f, "PURE"),
            TokenType::Elemental => write!(f, "ELEMENTAL"),
            TokenType::Eof => write!(f, "EOF"),
        }
    }
}

/// A token with its type and source location
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub token_type: TokenType,
    pub location: SourceLocation,
}

impl Token {
    pub fn new(token_type: TokenType, location: SourceLocation) -> Self {
        Self {
            token_type,
            location,
        }
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at {}", self.token_type, self.location)
    }
}
