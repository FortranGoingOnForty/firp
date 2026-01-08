//! Integration tests for control flow parsing

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
fn test_parse_simple_if() {
    let source = r#"
        program test
          integer :: x
          x = 5
          if (x > 0) then
            print *, 'Positive'
          end if
        end program test
    "#;

    let program = parse_program(source).expect("Should parse successfully");
    assert_eq!(program.statements.len(), 2);

    match &program.statements[1] {
        Statement::If { condition, then_block, else_if_blocks, else_block, .. } => {
            assert!(matches!(condition, Expr::BinaryOp { op: BinaryOperator::Greater, .. }));
            assert_eq!(then_block.len(), 1);
            assert_eq!(else_if_blocks.len(), 0);
            assert!(else_block.is_none());
        }
        _ => panic!("Expected If statement"),
    }
}

#[test]
fn test_parse_if_else() {
    let source = r#"
        program test
          integer :: x
          if (x > 0) then
            print *, 'Positive'
          else
            print *, 'Non-positive'
          end if
        end program test
    "#;

    let program = parse_program(source).expect("Should parse successfully");
    match &program.statements[0] {
        Statement::If { then_block, else_block, .. } => {
            assert_eq!(then_block.len(), 1);
            assert!(else_block.is_some());
            assert_eq!(else_block.as_ref().unwrap().len(), 1);
        }
        _ => panic!("Expected If statement"),
    }
}

#[test]
fn test_parse_if_elseif_else() {
    let source = r#"
        program test
          integer :: x
          if (x > 0) then
            print *, 'Positive'
          else if (x == 0) then
            print *, 'Zero'
          else
            print *, 'Negative'
          end if
        end program test
    "#;

    let program = parse_program(source).expect("Should parse successfully");
    match &program.statements[0] {
        Statement::If { then_block, else_if_blocks, else_block, .. } => {
            assert_eq!(then_block.len(), 1);
            assert_eq!(else_if_blocks.len(), 1);
            assert!(else_block.is_some());
        }
        _ => panic!("Expected If statement"),
    }
}

#[test]
fn test_parse_multiple_elseif() {
    let source = r#"
        program test
          integer :: x
          if (x > 10) then
            print *, 'Large'
          else if (x > 5) then
            print *, 'Medium'
          else if (x > 0) then
            print *, 'Small'
          else
            print *, 'Non-positive'
          end if
        end program test
    "#;

    let program = parse_program(source).expect("Should parse successfully");
    match &program.statements[0] {
        Statement::If { else_if_blocks, .. } => {
            assert_eq!(else_if_blocks.len(), 2);
        }
        _ => panic!("Expected If statement"),
    }
}

#[test]
fn test_parse_counted_do_loop() {
    let source = r#"
        program test
          integer :: i, sum
          sum = 0
          do i = 1, 10
            sum = sum + i
          end do
        end program test
    "#;

    let program = parse_program(source).expect("Should parse successfully");
    assert_eq!(program.statements.len(), 2);

    match &program.statements[1] {
        Statement::DoLoop { variable, start, end, step, body, .. } => {
            assert_eq!(variable, "I");
            assert!(matches!(start, Expr::IntegerLiteral(1, _)));
            assert!(matches!(end, Expr::IntegerLiteral(10, _)));
            assert!(step.is_none());
            assert_eq!(body.len(), 1);
        }
        _ => panic!("Expected DoLoop statement"),
    }
}

#[test]
fn test_parse_do_loop_with_step() {
    let source = r#"
        program test
          integer :: i
          do i = 1, 10, 2
            print *, i
          end do
        end program test
    "#;

    let program = parse_program(source).expect("Should parse successfully");
    match &program.statements[0] {
        Statement::DoLoop { step, .. } => {
            assert!(step.is_some());
            assert!(matches!(step.as_ref().unwrap(), Expr::IntegerLiteral(2, _)));
        }
        _ => panic!("Expected DoLoop statement"),
    }
}

#[test]
fn test_parse_do_while_loop() {
    let source = r#"
        program test
          integer :: i
          i = 1
          do while (i <= 10)
            print *, i
            i = i + 1
          end do
        end program test
    "#;

    let program = parse_program(source).expect("Should parse successfully");
    match &program.statements[1] {
        Statement::DoWhile { condition, body, .. } => {
            assert!(matches!(condition, Expr::BinaryOp { op: BinaryOperator::LessEqual, .. }));
            assert_eq!(body.len(), 2);
        }
        _ => panic!("Expected DoWhile statement"),
    }
}

