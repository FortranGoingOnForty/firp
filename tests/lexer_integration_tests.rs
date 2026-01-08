//! Integration tests for the lexer

use firp::lexer::{Lexer, TokenType};

#[test]
fn test_simple_program() {
    let source = r#"
        program test
          implicit none
          integer :: x, y, z
          x = 5
          y = 10
          z = x + y * 2
        end program test
    "#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().expect("Should tokenize successfully");

    // Verify we got tokens
    assert!(tokens.len() > 0);

    // Verify first token is PROGRAM
    assert_eq!(tokens[0].token_type, TokenType::Program);

    // Verify last token is EOF
    assert_eq!(tokens.last().unwrap().token_type, TokenType::Eof);
}

#[test]
fn test_arithmetic_expression() {
    let source = "x = (a + b) * (c - d) / e ** 2";

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().expect("Should tokenize successfully");

    // Check for expected operators
    let has_plus = tokens.iter().any(|t| t.token_type == TokenType::Plus);
    let has_minus = tokens.iter().any(|t| t.token_type == TokenType::Minus);
    let has_star = tokens.iter().any(|t| t.token_type == TokenType::Star);
    let has_slash = tokens.iter().any(|t| t.token_type == TokenType::Slash);
    let has_power = tokens.iter().any(|t| t.token_type == TokenType::Power);

    assert!(has_plus, "Should have + operator");
    assert!(has_minus, "Should have - operator");
    assert!(has_star, "Should have * operator");
    assert!(has_slash, "Should have / operator");
    assert!(has_power, "Should have ** operator");
}

#[test]
fn test_if_statement() {
    let source = r#"
        IF (x < 10) THEN
          PRINT *, 'Small'
        ELSE IF (x == 10) THEN
          PRINT *, 'Equal'
        ELSE
          PRINT *, 'Large'
        END IF
    "#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().expect("Should tokenize successfully");

    // Count IF keywords
    let if_count = tokens
        .iter()
        .filter(|t| t.token_type == TokenType::If)
        .count();
    assert_eq!(if_count, 3, "Should have 3 IF keywords (IF, ELSE IF, END IF)");

    // Check for THEN
    let then_count = tokens
        .iter()
        .filter(|t| t.token_type == TokenType::Then)
        .count();
    assert_eq!(then_count, 2, "Should have 2 THEN keywords");
}

#[test]
fn test_do_loop() {
    let source = r#"
        DO i = 1, 10, 2
          sum = sum + i
        END DO
    "#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().expect("Should tokenize successfully");

    // Verify DO keyword
    let has_do = tokens.iter().any(|t| t.token_type == TokenType::Do);
    assert!(has_do, "Should have DO keyword");

    // Verify commas
    let comma_count = tokens
        .iter()
        .filter(|t| t.token_type == TokenType::Comma)
        .count();
    assert_eq!(comma_count, 2, "Should have 2 commas in loop bounds");
}

#[test]
fn test_function_declaration() {
    let source = r#"
        FUNCTION add(a, b) RESULT(c)
          INTEGER, INTENT(IN) :: a, b
          INTEGER :: c
          c = a + b
        END FUNCTION add
    "#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().expect("Should tokenize successfully");

    // Verify FUNCTION keyword
    let has_function = tokens.iter().any(|t| t.token_type == TokenType::Function);
    assert!(has_function, "Should have FUNCTION keyword");

    // Verify RESULT keyword
    let has_result = tokens.iter().any(|t| t.token_type == TokenType::Result);
    assert!(has_result, "Should have RESULT keyword");

    // Verify INTENT keyword
    let has_intent = tokens.iter().any(|t| t.token_type == TokenType::Intent);
    assert!(has_intent, "Should have INTENT keyword");
}

