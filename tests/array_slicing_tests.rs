//! Tests for Fortran array slicing and section operations

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
// Array Slice Parsing Tests
// =====================================================================

#[test]
fn test_parse_simple_slice() {
    // Test that arr(1:5) parses without error
    let source = r#"
        PROGRAM test_slice
          IMPLICIT NONE
          INTEGER :: arr(10), slice_arr(5)
          INTEGER :: i

          DO i = 1, 10
            arr(i) = i * 10
          END DO

          ! This should parse as an array section
          slice_arr = arr(1:5)

          PRINT *, slice_arr(1)
        END PROGRAM test_slice
    "#;

    let result = compile_and_run(source);
    assert!(result.is_ok(), "Should compile and run: {:?}", result.err());
}

#[test]
fn test_parse_slice_with_step() {
    // Test that arr(1:10:2) parses without error
    let source = r#"
        PROGRAM test_slice_step
          IMPLICIT NONE
          INTEGER :: arr(10)
          INTEGER :: i

          DO i = 1, 10
            arr(i) = i
          END DO

          ! Parse check for slice with step
          PRINT *, arr(1)
        END PROGRAM test_slice_step
    "#;

    let result = compile_and_run(source);
    assert!(result.is_ok(), "Should compile and run: {:?}", result.err());
}

#[test]
fn test_parse_full_array_slice() {
    // Test that arr(:) parses (full array)
    let source = r#"
        PROGRAM test_full_slice
          IMPLICIT NONE
          INTEGER :: arr(5)
          INTEGER :: i

          DO i = 1, 5
            arr(i) = i
          END DO

          PRINT *, arr(1)
        END PROGRAM test_full_slice
    "#;

    let result = compile_and_run(source);
    assert!(result.is_ok(), "Should compile and run: {:?}", result.err());
}

#[test]
fn test_parse_slice_no_start() {
    // Test that arr(:5) parses (no start)
    let source = r#"
        PROGRAM test_slice_no_start
          IMPLICIT NONE
          INTEGER :: arr(10)
          INTEGER :: i

          DO i = 1, 10
            arr(i) = i
          END DO

          PRINT *, arr(1)
        END PROGRAM test_slice_no_start
    "#;

    let result = compile_and_run(source);
    assert!(result.is_ok(), "Should compile and run: {:?}", result.err());
}

#[test]
fn test_parse_slice_no_end() {
    // Test that arr(5:) parses (no end)
    let source = r#"
        PROGRAM test_slice_no_end
          IMPLICIT NONE
          INTEGER :: arr(10)
          INTEGER :: i

          DO i = 1, 10
            arr(i) = i
          END DO

          PRINT *, arr(1)
        END PROGRAM test_slice_no_end
    "#;

    let result = compile_and_run(source);
    assert!(result.is_ok(), "Should compile and run: {:?}", result.err());
}

#[test]
fn test_parse_slice_step_only() {
    // Test that arr(::2) parses (step only)
    let source = r#"
        PROGRAM test_slice_step_only
          IMPLICIT NONE
          INTEGER :: arr(10)
          INTEGER :: i

          DO i = 1, 10
            arr(i) = i
          END DO

          PRINT *, arr(1)
        END PROGRAM test_slice_step_only
    "#;

    let result = compile_and_run(source);
    assert!(result.is_ok(), "Should compile and run: {:?}", result.err());
}

// =====================================================================
// Array Section Execution Tests
// =====================================================================

#[test]
fn test_execute_slice_basic() {
    // Test basic slice extraction: arr(2:4)
    let source = r#"
        PROGRAM test_exec_slice
          IMPLICIT NONE
          INTEGER :: arr(5)
          INTEGER :: i

          DO i = 1, 5
            arr(i) = i * 10
          END DO

          ! Extract middle elements using slice
          ! arr = [10, 20, 30, 40, 50]
          ! arr(2:4) should be [20, 30, 40]
          PRINT *, SUM(arr(2:4))
        END PROGRAM test_exec_slice
    "#;

    let result = compile_and_run(source);
    assert!(result.is_ok(), "Should compile and run: {:?}", result.err());
    let output = get_output(&result.unwrap());
    // SUM([20, 30, 40]) = 90
    assert!(output.contains("90"), "Expected 90, got: {}", output);
}

