//! Tests for subroutine and function parsing

use firp::ast::*;
use firp::lexer::Lexer;
use firp::parser::Parser;

fn parse_program(source: &str) -> Program {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().expect("Lexer error");
    let mut parser = Parser::new(tokens);
    parser.parse_program().expect("Parse error")
}

#[test]
fn test_parse_call_statement() {
    let source = r#"
        program test
          implicit none
          call mysub()
        end program test
    "#;

    let program = parse_program(source);
    assert_eq!(program.statements.len(), 1);

    match &program.statements[0] {
        Statement::Call { name, arguments, .. } => {
            assert_eq!(name, "MYSUB");
            assert!(arguments.is_empty());
        }
        _ => panic!("Expected Call statement"),
    }
}

#[test]
fn test_parse_call_with_arguments() {
    let source = r#"
        program test
          implicit none
          integer :: x, y
          x = 5
          y = 10
          call compute(x, y, 42)
        end program test
    "#;

    let program = parse_program(source);

    // Find the call statement
    let call_stmt = program
        .statements
        .iter()
        .find(|s| matches!(s, Statement::Call { .. }))
        .expect("Should have call statement");

    match call_stmt {
        Statement::Call { name, arguments, .. } => {
            assert_eq!(name, "COMPUTE");
            assert_eq!(arguments.len(), 3);
        }
        _ => panic!("Expected Call statement"),
    }
}

#[test]
fn test_parse_return_statement() {
    let source = r#"
        program test
          implicit none
          return
        end program test
    "#;

    let program = parse_program(source);
    assert_eq!(program.statements.len(), 1);

    match &program.statements[0] {
        Statement::Return { value, .. } => {
            assert!(value.is_none());
        }
        _ => panic!("Expected Return statement"),
    }
}

#[test]
fn test_parse_return_with_value() {
    let source = r#"
        program test
          implicit none
          return 42
        end program test
    "#;

    let program = parse_program(source);
    assert_eq!(program.statements.len(), 1);

    match &program.statements[0] {
        Statement::Return { value, .. } => {
            assert!(value.is_some());
            match value.as_ref().unwrap() {
                Expr::IntegerLiteral(v, _) => assert_eq!(*v, 42),
                _ => panic!("Expected integer literal"),
            }
        }
        _ => panic!("Expected Return statement"),
    }
}

#[test]
fn test_parse_function_call_in_expression() {
    let source = r#"
        program test
          implicit none
          integer :: x
          x = compute(5)
        end program test
    "#;

    let program = parse_program(source);

    // Find the assignment statement
    let assign_stmt = program
        .statements
        .iter()
        .find(|s| matches!(s, Statement::Assignment { .. }))
        .expect("Should have assignment");

    match assign_stmt {
        Statement::Assignment { value, .. } => match value {
            Expr::FunctionCall {
                name, arguments, ..
            } => {
                assert_eq!(name, "COMPUTE");
                assert_eq!(arguments.len(), 1);
            }
            _ => panic!("Expected FunctionCall expression"),
        },
        _ => panic!("Expected Assignment statement"),
    }
}

#[test]
fn test_parse_nested_function_calls() {
    let source = r#"
        program test
          implicit none
          integer :: x
          x = outer(inner(5), 10)
        end program test
    "#;

    let program = parse_program(source);

    let assign_stmt = program
        .statements
        .iter()
        .find(|s| matches!(s, Statement::Assignment { .. }))
        .expect("Should have assignment");

    match assign_stmt {
        Statement::Assignment { value, .. } => match value {
            Expr::FunctionCall {
                name, arguments, ..
            } => {
                assert_eq!(name, "OUTER");
                assert_eq!(arguments.len(), 2);

                // First argument should be inner(5)
                match &arguments[0].value {
                    Expr::FunctionCall { name, .. } => {
                        assert_eq!(name, "INNER");
                    }
                    _ => panic!("Expected nested function call"),
                }
            }
            _ => panic!("Expected FunctionCall expression"),
        },
        _ => panic!("Expected Assignment statement"),
    }
}

#[test]
fn test_parse_simple_subroutine() {
    let source = r#"
        program test
          implicit none
        contains
          subroutine mysub()
            integer :: x
            x = 5
          end subroutine mysub
        end program test
    "#;

    let program = parse_program(source);
    assert_eq!(program.procedures.len(), 1);

    match &program.procedures[0] {
        Procedure::Subroutine(sub) => {
            assert_eq!(sub.name, "MYSUB");
            assert!(sub.parameters.is_empty());
            assert_eq!(sub.declarations.len(), 1);
            assert_eq!(sub.body.len(), 1);
        }
        _ => panic!("Expected Subroutine"),
    }
}

#[test]
fn test_parse_subroutine_with_parameters() {
    let source = r#"
        program test
          implicit none
        contains
          subroutine compute(a, b, c)
            integer :: a, b, c
            c = a + b
          end subroutine compute
        end program test
    "#;

    let program = parse_program(source);
    assert_eq!(program.procedures.len(), 1);

    match &program.procedures[0] {
        Procedure::Subroutine(sub) => {
            assert_eq!(sub.name, "COMPUTE");
            assert_eq!(sub.parameters.len(), 3);
            assert_eq!(sub.parameters[0].name, "A");
            assert_eq!(sub.parameters[1].name, "B");
            assert_eq!(sub.parameters[2].name, "C");
        }
        _ => panic!("Expected Subroutine"),
    }
}

