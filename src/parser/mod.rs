//! Recursive descent parser for Fortran

use crate::ast::*;
use crate::lexer::{Token, TokenType, SourceLocation};
use std::fmt;

/// Parser error types
#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    UnexpectedToken {
        expected: String,
        found: TokenType,
        location: SourceLocation,
    },
    UnexpectedEof {
        expected: String,
        location: SourceLocation,
    },
    InvalidNumber {
        value: String,
        location: SourceLocation,
    },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::UnexpectedToken {
                expected,
                found,
                location,
            } => {
                write!(
                    f,
                    "Expected {} but found {} at {}",
                    expected, found, location
                )
            }
            ParseError::UnexpectedEof { expected, location } => {
                write!(f, "Expected {} but reached end of file at {}", expected, location)
            }
            ParseError::InvalidNumber { value, location } => {
                write!(f, "Invalid number '{}' at {}", value, location)
            }
        }
    }
}

impl std::error::Error for ParseError {}

pub type ParseResult<T> = Result<T, ParseError>;

/// Recursive descent parser for Fortran
pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

impl Parser {
    /// Create a new parser from a token stream
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            position: 0,
        }
    }

    /// Parse a complete compilation unit (modules + program)
    pub fn parse_compilation_unit(&mut self) -> ParseResult<CompilationUnit> {
        let mut modules = Vec::new();

        // Parse any modules first
        while self.check(&TokenType::Module) {
            modules.push(self.parse_module()?);
        }

        // Parse program if present
        let program = if !self.is_at_end() {
            Some(self.parse_program()?)
        } else {
            None
        };

        Ok(CompilationUnit { modules, program })
    }

    /// Parse a complete Fortran program
    pub fn parse_program(&mut self) -> ParseResult<Program> {
        let location = self.current_location();

        // Optionally parse PROGRAM declaration
        let name = if self.check(&TokenType::Program) {
            self.advance(); // consume PROGRAM
            let name = self.expect_identifier()?;
            Some(name)
        } else {
            None
        };

        let mut uses = Vec::new();
        let mut declarations = Vec::new();
        let mut statements = Vec::new();
        let mut procedures = Vec::new();

        // Parse USE statements first (must come before declarations)
        while self.check(&TokenType::Use) {
            uses.push(self.parse_use_statement()?);
        }

        // Parse declarations and statements until END, CONTAINS, or EOF
        loop {
            // Check for EOF
            if self.is_at_end() {
                break;
            }

            // Check for CONTAINS (internal procedures follow)
            if self.check(&TokenType::Contains) {
                self.advance();
                // Parse internal procedures
                procedures = self.parse_procedures()?;
                break;
            }

            // Check for END PROGRAM/END (any END token stops program parsing)
            if self.check(&TokenType::End) {
                break;
            }

            if self.is_declaration_start() {
                declarations.push(self.parse_declaration()?);
            } else {
                statements.push(self.parse_statement()?);
            }
        }

        // Expect END PROGRAM (optional in some cases)
        if self.check(&TokenType::End) {
            self.advance();
            // Optional PROGRAM keyword
            if self.check(&TokenType::Program) {
                self.advance();
                // Optional program name after END PROGRAM
                if let TokenType::Identifier(_) = self.peek().token_type {
                    self.advance();
                }
            }
        }

        Ok(Program {
            name,
            uses,
            declarations,
            statements,
            procedures,
            location,
        })
    }

    /// Parse a Fortran module
    pub fn parse_module(&mut self) -> ParseResult<ModuleDef> {
        let location = self.current_location();

        // Expect MODULE keyword
        self.expect(&TokenType::Module, "MODULE")?;
        let name = self.expect_identifier()?;

        let mut default_visibility = Visibility::Public;
        let mut uses = Vec::new();
        let mut declarations = Vec::new();
        let mut visibility_stmts = Vec::new();
        let mut procedures = Vec::new();

        // Parse module body: USE statements, declarations, PUBLIC/PRIVATE, CONTAINS
        loop {
            if self.is_at_end() {
                break;
            }

            // Check for CONTAINS
            if self.check(&TokenType::Contains) {
                self.advance();
                procedures = self.parse_procedures()?;
                break;
            }

            // Check for END MODULE
            if self.check(&TokenType::End) {
                break;
            }

            // Parse USE statements
            if self.check(&TokenType::Use) {
                uses.push(self.parse_use_statement()?);
                continue;
            }

            // Parse PUBLIC/PRIVATE statements
            if self.check(&TokenType::Public) || self.check(&TokenType::Private) {
                let vis_stmt = self.parse_visibility_statement()?;
                // If no names, this sets default visibility
                if vis_stmt.names.is_empty() {
                    default_visibility = vis_stmt.visibility;
                }
                visibility_stmts.push(vis_stmt);
                continue;
            }

            // Parse declarations
            if self.is_declaration_start() {
                declarations.push(self.parse_declaration()?);
                continue;
            }

            // Skip unknown tokens (shouldn't happen in well-formed code)
            break;
        }

        // Expect END MODULE
        if self.check(&TokenType::End) {
            self.advance();
            if self.check(&TokenType::Module) {
                self.advance();
                // Optional module name
                if let TokenType::Identifier(_) = self.peek().token_type {
                    self.advance();
                }
            }
        }

        Ok(ModuleDef {
            name,
            default_visibility,
            uses,
            declarations,
            visibility_stmts,
            procedures,
            location,
        })
    }

    /// Parse a USE statement
    fn parse_use_statement(&mut self) -> ParseResult<UseStatement> {
        let location = self.current_location();

        self.expect(&TokenType::Use, "USE")?;
        let module_name = self.expect_identifier()?;

        // Check for ONLY clause
        let only = if self.check(&TokenType::Comma) {
            self.advance();
            if self.check(&TokenType::Only) {
                self.advance();
                self.expect(&TokenType::Colon, ":")?;

                // Parse list of items
                let mut items = Vec::new();
                loop {
                    let first_name = self.expect_identifier()?;

                    // Check for renaming: local_name => original_name
                    if self.check(&TokenType::Arrow) {
                        self.advance();
                        let original = self.expect_identifier()?;
                        items.push(UseItem {
                            local_name: first_name,
                            original_name: Some(original),
                        });
                    } else {
                        items.push(UseItem {
                            local_name: first_name.clone(),
                            original_name: None,
                        });
                    }

                    if self.check(&TokenType::Comma) {
                        self.advance();
                    } else {
                        break;
                    }
                }
                Some(items)
            } else {
                // Renaming without ONLY: USE mod, local => original
                let mut items = Vec::new();
                loop {
                    let first_name = self.expect_identifier()?;

                    if self.check(&TokenType::Arrow) {
                        self.advance();
                        let original = self.expect_identifier()?;
                        items.push(UseItem {
                            local_name: first_name,
                            original_name: Some(original),
                        });
                    } else {
                        items.push(UseItem {
                            local_name: first_name.clone(),
                            original_name: None,
                        });
                    }

                    if self.check(&TokenType::Comma) {
                        self.advance();
                    } else {
                        break;
                    }
                }
                Some(items)
            }
        } else {
            None
        };

        Ok(UseStatement {
            module_name,
            only,
            location,
        })
    }

    /// Parse a PUBLIC or PRIVATE statement
    fn parse_visibility_statement(&mut self) -> ParseResult<VisibilityStmt> {
        let location = self.current_location();

        let visibility = if self.check(&TokenType::Public) {
            self.advance();
            Visibility::Public
        } else if self.check(&TokenType::Private) {
            self.advance();
            Visibility::Private
        } else {
            return Err(ParseError::UnexpectedToken {
                expected: "PUBLIC or PRIVATE".to_string(),
                found: self.peek().token_type.clone(),
                location,
            });
        };

        let mut names = Vec::new();

        // Check for :: and list of names
        if self.check(&TokenType::DoubleColon) {
            self.advance();
            loop {
                names.push(self.expect_identifier()?);
                if self.check(&TokenType::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
        } else if self.check(&TokenType::Colon) {
            // Single colon also acceptable in some cases
            self.advance();
            loop {
                names.push(self.expect_identifier()?);
                if self.check(&TokenType::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
        }
        // If neither :: nor names, this just sets default visibility

        Ok(VisibilityStmt {
            visibility,
            names,
            location,
        })
    }

    /// Parse internal procedures after CONTAINS
    fn parse_procedures(&mut self) -> ParseResult<Vec<Procedure>> {
        let mut procedures = Vec::new();

        loop {
            if self.is_at_end() {
                break;
            }

            // Check for END PROGRAM (end of procedures)
            if self.check(&TokenType::End) {
                break;
            }

            // Parse RECURSIVE prefix (optional)
            let is_recursive = if self.check(&TokenType::Recursive) {
                self.advance();
                true
            } else {
                false
            };

            // Parse SUBROUTINE or FUNCTION
            if self.check(&TokenType::Subroutine) {
                procedures.push(Procedure::Subroutine(self.parse_subroutine()?));
            } else if self.check(&TokenType::Function) {
                procedures.push(Procedure::Function(self.parse_function(is_recursive)?));
            } else if is_recursive {
                // RECURSIVE without FUNCTION is an error
                return Err(ParseError::UnexpectedToken {
                    expected: "FUNCTION after RECURSIVE".to_string(),
                    found: self.peek().token_type.clone(),
                    location: self.current_location(),
                });
            } else {
                // Unknown token in procedures section
                return Err(ParseError::UnexpectedToken {
                    expected: "SUBROUTINE or FUNCTION".to_string(),
                    found: self.peek().token_type.clone(),
                    location: self.current_location(),
                });
            }
        }

        Ok(procedures)
    }

    /// Parse a subroutine definition
    fn parse_subroutine(&mut self) -> ParseResult<SubroutineDef> {
        let location = self.current_location();
        self.expect(&TokenType::Subroutine, "SUBROUTINE")?;

        let name = self.expect_identifier()?;

        // Parse parameter list
        let parameters = if self.check(&TokenType::LeftParen) {
            self.advance();
            let params = self.parse_parameter_names()?;
            self.expect(&TokenType::RightParen, ")")?;
            params
        } else {
            Vec::new()
        };

        // Parse declarations and body
        let mut declarations = Vec::new();
        let mut body = Vec::new();

        loop {
            if self.is_at_end() {
                break;
            }

            // Check for END SUBROUTINE
            if self.check(&TokenType::End) {
                break;
            }

            if self.is_declaration_start() {
                declarations.push(self.parse_declaration()?);
            } else {
                body.push(self.parse_statement()?);
            }
        }

        // Expect END SUBROUTINE
        if self.check(&TokenType::End) {
            self.advance();
            if self.check(&TokenType::Subroutine) {
                self.advance();
                // Optional subroutine name
                if let TokenType::Identifier(_) = self.peek().token_type {
                    self.advance();
                }
            }
        }

        Ok(SubroutineDef {
            name,
            parameters,
            declarations,
            body,
            location,
        })
    }

    /// Parse a function definition
    fn parse_function(&mut self, is_recursive: bool) -> ParseResult<FunctionDef> {
        let location = self.current_location();

        // Optional return type prefix
        let return_type = if self.is_type_spec_start() {
            Some(self.parse_type_spec()?)
        } else {
            None
        };

        self.expect(&TokenType::Function, "FUNCTION")?;

        let name = self.expect_identifier()?;

        // Parse parameter list
        let parameters = if self.check(&TokenType::LeftParen) {
            self.advance();
            let params = self.parse_parameter_names()?;
            self.expect(&TokenType::RightParen, ")")?;
            params
        } else {
            Vec::new()
        };

        // Parse optional RESULT clause
        let result_name = if self.check(&TokenType::Result) {
            self.advance();
            self.expect(&TokenType::LeftParen, "(")?;
            let result = self.expect_identifier()?;
            self.expect(&TokenType::RightParen, ")")?;
            Some(result)
        } else {
            None
        };

        // Parse declarations and body
        let mut declarations = Vec::new();
        let mut body = Vec::new();

        loop {
            if self.is_at_end() {
                break;
            }

            // Check for END FUNCTION
            if self.check(&TokenType::End) {
                break;
            }

            if self.is_declaration_start() {
                declarations.push(self.parse_declaration()?);
            } else {
                body.push(self.parse_statement()?);
            }
        }

        // Expect END FUNCTION
        if self.check(&TokenType::End) {
            self.advance();
            if self.check(&TokenType::Function) {
                self.advance();
                // Optional function name
                if let TokenType::Identifier(_) = self.peek().token_type {
                    self.advance();
                }
            }
        }

        Ok(FunctionDef {
            name,
            parameters,
            return_type,
            result_name,
            is_recursive,
            declarations,
            body,
            location,
        })
    }

    /// Parse parameter names (just identifiers for now)
    fn parse_parameter_names(&mut self) -> ParseResult<Vec<Parameter>> {
        let mut params = Vec::new();

        // Empty parameter list
        if self.check(&TokenType::RightParen) {
            return Ok(params);
        }

        loop {
            let location = self.current_location();
            let name = self.expect_identifier()?;
            params.push(Parameter {
                name,
                type_spec: None,
                intent: None,
                location,
            });

            if self.check(&TokenType::Comma) {
                self.advance();
            } else {
                break;
            }
        }

        Ok(params)
    }

    /// Check if current token starts a type specification
    fn is_type_spec_start(&self) -> bool {
        matches!(
            self.peek().token_type,
            TokenType::Integer
                | TokenType::Real
                | TokenType::Double
                | TokenType::Complex
                | TokenType::Logical
                | TokenType::Character
                | TokenType::Type
                | TokenType::Class
        )
    }

    /// Check if current token starts a declaration
    fn is_declaration_start(&self) -> bool {
        matches!(
            self.peek().token_type,
            TokenType::Integer
                | TokenType::Real
                | TokenType::Double
                | TokenType::Complex
                | TokenType::Logical
                | TokenType::Character
                | TokenType::Implicit
                | TokenType::Parameter
                | TokenType::Type
                | TokenType::Class
                | TokenType::Interface
        )
    }

    /// Parse a declaration statement
    fn parse_declaration(&mut self) -> ParseResult<Declaration> {
        let location = self.current_location();

        // IMPLICIT NONE
        if self.check(&TokenType::Implicit) {
            self.advance();
            self.expect(&TokenType::None, "NONE")?;
            return Ok(Declaration::ImplicitNone { location });
        }

        // INTERFACE block (for operator overloading and generic interfaces)
        if self.check(&TokenType::Interface) {
            return self.parse_interface_block();
        }

        // Check for TYPE definition (TYPE :: name or TYPE, EXTENDS(...) :: name)
        // This is different from TYPE(name) :: var declarations
        if self.check(&TokenType::Type) {
            // Peek ahead to see if this is TYPE :: (definition) or TYPE(name) (declaration)
            // TYPE definition: TYPE :: name or TYPE, EXTENDS(...) :: name
            // TYPE declaration: TYPE(name) :: varname
            let next = &self.peek_next().token_type;
            if *next == TokenType::DoubleColon || *next == TokenType::Comma {
                return self.parse_derived_type_def();
            }
            // Otherwise fall through to type spec parsing for TYPE(name) :: var
        }

        // Type declarations
        let type_spec = self.parse_type_spec()?;

        // Check for attributes (e.g., INTEGER, PARAMETER :: ...)
        let mut is_parameter = false;
        let mut intent: Option<Intent> = None;
        let mut is_optional = false;

        // Parse comma-separated attributes
        while self.check(&TokenType::Comma) {
            self.advance();
            // Parse attribute
            if self.check(&TokenType::Parameter) {
                self.advance();
                is_parameter = true;
            } else if self.check(&TokenType::Intent) {
                self.advance();
                self.expect(&TokenType::LeftParen, "(")?;
                // Parse IN, OUT, or INOUT
                if self.check(&TokenType::In) {
                    self.advance();
                    if self.check(&TokenType::Out) {
                        // INTENT(INOUT) - "IN" followed by "OUT"
                        self.advance();
                        intent = Some(Intent::InOut);
                    } else {
                        intent = Some(Intent::In);
                    }
                } else if self.check(&TokenType::Out) {
                    self.advance();
                    intent = Some(Intent::Out);
                } else if self.check(&TokenType::InOut) {
                    // INOUT as single token
                    self.advance();
                    intent = Some(Intent::InOut);
                } else if let TokenType::Identifier(ident) = &self.peek().token_type {
                    // Check for INOUT as single identifier
                    if ident.to_uppercase() == "INOUT" {
                        self.advance();
                        intent = Some(Intent::InOut);
                    } else {
                        return Err(ParseError::UnexpectedToken {
                            expected: "IN, OUT, or INOUT".to_string(),
                            found: self.peek().token_type.clone(),
                            location: self.current_location(),
                        });
                    }
                } else {
                    return Err(ParseError::UnexpectedToken {
                        expected: "IN, OUT, or INOUT".to_string(),
                        found: self.peek().token_type.clone(),
                        location: self.current_location(),
                    });
                }
                self.expect(&TokenType::RightParen, ")")?;
            } else if self.check(&TokenType::Optional) {
                self.advance();
                is_optional = true;
            }
            // Could add more attributes here (DIMENSION, etc.)
        }

        // Check for :: (optional but common in modern Fortran)
        if self.check(&TokenType::DoubleColon) {
            self.advance();
        }

        // If PARAMETER attribute, parse as parameter declaration
        if is_parameter {
            let name = self.expect_identifier()?;
            self.expect(&TokenType::Equal, "=")?;
            let value = self.parse_expression()?;

            return Ok(Declaration::Parameter {
                type_spec,
                name,
                value,
                location,
            });
        }

        // Parse variable names with optional array dimensions
        let mut names = Vec::new();
        let mut entities = Vec::new();
        let mut init_values = Vec::new();

        loop {
            let name = self.expect_identifier()?;
            names.push(name.clone());

            // Check for array dimensions: name(dim1, dim2, ...)
            let array_spec = if self.check(&TokenType::LeftParen) {
                Some(self.parse_array_spec()?)
            } else {
                None
            };

            // Check for initialization
            let init_expr = if self.check(&TokenType::Equal) {
                self.advance();
                let value = self.parse_expression()?;
                init_values.push(Some(value.clone()));
                Some(value)
            } else {
                init_values.push(None);
                None
            };

            // Build the entity with parsed attributes
            let mut attributes = VarAttributes::default();
            attributes.intent = intent;
            attributes.is_optional = is_optional;

            let entity = DeclaredEntity {
                name,
                array_spec,
                init: init_expr,
                attributes,
            };
            entities.push(entity);

            if self.check(&TokenType::Comma) {
                self.advance();
            } else {
                break;
            }
        }

        let init = if init_values.iter().any(|v| v.is_some()) {
            Some(init_values)
        } else {
            None
        };

        Ok(Declaration::Variable {
            type_spec,
            names,
            entities,
            init,
            location,
        })
    }

    /// Parse array dimension specification: (dim1, dim2, ...)
    /// Supports: (10), (1:10), (0:9, 1:5)
    fn parse_array_spec(&mut self) -> ParseResult<ArraySpec> {
        self.expect(&TokenType::LeftParen, "(")?;

        let mut dimensions = Vec::new();

        loop {
            let dim = self.parse_array_dim_spec()?;
            dimensions.push(dim);

            if self.check(&TokenType::Comma) {
                self.advance();
            } else {
                break;
            }
        }

        self.expect(&TokenType::RightParen, ")")?;

        Ok(ArraySpec { dimensions })
    }

    /// Parse a single array dimension specification
    /// Supports: 10 (upper only), 1:10 (lower:upper)
    fn parse_array_dim_spec(&mut self) -> ParseResult<ArrayDimSpec> {
        let first = self.parse_expression()?;

        if self.check(&TokenType::Colon) {
            // lower:upper form
            self.advance();
            let upper = self.parse_expression()?;
            Ok(ArrayDimSpec::with_bounds(first, upper))
        } else {
            // upper only form
            Ok(ArrayDimSpec::new(first))
        }
    }

    /// Parse a type specification
    fn parse_type_spec(&mut self) -> ParseResult<TypeSpec> {
        let location = self.current_location();

        match &self.peek().token_type {
            TokenType::Integer => {
                self.advance();
                // TODO: Parse KIND specification
                Ok(TypeSpec::integer())
            }
            TokenType::Real => {
                self.advance();
                Ok(TypeSpec::real())
            }
            TokenType::Double => {
                self.advance();
                if self.check(&TokenType::Precision) {
                    self.advance();
                }
                Ok(TypeSpec::double_precision())
            }
            TokenType::Complex => {
                self.advance();
                Ok(TypeSpec::complex())
            }
            TokenType::Logical => {
                self.advance();
                Ok(TypeSpec::logical())
            }
            TokenType::Character => {
                self.advance();
                // TODO: Parse LEN specification
                Ok(TypeSpec::character())
            }
            TokenType::Type => {
                self.advance();
                self.expect(&TokenType::LeftParen, "(")?;
                let type_name = self.expect_identifier()?;
                self.expect(&TokenType::RightParen, ")")?;
                Ok(TypeSpec::Derived { name: type_name, is_class: false })
            }
            TokenType::Class => {
                self.advance();
                self.expect(&TokenType::LeftParen, "(")?;
                let type_name = self.expect_identifier()?;
                self.expect(&TokenType::RightParen, ")")?;
                Ok(TypeSpec::Derived { name: type_name, is_class: true })
            }
            _ => Err(ParseError::UnexpectedToken {
                expected: "type specification".to_string(),
                found: self.peek().token_type.clone(),
                location,
            }),
        }
    }

    /// Parse a derived type definition: TYPE :: typename ... END TYPE
    fn parse_derived_type_def(&mut self) -> ParseResult<Declaration> {
        let location = self.current_location();

        self.expect(&TokenType::Type, "TYPE")?;

        // Check for optional EXTENDS
        let extends = if self.check(&TokenType::Comma) {
            self.advance();
            if self.check(&TokenType::Extends) {
                self.advance();
                self.expect(&TokenType::LeftParen, "(")?;
                let parent = self.expect_identifier()?;
                self.expect(&TokenType::RightParen, ")")?;
                Some(parent)
            } else {
                None
            }
        } else {
            None
        };

        self.expect(&TokenType::DoubleColon, "::")?;
        let name = self.expect_identifier()?;

        let mut components = Vec::new();
        let mut procedures = Vec::new();
        let mut in_contains = false;

        loop {
            if self.is_at_end() {
                break;
            }

            // Check for END TYPE
            if self.check(&TokenType::End) {
                self.advance();
                if self.check(&TokenType::Type) {
                    self.advance();
                    // Optional type name after END TYPE
                    if let TokenType::Identifier(_) = self.peek().token_type {
                        self.advance();
                    }
                }
                break;
            }

            // Check for CONTAINS (starts type-bound procedure section)
            if self.check(&TokenType::Contains) {
                self.advance();
                in_contains = true;
                continue;
            }

            if in_contains {
                // Parse type-bound procedure
                procedures.push(self.parse_type_bound_procedure()?);
            } else {
                // Parse component declaration
                components.push(self.parse_type_component()?);
            }
        }

        Ok(Declaration::DerivedType(DerivedTypeDef {
            name,
            extends,
            components,
            procedures,
            location,
        }))
    }

    /// Parse a type component (member variable)
    fn parse_type_component(&mut self) -> ParseResult<TypeComponent> {
        let location = self.current_location();

        let type_spec = self.parse_type_spec()?;

        // Optional attributes
        if self.check(&TokenType::Comma) {
            self.advance();
            // Skip attributes for now (POINTER, ALLOCATABLE, etc.)
            while !self.check(&TokenType::DoubleColon) && !self.is_at_end() {
                self.advance();
            }
        }

        // Check for ::
        if self.check(&TokenType::DoubleColon) {
            self.advance();
        }

        let name = self.expect_identifier()?;

        // Check for array dimensions
        let array_spec = if self.check(&TokenType::LeftParen) {
            Some(self.parse_array_spec()?)
        } else {
            None
        };

        // Check for initialization
        let init = if self.check(&TokenType::Equal) {
            self.advance();
            Some(self.parse_expression()?)
        } else {
            None
        };

        Ok(TypeComponent {
            name,
            type_spec,
            array_spec,
            init,
            location,
        })
    }

    /// Parse a type-bound procedure binding
    fn parse_type_bound_procedure(&mut self) -> ParseResult<TypeBoundProcedure> {
        let location = self.current_location();

        self.expect(&TokenType::Procedure, "PROCEDURE")?;

        // Check for attributes like PASS, NOPASS
        let mut pass_arg = None;
        let nopass = false;

        if self.check(&TokenType::LeftParen) {
            self.advance();
            // Parse PASS(arg)
            pass_arg = if let TokenType::Identifier(s) = &self.peek().token_type {
                let arg = s.clone();
                self.advance();
                Some(arg)
            } else {
                None
            };
            self.expect(&TokenType::RightParen, ")")?;
        }

        // Check for ::
        if self.check(&TokenType::DoubleColon) {
            self.advance();
        }

        let binding_name = self.expect_identifier()?;

        // Check for => procedure_name
        let procedure_name = if self.check(&TokenType::Arrow) {
            self.advance();
            Some(self.expect_identifier()?)
        } else {
            None
        };

        Ok(TypeBoundProcedure {
            binding_name,
            procedure_name,
            pass_arg,
            nopass,
            location,
        })
    }

    /// Parse an INTERFACE block
    fn parse_interface_block(&mut self) -> ParseResult<Declaration> {
        let location = self.current_location();

        self.expect(&TokenType::Interface, "INTERFACE")?;

        // Determine interface kind
        let kind = if self.check(&TokenType::Operator) {
            // INTERFACE OPERATOR(+)
            self.advance();
            self.expect(&TokenType::LeftParen, "(")?;
            let op = self.parse_overloadable_operator()?;
            self.expect(&TokenType::RightParen, ")")?;
            InterfaceKind::Operator(op)
        } else if self.check(&TokenType::Assignment) {
            // INTERFACE ASSIGNMENT(=)
            self.advance();
            self.expect(&TokenType::LeftParen, "(")?;
            self.expect(&TokenType::Equal, "=")?;
            self.expect(&TokenType::RightParen, ")")?;
            InterfaceKind::Assignment
        } else if let TokenType::Identifier(name) = &self.peek().token_type {
            // INTERFACE generic_name
            let generic_name = name.clone();
            self.advance();
            InterfaceKind::Generic(generic_name)
        } else {
            // Abstract interface
            InterfaceKind::Abstract
        };

        // Parse procedures in the interface
        let mut procedures = Vec::new();

        loop {
            if self.is_at_end() {
                break;
            }

            // Check for END INTERFACE
            if self.check(&TokenType::End) {
                self.advance();
                if self.check(&TokenType::Interface) {
                    self.advance();
                    // Optional operator/name after END INTERFACE
                    if self.check(&TokenType::Operator) {
                        self.advance();
                        if self.check(&TokenType::LeftParen) {
                            self.advance();
                            // Skip the operator
                            while !self.check(&TokenType::RightParen) && !self.is_at_end() {
                                self.advance();
                            }
                            if self.check(&TokenType::RightParen) {
                                self.advance();
                            }
                        }
                    } else if self.check(&TokenType::Assignment) {
                        self.advance();
                        if self.check(&TokenType::LeftParen) {
                            self.advance();
                            if self.check(&TokenType::Equal) {
                                self.advance();
                            }
                            if self.check(&TokenType::RightParen) {
                                self.advance();
                            }
                        }
                    } else if let TokenType::Identifier(_) = self.peek().token_type {
                        self.advance();
                    }
                }
                break;
            }

            // Parse MODULE PROCEDURE name
            if self.check(&TokenType::Module) {
                self.advance();
                self.expect(&TokenType::Procedure, "PROCEDURE")?;

                // Parse procedure name(s)
                loop {
                    let proc_name = self.expect_identifier()?;
                    procedures.push(InterfaceProcedure {
                        module_procedure: Some(proc_name),
                        procedure_def: None,
                        location: self.current_location(),
                    });

                    if self.check(&TokenType::Comma) {
                        self.advance();
                    } else {
                        break;
                    }
                }
            } else if self.check(&TokenType::Procedure) {
                // Just PROCEDURE name (alternate syntax)
                self.advance();
                loop {
                    let proc_name = self.expect_identifier()?;
                    procedures.push(InterfaceProcedure {
                        module_procedure: Some(proc_name),
                        procedure_def: None,
                        location: self.current_location(),
                    });

                    if self.check(&TokenType::Comma) {
                        self.advance();
                    } else {
                        break;
                    }
                }
            } else {
                // Skip other content (inline function definitions etc.)
                self.advance();
            }
        }

        Ok(Declaration::Interface(InterfaceBlock {
            kind,
            procedures,
            location,
        }))
    }

    /// Parse an overloadable operator
    fn parse_overloadable_operator(&mut self) -> ParseResult<OverloadableOperator> {
        let op = match &self.peek().token_type {
            TokenType::Plus => OverloadableOperator::Add,
            TokenType::Minus => OverloadableOperator::Subtract,
            TokenType::Star => OverloadableOperator::Multiply,
            TokenType::Slash => OverloadableOperator::Divide,
            TokenType::Power => OverloadableOperator::Power,
            TokenType::EqualEqual => OverloadableOperator::Equal,
            TokenType::NotEqual => OverloadableOperator::NotEqual,
            TokenType::Less => OverloadableOperator::Less,
            TokenType::LessEqual => OverloadableOperator::LessEqual,
            TokenType::Greater => OverloadableOperator::Greater,
            TokenType::GreaterEqual => OverloadableOperator::GreaterEqual,
            TokenType::And => OverloadableOperator::And,
            TokenType::Or => OverloadableOperator::Or,
            TokenType::Not => OverloadableOperator::Not,
            TokenType::Eqv => OverloadableOperator::Eqv,
            TokenType::Neqv => OverloadableOperator::Neqv,
            TokenType::Identifier(name) if name.starts_with('.') && name.ends_with('.') => {
                OverloadableOperator::UserDefined(name.clone())
            }
            _ => {
                return Err(ParseError::UnexpectedToken {
                    expected: "operator".to_string(),
                    found: self.peek().token_type.clone(),
                    location: self.current_location(),
                });
            }
        };
        self.advance();
        Ok(op)
    }

    /// Parse a statement
    fn parse_statement(&mut self) -> ParseResult<Statement> {
        let location = self.current_location();

        // If we're at END, this is likely a block terminator, not a statement
        if self.check(&TokenType::End) {
            return Err(ParseError::UnexpectedToken {
                expected: "statement".to_string(),
                found: TokenType::End,
                location,
            });
        }

        // Control flow statements
        if self.check(&TokenType::If) {
            return self.parse_if_statement();
        }

        if self.check(&TokenType::Do) {
            return self.parse_do_statement();
        }

        if self.check(&TokenType::Select) {
            return self.parse_select_case();
        }

        if self.check(&TokenType::Exit) {
            self.advance();
            return Ok(Statement::Exit { location });
        }

        if self.check(&TokenType::Cycle) {
            self.advance();
            return Ok(Statement::Cycle { location });
        }

        if self.check(&TokenType::Continue) {
            self.advance();
            return Ok(Statement::Continue { location });
        }

        // SYNC statement (SYNC ALL or SYNC IMAGES)
        if self.check(&TokenType::Sync) {
            return self.parse_sync_statement();
        }

        // CRITICAL section
        if self.check(&TokenType::Critical) {
            return self.parse_critical_section();
        }

        // WHERE statement/construct
        if self.check(&TokenType::Where) {
            return self.parse_where_statement();
        }

        // FORALL statement/construct
        if self.check(&TokenType::Forall) {
            return self.parse_forall_statement();
        }

        // ASSOCIATE construct
        if self.check(&TokenType::Associate) {
            return self.parse_associate_construct();
        }

        // BLOCK construct
        if self.check(&TokenType::Block) {
            return self.parse_block_construct();
        }

        // CALL statement
        if self.check(&TokenType::Call) {
            return self.parse_call_statement();
        }

        // RETURN statement
        if self.check(&TokenType::Return) {
            return self.parse_return_statement();
        }

        // PRINT statement
        if self.check(&TokenType::Print) {
            return self.parse_print_statement();
        }

        // WRITE statement
        if self.check(&TokenType::Write) {
            return self.parse_write_statement();
        }

        // READ statement
        if self.check(&TokenType::Read) {
            return self.parse_read_statement();
        }

        // OPEN statement
        if self.check(&TokenType::Open) {
            return self.parse_open_statement();
        }

        // CLOSE statement
        if self.check(&TokenType::Close) {
            return self.parse_close_statement();
        }

        // ALLOCATE statement
        if self.check(&TokenType::Allocate) {
            return self.parse_allocate_statement();
        }

        // DEALLOCATE statement
        if self.check(&TokenType::Deallocate) {
            return self.parse_deallocate_statement();
        }

        // NULLIFY statement (check for identifier NULLIFY since it's not a keyword yet)
        if let TokenType::Identifier(name) = &self.peek().token_type {
            if name.to_uppercase() == "NULLIFY" {
                return self.parse_nullify_statement();
            }
        }

        // Assignment: identifier = expression or arr(i) = expression
        // Check if this could be an identifier (including keywords used as identifiers)
        let is_identifier_like = matches!(
            self.peek().token_type,
            TokenType::Identifier(_)
                | TokenType::Result
                | TokenType::Stat
                | TokenType::Kind
                | TokenType::Len
        );

        if is_identifier_like {
            let name = self.expect_identifier()?;

            // Check for array indices: arr(i, j, ...)
            let indices = if self.check(&TokenType::LeftParen) {
                self.advance();
                let mut idx_list = Vec::new();
                loop {
                    idx_list.push(self.parse_expression()?);
                    if self.check(&TokenType::Comma) {
                        self.advance();
                    } else {
                        break;
                    }
                }
                self.expect(&TokenType::RightParen, ")")?;
                Some(idx_list)
            } else {
                None
            };

            // Check for component access: p%x or p%x%y
            let mut components = Vec::new();
            while self.check(&TokenType::Percent) {
                self.advance();
                let comp = self.expect_identifier()?;
                components.push(comp);
            }

            if self.check(&TokenType::Equal) {
                self.advance();
                let value = self.parse_expression()?;
                return Ok(Statement::Assignment {
                    target: name,
                    indices,
                    components,
                    value,
                    location,
                });
            } else if self.check(&TokenType::Arrow) {
                // Pointer assignment: ptr => target
                self.advance();
                let target_expr = self.parse_expression()?;
                return Ok(Statement::PointerAssign {
                    pointer: name,
                    pointer_components: components,
                    target: target_expr,
                    location,
                });
            } else {
                return Err(ParseError::UnexpectedToken {
                    expected: "= or => (assignment)".to_string(),
                    found: self.peek().token_type.clone(),
                    location,
                });
            }
        }

        Err(ParseError::UnexpectedToken {
            expected: "statement".to_string(),
            found: self.peek().token_type.clone(),
            location,
        })
    }

    /// Parse PRINT statement
    fn parse_print_statement(&mut self) -> ParseResult<Statement> {
        let location = self.current_location();
        self.expect(&TokenType::Print, "PRINT")?;

        // TODO: Parse format spec
        // For now, expect * for list-directed
        if self.check(&TokenType::Star) {
            self.advance();
        }

        if self.check(&TokenType::Comma) {
            self.advance();
        }

        // Parse expression list
        let mut values = Vec::new();
        loop {
            values.push(self.parse_expression()?);
            if self.check(&TokenType::Comma) {
                self.advance();
            } else {
                break;
            }
        }

        Ok(Statement::Print {
            format: None,
            values,
            location,
        })
    }

    /// Parse WRITE statement: WRITE(unit, fmt) values
    /// Supports: WRITE(*, *) x, y
    ///           WRITE(10, '(I5)') x
    ///           WRITE(UNIT=10, FMT='(I5)') x
    fn parse_write_statement(&mut self) -> ParseResult<Statement> {
        let location = self.current_location();
        self.expect(&TokenType::Write, "WRITE")?;

        self.expect(&TokenType::LeftParen, "(")?;

        // Parse unit and format specifiers
        let (unit, format) = self.parse_io_control_list()?;

        self.expect(&TokenType::RightParen, ")")?;

        // Parse optional expression list
        let mut values = Vec::new();
        if self.is_expression_start() {
            loop {
                values.push(self.parse_expression()?);
                if self.check(&TokenType::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
        }

        Ok(Statement::Write {
            unit,
            format,
            values,
            location,
        })
    }

    /// Parse READ statement: READ(unit, fmt) variables
    /// Supports: READ(*, *) x, y
    ///           READ(10, '(I5)') x
    fn parse_read_statement(&mut self) -> ParseResult<Statement> {
        let location = self.current_location();
        self.expect(&TokenType::Read, "READ")?;

        self.expect(&TokenType::LeftParen, "(")?;

        // Parse unit and format specifiers
        let (unit, format) = self.parse_io_control_list()?;

        self.expect(&TokenType::RightParen, ")")?;

        // Parse variable list
        let mut variables = Vec::new();
        if self.is_identifier_start() {
            loop {
                variables.push(self.expect_identifier()?);
                if self.check(&TokenType::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
        }

        Ok(Statement::Read {
            unit,
            format,
            variables,
            location,
        })
    }

    /// Parse I/O control list: (unit, fmt) or (UNIT=n, FMT='...')
    fn parse_io_control_list(&mut self) -> ParseResult<(Option<Expr>, Option<FormatSpec>)> {
        let mut unit: Option<Expr> = None;
        let mut format: Option<FormatSpec> = None;

        // Check for keyword style: UNIT=, FMT=
        if self.check(&TokenType::Unit) {
            // UNIT= keyword style
            while !self.check(&TokenType::RightParen) {
                if self.check(&TokenType::Unit) {
                    self.advance();
                    self.expect(&TokenType::Equal, "=")?;
                    if self.check(&TokenType::Star) {
                        self.advance();
                        // UNIT=* means stdout
                        unit = None;
                    } else {
                        unit = Some(self.parse_expression()?);
                    }
                } else if self.check(&TokenType::Format) {
                    self.advance();
                    self.expect(&TokenType::Equal, "=")?;
                    format = Some(self.parse_format_spec()?);
                } else if self.check(&TokenType::Comma) {
                    self.advance();
                } else {
                    // Skip unknown specifiers
                    self.advance();
                }
            }
        } else {
            // Positional style: (unit, fmt)
            // First position: unit (* for console)
            if self.check(&TokenType::Star) {
                self.advance();
                unit = None; // * means stdout/stdin
            } else {
                unit = Some(self.parse_expression()?);
            }

            // Optional second position: format
            if self.check(&TokenType::Comma) {
                self.advance();
                if !self.check(&TokenType::RightParen) {
                    format = Some(self.parse_format_spec()?);
                }
            }
        }

        Ok((unit, format))
    }

    /// Parse format specification: * or '(I5, F10.2)' or label
    fn parse_format_spec(&mut self) -> ParseResult<FormatSpec> {
        if self.check(&TokenType::Star) {
            self.advance();
            Ok(FormatSpec::ListDirected)
        } else if let TokenType::StringLiteral(s) = &self.peek().token_type.clone() {
            let s = s.clone();
            self.advance();
            Ok(FormatSpec::String(s))
        } else if let TokenType::IntegerLiteral(s) = &self.peek().token_type.clone() {
            let label = s.parse::<i64>().map_err(|_| ParseError::InvalidNumber {
                value: s.clone(),
                location: self.current_location(),
            })?;
            self.advance();
            Ok(FormatSpec::Label(label))
        } else {
            Err(ParseError::UnexpectedToken {
                expected: "format specification (*, string, or label)".to_string(),
                found: self.peek().token_type.clone(),
                location: self.current_location(),
            })
        }
    }

    /// Parse OPEN statement: OPEN(UNIT=n, FILE='name', STATUS='OLD', ...)
    fn parse_open_statement(&mut self) -> ParseResult<Statement> {
        let location = self.current_location();
        self.expect(&TokenType::Open, "OPEN")?;

        self.expect(&TokenType::LeftParen, "(")?;

        let mut unit: Option<Expr> = None;
        let mut file: Option<Expr> = None;
        let mut status: Option<String> = None;
        let mut action: Option<String> = None;
        let mut iostat: Option<String> = None;

        // Parse keyword arguments
        loop {
            if self.check(&TokenType::RightParen) {
                break;
            }

            if self.check(&TokenType::Unit) {
                self.advance();
                self.expect(&TokenType::Equal, "=")?;
                unit = Some(self.parse_expression()?);
            } else if self.check(&TokenType::File) {
                self.advance();
                self.expect(&TokenType::Equal, "=")?;
                file = Some(self.parse_expression()?);
            } else if self.check(&TokenType::Status) {
                self.advance();
                self.expect(&TokenType::Equal, "=")?;
                if let TokenType::StringLiteral(s) = &self.peek().token_type.clone() {
                    status = Some(s.clone());
                    self.advance();
                }
            } else if self.check(&TokenType::Action) {
                self.advance();
                self.expect(&TokenType::Equal, "=")?;
                if let TokenType::StringLiteral(s) = &self.peek().token_type.clone() {
                    action = Some(s.clone());
                    self.advance();
                }
            } else if self.check(&TokenType::Iostat) {
                self.advance();
                self.expect(&TokenType::Equal, "=")?;
                iostat = Some(self.expect_identifier()?);
            } else if self.is_expression_start() {
                // First positional arg is unit
                if unit.is_none() {
                    unit = Some(self.parse_expression()?);
                }
            }

            if self.check(&TokenType::Comma) {
                self.advance();
            } else {
                break;
            }
        }

        self.expect(&TokenType::RightParen, ")")?;

        // Unit is required
        let unit = unit.ok_or_else(|| ParseError::UnexpectedToken {
            expected: "unit number".to_string(),
            found: TokenType::RightParen,
            location,
        })?;

        Ok(Statement::Open {
            unit,
            file,
            status,
            action,
            iostat,
            location,
        })
    }

    /// Parse CLOSE statement: CLOSE(UNIT=n) or CLOSE(n)
    fn parse_close_statement(&mut self) -> ParseResult<Statement> {
        let location = self.current_location();
        self.expect(&TokenType::Close, "CLOSE")?;

        self.expect(&TokenType::LeftParen, "(")?;

        let mut unit: Option<Expr> = None;
        let mut iostat: Option<String> = None;

        // Parse arguments
        loop {
            if self.check(&TokenType::RightParen) {
                break;
            }

            if self.check(&TokenType::Unit) {
                self.advance();
                self.expect(&TokenType::Equal, "=")?;
                unit = Some(self.parse_expression()?);
            } else if self.check(&TokenType::Iostat) {
                self.advance();
                self.expect(&TokenType::Equal, "=")?;
                iostat = Some(self.expect_identifier()?);
            } else if self.is_expression_start() {
                // Positional unit arg
                if unit.is_none() {
                    unit = Some(self.parse_expression()?);
                }
            }

            if self.check(&TokenType::Comma) {
                self.advance();
            } else {
                break;
            }
        }

        self.expect(&TokenType::RightParen, ")")?;

        // Unit is required
        let unit = unit.ok_or_else(|| ParseError::UnexpectedToken {
            expected: "unit number".to_string(),
            found: TokenType::RightParen,
            location,
        })?;

        Ok(Statement::Close {
            unit,
            iostat,
            location,
        })
    }

    /// Parse ALLOCATE statement: ALLOCATE(var, STAT=stat)
    fn parse_allocate_statement(&mut self) -> ParseResult<Statement> {
        let location = self.current_location();
        self.expect(&TokenType::Allocate, "ALLOCATE")?;
        self.expect(&TokenType::LeftParen, "(")?;

        // Parse variable name
        let variable = self.expect_identifier()?;

        // Parse optional array dimensions
        let dimensions = if self.check(&TokenType::LeftParen) {
            self.advance();
            let mut dims = Vec::new();
            loop {
                dims.push(self.parse_expression()?);
                if self.check(&TokenType::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
            self.expect(&TokenType::RightParen, ")")?;
            Some(dims)
        } else {
            None
        };

        // Parse optional STAT
        let stat = if self.check(&TokenType::Comma) {
            self.advance();
            if self.check(&TokenType::Stat) {
                self.advance();
                self.expect(&TokenType::Equal, "=")?;
                Some(self.expect_identifier()?)
            } else {
                None
            }
        } else {
            None
        };

        self.expect(&TokenType::RightParen, ")")?;

        Ok(Statement::Allocate {
            variable,
            dimensions,
            stat,
            location,
        })
    }

    /// Parse DEALLOCATE statement: DEALLOCATE(var, STAT=stat)
    fn parse_deallocate_statement(&mut self) -> ParseResult<Statement> {
        let location = self.current_location();
        self.expect(&TokenType::Deallocate, "DEALLOCATE")?;
        self.expect(&TokenType::LeftParen, "(")?;

        // Parse variable name
        let variable = self.expect_identifier()?;

        // Parse optional STAT
        let stat = if self.check(&TokenType::Comma) {
            self.advance();
            if self.check(&TokenType::Stat) {
                self.advance();
                self.expect(&TokenType::Equal, "=")?;
                Some(self.expect_identifier()?)
            } else {
                None
            }
        } else {
            None
        };

        self.expect(&TokenType::RightParen, ")")?;

        Ok(Statement::Deallocate {
            variable,
            stat,
            location,
        })
    }

    /// Parse NULLIFY statement: NULLIFY(ptr1, ptr2, ...)
    fn parse_nullify_statement(&mut self) -> ParseResult<Statement> {
        let location = self.current_location();
        // Consume NULLIFY identifier
        self.advance();
        self.expect(&TokenType::LeftParen, "(")?;

        // Parse list of pointer names
        let mut pointers = Vec::new();
        loop {
            pointers.push(self.expect_identifier()?);
            if self.check(&TokenType::Comma) {
                self.advance();
            } else {
                break;
            }
        }

        self.expect(&TokenType::RightParen, ")")?;

        Ok(Statement::Nullify {
            pointers,
            location,
        })
    }

    /// Check if current token could start an identifier
    fn is_identifier_start(&self) -> bool {
        matches!(
            self.peek().token_type,
            TokenType::Identifier(_)
                | TokenType::Result
                | TokenType::Stat
                | TokenType::Kind
                | TokenType::Len
        )
    }

    /// Parse CALL statement: CALL subroutine(args)
    fn parse_call_statement(&mut self) -> ParseResult<Statement> {
        let location = self.current_location();
        self.expect(&TokenType::Call, "CALL")?;

        let name = self.expect_identifier()?;

        // Parse optional argument list
        let arguments = if self.check(&TokenType::LeftParen) {
            self.advance();
            let args = self.parse_argument_list()?;
            self.expect(&TokenType::RightParen, ")")?;
            args
        } else {
            Vec::new()
        };

        Ok(Statement::Call {
            name,
            arguments,
            location,
        })
    }

    /// Parse RETURN statement: RETURN [expr]
    fn parse_return_statement(&mut self) -> ParseResult<Statement> {
        let location = self.current_location();
        self.expect(&TokenType::Return, "RETURN")?;

        // Check if there's a return value expression
        // A return value is present if the next token looks like the start of an expression
        let value = if self.is_expression_start() {
            Some(self.parse_expression()?)
        } else {
            None
        };

        Ok(Statement::Return { value, location })
    }

    /// Check if current token could start an expression
    fn is_expression_start(&self) -> bool {
        matches!(
            self.peek().token_type,
            TokenType::IntegerLiteral(_)
                | TokenType::RealLiteral(_)
                | TokenType::StringLiteral(_)
                | TokenType::True
                | TokenType::False
                | TokenType::Identifier(_)
                | TokenType::LeftParen
                | TokenType::Plus
                | TokenType::Minus
                | TokenType::Not
                | TokenType::Result
                | TokenType::Stat
                | TokenType::Kind
                | TokenType::Len
        )
    }

    /// Parse a comma-separated argument list
    fn parse_argument_list(&mut self) -> ParseResult<Vec<Expr>> {
        let mut args = Vec::new();

        // Empty argument list
        if self.check(&TokenType::RightParen) {
            return Ok(args);
        }

        loop {
            args.push(self.parse_expression()?);
            if self.check(&TokenType::Comma) {
                self.advance();
            } else {
                break;
            }
        }

        Ok(args)
    }

    /// Parse array subscript list, which may contain slices
    /// Returns (subscripts, has_slices) where has_slices indicates if any slice notation was used
    fn parse_subscript_list(&mut self) -> ParseResult<(Vec<ArraySubscript>, bool)> {
        let mut subscripts = Vec::new();
        let mut has_slices = false;

        // Empty subscript list
        if self.check(&TokenType::RightParen) {
            return Ok((subscripts, false));
        }

        loop {
            let subscript = self.parse_subscript()?;
            if matches!(subscript, ArraySubscript::Slice { .. }) {
                has_slices = true;
            }
            subscripts.push(subscript);

            if self.check(&TokenType::Comma) {
                self.advance();
            } else {
                break;
            }
        }

        Ok((subscripts, has_slices))
    }

    /// Parse a single subscript (either an index or a slice)
    fn parse_subscript(&mut self) -> ParseResult<ArraySubscript> {
        // Check if this starts with :: (DoubleColon) - e.g., arr(::2)
        if self.check(&TokenType::DoubleColon) {
            // This is a slice with no start and no end, just potentially a step
            self.advance(); // consume ::
            let step = if self.check(&TokenType::Comma) || self.check(&TokenType::RightParen) {
                None
            } else {
                Some(Box::new(self.parse_expression()?))
            };
            return Ok(ArraySubscript::Slice { start: None, end: None, step });
        }

        // Check if this starts with a single colon (e.g., `:5` or `:`)
        if self.check(&TokenType::Colon) {
            // This is a slice starting with no lower bound
            return self.parse_slice(None);
        }

        // Parse the first expression
        let first_expr = self.parse_expression()?;

        // Check if followed by :: (DoubleColon) - e.g., arr(2::2)
        if self.check(&TokenType::DoubleColon) {
            self.advance(); // consume ::
            let step = if self.check(&TokenType::Comma) || self.check(&TokenType::RightParen) {
                None
            } else {
                Some(Box::new(self.parse_expression()?))
            };
            return Ok(ArraySubscript::Slice {
                start: Some(Box::new(first_expr)),
                end: None,
                step,
            });
        }

        // Check if followed by single colon (making it a slice)
        if self.check(&TokenType::Colon) {
            // This is a slice: first_expr:...
            return self.parse_slice(Some(Box::new(first_expr)));
        }

        // Just a simple index
        Ok(ArraySubscript::Index(first_expr))
    }

    /// Parse remainder of a slice after we've seen start (or know start is None)
    /// Handles: start:end, start:end:step, start:, start::step, :end, :end:step, :, ::step
    fn parse_slice(&mut self, start: Option<Box<Expr>>) -> ParseResult<ArraySubscript> {
        // Consume the first colon
        self.expect(&TokenType::Colon, ":")?;

        // Check what follows the colon
        let end = if self.check(&TokenType::Colon) || self.check(&TokenType::DoubleColon) || self.check(&TokenType::Comma) || self.check(&TokenType::RightParen) {
            // No end value: `start:` or `start::step` or `:` or `::step`
            None
        } else {
            // There's an end value
            Some(Box::new(self.parse_expression()?))
        };

        // Check for step (another colon or double colon)
        let step = if self.check(&TokenType::Colon) {
            self.advance();
            if self.check(&TokenType::Comma) || self.check(&TokenType::RightParen) {
                // No step value after second colon (weird but valid: `1:5:`)
                None
            } else {
                Some(Box::new(self.parse_expression()?))
            }
        } else if self.check(&TokenType::DoubleColon) {
            // Rare case: `:end::step` or similar, where :: appears
            self.advance();
            if self.check(&TokenType::Comma) || self.check(&TokenType::RightParen) {
                None
            } else {
                Some(Box::new(self.parse_expression()?))
            }
        } else {
            None
        };

        Ok(ArraySubscript::Slice { start, end, step })
    }

    /// Parse array constructor items (values and implied-DO loops)
    fn parse_array_constructor_items(&mut self) -> ParseResult<Vec<ArrayConstructorItem>> {
        let mut items = Vec::new();

        // Empty constructor
        if self.check(&TokenType::RightBracket) {
            return Ok(items);
        }

        loop {
            let item = self.parse_array_constructor_item()?;
            items.push(item);

            if self.check(&TokenType::Comma) {
                self.advance();
            } else {
                break;
            }
        }

        Ok(items)
    }

    /// Parse a single array constructor item (value or implied-DO)
    fn parse_array_constructor_item(&mut self) -> ParseResult<ArrayConstructorItem> {
        // Check for implied-DO: (expr, var=start,end[,step])
        if self.check(&TokenType::LeftParen) {
            // Could be an implied-DO or just a parenthesized expression
            // Look ahead to detect the pattern: (expr, identifier =
            let checkpoint = self.position;

            self.advance(); // consume (
            let first_expr = self.parse_expression()?;

            if self.check(&TokenType::Comma) {
                // This might be an implied-DO
                self.advance();
                if let TokenType::Identifier(var_name) = &self.peek().token_type.clone() {
                    let var = var_name.clone();
                    self.advance();

                    if self.check(&TokenType::Equal) {
                        // This is an implied-DO
                        self.advance();
                        let start = self.parse_expression()?;
                        self.expect(&TokenType::Comma, ",")?;
                        let end = self.parse_expression()?;

                        let step = if self.check(&TokenType::Comma) {
                            self.advance();
                            Some(Box::new(self.parse_expression()?))
                        } else {
                            None
                        };

                        self.expect(&TokenType::RightParen, ")")?;

                        return Ok(ArrayConstructorItem::ImpliedDo {
                            expr: Box::new(first_expr),
                            var,
                            start: Box::new(start),
                            end: Box::new(end),
                            step,
                        });
                    }
                }
                // Not an implied-DO, restore position and parse as value
                self.position = checkpoint;
            } else {
                // Just a parenthesized expression, restore and parse as value
                self.position = checkpoint;
            }
        }

        // Parse as a simple value
        let expr = self.parse_expression()?;
        Ok(ArrayConstructorItem::Value(expr))
    }

    /// Parse IF statement
    fn parse_if_statement(&mut self) -> ParseResult<Statement> {
        let location = self.current_location();
        self.expect(&TokenType::If, "IF")?;

        // Parse condition (in parentheses or not, Fortran is flexible)
        let condition = if self.check(&TokenType::LeftParen) {
            self.advance();
            let cond = self.parse_expression()?;
            self.expect(&TokenType::RightParen, ")")?;
            cond
        } else {
            self.parse_expression()?
        };

        // Check if this is a single-line IF statement (no THEN)
        if !self.check(&TokenType::Then) {
            // Single-line IF: IF (condition) statement
            let statement = self.parse_statement()?;
            return Ok(Statement::If {
                condition,
                then_block: vec![statement],
                else_if_blocks: Vec::new(),
                else_block: None,
                location,
            });
        }

        self.expect(&TokenType::Then, "THEN")?;

        // Parse THEN block
        let mut then_block = Vec::new();
        loop {
            // Stop at: EOF, ELSEIF (single token), ELSE IF (two tokens),
            // ELSE (not followed by IF), or END IF
            if self.is_at_end()
                || self.check(&TokenType::ElseIf)
                || self.is_else_if()
                || (self.check(&TokenType::Else) && !self.is_else_if())
                || self.is_end_if()
            {
                break;
            }
            then_block.push(self.parse_statement()?);
        }

        // Parse ELSE IF blocks
        let mut else_if_blocks = Vec::new();
        while self.check(&TokenType::ElseIf) || self.is_else_if() {
            // Handle both "ELSEIF" (single token) and "ELSE IF" (two tokens)
            if self.check(&TokenType::ElseIf) {
                self.advance();
            } else {
                // Must be ELSE followed by IF
                self.advance(); // consume ELSE
                self.advance(); // consume IF
            }

            let elif_condition = if self.check(&TokenType::LeftParen) {
                self.advance();
                let cond = self.parse_expression()?;
                self.expect(&TokenType::RightParen, ")")?;
                cond
            } else {
                self.parse_expression()?
            };

            self.expect(&TokenType::Then, "THEN")?;

            let mut elif_block = Vec::new();
            while !self.is_at_end()
                && !self.check(&TokenType::ElseIf)
                && !self.is_else_if()
                && !(self.check(&TokenType::Else) && !self.is_else_if())
                && !self.is_end_if()
            {
                elif_block.push(self.parse_statement()?);
            }

            else_if_blocks.push((elif_condition, elif_block));
        }

        // Parse ELSE block (but not ELSE IF)
        let else_block = if self.check(&TokenType::Else) && !self.is_else_if() {
            self.advance();
            let mut block = Vec::new();
            while !self.is_at_end() && !self.is_end_if() {
                block.push(self.parse_statement()?);
            }
            Some(block)
        } else {
            None
        };

        // Expect END IF
        self.expect(&TokenType::End, "END")?;
        self.expect(&TokenType::If, "IF")?;

        Ok(Statement::If {
            condition,
            then_block,
            else_if_blocks,
            else_block,
            location,
        })
    }

    /// Parse DO statement (loop, while, concurrent, or infinite)
    fn parse_do_statement(&mut self) -> ParseResult<Statement> {
        let location = self.current_location();
        self.expect(&TokenType::Do, "DO")?;

        // Check what kind of DO loop this is

        // DO CONCURRENT
        if self.check(&TokenType::Concurrent) {
            return self.parse_do_concurrent(location);
        }

        // DO WHILE
        if self.check(&TokenType::While) {
            self.advance();
            let condition = if self.check(&TokenType::LeftParen) {
                self.advance();
                let cond = self.parse_expression()?;
                self.expect(&TokenType::RightParen, ")")?;
                cond
            } else {
                self.parse_expression()?
            };

            let mut body = Vec::new();
            while !self.is_at_end() && !self.is_end_do() {
                body.push(self.parse_statement()?);
            }

            self.expect(&TokenType::End, "END")?;
            self.expect(&TokenType::Do, "DO")?;

            return Ok(Statement::DoWhile {
                condition,
                body,
                location,
            });
        }

        // Check if this is a counted DO loop (has loop variable)
        let is_counted_loop = matches!(self.peek().token_type, TokenType::Identifier(_));

        if is_counted_loop {
            // DO i = start, end [, step]
            let variable = self.expect_identifier()?;
            self.expect(&TokenType::Equal, "=")?;

            let start = self.parse_expression()?;
            self.expect(&TokenType::Comma, ",")?;

            let end = self.parse_expression()?;

            let step = if self.check(&TokenType::Comma) {
                self.advance();
                Some(self.parse_expression()?)
            } else {
                None
            };

            let mut body = Vec::new();
            while !self.is_at_end() && !self.is_end_do() {
                body.push(self.parse_statement()?);
            }

            self.expect(&TokenType::End, "END")?;
            self.expect(&TokenType::Do, "DO")?;

            return Ok(Statement::DoLoop {
                variable,
                start,
                end,
                step,
                body,
                location,
            });
        }

        // Infinite DO loop (DO ... END DO)
        let mut body = Vec::new();
        while !self.is_at_end() && !self.check(&TokenType::End) {
            body.push(self.parse_statement()?);
        }

        self.expect(&TokenType::End, "END")?;
        self.expect(&TokenType::Do, "DO")?;

        Ok(Statement::DoInfinite { body, location })
    }

    /// Parse DO CONCURRENT statement
    /// Syntax: DO CONCURRENT (i = start:end, j = start:end) [locality-specs]
    fn parse_do_concurrent(&mut self, location: SourceLocation) -> ParseResult<Statement> {
        self.expect(&TokenType::Concurrent, "CONCURRENT")?;
        self.expect(&TokenType::LeftParen, "(")?;

        // Parse concurrent controls
        let mut controls = Vec::new();
        loop {
            let variable = self.expect_identifier()?;
            self.expect(&TokenType::Equal, "=")?;

            let start = self.parse_expression()?;
            self.expect(&TokenType::Colon, ":")?;
            let end = self.parse_expression()?;

            // Optional step (start:end:step)
            let step = if self.check(&TokenType::Colon) {
                self.advance();
                Some(self.parse_expression()?)
            } else {
                None
            };

            controls.push(ConcurrentControl {
                variable,
                start,
                end,
                step,
            });

            if self.check(&TokenType::Comma) {
                self.advance();
            } else {
                break;
            }
        }
        self.expect(&TokenType::RightParen, ")")?;

        // Parse optional locality specifications
        let mut locality = Vec::new();
        while self.check_locality_keyword() {
            locality.push(self.parse_locality_spec()?);
        }

        // Parse body
        let mut body = Vec::new();
        while !self.is_at_end() && !self.is_end_do() {
            body.push(self.parse_statement()?);
        }

        self.expect(&TokenType::End, "END")?;
        self.expect(&TokenType::Do, "DO")?;

        Ok(Statement::DoConcurrent {
            controls,
            locality,
            body,
            location,
        })
    }

    /// Check if current token is a locality keyword
    fn check_locality_keyword(&self) -> bool {
        matches!(
            self.peek().token_type,
            TokenType::Identifier(ref s) if s.to_uppercase() == "LOCAL"
                || s.to_uppercase() == "LOCAL_INIT"
                || s.to_uppercase() == "SHARED"
                || s.to_uppercase() == "DEFAULT"
        )
    }

    /// Parse a locality specification
    fn parse_locality_spec(&mut self) -> ParseResult<LocalitySpec> {
        let keyword = self.expect_identifier()?.to_uppercase();

        match keyword.as_str() {
            "LOCAL" => {
                self.expect(&TokenType::LeftParen, "(")?;
                let vars = self.parse_identifier_list()?;
                self.expect(&TokenType::RightParen, ")")?;
                Ok(LocalitySpec::Local(vars))
            }
            "LOCAL_INIT" => {
                self.expect(&TokenType::LeftParen, "(")?;
                let vars = self.parse_identifier_list()?;
                self.expect(&TokenType::RightParen, ")")?;
                Ok(LocalitySpec::LocalInit(vars))
            }
            "SHARED" => {
                self.expect(&TokenType::LeftParen, "(")?;
                let vars = self.parse_identifier_list()?;
                self.expect(&TokenType::RightParen, ")")?;
                Ok(LocalitySpec::Shared(vars))
            }
            "DEFAULT" => {
                self.expect(&TokenType::LeftParen, "(")?;
                let mode = self.expect_identifier()?.to_uppercase();
                self.expect(&TokenType::RightParen, ")")?;
                if mode == "NONE" {
                    Ok(LocalitySpec::DefaultNone)
                } else {
                    Err(ParseError::UnexpectedToken {
                        expected: "NONE".to_string(),
                        found: TokenType::Identifier(mode),
                        location: self.current_location(),
                    })
                }
            }
            _ => Err(ParseError::UnexpectedToken {
                expected: "locality keyword (LOCAL, SHARED, DEFAULT)".to_string(),
                found: TokenType::Identifier(keyword),
                location: self.current_location(),
            }),
        }
    }

    /// Parse a comma-separated list of identifiers
    fn parse_identifier_list(&mut self) -> ParseResult<Vec<String>> {
        let mut names = Vec::new();
        names.push(self.expect_identifier()?);
        while self.check(&TokenType::Comma) {
            self.advance();
            names.push(self.expect_identifier()?);
        }
        Ok(names)
    }

    /// Parse SYNC statement (SYNC ALL or SYNC IMAGES)
    fn parse_sync_statement(&mut self) -> ParseResult<Statement> {
        let location = self.current_location();
        self.expect(&TokenType::Sync, "SYNC")?;

        if self.check(&TokenType::All) {
            self.advance();
            Ok(Statement::SyncAll { location })
        } else if self.check(&TokenType::Images) {
            self.advance();
            // Optional image list: SYNC IMAGES (1, 2, 3) or SYNC IMAGES (*)
            let images = if self.check(&TokenType::LeftParen) {
                self.advance();
                if self.check(&TokenType::Star) {
                    self.advance();
                    self.expect(&TokenType::RightParen, ")")?;
                    None // * means all images
                } else {
                    let mut img_list = Vec::new();
                    img_list.push(self.parse_expression()?);
                    while self.check(&TokenType::Comma) {
                        self.advance();
                        img_list.push(self.parse_expression()?);
                    }
                    self.expect(&TokenType::RightParen, ")")?;
                    Some(img_list)
                }
            } else {
                None
            };
            Ok(Statement::SyncImages { images, location })
        } else {
            Err(ParseError::UnexpectedToken {
                expected: "ALL or IMAGES".to_string(),
                found: self.peek().token_type.clone(),
                location,
            })
        }
    }

    /// Parse CRITICAL section
    fn parse_critical_section(&mut self) -> ParseResult<Statement> {
        let location = self.current_location();
        self.expect(&TokenType::Critical, "CRITICAL")?;

        // Parse body until END CRITICAL
        let mut body = Vec::new();
        while !self.is_at_end() && !self.check(&TokenType::End) {
            body.push(self.parse_statement()?);
        }

        self.expect(&TokenType::End, "END")?;
        self.expect(&TokenType::Critical, "CRITICAL")?;

        Ok(Statement::Critical { body, location })
    }

    /// Parse WHERE statement/construct
    /// Single-line form: WHERE (mask) assignment
    /// Block form: WHERE (mask) ... [ELSEWHERE ...] END WHERE
    fn parse_where_statement(&mut self) -> ParseResult<Statement> {
        let location = self.current_location();
        self.expect(&TokenType::Where, "WHERE")?;

        // Parse mask expression in parentheses
        self.expect(&TokenType::LeftParen, "(")?;
        let mask = self.parse_expression()?;

        // Save line of ) to detect single-line WHERE
        let rparen_line = self.peek().location.line;
        self.expect(&TokenType::RightParen, ")")?;

        // Detect single-line WHERE: statement is on same line as WHERE (mask)
        let next_token_line = self.peek().location.line;
        let is_single_line = next_token_line == rparen_line
            && !self.check(&TokenType::End)
            && !self.check(&TokenType::Elsewhere)
            && !self.is_at_end();

        if is_single_line {
            // Single-line WHERE: parse one statement and return (no END WHERE)
            let stmt = self.parse_statement()?;
            return Ok(Statement::Where {
                mask,
                body: vec![stmt],
                elsewhere: None,
                location,
            });
        }

        // Block WHERE: parse body statements until END or ELSEWHERE
        let mut body = Vec::new();
        while !self.is_at_end() && !self.check(&TokenType::End) && !self.check(&TokenType::Elsewhere) {
            body.push(self.parse_statement()?);
        }

        // Check for ELSEWHERE
        let elsewhere = if self.check(&TokenType::Elsewhere) {
            self.advance();
            let mut else_body = Vec::new();
            while !self.is_at_end() && !self.check(&TokenType::End) {
                else_body.push(self.parse_statement()?);
            }
            Some(else_body)
        } else {
            None
        };

        // Expect END WHERE for block form
        self.expect(&TokenType::End, "END")?;
        self.expect(&TokenType::Where, "WHERE")?;

        Ok(Statement::Where {
            mask,
            body,
            elsewhere,
            location,
        })
    }

    /// Parse FORALL statement/construct
    fn parse_forall_statement(&mut self) -> ParseResult<Statement> {
        let location = self.current_location();
        self.expect(&TokenType::Forall, "FORALL")?;

        // Parse index specifications in parentheses
        self.expect(&TokenType::LeftParen, "(")?;

        let mut indices = Vec::new();
        let mut mask = None;

        // Parse index specs: var = start:end[:step]
        loop {
            // Check if this is a mask expression (no '=' follows identifier)
            if let TokenType::Identifier(var_name) = &self.peek().token_type.clone() {
                let var = var_name.clone();
                self.advance();

                if self.check(&TokenType::Equal) {
                    // This is an index spec
                    self.advance();
                    let start = self.parse_expression()?;
                    self.expect(&TokenType::Colon, ":")?;
                    let end = self.parse_expression()?;

                    let step = if self.check(&TokenType::Colon) {
                        self.advance();
                        Some(self.parse_expression()?)
                    } else {
                        None
                    };

                    indices.push(ForallIndex { var, start, end, step });
                } else {
                    // This is a mask expression - backtrack and parse
                    // Actually, the mask comes after a comma at the end
                    // Let's handle this differently - restore var and parse as mask
                    // For now, assume mask is at the end after all indices
                    return Err(ParseError::UnexpectedToken {
                        expected: "= after index variable".to_string(),
                        found: self.peek().token_type.clone(),
                        location: self.current_location(),
                    });
                }
            } else {
                break;
            }

            if self.check(&TokenType::Comma) {
                self.advance();
                // Check if next is the mask (not an identifier followed by =)
                // For simplicity, if next token after comma is not identifier=, it's a mask
                if let TokenType::Identifier(_) = &self.peek().token_type {
                    // Look ahead to see if there's an =
                    // For now, continue parsing indices
                    continue;
                } else {
                    // Parse mask expression
                    mask = Some(self.parse_expression()?);
                    break;
                }
            } else {
                break;
            }
        }

        self.expect(&TokenType::RightParen, ")")?;

        // Check if single-line or multi-line FORALL
        if !self.is_at_end() && !self.check(&TokenType::End) {
            if let TokenType::Identifier(_) = self.peek().token_type {
                // Single-line FORALL
                let stmt = self.parse_statement()?;
                return Ok(Statement::Forall {
                    indices,
                    mask,
                    body: vec![stmt],
                    location,
                });
            }
        }

        // Multi-line FORALL construct
        let mut body = Vec::new();
        while !self.is_at_end() && !self.check(&TokenType::End) {
            body.push(self.parse_statement()?);
        }

        // Expect END FORALL
        self.expect(&TokenType::End, "END")?;
        self.expect(&TokenType::Forall, "FORALL")?;

        Ok(Statement::Forall {
            indices,
            mask,
            body,
            location,
        })
    }

    /// Parse ASSOCIATE construct
    /// ASSOCIATE (alias => expr, ...) ... END ASSOCIATE
    fn parse_associate_construct(&mut self) -> ParseResult<Statement> {
        let location = self.current_location();
        self.expect(&TokenType::Associate, "ASSOCIATE")?;
        self.expect(&TokenType::LeftParen, "(")?;

        // Parse associations: alias => expr
        let mut associations = Vec::new();
        loop {
            // Parse alias name
            let alias = self.expect_identifier()?;

            // Expect => (arrow operator)
            self.expect(&TokenType::Arrow, "=>")?;

            // Parse target expression
            let target = self.parse_expression()?;

            associations.push((alias, target));

            if self.check(&TokenType::Comma) {
                self.advance();
            } else {
                break;
            }
        }

        self.expect(&TokenType::RightParen, ")")?;

        // Parse body statements
        let mut body = Vec::new();
        while !self.is_at_end() && !self.check(&TokenType::End) {
            body.push(self.parse_statement()?);
        }

        // Expect END ASSOCIATE
        self.expect(&TokenType::End, "END")?;
        self.expect(&TokenType::Associate, "ASSOCIATE")?;

        Ok(Statement::Associate {
            associations,
            body,
            location,
        })
    }

    /// Parse BLOCK construct
    /// BLOCK [declarations] ... END BLOCK
    fn parse_block_construct(&mut self) -> ParseResult<Statement> {
        let location = self.current_location();
        self.expect(&TokenType::Block, "BLOCK")?;

        // Parse declarations and body statements
        let mut declarations = Vec::new();
        let mut body = Vec::new();
        let mut in_declarations = true;

        while !self.is_at_end() && !self.check(&TokenType::End) {
            // Check if this is a declaration (type declaration, etc.)
            if in_declarations && self.is_declaration_start() {
                declarations.push(self.parse_declaration()?);
            } else {
                in_declarations = false;
                body.push(self.parse_statement()?);
            }
        }

        // Expect END BLOCK
        self.expect(&TokenType::End, "END")?;
        self.expect(&TokenType::Block, "BLOCK")?;

        Ok(Statement::Block {
            declarations,
            body,
            location,
        })
    }

    /// Parse SELECT CASE statement
    fn parse_select_case(&mut self) -> ParseResult<Statement> {
        let location = self.current_location();
        self.expect(&TokenType::Select, "SELECT")?;
        self.expect(&TokenType::Case, "CASE")?;

        // Parse selector expression
        let selector = if self.check(&TokenType::LeftParen) {
            self.advance();
            let sel = self.parse_expression()?;
            self.expect(&TokenType::RightParen, ")")?;
            sel
        } else {
            self.parse_expression()?
        };

        let mut cases = Vec::new();
        let mut default = None;

        // Parse case clauses
        while self.check(&TokenType::Case) {
            self.advance();

            // Check for DEFAULT
            if self.check(&TokenType::Default) {
                self.advance();
                let mut body = Vec::new();
                while !self.is_at_end() && !self.check(&TokenType::Case) && !self.is_end_select() {
                    body.push(self.parse_statement()?);
                }
                default = Some(body);
                continue;
            }

            // Parse case selector
            self.expect(&TokenType::LeftParen, "(")?;

            let case_selector = if self.check(&TokenType::Colon) {
                // Range with no start (:end)
                self.advance();
                let end = self.parse_expression()?;
                let start = Expr::IntegerLiteral(std::i64::MIN, location);
                CaseSelector::Range(start, end)
            } else {
                let first = self.parse_expression()?;

                if self.check(&TokenType::Colon) {
                    // Range (start:end)
                    self.advance();
                    let end = if self.check(&TokenType::RightParen) {
                        Expr::IntegerLiteral(std::i64::MAX, location)
                    } else {
                        self.parse_expression()?
                    };
                    CaseSelector::Range(first, end)
                } else if self.check(&TokenType::Comma) {
                    // Value list (val1, val2, ...)
                    let mut values = vec![first];
                    while self.check(&TokenType::Comma) {
                        self.advance();
                        if self.check(&TokenType::RightParen) {
                            break;
                        }
                        values.push(self.parse_expression()?);
                    }
                    CaseSelector::Values(values)
                } else {
                    // Single value
                    CaseSelector::Value(first)
                }
            };

            self.expect(&TokenType::RightParen, ")")?;

            // Parse case body
            let mut body = Vec::new();
            while !self.is_at_end() && !self.check(&TokenType::Case) && !self.is_end_select() {
                body.push(self.parse_statement()?);
            }

            cases.push(CaseClause {
                selector: case_selector,
                body,
                location,
            });
        }

        // Expect END SELECT
        self.expect(&TokenType::End, "END")?;
        self.expect(&TokenType::Select, "SELECT")?;

        Ok(Statement::SelectCase {
            selector,
            cases,
            default,
            location,
        })
    }

    /// Parse an expression (entry point for expression parsing)
    fn parse_expression(&mut self) -> ParseResult<Expr> {
        self.parse_logical_or()
    }

    /// Parse logical OR expression (.OR.)
    fn parse_logical_or(&mut self) -> ParseResult<Expr> {
        let mut left = self.parse_logical_and()?;

        while self.check(&TokenType::Or) {
            let location = self.current_location();
            self.advance();
            let right = self.parse_logical_and()?;
            left = Expr::BinaryOp {
                op: BinaryOperator::Or,
                left: Box::new(left),
                right: Box::new(right),
                location,
            };
        }

        Ok(left)
    }

    /// Parse logical AND expression (.AND.)
    fn parse_logical_and(&mut self) -> ParseResult<Expr> {
        let mut left = self.parse_relational()?;

        while self.check(&TokenType::And) {
            let location = self.current_location();
            self.advance();
            let right = self.parse_relational()?;
            left = Expr::BinaryOp {
                op: BinaryOperator::And,
                left: Box::new(left),
                right: Box::new(right),
                location,
            };
        }

        Ok(left)
    }

    /// Parse relational expression (==, /=, <, <=, >, >=)
    fn parse_relational(&mut self) -> ParseResult<Expr> {
        let mut left = self.parse_additive()?;

        if let Some(op) = self.match_relational_op() {
            let location = self.current_location();
            self.advance();
            let right = self.parse_additive()?;
            left = Expr::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
                location,
            };
        }

        Ok(left)
    }

    /// Parse additive expression (+, -)
    fn parse_additive(&mut self) -> ParseResult<Expr> {
        let mut left = self.parse_multiplicative()?;

        while let Some(op) = self.match_additive_op() {
            let location = self.current_location();
            self.advance();
            let right = self.parse_multiplicative()?;
            left = Expr::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
                location,
            };
        }

        Ok(left)
    }

    /// Parse multiplicative expression (*, /)
    fn parse_multiplicative(&mut self) -> ParseResult<Expr> {
        let mut left = self.parse_power()?;

        while let Some(op) = self.match_multiplicative_op() {
            let location = self.current_location();
            self.advance();
            let right = self.parse_power()?;
            left = Expr::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
                location,
            };
        }

        Ok(left)
    }

    /// Parse power expression (**) - right associative
    fn parse_power(&mut self) -> ParseResult<Expr> {
        let mut left = self.parse_unary()?;

        if self.check(&TokenType::Power) {
            let location = self.current_location();
            self.advance();
            // Right associative: 2**3**4 = 2**(3**4)
            let right = self.parse_power()?;
            left = Expr::BinaryOp {
                op: BinaryOperator::Power,
                left: Box::new(left),
                right: Box::new(right),
                location,
            };
        }

        Ok(left)
    }

    /// Parse unary expression (+, -, .NOT.)
    fn parse_unary(&mut self) -> ParseResult<Expr> {
        let location = self.current_location();

        if self.check(&TokenType::Plus) {
            self.advance();
            let operand = self.parse_unary()?;
            return Ok(Expr::UnaryOp {
                op: UnaryOperator::Plus,
                operand: Box::new(operand),
                location,
            });
        }

        if self.check(&TokenType::Minus) {
            self.advance();
            let operand = self.parse_unary()?;
            return Ok(Expr::UnaryOp {
                op: UnaryOperator::Minus,
                operand: Box::new(operand),
                location,
            });
        }

        if self.check(&TokenType::Not) {
            self.advance();
            let operand = self.parse_unary()?;
            return Ok(Expr::UnaryOp {
                op: UnaryOperator::Not,
                operand: Box::new(operand),
                location,
            });
        }

        self.parse_primary()
    }

    /// Parse primary expression (literals, identifiers, parenthesized)
    fn parse_primary(&mut self) -> ParseResult<Expr> {
        let location = self.current_location();
        let token = self.peek().clone();

        match &token.token_type {
            // Integer literal
            TokenType::IntegerLiteral(s) => {
                self.advance();
                let value = s.parse::<i64>().map_err(|_| ParseError::InvalidNumber {
                    value: s.clone(),
                    location,
                })?;
                Ok(Expr::IntegerLiteral(value, location))
            }

            // Real literal
            TokenType::RealLiteral(s) => {
                self.advance();
                let value = s.parse::<f64>().map_err(|_| ParseError::InvalidNumber {
                    value: s.clone(),
                    location,
                })?;
                Ok(Expr::RealLiteral(value, location))
            }

            // String literal
            TokenType::StringLiteral(s) => {
                self.advance();
                Ok(Expr::StringLiteral(s.clone(), location))
            }

            // Logical literals
            TokenType::True => {
                self.advance();
                Ok(Expr::LogicalLiteral(true, location))
            }
            TokenType::False => {
                self.advance();
                Ok(Expr::LogicalLiteral(false, location))
            }

            // Identifier (or function call / array access if followed by parentheses)
            TokenType::Identifier(name) => {
                let name = name.clone();
                self.advance();

                // Check if this is a function call / array access / array section
                let mut expr = if self.check(&TokenType::LeftParen) {
                    self.advance();
                    let (subscripts, has_slices) = self.parse_subscript_list()?;
                    self.expect(&TokenType::RightParen, ")")?;

                    if has_slices {
                        // This is an array section (slice notation used)
                        Expr::ArraySection {
                            name,
                            subscripts,
                            location,
                        }
                    } else {
                        // No slices - treat as function call (could also be array element access)
                        // Extract expressions from Index variants
                        let arguments: Vec<Expr> = subscripts.into_iter().map(|s| {
                            match s {
                                ArraySubscript::Index(e) => e,
                                ArraySubscript::Slice { .. } => unreachable!("has_slices was false"),
                            }
                        }).collect();
                        Expr::FunctionCall {
                            name,
                            arguments,
                            location,
                        }
                    }
                } else {
                    Expr::Identifier(name, location)
                };

                // Check for component access (obj%component, can be chained)
                // Also check for method calls (obj%method(args))
                while self.check(&TokenType::Percent) {
                    self.advance();
                    let comp_location = self.current_location();
                    let component = self.expect_identifier()?;

                    // Check if this is a method call (component followed by parentheses)
                    if self.check(&TokenType::LeftParen) {
                        self.advance();
                        let arguments = self.parse_argument_list()?;
                        self.expect(&TokenType::RightParen, ")")?;
                        expr = Expr::MethodCall {
                            object: Box::new(expr),
                            method_name: component,
                            arguments,
                            location: comp_location,
                        };
                    } else {
                        expr = Expr::ComponentAccess {
                            object: Box::new(expr),
                            component,
                            location: comp_location,
                        };
                    }
                }

                Ok(expr)
            }

            // Keywords that can be used as identifiers or intrinsic function calls
            TokenType::Result | TokenType::Stat | TokenType::Kind | TokenType::Len
            | TokenType::Present => {
                let name = match &token.token_type {
                    TokenType::Result => "RESULT",
                    TokenType::Stat => "STAT",
                    TokenType::Kind => "KIND",
                    TokenType::Len => "LEN",
                    TokenType::Present => "PRESENT",
                    _ => unreachable!(),
                };
                self.advance();

                // Check if this is a function call
                if self.check(&TokenType::LeftParen) {
                    self.advance();
                    let arguments = self.parse_argument_list()?;
                    self.expect(&TokenType::RightParen, ")")?;
                    Ok(Expr::FunctionCall {
                        name: name.to_string(),
                        arguments,
                        location,
                    })
                } else {
                    Ok(Expr::Identifier(name.to_string(), location))
                }
            }

            // Parenthesized expression
            TokenType::LeftParen => {
                self.advance();
                let expr = self.parse_expression()?;
                self.expect(&TokenType::RightParen, ")")?;
                Ok(Expr::Parenthesized(Box::new(expr), location))
            }

            // Array constructor: [1, 2, 3] or [(i, i=1,10)]
            TokenType::LeftBracket => {
                self.advance();
                let elements = self.parse_array_constructor_items()?;
                self.expect(&TokenType::RightBracket, "]")?;
                Ok(Expr::ArrayConstructor { elements, location })
            }

            _ => Err(ParseError::UnexpectedToken {
                expected: "expression".to_string(),
                found: token.token_type,
                location,
            }),
        }
    }

    // Helper methods

    fn match_relational_op(&self) -> Option<BinaryOperator> {
        match &self.peek().token_type {
            TokenType::EqualEqual | TokenType::Eq => Some(BinaryOperator::Equal),
            TokenType::NotEqual | TokenType::Ne => Some(BinaryOperator::NotEqual),
            TokenType::Less | TokenType::Lt => Some(BinaryOperator::Less),
            TokenType::LessEqual | TokenType::Le => Some(BinaryOperator::LessEqual),
            TokenType::Greater | TokenType::Gt => Some(BinaryOperator::Greater),
            TokenType::GreaterEqual | TokenType::Ge => Some(BinaryOperator::GreaterEqual),
            _ => None,
        }
    }

    fn match_additive_op(&self) -> Option<BinaryOperator> {
        match &self.peek().token_type {
            TokenType::Plus => Some(BinaryOperator::Add),
            TokenType::Minus => Some(BinaryOperator::Subtract),
            _ => None,
        }
    }

    fn match_multiplicative_op(&self) -> Option<BinaryOperator> {
        match &self.peek().token_type {
            TokenType::Star => Some(BinaryOperator::Multiply),
            TokenType::Slash => Some(BinaryOperator::Divide),
            _ => None,
        }
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.position]
    }

    fn peek_next(&self) -> &Token {
        if self.position + 1 < self.tokens.len() {
            &self.tokens[self.position + 1]
        } else {
            &self.tokens[self.tokens.len() - 1] // Return EOF
        }
    }

    fn advance(&mut self) -> Token {
        if !self.is_at_end() {
            self.position += 1;
        }
        self.tokens[self.position - 1].clone()
    }

    fn check(&self, token_type: &TokenType) -> bool {
        if self.is_at_end() {
            return false;
        }
        std::mem::discriminant(&self.peek().token_type) == std::mem::discriminant(token_type)
    }

    fn is_at_end(&self) -> bool {
        matches!(self.peek().token_type, TokenType::Eof)
    }

    fn current_location(&self) -> SourceLocation {
        self.peek().location
    }

    /// Check if we're at "ELSE IF" (two tokens)
    fn is_else_if(&self) -> bool {
        if !self.check(&TokenType::Else) {
            return false;
        }
        // Peek ahead to see if next token is IF
        if self.position + 1 < self.tokens.len() {
            matches!(self.tokens[self.position + 1].token_type, TokenType::If)
        } else {
            false
        }
    }

    /// Check if we're at "END IF"
    fn is_end_if(&self) -> bool {
        if !self.check(&TokenType::End) {
            return false;
        }
        // Peek ahead to see if next token is IF
        if self.position + 1 < self.tokens.len() {
            matches!(self.tokens[self.position + 1].token_type, TokenType::If)
        } else {
            false
        }
    }

    /// Check if we're at "END DO"
    fn is_end_do(&self) -> bool {
        if !self.check(&TokenType::End) {
            return false;
        }
        if self.position + 1 < self.tokens.len() {
            matches!(self.tokens[self.position + 1].token_type, TokenType::Do)
        } else {
            false
        }
    }

    /// Check if we're at "END SELECT"
    fn is_end_select(&self) -> bool {
        if !self.check(&TokenType::End) {
            return false;
        }
        if self.position + 1 < self.tokens.len() {
            matches!(self.tokens[self.position + 1].token_type, TokenType::Select)
        } else {
            false
        }
    }

    fn expect(&mut self, token_type: &TokenType, name: &str) -> ParseResult<()> {
        if self.check(token_type) {
            self.advance();
            Ok(())
        } else {
            Err(ParseError::UnexpectedToken {
                expected: name.to_string(),
                found: self.peek().token_type.clone(),
                location: self.current_location(),
            })
        }
    }

    fn expect_identifier(&mut self) -> ParseResult<String> {
        // Many Fortran keywords can also be used as identifiers (variable names)
        // depending on context. Allow keywords in identifier positions.
        match &self.peek().token_type {
            TokenType::Identifier(name) => {
                let name = name.clone();
                self.advance();
                Ok(name)
            }
            // Allow certain keywords as identifiers in variable contexts
            TokenType::Result => {
                self.advance();
                Ok("RESULT".to_string())
            }
            TokenType::Stat => {
                self.advance();
                Ok("STAT".to_string())
            }
            TokenType::Kind => {
                self.advance();
                Ok("KIND".to_string())
            }
            TokenType::Len => {
                self.advance();
                Ok("LEN".to_string())
            }
            _ => Err(ParseError::UnexpectedToken {
                expected: "identifier".to_string(),
                found: self.peek().token_type.clone(),
                location: self.current_location(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    fn parse_expr(source: &str) -> ParseResult<Expr> {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        parser.parse_expression()
    }

    #[test]
    fn test_parse_integer_literal() {
        let expr = parse_expr("42").unwrap();
        assert!(matches!(expr, Expr::IntegerLiteral(42, _)));
    }

    #[test]
    fn test_parse_real_literal() {
        let expr = parse_expr("3.14").unwrap();
        assert!(matches!(expr, Expr::RealLiteral(x, _) if (x - 3.14).abs() < 1e-10));
    }

    #[test]
    fn test_parse_identifier() {
        let expr = parse_expr("x").unwrap();
        assert!(matches!(expr, Expr::Identifier(name, _) if name == "X"));
    }

    #[test]
    fn test_parse_addition() {
        let expr = parse_expr("1 + 2").unwrap();
        match expr {
            Expr::BinaryOp { op, .. } => {
                assert_eq!(op, BinaryOperator::Add);
            }
            _ => panic!("Expected BinaryOp"),
        }
    }

    #[test]
    fn test_operator_precedence() {
        // 1 + 2 * 3 should be 1 + (2 * 3)
        let expr = parse_expr("1 + 2 * 3").unwrap();
        match expr {
            Expr::BinaryOp { op, left, right, .. } => {
                assert_eq!(op, BinaryOperator::Add);
                assert!(matches!(*left, Expr::IntegerLiteral(1, _)));
                assert!(matches!(*right, Expr::BinaryOp { op: BinaryOperator::Multiply, .. }));
            }
            _ => panic!("Expected BinaryOp"),
        }
    }

    #[test]
    fn test_power_right_associative() {
        // 2 ** 3 ** 4 should be 2 ** (3 ** 4)
        let expr = parse_expr("2 ** 3 ** 4").unwrap();
        match expr {
            Expr::BinaryOp { op, left, right, .. } => {
                assert_eq!(op, BinaryOperator::Power);
                assert!(matches!(*left, Expr::IntegerLiteral(2, _)));
                assert!(matches!(*right, Expr::BinaryOp { op: BinaryOperator::Power, .. }));
            }
            _ => panic!("Expected BinaryOp"),
        }
    }

    #[test]
    fn test_parenthesized_expression() {
        let expr = parse_expr("(1 + 2) * 3").unwrap();
        match expr {
            Expr::BinaryOp { op, left, .. } => {
                assert_eq!(op, BinaryOperator::Multiply);
                assert!(matches!(*left, Expr::Parenthesized(_, _)));
            }
            _ => panic!("Expected BinaryOp"),
        }
    }

    #[test]
    fn test_unary_minus() {
        let expr = parse_expr("-5").unwrap();
        assert!(matches!(expr, Expr::UnaryOp { op: UnaryOperator::Minus, .. }));
    }

    #[test]
    fn test_parse_simple_program() {
        let source = r#"
            program test
              integer :: x, y, z
              x = 5
              y = 10
              z = x + y * 2
            end program test
        "#;

        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().unwrap();

        assert_eq!(program.name, Some("TEST".to_string()));
        assert_eq!(program.declarations.len(), 1);
        assert_eq!(program.statements.len(), 3);
    }
}
