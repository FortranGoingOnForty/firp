//! Integration tests for the parser

use firp::lexer::Lexer;
use firp::parser::Parser;
use firp::ast::*;

fn parse_program(source: &str) -> Result<Program, Box<dyn std::error::Error>> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize()?;
    let mut parser = Parser::new(tokens);
    Ok(parser.parse_program()?)
}

#[test]
fn test_parse_minimal_program() {
    let source = r#"
        program minimal
        end program minimal
    "#;

    let program = parse_program(source).expect("Should parse successfully");
    assert_eq!(program.name, Some("MINIMAL".to_string()));
    assert_eq!(program.declarations.len(), 0);
    assert_eq!(program.statements.len(), 0);
}

#[test]
fn test_parse_implicit_none() {
    let source = r#"
        program test
          implicit none
        end program test
    "#;

    let program = parse_program(source).expect("Should parse successfully");
    assert_eq!(program.declarations.len(), 1);
    assert!(matches!(program.declarations[0], Declaration::ImplicitNone { .. }));
}

#[test]
fn test_parse_integer_declaration() {
    let source = r#"
        program test
          integer :: x, y, z
        end program test
    "#;

    let program = parse_program(source).expect("Should parse successfully");
    assert_eq!(program.declarations.len(), 1);

    match &program.declarations[0] {
        Declaration::Variable { type_spec, names, .. } => {
            assert!(matches!(type_spec, TypeSpec::Integer { .. }));
            assert_eq!(names.len(), 3);
            assert_eq!(names[0], "X");
            assert_eq!(names[1], "Y");
            assert_eq!(names[2], "Z");
        }
        _ => panic!("Expected Variable declaration"),
    }
}

#[test]
fn test_parse_real_declaration() {
    let source = r#"
        program test
          real :: temperature
        end program test
    "#;

    let program = parse_program(source).expect("Should parse successfully");
    match &program.declarations[0] {
        Declaration::Variable { type_spec, names, .. } => {
            assert!(matches!(type_spec, TypeSpec::Real { .. }));
            assert_eq!(names[0], "TEMPERATURE");
        }
        _ => panic!("Expected Variable declaration"),
    }
}

#[test]
fn test_parse_double_precision_declaration() {
    let source = r#"
        program test
          double precision :: pi
        end program test
    "#;

    let program = parse_program(source).expect("Should parse successfully");
    match &program.declarations[0] {
        Declaration::Variable { type_spec, names, .. } => {
            assert!(matches!(type_spec, TypeSpec::DoublePrecision));
            assert_eq!(names[0], "PI");
        }
        _ => panic!("Expected Variable declaration"),
    }
}

#[test]
fn test_parse_declaration_with_initialization() {
    let source = r#"
        program test
          integer :: x = 5, y = 10
        end program test
    "#;

    let program = parse_program(source).expect("Should parse successfully");
    match &program.declarations[0] {
        Declaration::Variable {
            names, init, ..
        } => {
            assert_eq!(names.len(), 2);
            assert!(init.is_some());
            let init_values = init.as_ref().unwrap();
            assert!(init_values[0].is_some());
            assert!(init_values[1].is_some());
        }
        _ => panic!("Expected Variable declaration"),
    }
}

#[test]
fn test_parse_simple_assignment() {
    let source = r#"
        program test
          integer :: x
          x = 42
        end program test
    "#;

    let program = parse_program(source).expect("Should parse successfully");
    assert_eq!(program.statements.len(), 1);

    match &program.statements[0] {
        Statement::Assignment { target, value, .. } => {
            assert_eq!(target, "X");
            assert!(matches!(value, Expr::IntegerLiteral(42, _)));
        }
        _ => panic!("Expected Assignment statement"),
    }
}

#[test]
fn test_parse_arithmetic_expression() {
    let source = r#"
        program test
          integer :: result
          result = 5 + 3 * 2
        end program test
    "#;

    let program = parse_program(source).expect("Should parse successfully");
    match &program.statements[0] {
        Statement::Assignment { value, .. } => {
            // Should be parsed as 5 + (3 * 2) due to precedence
            match value {
                Expr::BinaryOp { op, left, right, .. } => {
                    assert_eq!(*op, BinaryOperator::Add);
                    assert!(matches!(**left, Expr::IntegerLiteral(5, _)));
                    assert!(matches!(**right, Expr::BinaryOp { op: BinaryOperator::Multiply, .. }));
                }
                _ => panic!("Expected BinaryOp"),
            }
        }
        _ => panic!("Expected Assignment statement"),
    }
}

#[test]
fn test_parse_power_expression() {
    let source = r#"
        program test
          integer :: x
          x = 2 ** 3 ** 2
        end program test
    "#;

    let program = parse_program(source).expect("Should parse successfully");
    match &program.statements[0] {
        Statement::Assignment { value, .. } => {
            // Should be parsed as 2 ** (3 ** 2) because power is right-associative
            match value {
                Expr::BinaryOp { op, left, right, .. } => {
                    assert_eq!(*op, BinaryOperator::Power);
                    assert!(matches!(**left, Expr::IntegerLiteral(2, _)));
                    assert!(matches!(**right, Expr::BinaryOp { op: BinaryOperator::Power, .. }));
                }
                _ => panic!("Expected BinaryOp"),
            }
        }
        _ => panic!("Expected Assignment statement"),
    }
}

