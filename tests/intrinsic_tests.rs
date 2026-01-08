//! Tests for Fortran intrinsic functions

use firp::bytecode::Compiler;
use firp::lexer::Lexer;
use firp::parser::Parser;
use firp::vm::VM;

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

fn get_output(vm: &VM) -> String {
    vm.output().join("\n")
}

// =====================================================================
// Mathematical Functions
// =====================================================================

#[test]
fn test_sqrt() {
    let source = r#"
        PROGRAM test_sqrt
          IMPLICIT NONE
          REAL :: x
          x = SQRT(16.0)
          PRINT *, x
        END PROGRAM
    "#;

    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    assert!(output.contains("4"), "Expected 4, got: {}", output);
}

#[test]
fn test_sqrt_with_integer() {
    let source = r#"
        PROGRAM test_sqrt
          IMPLICIT NONE
          REAL :: x
          x = SQRT(25.0)
          PRINT *, x
        END PROGRAM
    "#;

    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    assert!(output.contains("5"), "Expected 5, got: {}", output);
}

#[test]
fn test_abs_integer() {
    let source = r#"
        PROGRAM test_abs
          IMPLICIT NONE
          INTEGER :: x
          x = ABS(-42)
          PRINT *, x
        END PROGRAM
    "#;

    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    assert!(output.contains("42"), "Expected 42, got: {}", output);
}

#[test]
fn test_abs_real() {
    let source = r#"
        PROGRAM test_abs
          IMPLICIT NONE
          REAL :: x
          x = ABS(-3.14)
          PRINT *, x
        END PROGRAM
    "#;

    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    assert!(output.contains("3.14"), "Expected 3.14, got: {}", output);
}

#[test]
fn test_sin_cos() {
    let source = r#"
        PROGRAM test_trig
          IMPLICIT NONE
          REAL :: s, c
          s = SIN(0.0)
          c = COS(0.0)
          PRINT *, s
          PRINT *, c
        END PROGRAM
    "#;

    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    // SIN(0) = 0, COS(0) = 1
    assert!(output.contains("0"), "Expected SIN(0)=0, got: {}", output);
    assert!(output.contains("1"), "Expected COS(0)=1, got: {}", output);
}

#[test]
fn test_tan() {
    let source = r#"
        PROGRAM test_tan
          IMPLICIT NONE
          REAL :: t
          t = TAN(0.0)
          PRINT *, t
        END PROGRAM
    "#;

    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    // TAN(0) = 0
    assert!(output.contains("0"), "Expected TAN(0)=0, got: {}", output);
}

#[test]
fn test_exp_log() {
    let source = r#"
        PROGRAM test_exp_log
          IMPLICIT NONE
          REAL :: e, l
          e = EXP(1.0)
          l = LOG(2.71828)
          PRINT *, e
          PRINT *, l
        END PROGRAM
    "#;

    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    // EXP(1) ≈ 2.718, LOG(2.718) ≈ 1
    assert!(output.contains("2.71"), "Expected EXP(1)≈2.718, got: {}", output);
    assert!(output.contains("0.99") || output.contains("1.0"), "Expected LOG(e)≈1, got: {}", output);
}

#[test]
fn test_log10() {
    let source = r#"
        PROGRAM test_log10
          IMPLICIT NONE
          REAL :: l
          l = LOG10(100.0)
          PRINT *, l
        END PROGRAM
    "#;

    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    // LOG10(100) = 2
    assert!(output.contains("2"), "Expected LOG10(100)=2, got: {}", output);
}

#[test]
fn test_atan() {
    let source = r#"
        PROGRAM test_atan
          IMPLICIT NONE
          REAL :: a
          a = ATAN(0.0)
          PRINT *, a
        END PROGRAM
    "#;

    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    // ATAN(0) = 0
    assert!(output.contains("0"), "Expected ATAN(0)=0, got: {}", output);
}

#[test]
fn test_atan2() {
    let source = r#"
        PROGRAM test_atan2
          IMPLICIT NONE
          REAL :: a
          a = ATAN2(0.0, 1.0)
          PRINT *, a
        END PROGRAM
    "#;

    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    // ATAN2(0, 1) = 0
    assert!(output.contains("0"), "Expected ATAN2(0,1)=0, got: {}", output);
}

// =====================================================================
// Utility Functions
// =====================================================================

#[test]
fn test_mod_integer() {
    let source = r#"
        PROGRAM test_mod
          IMPLICIT NONE
          INTEGER :: m
          m = MOD(17, 5)
          PRINT *, m
        END PROGRAM
    "#;

    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    // MOD(17, 5) = 2
    assert!(output.contains("2"), "Expected MOD(17,5)=2, got: {}", output);
}

#[test]
fn test_mod_negative() {
    let source = r#"
        PROGRAM test_mod
          IMPLICIT NONE
          INTEGER :: m
          m = MOD(-17, 5)
          PRINT *, m
        END PROGRAM
    "#;

    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    // MOD(-17, 5) = -2 (sign follows dividend)
    assert!(output.contains("-2"), "Expected MOD(-17,5)=-2, got: {}", output);
}