#[test]
fn test_parse_infinite_do_loop() {
    let source = r#"
        program test
          integer :: i
          i = 1
          do
            if (i > 10) exit
            i = i + 1
          end do
        end program test
    "#;

    let program = parse_program(source).expect("Should parse successfully");
    match &program.statements[1] {
        Statement::DoInfinite { body, .. } => {
            assert_eq!(body.len(), 2);
        }
        _ => panic!("Expected DoInfinite statement"),
    }
}

#[test]
fn test_parse_exit_statement() {
    let source = r#"
        program test
          integer :: i
          do i = 1, 100
            if (i > 10) exit
          end do
        end program test
    "#;

    let program = parse_program(source).expect("Should parse successfully");
    match &program.statements[0] {
        Statement::DoLoop { body, .. } => {
            assert_eq!(body.len(), 1);
            match &body[0] {
                Statement::If { then_block, .. } => {
                    assert!(matches!(then_block[0], Statement::Exit { .. }));
                }
                _ => panic!("Expected If statement in loop body"),
            }
        }
        _ => panic!("Expected DoLoop statement"),
    }
}

#[test]
fn test_parse_cycle_statement() {
    let source = r#"
        program test
          integer :: i
          do i = 1, 10
            if (i == 5) cycle
            print *, i
          end do
        end program test
    "#;

    let program = parse_program(source).expect("Should parse successfully");
    match &program.statements[0] {
        Statement::DoLoop { body, .. } => {
            match &body[0] {
                Statement::If { then_block, .. } => {
                    assert!(matches!(then_block[0], Statement::Cycle { .. }));
                }
                _ => panic!("Expected If statement"),
            }
        }
        _ => panic!("Expected DoLoop statement"),
    }
}

#[test]
fn test_parse_continue_statement() {
    let source = r#"
        program test
          integer :: i
          do i = 1, 10
            continue
          end do
        end program test
    "#;

    let program = parse_program(source).expect("Should parse successfully");
    match &program.statements[0] {
        Statement::DoLoop { body, .. } => {
            assert!(matches!(body[0], Statement::Continue { .. }));
        }
        _ => panic!("Expected DoLoop statement"),
    }
}

#[test]
fn test_parse_select_case_single_value() {
    let source = r#"
        program test
          integer :: x
          select case (x)
            case (1)
              print *, 'One'
            case (2)
              print *, 'Two'
            case default
              print *, 'Other'
          end select
        end program test
    "#;

    let program = parse_program(source).expect("Should parse successfully");
    match &program.statements[0] {
        Statement::SelectCase { selector, cases, default, .. } => {
            assert!(matches!(selector, Expr::Identifier(_, _)));
            assert_eq!(cases.len(), 2);
            assert!(default.is_some());
            assert!(matches!(cases[0].selector, CaseSelector::Value(_)));
        }
        _ => panic!("Expected SelectCase statement"),
    }
}

#[test]
fn test_parse_select_case_range() {
    let source = r#"
        program test
          integer :: x
          select case (x)
            case (1:10)
              print *, 'Small'
            case (11:100)
              print *, 'Medium'
            case (101:)
              print *, 'Large'
          end select
        end program test
    "#;

    let program = parse_program(source).expect("Should parse successfully");
    match &program.statements[0] {
        Statement::SelectCase { cases, .. } => {
            assert_eq!(cases.len(), 3);
            assert!(matches!(cases[0].selector, CaseSelector::Range(_, _)));
            assert!(matches!(cases[1].selector, CaseSelector::Range(_, _)));
            assert!(matches!(cases[2].selector, CaseSelector::Range(_, _)));
        }
        _ => panic!("Expected SelectCase statement"),
    }
}

#[test]
fn test_parse_select_case_value_list() {
    let source = r#"
        program test
          integer :: day
          select case (day)
            case (1, 7)
              print *, 'Weekend'
            case (2, 3, 4, 5, 6)
              print *, 'Weekday'
          end select
        end program test
    "#;

    let program = parse_program(source).expect("Should parse successfully");
    match &program.statements[0] {
        Statement::SelectCase { cases, .. } => {
            assert_eq!(cases.len(), 2);
            match &cases[0].selector {
                CaseSelector::Values(values) => {
                    assert_eq!(values.len(), 2);
                }
                _ => panic!("Expected Values selector"),
            }
            match &cases[1].selector {
                CaseSelector::Values(values) => {
                    assert_eq!(values.len(), 5);
                }
                _ => panic!("Expected Values selector"),
            }
        }
        _ => panic!("Expected SelectCase statement"),
    }
}

