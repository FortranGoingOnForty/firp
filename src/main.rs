//! FIRP - Fortran Interpreter
//!
//! Usage:
//!   firp              Start interactive REPL
//!   firp <file>       Execute a Fortran file
//!   firp --help       Show help

use firp::bytecode::Compiler;
use firp::lexer::Lexer;
use firp::parser::Parser;
use firp::repl::Repl;
use firp::vm::VM;
use std::env;
use std::fs;
use std::process;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    let args: Vec<String> = env::args().collect();

    match args.len() {
        1 => {
            // No arguments - start REPL
            run_repl();
        }
        2 => {
            let arg = &args[1];
            match arg.as_str() {
                "--help" | "-h" => print_help(),
                "--version" | "-v" => print_version(),
                _ => {
                    // Assume it's a filename
                    run_file(arg);
                }
            }
        }
        _ => {
            eprintln!("Usage: firp [file]");
            eprintln!("       firp --help");
            process::exit(1);
        }
    }
}

fn print_help() {
    println!("FIRP - Fortran Interpreter v{}", VERSION);
    println!();
    println!("Usage:");
    println!("  firp              Start interactive REPL");
    println!("  firp <file>       Execute a Fortran file");
    println!("  firp --help       Show this help message");
    println!("  firp --version    Show version information");
    println!();
    println!("REPL Commands:");
    println!("  :help             Show REPL help");
    println!("  :quit             Exit the REPL");
    println!("  :vars             Show defined variables");
    println!("  :load <file>      Load and execute a file");
    println!();
    println!("For more information, visit: https://github.com/FortranGoingOnForty/firp");
}

fn print_version() {
    println!("FIRP v{}", VERSION);
}

fn run_repl() {
    match Repl::new() {
        Ok(mut repl) => {
            if let Err(e) = repl.run() {
                eprintln!("REPL error: {}", e);
                process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("Failed to initialize REPL: {}", e);
            process::exit(1);
        }
    }
}

fn run_file(filename: &str) {
    // Read source file
    let source = match fs::read_to_string(filename) {
        Ok(contents) => contents,
        Err(e) => {
            eprintln!("Error reading file '{}': {}", filename, e);
            process::exit(1);
        }
    };

    // Tokenize
    let mut lexer = Lexer::new(&source);
    let tokens = match lexer.tokenize() {
        Ok(tokens) => tokens,
        Err(e) => {
            eprintln!("Lexer error: {}", e);
            process::exit(1);
        }
    };

    // Parse
    let mut parser = Parser::new(tokens);
    let program = match parser.parse_program() {
        Ok(program) => program,
        Err(e) => {
            eprintln!("Parse error: {}", e);
            process::exit(1);
        }
    };

    // Compile
    let mut compiler = Compiler::new();
    let chunk = match compiler.compile(&program) {
        Ok(chunk) => chunk,
        Err(e) => {
            eprintln!("Compile error: {}", e);
            process::exit(1);
        }
    };

    // Execute
    let mut vm = VM::new();
    match vm.run(chunk) {
        Ok(()) => {
            // Print output
            for line in vm.output() {
                println!("{}", line);
            }
        }
        Err(e) => {
            eprintln!("Runtime error: {}", e);
            process::exit(1);
        }
    }
}
