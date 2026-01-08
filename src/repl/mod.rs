//! REPL (Read-Eval-Print-Loop) for interactive Fortran execution

use crate::bytecode::{Chunk, Compiler, Value};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::vm::VM;
use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use rustyline::{DefaultEditor, Editor};
use std::collections::HashMap;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const HISTORY_FILE: &str = ".firp_history";

/// REPL state maintaining persistent context across inputs
pub struct Repl {
    /// Editor for line input with history
    editor: Editor<(), DefaultHistory>,
    /// Persistent VM state
    vm: VM,
    /// Persistent compiler state (for type registry, procedures, etc.)
    compiler: Compiler,
    /// Variable values for :vars command
    variables: HashMap<String, Value>,
    /// Whether to show bytecode for each input
    show_bytecode: bool,
    /// Accumulated multi-line input
    input_buffer: String,
    /// Whether we're in multi-line input mode
    in_multiline: bool,
}

impl Repl {
    /// Create a new REPL instance
    pub fn new() -> Result<Self, String> {
        let editor = DefaultEditor::new().map_err(|e| format!("Failed to create editor: {}", e))?;

        Ok(Repl {
            editor,
            vm: VM::new(),
            compiler: Compiler::new(),
            variables: HashMap::new(),
            show_bytecode: false,
            input_buffer: String::new(),
            in_multiline: false,
        })
    }

