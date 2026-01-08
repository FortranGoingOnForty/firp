//! Tests for Fortran derived type parsing and execution

use firp::ast::*;
use firp::bytecode::Compiler;
use firp::lexer::Lexer;
use firp::parser::Parser;
use firp::vm::VM;

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

fn compile_and_run(source: &str) -> Result<VM, String> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().map_err(|e| format!("Lexer error: {}", e))?;

    let mut parser = Parser::new(tokens);
    let program = parser.parse_program().map_err(|e| format!("Parser error: {}", e))?;

    let mut compiler = Compiler::new();
    let chunk = compiler.compile(&program).map_err(|e| format!("Compile error: {}", e))?;

    let mut vm = VM::new();
    vm.run(chunk).map_err(|e| format!("Runtime error: {}", e))?;

    Ok(vm)
}

fn compile_and_run_unit(source: &str) -> Result<VM, String> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().map_err(|e| format!("Lexer error: {}", e))?;

    let mut parser = Parser::new(tokens);
    let unit = parser.parse_compilation_unit().map_err(|e| format!("Parser error: {}", e))?;

    let mut compiler = Compiler::new();
    let chunk = compiler.compile_unit(&unit).map_err(|e| format!("Compile error: {}", e))?;

    let mut vm = VM::new();
    vm.run(chunk).map_err(|e| format!("Runtime error: {}", e))?;

    Ok(vm)
}