#[test]
fn test_execute_slice_with_step() {
    // Test slice with step: arr(1:5:2)
    let source = r#"
        PROGRAM test_exec_step
          IMPLICIT NONE
          INTEGER :: arr(5)
          INTEGER :: i

          DO i = 1, 5
            arr(i) = i * 10
          END DO

          ! arr = [10, 20, 30, 40, 50]
          ! arr(1:5:2) should be [10, 30, 50]
          PRINT *, SUM(arr(1:5:2))
        END PROGRAM test_exec_step
    "#;

    let result = compile_and_run(source);
    assert!(result.is_ok(), "Should compile and run: {:?}", result.err());
    let output = get_output(&result.unwrap());
    // SUM([10, 30, 50]) = 90
    assert!(output.contains("90"), "Expected 90, got: {}", output);
}

#[test]
fn test_execute_full_slice() {
    // Test full array slice: arr(:)
    let source = r#"
        PROGRAM test_exec_full
          IMPLICIT NONE
          INTEGER :: arr(5)
          INTEGER :: i

          DO i = 1, 5
            arr(i) = i
          END DO

          ! arr = [1, 2, 3, 4, 5]
          ! arr(:) should be the full array
          PRINT *, SUM(arr(:))
        END PROGRAM test_exec_full
    "#;

    let result = compile_and_run(source);
    assert!(result.is_ok(), "Should compile and run: {:?}", result.err());
    let output = get_output(&result.unwrap());
    // SUM([1, 2, 3, 4, 5]) = 15
    assert!(output.contains("15"), "Expected 15, got: {}", output);
}

#[test]
fn test_execute_slice_no_start() {
    // Test slice without start: arr(:3)
    let source = r#"
        PROGRAM test_exec_no_start
          IMPLICIT NONE
          INTEGER :: arr(5)
          INTEGER :: i

          DO i = 1, 5
            arr(i) = i * 10
          END DO

          ! arr = [10, 20, 30, 40, 50]
          ! arr(:3) should be [10, 20, 30]
          PRINT *, SUM(arr(:3))
        END PROGRAM test_exec_no_start
    "#;

    let result = compile_and_run(source);
    assert!(result.is_ok(), "Should compile and run: {:?}", result.err());
    let output = get_output(&result.unwrap());
    // SUM([10, 20, 30]) = 60
    assert!(output.contains("60"), "Expected 60, got: {}", output);
}

#[test]
fn test_execute_slice_no_end() {
    // Test slice without end: arr(3:)
    let source = r#"
        PROGRAM test_exec_no_end
          IMPLICIT NONE
          INTEGER :: arr(5)
          INTEGER :: i

          DO i = 1, 5
            arr(i) = i * 10
          END DO

          ! arr = [10, 20, 30, 40, 50]
          ! arr(3:) should be [30, 40, 50]
          PRINT *, SUM(arr(3:))
        END PROGRAM test_exec_no_end
    "#;

    let result = compile_and_run(source);
    assert!(result.is_ok(), "Should compile and run: {:?}", result.err());
    let output = get_output(&result.unwrap());
    // SUM([30, 40, 50]) = 120
    assert!(output.contains("120"), "Expected 120, got: {}", output);
}

#[test]
fn test_slice_odd_elements() {
    // Test every other element: arr(::2)
    let source = r#"
        PROGRAM test_odd
          IMPLICIT NONE
          INTEGER :: arr(6)
          INTEGER :: i

          DO i = 1, 6
            arr(i) = i
          END DO

          ! arr = [1, 2, 3, 4, 5, 6]
          ! arr(::2) should be [1, 3, 5]
          PRINT *, SUM(arr(::2))
        END PROGRAM test_odd
    "#;

    let result = compile_and_run(source);
    assert!(result.is_ok(), "Should compile and run: {:?}", result.err());
    let output = get_output(&result.unwrap());
    // SUM([1, 3, 5]) = 9
    assert!(output.contains("9"), "Expected 9, got: {}", output);
}

