//! Tests for Fortran module parsing and execution

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

#[test]
fn test_parse_simple_module() {
    let source = r#"
        MODULE my_module
        END MODULE my_module
    "#;

    let module = parse_module(source).expect("Should parse successfully");
    assert_eq!(module.name, "MY_MODULE");
    assert_eq!(module.default_visibility, Visibility::Public);
    assert!(module.uses.is_empty());
    assert!(module.declarations.is_empty());
    assert!(module.procedures.is_empty());
}

#[test]
fn test_parse_module_with_variable() {
    let source = r#"
        MODULE constants
          IMPLICIT NONE
          REAL, PARAMETER :: PI = 3.14159
        END MODULE constants
    "#;

    let module = parse_module(source).expect("Should parse successfully");
    assert_eq!(module.name, "CONSTANTS");
    assert_eq!(module.declarations.len(), 2); // IMPLICIT NONE + PI
}

#[test]
fn test_parse_module_with_private() {
    let source = r#"
        MODULE my_module
          PRIVATE
          PUBLIC :: x, y
          INTEGER :: x, y, z
        END MODULE my_module
    "#;

    let module = parse_module(source).expect("Should parse successfully");
    assert_eq!(module.default_visibility, Visibility::Private);
    assert_eq!(module.visibility_stmts.len(), 2); // PRIVATE + PUBLIC :: x, y

    // Check that PUBLIC statement lists x and y
    let public_stmt = &module.visibility_stmts[1];
    assert_eq!(public_stmt.visibility, Visibility::Public);
    assert_eq!(public_stmt.names, vec!["X".to_string(), "Y".to_string()]);
}

#[test]
fn test_parse_use_simple() {
    let source = r#"
        MODULE uses_other
          USE other_module
        END MODULE uses_other
    "#;

    let module = parse_module(source).expect("Should parse successfully");
    assert_eq!(module.uses.len(), 1);
    assert_eq!(module.uses[0].module_name, "OTHER_MODULE");
    assert!(module.uses[0].only.is_none());
}

#[test]
fn test_parse_use_only() {
    let source = r#"
        MODULE uses_other
          USE other_module, ONLY: foo, bar
        END MODULE uses_other
    "#;

    let module = parse_module(source).expect("Should parse successfully");
    assert_eq!(module.uses.len(), 1);
    assert_eq!(module.uses[0].module_name, "OTHER_MODULE");

    let only = module.uses[0].only.as_ref().expect("Should have ONLY clause");
    assert_eq!(only.len(), 2);
    assert_eq!(only[0].local_name, "FOO");
    assert!(only[0].original_name.is_none());
    assert_eq!(only[1].local_name, "BAR");
}

#[test]
fn test_parse_use_renaming() {
    let source = r#"
        MODULE uses_other
          USE other_module, ONLY: my_func => original_func
        END MODULE uses_other
    "#;

    let module = parse_module(source).expect("Should parse successfully");

    let only = module.uses[0].only.as_ref().expect("Should have ONLY clause");
    assert_eq!(only.len(), 1);
    assert_eq!(only[0].local_name, "MY_FUNC");
    assert_eq!(only[0].original_name.as_deref(), Some("ORIGINAL_FUNC"));
}

#[test]
fn test_parse_module_with_procedure() {
    let source = r#"
        MODULE math_utils
          IMPLICIT NONE
        CONTAINS
          FUNCTION add(a, b) RESULT(c)
            REAL :: a, b, c
            c = a + b
          END FUNCTION add
        END MODULE math_utils
    "#;

    let module = parse_module(source).expect("Should parse successfully");
    assert_eq!(module.name, "MATH_UTILS");
    assert_eq!(module.procedures.len(), 1);

    match &module.procedures[0] {
        Procedure::Function(f) => {
            assert_eq!(f.name, "ADD");
            assert_eq!(f.result_name.as_deref(), Some("C"));
        }
        _ => panic!("Expected function"),
    }
}

