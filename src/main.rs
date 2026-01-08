use firp::lexer::Lexer;

fn main() {
    println!("FIRP - Fortran Interpreter v0.1.0");
    println!("Sprint 01: Lexer implementation\n");

    // Simple test
    let source = r#"
        PROGRAM hello
          INTEGER :: x
          x = 42
        END PROGRAM hello
    "#;

    let mut lexer = Lexer::new(source);
    match lexer.tokenize() {
        Ok(tokens) => {
            println!("Successfully tokenized {} tokens:", tokens.len());
            for token in tokens.iter().take(10) {
                println!("  {}", token);
            }
        }
        Err(e) => {
            eprintln!("Lexer error: {}", e);
        }
    }
}
