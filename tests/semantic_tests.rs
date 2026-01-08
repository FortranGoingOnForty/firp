//! Integration tests for semantic analysis

use firp::lexer::Lexer;
use firp::parser::Parser;
use firp::semantic::{SemanticAnalyzer, SemanticError};

fn analyze_program(source: &str) -> Result<(), Vec<SemanticError>> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().map_err(|e| vec![SemanticError::UndeclaredVariable {
        name: format!("Lexer error: {}", e),
        location: firp::lexer::SourceLocation { line: 0, column: 0 },
    }])?;

    let mut parser = Parser::new(tokens);
    let program = parser.parse_program().map_err(|e| vec![SemanticError::UndeclaredVariable {
        name: format!("Parser error: {}", e),
        location: firp::lexer::SourceLocation { line: 0, column: 0 },
    }])?;

    let mut analyzer = SemanticAnalyzer::new();
    let errors = analyzer.analyze(&program);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[test]
fn test_simple_valid_program() {
    let source = r#"
        program test
          implicit none
          integer :: x, y, z

          x = 5
          y = 10
          z = x + y

          print *, z
        end program test
    "#;

    assert!(analyze_program(source).is_ok());
}

#[test]
fn test_undeclared_variable_with_implicit_none() {
    let source = r#"
        program test
          implicit none
          integer :: x

          x = 5
          y = 10
        end program test
    "#;

    let result = analyze_program(source);
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert_eq!(errors.len(), 1);
    assert!(matches!(errors[0], SemanticError::UndeclaredVariable { .. }));
}

#[test]
fn test_implicit_typing_without_implicit_none() {
    let source = r#"
        program test
          i = 5
          x = 3.14
          print *, i, x
        end program test
    "#;

    // Without IMPLICIT NONE, variables can be implicitly typed
    assert!(analyze_program(source).is_ok());
}

#[test]
fn test_duplicate_declaration() {
    let source = r#"
        program test
          implicit none
          integer :: x
          real :: x
        end program test
    "#;

    let result = analyze_program(source);
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert_eq!(errors.len(), 1);
    assert!(matches!(errors[0], SemanticError::DuplicateDeclaration { .. }));
}

#[test]
fn test_type_mismatch_in_assignment() {
    let source = r#"
        program test
          implicit none
          integer :: x
          logical :: flag

          x = 5
          flag = x
        end program test
    "#;

    let result = analyze_program(source);
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(errors.iter().any(|e| matches!(e, SemanticError::TypeMismatch { .. })));
}

#[test]
fn test_numeric_type_conversion() {
    let source = r#"
        program test
          implicit none
          integer :: i
          real :: r
          double precision :: d

          i = 5
          r = i
          d = r
          i = d
        end program test
    "#;

    // Numeric types can be assigned to each other (with implicit conversion)
    assert!(analyze_program(source).is_ok());
}

#[test]
fn test_arithmetic_operations() {
    let source = r#"
        program test
          implicit none
          integer :: i, j
          real :: r, s

          i = 5
          j = 10
          r = 3.14

          i = i + j
          r = r + s
          r = i + r
        end program test
    "#;

    assert!(analyze_program(source).is_ok());
}

#[test]
fn test_invalid_arithmetic_operation() {
    let source = r#"
        program test
          implicit none
          integer :: x
          logical :: flag

          x = 5
          flag = .true.
          x = x + flag
        end program test
    "#;

    let result = analyze_program(source);
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(errors.iter().any(|e| matches!(e, SemanticError::InvalidOperation { .. })));
}

#[test]
fn test_relational_operations() {
    let source = r#"
        program test
          implicit none
          integer :: x, y
          logical :: flag

          x = 5
          y = 10
          flag = x > y
          flag = x == y
          flag = x /= y
        end program test
    "#;

    assert!(analyze_program(source).is_ok());
}

#[test]
fn test_logical_operations() {
    let source = r#"
        program test
          implicit none
          logical :: a, b, c

          a = .true.
          b = .false.
          c = a .and. b
          c = a .or. b
          c = .not. a
        end program test
    "#;

    assert!(analyze_program(source).is_ok());
}

#[test]
fn test_invalid_logical_operation() {
    let source = r#"
        program test
          implicit none
          integer :: x, y
          logical :: flag

          x = 5
          y = 10
          flag = x .and. y
        end program test
    "#;

    let result = analyze_program(source);
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(errors.iter().any(|e| matches!(e, SemanticError::InvalidOperation { .. })));
}

#[test]
fn test_if_condition_must_be_logical() {
    let source = r#"
        program test
          implicit none
          integer :: x

          x = 5
          if (x) then
            print *, 'x is nonzero'
          end if
        end program test
    "#;

    let result = analyze_program(source);
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(errors.iter().any(|e| matches!(e, SemanticError::TypeMismatch { .. })));
}

#[test]
fn test_if_with_logical_condition() {
    let source = r#"
        program test
          implicit none
          integer :: x
          logical :: flag

          x = 5
          flag = x > 0

          if (flag) then
            print *, 'Positive'
          end if
        end program test
    "#;

    assert!(analyze_program(source).is_ok());
}

#[test]
fn test_do_loop_variable_must_be_integer() {
    let source = r#"
        program test
          implicit none
          real :: r

          do r = 1.0, 10.0
            print *, r
          end do
        end program test
    "#;

    let result = analyze_program(source);
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(errors.iter().any(|e| matches!(e, SemanticError::TypeMismatch { .. })));
}