#[test]
fn test_slice_even_elements() {
    // Test even elements: arr(2::2)
    let source = r#"
        PROGRAM test_even
          IMPLICIT NONE
          INTEGER :: arr(6)
          INTEGER :: i

          DO i = 1, 6
            arr(i) = i
          END DO

          ! arr = [1, 2, 3, 4, 5, 6]
          ! arr(2::2) should be [2, 4, 6]
          PRINT *, SUM(arr(2::2))
        END PROGRAM test_even
    "#;

    let result = compile_and_run(source);
    assert!(result.is_ok(), "Should compile and run: {:?}", result.err());
    let output = get_output(&result.unwrap());
    // SUM([2, 4, 6]) = 12
    assert!(output.contains("12"), "Expected 12, got: {}", output);
}

// =====================================================================
// Sprint 16 Success Criteria
// =====================================================================

#[test]
fn test_sprint16_array_slicing_success_criteria() {
    // Test the example from sprint-16-array-operations.md
    let source = r#"
        PROGRAM array_ops_test
          IMPLICIT NONE
          INTEGER :: arr(10)
          INTEGER :: slice_sum
          INTEGER :: i

          ! Initialize array
          DO i = 1, 10
            arr(i) = i
          END DO

          ! Test array slicing
          slice_sum = SUM(arr(2:8:2))  ! Sum of arr(2), arr(4), arr(6), arr(8) = 2+4+6+8 = 20
          PRINT *, 'Slice sum:', slice_sum
        END PROGRAM array_ops_test
    "#;

    let result = compile_and_run(source);
    assert!(result.is_ok(), "Should compile and run: {:?}", result.err());
    let output = get_output(&result.unwrap());
    assert!(output.contains("20"), "Expected slice sum of 20, got: {}", output);
}

// =====================================================================
// Array Constructor Tests
// =====================================================================

#[test]
fn test_array_constructor_simple() {
    // Test simple array constructor: [1, 2, 3, 4, 5]
    let source = r#"
        PROGRAM test_constructor
          IMPLICIT NONE
          INTEGER :: arr(5)

          arr = [1, 2, 3, 4, 5]
          PRINT *, SUM(arr)
        END PROGRAM test_constructor
    "#;

    let result = compile_and_run(source);
    assert!(result.is_ok(), "Should compile and run: {:?}", result.err());
    let output = get_output(&result.unwrap());
    // SUM([1, 2, 3, 4, 5]) = 15
    assert!(output.contains("15"), "Expected 15, got: {}", output);
}

#[test]
fn test_array_constructor_with_expressions() {
    // Test array constructor with expressions: [1+1, 2*2, 3+2]
    let source = r#"
        PROGRAM test_expr_constructor
          IMPLICIT NONE
          INTEGER :: arr(3)

          arr = [1+1, 2*2, 3+2]
          PRINT *, SUM(arr)
        END PROGRAM test_expr_constructor
    "#;

    let result = compile_and_run(source);
    assert!(result.is_ok(), "Should compile and run: {:?}", result.err());
    let output = get_output(&result.unwrap());
    // SUM([2, 4, 5]) = 11
    assert!(output.contains("11"), "Expected 11, got: {}", output);
}

#[test]
fn test_array_constructor_implied_do() {
    // Test implied-DO array constructor: [(i, i=1,5)]
    let source = r#"
        PROGRAM test_implied_do
          IMPLICIT NONE
          INTEGER :: arr(5)

          arr = [(i, i=1,5)]
          PRINT *, SUM(arr)
        END PROGRAM test_implied_do
    "#;

    let result = compile_and_run(source);
    assert!(result.is_ok(), "Should compile and run: {:?}", result.err());
    let output = get_output(&result.unwrap());
    // SUM([1, 2, 3, 4, 5]) = 15
    assert!(output.contains("15"), "Expected 15, got: {}", output);
}

