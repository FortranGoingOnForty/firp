//! Semantic analysis and type checking for Fortran programs

use crate::ast::*;
use crate::lexer::SourceLocation;
use std::collections::HashMap;
use std::fmt;

/// Array dimension bounds
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ArrayBound {
    /// Lower bound (defaults to 1 in Fortran)
    pub lower: i64,
    /// Upper bound
    pub upper: i64,
}

impl ArrayBound {
    pub fn new(lower: i64, upper: i64) -> Self {
        Self { lower, upper }
    }

    pub fn size(&self) -> usize {
        (self.upper - self.lower + 1).max(0) as usize
    }
}

/// Fortran types with kind specifications
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    /// INTEGER with optional kind
    Integer { kind: Option<i32> },
    /// REAL with optional kind
    Real { kind: Option<i32> },
    /// DOUBLE PRECISION (equivalent to REAL(KIND=8))
    DoublePrecision,
    /// COMPLEX with optional kind
    Complex { kind: Option<i32> },
    /// LOGICAL with optional kind
    Logical { kind: Option<i32> },
    /// CHARACTER with length and optional kind
    Character { len: Option<usize>, kind: Option<i32> },
    /// Array type
    Array {
        /// Element type (simplified - just the base type kind)
        element_kind: ArrayElementKind,
        /// Dimensions with bounds
        dimensions: Vec<ArrayBound>,
    },
    /// Void type (for statements that don't produce values)
    Void,
    /// Derived type (user-defined type)
    Derived { name: String },
}

/// Simplified element kind for arrays (to maintain Hash/Eq)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArrayElementKind {
    Integer,
    Real,
    DoublePrecision,
    Complex,
    Logical,
    Character,
}

impl Type {
    /// Create default INTEGER type
    pub fn integer() -> Self {
        Type::Integer { kind: None }
    }

    /// Create default REAL type
    pub fn real() -> Self {
        Type::Real { kind: None }
    }

    /// Create DOUBLE PRECISION type
    pub fn double_precision() -> Self {
        Type::DoublePrecision
    }

    /// Create default COMPLEX type
    pub fn complex() -> Self {
        Type::Complex { kind: None }
    }

    /// Create default LOGICAL type
    pub fn logical() -> Self {
        Type::Logical { kind: None }
    }

    /// Create default CHARACTER type
    pub fn character() -> Self {
        Type::Character { len: None, kind: None }
    }

