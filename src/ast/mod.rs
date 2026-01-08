//! Abstract Syntax Tree definitions for Fortran programs

use crate::lexer::SourceLocation;

/// Root AST node representing a complete Fortran program
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub name: Option<String>,
    pub declarations: Vec<Declaration>,
    pub statements: Vec<Statement>,
    pub location: SourceLocation,
}

/// Type specification for variables
#[derive(Debug, Clone, PartialEq)]
pub enum TypeSpec {
    Integer { kind: Option<String> },
    Real { kind: Option<String> },
    DoublePrecision,
    Complex { kind: Option<String> },
    Logical { kind: Option<String> },
    Character { len: Option<Box<Expr>>, kind: Option<String> },
}

impl TypeSpec {
    pub fn integer() -> Self {
        TypeSpec::Integer { kind: None }
    }

    pub fn real() -> Self {
        TypeSpec::Real { kind: None }
    }

    pub fn double_precision() -> Self {
        TypeSpec::DoublePrecision
    }

    pub fn complex() -> Self {
        TypeSpec::Complex { kind: None }
    }

    pub fn logical() -> Self {
        TypeSpec::Logical { kind: None }
    }

    pub fn character() -> Self {
        TypeSpec::Character {
            len: None,
            kind: None,
        }
    }
}

/// Declaration statement
#[derive(Debug, Clone, PartialEq)]
pub enum Declaration {
    /// Variable declaration: INTEGER :: x, y
    Variable {
        type_spec: TypeSpec,
        names: Vec<String>,
        init: Option<Vec<Option<Expr>>>,
        location: SourceLocation,
    },
    /// IMPLICIT NONE
    ImplicitNone { location: SourceLocation },
    /// PARAMETER declaration (constants)
    Parameter {
        type_spec: TypeSpec,
        name: String,
        value: Expr,
        location: SourceLocation,
    },
}

/// Statements
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    /// Assignment: x = expr
    Assignment {
        target: String,
        value: Expr,
        location: SourceLocation,
    },
    /// PRINT statement
    Print {
        format: Option<String>,
        values: Vec<Expr>,
        location: SourceLocation,
    },
}

/// Expressions
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Integer literal
    IntegerLiteral(i64, SourceLocation),
    /// Real literal
    RealLiteral(f64, SourceLocation),
    /// String literal
    StringLiteral(String, SourceLocation),
    /// Logical literal
    LogicalLiteral(bool, SourceLocation),
    /// Variable reference
    Identifier(String, SourceLocation),
    /// Binary operation
    BinaryOp {
        op: BinaryOperator,
        left: Box<Expr>,
        right: Box<Expr>,
        location: SourceLocation,
    },
    /// Unary operation
    UnaryOp {
        op: UnaryOperator,
        operand: Box<Expr>,
        location: SourceLocation,
    },
    /// Parenthesized expression
    Parenthesized(Box<Expr>, SourceLocation),
}

impl Expr {
    pub fn location(&self) -> &SourceLocation {
        match self {
            Expr::IntegerLiteral(_, loc) => loc,
            Expr::RealLiteral(_, loc) => loc,
            Expr::StringLiteral(_, loc) => loc,
            Expr::LogicalLiteral(_, loc) => loc,
            Expr::Identifier(_, loc) => loc,
            Expr::BinaryOp { location, .. } => location,
            Expr::UnaryOp { location, .. } => location,
            Expr::Parenthesized(_, loc) => loc,
        }
    }
}

/// Binary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOperator {
    // Arithmetic
    Add,
    Subtract,
    Multiply,
    Divide,
    Power,

    // Relational
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,

    // Logical
    And,
    Or,
    Eqv,
    Neqv,
}

impl BinaryOperator {
    pub fn precedence(&self) -> u8 {
        match self {
            // Logical operators (lowest precedence)
            BinaryOperator::Eqv | BinaryOperator::Neqv => 1,
            BinaryOperator::Or => 2,
            BinaryOperator::And => 3,

            // Relational operators
            BinaryOperator::Equal
            | BinaryOperator::NotEqual
            | BinaryOperator::Less
            | BinaryOperator::LessEqual
            | BinaryOperator::Greater
            | BinaryOperator::GreaterEqual => 4,

            // Arithmetic operators
            BinaryOperator::Add | BinaryOperator::Subtract => 5,
            BinaryOperator::Multiply | BinaryOperator::Divide => 6,
            BinaryOperator::Power => 7, // Highest precedence (right-associative)
        }
    }

    pub fn is_right_associative(&self) -> bool {
        matches!(self, BinaryOperator::Power)
    }
}

/// Unary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOperator {
    Plus,
    Minus,
    Not,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operator_precedence() {
        // Power has highest precedence
        assert!(BinaryOperator::Power.precedence() > BinaryOperator::Multiply.precedence());

        // Multiplication before addition
        assert!(BinaryOperator::Multiply.precedence() > BinaryOperator::Add.precedence());

        // Arithmetic before relational
        assert!(BinaryOperator::Add.precedence() > BinaryOperator::Equal.precedence());

        // Relational before logical
        assert!(BinaryOperator::Equal.precedence() > BinaryOperator::And.precedence());
    }

    #[test]
    fn test_power_is_right_associative() {
        assert!(BinaryOperator::Power.is_right_associative());
        assert!(!BinaryOperator::Multiply.is_right_associative());
    }

    #[test]
    fn test_expr_location() {
        let loc = SourceLocation { line: 1, column: 1 };
        let expr = Expr::IntegerLiteral(42, loc);
        assert_eq!(expr.location(), &loc);
    }
}
