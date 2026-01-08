//! Tests for Sprint 16 and Sprint 17 intrinsics

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

// ==================== Sprint 17: Numeric Inquiry Intrinsics ====================

#[test]
fn test_huge_integer() {
    let source = r#"
        PROGRAM test_huge
          IMPLICIT NONE
          INTEGER :: max_int
          max_int = HUGE(1)
        END PROGRAM
    "#;
    compile_and_run(source).expect("Should compile and run");
}

#[test]
fn test_huge_real() {
    let source = r#"
        PROGRAM test_huge_real
          IMPLICIT NONE
          REAL :: max_real
          max_real = HUGE(1.0)
        END PROGRAM
    "#;
    compile_and_run(source).expect("Should compile and run");
}

#[test]
fn test_tiny() {
    let source = r#"
        PROGRAM test_tiny
          IMPLICIT NONE
          REAL :: min_pos
          min_pos = TINY(1.0)
        END PROGRAM
    "#;
    compile_and_run(source).expect("Should compile and run");
}

#[test]
fn test_epsilon() {
    let source = r#"
        PROGRAM test_epsilon
          IMPLICIT NONE
          REAL :: eps
          eps = EPSILON(1.0)
        END PROGRAM
    "#;
    compile_and_run(source).expect("Should compile and run");
}

#[test]
fn test_digits() {
    let source = r#"
        PROGRAM test_digits
          IMPLICIT NONE
          INTEGER :: int_digits, real_digits
          int_digits = DIGITS(1)
          real_digits = DIGITS(1.0)
        END PROGRAM
    "#;
    compile_and_run(source).expect("Should compile and run");
}

#[test]
fn test_precision() {
    let source = r#"
        PROGRAM test_precision
          IMPLICIT NONE
          INTEGER :: prec
          prec = PRECISION(1.0)
        END PROGRAM
    "#;
    compile_and_run(source).expect("Should compile and run");
}

#[test]
fn test_range() {
    let source = r#"
        PROGRAM test_range
          IMPLICIT NONE
          INTEGER :: int_range, real_range
          int_range = RANGE(1)
          real_range = RANGE(1.0)
        END PROGRAM
    "#;
    compile_and_run(source).expect("Should compile and run");
}

#[test]
fn test_radix() {
    let source = r#"
        PROGRAM test_radix
          IMPLICIT NONE
          INTEGER :: rad
          rad = RADIX(1.0)
        END PROGRAM
    "#;
    compile_and_run(source).expect("Should compile and run");
}

#[test]
fn test_bit_size() {
    let source = r#"
        PROGRAM test_bit_size
          IMPLICIT NONE
          INTEGER :: bits
          bits = BIT_SIZE(1)
        END PROGRAM
    "#;
    compile_and_run(source).expect("Should compile and run");
}

// ==================== Sprint 17: Additional Bit Intrinsics ====================

#[test]
fn test_ishftc() {
    let source = r#"
        PROGRAM test_ishftc
          IMPLICIT NONE
          INTEGER :: x, y
          x = 5
          y = ISHFTC(x, 2)
        END PROGRAM
    "#;
    compile_and_run(source).expect("Should compile and run");
}

#[test]
fn test_ishftc_with_size() {
    let source = r#"
        PROGRAM test_ishftc_size
          IMPLICIT NONE
          INTEGER :: x, y
          x = 5
          y = ISHFTC(x, 2, 8)
        END PROGRAM
    "#;
    compile_and_run(source).expect("Should compile and run");
}

#[test]
fn test_mvbits() {
    let source = r#"
        PROGRAM test_mvbits
          IMPLICIT NONE
          INTEGER :: from_val, to_val, result
          from_val = 15
          to_val = 0
          result = MVBITS(from_val, 0, 4, to_val, 4)
        END PROGRAM
    "#;
    compile_and_run(source).expect("Should compile and run");
}

