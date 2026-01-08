# Parser Module

The parser module transforms a stream of tokens from the lexer into an Abstract Syntax Tree (AST).

## Design

The parser uses **hand-written recursive descent** with:
- Explicit operator precedence handling
- Clear error messages with source locations
- Support for keywords used as identifiers (context-sensitive parsing)
- No backtracking - single-pass parsing

## Architecture

### Recursive Descent

Each grammar rule is implemented as a method:
- `parse_program()` - entry point for complete programs
- `parse_declaration()` - type declarations, IMPLICIT NONE
- `parse_statement()` - assignment, PRINT, etc.
- `parse_expression()` - entry point for expressions
- `parse_logical_or()`, `parse_logical_and()`, etc. - operator precedence levels

### Operator Precedence

Expressions are parsed using precedence climbing:

```
Precedence (lowest to highest):
1. .EQV., .NEQV.         (logical equivalence)
2. .OR.                  (logical or)
3. .AND.                 (logical and)
4. ==, /=, <, <=, >, >=  (relational)
5. +, -                  (additive)
6. *, /                  (multiplicative)
7. **                    (power, right-associative)
8. unary +, -, .NOT.     (unary operators)
```

### Error Handling

The parser provides detailed error messages:
- Expected token vs. what was found
- Source location (line and column)
- Context-appropriate error messages

Example:
```
Expected NONE but found Integer at line 3, column 20
```

## Supported Language Features

### Declarations
- `implicit none`
- Type declarations: `integer`, `real`, `double precision`, `complex`, `logical`, `character`
- Multiple variables: `integer :: x, y, z`
- Initialization: `integer :: x = 5`

### Statements
- Assignment: `x = expression`
- PRINT: `print *, value1, value2, ...`

### Expressions
- Literals: integers, reals, strings, logical (.true., .false.)
- Variables: `x`, `temperature`, etc.
- Binary operators: +, -, *, /, **, ==, /=, <, <=, >, >=, .and., .or.
- Unary operators: +, -, .not.
- Parentheses: `(expr)`

### Program Structure
- `program name ... end program name`
- Declaration section followed by executable section

## Usage

```rust
use firp::lexer::Lexer;
use firp::parser::Parser;

let source = r#"
    program test
      integer :: x
      x = 42
    end program test
"#;

// Lex
let mut lexer = Lexer::new(source);
let tokens = lexer.tokenize()?;

// Parse
let mut parser = Parser::new(tokens);
let program = parser.parse_program()?;

println!("Program name: {:?}", program.name);
println!("Declarations: {}", program.declarations.len());
println!("Statements: {}", program.statements.len());
```

## AST Structure

See `src/ast/mod.rs` for complete AST node definitions.

### Key Types

- `Program` - Root node with declarations and statements
- `Declaration` - Variable declarations, IMPLICIT NONE
- `Statement` - Assignment, PRINT
- `Expr` - All expression types
- `TypeSpec` - Fortran types
- `BinaryOperator`, `UnaryOperator` - Operators

## Context-Sensitive Parsing

Fortran allows certain keywords to be used as identifiers in specific contexts. For example, `result` is a keyword (used in `FUNCTION ... RESULT(x)`), but it's also valid as a variable name:

```fortran
program test
  integer :: result   ! 'result' as variable name
  result = 42         ! valid
end program test
```

The parser handles this by checking context when encountering these keywords.

## Testing

Run parser tests:
```bash
cargo test parser
```

Integration tests are in `tests/parser_integration_tests.rs`.

## Future Enhancements

Sprint 03 will add:
- IF/THEN/ELSE statements
- DO loops
- SELECT CASE
- More complex expressions

Later sprints will add:
- Subroutines and functions
- Arrays
- I/O statements
- Modules
- And much more!