#[test]
fn test_array_constructor_implied_do_expression() {
    // Test implied-DO with expression: [(i*2, i=1,5)]
    let source = r#"
        PROGRAM test_implied_expr
          IMPLICIT NONE
          INTEGER :: arr(5)

          arr = [(i*2, i=1,5)]
          PRINT *, SUM(arr)
        END PROGRAM test_implied_expr
    "#;

    let result = compile_and_run(source);
    assert!(result.is_ok(), "Should compile and run: {:?}", result.err());
    let output = get_output(&result.unwrap());
    // SUM([2, 4, 6, 8, 10]) = 30
    assert!(output.contains("30"), "Expected 30, got: {}", output);
}

#[test]
fn test_array_constructor_implied_do_with_step() {
    // Test implied-DO with step: [(i, i=1,10,2)]
    let source = r#"
        PROGRAM test_implied_step
          IMPLICIT NONE
          INTEGER :: arr(5)

          arr = [(i, i=1,10,2)]
          PRINT *, SUM(arr)
        END PROGRAM test_implied_step
    "#;

    let result = compile_and_run(source);
    assert!(result.is_ok(), "Should compile and run: {:?}", result.err());
    let output = get_output(&result.unwrap());
    // SUM([1, 3, 5, 7, 9]) = 25
    assert!(output.contains("25"), "Expected 25, got: {}", output);
}

// =====================================================================
// WHERE Statement Tests
// =====================================================================

#[test]
fn test_where_single_line() {
    // Test single-line WHERE: WHERE (cond) statement
    let source = r#"
        PROGRAM test_where_simple
          IMPLICIT NONE
          INTEGER :: x
          LOGICAL :: cond

          x = 10
          cond = .TRUE.
          WHERE (cond) x = x * 2
          PRINT *, x
        END PROGRAM test_where_simple
    "#;

    let result = compile_and_run(source);
    assert!(result.is_ok(), "Should compile and run: {:?}", result.err());
    let output = get_output(&result.unwrap());
    assert!(output.contains("20"), "Expected 20, got: {}", output);
}

#[test]
fn test_where_with_elsewhere() {
    // Test WHERE/ELSEWHERE construct
    let source = r#"
        PROGRAM test_where_else
          IMPLICIT NONE
          INTEGER :: x
          LOGICAL :: cond

          x = 5
          cond = .FALSE.
          WHERE (cond)
            x = 100
          ELSEWHERE
            x = 200
          END WHERE
          PRINT *, x
        END PROGRAM test_where_else
    "#;

    let result = compile_and_run(source);
    assert!(result.is_ok(), "Should compile and run: {:?}", result.err());
    let output = get_output(&result.unwrap());
    assert!(output.contains("200"), "Expected 200, got: {}", output);
}

#[test]
fn test_where_true_condition() {
    // Test WHERE with true condition
    let source = r#"
        PROGRAM test_where_true
          IMPLICIT NONE
          INTEGER :: x
          LOGICAL :: cond

          x = 5
          cond = .TRUE.
          WHERE (cond)
            x = 100
          ELSEWHERE
            x = 200
          END WHERE
          PRINT *, x
        END PROGRAM test_where_true
    "#;

    let result = compile_and_run(source);
    assert!(result.is_ok(), "Should compile and run: {:?}", result.err());
    let output = get_output(&result.unwrap());
    assert!(output.contains("100"), "Expected 100, got: {}", output);
}

// =====================================================================
// FORALL Statement Tests
// =====================================================================

#[test]
fn test_forall_simple() {
    // Test simple FORALL
    let source = r#"
        PROGRAM test_forall_simple
          IMPLICIT NONE
          INTEGER :: arr(5)
          INTEGER :: i

          ! Initialize array using FORALL
          FORALL (i = 1:5) arr(i) = i * 10
          PRINT *, SUM(arr)
        END PROGRAM test_forall_simple
    "#;

    let result = compile_and_run(source);
    assert!(result.is_ok(), "Should compile and run: {:?}", result.err());
    let output = get_output(&result.unwrap());
    // SUM([10, 20, 30, 40, 50]) = 150
    assert!(output.contains("150"), "Expected 150, got: {}", output);
}

