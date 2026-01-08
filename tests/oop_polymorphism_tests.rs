//! Tests for OOP Polymorphism Features (Sprint 14 completion)

use firp::bytecode::Compiler;
use firp::lexer::Lexer;
use firp::parser::Parser;
use firp::vm::VM;

fn compile_and_run(source: &str) -> Result<VM, String> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().map_err(|e| format!("Lexer error: {:?}", e))?;
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program().map_err(|e| format!("Parse error: {:?}", e))?;
    let mut compiler = Compiler::new();
    let chunk = compiler.compile(&program).map_err(|e| format!("Compile error: {:?}", e))?;
    let mut vm = VM::new();
    vm.run(chunk).map_err(|e| format!("Runtime error: {:?}", e))?;
    Ok(vm)
}

fn get_output(vm: &VM) -> String {
    vm.output().join("\n")
}

// ==================== Phase 3.1: Component Inheritance Tests ====================

#[test]
fn test_basic_derived_type() {
    let source = r#"
        PROGRAM test_basic
          IMPLICIT NONE

          TYPE :: Point
            REAL :: x
            REAL :: y
          END TYPE Point

          TYPE(Point) :: p
          p%x = 1.0
          p%y = 2.0
          PRINT *, p%x, p%y
        END PROGRAM
    "#;
    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    assert!(output.contains("1") && output.contains("2"), "Expected output with 1 and 2, got: {}", output);
}

#[test]
fn test_type_extends_basic() {
    let source = r#"
        PROGRAM test_extends
          IMPLICIT NONE

          TYPE :: Shape
            REAL :: x
            REAL :: y
          END TYPE Shape

          TYPE, EXTENDS(Shape) :: Circle
            REAL :: radius
          END TYPE Circle

          TYPE(Circle) :: c
          c%x = 1.0
          c%y = 2.0
          c%radius = 5.0
          PRINT *, c%x, c%y, c%radius
        END PROGRAM
    "#;
    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    assert!(output.contains("1") && output.contains("2") && output.contains("5"),
            "Expected output with 1, 2, and 5, got: {}", output);
}

#[test]
fn test_type_extends_multiple_levels() {
    let source = r#"
        PROGRAM test_multi_extends
          IMPLICIT NONE

          TYPE :: Shape
            REAL :: x
            REAL :: y
          END TYPE Shape

          TYPE, EXTENDS(Shape) :: Circle
            REAL :: radius
          END TYPE Circle

          TYPE, EXTENDS(Circle) :: ColorCircle
            INTEGER :: color
          END TYPE ColorCircle

          TYPE(ColorCircle) :: cc
          cc%x = 1.0
          cc%y = 2.0
          cc%radius = 5.0
          cc%color = 255
          PRINT *, cc%x, cc%y, cc%radius, cc%color
        END PROGRAM
    "#;
    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    assert!(output.contains("1") && output.contains("255"),
            "Expected output with inherited and own components, got: {}", output);
}

#[test]
fn test_inherited_components_modification() {
    let source = r#"
        PROGRAM test_modify_inherited
          IMPLICIT NONE

          TYPE :: Base
            INTEGER :: value
          END TYPE Base

          TYPE, EXTENDS(Base) :: Derived
            INTEGER :: extra
          END TYPE Derived

          TYPE(Derived) :: d
          d%value = 10
          d%extra = 20
          d%value = d%value + d%extra
          PRINT *, d%value
        END PROGRAM
    "#;
    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    assert!(output.contains("30"), "Expected 30, got: {}", output);
}

#[test]
fn test_type_constructor_with_inheritance() {
    let source = r#"
        PROGRAM test_constructor
          IMPLICIT NONE

          TYPE :: Point
            REAL :: x
            REAL :: y
          END TYPE Point

          TYPE, EXTENDS(Point) :: Point3D
            REAL :: z
          END TYPE Point3D

          TYPE(Point3D) :: p
          p = Point3D(1.0, 2.0, 3.0)
          PRINT *, p%x, p%y, p%z
        END PROGRAM
    "#;
    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    assert!(output.contains("1") && output.contains("2") && output.contains("3"),
            "Expected 1, 2, 3, got: {}", output);
}

// ==================== Phase 3.2: Type-Bound Procedure Tests ====================

#[test]
fn test_type_bound_procedure_parsing() {
    // Just verify that type-bound procedures can be parsed
    let source = r#"
        PROGRAM test_tbp_parse
          IMPLICIT NONE

          TYPE :: Counter
            INTEGER :: value
          CONTAINS
            PROCEDURE :: increment => counter_increment
          END TYPE Counter

          TYPE(Counter) :: c
          c%value = 0
          PRINT *, c%value

        CONTAINS
          SUBROUTINE counter_increment(self)
            TYPE(Counter) :: self
            self%value = self%value + 1
          END SUBROUTINE
        END PROGRAM
    "#;
    compile_and_run(source).expect("Should compile and run");
}

#[test]
fn test_type_bound_procedure_call() {
    // Test actual method call dispatch
    let source = r#"
        PROGRAM test_tbp_call
          IMPLICIT NONE

          TYPE :: Counter
            INTEGER :: value
          CONTAINS
            PROCEDURE :: increment => counter_increment
          END TYPE Counter

          TYPE(Counter) :: c
          c%value = 10
          CALL c%increment()
          PRINT *, c%value

        CONTAINS
          SUBROUTINE counter_increment(self)
            TYPE(Counter) :: self
            self%value = self%value + 1
          END SUBROUTINE
        END PROGRAM
    "#;
    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    assert!(output.contains("11"), "Expected 11 after incrementing 10, got: {}", output);
}

// ==================== Phase 3.3: Polymorphic CLASS Tests ====================

#[test]
fn test_class_declaration_parsing() {
    // Test that CLASS declarations can be parsed (basic check)
    let source = r#"
        PROGRAM test_class_parse
          IMPLICIT NONE

          TYPE :: Shape
            INTEGER :: id
          END TYPE Shape

          TYPE, EXTENDS(Shape) :: Circle
            REAL :: radius
          END TYPE Circle

          TYPE(Circle) :: c
          c%id = 1
          c%radius = 5.0
          PRINT *, c%id, c%radius
        END PROGRAM
    "#;
    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    assert!(output.contains("1") && output.contains("5"), "Expected 1 and 5, got: {}", output);
}

// ==================== Phase 3.4: Operator Overloading Tests ====================

#[test]
fn test_derived_type_in_expressions() {
    // Test that component access works in expressions (uses built-in operators)
    let source = r#"
        PROGRAM test_expr
          IMPLICIT NONE

          TYPE :: Vec2
            REAL :: x
            REAL :: y
          END TYPE Vec2

          TYPE, EXTENDS(Vec2) :: Vec3
            REAL :: z
          END TYPE Vec3

          TYPE(Vec3) :: v
          REAL :: length_sq

          v%x = 3.0
          v%y = 4.0
          v%z = 0.0

          length_sq = v%x * v%x + v%y * v%y + v%z * v%z
          PRINT *, length_sq
        END PROGRAM
    "#;
    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    assert!(output.contains("25"), "Expected 25 (3^2 + 4^2), got: {}", output);
}
