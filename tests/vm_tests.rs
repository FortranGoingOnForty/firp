//! Integration tests for the Virtual Machine

use firp::bytecode::{Compiler, Chunk};
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

fn compile_program(source: &str) -> Result<Chunk, String> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().map_err(|e| format!("Lexer error: {}", e))?;

    let mut parser = Parser::new(tokens);
    let program = parser.parse_program().map_err(|e| format!("Parser error: {}", e))?;

    let mut compiler = Compiler::new();
    compiler.compile(&program).map_err(|e| format!("Compile error: {}", e))
}

#[test]
fn test_simple_assignment() {
    let source = r#"
        program test
          implicit none
          integer :: x
          x = 42
        end program test
    "#;

    let vm = compile_and_run(source).expect("Should run successfully");
    assert_eq!(vm.get_variable("X"), Some(&firp::bytecode::Value::Integer(42)));
}

#[test]
fn test_arithmetic_operations() {
    let source = r#"
        program test
          implicit none
          integer :: a, b, c, d
          a = 10 + 5
          b = 20 - 7
          c = 6 * 7
          d = 100 / 4
        end program test
    "#;

    let vm = compile_and_run(source).expect("Should run successfully");
    assert_eq!(vm.get_variable("A"), Some(&firp::bytecode::Value::Integer(15)));
    assert_eq!(vm.get_variable("B"), Some(&firp::bytecode::Value::Integer(13)));
    assert_eq!(vm.get_variable("C"), Some(&firp::bytecode::Value::Integer(42)));
    assert_eq!(vm.get_variable("D"), Some(&firp::bytecode::Value::Integer(25)));
}

#[test]
fn test_operator_precedence() {
    let source = r#"
        program test
          implicit none
          integer :: result
          result = 2 + 3 * 4
        end program test
    "#;

    let vm = compile_and_run(source).expect("Should run successfully");
    // 2 + (3 * 4) = 2 + 12 = 14
    assert_eq!(vm.get_variable("RESULT"), Some(&firp::bytecode::Value::Integer(14)));
}

#[test]
fn test_parenthesized_expression() {
    let source = r#"
        program test
          implicit none
          integer :: result
          result = (2 + 3) * 4
        end program test
    "#;

    let vm = compile_and_run(source).expect("Should run successfully");
    // (2 + 3) * 4 = 5 * 4 = 20
    assert_eq!(vm.get_variable("RESULT"), Some(&firp::bytecode::Value::Integer(20)));
}

#[test]
fn test_power_operation() {
    let source = r#"
        program test
          implicit none
          integer :: result
          result = 2 ** 10
        end program test
    "#;

    let vm = compile_and_run(source).expect("Should run successfully");
    assert_eq!(vm.get_variable("RESULT"), Some(&firp::bytecode::Value::Integer(1024)));
}

#[test]
fn test_unary_minus() {
    let source = r#"
        program test
          implicit none
          integer :: x, y
          x = 5
          y = -x
        end program test
    "#;

    let vm = compile_and_run(source).expect("Should run successfully");
    assert_eq!(vm.get_variable("Y"), Some(&firp::bytecode::Value::Integer(-5)));
}

#[test]
fn test_logical_operations() {
    let source = r#"
        program test
          implicit none
          logical :: a, b, c, d, e
          a = .true.
          b = .false.
          c = a .and. b
          d = a .or. b
          e = .not. a
        end program test
    "#;

    let vm = compile_and_run(source).expect("Should run successfully");
    assert_eq!(vm.get_variable("C"), Some(&firp::bytecode::Value::Logical(false)));  // true AND false = false
    assert_eq!(vm.get_variable("D"), Some(&firp::bytecode::Value::Logical(true)));   // true OR false = true
    assert_eq!(vm.get_variable("E"), Some(&firp::bytecode::Value::Logical(false)));  // NOT true = false
}

