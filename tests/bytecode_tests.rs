//! Integration tests for bytecode compilation

use firp::bytecode::{Compiler, Chunk, OpCode, Value};
use firp::lexer::Lexer;
use firp::parser::Parser;

fn compile_program(source: &str) -> Result<Chunk, String> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().map_err(|e| format!("Lexer error: {}", e))?;

    let mut parser = Parser::new(tokens);
    let program = parser.parse_program().map_err(|e| format!("Parser error: {}", e))?;

    let mut compiler = Compiler::new();
    let chunk = compiler.compile(&program).map_err(|e| format!("Compile error: {}", e))?;

    Ok(chunk)
}

#[test]
fn test_compile_simple_assignment() {
    let source = r#"
        program test
          implicit none
          integer :: x
          x = 5
        end program test
    "#;

    let chunk = compile_program(source).expect("Should compile successfully");

    // Should have:
    // LoadConst (5)
    // StoreVar (x)
    // Halt
    assert!(chunk.instructions.len() >= 3);

    // Find the StoreVar instruction
    let store_var = chunk.instructions.iter().find(|i| i.opcode == OpCode::StoreVar);
    assert!(store_var.is_some());

    // Verify constant pool has 5
    assert_eq!(chunk.constants.get(0), Some(&Value::Integer(5)));
}

#[test]
fn test_compile_arithmetic_expression() {
    let source = r#"
        program test
          implicit none
          integer :: x, y, z
          x = 5
          y = 10
          z = x + y * 2
        end program test
    "#;

    let chunk = compile_program(source).expect("Should compile successfully");

    // Check that we have arithmetic opcodes
    let has_add = chunk.instructions.iter().any(|i| i.opcode == OpCode::Add);
    let has_multiply = chunk.instructions.iter().any(|i| i.opcode == OpCode::Multiply);

    assert!(has_add, "Should have Add instruction");
    assert!(has_multiply, "Should have Multiply instruction");

    // Print disassembly for inspection
    println!("{}", chunk.disassemble("arithmetic"));
}

#[test]
fn test_compile_success_criteria() {
    // Test the exact program from the sprint success criteria
    let source = r#"
        program bytecode_test
          implicit none
          integer :: x, y, z
          x = 5
          y = 10
          z = x + y * 2
        end program bytecode_test
    "#;

    let chunk = compile_program(source).expect("Should compile successfully");

    // Verify instructions are generated
    assert!(chunk.instructions.len() > 5, "Should have multiple instructions");

    // Verify variables are tracked
    assert!(chunk.variables.contains(&"X".to_string()));
    assert!(chunk.variables.contains(&"Y".to_string()));
    assert!(chunk.variables.contains(&"Z".to_string()));

    // Verify constants
    assert!(chunk.constants.constants().iter().any(|v| *v == Value::Integer(5)));
    assert!(chunk.constants.constants().iter().any(|v| *v == Value::Integer(10)));
    assert!(chunk.constants.constants().iter().any(|v| *v == Value::Integer(2)));

    // Verify ends with Halt
    assert_eq!(
        chunk.instructions.last().map(|i| i.opcode),
        Some(OpCode::Halt)
    );

    // Print disassembly for verification
    println!("{}", chunk.disassemble("bytecode_test"));
}

#[test]
fn test_compile_if_statement() {
    let source = r#"
        program test
          implicit none
          integer :: x
          logical :: flag

          x = 5
          flag = x > 0

          if (flag) then
            x = 10
          end if
        end program test
    "#;

    let chunk = compile_program(source).expect("Should compile successfully");

    // Should have conditional jump
    let has_jump_if_false = chunk.instructions.iter().any(|i| i.opcode == OpCode::JumpIfFalse);
    assert!(has_jump_if_false, "Should have JumpIfFalse instruction");

    // Should have comparison
    let has_greater = chunk.instructions.iter().any(|i| i.opcode == OpCode::Greater);
    assert!(has_greater, "Should have Greater instruction");
}

