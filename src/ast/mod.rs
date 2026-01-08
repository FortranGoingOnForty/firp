//! Abstract Syntax Tree definitions for Fortran programs

use crate::lexer::SourceLocation;

/// A compilation unit containing modules and/or a program
#[derive(Debug, Clone, PartialEq)]
pub struct CompilationUnit {
    /// Module definitions (parsed before the main program)
    pub modules: Vec<ModuleDef>,
    /// Main program (optional - file might only contain modules)
    pub program: Option<Program>,
}

/// Root AST node representing a complete Fortran program
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub name: Option<String>,
    /// USE statements
    pub uses: Vec<UseStatement>,
    pub declarations: Vec<Declaration>,
    pub statements: Vec<Statement>,
    /// Contained procedures (after CONTAINS)
    pub procedures: Vec<Procedure>,
    pub location: SourceLocation,
}

/// Module definition
#[derive(Debug, Clone, PartialEq)]
pub struct ModuleDef {
    pub name: String,
    /// Default visibility (PUBLIC or PRIVATE)
    pub default_visibility: Visibility,
    /// USE statements
    pub uses: Vec<UseStatement>,
    /// Module-level declarations
    pub declarations: Vec<Declaration>,
    /// PUBLIC/PRIVATE statements for specific symbols
    pub visibility_stmts: Vec<VisibilityStmt>,
    /// Module procedures (after CONTAINS)
    pub procedures: Vec<Procedure>,
    pub location: SourceLocation,
}

/// Visibility specifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Private,
}

impl Default for Visibility {
    fn default() -> Self {
        Visibility::Public
    }
}

/// Visibility statement: PUBLIC :: name1, name2 or PRIVATE :: name1, name2
#[derive(Debug, Clone, PartialEq)]
pub struct VisibilityStmt {
    pub visibility: Visibility,
    /// Names affected (empty means set default visibility)
    pub names: Vec<String>,
    pub location: SourceLocation,
}

/// USE statement
#[derive(Debug, Clone, PartialEq)]
pub struct UseStatement {
    /// Module name to use
    pub module_name: String,
    /// ONLY clause items (None = import all)
    pub only: Option<Vec<UseItem>>,
    pub location: SourceLocation,
}

/// Item in USE statement (can be renamed)
#[derive(Debug, Clone, PartialEq)]
pub struct UseItem {
    /// Local name (after renaming, or original if not renamed)
    pub local_name: String,
    /// Original name from module (None if not renamed)
    pub original_name: Option<String>,
}

/// Derived type definition (user-defined type/struct)
#[derive(Debug, Clone, PartialEq)]
pub struct DerivedTypeDef {
    /// Type name
    pub name: String,
    /// Parent type name if EXTENDS(parent_type)
    pub extends: Option<String>,
    /// Component declarations
    pub components: Vec<TypeComponent>,
    /// Type-bound procedures (after CONTAINS)
    pub procedures: Vec<TypeBoundProcedure>,
    pub location: SourceLocation,
}

/// A component within a derived type
#[derive(Debug, Clone, PartialEq)]
pub struct TypeComponent {
    /// Component name
    pub name: String,
    /// Type of the component
    pub type_spec: TypeSpec,
    /// Array dimensions (None for scalars)
    pub array_spec: Option<ArraySpec>,
    /// Default initialization value
    pub init: Option<Expr>,
    pub location: SourceLocation,
}

/// Type-bound procedure binding
#[derive(Debug, Clone, PartialEq)]
pub struct TypeBoundProcedure {
    /// Binding name (how it's called on the type)
    pub binding_name: String,
    /// Procedure name it refers to (after =>)
    pub procedure_name: Option<String>,
    /// PASS attribute (which argument gets the object)
    pub pass_arg: Option<String>,
    /// NOPASS if explicitly no pass
    pub nopass: bool,
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
    /// Derived type: TYPE(type_name) or CLASS(type_name)
    Derived { name: String, is_class: bool },
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
    /// Derived type definition
    DerivedType(DerivedTypeDef),
}

/// Assignment target (left-hand side of assignment)
#[derive(Debug, Clone, PartialEq)]
pub enum AssignmentTarget {
    /// Simple variable: x
    Variable(String),
    /// Array element: arr(i) or arr(i, j)
    ArrayElement { name: String, indices: Vec<Expr> },
    /// Component access: obj%component (can be chained)
    Component { object: Box<AssignmentTarget>, component: String },
}

/// Statements
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    /// Assignment: x = expr or arr(i) = expr or obj%comp = expr
    Assignment {
        target: String,
        /// Array indices for array element assignment (None for scalar)
        indices: Option<Vec<Expr>>,
        /// Component path for derived type access (e.g., ["x"] for p%x)
        components: Vec<String>,
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
    /// WRITE statement: WRITE(unit, fmt) values
    Write {
        /// Unit number (None = stdout, Some(expr) = file unit)
        unit: Option<Expr>,
        /// Format specification (None = list-directed)
        format: Option<FormatSpec>,
        /// Values to write
        values: Vec<Expr>,
        location: SourceLocation,
    },
    /// READ statement: READ(unit, fmt) variables
    Read {
        /// Unit number (None = stdin, Some(expr) = file unit)
        unit: Option<Expr>,
        /// Format specification (None = list-directed)
        format: Option<FormatSpec>,
        /// Variables to read into
        variables: Vec<String>,
        location: SourceLocation,
    },
    /// OPEN statement: OPEN(UNIT=n, FILE='name', ...)
    Open {
        /// Unit number
        unit: Expr,
        /// File name
        file: Option<Expr>,
        /// STATUS: OLD, NEW, REPLACE, SCRATCH, UNKNOWN
        status: Option<String>,
        /// ACTION: READ, WRITE, READWRITE
        action: Option<String>,
        /// IOSTAT variable for error code
        iostat: Option<String>,
        location: SourceLocation,
    },
    /// CLOSE statement: CLOSE(UNIT=n)
    Close {
        /// Unit number
        unit: Expr,
        /// IOSTAT variable for error code
        iostat: Option<String>,
        location: SourceLocation,
    },
}

/// Format specification for I/O
#[derive(Debug, Clone, PartialEq)]
pub enum FormatSpec {
    /// List-directed (free format): *
    ListDirected,
    /// Inline format string: '(I5, F10.2)'
    String(String),
    /// Label reference to FORMAT statement
    Label(i64),
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
    /// Component access: obj%component
    ComponentAccess {
        object: Box<Expr>,
        component: String,
        location: SourceLocation,
    },
    /// Type constructor: Point(1.0, 2.0)
    TypeConstructor {
        type_name: String,
        arguments: Vec<Expr>,
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
            Expr::ComponentAccess { location, .. } => location,
            Expr::TypeConstructor { location, .. } => location,
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