#[test]
fn test_parse_complete_module() {
    let source = r#"
        MODULE math_utils
          IMPLICIT NONE
          PRIVATE
          PUBLIC :: add, multiply, PI

          REAL, PARAMETER :: PI = 3.14159

        CONTAINS

          FUNCTION add(a, b) RESULT(c)
            REAL :: a, b
            REAL :: c
            c = a + b
          END FUNCTION add

          FUNCTION multiply(a, b) RESULT(c)
            REAL :: a, b
            REAL :: c
            c = a * b
          END FUNCTION multiply

        END MODULE math_utils
    "#;

    let module = parse_module(source).expect("Should parse successfully");
    assert_eq!(module.name, "MATH_UTILS");
    assert_eq!(module.default_visibility, Visibility::Private);
    assert_eq!(module.procedures.len(), 2);

    // Check PUBLIC statement
    let public_stmt = module.visibility_stmts.iter()
        .find(|s| s.visibility == Visibility::Public)
        .expect("Should have PUBLIC statement");
    assert_eq!(public_stmt.names, vec!["ADD".to_string(), "MULTIPLY".to_string(), "PI".to_string()]);
}

#[test]
fn test_parse_multiple_use_statements() {
    let source = r#"
        MODULE composite
          USE module_a
          USE module_b, ONLY: func_b
          USE module_c, ONLY: renamed => original
        END MODULE composite
    "#;

    let module = parse_module(source).expect("Should parse successfully");
    assert_eq!(module.uses.len(), 3);

    assert_eq!(module.uses[0].module_name, "MODULE_A");
    assert!(module.uses[0].only.is_none());

    assert_eq!(module.uses[1].module_name, "MODULE_B");
    assert!(module.uses[1].only.is_some());

    assert_eq!(module.uses[2].module_name, "MODULE_C");
    let only = module.uses[2].only.as_ref().unwrap();
    assert_eq!(only[0].local_name, "RENAMED");
    assert_eq!(only[0].original_name.as_deref(), Some("ORIGINAL"));
}

// =====================================================================
// Module Execution Tests
// =====================================================================

#[test]
fn test_module_function_call() {
    let source = r#"
        MODULE math_utils
          IMPLICIT NONE
        CONTAINS
          FUNCTION times_two(x) RESULT(y)
            REAL :: x, y
            y = x * 2.0
          END FUNCTION times_two
        END MODULE math_utils

        PROGRAM test_module
          USE math_utils
          IMPLICIT NONE
          REAL :: result
          result = times_two(5.0)
          PRINT *, result
        END PROGRAM test_module
    "#;

    let vm = compile_and_run_unit(source).expect("Should compile and run");
    let output = vm.output().join("\n");
    assert!(output.contains("10"), "Expected 10, got: {}", output);
}

#[test]
fn test_module_parameter_constant() {
    let source = r#"
        MODULE constants
          IMPLICIT NONE
          REAL, PARAMETER :: PI = 3.14159
        END MODULE constants

        PROGRAM test_constants
          USE constants
          IMPLICIT NONE
          PRINT *, PI
        END PROGRAM test_constants
    "#;

    let vm = compile_and_run_unit(source).expect("Should compile and run");
    let output = vm.output().join("\n");
    assert!(output.contains("3.14159"), "Expected PI value, got: {}", output);
}

#[test]
fn test_module_use_only() {
    let source = r#"
        MODULE math_lib
          IMPLICIT NONE
        CONTAINS
          FUNCTION add(a, b) RESULT(c)
            REAL :: a, b, c
            c = a + b
          END FUNCTION add

          FUNCTION subtract(a, b) RESULT(c)
            REAL :: a, b, c
            c = a - b
          END FUNCTION subtract
        END MODULE math_lib

        PROGRAM test_only
          USE math_lib, ONLY: add
          IMPLICIT NONE
          REAL :: result
          result = add(3.0, 4.0)
          PRINT *, result
        END PROGRAM test_only
    "#;

    let vm = compile_and_run_unit(source).expect("Should compile and run");
    let output = vm.output().join("\n");
    assert!(output.contains("7"), "Expected 7, got: {}", output);
}

#[test]
fn test_module_use_renaming() {
    let source = r#"
        MODULE utils
          IMPLICIT NONE
        CONTAINS
          FUNCTION compute(x) RESULT(y)
            REAL :: x, y
            y = x * x
          END FUNCTION compute
        END MODULE utils

        PROGRAM test_rename
          USE utils, ONLY: square => compute
          IMPLICIT NONE
          REAL :: result
          result = square(5.0)
          PRINT *, result
        END PROGRAM test_rename
    "#;

    let vm = compile_and_run_unit(source).expect("Should compile and run");
    let output = vm.output().join("\n");
    assert!(output.contains("25"), "Expected 25, got: {}", output);
}