// ==================== Sprint 17: Additional Character Intrinsics ====================

#[test]
fn test_scan() {
    let source = r#"
        PROGRAM test_scan
          IMPLICIT NONE
          INTEGER :: pos
          pos = SCAN("hello", "aeiou")
        END PROGRAM
    "#;
    compile_and_run(source).expect("Should compile and run");
}

#[test]
fn test_scan_not_found() {
    let source = r#"
        PROGRAM test_scan_not_found
          IMPLICIT NONE
          INTEGER :: pos
          pos = SCAN("xyz", "aeiou")
        END PROGRAM
    "#;
    compile_and_run(source).expect("Should compile and run");
}

#[test]
fn test_verify() {
    let source = r#"
        PROGRAM test_verify
          IMPLICIT NONE
          INTEGER :: pos
          pos = VERIFY("aeiou", "aeiou")
        END PROGRAM
    "#;
    compile_and_run(source).expect("Should compile and run");
}

#[test]
fn test_verify_found() {
    let source = r#"
        PROGRAM test_verify_found
          IMPLICIT NONE
          INTEGER :: pos
          pos = VERIFY("hello", "helo")
        END PROGRAM
    "#;
    compile_and_run(source).expect("Should compile and run");
}

#[test]
fn test_achar() {
    let source = r#"
        PROGRAM test_achar
          IMPLICIT NONE
          CHARACTER :: c
          c = ACHAR(65)
        END PROGRAM
    "#;
    compile_and_run(source).expect("Should compile and run");
}

#[test]
fn test_iachar() {
    let source = r#"
        PROGRAM test_iachar
          IMPLICIT NONE
          INTEGER :: code
          code = IACHAR("A")
        END PROGRAM
    "#;
    compile_and_run(source).expect("Should compile and run");
}

// ==================== Sprint 16: Array Inquiry Intrinsics ====================

#[test]
fn test_shape_1d() {
    let source = r#"
        PROGRAM test_shape_1d
          IMPLICIT NONE
          INTEGER :: arr(5)
          INTEGER :: shp(1)
          shp = SHAPE(arr)
        END PROGRAM
    "#;
    compile_and_run(source).expect("Should compile and run");
}

#[test]
fn test_shape_2d() {
    let source = r#"
        PROGRAM test_shape_2d
          IMPLICIT NONE
          INTEGER :: arr(3, 4)
          INTEGER :: shp(2)
          shp = SHAPE(arr)
        END PROGRAM
    "#;
    compile_and_run(source).expect("Should compile and run");
}

#[test]
fn test_lbound() {
    let source = r#"
        PROGRAM test_lbound
          IMPLICIT NONE
          INTEGER :: arr(5)
          INTEGER :: lb
          lb = LBOUND(arr, 1)
        END PROGRAM
    "#;
    compile_and_run(source).expect("Should compile and run");
}

#[test]
fn test_ubound() {
    let source = r#"
        PROGRAM test_ubound
          IMPLICIT NONE
          INTEGER :: arr(5)
          INTEGER :: ub
          ub = UBOUND(arr, 1)
        END PROGRAM
    "#;
    compile_and_run(source).expect("Should compile and run");
}

#[test]
fn test_rank() {
    let source = r#"
        PROGRAM test_rank
          IMPLICIT NONE
          INTEGER :: arr(3, 4, 5)
          INTEGER :: r
          r = RANK(arr)
        END PROGRAM
    "#;
    compile_and_run(source).expect("Should compile and run");
}

#[test]
fn test_rank_scalar() {
    let source = r#"
        PROGRAM test_rank_scalar
          IMPLICIT NONE
          INTEGER :: x
          INTEGER :: r
          x = 5
          r = RANK(x)
        END PROGRAM
    "#;
    compile_and_run(source).expect("Should compile and run");
}