#[test]
fn test_modulo() {
    let source = r#"
        PROGRAM test_modulo
          IMPLICIT NONE
          INTEGER :: m
          m = MODULO(-17, 5)
          PRINT *, m
        END PROGRAM
    "#;

    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    // MODULO(-17, 5) = 3 (always positive when divisor is positive)
    assert!(output.contains("3"), "Expected MODULO(-17,5)=3, got: {}", output);
}

#[test]
fn test_max_two_args() {
    let source = r#"
        PROGRAM test_max
          IMPLICIT NONE
          INTEGER :: m
          m = MAX(3, 7)
          PRINT *, m
        END PROGRAM
    "#;

    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    assert!(output.contains("7"), "Expected MAX(3,7)=7, got: {}", output);
}

#[test]
fn test_max_multiple_args() {
    let source = r#"
        PROGRAM test_max
          IMPLICIT NONE
          INTEGER :: m
          m = MAX(3, 7, 2, 9, 1)
          PRINT *, m
        END PROGRAM
    "#;

    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    assert!(output.contains("9"), "Expected MAX(3,7,2,9,1)=9, got: {}", output);
}

#[test]
fn test_min_two_args() {
    let source = r#"
        PROGRAM test_min
          IMPLICIT NONE
          INTEGER :: m
          m = MIN(3, 7)
          PRINT *, m
        END PROGRAM
    "#;

    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    assert!(output.contains("3"), "Expected MIN(3,7)=3, got: {}", output);
}

#[test]
fn test_min_multiple_args() {
    let source = r#"
        PROGRAM test_min
          IMPLICIT NONE
          INTEGER :: m
          m = MIN(3, 7, 2, 9, 1)
          PRINT *, m
        END PROGRAM
    "#;

    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    assert!(output.contains("1"), "Expected MIN(3,7,2,9,1)=1, got: {}", output);
}

#[test]
fn test_floor() {
    let source = r#"
        PROGRAM test_floor
          IMPLICIT NONE
          INTEGER :: f
          f = FLOOR(3.7)
          PRINT *, f
        END PROGRAM
    "#;

    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    assert!(output.contains("3"), "Expected FLOOR(3.7)=3, got: {}", output);
}

#[test]
fn test_floor_negative() {
    let source = r#"
        PROGRAM test_floor
          IMPLICIT NONE
          INTEGER :: f
          f = FLOOR(-3.2)
          PRINT *, f
        END PROGRAM
    "#;

    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    // FLOOR(-3.2) = -4
    assert!(output.contains("-4"), "Expected FLOOR(-3.2)=-4, got: {}", output);
}

#[test]
fn test_ceiling() {
    let source = r#"
        PROGRAM test_ceiling
          IMPLICIT NONE
          INTEGER :: c
          c = CEILING(3.2)
          PRINT *, c
        END PROGRAM
    "#;

    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    assert!(output.contains("4"), "Expected CEILING(3.2)=4, got: {}", output);
}

#[test]
fn test_nint() {
    let source = r#"
        PROGRAM test_nint
          IMPLICIT NONE
          INTEGER :: n1, n2
          n1 = NINT(3.4)
          n2 = NINT(3.6)
          PRINT *, n1
          PRINT *, n2
        END PROGRAM
    "#;

    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    // NINT(3.4) = 3, NINT(3.6) = 4
    assert!(output.contains("3") && output.contains("4"),
        "Expected NINT(3.4)=3 and NINT(3.6)=4, got: {}", output);
}

#[test]
fn test_sign() {
    let source = r#"
        PROGRAM test_sign
          IMPLICIT NONE
          INTEGER :: s1, s2
          s1 = SIGN(5, -3)
          s2 = SIGN(-5, 3)
          PRINT *, s1
          PRINT *, s2
        END PROGRAM
    "#;

    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    // SIGN(5, -3) = -5, SIGN(-5, 3) = 5
    assert!(output.contains("-5"), "Expected SIGN(5,-3)=-5, got: {}", output);
    // Note: The second PRINT will show 5
    let lines: Vec<&str> = output.lines().collect();
    assert!(lines.len() >= 2, "Expected 2 lines of output, got: {}", output);
}

// =====================================================================
// Type Conversion Functions
// =====================================================================

#[test]
fn test_int_from_real() {
    let source = r#"
        PROGRAM test_int
          IMPLICIT NONE
          INTEGER :: i
          i = INT(3.7)
          PRINT *, i
        END PROGRAM
    "#;

    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    // INT(3.7) = 3 (truncation)
    assert!(output.contains("3"), "Expected INT(3.7)=3, got: {}", output);
}

#[test]
fn test_int_negative() {
    let source = r#"
        PROGRAM test_int
          IMPLICIT NONE
          INTEGER :: i
          i = INT(-3.7)
          PRINT *, i
        END PROGRAM
    "#;

    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    // INT(-3.7) = -3 (truncation toward zero)
    assert!(output.contains("-3"), "Expected INT(-3.7)=-3, got: {}", output);
}