#[test]
fn test_parse_simple_function() {
    let source = r#"
        program test
          implicit none
        contains
          function square(x)
            integer :: x
            integer :: square
            square = x * x
          end function square
        end program test
    "#;

    let program = parse_program(source);
    assert_eq!(program.procedures.len(), 1);

    match &program.procedures[0] {
        Procedure::Function(func) => {
            assert_eq!(func.name, "SQUARE");
            assert_eq!(func.parameters.len(), 1);
            assert!(!func.is_recursive);
            assert!(func.result_name.is_none());
        }
        _ => panic!("Expected Function"),
    }
}

#[test]
fn test_parse_function_with_result() {
    let source = r#"
        program test
          implicit none
        contains
          function factorial(n) result(f)
            integer :: n, f
            f = 1
          end function factorial
        end program test
    "#;

    let program = parse_program(source);
    assert_eq!(program.procedures.len(), 1);

    match &program.procedures[0] {
        Procedure::Function(func) => {
            assert_eq!(func.name, "FACTORIAL");
            assert_eq!(func.result_name, Some("F".to_string()));
        }
        _ => panic!("Expected Function"),
    }
}

#[test]
fn test_parse_recursive_function() {
    let source = r#"
        program test
          implicit none
        contains
          recursive function factorial(n) result(f)
            integer :: n, f
            if (n <= 1) then
              f = 1
            else
              f = n * factorial(n - 1)
            end if
          end function factorial
        end program test
    "#;

    let program = parse_program(source);
    assert_eq!(program.procedures.len(), 1);

    match &program.procedures[0] {
        Procedure::Function(func) => {
            assert_eq!(func.name, "FACTORIAL");
            assert!(func.is_recursive);
            assert_eq!(func.result_name, Some("F".to_string()));
        }
        _ => panic!("Expected Function"),
    }
}

#[test]
fn test_parse_multiple_procedures() {
    let source = r#"
        program test
          implicit none
        contains
          subroutine sub1()
          end subroutine sub1

          subroutine sub2(x)
            integer :: x
          end subroutine sub2

          function func1(y)
            integer :: y
            integer :: func1
            func1 = y
          end function func1
        end program test
    "#;

    let program = parse_program(source);
    assert_eq!(program.procedures.len(), 3);

    match &program.procedures[0] {
        Procedure::Subroutine(sub) => assert_eq!(sub.name, "SUB1"),
        _ => panic!("Expected Subroutine"),
    }

    match &program.procedures[1] {
        Procedure::Subroutine(sub) => assert_eq!(sub.name, "SUB2"),
        _ => panic!("Expected Subroutine"),
    }

    match &program.procedures[2] {
        Procedure::Function(func) => assert_eq!(func.name, "FUNC1"),
        _ => panic!("Expected Function"),
    }
}

#[test]
fn test_parse_program_with_call_and_procedure() {
    let source = r#"
        program test
          implicit none
          integer :: result
          call calculate(5, 3, result)
        contains
          subroutine calculate(a, b, c)
            integer :: a, b, c
            c = a * b
          end subroutine calculate
        end program test
    "#;

    let program = parse_program(source);

    // Should have statements and procedures
    assert!(program.statements.len() >= 1);
    assert_eq!(program.procedures.len(), 1);

    // Find the call statement
    let has_call = program.statements.iter().any(|s| {
        matches!(s, Statement::Call { name, .. } if name == "CALCULATE")
    });
    assert!(has_call, "Should have CALL statement");
}

#[test]
fn test_parse_sprint08_success_criteria() {
    // This is the program from the sprint success criteria
    let source = r#"
        PROGRAM sub_func_test
          IMPLICIT NONE
          INTEGER :: result

          CALL calculate(5, 3, result)
          PRINT *, result

          PRINT *, factorial(5)

        CONTAINS

          SUBROUTINE calculate(a, b, c)
            INTEGER :: a, b
            INTEGER :: c
            c = a * b + 2
          END SUBROUTINE calculate

          RECURSIVE FUNCTION factorial(n) RESULT(f)
            INTEGER :: n
            INTEGER :: f
            IF (n <= 1) THEN
              f = 1
            ELSE
              f = n * factorial(n - 1)
            END IF
          END FUNCTION factorial

        END PROGRAM sub_func_test
    "#;

    let program = parse_program(source);

    assert_eq!(program.name, Some("SUB_FUNC_TEST".to_string()));
    assert_eq!(program.procedures.len(), 2);

    // Check subroutine
    match &program.procedures[0] {
        Procedure::Subroutine(sub) => {
            assert_eq!(sub.name, "CALCULATE");
            assert_eq!(sub.parameters.len(), 3);
        }
        _ => panic!("Expected Subroutine"),
    }

    // Check function
    match &program.procedures[1] {
        Procedure::Function(func) => {
            assert_eq!(func.name, "FACTORIAL");
            assert!(func.is_recursive);
            assert_eq!(func.result_name, Some("F".to_string()));
        }
        _ => panic!("Expected Function"),
    }

    // Check that main program has CALL and PRINT statements
    let has_call = program.statements.iter().any(|s| {
        matches!(s, Statement::Call { name, .. } if name == "CALCULATE")
    });
    assert!(has_call);

    // Check for function call in expression (factorial(5))
    let has_func_call = program.statements.iter().any(|s| {
        if let Statement::Print { values, .. } = s {
            values.iter().any(|v| matches!(v, Expr::FunctionCall { name, .. } if name == "FACTORIAL"))
        } else {
            false
        }
    });
    assert!(has_func_call, "Should have factorial function call in PRINT");
}