#[test]
fn test_array_declaration() {
    let source = "REAL, DIMENSION(10, 20) :: matrix";

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().expect("Should tokenize successfully");

    // Verify DIMENSION keyword
    let has_dimension = tokens.iter().any(|t| t.token_type == TokenType::Dimension);
    assert!(has_dimension, "Should have DIMENSION keyword");

    // Verify double colon
    let has_double_colon = tokens
        .iter()
        .any(|t| t.token_type == TokenType::DoubleColon);
    assert!(has_double_colon, "Should have :: token");
}

#[test]
fn test_logical_expression() {
    let source = "flag = .TRUE. .AND. (x > 5 .OR. y < 10) .AND. .NOT. done";

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().expect("Should tokenize successfully");

    // Check logical operators
    let has_true = tokens.iter().any(|t| t.token_type == TokenType::True);
    let has_and = tokens.iter().any(|t| t.token_type == TokenType::And);
    let has_or = tokens.iter().any(|t| t.token_type == TokenType::Or);
    let has_not = tokens.iter().any(|t| t.token_type == TokenType::Not);

    assert!(has_true, "Should have .TRUE.");
    assert!(has_and, "Should have .AND.");
    assert!(has_or, "Should have .OR.");
    assert!(has_not, "Should have .NOT.");
}

#[test]
fn test_module_declaration() {
    let source = r#"
        MODULE my_module
          IMPLICIT NONE
          PUBLIC :: my_function
          PRIVATE :: helper
        CONTAINS
          SUBROUTINE my_function()
          END SUBROUTINE
        END MODULE
    "#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().expect("Should tokenize successfully");

    // Verify MODULE keyword
    let has_module = tokens.iter().any(|t| t.token_type == TokenType::Module);
    assert!(has_module, "Should have MODULE keyword");

    // Verify PUBLIC/PRIVATE
    let has_public = tokens.iter().any(|t| t.token_type == TokenType::Public);
    let has_private = tokens.iter().any(|t| t.token_type == TokenType::Private);
    assert!(has_public, "Should have PUBLIC keyword");
    assert!(has_private, "Should have PRIVATE keyword");

    // Verify CONTAINS
    let has_contains = tokens.iter().any(|t| t.token_type == TokenType::Contains);
    assert!(has_contains, "Should have CONTAINS keyword");
}

#[test]
fn test_complex_number_literals() {
    let source = "1.0 2.5E10 3.14D-5 42 123_INT64 1.0_REAL32";

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().expect("Should tokenize successfully");

    // Count numeric literals (excluding EOF)
    let num_count = tokens.iter().filter(|t| {
        matches!(t.token_type, TokenType::IntegerLiteral(_) | TokenType::RealLiteral(_))
    }).count();

    assert_eq!(num_count, 6, "Should have 6 numeric literals");
}

#[test]
fn test_string_with_escaped_quotes() {
    let source = r#"message = 'It''s a test'"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().expect("Should tokenize successfully");

    // Find the string token
    let string_token = tokens
        .iter()
        .find(|t| matches!(t.token_type, TokenType::StringLiteral(_)));

    assert!(string_token.is_some(), "Should have a string literal");

    if let Some(token) = string_token {
        if let TokenType::StringLiteral(s) = &token.token_type {
            assert_eq!(s, "It's a test", "Should handle escaped quotes");
        }
    }
}

#[test]
fn test_select_case_statement() {
    let source = r#"
        SELECT CASE (x)
          CASE (1:10)
            PRINT *, 'Small'
          CASE (11:100)
            PRINT *, 'Medium'
          CASE DEFAULT
            PRINT *, 'Large'
        END SELECT
    "#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().expect("Should tokenize successfully");

    // Verify SELECT and CASE keywords
    let has_select = tokens.iter().any(|t| t.token_type == TokenType::Select);
    let has_case = tokens.iter().any(|t| t.token_type == TokenType::Case);
    let has_default = tokens.iter().any(|t| t.token_type == TokenType::Default);

    assert!(has_select, "Should have SELECT keyword");
    assert!(has_case, "Should have CASE keyword");
    assert!(has_default, "Should have DEFAULT keyword");
}