    /// Check if this is a numeric type (integer, real, double precision, complex)
    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            Type::Integer { .. }
                | Type::Real { .. }
                | Type::DoublePrecision
                | Type::Complex { .. }
        )
    }

    /// Check if this is an integer type
    pub fn is_integer(&self) -> bool {
        matches!(self, Type::Integer { .. })
    }

    /// Check if this is a real type (real or double precision)
    pub fn is_real(&self) -> bool {
        matches!(self, Type::Real { .. } | Type::DoublePrecision)
    }

    /// Check if this is a logical type
    pub fn is_logical(&self) -> bool {
        matches!(self, Type::Logical { .. })
    }

    /// Check if this is an array type
    pub fn is_array(&self) -> bool {
        matches!(self, Type::Array { .. })
    }

    /// Get array element type (returns self if not an array)
    pub fn element_type(&self) -> Type {
        match self {
            Type::Array { element_kind, .. } => match element_kind {
                ArrayElementKind::Integer => Type::integer(),
                ArrayElementKind::Real => Type::real(),
                ArrayElementKind::DoublePrecision => Type::double_precision(),
                ArrayElementKind::Complex => Type::complex(),
                ArrayElementKind::Logical => Type::logical(),
                ArrayElementKind::Character => Type::character(),
            },
            _ => self.clone(),
        }
    }

    /// Get array rank (number of dimensions), 0 for scalars
    pub fn rank(&self) -> usize {
        match self {
            Type::Array { dimensions, .. } => dimensions.len(),
            _ => 0,
        }
    }

    /// Create an array type from element type and dimensions
    pub fn array(element: &Type, dimensions: Vec<ArrayBound>) -> Self {
        let element_kind = match element {
            Type::Integer { .. } => ArrayElementKind::Integer,
            Type::Real { .. } => ArrayElementKind::Real,
            Type::DoublePrecision => ArrayElementKind::DoublePrecision,
            Type::Complex { .. } => ArrayElementKind::Complex,
            Type::Logical { .. } => ArrayElementKind::Logical,
            Type::Character { .. } => ArrayElementKind::Character,
            _ => ArrayElementKind::Integer, // Default fallback
        };
        Type::Array { element_kind, dimensions }
    }

    /// Check if two types are compatible for assignment
    /// Returns true if source can be assigned to target (with implicit conversion if needed)
    pub fn is_assignable_from(&self, other: &Type) -> bool {
        // Exact match
        if self == other {
            return true;
        }

        match (self, other) {
            // Numeric types can be assigned to each other (with implicit conversion)
            (Type::Integer { .. }, Type::Real { .. })
            | (Type::Integer { .. }, Type::DoublePrecision)
            | (Type::Real { .. }, Type::Integer { .. })
            | (Type::Real { .. }, Type::DoublePrecision)
            | (Type::DoublePrecision, Type::Integer { .. })
            | (Type::DoublePrecision, Type::Real { .. }) => true,

            // Complex can be assigned from any numeric type
            (Type::Complex { .. }, other) if other.is_numeric() => true,

            _ => false,
        }
    }

    /// Determine the result type of a binary arithmetic operation
    /// Follows Fortran's type promotion rules
    pub fn arithmetic_result_type(left: &Type, right: &Type) -> Option<Type> {
        match (left, right) {
            // Complex propagates to result
            (Type::Complex { kind }, _) | (_, Type::Complex { kind }) => {
                Some(Type::Complex { kind: *kind })
            }

            // Double precision propagates
            (Type::DoublePrecision, _) | (_, Type::DoublePrecision) => {
                Some(Type::DoublePrecision)
            }

            // Real + anything numeric (except complex/double) -> Real
            (Type::Real { kind }, Type::Integer { .. })
            | (Type::Integer { .. }, Type::Real { kind })
            | (Type::Real { kind }, Type::Real { .. }) => Some(Type::Real { kind: *kind }),

            // Integer + Integer -> Integer
            (Type::Integer { kind }, Type::Integer { .. }) => Some(Type::Integer { kind: *kind }),

            _ => None,
        }
    }

    /// Determine the result type of a relational operation
    /// Relational operations always return LOGICAL
    pub fn relational_result_type(left: &Type, right: &Type) -> Option<Type> {
        // Both operands must be numeric
        if left.is_numeric() && right.is_numeric() {
            Some(Type::logical())
        } else if left.is_logical() && right.is_logical() {
            // Logical can be compared with == and /=
            Some(Type::logical())
        } else if matches!(left, Type::Character { .. })
            && matches!(right, Type::Character { .. })
        {
            // Strings can be compared
            Some(Type::logical())
        } else {
            None
        }
    }

    /// Determine the result type of a logical operation
    /// Logical operations require logical operands and return logical
    pub fn logical_result_type(left: &Type, right: &Type) -> Option<Type> {
        if left.is_logical() && right.is_logical() {
            Some(Type::logical())
        } else {
            None
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Integer { kind: Some(k) } => write!(f, "INTEGER(KIND={})", k),
            Type::Integer { kind: None } => write!(f, "INTEGER"),
            Type::Real { kind: Some(k) } => write!(f, "REAL(KIND={})", k),
            Type::Real { kind: None } => write!(f, "REAL"),
            Type::DoublePrecision => write!(f, "DOUBLE PRECISION"),
            Type::Complex { kind: Some(k) } => write!(f, "COMPLEX(KIND={})", k),
            Type::Complex { kind: None } => write!(f, "COMPLEX"),
            Type::Logical { kind: Some(k) } => write!(f, "LOGICAL(KIND={})", k),
            Type::Logical { kind: None } => write!(f, "LOGICAL"),
            Type::Character { len: Some(l), kind: None } => write!(f, "CHARACTER(LEN={})", l),
            Type::Character { len: None, kind: None } => write!(f, "CHARACTER"),
            Type::Character { len, kind: Some(k) } => {
                write!(f, "CHARACTER(LEN={:?},KIND={})", len, k)
            }
            Type::Void => write!(f, "VOID"),
            Type::Array { element_kind, dimensions } => {
                let elem_str = match element_kind {
                    ArrayElementKind::Integer => "INTEGER",
                    ArrayElementKind::Real => "REAL",
                    ArrayElementKind::DoublePrecision => "DOUBLE PRECISION",
                    ArrayElementKind::Complex => "COMPLEX",
                    ArrayElementKind::Logical => "LOGICAL",
                    ArrayElementKind::Character => "CHARACTER",
                };
                let dims: Vec<String> = dimensions.iter()
                    .map(|d| format!("{}:{}", d.lower, d.upper))
                    .collect();
                write!(f, "{}, DIMENSION({})", elem_str, dims.join(", "))
            }
            Type::Derived { name } => write!(f, "TYPE({})", name),
        }
    }
}

/// Convert TypeSpec from AST to semantic Type
impl From<&TypeSpec> for Type {
    fn from(spec: &TypeSpec) -> Self {
        match spec {
            TypeSpec::Integer { kind } => Type::Integer {
                kind: kind.as_ref().and_then(|k| k.parse().ok()),
            },
            TypeSpec::Real { kind } => Type::Real {
                kind: kind.as_ref().and_then(|k| k.parse().ok()),
            },
            TypeSpec::DoublePrecision => Type::DoublePrecision,
            TypeSpec::Complex { kind } => Type::Complex {
                kind: kind.as_ref().and_then(|k| k.parse().ok()),
            },
            TypeSpec::Logical { kind } => Type::Logical {
                kind: kind.as_ref().and_then(|k| k.parse().ok()),
            },
            TypeSpec::Character { len, kind } => {
                let length = len.as_ref().and_then(|expr| {
                    // Try to evaluate constant length expression
                    // For now, just handle integer literals
                    if let Expr::IntegerLiteral(val, _) = expr.as_ref() {
                        Some(*val as usize)
                    } else {
                        None
                    }
                });
                Type::Character {
                    len: length,
                    kind: kind.as_ref().and_then(|k| k.parse().ok()),
                }
            }
            TypeSpec::Derived { name, .. } => Type::Derived { name: name.clone() },
        }
    }
}