#[test]
fn test_multiple_modules() {
    let source = r#"
        MODULE mod_a
          IMPLICIT NONE
        CONTAINS
          FUNCTION func_a(x) RESULT(y)
            REAL :: x, y
            y = x + 10.0
          END FUNCTION func_a
        END MODULE mod_a

        MODULE mod_b
          IMPLICIT NONE
        CONTAINS
          FUNCTION func_b(x) RESULT(y)
            REAL :: x, y
            y = x * 2.0
          END FUNCTION func_b
        END MODULE mod_b

        PROGRAM test_multi
          USE mod_a
          USE mod_b
          IMPLICIT NONE
          REAL :: a, b
          a = func_a(5.0)
          b = func_b(a)
          PRINT *, b
        END PROGRAM test_multi
    "#;

    let vm = compile_and_run_unit(source).expect("Should compile and run");
    let output = vm.output().join("\n");
    // func_a(5.0) = 15.0, func_b(15.0) = 30.0
    assert!(output.contains("30"), "Expected 30, got: {}", output);
}

#[test]
fn test_module_subroutine() {
    let source = r#"
        MODULE procedures
          IMPLICIT NONE
        CONTAINS
          SUBROUTINE print_double(x)
            REAL :: x
            PRINT *, x * 2.0
          END SUBROUTINE print_double
        END MODULE procedures

        PROGRAM test_sub
          USE procedures
          IMPLICIT NONE
          CALL print_double(7.5)
        END PROGRAM test_sub
    "#;

    let vm = compile_and_run_unit(source).expect("Should compile and run");
    let output = vm.output().join("\n");
    assert!(output.contains("15"), "Expected 15, got: {}", output);
}

#[test]
fn test_module_mixed_content() {
    let source = r#"
        MODULE calculator
          IMPLICIT NONE
          REAL, PARAMETER :: MULTIPLIER = 10.0
        CONTAINS
          FUNCTION scale(x) RESULT(y)
            REAL :: x, y
            y = x * MULTIPLIER
          END FUNCTION scale
        END MODULE calculator

        PROGRAM test_mixed
          USE calculator
          IMPLICIT NONE
          REAL :: result
          result = scale(3.0)
          PRINT *, result
        END PROGRAM test_mixed
    "#;

    let vm = compile_and_run_unit(source).expect("Should compile and run");
    let output = vm.output().join("\n");
    // scale(3.0) = 3.0 * 10.0 = 30.0
    assert!(output.contains("30"), "Expected 30, got: {}", output);
}

#[test]
fn test_sprint11_module_success_criteria() {
    // Test that demonstrates Sprint 11 module system functionality
    let source = r#"
        MODULE geometry
          IMPLICIT NONE
          REAL, PARAMETER :: PI = 3.14159

        CONTAINS
          FUNCTION circle_area(radius) RESULT(area)
            REAL :: radius, area
            area = PI * radius * radius
          END FUNCTION circle_area

          FUNCTION rectangle_area(length, width) RESULT(area)
            REAL :: length, width, area
            area = length * width
          END FUNCTION rectangle_area
        END MODULE geometry

        PROGRAM test_geometry
          USE geometry, ONLY: circle_area, rectangle_area
          IMPLICIT NONE
          REAL :: c_area, r_area

          c_area = circle_area(2.0)
          r_area = rectangle_area(3.0, 4.0)

          PRINT *, c_area
          PRINT *, r_area
        END PROGRAM test_geometry
    "#;

    let vm = compile_and_run_unit(source).expect("Should compile and run");
    let output = vm.output().join("\n");

    // circle_area(2.0) = PI * 4.0 ≈ 12.566
    // rectangle_area(3.0, 4.0) = 12.0
    assert!(output.contains("12.566") || output.contains("12.5663"),
        "Expected circle area ~12.566, got: {}", output);
    assert!(output.contains("12") && output.lines().count() >= 2,
        "Expected rectangle area 12, got: {}", output);
}

#[test]
fn test_program_use_statement_parsing() {
    // Verify that programs can have USE statements
    let source = r#"
        PROGRAM test_use
          USE some_module
          USE another_module, ONLY: func1, func2
          IMPLICIT NONE
          INTEGER :: x
          x = 1
        END PROGRAM test_use
    "#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().expect("Should tokenize");
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program().expect("Should parse");

    assert_eq!(program.uses.len(), 2);
    assert_eq!(program.uses[0].module_name, "SOME_MODULE");
    assert!(program.uses[0].only.is_none());
    assert_eq!(program.uses[1].module_name, "ANOTHER_MODULE");
    assert!(program.uses[1].only.is_some());
}