#[test]
fn test_comparison_operations() {
    let source = r#"
        program test
          implicit none
          integer :: x, y
          logical :: gt, lt, eq, ne, ge, le

          x = 10
          y = 5
          gt = x > y
          lt = x < y
          eq = x == y
          ne = x /= y
          ge = x >= y
          le = x <= y
        end program test
    "#;

    let vm = compile_and_run(source).expect("Should run successfully");
    assert_eq!(vm.get_variable("GT"), Some(&firp::bytecode::Value::Logical(true)));   // 10 > 5
    assert_eq!(vm.get_variable("LT"), Some(&firp::bytecode::Value::Logical(false)));  // 10 < 5
    assert_eq!(vm.get_variable("EQ"), Some(&firp::bytecode::Value::Logical(false)));  // 10 == 5
    assert_eq!(vm.get_variable("NE"), Some(&firp::bytecode::Value::Logical(true)));   // 10 /= 5
    assert_eq!(vm.get_variable("GE"), Some(&firp::bytecode::Value::Logical(true)));   // 10 >= 5
    assert_eq!(vm.get_variable("LE"), Some(&firp::bytecode::Value::Logical(false)));  // 10 <= 5
}

#[test]
fn test_if_statement() {
    let source = r#"
        program test
          implicit none
          integer :: x, result

          x = 10
          result = 0

          if (x > 5) then
            result = 1
          end if
        end program test
    "#;

    let vm = compile_and_run(source).expect("Should run successfully");
    assert_eq!(vm.get_variable("RESULT"), Some(&firp::bytecode::Value::Integer(1)));
}

#[test]
fn test_if_else_statement() {
    let source = r#"
        program test
          implicit none
          integer :: x, result

          x = 3

          if (x > 5) then
            result = 1
          else
            result = -1
          end if
        end program test
    "#;

    let vm = compile_and_run(source).expect("Should run successfully");
    assert_eq!(vm.get_variable("RESULT"), Some(&firp::bytecode::Value::Integer(-1)));
}

#[test]
fn test_if_elseif_else() {
    let source = r#"
        program test
          implicit none
          integer :: x, result

          x = 50

          if (x < 10) then
            result = 1
          else if (x < 100) then
            result = 2
          else
            result = 3
          end if
        end program test
    "#;

    let vm = compile_and_run(source).expect("Should run successfully");
    assert_eq!(vm.get_variable("RESULT"), Some(&firp::bytecode::Value::Integer(2)));
}

#[test]
fn test_do_loop() {
    let source = r#"
        program test
          implicit none
          integer :: i, sum

          sum = 0
          do i = 1, 10
            sum = sum + i
          end do
        end program test
    "#;

    let vm = compile_and_run(source).expect("Should run successfully");
    // Sum of 1 to 10 = 55
    assert_eq!(vm.get_variable("SUM"), Some(&firp::bytecode::Value::Integer(55)));
}

#[test]
fn test_do_loop_with_step() {
    let source = r#"
        program test
          implicit none
          integer :: i, sum

          sum = 0
          do i = 1, 10, 2
            sum = sum + i
          end do
        end program test
    "#;

    let vm = compile_and_run(source).expect("Should run successfully");
    // 1 + 3 + 5 + 7 + 9 = 25
    assert_eq!(vm.get_variable("SUM"), Some(&firp::bytecode::Value::Integer(25)));
}

#[test]
fn test_nested_loops() {
    let source = r#"
        program test
          implicit none
          integer :: i, j, count

          count = 0
          do i = 1, 3
            do j = 1, 4
              count = count + 1
            end do
          end do
        end program test
    "#;

    let vm = compile_and_run(source).expect("Should run successfully");
    // 3 * 4 = 12 iterations
    assert_eq!(vm.get_variable("COUNT"), Some(&firp::bytecode::Value::Integer(12)));
}

#[test]
fn test_do_while_loop() {
    let source = r#"
        program test
          implicit none
          integer :: i
          logical :: flag

          i = 1
          flag = i <= 5

          do while (flag)
            i = i + 1
            flag = i <= 5
          end do
        end program test
    "#;

    let vm = compile_and_run(source).expect("Should run successfully");
    assert_eq!(vm.get_variable("I"), Some(&firp::bytecode::Value::Integer(6)));
}

#[test]
fn test_print_output() {
    let source = r#"
        program test
          implicit none
          integer :: x

          x = 42
          print *, 'The answer is', x
        end program test
    "#;

    let vm = compile_and_run(source).expect("Should run successfully");
    let output = vm.output();
    assert_eq!(output.len(), 1);
    assert!(output[0].contains("42"));
    assert!(output[0].contains("The answer is"));
}