/// Symbol representing a variable or constant
#[derive(Debug, Clone, PartialEq)]
pub struct Symbol {
    pub name: String,
    pub ty: Type,
    pub is_parameter: bool,  // PARAMETER (constant)
    pub location: SourceLocation,
}

impl Symbol {
    pub fn new(name: String, ty: Type, location: SourceLocation) -> Self {
        Self {
            name,
            ty,
            is_parameter: false,
            location,
        }
    }

    pub fn with_parameter(mut self) -> Self {
        self.is_parameter = true;
        self
    }
}

/// Symbol table with scope management
#[derive(Debug, Clone)]
pub struct SymbolTable {
    /// Scopes stack (innermost scope is last)
    scopes: Vec<HashMap<String, Symbol>>,
    /// Whether IMPLICIT NONE is in effect
    implicit_none: bool,
}

impl SymbolTable {
    /// Create a new symbol table with global scope
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            implicit_none: false,
        }
    }

    /// Enter a new scope
    pub fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    /// Exit the current scope
    pub fn exit_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    /// Set IMPLICIT NONE flag
    pub fn set_implicit_none(&mut self) {
        self.implicit_none = true;
    }

    /// Check if IMPLICIT NONE is in effect
    pub fn is_implicit_none(&self) -> bool {
        self.implicit_none
    }

    /// Define a symbol in the current scope
    /// Returns error if symbol already exists in current scope
    pub fn define(&mut self, symbol: Symbol) -> Result<(), SemanticError> {
        let scope = self.scopes.last_mut().unwrap();
        let name_upper = symbol.name.to_uppercase();

        if scope.contains_key(&name_upper) {
            let existing = &scope[&name_upper];
            return Err(SemanticError::DuplicateDeclaration {
                name: symbol.name.clone(),
                first_location: existing.location,
                second_location: symbol.location,
            });
        }

        scope.insert(name_upper, symbol);
        Ok(())
    }

    /// Lookup a symbol in the scope chain
    /// Returns None if symbol not found
    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        let name_upper = name.to_uppercase();
        // Search from innermost to outermost scope
        for scope in self.scopes.iter().rev() {
            if let Some(symbol) = scope.get(&name_upper) {
                return Some(symbol);
            }
        }
        None
    }

    /// Get implicit type for a variable based on first letter
    /// Only used when IMPLICIT NONE is not in effect
    pub fn get_implicit_type(&self, name: &str) -> Option<Type> {
        if self.implicit_none {
            return None;
        }

        let first_char = name.chars().next()?.to_uppercase().next()?;
        match first_char {
            // I-N are implicit INTEGER
            'I' | 'J' | 'K' | 'L' | 'M' | 'N' => Some(Type::integer()),
            // Others are implicit REAL
            'A'..='H' | 'O'..='Z' => Some(Type::real()),
            _ => None,
        }
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

/// Semantic error types
#[derive(Debug, Clone, PartialEq)]
pub enum SemanticError {
    /// Variable used without declaration (when IMPLICIT NONE is in effect)
    UndeclaredVariable {
        name: String,
        location: SourceLocation,
    },
    /// Variable declared multiple times in same scope
    DuplicateDeclaration {
        name: String,
        first_location: SourceLocation,
        second_location: SourceLocation,
    },
    /// Type mismatch in expression or assignment
    TypeMismatch {
        expected: Type,
        found: Type,
        location: SourceLocation,
    },
    /// Invalid operation for given types
    InvalidOperation {
        operation: String,
        left_type: Type,
        right_type: Type,
        location: SourceLocation,
    },
    /// Assignment to constant (PARAMETER)
    AssignmentToConstant {
        name: String,
        location: SourceLocation,
    },
}

impl fmt::Display for SemanticError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SemanticError::UndeclaredVariable { name, location } => {
                write!(
                    f,
                    "Undeclared variable '{}' at {}",
                    name, location
                )
            }
            SemanticError::DuplicateDeclaration {
                name,
                first_location,
                second_location,
            } => {
                write!(
                    f,
                    "Duplicate declaration of '{}': first declared at {}, redeclared at {}",
                    name, first_location, second_location
                )
            }
            SemanticError::TypeMismatch {
                expected,
                found,
                location,
            } => {
                write!(
                    f,
                    "Type mismatch at {}: expected {}, found {}",
                    location, expected, found
                )
            }
            SemanticError::InvalidOperation {
                operation,
                left_type,
                right_type,
                location,
            } => {
                write!(
                    f,
                    "Invalid operation '{}' at {}: cannot apply to {} and {}",
                    operation, location, left_type, right_type
                )
            }
            SemanticError::AssignmentToConstant { name, location } => {
                write!(
                    f,
                    "Cannot assign to constant '{}' at {}",
                    name, location
                )
            }
        }
    }
}

impl std::error::Error for SemanticError {}

pub type SemanticResult<T> = Result<T, SemanticError>;