#[test]
fn test_parse_parenthesized_expression() {
    let source = r#"
        program test
          integer :: result
          result = (5 + 3) * 2
        end program test
    "#;

    let program = parse_program(source).expect("Should parse successfully");
    match &program.statements[0] {
        Statement::Assignment { value, .. } => {
            match value {
                Expr::BinaryOp { op, left, .. } => {
                    assert_eq!(*op, BinaryOperator::Multiply);
                    assert!(matches!(**left, Expr::Parenthesized(_, _)));
                }
                _ => panic!("Expected BinaryOp"),
            }
        }
        _ => panic!("Expected Assignment statement"),
    }
}

#[test]
fn test_parse_unary_minus() {
    let source = r#"
        program test
          integer :: x
          x = -5
        end program test
    "#;

    let program = parse_program(source).expect("Should parse successfully");
    match &program.statements[0] {
        Statement::Assignment { value, .. } => {
            assert!(matches!(value, Expr::UnaryOp { op: UnaryOperator::Minus, .. }));
        }
        _ => panic!("Expected Assignment statement"),
    }
}

#[test]
fn test_parse_print_statement() {
    let source = r#"
        program test
          integer :: x
          x = 42
          print *, 'Result:', x
        end program test
    "#;

    let program = parse_program(source).expect("Should parse successfully");
    assert_eq!(program.statements.len(), 2);

    match &program.statements[1] {
        Statement::Print { values, .. } => {
            assert_eq!(values.len(), 2);
            assert!(matches!(values[0], Expr::StringLiteral(_, _)));
            assert!(matches!(values[1], Expr::Identifier(_, _)));
        }
        _ => panic!("Expected Print statement"),
    }
}

#[test]
fn test_parse_complete_program() {
    let source = r#"
        program factorial_calc
          implicit none
          integer :: n, result

          n = 5
          result = n * (n - 1) * (n - 2)

          print *, 'Factorial approximation:', result
        end program factorial_calc
    "#;

    let program = parse_program(source).expect("Should parse successfully");
    assert_eq!(program.name, Some("FACTORIAL_CALC".to_string()));
    assert_eq!(program.declarations.len(), 2); // implicit none + integer declaration
    assert_eq!(program.statements.len(), 3); // 2 assignments + 1 print
}

#[test]
fn test_parse_complex_expression() {
    let source = r#"
        program test
          real :: x
          x = 3.14 * 2.0 ** 2 + 1.0 / 2.0 - 0.5
        end program test
    "#;

    let program = parse_program(source).expect("Should parse successfully");
    assert_eq!(program.statements.len(), 1);
    // Just verify it parses without panicking
}

#[test]
fn test_parse_logical_expression() {
    let source = r#"
        program test
          logical :: flag
          flag = .true. .and. .false.
        end program test
    "#;

    let program = parse_program(source).expect("Should parse successfully");
    match &program.statements[0] {
        Statement::Assignment { value, .. } => {
            assert!(matches!(value, Expr::BinaryOp { op: BinaryOperator::And, .. }));
        }
        _ => panic!("Expected Assignment statement"),
    }
}

#[test]
fn test_parse_relational_expression() {
    let source = r#"
        program test
          logical :: result
          integer :: x, y
          x = 5
          y = 10
          result = x < y
        end program test
    "#;

    let program = parse_program(source).expect("Should parse successfully");
    match &program.statements[2] {
        Statement::Assignment { value, .. } => {
            assert!(matches!(value, Expr::BinaryOp { op: BinaryOperator::Less, .. }));
        }
        _ => panic!("Expected Assignment statement"),
    }
}

#[test]
fn test_parse_multiple_declarations() {
    let source = r#"
        program test
          implicit none
          integer :: x, y
          real :: a, b, c
          logical :: flag
        end program test
    "#;

    let program = parse_program(source).expect("Should parse successfully");
    assert_eq!(program.declarations.len(), 4);
}

#[test]
fn test_parse_mixed_declarations_and_statements() {
    let source = r#"
        program test
          integer :: x
          real :: y
          x = 10
          y = 3.14
          print *, x, y
        end program test
    "#;

    let program = parse_program(source).expect("Should parse successfully");
    assert_eq!(program.declarations.len(), 2);
    assert_eq!(program.statements.len(), 3);
}

#[test]
fn test_parse_error_unexpected_token() {
    let source = r#"
        program test
          integer :: x
          x = +
        end program test
    "#;

    let result = parse_program(source);
    assert!(result.is_err());
}

#[test]
fn test_parse_error_missing_equals() {
    let source = r#"
        program test
          integer :: x
          x 5
        end program test
    "#;

    let result = parse_program(source);
    assert!(result.is_err());
}