#[test]
fn test_compile_if_else() {
    let source = r#"
        program test
          implicit none
          integer :: x, result

          x = 5

          if (x > 0) then
            result = 1
          else
            result = -1
          end if
        end program test
    "#;

    let chunk = compile_program(source).expect("Should compile successfully");

    // Should have both conditional and unconditional jumps
    let has_jump_if_false = chunk.instructions.iter().any(|i| i.opcode == OpCode::JumpIfFalse);
    let has_jump = chunk.instructions.iter().any(|i| i.opcode == OpCode::Jump);

    assert!(has_jump_if_false, "Should have JumpIfFalse");
    assert!(has_jump, "Should have Jump");
}

#[test]
fn test_compile_do_loop() {
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

    let chunk = compile_program(source).expect("Should compile successfully");

    // Should have loop control
    let has_less_equal = chunk.instructions.iter().any(|i| i.opcode == OpCode::LessEqual);
    let has_jump = chunk.instructions.iter().any(|i| i.opcode == OpCode::Jump);
    let has_jump_if_false = chunk.instructions.iter().any(|i| i.opcode == OpCode::JumpIfFalse);

    assert!(has_less_equal, "Should have LessEqual (loop condition)");
    assert!(has_jump, "Should have Jump (back to loop start)");
    assert!(has_jump_if_false, "Should have JumpIfFalse (exit loop)");
}

#[test]
fn test_compile_do_while_loop() {
    let source = r#"
        program test
          implicit none
          integer :: i
          logical :: flag

          i = 1
          flag = i <= 10

          do while (flag)
            i = i + 1
            flag = i <= 10
          end do
        end program test
    "#;

    let chunk = compile_program(source).expect("Should compile successfully");

    // Should have loop control
    let has_jump = chunk.instructions.iter().any(|i| i.opcode == OpCode::Jump);
    let has_jump_if_false = chunk.instructions.iter().any(|i| i.opcode == OpCode::JumpIfFalse);

    assert!(has_jump, "Should have Jump (back to loop start)");
    assert!(has_jump_if_false, "Should have JumpIfFalse (exit loop)");
}

#[test]
fn test_compile_print_statement() {
    let source = r#"
        program test
          implicit none
          integer :: x

          x = 42
          print *, 'Value:', x
        end program test
    "#;

    let chunk = compile_program(source).expect("Should compile successfully");

    // Should have Print instruction
    let print_instr = chunk.instructions.iter().find(|i| i.opcode == OpCode::Print);
    assert!(print_instr.is_some(), "Should have Print instruction");

    // Print should have operand = 2 (two values)
    assert_eq!(print_instr.unwrap().operand, Some(2));
}

#[test]
fn test_compile_logical_operations() {
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

    let chunk = compile_program(source).expect("Should compile successfully");

    // Should have logical operations
    let has_and = chunk.instructions.iter().any(|i| i.opcode == OpCode::And);
    let has_or = chunk.instructions.iter().any(|i| i.opcode == OpCode::Or);
    let has_not = chunk.instructions.iter().any(|i| i.opcode == OpCode::Not);
    let has_load_true = chunk.instructions.iter().any(|i| i.opcode == OpCode::LoadTrue);
    let has_load_false = chunk.instructions.iter().any(|i| i.opcode == OpCode::LoadFalse);

    assert!(has_and, "Should have And instruction");
    assert!(has_or, "Should have Or instruction");
    assert!(has_not, "Should have Not instruction");
    assert!(has_load_true, "Should have LoadTrue instruction");
    assert!(has_load_false, "Should have LoadFalse instruction");
}

#[test]
fn test_compile_relational_operations() {
    let source = r#"
        program test
          implicit none
          integer :: x, y
          logical :: result

          x = 5
          y = 10

          result = x == y
          result = x /= y
          result = x < y
          result = x <= y
          result = x > y
          result = x >= y
        end program test
    "#;

    let chunk = compile_program(source).expect("Should compile successfully");

    // Should have all relational operations
    let has_equal = chunk.instructions.iter().any(|i| i.opcode == OpCode::Equal);
    let has_not_equal = chunk.instructions.iter().any(|i| i.opcode == OpCode::NotEqual);
    let has_less = chunk.instructions.iter().any(|i| i.opcode == OpCode::Less);
    let has_less_equal = chunk.instructions.iter().any(|i| i.opcode == OpCode::LessEqual);
    let has_greater = chunk.instructions.iter().any(|i| i.opcode == OpCode::Greater);
    let has_greater_equal = chunk.instructions.iter().any(|i| i.opcode == OpCode::GreaterEqual);

    assert!(has_equal, "Should have Equal instruction");
    assert!(has_not_equal, "Should have NotEqual instruction");
    assert!(has_less, "Should have Less instruction");
    assert!(has_less_equal, "Should have LessEqual instruction");
    assert!(has_greater, "Should have Greater instruction");
    assert!(has_greater_equal, "Should have GreaterEqual instruction");
}