/// Semantic analyzer that performs type checking and validation
pub struct SemanticAnalyzer {
    symbol_table: SymbolTable,
    errors: Vec<SemanticError>,
    in_loop: bool,  // Track if we're inside a loop (for EXIT/CYCLE validation)
}

impl SemanticAnalyzer {
    /// Create a new semantic analyzer
    pub fn new() -> Self {
        Self {
            symbol_table: SymbolTable::new(),
            errors: Vec::new(),
            in_loop: false,
        }
    }

    /// Analyze a complete program
    /// Returns errors collected during analysis (or empty vec if successful)
    pub fn analyze(&mut self, program: &Program) -> Vec<SemanticError> {
        self.errors.clear();

        // Process declarations
        for decl in &program.declarations {
            if let Err(e) = self.analyze_declaration(decl) {
                self.errors.push(e);
            }
        }

        // Process statements
        for stmt in &program.statements {
            if let Err(e) = self.analyze_statement(stmt) {
                self.errors.push(e);
            }
        }

        self.errors.clone()
    }

    /// Analyze a declaration and add symbols to table
    fn analyze_declaration(&mut self, decl: &Declaration) -> SemanticResult<()> {
        match decl {
            Declaration::ImplicitNone { .. } => {
                self.symbol_table.set_implicit_none();
                Ok(())
            }
            Declaration::Variable {
                type_spec,
                entities,
                location,
                ..
            } => {
                let base_ty = Type::from(type_spec);

                // Define each variable/array
                for entity in entities {
                    // If it's an array, create an array type
                    let ty = if let Some(array_spec) = &entity.array_spec {
                        // For now, we'll create an Array type with the dimensions
                        // The actual bounds are expressions that need evaluation
                        let dims: Vec<ArrayBound> = array_spec.dimensions.iter().map(|dim| {
                            // For compile-time constants, we can evaluate them
                            // For now, assume constant bounds
                            let lower = dim.lower.as_ref()
                                .and_then(|e| self.eval_constant_int(e))
                                .unwrap_or(1);
                            let upper = self.eval_constant_int(&dim.upper).unwrap_or(10);
                            ArrayBound::new(lower, upper)
                        }).collect();

                        let element_kind = match &base_ty {
                            Type::Integer { .. } => ArrayElementKind::Integer,
                            Type::Real { .. } => ArrayElementKind::Real,
                            Type::DoublePrecision => ArrayElementKind::DoublePrecision,
                            Type::Complex { .. } => ArrayElementKind::Complex,
                            Type::Logical { .. } => ArrayElementKind::Logical,
                            Type::Character { .. } => ArrayElementKind::Character,
                            _ => ArrayElementKind::Integer, // Default fallback
                        };

                        Type::Array {
                            element_kind,
                            dimensions: dims,
                        }
                    } else {
                        base_ty.clone()
                    };

                    let symbol = Symbol::new(entity.name.clone(), ty.clone(), *location);
                    self.symbol_table.define(symbol)?;

                    // If there's initialization, type check it
                    if let Some(init_expr) = &entity.init {
                        let expr_type = self.check_expression(init_expr)?;
                        // For scalar assignment compatibility check
                        let target_ty = if ty.is_array() {
                            ty.element_type()
                        } else {
                            ty.clone()
                        };
                        if !target_ty.is_assignable_from(&expr_type) {
                            return Err(SemanticError::TypeMismatch {
                                expected: target_ty,
                                found: expr_type,
                                location: *init_expr.location(),
                            });
                        }
                    }
                }
                Ok(())
            }
            Declaration::Parameter {
                type_spec,
                name,
                value,
                location,
            } => {
                let ty = Type::from(type_spec);
                let value_type = self.check_expression(value)?;

                if !ty.is_assignable_from(&value_type) {
                    return Err(SemanticError::TypeMismatch {
                        expected: ty.clone(),
                        found: value_type,
                        location: *value.location(),
                    });
                }

                let symbol = Symbol::new(name.clone(), ty, *location).with_parameter();
                self.symbol_table.define(symbol)?;
                Ok(())
            }
            Declaration::DerivedType(_type_def) => {
                // TODO: Register derived type in symbol table
                // For now, we accept it without additional checks
                Ok(())
            }
            Declaration::Interface(_interface) => {
                // TODO: Register interface in symbol table for operator overloading
                // For now, we accept it without additional checks
                Ok(())
            }
        }
    }

