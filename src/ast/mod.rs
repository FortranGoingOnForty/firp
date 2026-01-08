//! Abstract Syntax Tree definitions for Fortran programs

use crate::lexer::SourceLocation;

/// Root AST node representing a complete Fortran program
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub name: Option<String>,
    pub declarations: Vec<Declaration>,
    pub statements: Vec<Statement>,
    /// Contained procedures (after CONTAINS)
    pub procedures: Vec<Procedure>,
    pub location: SourceLocation,
}

/// A procedure (subroutine or function)
#[derive(Debug, Clone, PartialEq)]
pub enum Procedure {
    Subroutine(SubroutineDef),
    Function(FunctionDef),
}

/// Subroutine definition
#[derive(Debug, Clone, PartialEq)]
pub struct SubroutineDef {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub declarations: Vec<Declaration>,
    pub body: Vec<Statement>,
    pub location: SourceLocation,
}

/// Function definition
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDef {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<TypeSpec>,
    /// RESULT variable name (defaults to function name)
    pub result_name: Option<String>,
    pub is_recursive: bool,
    pub declarations: Vec<Declaration>,
    pub body: Vec<Statement>,
    pub location: SourceLocation,
}

/// Parameter declaration for subroutine/function
#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: String,
    pub type_spec: Option<TypeSpec>,
    pub intent: Option<Intent>,
    pub location: SourceLocation,
}

/// INTENT attribute for parameters
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    In,
    Out,
    InOut,
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

/// Array dimension specification
#[derive(Debug, Clone, PartialEq)]
pub struct ArrayDimSpec {
    /// Lower bound (defaults to 1 if not specified)
    pub lower: Option<Expr>,
    /// Upper bound (required)
    pub upper: Expr,
}

impl ArrayDimSpec {
    pub fn new(upper: Expr) -> Self {
        Self { lower: None, upper }
    }

    pub fn with_bounds(lower: Expr, upper: Expr) -> Self {
        Self { lower: Some(lower), upper }
    }
}

/// Array specification for a declared variable
#[derive(Debug, Clone, PartialEq)]
pub struct ArraySpec {
    /// Dimensions (at least one for arrays)
    pub dimensions: Vec<ArrayDimSpec>,
}

/// A single declared entity (name with optional array dimensions)
#[derive(Debug, Clone, PartialEq)]
pub struct DeclaredEntity {
    pub name: String,
    /// Array dimensions (None for scalars)
    pub array_spec: Option<ArraySpec>,
    /// Initialization expression
    pub init: Option<Expr>,
}

impl DeclaredEntity {
    pub fn scalar(name: String) -> Self {
        Self { name, array_spec: None, init: None }
    }

    pub fn array(name: String, dims: Vec<ArrayDimSpec>) -> Self {
        Self {
            name,
            array_spec: Some(ArraySpec { dimensions: dims }),
            init: None,
        }
    }

    pub fn with_init(mut self, init: Expr) -> Self {
        self.init = Some(init);
        self
    }
}

/// Declaration statement
#[derive(Debug, Clone, PartialEq)]
pub enum Declaration {
    /// Variable declaration: INTEGER :: x, y, arr(10)
    Variable {
        type_spec: TypeSpec,
        /// For backward compatibility, simple names list
        names: Vec<String>,
        /// Full entity list with array specs
        entities: Vec<DeclaredEntity>,
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
    /// Assignment: x = expr or arr(i) = expr
    Assignment {
        target: String,
        /// Array indices for array element assignment (None for scalar)
        indices: Option<Vec<Expr>>,
        value: Expr,
        location: SourceLocation,
    },
    /// PRINT statement
    Print {
        format: Option<String>,
        values: Vec<Expr>,
        location: SourceLocation,
    },
    /// IF statement
    If {
        condition: Expr,
        then_block: Vec<Statement>,
        else_if_blocks: Vec<(Expr, Vec<Statement>)>,
        else_block: Option<Vec<Statement>>,
        location: SourceLocation,
    },
    /// DO loop (counted)
    DoLoop {
        variable: String,
        start: Expr,
        end: Expr,
        step: Option<Expr>,
        body: Vec<Statement>,
        location: SourceLocation,
    },
    /// DO WHILE loop
    DoWhile {
        condition: Expr,
        body: Vec<Statement>,
        location: SourceLocation,
    },
    /// Infinite DO loop
    DoInfinite {
        body: Vec<Statement>,
        location: SourceLocation,
    },
    /// SELECT CASE statement
    SelectCase {
        selector: Expr,
        cases: Vec<CaseClause>,
        default: Option<Vec<Statement>>,
        location: SourceLocation,
    },
    /// EXIT statement
    Exit {
        location: SourceLocation,
    },
    /// CYCLE statement
    Cycle {
        location: SourceLocation,
    },
    /// CONTINUE statement
    Continue {
        location: SourceLocation,
    },
    /// CALL statement: CALL subroutine(args)
    Call {
        name: String,
        arguments: Vec<Expr>,
        location: SourceLocation,
    },
    /// RETURN statement
    Return {
        value: Option<Expr>,
        location: SourceLocation,
    },
}

/// Case clause for SELECT CASE
#[derive(Debug, Clone, PartialEq)]
pub struct CaseClause {
    pub selector: CaseSelector,
    pub body: Vec<Statement>,
    pub location: SourceLocation,
}

/// Case selector types
#[derive(Debug, Clone, PartialEq)]
pub enum CaseSelector {
    /// Single value: CASE (5)
    Value(Expr),
    /// Value list: CASE (1, 3, 5)
    Values(Vec<Expr>),
    /// Range: CASE (1:10)
    Range(Expr, Expr),
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
    /// Function call: func(args)
    FunctionCall {
        name: String,
        arguments: Vec<Expr>,
        location: SourceLocation,
    },
    /// Array element access: arr(i) or arr(i, j)
    ArrayAccess {
        name: String,
        indices: Vec<Expr>,
        location: SourceLocation,
    },
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
            Expr::FunctionCall { location, .. } => location,
            Expr::ArrayAccess { location, .. } => location,
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
