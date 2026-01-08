//! Tests for Fortran derived type parsing

use firp::ast::*;
use firp::lexer::Lexer;
use firp::parser::Parser;

fn parse_module(source: &str) -> Result<ModuleDef, Box<dyn std::error::Error>> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize()?;
    let mut parser = Parser::new(tokens);
    Ok(parser.parse_module()?)
}

fn parse_program(source: &str) -> Result<Program, Box<dyn std::error::Error>> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize()?;
    let mut parser = Parser::new(tokens);
    Ok(parser.parse_program()?)
}

// =====================================================================
// TYPE Definition Parsing Tests
// =====================================================================

#[test]
fn test_parse_simple_derived_type() {
    let source = r#"
        MODULE test_types
          TYPE :: Point
            REAL :: x
            REAL :: y
          END TYPE Point
        END MODULE test_types
    "#;

    let module = parse_module(source).expect("Should parse successfully");
    assert_eq!(module.name, "TEST_TYPES");

    // Find the derived type declaration
    let type_def = module.declarations.iter()
        .find_map(|d| match d {
            Declaration::DerivedType(t) => Some(t),
            _ => None,
        })
        .expect("Should have a derived type");

    assert_eq!(type_def.name, "POINT");
    assert!(type_def.extends.is_none());
    assert_eq!(type_def.components.len(), 2);
    assert_eq!(type_def.components[0].name, "X");
    assert_eq!(type_def.components[1].name, "Y");
}

#[test]
fn test_parse_derived_type_with_multiple_types() {
    let source = r#"
        MODULE shapes
          TYPE :: Rectangle
            REAL :: width
            REAL :: height
            INTEGER :: color
          END TYPE Rectangle
        END MODULE shapes
    "#;

    let module = parse_module(source).expect("Should parse successfully");

    let type_def = module.declarations.iter()
        .find_map(|d| match d {
            Declaration::DerivedType(t) => Some(t),
            _ => None,
        })
        .expect("Should have a derived type");

    assert_eq!(type_def.name, "RECTANGLE");
    assert_eq!(type_def.components.len(), 3);

    // Check types
    assert!(matches!(type_def.components[0].type_spec, TypeSpec::Real { .. }));
    assert!(matches!(type_def.components[1].type_spec, TypeSpec::Real { .. }));
    assert!(matches!(type_def.components[2].type_spec, TypeSpec::Integer { .. }));
}

#[test]
fn test_parse_derived_type_with_extends() {
    let source = r#"
        MODULE inheritance
          TYPE, EXTENDS(Base) :: Derived
            REAL :: extra
          END TYPE Derived
        END MODULE inheritance
    "#;

    let module = parse_module(source).expect("Should parse successfully");

    let type_def = module.declarations.iter()
        .find_map(|d| match d {
            Declaration::DerivedType(t) => Some(t),
            _ => None,
        })
        .expect("Should have a derived type");

    assert_eq!(type_def.name, "DERIVED");
    assert_eq!(type_def.extends, Some("BASE".to_string()));
}

#[test]
fn test_parse_derived_type_with_procedure() {
    let source = r#"
        MODULE with_procs
          TYPE :: Counter
            INTEGER :: value
          CONTAINS
            PROCEDURE :: increment => counter_increment
          END TYPE Counter
        END MODULE with_procs
    "#;

    let module = parse_module(source).expect("Should parse successfully");

    let type_def = module.declarations.iter()
        .find_map(|d| match d {
            Declaration::DerivedType(t) => Some(t),
            _ => None,
        })
        .expect("Should have a derived type");

    assert_eq!(type_def.name, "COUNTER");
    assert_eq!(type_def.components.len(), 1);
    assert_eq!(type_def.procedures.len(), 1);
    assert_eq!(type_def.procedures[0].binding_name, "INCREMENT");
    assert_eq!(type_def.procedures[0].procedure_name, Some("COUNTER_INCREMENT".to_string()));
}

// =====================================================================
// TYPE Declaration Parsing Tests (TYPE(name) :: var)
// =====================================================================

#[test]
fn test_parse_type_variable_declaration() {
    let source = r#"
        PROGRAM test_type_decl
          TYPE(Point) :: p
        END PROGRAM
    "#;

    let program = parse_program(source).expect("Should parse successfully");

    // Find the variable declaration
    let var_decl = program.declarations.iter()
        .find_map(|d| match d {
            Declaration::Variable { type_spec, entities, .. } => Some((type_spec, entities)),
            _ => None,
        })
        .expect("Should have a variable declaration");

    let (type_spec, entities) = var_decl;

    // Check that it's a derived type
    match type_spec {
        TypeSpec::Derived { name, is_class } => {
            assert_eq!(name, "POINT");
            assert!(!is_class);
        }
        _ => panic!("Expected Derived type"),
    }

    assert_eq!(entities.len(), 1);
    assert_eq!(entities[0].name, "P");
}