// Array parsing tests

#[test]
fn test_parse_1d_array_declaration() {
    let source = r#"
        program test
          integer :: arr(10)
        end program test
    "#;

    let program = parse_program(source).expect("Should parse successfully");
    assert_eq!(program.declarations.len(), 1);

    if let Declaration::Variable { entities, .. } = &program.declarations[0] {
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "ARR");
        assert!(entities[0].array_spec.is_some());

        let array_spec = entities[0].array_spec.as_ref().unwrap();
        assert_eq!(array_spec.dimensions.len(), 1);
        // Upper bound should be 10
        if let Expr::IntegerLiteral(val, _) = &array_spec.dimensions[0].upper {
            assert_eq!(*val, 10);
        } else {
            panic!("Expected integer literal for upper bound");
        }
    } else {
        panic!("Expected Variable declaration");
    }
}

#[test]
fn test_parse_1d_array_with_explicit_bounds() {
    let source = r#"
        program test
          integer :: arr(0:9)
        end program test
    "#;

    let program = parse_program(source).expect("Should parse successfully");

    if let Declaration::Variable { entities, .. } = &program.declarations[0] {
        let array_spec = entities[0].array_spec.as_ref().unwrap();
        assert_eq!(array_spec.dimensions.len(), 1);

        // Check lower bound is 0
        let lower = array_spec.dimensions[0].lower.as_ref().unwrap();
        if let Expr::IntegerLiteral(val, _) = lower {
            assert_eq!(*val, 0);
        } else {
            panic!("Expected integer literal for lower bound");
        }

        // Check upper bound is 9
        if let Expr::IntegerLiteral(val, _) = &array_spec.dimensions[0].upper {
            assert_eq!(*val, 9);
        } else {
            panic!("Expected integer literal for upper bound");
        }
    } else {
        panic!("Expected Variable declaration");
    }
}

#[test]
fn test_parse_2d_array_declaration() {
    let source = r#"
        program test
          real :: matrix(3, 4)
        end program test
    "#;

    let program = parse_program(source).expect("Should parse successfully");

    if let Declaration::Variable { entities, .. } = &program.declarations[0] {
        let array_spec = entities[0].array_spec.as_ref().unwrap();
        assert_eq!(array_spec.dimensions.len(), 2);
    } else {
        panic!("Expected Variable declaration");
    }
}

#[test]
fn test_parse_mixed_scalar_and_array() {
    let source = r#"
        program test
          integer :: x, arr(5), y
        end program test
    "#;

    let program = parse_program(source).expect("Should parse successfully");

    if let Declaration::Variable { entities, .. } = &program.declarations[0] {
        assert_eq!(entities.len(), 3);

        // x is scalar
        assert!(entities[0].array_spec.is_none());

        // arr is array
        assert!(entities[1].array_spec.is_some());

        // y is scalar
        assert!(entities[2].array_spec.is_none());
    } else {
        panic!("Expected Variable declaration");
    }
}

#[test]
fn test_parse_array_assignment() {
    let source = r#"
        program test
          integer :: arr(10)
          arr(5) = 42
        end program test
    "#;

    let program = parse_program(source).expect("Should parse successfully");
    assert_eq!(program.statements.len(), 1);

    if let Statement::Assignment { target, indices, value, .. } = &program.statements[0] {
        assert_eq!(target, "ARR");
        assert!(indices.is_some());

        let idx = indices.as_ref().unwrap();
        assert_eq!(idx.len(), 1);
        if let Expr::IntegerLiteral(val, _) = &idx[0] {
            assert_eq!(*val, 5);
        } else {
            panic!("Expected integer literal for index");
        }

        if let Expr::IntegerLiteral(val, _) = value {
            assert_eq!(*val, 42);
        } else {
            panic!("Expected integer literal for value");
        }
    } else {
        panic!("Expected Assignment statement");
    }
}

#[test]
fn test_parse_2d_array_assignment() {
    let source = r#"
        program test
          real :: mat(3, 4)
          mat(1, 2) = 3.14
        end program test
    "#;

    let program = parse_program(source).expect("Should parse successfully");

    if let Statement::Assignment { indices, .. } = &program.statements[0] {
        assert!(indices.is_some());
        let idx = indices.as_ref().unwrap();
        assert_eq!(idx.len(), 2);
    } else {
        panic!("Expected Assignment statement");
    }
}

#[test]
fn test_parse_array_with_negative_lower_bound() {
    let source = r#"
        program test
          integer :: arr(-5:5)
        end program test
    "#;

    let program = parse_program(source).expect("Should parse successfully");

    if let Declaration::Variable { entities, .. } = &program.declarations[0] {
        let array_spec = entities[0].array_spec.as_ref().unwrap();

        // Check lower bound is -5 (parsed as unary minus on 5)
        let lower = array_spec.dimensions[0].lower.as_ref().unwrap();
        assert!(matches!(lower, Expr::UnaryOp { op: UnaryOperator::Minus, .. }));
    } else {
        panic!("Expected Variable declaration");
    }
}