    /// Analyze a statement
    fn analyze_statement(&mut self, stmt: &Statement) -> SemanticResult<()> {
        match stmt {
            Statement::Assignment {
                target,
                indices,
                value,
                location,
                ..
            } => {
                // Look up the target variable
                let symbol = self.lookup_variable(target, *location)?;

                // Check if trying to assign to a constant
                if symbol.is_parameter {
                    return Err(SemanticError::AssignmentToConstant {
                        name: target.clone(),
                        location: *location,
                    });
                }

                // Determine the target type (for arrays, this is the element type)
                let target_type = if let Some(idx_exprs) = indices {
                    // Array element assignment
                    // Check that target is an array
                    if !symbol.ty.is_array() {
                        return Err(SemanticError::TypeMismatch {
                            expected: Type::Array {
                                element_kind: ArrayElementKind::Integer,
                                dimensions: vec![],
                            },
                            found: symbol.ty.clone(),
                            location: *location,
                        });
                    }

                    // Check index types (must be integer)
                    for idx in idx_exprs {
                        let idx_type = self.check_expression(idx)?;
                        if !idx_type.is_numeric() {
                            return Err(SemanticError::TypeMismatch {
                                expected: Type::integer(),
                                found: idx_type,
                                location: *idx.location(),
                            });
                        }
                    }

                    // Return element type
                    symbol.ty.element_type()
                } else {
                    symbol.ty.clone()
                };

                // Type check the value
                let value_type = self.check_expression(value)?;

                // Check assignment compatibility
                if !target_type.is_assignable_from(&value_type) {
                    return Err(SemanticError::TypeMismatch {
                        expected: target_type,
                        found: value_type,
                        location: *value.location(),
                    });
                }

                Ok(())
            }

            Statement::Print { values, .. } => {
                // Type check all print expressions
                for expr in values {
                    self.check_expression(expr)?;
                }
                Ok(())
            }

            Statement::If {
                condition,
                then_block,
                else_if_blocks,
                else_block,
                ..
            } => {
                // Condition must be logical
                let cond_type = self.check_expression(condition)?;
                if !cond_type.is_logical() {
                    return Err(SemanticError::TypeMismatch {
                        expected: Type::logical(),
                        found: cond_type,
                        location: *condition.location(),
                    });
                }

                // Check THEN block
                for stmt in then_block {
                    self.analyze_statement(stmt)?;
                }

                // Check ELSE IF blocks
                for (elif_cond, elif_block) in else_if_blocks {
                    let elif_type = self.check_expression(elif_cond)?;
                    if !elif_type.is_logical() {
                        return Err(SemanticError::TypeMismatch {
                            expected: Type::logical(),
                            found: elif_type,
                            location: *elif_cond.location(),
                        });
                    }
                    for stmt in elif_block {
                        self.analyze_statement(stmt)?;
                    }
                }

                // Check ELSE block
                if let Some(else_stmts) = else_block {
                    for stmt in else_stmts {
                        self.analyze_statement(stmt)?;
                    }
                }

                Ok(())
            }

            Statement::DoLoop {
                variable,
                start,
                end,
                step,
                body,
                location,
            } => {
                // Loop variable must be integer
                let var_symbol = self.lookup_variable(variable, *location)?;
                if !var_symbol.ty.is_integer() {
                    return Err(SemanticError::TypeMismatch {
                        expected: Type::integer(),
                        found: var_symbol.ty.clone(),
                        location: *location,
                    });
                }

                // Start, end, and step must be integer
                let start_type = self.check_expression(start)?;
                if !start_type.is_integer() {
                    return Err(SemanticError::TypeMismatch {
                        expected: Type::integer(),
                        found: start_type,
                        location: *start.location(),
                    });
                }

                let end_type = self.check_expression(end)?;
                if !end_type.is_integer() {
                    return Err(SemanticError::TypeMismatch {
                        expected: Type::integer(),
                        found: end_type,
                        location: *end.location(),
                    });
                }

                if let Some(step_expr) = step {
                    let step_type = self.check_expression(step_expr)?;
                    if !step_type.is_integer() {
                        return Err(SemanticError::TypeMismatch {
                            expected: Type::integer(),
                            found: step_type,
                            location: *step_expr.location(),
                        });
                    }
                }

                // Check body (inside a loop)
                let was_in_loop = self.in_loop;
                self.in_loop = true;
                for stmt in body {
                    self.analyze_statement(stmt)?;
                }
                self.in_loop = was_in_loop;

                Ok(())
            }

            Statement::DoWhile {
                condition,
                body,
                ..
            } => {
                // Condition must be logical
                let cond_type = self.check_expression(condition)?;
                if !cond_type.is_logical() {
                    return Err(SemanticError::TypeMismatch {
                        expected: Type::logical(),
                        found: cond_type,
                        location: *condition.location(),
                    });
                }

                // Check body (inside a loop)
                let was_in_loop = self.in_loop;
                self.in_loop = true;
                for stmt in body {
                    self.analyze_statement(stmt)?;
                }
                self.in_loop = was_in_loop;

                Ok(())
            }

            Statement::DoInfinite { body, .. } => {
                // Check body (inside a loop)
                let was_in_loop = self.in_loop;
                self.in_loop = true;
                for stmt in body {
                    self.analyze_statement(stmt)?;
                }
                self.in_loop = was_in_loop;

                Ok(())
            }

            Statement::SelectCase {
                selector,
                cases,
                default,
                ..
            } => {
                let selector_type = self.check_expression(selector)?;

                // Check each case
                for case in cases {
                    // Type check case selector values
                    match &case.selector {
                        CaseSelector::Value(expr) => {
                            let case_type = self.check_expression(expr)?;
                            // Type must be compatible with selector
                            if case_type != selector_type {
                                return Err(SemanticError::TypeMismatch {
                                    expected: selector_type.clone(),
                                    found: case_type,
                                    location: *expr.location(),
                                });
                            }
                        }
                        CaseSelector::Values(exprs) => {
                            for expr in exprs {
                                let case_type = self.check_expression(expr)?;
                                if case_type != selector_type {
                                    return Err(SemanticError::TypeMismatch {
                                        expected: selector_type.clone(),
                                        found: case_type,
                                        location: *expr.location(),
                                    });
                                }
                            }
                        }
                        CaseSelector::Range(start, end) => {
                            let start_type = self.check_expression(start)?;
                            let end_type = self.check_expression(end)?;
                            if start_type != selector_type {
                                return Err(SemanticError::TypeMismatch {
                                    expected: selector_type.clone(),
                                    found: start_type,
                                    location: *start.location(),
                                });
                            }
                            if end_type != selector_type {
                                return Err(SemanticError::TypeMismatch {
                                    expected: selector_type.clone(),
                                    found: end_type,
                                    location: *end.location(),
                                });
                            }
                        }
                    }

                    // Check case body
                    for stmt in &case.body {
                        self.analyze_statement(stmt)?;
                    }
                }

                // Check default case
                if let Some(default_stmts) = default {
                    for stmt in default_stmts {
                        self.analyze_statement(stmt)?;
                    }
                }

                Ok(())
            }

            Statement::Exit { .. } | Statement::Cycle { .. } => {
                // EXIT and CYCLE are valid (loop context checking could be added)
                Ok(())
            }

            Statement::Continue { .. } => {
                // CONTINUE is always valid (no-op)
                Ok(())
            }

            Statement::Call { arguments, .. } => {
                // TODO: Full implementation in Sprint 08
                // For now, just type check the arguments
                for arg in arguments {
                    self.check_expression(arg)?;
                }
                Ok(())
            }

            Statement::Return { value, .. } => {
                // TODO: Full implementation in Sprint 08
                if let Some(val) = value {
                    self.check_expression(val)?;
                }
                Ok(())
            }

            Statement::Write { values, unit, .. } => {
                // Type check unit expression if present
                if let Some(u) = unit {
                    self.check_expression(u)?;
                }
                // Type check all output expressions
                for expr in values {
                    self.check_expression(expr)?;
                }
                Ok(())
            }

            Statement::Read { unit, .. } => {
                // Type check unit expression if present
                if let Some(u) = unit {
                    self.check_expression(u)?;
                }
                // TODO: Check that variables exist and are not constants
                Ok(())
            }

            Statement::Open { unit, file, .. } => {
                // Type check unit expression
                self.check_expression(unit)?;
                // Type check file expression if present
                if let Some(f) = file {
                    self.check_expression(f)?;
                }
                Ok(())
            }

            Statement::Close { unit, .. } => {
                // Type check unit expression
                self.check_expression(unit)?;
                Ok(())
            }
        }
    }