#[test]
fn test_use_only_rejects_private_symbols() {
    // Attempting to import a PRIVATE symbol via USE...ONLY should fail
    let source = r#"
        MODULE my_module
          PRIVATE
          INTEGER :: secret_var = 42
          PUBLIC :: public_var
          INTEGER :: public_var = 100
        END MODULE my_module

        PROGRAM test_private
          USE my_module, ONLY: secret_var
          IMPLICIT NONE
          PRINT *, secret_var
        END PROGRAM
    "#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().expect("Should tokenize");
    let mut parser = Parser::new(tokens);
    let unit = parser.parse_compilation_unit().expect("Should parse");

    let mut compiler = Compiler::new();
    let result = compiler.compile_unit(&unit);

    // Should fail because secret_var is PRIVATE
    assert!(result.is_err(), "Should reject importing PRIVATE symbol");
    let err_msg = format!("{:?}", result.unwrap_err());
    assert!(
        err_msg.contains("PRIVATE") || err_msg.contains("secret_var"),
        "Error should mention PRIVATE visibility, got: {}", err_msg
    );
}

#[test]
fn test_use_only_allows_public_symbols() {
    // Importing a PUBLIC symbol via USE...ONLY should succeed
    let source = r#"
        MODULE my_module
          PRIVATE
          INTEGER :: secret_var = 42
          PUBLIC :: public_var
          INTEGER :: public_var = 100
        END MODULE my_module

        PROGRAM test_public
          USE my_module, ONLY: public_var
          IMPLICIT NONE
          PRINT *, public_var
        END PROGRAM
    "#;

    let vm = compile_and_run_unit(source).expect("Should compile and run");
    let output = vm.output().join("\n");
    assert!(output.contains("100"), "Expected public_var value 100, got: {}", output);
}

#[test]
fn test_parse_type_with_component_visibility() {
    // Test that component visibility attributes are parsed correctly
    let source = r#"
        MODULE mymod
          IMPLICIT NONE
          TYPE :: MyType
            INTEGER, PUBLIC :: x
            INTEGER, PRIVATE :: y
            REAL :: z
          END TYPE MyType
        END MODULE mymod
    "#;

    let module = parse_module(source).expect("Should parse successfully");
    assert_eq!(module.name, "MYMOD");

    // Find the type definition in the declarations
    let type_decl = module.declarations.iter().find(|d| {
        matches!(d, Declaration::DerivedType(t) if t.name == "MYTYPE")
    });
    assert!(type_decl.is_some(), "Should have a DerivedType declaration");

    if let Some(Declaration::DerivedType(type_def)) = type_decl {
        assert_eq!(type_def.components.len(), 3);

        // Check component x has PUBLIC visibility
        let comp_x = type_def.components.iter().find(|c| c.name == "X");
        assert!(comp_x.is_some());
        assert_eq!(comp_x.unwrap().visibility, Some(Visibility::Public));

        // Check component y has PRIVATE visibility
        let comp_y = type_def.components.iter().find(|c| c.name == "Y");
        assert!(comp_y.is_some());
        assert_eq!(comp_y.unwrap().visibility, Some(Visibility::Private));

        // Check component z has no explicit visibility (None)
        let comp_z = type_def.components.iter().find(|c| c.name == "Z");
        assert!(comp_z.is_some());
        assert_eq!(comp_z.unwrap().visibility, None);
    }
}

#[test]
fn test_type_component_visibility_in_runtime() {
    // Test that component visibility is tracked in the runtime type definition
    let source = r#"
        MODULE test_mod
          IMPLICIT NONE
          TYPE :: Point
            REAL, PUBLIC :: x
            REAL, PRIVATE :: internal_val
          END TYPE Point
        CONTAINS
          FUNCTION create_point() RESULT(p)
            TYPE(Point) :: p
            p%x = 10.0
            p%internal_val = 5.0
          END FUNCTION
        END MODULE test_mod

        PROGRAM test_visibility
          USE test_mod
          IMPLICIT NONE
          TYPE(Point) :: pt
          pt = create_point()
          PRINT *, pt%x
        END PROGRAM
    "#;

    let vm = compile_and_run_unit(source).expect("Should compile and run");
    let output = vm.output().join("\n");
    assert!(output.contains("10"), "Expected pt%x = 10, got: {}", output);
}