#[test]
fn test_any() {
    let source = r#"
        PROGRAM test_any
          IMPLICIT NONE
          LOGICAL :: mask(4)
          LOGICAL :: result
          mask(1) = .TRUE.
          mask(2) = .FALSE.
          mask(3) = .FALSE.
          mask(4) = .FALSE.
          result = ANY(mask)
        END PROGRAM
    "#;
    compile_and_run(source).expect("Should compile and run");
}

#[test]
fn test_all() {
    let source = r#"
        PROGRAM test_all
          IMPLICIT NONE
          LOGICAL :: mask(3)
          LOGICAL :: result
          mask(1) = .TRUE.
          mask(2) = .TRUE.
          mask(3) = .TRUE.
          result = ALL(mask)
        END PROGRAM
    "#;
    compile_and_run(source).expect("Should compile and run");
}

#[test]
fn test_count() {
    let source = r#"
        PROGRAM test_count
          IMPLICIT NONE
          LOGICAL :: mask(5)
          INTEGER :: c
          mask(1) = .TRUE.
          mask(2) = .FALSE.
          mask(3) = .TRUE.
          mask(4) = .TRUE.
          mask(5) = .FALSE.
          c = COUNT(mask)
        END PROGRAM
    "#;
    compile_and_run(source).expect("Should compile and run");
}

#[test]
fn test_maxloc() {
    let source = r#"
        PROGRAM test_maxloc
          IMPLICIT NONE
          INTEGER :: arr(5)
          INTEGER :: loc(1)
          arr(1) = 3
          arr(2) = 7
          arr(3) = 2
          arr(4) = 9
          arr(5) = 1
          loc = MAXLOC(arr)
        END PROGRAM
    "#;
    compile_and_run(source).expect("Should compile and run");
}

#[test]
fn test_minloc() {
    let source = r#"
        PROGRAM test_minloc
          IMPLICIT NONE
          INTEGER :: arr(5)
          INTEGER :: loc(1)
          arr(1) = 3
          arr(2) = 7
          arr(3) = 2
          arr(4) = 9
          arr(5) = 1
          loc = MINLOC(arr)
        END PROGRAM
    "#;
    compile_and_run(source).expect("Should compile and run");
}

#[test]
fn test_allocated() {
    let source = r#"
        PROGRAM test_allocated
          IMPLICIT NONE
          INTEGER :: arr(5)
          LOGICAL :: is_alloc
          is_alloc = ALLOCATED(arr)
        END PROGRAM
    "#;
    compile_and_run(source).expect("Should compile and run");
}

// ==================== Combined Tests ====================

#[test]
fn test_numeric_inquiry_combined() {
    let source = r#"
        PROGRAM test_combined
          IMPLICIT NONE
          INTEGER :: huge_int, bits, radix_val
          REAL :: huge_real, tiny_real, eps

          huge_int = HUGE(1)
          huge_real = HUGE(1.0)
          tiny_real = TINY(1.0)
          eps = EPSILON(1.0)
          bits = BIT_SIZE(1)
          radix_val = RADIX(1.0)
        END PROGRAM
    "#;
    compile_and_run(source).expect("Should compile and run");
}

#[test]
fn test_array_inquiry_combined() {
    let source = r#"
        PROGRAM test_array_combined
          IMPLICIT NONE
          INTEGER :: arr(3, 4)
          INTEGER :: shp(2), lb, ub, r

          shp = SHAPE(arr)
          lb = LBOUND(arr, 1)
          ub = UBOUND(arr, 2)
          r = RANK(arr)
        END PROGRAM
    "#;
    compile_and_run(source).expect("Should compile and run");
}

#[test]
fn test_character_ascii_roundtrip() {
    let source = r#"
        PROGRAM test_ascii_roundtrip
          IMPLICIT NONE
          CHARACTER :: c
          INTEGER :: code

          c = ACHAR(65)
          code = IACHAR(c)
        END PROGRAM
    "#;
    compile_and_run(source).expect("Should compile and run");
}