#[test]
fn test_parse_class_variable_declaration() {
    let source = r#"
        PROGRAM test_class_decl
          CLASS(Shape) :: s
        END PROGRAM
    "#;

    let program = parse_program(source).expect("Should parse successfully");

    let var_decl = program.declarations.iter()
        .find_map(|d| match d {
            Declaration::Variable { type_spec, entities, .. } => Some((type_spec, entities)),
            _ => None,
        })
        .expect("Should have a variable declaration");

    let (type_spec, entities) = var_decl;

    match type_spec {
        TypeSpec::Derived { name, is_class } => {
            assert_eq!(name, "SHAPE");
            assert!(is_class); // CLASS, not TYPE
        }
        _ => panic!("Expected Derived type"),
    }

    assert_eq!(entities[0].name, "S");
}

#[test]
fn test_parse_multiple_type_variables() {
    let source = r#"
        PROGRAM test_multi
          TYPE(Vector) :: v1, v2, v3
        END PROGRAM
    "#;

    let program = parse_program(source).expect("Should parse successfully");

    let var_decl = program.declarations.iter()
        .find_map(|d| match d {
            Declaration::Variable { entities, .. } => Some(entities),
            _ => None,
        })
        .expect("Should have a variable declaration");

    assert_eq!(var_decl.len(), 3);
    assert_eq!(var_decl[0].name, "V1");
    assert_eq!(var_decl[1].name, "V2");
    assert_eq!(var_decl[2].name, "V3");
}

// =====================================================================
// Component Access Parsing Tests (obj%component)
// =====================================================================

#[test]
fn test_parse_component_access_assignment() {
    let source = r#"
        PROGRAM test_comp_assign
          INTEGER :: x
          x = 1
          p%x = 2.0
        END PROGRAM
    "#;

    let program = parse_program(source).expect("Should parse successfully");

    // Find the component assignment
    let comp_assign = program.statements.iter()
        .find(|s| match s {
            Statement::Assignment { components, .. } => !components.is_empty(),
            _ => false,
        })
        .expect("Should have a component assignment");

    match comp_assign {
        Statement::Assignment { target, components, .. } => {
            assert_eq!(target, "P");
            assert_eq!(components.len(), 1);
            assert_eq!(components[0], "X");
        }
        _ => panic!("Expected Assignment"),
    }
}

#[test]
fn test_parse_chained_component_access() {
    let source = r#"
        PROGRAM test_chain
          INTEGER :: x
          x = 1
          obj%inner%value = 42
        END PROGRAM
    "#;

    let program = parse_program(source).expect("Should parse successfully");

    let comp_assign = program.statements.iter()
        .find(|s| match s {
            Statement::Assignment { components, .. } => components.len() > 1,
            _ => false,
        })
        .expect("Should have a chained component assignment");

    match comp_assign {
        Statement::Assignment { target, components, .. } => {
            assert_eq!(target, "OBJ");
            assert_eq!(components.len(), 2);
            assert_eq!(components[0], "INNER");
            assert_eq!(components[1], "VALUE");
        }
        _ => panic!("Expected Assignment"),
    }
}

#[test]
fn test_parse_component_in_expression() {
    let source = r#"
        PROGRAM test_expr
          INTEGER :: result
          result = p%x + p%y
        END PROGRAM
    "#;

    let program = parse_program(source).expect("Should parse successfully");

    // The expression should contain component accesses
    let assign = &program.statements[0];
    match assign {
        Statement::Assignment { value, .. } => {
            // Should be a binary operation with component accesses
            match value {
                Expr::BinaryOp { left, right, .. } => {
                    assert!(matches!(left.as_ref(), Expr::ComponentAccess { .. }));
                    assert!(matches!(right.as_ref(), Expr::ComponentAccess { .. }));
                }
                _ => panic!("Expected BinaryOp"),
            }
        }
        _ => panic!("Expected Assignment"),
    }
}

// =====================================================================
// Combined Tests
// =====================================================================

#[test]
fn test_parse_complete_derived_type_module() {
    let source = r#"
        MODULE geometry
          IMPLICIT NONE

          TYPE :: Point
            REAL :: x
            REAL :: y
          END TYPE Point

          TYPE :: Circle
            TYPE(Point) :: center
            REAL :: radius
          END TYPE Circle

        END MODULE geometry
    "#;

    let module = parse_module(source).expect("Should parse successfully");
    assert_eq!(module.name, "GEOMETRY");

    // Should have 2 type definitions (plus IMPLICIT NONE)
    let type_count = module.declarations.iter()
        .filter(|d| matches!(d, Declaration::DerivedType(_)))
        .count();
    assert_eq!(type_count, 2);
}

#[test]
fn test_sprint14_parsing_success_criteria() {
    // This tests that the basic parsing infrastructure for Sprint 14 is in place
    let source = r#"
        MODULE point_module
          IMPLICIT NONE

          TYPE :: Point
            REAL :: x
            REAL :: y
          END TYPE Point

        CONTAINS

          FUNCTION create_point(x, y) RESULT(p)
            REAL :: x, y
            TYPE(Point) :: p
          END FUNCTION create_point

        END MODULE point_module
    "#;

    let module = parse_module(source).expect("Should parse successfully");
    assert_eq!(module.name, "POINT_MODULE");

    // Check for Point type
    let point_type = module.declarations.iter()
        .find_map(|d| match d {
            Declaration::DerivedType(t) if t.name == "POINT" => Some(t),
            _ => None,
        })
        .expect("Should have Point type");

    assert_eq!(point_type.components.len(), 2);

    // Check for create_point function
    assert_eq!(module.procedures.len(), 1);
}
