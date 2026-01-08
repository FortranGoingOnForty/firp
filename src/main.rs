use firp::lexer::Lexer;
use firp::parser::Parser;

fn main() {
    println!("FIRP - Fortran Interpreter v0.1.0");
    println!("Sprint 02: Parser implementation\n");

    // Simple test program
    let source = r#"
        program arithmetic
          implicit none
          integer :: x, y, z
          real :: a, b

          x = 10
          y = 3
          z = x + y * 2

          a = 3.14
          b = a * 2.0

          print *, 'Result:', z
        end program arithmetic
    "#;

    println!("Source code:");
    println!("{}\n", source);

    // Lex
    let mut lexer = Lexer::new(source);
    let tokens = match lexer.tokenize() {
        Ok(tokens) => tokens,
        Err(e) => {
            eprintln!("Lexer error: {}", e);
            return;
        }
    };

    println!("✓ Lexer: {} tokens\n", tokens.len());

    // Parse
    let mut parser = Parser::new(tokens);
    match parser.parse_program() {
        Ok(program) => {
            println!("✓ Parser: Successfully parsed program");
            println!("  Program name: {}", program.name.as_ref().unwrap_or(&"<unnamed>".to_string()));
            println!("  Declarations: {}", program.declarations.len());
            println!("  Statements: {}", program.statements.len());
            println!("\n{:#?}", program);
        }
        Err(e) => {
            eprintln!("Parser error: {}", e);
        }
    }
}