#[test]
fn test_parse_nested_if_in_loop() {
    let source = r#"
        program test
          integer :: i, j
          do i = 1, 10
            if (i > 5) then
              do j = 1, i
                print *, j
              end do
            else
              print *, i
            end if
          end do
        end program test
    "#;

    let program = parse_program(source).expect("Should parse successfully");
    match &program.statements[0] {
        Statement::DoLoop { body, .. } => {
            match &body[0] {
                Statement::If { then_block, else_block, .. } => {
                    assert!(matches!(then_block[0], Statement::DoLoop { .. }));
                    assert!(else_block.is_some());
                }
                _ => panic!("Expected If statement"),
            }
        }
        _ => panic!("Expected DoLoop statement"),
    }
}

#[test]
fn test_parse_nested_loops() {
    let source = r#"
        program test
          integer :: i, j, k
          do i = 1, 10
            do j = 1, 10
              do k = 1, 10
                print *, i * j * k
              end do
            end do
          end do
        end program test
    "#;

    let program = parse_program(source).expect("Should parse successfully");
    assert_eq!(program.statements.len(), 1);

    // Check triple nesting
    match &program.statements[0] {
        Statement::DoLoop { body: outer_body, .. } => {
            match &outer_body[0] {
                Statement::DoLoop { body: middle_body, .. } => {
                    assert!(matches!(middle_body[0], Statement::DoLoop { .. }));
                }
                _ => panic!("Expected nested DoLoop"),
            }
        }
        _ => panic!("Expected DoLoop statement"),
    }
}

#[test]
fn test_parse_complex_control_flow() {
    let source = r#"
        program test
          integer :: i, x, sum
          sum = 0
          do i = 1, 100
            x = i
            if (x < 10) then
              cycle
            else if (x > 90) then
              exit
            else
              select case (x)
                case (10:20)
                  sum = sum + 1
                case (21:50)
                  sum = sum + 2
                case default
                  sum = sum + 3
              end select
            end if
          end do
        end program test
    "#;

    let program = parse_program(source).expect("Should parse successfully");
    assert_eq!(program.statements.len(), 2);

    match &program.statements[1] {
        Statement::DoLoop { body, .. } => {
            assert_eq!(body.len(), 2); // assignment + if statement
            match &body[1] {
                Statement::If { then_block, else_if_blocks, else_block, .. } => {
                    assert!(matches!(then_block[0], Statement::Cycle { .. }));
                    assert_eq!(else_if_blocks.len(), 1);
                    assert!(matches!(else_if_blocks[0].1[0], Statement::Exit { .. }));
                    assert!(else_block.is_some());
                    let else_stmts = else_block.as_ref().unwrap();
                    assert!(matches!(else_stmts[0], Statement::SelectCase { .. }));
                }
                _ => panic!("Expected If statement"),
            }
        }
        _ => panic!("Expected DoLoop statement"),
    }
}

#[test]
fn test_parse_if_without_parentheses() {
    let source = r#"
        program test
          logical :: flag
          if flag then
            print *, 'True'
          end if
        end program test
    "#;

    let program = parse_program(source).expect("Should parse successfully");
    match &program.statements[0] {
        Statement::If { condition, .. } => {
            assert!(matches!(condition, Expr::Identifier(_, _)));
        }
        _ => panic!("Expected If statement"),
    }
}

#[test]
fn test_parse_do_while_without_parentheses() {
    let source = r#"
        program test
          logical :: flag
          flag = .true.
          do while flag
            print *, 'Loop'
            flag = .false.
          end do
        end program test
    "#;

    let program = parse_program(source).expect("Should parse successfully");
    match &program.statements[1] {
        Statement::DoWhile { condition, .. } => {
            assert!(matches!(condition, Expr::Identifier(_, _)));
        }
        _ => panic!("Expected DoWhile statement"),
    }
}

// ========== DO CONCURRENT TESTS (Sprint 15) ==========