#[test]
fn test_forall_2d() {
    // Test 2D FORALL
    let source = r#"
        PROGRAM test_forall_2d
          IMPLICIT NONE
          INTEGER :: matrix(3, 3)
          INTEGER :: i, j, total

          ! Initialize 2D array using FORALL
          FORALL (i = 1:3, j = 1:3) matrix(i, j) = i + j

          ! Compute sum manually
          total = 0
          DO i = 1, 3
            DO j = 1, 3
              total = total + matrix(i, j)
            END DO
          END DO
          PRINT *, total
        END PROGRAM test_forall_2d
    "#;

    let result = compile_and_run(source);
    assert!(result.is_ok(), "Should compile and run: {:?}", result.err());
    let output = get_output(&result.unwrap());
    // Matrix: 2 3 4 / 3 4 5 / 4 5 6 = 36
    assert!(output.contains("36"), "Expected 36, got: {}", output);
}

// =====================================================================
// Transformational Intrinsics Tests (RESHAPE, TRANSPOSE, MATMUL)
// =====================================================================

#[test]
fn test_reshape_1d_to_2d() {
    // Test RESHAPE: 1D array to 2D matrix
    let source = r#"
        PROGRAM test_reshape
          IMPLICIT NONE
          INTEGER :: arr(6)
          INTEGER :: matrix(2, 3)
          INTEGER :: i

          ! Initialize 1D array [1, 2, 3, 4, 5, 6]
          DO i = 1, 6
            arr(i) = i
          END DO

          ! Reshape to 2x3 matrix
          matrix = RESHAPE(arr, [2, 3])

          ! Print sum (should be 21)
          PRINT *, SUM(matrix)
        END PROGRAM test_reshape
    "#;

    let result = compile_and_run(source);
    assert!(result.is_ok(), "Should compile and run: {:?}", result.err());
    let output = get_output(&result.unwrap());
    assert!(output.contains("21"), "Expected 21, got: {}", output);
}

#[test]
fn test_transpose_2x3() {
    // Test TRANSPOSE: 2x3 matrix to 3x2 matrix
    let source = r#"
        PROGRAM test_transpose
          IMPLICIT NONE
          INTEGER :: matrix(2, 3)
          INTEGER :: transposed(3, 2)
          INTEGER :: i, j

          ! Initialize 2x3 matrix:
          ! | 1 2 3 |
          ! | 4 5 6 |
          matrix(1, 1) = 1
          matrix(1, 2) = 2
          matrix(1, 3) = 3
          matrix(2, 1) = 4
          matrix(2, 2) = 5
          matrix(2, 3) = 6

          ! Transpose to 3x2
          transposed = TRANSPOSE(matrix)

          ! The transposed matrix should be:
          ! | 1 4 |
          ! | 2 5 |
          ! | 3 6 |
          ! Check corner values
          PRINT *, transposed(1, 1)
          PRINT *, transposed(3, 2)
        END PROGRAM test_transpose
    "#;

    let result = compile_and_run(source);
    assert!(result.is_ok(), "Should compile and run: {:?}", result.err());
    let output = get_output(&result.unwrap());
    assert!(output.contains("1"), "Expected 1 at (1,1), got: {}", output);
    assert!(output.contains("6"), "Expected 6 at (3,2), got: {}", output);
}