    /// Check the type of an expression
    fn check_expression(&mut self, expr: &Expr) -> SemanticResult<Type> {
        match expr {
            Expr::IntegerLiteral(_, _) => Ok(Type::integer()),
            Expr::RealLiteral(_, _) => Ok(Type::real()),
            Expr::StringLiteral(_, _) => Ok(Type::character()),
            Expr::LogicalLiteral(_, _) => Ok(Type::logical()),

            Expr::Identifier(name, location) => {
                let symbol = self.lookup_variable(name, *location)?;
                Ok(symbol.ty.clone())
            }

            Expr::BinaryOp {
                op,
                left,
                right,
                location,
            } => {
                let left_type = self.check_expression(left)?;
                let right_type = self.check_expression(right)?;

                match op {
                    // Arithmetic operators
                    BinaryOperator::Add
                    | BinaryOperator::Subtract
                    | BinaryOperator::Multiply
                    | BinaryOperator::Divide
                    | BinaryOperator::Power => {
                        if let Some(result_type) =
                            Type::arithmetic_result_type(&left_type, &right_type)
                        {
                            Ok(result_type)
                        } else {
                            Err(SemanticError::InvalidOperation {
                                operation: format!("{:?}", op),
                                left_type,
                                right_type,
                                location: *location,
                            })
                        }
                    }

                    // Relational operators
                    BinaryOperator::Equal
                    | BinaryOperator::NotEqual
                    | BinaryOperator::Less
                    | BinaryOperator::LessEqual
                    | BinaryOperator::Greater
                    | BinaryOperator::GreaterEqual => {
                        if let Some(result_type) =
                            Type::relational_result_type(&left_type, &right_type)
                        {
                            Ok(result_type)
                        } else {
                            Err(SemanticError::InvalidOperation {
                                operation: format!("{:?}", op),
                                left_type,
                                right_type,
                                location: *location,
                            })
                        }
                    }

                    // Logical operators
                    BinaryOperator::And | BinaryOperator::Or | BinaryOperator::Eqv | BinaryOperator::Neqv => {
                        if let Some(result_type) =
                            Type::logical_result_type(&left_type, &right_type)
                        {
                            Ok(result_type)
                        } else {
                            Err(SemanticError::InvalidOperation {
                                operation: format!("{:?}", op),
                                left_type,
                                right_type,
                                location: *location,
                            })
                        }
                    }
                }
            }

            Expr::UnaryOp {
                op,
                operand,
                location,
            } => {
                let operand_type = self.check_expression(operand)?;

                match op {
                    UnaryOperator::Plus | UnaryOperator::Minus => {
                        if operand_type.is_numeric() {
                            Ok(operand_type)
                        } else {
                            Err(SemanticError::InvalidOperation {
                                operation: format!("{:?}", op),
                                left_type: operand_type,
                                right_type: Type::Void,
                                location: *location,
                            })
                        }
                    }
                    UnaryOperator::Not => {
                        if operand_type.is_logical() {
                            Ok(Type::logical())
                        } else {
                            Err(SemanticError::InvalidOperation {
                                operation: "NOT".to_string(),
                                left_type: operand_type,
                                right_type: Type::Void,
                                location: *location,
                            })
                        }
                    }
                }
            }

            Expr::Parenthesized(inner, _) => self.check_expression(inner),

            Expr::FunctionCall { arguments, .. } => {
                // TODO: Full implementation in Sprint 08
                // For now, just type check arguments and return integer
                for arg in arguments {
                    self.check_expression(arg)?;
                }
                Ok(Type::integer())
            }

            Expr::ArrayAccess { name, indices, location } => {
                // Look up the array variable
                let symbol = self.lookup_variable(name, *location)?;

                // Check that it's an array
                if !symbol.ty.is_array() {
                    return Err(SemanticError::TypeMismatch {
                        expected: Type::Array {
                            element_kind: ArrayElementKind::Integer,
                            dimensions: vec![],
                        },
                        found: symbol.ty.clone(),
                        location: *location,
                    });
                }

                // Check index types (must be integer/numeric)
                for idx in indices {
                    let idx_type = self.check_expression(idx)?;
                    if !idx_type.is_numeric() {
                        return Err(SemanticError::TypeMismatch {
                            expected: Type::integer(),
                            found: idx_type,
                            location: *idx.location(),
                        });
                    }
                }

                // Return element type
                Ok(symbol.ty.element_type())
            }

            Expr::ComponentAccess { object, .. } => {
                // TODO: Full implementation - look up component type from derived type definition
                // For now, just check the object and return a generic type
                self.check_expression(object)?;
                Ok(Type::real())
            }

            Expr::TypeConstructor { arguments, type_name, .. } => {
                // TODO: Full implementation - verify arguments match type definition
                // For now, just type check arguments
                for arg in arguments {
                    self.check_expression(arg)?;
                }
                Ok(Type::Derived { name: type_name.clone() })
            }
        }
    }