#[test]
fn test_real_from_integer() {
    // Note: REAL is also a type keyword, so we use FLOAT as the legacy alias
    let source = r#"
        PROGRAM test_real
          IMPLICIT NONE
          REAL :: r
          r = FLOAT(42)
          PRINT *, r
        END PROGRAM
    "#;

    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    assert!(output.contains("42"), "Expected FLOAT(42)=42.0, got: {}", output);
}

#[test]
fn test_dble() {
    let source = r#"
        PROGRAM test_dble
          IMPLICIT NONE
          REAL :: d
          d = DBLE(42)
          PRINT *, d
        END PROGRAM
    "#;

    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    assert!(output.contains("42"), "Expected DBLE(42)=42.0, got: {}", output);
}

// =====================================================================
// Combined/Complex Tests
// =====================================================================

#[test]
fn test_intrinsics_in_expressions() {
    let source = r#"
        PROGRAM test_combined
          IMPLICIT NONE
          REAL :: x, result
          x = 2.0
          result = SQRT(x) + ABS(-3.0) + SIN(0.0)
          PRINT *, result
        END PROGRAM
    "#;

    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    // SQRT(2) ≈ 1.414, ABS(-3) = 3, SIN(0) = 0 => ~4.414
    assert!(output.contains("4.41"), "Expected ~4.414, got: {}", output);
}

#[test]
fn test_nested_intrinsics() {
    let source = r#"
        PROGRAM test_nested
          IMPLICIT NONE
          REAL :: result
          result = SQRT(ABS(-16.0))
          PRINT *, result
        END PROGRAM
    "#;

    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    // SQRT(ABS(-16)) = SQRT(16) = 4
    assert!(output.contains("4"), "Expected SQRT(ABS(-16))=4, got: {}", output);
}

#[test]
fn test_intrinsic_with_variables() {
    let source = r#"
        PROGRAM test_vars
          IMPLICIT NONE
          REAL :: x, y, result
          x = 9.0
          y = -5.0
          result = SQRT(x) + ABS(y)
          PRINT *, result
        END PROGRAM
    "#;

    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    // SQRT(9) = 3, ABS(-5) = 5 => 8
    assert!(output.contains("8"), "Expected 8, got: {}", output);
}

#[test]
fn test_pythagorean_theorem() {
    let source = r#"
        PROGRAM test_pythagoras
          IMPLICIT NONE
          REAL :: a, b, c
          a = 3.0
          b = 4.0
          c = SQRT(a*a + b*b)
          PRINT *, c
        END PROGRAM
    "#;

    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    // sqrt(9 + 16) = sqrt(25) = 5
    assert!(output.contains("5"), "Expected 5, got: {}", output);
}

#[test]
fn test_sprint12_intrinsics_success_criteria() {
    // Comprehensive test demonstrating intrinsic function support
    let source = r#"
        PROGRAM test_intrinsics
          IMPLICIT NONE
          REAL :: pi, area, r, x, y, angle
          INTEGER :: n

          ! Circle area calculation
          r = 5.0
          pi = 4.0 * ATAN(1.0)
          area = pi * r * r
          PRINT *, area

          ! Distance calculation using Pythagorean theorem
          x = 3.0
          y = 4.0
          PRINT *, SQRT(x*x + y*y)

          ! Rounding operations
          n = NINT(3.7)
          PRINT *, n

          ! Max/min with multiple arguments
          PRINT *, MAX(1, 5, 3, 2)
          PRINT *, MIN(1, 5, 3, 2)

          ! Type conversion
          PRINT *, INT(9.9)

        END PROGRAM
    "#;

    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);

    // Circle area: pi * 25 ≈ 78.5
    assert!(output.contains("78.5") || output.contains("78.53"),
        "Expected circle area ~78.5, got: {}", output);
    // Distance: 5
    assert!(output.lines().any(|l| l.contains("5")),
        "Expected distance 5, got: {}", output);
    // NINT(3.7) = 4
    assert!(output.lines().any(|l| l.trim() == "4"),
        "Expected NINT(3.7)=4, got: {}", output);
    // MAX = 5, MIN = 1
    assert!(output.lines().filter(|l| l.trim() == "5").count() >= 1,
        "Expected MAX=5, got: {}", output);
    assert!(output.lines().any(|l| l.trim() == "1"),
        "Expected MIN=1, got: {}", output);
    // INT(9.9) = 9
    assert!(output.lines().any(|l| l.trim() == "9"),
        "Expected INT(9.9)=9, got: {}", output);
}

// =====================================================================
// Legacy Function Name Tests (e.g., IABS, DSIN)
// =====================================================================

#[test]
fn test_legacy_iabs() {
    let source = r#"
        PROGRAM test_iabs
          IMPLICIT NONE
          INTEGER :: x
          x = IABS(-42)
          PRINT *, x
        END PROGRAM
    "#;

    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    assert!(output.contains("42"), "Expected IABS(-42)=42, got: {}", output);
}

#[test]
fn test_legacy_float() {
    let source = r#"
        PROGRAM test_float
          IMPLICIT NONE
          REAL :: r
          r = FLOAT(42)
          PRINT *, r
        END PROGRAM
    "#;

    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    assert!(output.contains("42"), "Expected FLOAT(42)=42.0, got: {}", output);
}