#[test]
fn test_parse_do_concurrent_simple() {
    let source = r#"
        program test
          integer :: i
          integer :: arr(10)
          do concurrent (i = 1:10)
            arr(i) = i * 2
          end do
        end program test
    "#;

    let program = parse_program(source).expect("Should parse successfully");
    match &program.statements[0] {
        Statement::DoConcurrent { controls, body, .. } => {
            assert_eq!(controls.len(), 1);
            assert_eq!(controls[0].variable, "I");
            assert_eq!(body.len(), 1);
        }
        _ => panic!("Expected DoConcurrent statement"),
    }
}

#[test]
fn test_parse_do_concurrent_multiple_indices() {
    let source = r#"
        program test
          integer :: i, j
          integer :: matrix(10, 10)
          do concurrent (i = 1:10, j = 1:10)
            matrix(i, j) = i + j
          end do
        end program test
    "#;

    let program = parse_program(source).expect("Should parse successfully");
    match &program.statements[0] {
        Statement::DoConcurrent { controls, .. } => {
            assert_eq!(controls.len(), 2);
            assert_eq!(controls[0].variable, "I");
            assert_eq!(controls[1].variable, "J");
        }
        _ => panic!("Expected DoConcurrent statement"),
    }
}

#[test]
fn test_parse_do_concurrent_with_step() {
    let source = r#"
        program test
          integer :: i
          integer :: arr(10)
          do concurrent (i = 1:10:2)
            arr(i) = i
          end do
        end program test
    "#;

    let program = parse_program(source).expect("Should parse successfully");
    match &program.statements[0] {
        Statement::DoConcurrent { controls, .. } => {
            assert_eq!(controls.len(), 1);
            assert!(controls[0].step.is_some());
        }
        _ => panic!("Expected DoConcurrent statement"),
    }
}

#[test]
fn test_parse_do_concurrent_with_locality() {
    let source = r#"
        program test
          integer :: i, temp, sum
          integer :: arr(10)
          sum = 0
          do concurrent (i = 1:10) local(temp) shared(sum)
            temp = i * 2
            arr(i) = temp
          end do
        end program test
    "#;

    let program = parse_program(source).expect("Should parse successfully");
    match &program.statements[1] {
        Statement::DoConcurrent { controls, locality, .. } => {
            assert_eq!(controls.len(), 1);
            assert_eq!(locality.len(), 2);
            assert!(matches!(&locality[0], LocalitySpec::Local(vars) if vars == &vec!["TEMP".to_string()]));
            assert!(matches!(&locality[1], LocalitySpec::Shared(vars) if vars == &vec!["SUM".to_string()]));
        }
        _ => panic!("Expected DoConcurrent statement"),
    }
}

// ========== SYNC AND CRITICAL TESTS (Sprint 15) ==========

#[test]
fn test_parse_sync_all() {
    let source = r#"
        program test_sync
          implicit none
          sync all
          print *, "synced"
        end program test_sync
    "#;

    let program = parse_program(source).expect("Should parse successfully");
    assert!(matches!(&program.statements[0], Statement::SyncAll { .. }));
}

#[test]
fn test_parse_sync_images() {
    let source = r#"
        program test_sync_images
          implicit none
          sync images (1)
          print *, "synced with image 1"
        end program test_sync_images
    "#;

    let program = parse_program(source).expect("Should parse successfully");
    match &program.statements[0] {
        Statement::SyncImages { images, .. } => {
            assert!(images.is_some());
            assert_eq!(images.as_ref().unwrap().len(), 1);
        }
        _ => panic!("Expected SyncImages statement"),
    }
}

#[test]
fn test_parse_sync_images_all() {
    let source = r#"
        program test_sync_all_images
          implicit none
          sync images (*)
          print *, "synced with all"
        end program test_sync_all_images
    "#;

    let program = parse_program(source).expect("Should parse successfully");
    match &program.statements[0] {
        Statement::SyncImages { images, .. } => {
            assert!(images.is_none()); // * means all images
        }
        _ => panic!("Expected SyncImages statement"),
    }
}

#[test]
fn test_parse_critical_section() {
    let source = r#"
        program test_critical
          implicit none
          integer :: counter
          counter = 0
          critical
            counter = counter + 1
          end critical
          print *, counter
        end program test_critical
    "#;

    let program = parse_program(source).expect("Should parse successfully");
    match &program.statements[1] {
        Statement::Critical { body, .. } => {
            assert_eq!(body.len(), 1);
            assert!(matches!(&body[0], Statement::Assignment { .. }));
        }
        _ => panic!("Expected Critical statement"),
    }
}