    /// Evaluate a constant integer expression (for array bounds)
    fn eval_constant_int(&self, expr: &Expr) -> Option<i64> {
        match expr {
            Expr::IntegerLiteral(value, _) => Some(*value),
            Expr::UnaryOp { op: UnaryOperator::Minus, operand, .. } => {
                self.eval_constant_int(operand).map(|v| -v)
            }
            Expr::UnaryOp { op: UnaryOperator::Plus, operand, .. } => {
                self.eval_constant_int(operand)
            }
            // Could add more constant evaluation (parameters, simple arithmetic)
            _ => None,
        }
    }

    /// Look up a variable in the symbol table
    /// If IMPLICIT NONE is not in effect, creates implicit symbol
    fn lookup_variable(&mut self, name: &str, location: SourceLocation) -> SemanticResult<Symbol> {
        if let Some(symbol) = self.symbol_table.lookup(name) {
            return Ok(symbol.clone());
        }

        // Variable not found
        if self.symbol_table.is_implicit_none() {
            Err(SemanticError::UndeclaredVariable {
                name: name.to_string(),
                location,
            })
        } else {
            // Use implicit typing
            if let Some(ty) = self.symbol_table.get_implicit_type(name) {
                // Create implicit symbol
                let symbol = Symbol::new(name.to_string(), ty, location);
                // Note: we don't add it to the symbol table as implicit variables
                // are allowed to be redeclared explicitly
                Ok(symbol)
            } else {
                Err(SemanticError::UndeclaredVariable {
                    name: name.to_string(),
                    location,
                })
            }
        }
    }

    /// Get the symbol table (for inspection/testing)
    pub fn symbol_table(&self) -> &SymbolTable {
        &self.symbol_table
    }

    /// Get collected errors
    pub fn errors(&self) -> &[SemanticError] {
        &self.errors
    }
}

