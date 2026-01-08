//! Tests for Fortran array intrinsic functions

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
// SUM Tests
// =====================================================================

#[test]
fn test_sum_integer_array() {
    let source = r#"
        PROGRAM test_sum
          IMPLICIT NONE
          INTEGER :: arr(5), total
          INTEGER :: i

          DO i = 1, 5
            arr(i) = i
          END DO

          total = SUM(arr)
          PRINT *, total
        END PROGRAM
    "#;

    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    // 1+2+3+4+5 = 15
    assert!(output.contains("15"), "Expected SUM=15, got: {}", output);
}

#[test]
fn test_sum_real_array() {
    let source = r#"
        PROGRAM test_sum
          IMPLICIT NONE
          REAL :: arr(3), total

          arr(1) = 1.5
          arr(2) = 2.5
          arr(3) = 3.0

          total = SUM(arr)
          PRINT *, total
        END PROGRAM
    "#;

    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    // 1.5 + 2.5 + 3.0 = 7.0
    assert!(output.contains("7"), "Expected SUM=7.0, got: {}", output);
}

// =====================================================================
// PRODUCT Tests
// =====================================================================

#[test]
fn test_product_integer_array() {
    let source = r#"
        PROGRAM test_product
          IMPLICIT NONE
          INTEGER :: arr(4), prod
          INTEGER :: i

          DO i = 1, 4
            arr(i) = i
          END DO

          prod = PRODUCT(arr)
          PRINT *, prod
        END PROGRAM
    "#;

    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    // 1*2*3*4 = 24
    assert!(output.contains("24"), "Expected PRODUCT=24, got: {}", output);
}

#[test]
fn test_product_real_array() {
    let source = r#"
        PROGRAM test_product
          IMPLICIT NONE
          REAL :: arr(3), prod

          arr(1) = 2.0
          arr(2) = 3.0
          arr(3) = 4.0

          prod = PRODUCT(arr)
          PRINT *, prod
        END PROGRAM
    "#;

    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    // 2.0 * 3.0 * 4.0 = 24.0
    assert!(output.contains("24"), "Expected PRODUCT=24.0, got: {}", output);
}

// =====================================================================
// SIZE Tests
// =====================================================================

#[test]
fn test_size_1d_array() {
    let source = r#"
        PROGRAM test_size
          IMPLICIT NONE
          INTEGER :: arr(10), n

          n = SIZE(arr)
          PRINT *, n
        END PROGRAM
    "#;

    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    assert!(output.contains("10"), "Expected SIZE=10, got: {}", output);
}

#[test]
fn test_size_2d_array() {
    let source = r#"
        PROGRAM test_size
          IMPLICIT NONE
          INTEGER :: arr(3, 4), n

          n = SIZE(arr)
          PRINT *, n
        END PROGRAM
    "#;

    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    // 3 * 4 = 12
    assert!(output.contains("12"), "Expected SIZE=12, got: {}", output);
}

// =====================================================================
// MAXVAL Tests
// =====================================================================

#[test]
fn test_maxval_integer_array() {
    let source = r#"
        PROGRAM test_maxval
          IMPLICIT NONE
          INTEGER :: arr(5), m

          arr(1) = 3
          arr(2) = 7
          arr(3) = 2
          arr(4) = 9
          arr(5) = 1

          m = MAXVAL(arr)
          PRINT *, m
        END PROGRAM
    "#;

    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    assert!(output.contains("9"), "Expected MAXVAL=9, got: {}", output);
}

#[test]
fn test_maxval_negative_numbers() {
    let source = r#"
        PROGRAM test_maxval
          IMPLICIT NONE
          INTEGER :: arr(4), m

          arr(1) = -5
          arr(2) = -2
          arr(3) = -8
          arr(4) = -1

          m = MAXVAL(arr)
          PRINT *, m
        END PROGRAM
    "#;

    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    assert!(output.contains("-1"), "Expected MAXVAL=-1, got: {}", output);
}