    /// Run the REPL main loop
    pub fn run(&mut self) -> Result<(), String> {
        self.print_welcome();
        self.load_history();

        loop {
            let prompt = if self.in_multiline { ".. " } else { ":: " };

            match self.editor.readline(prompt) {
                Ok(line) => {
                    if line.trim().is_empty() && !self.in_multiline {
                        continue;
                    }

                    // Handle REPL commands
                    if !self.in_multiline && line.trim().starts_with(':') {
                        if self.handle_command(&line) {
                            break; // :quit was called
                        }
                        continue;
                    }

                    // Add to history
                    let _ = self.editor.add_history_entry(&line);

                    // Accumulate input
                    if !self.input_buffer.is_empty() {
                        self.input_buffer.push('\n');
                    }
                    self.input_buffer.push_str(&line);

                    // Check if input is complete
                    if self.is_input_complete(&self.input_buffer) {
                        let input = std::mem::take(&mut self.input_buffer);
                        self.in_multiline = false;
                        self.execute_input(&input);
                    } else {
                        self.in_multiline = true;
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    // Ctrl-C - cancel current input
                    if self.in_multiline {
                        println!("^C");
                        self.input_buffer.clear();
                        self.in_multiline = false;
                    } else {
                        println!("^C (use :quit to exit)");
                    }
                }
                Err(ReadlineError::Eof) => {
                    // Ctrl-D - exit
                    println!("Goodbye!");
                    break;
                }
                Err(err) => {
                    eprintln!("Error: {:?}", err);
                    break;
                }
            }
        }

        self.save_history();
        Ok(())
    }

    /// Print welcome message
    fn print_welcome(&self) {
        println!("FIRP v{} - Fortran Interpreter", VERSION);
        println!("Type :help for help, :quit to exit\n");
    }

    /// Load command history from file
    fn load_history(&mut self) {
        if let Some(home) = dirs_home() {
            let history_path = format!("{}/{}", home, HISTORY_FILE);
            let _ = self.editor.load_history(&history_path);
        }
    }

    /// Save command history to file
    fn save_history(&mut self) {
        if let Some(home) = dirs_home() {
            let history_path = format!("{}/{}", home, HISTORY_FILE);
            let _ = self.editor.save_history(&history_path);
        }
    }

    /// Handle REPL commands (starting with :)
    /// Returns true if REPL should exit
    fn handle_command(&mut self, line: &str) -> bool {
        let parts: Vec<&str> = line.trim().splitn(2, ' ').collect();
        let cmd = parts[0].to_lowercase();
        let args = parts.get(1).map(|s| s.trim()).unwrap_or("");

        match cmd.as_str() {
            ":quit" | ":q" | ":exit" => {
                println!("Goodbye!");
                return true;
            }
            ":help" | ":h" | ":?" => {
                self.print_help();
            }
            ":vars" | ":variables" => {
                self.print_variables();
            }
            ":clear" => {
                self.clear_state();
                println!("State cleared.");
            }
            ":reset" => {
                self.reset();
                println!("REPL reset.");
            }
            ":load" | ":l" => {
                if args.is_empty() {
                    println!("Usage: :load <filename>");
                } else {
                    self.load_file(args);
                }
            }
            ":bytecode" | ":bc" => {
                if args.is_empty() {
                    self.show_bytecode = !self.show_bytecode;
                    println!("Bytecode display: {}", if self.show_bytecode { "ON" } else { "OFF" });
                } else {
                    self.show_bytecode_for(args);
                }
            }
            ":type" | ":t" => {
                if args.is_empty() {
                    println!("Usage: :type <expression>");
                } else {
                    self.show_type(args);
                }
            }
            _ => {
                println!("Unknown command: {}. Type :help for available commands.", cmd);
            }
        }
        false
    }

    /// Print help message
    fn print_help(&self) {
        println!("FIRP REPL Commands:");
        println!("  :help, :h, :?     Show this help message");
        println!("  :quit, :q, :exit  Exit the REPL");
        println!("  :vars             Show defined variables");
        println!("  :clear            Clear all variables");
        println!("  :reset            Reset REPL state completely");
        println!("  :load <file>      Load and execute a Fortran file");
        println!("  :bytecode [stmt]  Toggle bytecode display or show bytecode for statement");
        println!("  :type <expr>      Show the type of an expression");
        println!();
        println!("Enter Fortran statements or expressions directly.");
        println!("Multi-line input is supported - continue typing until the statement is complete.");
        println!("Press Ctrl-C to cancel input, Ctrl-D to exit.");
    }

    /// Print all defined variables
    fn print_variables(&self) {
        if self.variables.is_empty() {
            println!("No variables defined.");
            return;
        }

        println!("Defined variables:");
        let mut vars: Vec<_> = self.variables.iter().collect();
        vars.sort_by(|a, b| a.0.cmp(b.0));

        for (name, value) in vars {
            println!("  {} = {}", name, value);
        }
    }

    /// Clear variable state
    fn clear_state(&mut self) {
        self.variables.clear();
        self.vm = VM::new();
    }

    /// Reset REPL completely
    fn reset(&mut self) {
        self.variables.clear();
        self.vm = VM::new();
        self.compiler = Compiler::new();
        self.input_buffer.clear();
        self.in_multiline = false;
    }

    /// Load and execute a file
    fn load_file(&mut self, filename: &str) {
        match std::fs::read_to_string(filename) {
            Ok(contents) => {
                println!("Loading {}...", filename);
                self.execute_input(&contents);
            }
            Err(e) => {
                println!("Error loading file: {}", e);
            }
        }
    }

    /// Show bytecode for a statement
    fn show_bytecode_for(&mut self, input: &str) {
        match self.compile_input(input) {
            Ok(chunk) => {
                println!("{}", chunk.disassemble("input"));
            }
            Err(e) => {
                println!("Compilation error: {}", e);
            }
        }
    }

    /// Show type of an expression
    fn show_type(&self, _input: &str) {
        // TODO: Implement type inference
        println!("Type inference not yet implemented.");
    }

    /// Check if input appears complete (not in the middle of a block)
    fn is_input_complete(&self, input: &str) -> bool {
        let upper = input.to_uppercase();
        let trimmed = upper.trim();

        // Count block openers and closers
        let mut depth = 0;

        // Simple heuristic: count block keywords
        for line in trimmed.lines() {
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('!') {
                continue;
            }

            // Block openers
            if line.starts_with("PROGRAM ") || line == "PROGRAM" {
                depth += 1;
            } else if line.starts_with("SUBROUTINE ") {
                depth += 1;
            } else if line.starts_with("FUNCTION ") || line.contains(" FUNCTION ") {
                depth += 1;
            } else if line.starts_with("MODULE ") && !line.starts_with("MODULE PROCEDURE") {
                depth += 1;
            } else if line.starts_with("IF ") && line.contains(" THEN") {
                depth += 1;
            } else if line.starts_with("DO ") || line == "DO" {
                depth += 1;
            } else if line.starts_with("SELECT ") {
                depth += 1;
            } else if line.starts_with("TYPE ") && line.contains("::") && !line.contains("TYPE(") {
                // TYPE definition, not TYPE(name) declaration
                depth += 1;
            }

            // Block closers
            if line.starts_with("END PROGRAM") || line == "END" && depth == 1 {
                depth -= 1;
            } else if line.starts_with("END SUBROUTINE") {
                depth -= 1;
            } else if line.starts_with("END FUNCTION") {
                depth -= 1;
            } else if line.starts_with("END MODULE") {
                depth -= 1;
            } else if line.starts_with("END IF") || line == "ENDIF" {
                depth -= 1;
            } else if line.starts_with("END DO") || line == "ENDDO" {
                depth -= 1;
            } else if line.starts_with("END SELECT") {
                depth -= 1;
            } else if line.starts_with("END TYPE") {
                depth -= 1;
            }
        }

        // Input is complete if all blocks are closed
        // Also complete for simple statements (depth == 0 from start)
        depth <= 0
    }

    /// Compile input to bytecode
    fn compile_input(&mut self, input: &str) -> Result<Chunk, String> {
        // Check if input contains a program unit (PROGRAM, MODULE, SUBROUTINE, FUNCTION)
        let upper = input.to_uppercase();
        let has_program_unit = upper.contains("PROGRAM ")
            || upper.contains("MODULE ")
            || upper.contains("SUBROUTINE ")
            || upper.contains("FUNCTION ");

        if has_program_unit {
            // Compile directly without wrapping
            let mut lexer = Lexer::new(input);
            let tokens = lexer.tokenize().map_err(|e| format!("Lexer error: {}", e))?;

            if tokens.is_empty() {
                return Err("Empty input".to_string());
            }

            let mut parser = Parser::new(tokens);
            let program = parser.parse_program().map_err(|e| format!("Parse error: {}", e))?;

            let mut compiler = Compiler::new();
            return compiler.compile(&program).map_err(|e| format!("Compile error: {}", e));
        }

        // Wrap simple statements in a program
        let wrapped = format!("PROGRAM _repl_\n{}\nEND PROGRAM _repl_", input);
        let mut lexer = Lexer::new(&wrapped);
        let tokens = lexer.tokenize().map_err(|e| format!("Lexer error: {}", e))?;

        if tokens.is_empty() {
            return Err("Empty input".to_string());
        }

        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().map_err(|e| format!("Parse error: {}", e))?;

        let mut compiler = Compiler::new();
        compiler.compile(&program).map_err(|e| format!("Compile error: {}", e))
    }

    /// Execute input and print results
    fn execute_input(&mut self, input: &str) {
        // Compile
        let chunk = match self.compile_input(input) {
            Ok(c) => c,
            Err(e) => {
                println!("Error: {}", e);
                return;
            }
        };

        // Show bytecode if enabled
        if self.show_bytecode {
            println!("--- Bytecode ---");
            println!("{}", chunk.disassemble("repl"));
            println!("----------------");
        }

        // Execute
        let mut vm = VM::new();
        match vm.run(chunk) {
            Ok(()) => {
                // Print any output
                for line in vm.output() {
                    println!("{}", line);
                }

                // If there's a result value on the stack, print it
                // (for expression evaluation)
            }
            Err(e) => {
                println!("Runtime error: {}", e);
            }
        }
    }
}

impl Default for Repl {
    fn default() -> Self {
        Self::new().expect("Failed to create REPL")
    }
}

/// Get home directory path
fn dirs_home() -> Option<String> {
    std::env::var("HOME").ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_input_complete_simple_statement() {
        let repl = Repl::new().unwrap();
        assert!(repl.is_input_complete("x = 5"));
        assert!(repl.is_input_complete("PRINT *, 'hello'"));
    }

    #[test]
    fn test_is_input_complete_program_block() {
        let repl = Repl::new().unwrap();
        assert!(!repl.is_input_complete("PROGRAM test"));
        assert!(repl.is_input_complete("PROGRAM test\nEND PROGRAM test"));
    }

    #[test]
    fn test_is_input_complete_if_block() {
        let repl = Repl::new().unwrap();
        assert!(!repl.is_input_complete("IF (x > 0) THEN"));
        assert!(repl.is_input_complete("IF (x > 0) THEN\nx = 1\nEND IF"));
    }

    #[test]
    fn test_is_input_complete_do_loop() {
        let repl = Repl::new().unwrap();
        assert!(!repl.is_input_complete("DO i = 1, 10"));
        assert!(repl.is_input_complete("DO i = 1, 10\nx = i\nEND DO"));
    }
}