impl Default for SemanticAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_numeric() {
        assert!(Type::integer().is_numeric());
        assert!(Type::real().is_numeric());
        assert!(Type::double_precision().is_numeric());
        assert!(Type::complex().is_numeric());
        assert!(!Type::logical().is_numeric());
        assert!(!Type::character().is_numeric());
    }

    #[test]
    fn test_type_assignability() {
        // Same types are assignable
        assert!(Type::integer().is_assignable_from(&Type::integer()));
        assert!(Type::real().is_assignable_from(&Type::real()));

        // Numeric types can be assigned to each other
        assert!(Type::real().is_assignable_from(&Type::integer()));
        assert!(Type::integer().is_assignable_from(&Type::real()));
        assert!(Type::double_precision().is_assignable_from(&Type::integer()));

        // Complex can receive any numeric type
        assert!(Type::complex().is_assignable_from(&Type::integer()));
        assert!(Type::complex().is_assignable_from(&Type::real()));

        // Logical cannot be assigned from numeric
        assert!(!Type::logical().is_assignable_from(&Type::integer()));
        assert!(!Type::integer().is_assignable_from(&Type::logical()));
    }

    #[test]
    fn test_arithmetic_result_type() {
        // Integer + Integer = Integer
        assert_eq!(
            Type::arithmetic_result_type(&Type::integer(), &Type::integer()),
            Some(Type::integer())
        );

        // Integer + Real = Real
        assert_eq!(
            Type::arithmetic_result_type(&Type::integer(), &Type::real()),
            Some(Type::real())
        );

        // Real + Integer = Real
        assert_eq!(
            Type::arithmetic_result_type(&Type::real(), &Type::integer()),
            Some(Type::real())
        );

        // Double precision propagates
        assert_eq!(
            Type::arithmetic_result_type(&Type::integer(), &Type::double_precision()),
            Some(Type::double_precision())
        );

        // Complex propagates
        assert_eq!(
            Type::arithmetic_result_type(&Type::complex(), &Type::integer()),
            Some(Type::complex())
        );
    }

    #[test]
    fn test_relational_result_type() {
        // Numeric comparisons return logical
        assert_eq!(
            Type::relational_result_type(&Type::integer(), &Type::integer()),
            Some(Type::logical())
        );
        assert_eq!(
            Type::relational_result_type(&Type::real(), &Type::integer()),
            Some(Type::logical())
        );

        // Logical cannot be compared with <, >, etc. (only == and /=)
        assert_eq!(
            Type::relational_result_type(&Type::logical(), &Type::logical()),
            Some(Type::logical())
        );

        // Invalid comparisons
        assert_eq!(
            Type::relational_result_type(&Type::integer(), &Type::logical()),
            None
        );
    }

    #[test]
    fn test_logical_result_type() {
        // Logical operations require logical operands
        assert_eq!(
            Type::logical_result_type(&Type::logical(), &Type::logical()),
            Some(Type::logical())
        );

        // Invalid logical operations
        assert_eq!(
            Type::logical_result_type(&Type::integer(), &Type::integer()),
            None
        );
        assert_eq!(
            Type::logical_result_type(&Type::logical(), &Type::integer()),
            None
        );
    }

    #[test]
    fn test_symbol_table_define_and_lookup() {
        let mut table = SymbolTable::new();
        let loc = SourceLocation { line: 1, column: 1 };

        let symbol = Symbol::new("X".to_string(), Type::integer(), loc);
        table.define(symbol).unwrap();

        let found = table.lookup("X");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "X");
        assert_eq!(found.unwrap().ty, Type::integer());

        // Lookup is case-insensitive
        let found = table.lookup("x");
        assert!(found.is_some());
    }

    #[test]
    fn test_symbol_table_duplicate_declaration() {
        let mut table = SymbolTable::new();
        let loc = SourceLocation { line: 1, column: 1 };

        let symbol1 = Symbol::new("X".to_string(), Type::integer(), loc);
        table.define(symbol1).unwrap();

        let symbol2 = Symbol::new("X".to_string(), Type::real(), loc);
        let result = table.define(symbol2);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SemanticError::DuplicateDeclaration { .. }
        ));
    }

    #[test]
    fn test_symbol_table_scoping() {
        let mut table = SymbolTable::new();
        let loc = SourceLocation { line: 1, column: 1 };

        // Define in outer scope
        let symbol1 = Symbol::new("X".to_string(), Type::integer(), loc);
        table.define(symbol1).unwrap();

        // Enter inner scope
        table.enter_scope();

        // Shadow with different type
        let symbol2 = Symbol::new("X".to_string(), Type::real(), loc);
        table.define(symbol2).unwrap();

        // Lookup finds inner scope version
        let found = table.lookup("X").unwrap();
        assert_eq!(found.ty, Type::real());

        // Exit inner scope
        table.exit_scope();

        // Now finds outer scope version
        let found = table.lookup("X").unwrap();
        assert_eq!(found.ty, Type::integer());
    }

    #[test]
    fn test_implicit_typing() {
        let table = SymbolTable::new();

        // I-N are implicit INTEGER
        assert_eq!(table.get_implicit_type("I"), Some(Type::integer()));
        assert_eq!(table.get_implicit_type("J"), Some(Type::integer()));
        assert_eq!(table.get_implicit_type("N"), Some(Type::integer()));

        // Others are implicit REAL
        assert_eq!(table.get_implicit_type("A"), Some(Type::real()));
        assert_eq!(table.get_implicit_type("X"), Some(Type::real()));
        assert_eq!(table.get_implicit_type("Z"), Some(Type::real()));
    }

    #[test]
    fn test_implicit_none() {
        let mut table = SymbolTable::new();
        assert!(!table.is_implicit_none());

        table.set_implicit_none();
        assert!(table.is_implicit_none());

        // With IMPLICIT NONE, no implicit typing
        assert_eq!(table.get_implicit_type("I"), None);
        assert_eq!(table.get_implicit_type("X"), None);
    }
}