#[test]
fn test_do_loop_bounds_must_be_integer() {
    let source = r#"
        program test
          implicit none
          integer :: i
          real :: r

          r = 10.0
          do i = 1, r
            print *, i
          end do
        end program test
    "#;

    let result = analyze_program(source);
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(errors.iter().any(|e| matches!(e, SemanticError::TypeMismatch { .. })));
}

#[test]
fn test_do_while_condition_must_be_logical() {
    let source = r#"
        program test
          implicit none
          integer :: i

          i = 1
          do while (i)
            print *, i
            i = i + 1
          end do
        end program test
    "#;

    let result = analyze_program(source);
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(errors.iter().any(|e| matches!(e, SemanticError::TypeMismatch { .. })));
}

#[test]
fn test_valid_do_while_loop() {
    let source = r#"
        program test
          implicit none
          integer :: i
          logical :: flag

          i = 1
          flag = i <= 10

          do while (flag)
            print *, i
            i = i + 1
            flag = i <= 10
          end do
        end program test
    "#;

    assert!(analyze_program(source).is_ok());
}

#[test]
fn test_select_case_type_checking() {
    let source = r#"
        program test
          implicit none
          integer :: x

          x = 5
          select case (x)
            case (1)
              print *, 'One'
            case (2, 3, 4)
              print *, 'Two-Four'
            case (10:20)
              print *, 'Ten-Twenty'
            case default
              print *, 'Other'
          end select
        end program test
    "#;

    assert!(analyze_program(source).is_ok());
}

#[test]
fn test_select_case_type_mismatch() {
    let source = r#"
        program test
          implicit none
          integer :: x
          real :: r

          x = 5
          r = 3.14
          select case (x)
            case (1)
              print *, 'One'
            case (r)
              print *, 'Pi'
          end select
        end program test
    "#;

    let result = analyze_program(source);
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(errors.iter().any(|e| matches!(e, SemanticError::TypeMismatch { .. })));
}

#[test]
fn test_parameter_constant() {
    let source = r#"
        program test
          implicit none
          integer, parameter :: MAX_SIZE = 100
          integer :: array_size

          array_size = MAX_SIZE
        end program test
    "#;

    assert!(analyze_program(source).is_ok());
}

#[test]
fn test_assignment_to_parameter() {
    let source = r#"
        program test
          implicit none
          integer, parameter :: MAX_SIZE = 100

          MAX_SIZE = 200
        end program test
    "#;

    let result = analyze_program(source);
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(errors.iter().any(|e| matches!(e, SemanticError::AssignmentToConstant { .. })));
}

#[test]
fn test_initialization_type_checking() {
    let source = r#"
        program test
          implicit none
          integer :: x = 5
          real :: r = 3.14
          logical :: flag = .true.
        end program test
    "#;

    assert!(analyze_program(source).is_ok());
}

#[test]
fn test_initialization_type_mismatch() {
    let source = r#"
        program test
          implicit none
          integer :: x = .true.
        end program test
    "#;

    let result = analyze_program(source);
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(errors.iter().any(|e| matches!(e, SemanticError::TypeMismatch { .. })));
}

#[test]
fn test_nested_expressions() {
    let source = r#"
        program test
          implicit none
          integer :: a, b, c, d

          a = 1
          b = 2
          c = 3
          d = (a + b) * (c - a)
        end program test
    "#;

    assert!(analyze_program(source).is_ok());
}

#[test]
fn test_complex_control_flow() {
    let source = r#"
        program test
          implicit none
          integer :: i, x, sum

          sum = 0
          do i = 1, 10
            x = i * 2
            if (x > 10) then
              exit
            else
              sum = sum + x
            end if
          end do

          print *, sum
        end program test
    "#;

    assert!(analyze_program(source).is_ok());
}

#[test]
fn test_type_error_from_sprint_plan() {
    let source = r#"
        program semantic_test
          implicit none
          integer :: x
          real :: y
          logical :: flag

          x = 5
          y = 3.14
          flag = x + y
          z = 10
        end program semantic_test
    "#;

    let result = analyze_program(source);
    assert!(result.is_err());
    let errors = result.unwrap_err();

    // Should have 2 errors:
    // 1. Type mismatch: arithmetic result assigned to logical
    // 2. Undeclared variable z
    assert!(errors.len() >= 2);

    assert!(errors.iter().any(|e| matches!(e, SemanticError::TypeMismatch { .. })));
    assert!(errors.iter().any(|e| matches!(e, SemanticError::UndeclaredVariable { .. })));
}

#[test]
fn test_unary_operators() {
    let source = r#"
        program test
          implicit none
          integer :: x, y
          logical :: flag

          x = 5
          y = -x
          y = +x

          flag = .true.
          flag = .not. flag
        end program test
    "#;

    assert!(analyze_program(source).is_ok());
}

#[test]
fn test_invalid_unary_operator() {
    let source = r#"
        program test
          implicit none
          logical :: flag
          integer :: x

          flag = .true.
          x = -flag
        end program test
    "#;

    let result = analyze_program(source);
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(errors.iter().any(|e| matches!(e, SemanticError::InvalidOperation { .. })));
}

#[test]
fn test_multiple_errors_collected() {
    let source = r#"
        program test
          implicit none
          integer :: x
          logical :: flag

          y = 5
          z = 10
          flag = x .and. y
        end program test
    "#;

    let result = analyze_program(source);
    assert!(result.is_err());
    let errors = result.unwrap_err();

    // Should collect multiple errors: undeclared y, undeclared z, invalid operation
    assert!(errors.len() >= 2);
}