#[test]
fn test_maxval_real_array() {
    let source = r#"
        PROGRAM test_maxval
          IMPLICIT NONE
          REAL :: arr(3), m

          arr(1) = 1.5
          arr(2) = 3.7
          arr(3) = 2.1

          m = MAXVAL(arr)
          PRINT *, m
        END PROGRAM
    "#;

    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    assert!(output.contains("3.7"), "Expected MAXVAL=3.7, got: {}", output);
}

// =====================================================================
// MINVAL Tests
// =====================================================================

#[test]
fn test_minval_integer_array() {
    let source = r#"
        PROGRAM test_minval
          IMPLICIT NONE
          INTEGER :: arr(5), m

          arr(1) = 3
          arr(2) = 7
          arr(3) = 2
          arr(4) = 9
          arr(5) = 1

          m = MINVAL(arr)
          PRINT *, m
        END PROGRAM
    "#;

    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    assert!(output.contains("1"), "Expected MINVAL=1, got: {}", output);
}

#[test]
fn test_minval_real_array() {
    let source = r#"
        PROGRAM test_minval
          IMPLICIT NONE
          REAL :: arr(3), m

          arr(1) = 1.5
          arr(2) = 0.3
          arr(3) = 2.1

          m = MINVAL(arr)
          PRINT *, m
        END PROGRAM
    "#;

    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    assert!(output.contains("0.3"), "Expected MINVAL=0.3, got: {}", output);
}

// =====================================================================
// DOT_PRODUCT Tests
// =====================================================================

#[test]
fn test_dot_product_integer() {
    let source = r#"
        PROGRAM test_dot
          IMPLICIT NONE
          INTEGER :: a(3), b(3), d

          a(1) = 1
          a(2) = 2
          a(3) = 3

          b(1) = 4
          b(2) = 5
          b(3) = 6

          d = DOT_PRODUCT(a, b)
          PRINT *, d
        END PROGRAM
    "#;

    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    // 1*4 + 2*5 + 3*6 = 4 + 10 + 18 = 32
    assert!(output.contains("32"), "Expected DOT_PRODUCT=32, got: {}", output);
}

#[test]
fn test_dot_product_real() {
    let source = r#"
        PROGRAM test_dot
          IMPLICIT NONE
          REAL :: a(2), b(2), d

          a(1) = 1.0
          a(2) = 2.0

          b(1) = 3.0
          b(2) = 4.0

          d = DOT_PRODUCT(a, b)
          PRINT *, d
        END PROGRAM
    "#;

    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    // 1.0*3.0 + 2.0*4.0 = 3.0 + 8.0 = 11.0
    assert!(output.contains("11"), "Expected DOT_PRODUCT=11.0, got: {}", output);
}

// =====================================================================
// Combined/Complex Tests
// =====================================================================

#[test]
fn test_combined_array_intrinsics() {
    let source = r#"
        PROGRAM test_combined
          IMPLICIT NONE
          INTEGER :: arr(5)
          INTEGER :: i

          DO i = 1, 5
            arr(i) = i * 2
          END DO

          ! arr = [2, 4, 6, 8, 10]
          PRINT *, SUM(arr)
          PRINT *, PRODUCT(arr)
          PRINT *, SIZE(arr)
          PRINT *, MAXVAL(arr)
          PRINT *, MINVAL(arr)
        END PROGRAM
    "#;

    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);

    // SUM = 2+4+6+8+10 = 30
    // PRODUCT = 2*4*6*8*10 = 3840
    // SIZE = 5
    // MAXVAL = 10
    // MINVAL = 2
    let lines: Vec<&str> = output.lines().collect();
    assert!(lines.len() >= 5, "Expected 5 lines of output, got: {}", output);
    assert!(output.contains("30"), "Expected SUM=30, got: {}", output);
    assert!(output.contains("3840"), "Expected PRODUCT=3840, got: {}", output);
    assert!(output.lines().any(|l| l.trim() == "5"), "Expected SIZE=5, got: {}", output);
    assert!(output.lines().any(|l| l.trim() == "10"), "Expected MAXVAL=10, got: {}", output);
    assert!(output.lines().any(|l| l.trim() == "2"), "Expected MINVAL=2, got: {}", output);
}