#[test]
fn test_compile_unary_operations() {
    let source = r#"
        program test
          implicit none
          integer :: x, y

          x = 5
          y = -x
        end program test
    "#;

    let chunk = compile_program(source).expect("Should compile successfully");

    // Should have Negate instruction
    let has_negate = chunk.instructions.iter().any(|i| i.opcode == OpCode::Negate);
    assert!(has_negate, "Should have Negate instruction");
}

#[test]
fn test_compile_power_operation() {
    let source = r#"
        program test
          implicit none
          integer :: x, y

          x = 2
          y = x ** 3
        end program test
    "#;

    let chunk = compile_program(source).expect("Should compile successfully");

    // Should have Power instruction
    let has_power = chunk.instructions.iter().any(|i| i.opcode == OpCode::Power);
    assert!(has_power, "Should have Power instruction");
}

#[test]
fn test_constant_deduplication() {
    let source = r#"
        program test
          implicit none
          integer :: a, b, c

          a = 5
          b = 5
          c = 5
        end program test
    "#;

    let chunk = compile_program(source).expect("Should compile successfully");

    // Should only have one constant (5) due to deduplication
    let five_count = chunk.constants.constants().iter().filter(|v| **v == Value::Integer(5)).count();
    assert_eq!(five_count, 1, "Should deduplicate constant 5");
}

#[test]
fn test_compile_nested_expressions() {
    let source = r#"
        program test
          implicit none
          integer :: a, b, c, result

          a = 1
          b = 2
          c = 3
          result = (a + b) * (c - a)
        end program test
    "#;

    let chunk = compile_program(source).expect("Should compile successfully");

    // Should have arithmetic operations
    let has_add = chunk.instructions.iter().any(|i| i.opcode == OpCode::Add);
    let has_subtract = chunk.instructions.iter().any(|i| i.opcode == OpCode::Subtract);
    let has_multiply = chunk.instructions.iter().any(|i| i.opcode == OpCode::Multiply);

    assert!(has_add, "Should have Add instruction");
    assert!(has_subtract, "Should have Subtract instruction");
    assert!(has_multiply, "Should have Multiply instruction");
}

#[test]
fn test_disassembler_output() {
    let source = r#"
        program test
          implicit none
          integer :: x
          x = 42
        end program test
    "#;

    let chunk = compile_program(source).expect("Should compile successfully");
    let output = chunk.disassemble("test");

    // Check disassembler output format
    assert!(output.contains("== test =="));
    assert!(output.contains("Constants:"));
    assert!(output.contains("42"));
    assert!(output.contains("Variables:"));
    assert!(output.contains("X"));
    assert!(output.contains("Instructions:"));
    assert!(output.contains("LoadConst"));
    assert!(output.contains("StoreVar"));
    assert!(output.contains("Halt"));
}

#[test]
fn test_compile_select_case() {
    let source = r#"
        program test
          implicit none
          integer :: x, result

          x = 5
          select case (x)
            case (1)
              result = 10
            case (2, 3, 4)
              result = 20
            case default
              result = 0
          end select
        end program test
    "#;

    let chunk = compile_program(source).expect("Should compile successfully");

    // Should have comparison and jump instructions
    let has_equal = chunk.instructions.iter().any(|i| i.opcode == OpCode::Equal);
    let has_dup = chunk.instructions.iter().any(|i| i.opcode == OpCode::Dup);

    assert!(has_equal, "Should have Equal instruction for case matching");
    assert!(has_dup, "Should have Dup instruction for preserving selector");
}

#[test]
fn test_compile_complex_program() {
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
          print *, 'Sum:', sum, 'Flag:', flag
        end program elaborate
    "#;

    let chunk = compile_program(source).expect("Should compile successfully");

    // Verify complex control flow
    assert!(chunk.instructions.len() > 20, "Complex program should have many instructions");

    // Print disassembly
    println!("{}", chunk.disassemble("elaborate"));
}