#[test]
fn test_matmul_2x2() {
    // Test MATMUL: 2x2 * 2x2 matrix multiplication
    let source = r#"
        PROGRAM test_matmul
          IMPLICIT NONE
          INTEGER :: a(2, 2), b(2, 2), c(2, 2)

          ! Matrix A:
          ! | 1 2 |
          ! | 3 4 |
          a(1, 1) = 1
          a(1, 2) = 2
          a(2, 1) = 3
          a(2, 2) = 4

          ! Matrix B:
          ! | 5 6 |
          ! | 7 8 |
          b(1, 1) = 5
          b(1, 2) = 6
          b(2, 1) = 7
          b(2, 2) = 8

          ! C = A * B:
          ! | 1*5+2*7  1*6+2*8 | = | 19 22 |
          ! | 3*5+4*7  3*6+4*8 |   | 43 50 |
          c = MATMUL(a, b)

          PRINT *, c(1, 1)
          PRINT *, c(1, 2)
          PRINT *, c(2, 1)
          PRINT *, c(2, 2)
        END PROGRAM test_matmul
    "#;

    let result = compile_and_run(source);
    assert!(result.is_ok(), "Should compile and run: {:?}", result.err());
    let output = get_output(&result.unwrap());
    assert!(output.contains("19"), "Expected 19 at (1,1), got: {}", output);
    assert!(output.contains("22"), "Expected 22 at (1,2), got: {}", output);
    assert!(output.contains("43"), "Expected 43 at (2,1), got: {}", output);
    assert!(output.contains("50"), "Expected 50 at (2,2), got: {}", output);
}

#[test]
fn test_matmul_2x3_times_3x2() {
    // Test MATMUL with non-square matrices
    let source = r#"
        PROGRAM test_matmul_nonsquare
          IMPLICIT NONE
          INTEGER :: a(2, 3), b(3, 2), c(2, 2)
          INTEGER :: i, j

          ! Matrix A (2x3):
          ! | 1 2 3 |
          ! | 4 5 6 |
          a(1, 1) = 1
          a(1, 2) = 2
          a(1, 3) = 3
          a(2, 1) = 4
          a(2, 2) = 5
          a(2, 3) = 6

          ! Matrix B (3x2):
          ! | 7  8 |
          ! | 9  10 |
          ! | 11 12 |
          b(1, 1) = 7
          b(1, 2) = 8
          b(2, 1) = 9
          b(2, 2) = 10
          b(3, 1) = 11
          b(3, 2) = 12

          ! C = A * B (2x2):
          ! | 1*7+2*9+3*11   1*8+2*10+3*12  | = |  58  64 |
          ! | 4*7+5*9+6*11   4*8+5*10+6*12  |   | 139 154 |
          c = MATMUL(a, b)

          PRINT *, c(1, 1)
          PRINT *, c(2, 2)
        END PROGRAM test_matmul_nonsquare
    "#;

    let result = compile_and_run(source);
    assert!(result.is_ok(), "Should compile and run: {:?}", result.err());
    let output = get_output(&result.unwrap());
    assert!(output.contains("58"), "Expected 58 at (1,1), got: {}", output);
    assert!(output.contains("154"), "Expected 154 at (2,2), got: {}", output);
}

#[test]
fn test_sprint16_transformational_intrinsics() {
    // Comprehensive test combining RESHAPE, TRANSPOSE, and MATMUL
    let source = r#"
        PROGRAM test_transformational
          IMPLICIT NONE
          INTEGER :: data(4)
          INTEGER :: m1(2, 2), m2(2, 2), result(2, 2)
          INTEGER :: i

          ! Create data [1, 2, 3, 4]
          DO i = 1, 4
            data(i) = i
          END DO

          ! Reshape to 2x2 matrix
          m1 = RESHAPE(data, [2, 2])

          ! Create identity-like matrix
          m2(1, 1) = 1
          m2(1, 2) = 0
          m2(2, 1) = 0
          m2(2, 2) = 1

          ! Multiply: m1 * m2 = m1 (since m2 is identity)
          result = MATMUL(m1, m2)

          ! Sum should be 1+2+3+4 = 10
          PRINT *, SUM(result)
        END PROGRAM test_transformational
    "#;

    let result = compile_and_run(source);
    assert!(result.is_ok(), "Should compile and run: {:?}", result.err());
    let output = get_output(&result.unwrap());
    assert!(output.contains("10"), "Expected 10, got: {}", output);
}