#[test]
fn test_array_intrinsics_in_expression() {
    let source = r#"
        PROGRAM test_expr
          IMPLICIT NONE
          INTEGER :: arr(4), result
          INTEGER :: i

          DO i = 1, 4
            arr(i) = i
          END DO

          ! Calculate: SUM(arr) + MAXVAL(arr) - MINVAL(arr)
          ! = (1+2+3+4) + 4 - 1 = 10 + 4 - 1 = 13
          result = SUM(arr) + MAXVAL(arr) - MINVAL(arr)
          PRINT *, result
        END PROGRAM
    "#;

    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    assert!(output.contains("13"), "Expected 13, got: {}", output);
}

#[test]
fn test_vector_magnitude() {
    // Calculate magnitude of a 3D vector using DOT_PRODUCT
    let source = r#"
        PROGRAM test_magnitude
          IMPLICIT NONE
          REAL :: vec(3), mag

          vec(1) = 3.0
          vec(2) = 4.0
          vec(3) = 0.0

          ! magnitude = sqrt(v . v)
          mag = SQRT(FLOAT(DOT_PRODUCT(vec, vec)))
          PRINT *, mag
        END PROGRAM
    "#;

    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    // sqrt(9 + 16 + 0) = sqrt(25) = 5
    assert!(output.contains("5"), "Expected magnitude=5, got: {}", output);
}

#[test]
fn test_array_statistics() {
    // Calculate mean using SUM and SIZE
    let source = r#"
        PROGRAM test_mean
          IMPLICIT NONE
          REAL :: data(4), mean

          data(1) = 10.0
          data(2) = 20.0
          data(3) = 30.0
          data(4) = 40.0

          mean = SUM(data) / SIZE(data)
          PRINT *, mean
        END PROGRAM
    "#;

    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);
    // (10 + 20 + 30 + 40) / 4 = 100 / 4 = 25
    assert!(output.contains("25"), "Expected mean=25, got: {}", output);
}

#[test]
fn test_sprint13_array_intrinsics_success() {
    // Comprehensive test for array intrinsics
    let source = r#"
        PROGRAM test_sprint13
          IMPLICIT NONE
          INTEGER :: scores(5)
          INTEGER :: weights(5)
          INTEGER :: i, total, count, highest, lowest

          ! Student scores
          scores(1) = 85
          scores(2) = 92
          scores(3) = 78
          scores(4) = 95
          scores(5) = 88

          ! Calculate statistics
          total = SUM(scores)
          count = SIZE(scores)
          highest = MAXVAL(scores)
          lowest = MINVAL(scores)

          PRINT *, total
          PRINT *, count
          PRINT *, highest
          PRINT *, lowest

          ! Weighted scores
          weights(1) = 1
          weights(2) = 2
          weights(3) = 1
          weights(4) = 2
          weights(5) = 1

          PRINT *, DOT_PRODUCT(scores, weights)
        END PROGRAM
    "#;

    let vm = compile_and_run(source).expect("Should compile and run");
    let output = get_output(&vm);

    // total = 85+92+78+95+88 = 438
    // count = 5
    // highest = 95
    // lowest = 78
    // DOT_PRODUCT = 85*1 + 92*2 + 78*1 + 95*2 + 88*1 = 85 + 184 + 78 + 190 + 88 = 625
    assert!(output.contains("438"), "Expected total=438, got: {}", output);
    assert!(output.lines().any(|l| l.trim() == "5"), "Expected count=5, got: {}", output);
    assert!(output.contains("95"), "Expected highest=95, got: {}", output);
    assert!(output.contains("78"), "Expected lowest=78, got: {}", output);
    assert!(output.contains("625"), "Expected weighted sum=625, got: {}", output);
}