fn get_output(vm: &VM) -> String {
    vm.output().join("\n")
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

// =====================================================================
// Execution Tests for Derived Types
// =====================================================================

#[test]
fn test_execute_type_constructor() {
    let source = r#"
        PROGRAM test_constructor
          IMPLICIT NONE
          TYPE :: Point
            REAL :: x
            REAL :: y
          END TYPE Point

          TYPE(Point) :: p
          p = Point(3.0, 4.0)
          PRINT *, p%x
          PRINT *, p%y
        END PROGRAM test_constructor
    "#;

    let vm = compile_and_run(source).expect("Should run successfully");
    let output = get_output(&vm);
    assert!(output.contains("3"), "Output should contain x value: {}", output);
    assert!(output.contains("4"), "Output should contain y value: {}", output);
}

#[test]
fn test_execute_component_assignment() {
    let source = r#"
        PROGRAM test_comp_assign
          IMPLICIT NONE
          TYPE :: Counter
            INTEGER :: value
          END TYPE Counter

          TYPE(Counter) :: c
          c%value = 42
          PRINT *, c%value
        END PROGRAM test_comp_assign
    "#;

    let vm = compile_and_run(source).expect("Should run successfully");
    let output = get_output(&vm);
    assert!(output.contains("42"), "Output should contain 42: {}", output);
}

#[test]
fn test_execute_component_in_expression() {
    let source = r#"
        PROGRAM test_expr
          IMPLICIT NONE
          TYPE :: Point
            REAL :: x
            REAL :: y
          END TYPE Point

          TYPE(Point) :: p
          REAL :: sum
          p = Point(10.0, 20.0)
          sum = p%x + p%y
          PRINT *, sum
        END PROGRAM test_expr
    "#;

    let vm = compile_and_run(source).expect("Should run successfully");
    let output = get_output(&vm);
    assert!(output.contains("30"), "Output should contain sum 30: {}", output);
}

#[test]
fn test_execute_multiple_instances() {
    let source = r#"
        PROGRAM test_multi_inst
          IMPLICIT NONE
          TYPE :: Point
            REAL :: x
            REAL :: y
          END TYPE Point

          TYPE(Point) :: p1, p2
          p1 = Point(1.0, 2.0)
          p2 = Point(3.0, 4.0)
          PRINT *, p1%x
          PRINT *, p2%x
        END PROGRAM test_multi_inst
    "#;

    let vm = compile_and_run(source).expect("Should run successfully");
    let output = get_output(&vm);
    assert!(output.contains("1"), "Output should contain p1%x: {}", output);
    assert!(output.contains("3"), "Output should contain p2%x: {}", output);
}

#[test]
fn test_execute_modify_component() {
    let source = r#"
        PROGRAM test_modify
          IMPLICIT NONE
          TYPE :: Point
            REAL :: x
            REAL :: y
          END TYPE Point

          TYPE(Point) :: p
          p = Point(1.0, 2.0)
          p%x = 100.0
          PRINT *, p%x
          PRINT *, p%y
        END PROGRAM test_modify
    "#;

    let vm = compile_and_run(source).expect("Should run successfully");
    let output = get_output(&vm);
    assert!(output.contains("100"), "Output should contain modified x: {}", output);
    assert!(output.contains("2"), "Output should contain original y: {}", output);
}

// =====================================================================
// Operator Overloading Parsing Tests
// =====================================================================

#[test]
fn test_parse_operator_interface() {
    let source = r#"
        MODULE vector_ops
          IMPLICIT NONE

          TYPE :: Vector
            REAL :: x
            REAL :: y
          END TYPE Vector

          INTERFACE OPERATOR(+)
            MODULE PROCEDURE add_vectors
          END INTERFACE

        CONTAINS

          FUNCTION add_vectors(v1, v2) RESULT(v3)
            TYPE(Vector) :: v1, v2
            TYPE(Vector) :: v3
            v3%x = v1%x + v2%x
            v3%y = v1%y + v2%y
          END FUNCTION add_vectors

        END MODULE vector_ops
    "#;

    let module = parse_module(source).expect("Should parse successfully");
    assert_eq!(module.name, "VECTOR_OPS");

    // Check for the interface declaration
    let has_interface = module.declarations.iter().any(|d| {
        matches!(d, Declaration::Interface(_))
    });
    assert!(has_interface, "Module should have an interface declaration");
}

// =====================================================================
// Operator Overloading Execution Tests
// =====================================================================

#[test]
fn test_execute_operator_overloading_simple() {
    // Simplified test: just test calling a function that works with derived types
    let source = r#"
        PROGRAM test_derived_func
          IMPLICIT NONE

          TYPE :: Vec2
            REAL :: x
            REAL :: y
          END TYPE Vec2

          TYPE(Vec2) :: v1, v2, v3
          v1 = Vec2(1.0, 2.0)
          v2 = Vec2(3.0, 4.0)

          ! Direct function call instead of operator
          v3 = add_vec2(v1, v2)
          PRINT *, v3%x
          PRINT *, v3%y

        CONTAINS

          FUNCTION add_vec2(a, b) RESULT(c)
            TYPE(Vec2) :: a, b
            TYPE(Vec2) :: c
            c%x = a%x + b%x
            c%y = a%y + b%y
          END FUNCTION add_vec2

        END PROGRAM test_derived_func
    "#;

    let vm = compile_and_run(source).expect("Should run successfully");
    let output = get_output(&vm);
    assert!(output.contains("4"), "Output should contain v3%x = 4: {}", output);
    assert!(output.contains("6"), "Output should contain v3%y = 6: {}", output);
}

#[test]
fn test_execute_operator_overloading() {
    let source = r#"
        MODULE vector_math
          IMPLICIT NONE

          TYPE :: Vec2
            REAL :: x
            REAL :: y
          END TYPE Vec2

          INTERFACE OPERATOR(+)
            MODULE PROCEDURE add_vec2
          END INTERFACE

        CONTAINS

          FUNCTION add_vec2(a, b) RESULT(c)
            TYPE(Vec2) :: a, b
            TYPE(Vec2) :: c
            c%x = a%x + b%x
            c%y = a%y + b%y
          END FUNCTION add_vec2

        END MODULE vector_math

        PROGRAM test_overload
          USE vector_math
          IMPLICIT NONE

          TYPE(Vec2) :: v1, v2, v3
          v1 = Vec2(1.0, 2.0)
          v2 = Vec2(3.0, 4.0)
          v3 = v1 + v2
          PRINT *, v3%x
          PRINT *, v3%y
        END PROGRAM test_overload
    "#;

    let vm = compile_and_run_unit(source).expect("Should run successfully");
    let output = get_output(&vm);
    assert!(output.contains("4"), "Output should contain v3%x = 4: {}", output);
    assert!(output.contains("6"), "Output should contain v3%y = 6: {}", output);
}

#[test]
fn test_parse_generic_interface() {
    let source = r#"
        MODULE math_ops
          IMPLICIT NONE

          INTERFACE add
            MODULE PROCEDURE add_int
            MODULE PROCEDURE add_real
          END INTERFACE add

        CONTAINS

          FUNCTION add_int(a, b) RESULT(c)
            INTEGER :: a, b, c
            c = a + b
          END FUNCTION add_int

          FUNCTION add_real(x, y) RESULT(z)
            REAL :: x, y, z
            z = x + y
          END FUNCTION add_real

        END MODULE math_ops
    "#;

    let module = parse_module(source).expect("Should parse successfully");
    assert_eq!(module.name, "MATH_OPS");

    // Check for the generic interface declaration
    let interface = module.declarations.iter()
        .find_map(|d| match d {
            Declaration::Interface(i) => Some(i),
            _ => None,
        })
        .expect("Should have an interface declaration");

    match &interface.kind {
        InterfaceKind::Generic(name) => assert_eq!(name, "ADD"),
        _ => panic!("Expected Generic interface"),
    }
    assert_eq!(interface.procedures.len(), 2);
}

#[test]
fn test_execute_generic_interface() {
    let source = r#"
        PROGRAM test_generic
          IMPLICIT NONE

          INTERFACE add
            MODULE PROCEDURE add_int
          END INTERFACE add

          INTEGER :: x, y, result
          x = 10
          y = 20
          result = add(x, y)
          PRINT *, result

        CONTAINS

          FUNCTION add_int(a, b) RESULT(c)
            INTEGER :: a, b, c
            c = a + b
          END FUNCTION add_int

        END PROGRAM test_generic
    "#;

    let vm = compile_and_run(source).expect("Should run successfully");
    let output = get_output(&vm);
    assert!(output.contains("30"), "Output should contain 30: {}", output);
}