#[test]
fn test_select_case_single() {
    let source = r#"
        program test
          implicit none
          integer :: x, result

          x = 2
          select case (x)
            case (1)
              result = 10
            case (2)
              result = 20
            case (3)
              result = 30
            case default
              result = 0
          end select
        end program test
    "#;

    let vm = compile_and_run(source).expect("Should run successfully");
    assert_eq!(vm.get_variable("RESULT"), Some(&firp::bytecode::Value::Integer(20)));
}

#[test]
fn test_select_case_default() {
    let source = r#"
        program test
          implicit none
          integer :: x, result

          x = 99
          select case (x)
            case (1)
              result = 10
            case (2)
              result = 20
            case default
              result = -1
          end select
        end program test
    "#;

    let vm = compile_and_run(source).expect("Should run successfully");
    assert_eq!(vm.get_variable("RESULT"), Some(&firp::bytecode::Value::Integer(-1)));
}

#[test]
fn test_mixed_type_arithmetic() {
    let source = r#"
        program test
          implicit none
          real :: r

          r = 5 + 2.5
        end program test
    "#;

    let vm = compile_and_run(source).expect("Should run successfully");
    assert_eq!(vm.get_variable("R"), Some(&firp::bytecode::Value::Real(7.5)));
}

#[test]
fn test_complex_expression() {
    let source = r#"
        program test
          implicit none
          integer :: a, b, c, result

          a = 2
          b = 3
          c = 4
          result = (a + b) * c - a ** 2
        end program test
    "#;

    let vm = compile_and_run(source).expect("Should run successfully");
    // (2 + 3) * 4 - 2^2 = 5 * 4 - 4 = 20 - 4 = 16
    assert_eq!(vm.get_variable("RESULT"), Some(&firp::bytecode::Value::Integer(16)));
}

#[test]
fn test_fibonacci_loop() {
    let source = r#"
        program test
          implicit none
          integer :: i, fib_prev, fib_curr, temp

          fib_prev = 0
          fib_curr = 1

          do i = 2, 10
            temp = fib_curr
            fib_curr = fib_prev + fib_curr
            fib_prev = temp
          end do
        end program test
    "#;

    let vm = compile_and_run(source).expect("Should run successfully");
    // Fibonacci(10) = 55
    assert_eq!(vm.get_variable("FIB_CURR"), Some(&firp::bytecode::Value::Integer(55)));
}

#[test]
fn test_sprint_success_criteria() {
    // The exact program from the sprint success criteria
    let source = r#"
        program bytecode_test
          implicit none
          integer :: x, y, z

          x = 5
          y = 10
          z = x + y * 2
        end program bytecode_test
    "#;

    let vm = compile_and_run(source).expect("Should run successfully");

    // Verify the computation: x = 5, y = 10, z = 5 + 10 * 2 = 5 + 20 = 25
    assert_eq!(vm.get_variable("X"), Some(&firp::bytecode::Value::Integer(5)));
    assert_eq!(vm.get_variable("Y"), Some(&firp::bytecode::Value::Integer(10)));
    assert_eq!(vm.get_variable("Z"), Some(&firp::bytecode::Value::Integer(25)));
}

#[test]
fn test_elaborate_program() {
    let source = r#"
        program elaborate
          implicit none
          integer :: i, sum
          logical :: flag

          sum = 0
          do i = 1, 10
            if (i > 5) then
              sum = sum + i * 2
            else
              sum = sum + i
            end if
          end do

          flag = sum > 50
        end program elaborate
    "#;

    let vm = compile_and_run(source).expect("Should run successfully");

    // For i = 1 to 5: sum += i -> 1 + 2 + 3 + 4 + 5 = 15
    // For i = 6 to 10: sum += i * 2 -> 12 + 14 + 16 + 18 + 20 = 80
    // Total: 15 + 80 = 95
    assert_eq!(vm.get_variable("SUM"), Some(&firp::bytecode::Value::Integer(95)));
    assert_eq!(vm.get_variable("FLAG"), Some(&firp::bytecode::Value::Logical(true)));
}

#[test]
fn test_trace_mode() {
    let source = r#"
        program test
          implicit none
          integer :: x
          x = 42
        end program test
    "#;

    let chunk = compile_program(source).expect("Should compile successfully");

    let mut vm = VM::new();
    vm.set_trace(true);

    // Should not panic even with trace enabled
    vm.run(chunk).expect("Should run successfully");

    assert_eq!(vm.get_variable("X"), Some(&firp::bytecode::Value::Integer(42)));
}
