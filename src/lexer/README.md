# Lexer Module

The lexer module tokenizes Modern Fortran source code into a stream of tokens.

## Features

- **Case-insensitive**: Fortran keywords are recognized regardless of case
- **Free-form source**: Supports Modern Fortran free-form syntax
- **Line continuations**: Handles `&` for multi-line statements
- **Comments**: Supports `!` comments (to end of line)
- **Comprehensive token support**: All Modern Fortran keywords and operators
- **Source location tracking**: Every token tracks its line and column for error reporting

## Token Types

### Keywords
- Control flow: `program`, `end`, `if`, `then`, `else`, `do`, `while`, `select`, `case`, etc.
- Declarations: `integer`, `real`, `complex`, `logical`, `character`, `type`, etc.
- Subprograms: `subroutine`, `function`, `return`, `call`, `contains`, etc.
- Modules: `module`, `use`, `public`, `private`, etc.
- I/O: `print`, `read`, `write`, `open`, `close`, `format`, etc.

### Literals
- Integer literals: `42`, `123_INT64`
- Real literals: `3.14`, `2.5E10`, `1.0D-5`, `1.0_REAL32`
- String literals: `'hello'`, `"world"` (with `''` escape for quotes)
- Logical literals: `.true.`, `.false.`

### Operators
- Arithmetic: `+`, `-`, `*`, `/`, `**` (power)
- Relational: `==`, `/=`, `<`, `<=`, `>`, `>=`
- Logical: `.and.`, `.or.`, `.not.`, `.eqv.`, `.neqv.`
- Old-style relational: `.eq.`, `.ne.`, `.lt.`, `.le.`, `.gt.`, `.ge.`

### Delimiters
- `(`, `)`, `[`, `]`, `,`, `:`, `::`, `;`, `.`, `%`, `=>`

## Usage

```rust
use firp::lexer::Lexer;

let source = r#"
    program hello
      integer :: x
      x = 42
    end program hello
"#;

let mut lexer = Lexer::new(source);
let tokens = lexer.tokenize()?;

for token in tokens {
    println!("{}", token);
}
```

## Error Handling

The lexer returns helpful error messages with source locations:

```rust
match lexer.tokenize() {
    Ok(tokens) => { /* process tokens */ },
    Err(LexerError::UnexpectedCharacter(ch, loc)) => {
        eprintln!("Unexpected character '{}' at {}", ch, loc);
    },
    Err(LexerError::InvalidNumber(num, loc)) => {
        eprintln!("Invalid number '{}' at {}", num, loc);
    },
    // ... other errors
}
```

## Testing

Run lexer tests with:
```bash
cargo test lexer
```

See `tests/lexer_integration_tests.rs` for comprehensive examples.
