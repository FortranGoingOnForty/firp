//! REPL (Read-Eval-Print-Loop) for interactive Fortran execution

use crate::bytecode::{Chunk, Compiler, Value};
use crate::diagnostic::{DiagnosticRenderer, Diagnostic, SourceMap, RenderStyle};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::profiler::{ProfilerDebugger, ProfileReport, ProfileData};
use crate::vm::VM;
use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use rustyline::{DefaultEditor, Editor};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

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
    /// Accumulated declarations for persistent state
    accumulated_decls: Vec<String>,
    /// Accumulated statements for persistent state
    accumulated_stmts: Vec<String>,
    /// Accumulated procedure definitions (functions/subroutines) for CONTAINS section
    accumulated_procs: Vec<String>,
    /// Previous output line count (to show only new output)
    prev_output_count: usize,
    /// Current block nesting depth (for auto-indentation)
    current_depth: usize,
    /// Source map for diagnostic rendering
    source_map: SourceMap,
    /// Input counter for unique file names
    input_counter: usize,
    /// Whether profiling is enabled
    profiling_enabled: bool,
    /// Shared profile data (persists across executions)
    profile_data: Arc<Mutex<ProfileData>>,
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
            accumulated_decls: Vec::new(),
            accumulated_stmts: Vec::new(),
            accumulated_procs: Vec::new(),
            prev_output_count: 0,
            current_depth: 0,
            source_map: SourceMap::new(),
            input_counter: 0,
            profiling_enabled: false,
            profile_data: Arc::new(Mutex::new(ProfileData::default())),
        })
    }

    /// Emit a diagnostic with rich formatting
    fn emit_diagnostic(&mut self, diag: Diagnostic, source: &str) {
        // Register source in the map with a unique input name
        self.input_counter += 1;
        let file_name = format!("<input:{}>", self.input_counter);
        self.source_map.add_file(&file_name, source);

        // Create renderer and emit diagnostic
        let renderer = DiagnosticRenderer::new(Arc::new(self.source_map.clone()))
            .with_color(true)
            .with_style(RenderStyle::Rich);

        let _ = renderer.emit(&diag);
    }

    /// Run the REPL main loop
    pub fn run(&mut self) -> Result<(), String> {
        self.print_welcome();
        self.load_history();

        loop {
            let prompt = if self.in_multiline { ".. " } else { ":: " };

            // Calculate indentation for auto-indent
            let indent = if self.in_multiline {
                "  ".repeat(self.current_depth)
            } else {
                String::new()
            };

            // Read line with initial indentation
            let readline_result = if indent.is_empty() {
                self.editor.readline(prompt)
            } else {
                self.editor.readline_with_initial(prompt, (&indent, ""))
            };

            match readline_result {
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

                    // Check if this line should be dedented (END, ELSE, CASE, CONTAINS)
                    let trimmed_upper = line.trim().to_uppercase();
                    let is_dedent_line = self.is_dedent_keyword(&trimmed_upper);

                    // If it's a dedent line and was auto-indented, reprint with correct indent
                    if is_dedent_line && self.in_multiline && self.current_depth > 0 {
                        // Calculate the correct indent (one level less)
                        let correct_indent = "  ".repeat(self.current_depth - 1);
                        let display_line = format!("{}{}", correct_indent, line.trim());

                        // Move cursor up, clear line, and reprint with correct indentation
                        // \x1b[A = move up, \x1b[2K = clear line, \r = carriage return
                        print!("\x1b[A\x1b[2K\r.. {}\n", display_line);
                        use std::io::Write;
                        let _ = std::io::stdout().flush();
                    }

                    // Add to history (store the trimmed version)
                    let _ = self.editor.add_history_entry(line.trim());

                    // Calculate depth change from this line
                    let (depth_increase, depth_decrease) = self.calculate_depth_change(&trimmed_upper);

                    // For dedent lines (END, ELSE, CASE), decrease depth BEFORE accumulating
                    // so the line appears at the correct level
                    if is_dedent_line && self.current_depth > 0 {
                        self.current_depth = self.current_depth.saturating_sub(depth_decrease);
                    }

                    // Accumulate input with proper indentation
                    if !self.input_buffer.is_empty() {
                        self.input_buffer.push('\n');
                    }
                    // Store with consistent indentation based on depth at time of entry
                    let stored_indent = "  ".repeat(self.current_depth);
                    self.input_buffer.push_str(&format!("{}{}", stored_indent, line.trim()));

                    // Now apply depth increase for next line
                    self.current_depth += depth_increase;

                    // For non-dedent lines, apply decrease after (for things like ELSE which both close and open)
                    if !is_dedent_line {
                        self.current_depth = self.current_depth.saturating_sub(depth_decrease);
                    }

                    // Check if input is complete
                    if self.is_input_complete(&self.input_buffer) {
                        let input = std::mem::take(&mut self.input_buffer);
                        self.in_multiline = false;
                        self.current_depth = 0;
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
                        self.current_depth = 0;
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

    /// Check if a line (uppercase) is a dedent keyword
    fn is_dedent_keyword(&self, line: &str) -> bool {
        line.starts_with("END ")
            || line.starts_with("END")  // ENDIF, ENDDO, etc.
            || line.starts_with("ELSE")
            || line.starts_with("CASE ")
            || line.starts_with("CASE(")
            || line == "CASE"
            || line.starts_with("CONTAINS")
    }

    /// Calculate depth change from a line: returns (increase, decrease)
    fn calculate_depth_change(&self, line: &str) -> (usize, usize) {
        let mut increase = 0;
        let mut decrease = 0;

        // Block openers
        if !line.starts_with("END") {
            if line.starts_with("PROGRAM ") || line == "PROGRAM" {
                increase = 1;
            } else if line.starts_with("SUBROUTINE ") || line.contains(" SUBROUTINE ") {
                increase = 1;
            } else if line.starts_with("FUNCTION ") || line.contains(" FUNCTION ") {
                increase = 1;
            } else if line.starts_with("MODULE ") && !line.starts_with("MODULE PROCEDURE") {
                increase = 1;
            } else if line.starts_with("IF ") && line.contains(" THEN") {
                increase = 1;
            } else if line.starts_with("DO ") || line == "DO" {
                increase = 1;
            } else if line.starts_with("SELECT ") {
                increase = 1;
            } else if line.starts_with("TYPE ") && !line.starts_with("TYPE(") && !line.contains("::") {
                increase = 1;
            } else if line == "TYPE" {
                increase = 1;
            } else if line.starts_with("ELSE") && !line.starts_with("ELSEIF") && !line.starts_with("ELSE IF") {
                // Plain ELSE opens a new block (but also closes IF block - handled separately)
            } else if line.starts_with("ELSE IF") || line.starts_with("ELSEIF") {
                // ELSE IF closes previous IF block and opens new one
                if line.contains(" THEN") {
                    // It opens a new block
                    increase = 1;
                    decrease = 1;  // Closes the previous block
                }
            } else if line.starts_with("CASE ") || line.starts_with("CASE(") || line == "CASE DEFAULT" {
                // CASE is at same level as SELECT, body is indented
                increase = 1;
                decrease = 1;  // Close previous CASE
            }
        }

        // Block closers
        if line.starts_with("END PROGRAM") || line.starts_with("ENDPROGRAM") {
            decrease = 1;
        } else if line.starts_with("END SUBROUTINE") || line.starts_with("ENDSUBROUTINE") {
            decrease = 1;
        } else if line.starts_with("END FUNCTION") || line.starts_with("ENDFUNCTION") {
            decrease = 1;
        } else if line.starts_with("END MODULE") || line.starts_with("ENDMODULE") {
            decrease = 1;
        } else if line.starts_with("END IF") || line.starts_with("ENDIF") {
            decrease = 1;
        } else if line.starts_with("END DO") || line.starts_with("ENDDO") {
            decrease = 1;
        } else if line.starts_with("END SELECT") || line.starts_with("ENDSELECT") {
            decrease = 1;
        } else if line.starts_with("END TYPE") || line.starts_with("ENDTYPE") {
            decrease = 1;
        } else if line == "END" {
            decrease = 1;
        } else if line.starts_with("ELSE") && !line.contains(" IF") && !line.contains("IF ") {
            // Plain ELSE - closes IF block, but body is still indented
            // We don't decrease here because the ELSE body needs indent
        } else if line.starts_with("CONTAINS") {
            // CONTAINS doesn't change depth, procedures inside are at same level
        }

        (increase, decrease)
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
            ":profile" => {
                self.handle_profile_command(args);
            }
            ":bench" => {
                if args.is_empty() {
                    println!("Usage: :bench <expression> [count]");
                } else {
                    self.benchmark(args);
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
        println!("Profiling Commands:");
        println!("  :profile on       Enable profiling");
        println!("  :profile off      Disable profiling");
        println!("  :profile show     Show current profile");
        println!("  :profile clear    Clear profile data");
        println!("  :bench <expr> [n] Run expression n times (default 100) and report timing");
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
        self.accumulated_decls.clear();
        self.accumulated_stmts.clear();
        self.accumulated_procs.clear();
        self.prev_output_count = 0;
        self.current_depth = 0;
    }

    /// Reset REPL completely
    fn reset(&mut self) {
        self.variables.clear();
        self.vm = VM::new();
        self.compiler = Compiler::new();
        self.input_buffer.clear();
        self.in_multiline = false;
        self.accumulated_decls.clear();
        self.accumulated_stmts.clear();
        self.accumulated_procs.clear();
        self.prev_output_count = 0;
        self.current_depth = 0;
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
        let source = if self.is_program_unit(input) {
            input.to_string()
        } else {
            let is_decl = self.is_declaration(input);
            self.build_program(input, is_decl)
        };

        match self.compile_program_source(&source) {
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

            // Block openers - must NOT start with "END" to avoid matching closers
            if !line.starts_with("END") && !line.starts_with("END ") {
                if line.starts_with("PROGRAM ") || line == "PROGRAM" {
                    depth += 1;
                } else if line.starts_with("SUBROUTINE ") || line.contains(" SUBROUTINE ") {
                    depth += 1;
                } else if line.starts_with("FUNCTION ") || line.contains(" FUNCTION ") {
                    depth += 1;
                } else if line.starts_with("MODULE ") && !line.starts_with("MODULE PROCEDURE") {
                    depth += 1;
                } else if line.starts_with("IF ") && line.contains(" THEN") {
                    depth += 1;
                } else if (line.starts_with("DO ") || line == "DO") && !line.starts_with("DO WHILE") {
                    depth += 1;
                } else if line.starts_with("DO WHILE") {
                    depth += 1;
                } else if line.starts_with("SELECT ") {
                    depth += 1;
                } else if line.starts_with("TYPE ") && !line.starts_with("TYPE(") && !line.starts_with("TYPE ::") {
                    // TYPE definition block (TYPE :: name or TYPE, ... :: name), not TYPE(x) declaration
                    if line.contains("::") {
                        // TYPE, EXTENDS(...) :: name - not a block opener, it's a variable declaration
                    } else {
                        // TYPE name - block opener
                        depth += 1;
                    }
                } else if line == "TYPE" {
                    depth += 1;
                }
            }

            // Block closers - check these AFTER openers since a line can't be both
            // But we need to check closers even if opener matched for lines like "END FUNCTION"
            // which don't match openers anyway
            if line.starts_with("END PROGRAM") || (line == "END" && depth == 1) {
                depth -= 1;
            } else if line.starts_with("END SUBROUTINE") || line == "ENDSUBROUTINE" {
                depth -= 1;
            } else if line.starts_with("END FUNCTION") || line == "ENDFUNCTION" {
                depth -= 1;
            } else if line.starts_with("END MODULE") || line == "ENDMODULE" {
                depth -= 1;
            } else if line.starts_with("END IF") || line == "ENDIF" {
                depth -= 1;
            } else if line.starts_with("END DO") || line == "ENDDO" {
                depth -= 1;
            } else if line.starts_with("END SELECT") || line == "ENDSELECT" {
                depth -= 1;
            } else if line.starts_with("END TYPE") || line == "ENDTYPE" {
                depth -= 1;
            }
        }

        // Input is complete if all blocks are closed
        // Also complete for simple statements (depth == 0 from start)
        depth <= 0
    }

    /// Check if input looks like a declaration (vs a statement)
    fn is_declaration(&self, input: &str) -> bool {
        let upper = input.trim().to_uppercase();
        // Declarations start with type specs or declaration keywords
        upper.starts_with("INTEGER")
            || upper.starts_with("REAL")
            || upper.starts_with("DOUBLE")
            || upper.starts_with("COMPLEX")
            || upper.starts_with("LOGICAL")
            || upper.starts_with("CHARACTER")
            || upper.starts_with("TYPE ")
            || upper.starts_with("TYPE(")
            || upper.starts_with("CLASS(")
            || upper.starts_with("IMPLICIT")
            || upper.starts_with("PARAMETER")
    }

    /// Check if input looks like a bare expression that should be auto-printed
    /// Returns true for expressions like: 2+3, sqrt(16.0), x, arr(1)
    /// Returns false for statements like: x=5, print *, call sub(), if/do/etc.
    fn is_bare_expression(&self, input: &str) -> bool {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return false;
        }

        let upper = trimmed.to_uppercase();

        // Not an expression if it's a declaration
        if self.is_declaration(trimmed) {
            return false;
        }

        // Not an expression if it's a known statement keyword
        if upper.starts_with("PRINT ")
            || upper.starts_with("PRINT*")
            || upper.starts_with("WRITE ")
            || upper.starts_with("WRITE(")
            || upper.starts_with("READ ")
            || upper.starts_with("READ(")
            || upper.starts_with("CALL ")
            || upper.starts_with("IF ")
            || upper.starts_with("IF(")
            || upper.starts_with("DO ")
            || upper == "DO"
            || upper.starts_with("SELECT ")
            || upper.starts_with("RETURN")
            || upper.starts_with("STOP")
            || upper.starts_with("EXIT")
            || upper.starts_with("CYCLE")
            || upper.starts_with("GOTO ")
            || upper.starts_with("GO TO ")
            || upper.starts_with("ALLOCATE")
            || upper.starts_with("DEALLOCATE")
            || upper.starts_with("OPEN")
            || upper.starts_with("CLOSE")
            || upper.starts_with("CONTAINS")
            || upper.starts_with("END ")
            || upper == "END"
        {
            return false;
        }

        // Check if it looks like an assignment (has = but not == or /= or <= or >=)
        // This is tricky because x=5 is assignment but x==5 is comparison expression
        if let Some(eq_pos) = trimmed.find('=') {
            // Check what's before and after the =
            let before = &trimmed[..eq_pos];
            let after = if eq_pos + 1 < trimmed.len() { &trimmed[eq_pos + 1..eq_pos + 2] } else { "" };
            let prev_char = if eq_pos > 0 { &trimmed[eq_pos - 1..eq_pos] } else { "" };

            // It's a comparison if: ==, /=, <=, >=, =>
            let is_comparison = after == "=" || prev_char == "/" || prev_char == "<" || prev_char == ">" || after == ">";

            if !is_comparison {
                // Looks like assignment: identifier = value
                // Check if before the = is a valid lvalue (identifier or array element)
                let lhs = before.trim();
                if !lhs.is_empty() && (lhs.chars().next().unwrap().is_alphabetic() || lhs.ends_with(')')) {
                    return false; // It's an assignment
                }
            }
        }

        // If we get here, it's likely an expression
        true
    }

    /// Check if input is a full program unit (PROGRAM or MODULE)
    fn is_program_unit(&self, input: &str) -> bool {
        let upper = input.to_uppercase();
        (upper.contains("PROGRAM ") && upper.contains("END PROGRAM"))
            || (upper.contains("MODULE ") && upper.contains("END MODULE"))
    }

    /// Check if input is a complete procedure definition (FUNCTION or SUBROUTINE)
    fn is_procedure_definition(&self, input: &str) -> bool {
        let upper = input.to_uppercase();
        let first_line = upper.lines().next().unwrap_or("").trim();

        // Check if it starts with a procedure definition keyword
        let starts_with_proc = first_line.starts_with("FUNCTION ")
            || first_line.starts_with("SUBROUTINE ")
            || first_line.contains(" FUNCTION ")  // RECURSIVE FUNCTION, PURE FUNCTION, etc.
            || first_line.contains(" SUBROUTINE ") // RECURSIVE SUBROUTINE, etc.
            || first_line.starts_with("RECURSIVE FUNCTION")
            || first_line.starts_with("RECURSIVE SUBROUTINE")
            || first_line.starts_with("PURE FUNCTION")
            || first_line.starts_with("PURE SUBROUTINE")
            || first_line.starts_with("ELEMENTAL FUNCTION")
            || first_line.starts_with("ELEMENTAL SUBROUTINE");

        if !starts_with_proc {
            return false;
        }

        // Check if it has a matching END
        (upper.contains("FUNCTION ") && upper.contains("END FUNCTION"))
            || (upper.contains("SUBROUTINE ") && upper.contains("END SUBROUTINE"))
    }

    /// Build a complete program from accumulated state plus new input
    fn build_program(&self, new_input: &str, is_decl: bool) -> String {
        let mut program = String::from("PROGRAM _repl_\n  IMPLICIT NONE\n");

        // Add all accumulated declarations
        for decl in &self.accumulated_decls {
            program.push_str("  ");
            program.push_str(decl);
            program.push('\n');
        }

        // Add new declaration if applicable
        if is_decl && !new_input.trim().is_empty() {
            program.push_str("  ");
            program.push_str(new_input.trim());
            program.push('\n');
        }

        // Add all accumulated statements
        for stmt in &self.accumulated_stmts {
            program.push_str("  ");
            program.push_str(stmt);
            program.push('\n');
        }

        // Add new statement if applicable
        if !is_decl && !new_input.trim().is_empty() {
            program.push_str("  ");
            program.push_str(new_input.trim());
            program.push('\n');
        }

        // Add CONTAINS section with accumulated procedures
        if !self.accumulated_procs.is_empty() {
            program.push_str("\nCONTAINS\n\n");
            for proc in &self.accumulated_procs {
                // Indent each line of the procedure
                for line in proc.lines() {
                    program.push_str("  ");
                    program.push_str(line);
                    program.push('\n');
                }
                program.push('\n');
            }
        }

        program.push_str("END PROGRAM _repl_\n");
        program
    }

    /// Compile input to bytecode with rich diagnostic display
    fn compile_program_source(&mut self, source: &str) -> Result<Chunk, String> {
        let mut lexer = Lexer::new(source);
        let tokens = match lexer.tokenize() {
            Ok(t) => t,
            Err(e) => {
                self.emit_diagnostic(e.clone().into(), source);
                return Err(format!("Lexer error: {}", e));
            }
        };

        if tokens.is_empty() {
            return Err("Empty input".to_string());
        }

        let mut parser = Parser::new(tokens);
        let program = match parser.parse_program() {
            Ok(p) => p,
            Err(e) => {
                self.emit_diagnostic(e.clone().into(), source);
                return Err(format!("Parse error: {}", e));
            }
        };

        let mut compiler = Compiler::new();
        match compiler.compile(&program) {
            Ok(chunk) => Ok(chunk),
            Err(e) => {
                self.emit_diagnostic(e.clone().into(), source);
                Err(format!("Compile error: {}", e))
            }
        }
    }

    /// Execute input and print results
    fn execute_input(&mut self, input: &str) {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return;
        }

        // Check if this is a complete program unit
        if self.is_program_unit(input) {
            // Execute as standalone program
            match self.compile_program_source(input) {
                Ok(chunk) => {
                    if self.show_bytecode {
                        println!("--- Bytecode ---");
                        println!("{}", chunk.disassemble("program"));
                        println!("----------------");
                    }
                    let mut vm = VM::new();
                    match vm.run(chunk) {
                        Ok(()) => {
                            for line in vm.output() {
                                println!("{}", line);
                            }
                        }
                        Err(e) => {
                            self.emit_diagnostic(e.into(), input);
                        }
                    }
                }
                Err(_) => {
                    // Diagnostic already emitted by compile_program_source
                }
            }
            return;
        }

        // Check if this is a procedure definition (FUNCTION or SUBROUTINE)
        if self.is_procedure_definition(input) {
            // Validate by trying to compile with this procedure
            let mut test_procs = self.accumulated_procs.clone();
            test_procs.push(trimmed.to_string());

            // Build a test program with this procedure
            let mut test_program = String::from("PROGRAM _repl_\n  IMPLICIT NONE\n");
            for decl in &self.accumulated_decls {
                test_program.push_str("  ");
                test_program.push_str(decl);
                test_program.push('\n');
            }
            test_program.push_str("\nCONTAINS\n\n");
            for proc in &test_procs {
                for line in proc.lines() {
                    test_program.push_str("  ");
                    test_program.push_str(line);
                    test_program.push('\n');
                }
                test_program.push('\n');
            }
            test_program.push_str("END PROGRAM _repl_\n");

            // Try to compile - if successful, store the procedure
            match self.compile_program_source(&test_program) {
                Ok(_) => {
                    // Extract procedure name for feedback
                    let first_line = trimmed.lines().next().unwrap_or("");
                    let proc_type = if first_line.to_uppercase().contains("FUNCTION") {
                        "Function"
                    } else {
                        "Subroutine"
                    };
                    // Extract name (rough heuristic)
                    let upper = first_line.to_uppercase();
                    let name = if let Some(pos) = upper.find("FUNCTION ") {
                        let after = &first_line[pos + 9..];
                        after.split(&['(', ' '][..]).next().unwrap_or("?")
                    } else if let Some(pos) = upper.find("SUBROUTINE ") {
                        let after = &first_line[pos + 11..];
                        after.split(&['(', ' '][..]).next().unwrap_or("?")
                    } else {
                        "?"
                    };

                    self.accumulated_procs.push(trimmed.to_string());
                    println!("{} '{}' defined.", proc_type, name.trim());
                }
                Err(_) => {
                    // Diagnostic already emitted
                }
            }
            return;
        }

        // Check if this looks like a bare expression - auto-wrap with print
        let (actual_input, is_auto_print) = if self.is_bare_expression(trimmed) {
            (format!("print *, {}", trimmed), true)
        } else {
            (trimmed.to_string(), false)
        };

        // Determine if this is a declaration or statement
        let is_decl = self.is_declaration(&actual_input);

        // Build program with accumulated state plus new input
        let source = self.build_program(&actual_input, is_decl);

        // Try to compile
        match self.compile_program_source(&source) {
            Ok(chunk) => {
                if self.show_bytecode {
                    println!("--- Bytecode ---");
                    println!("{}", chunk.disassemble("repl"));
                    println!("----------------");
                }

                // Execute
                let mut vm = VM::new();
                match vm.run(chunk) {
                    Ok(()) => {
                        // Only print new output (skip output from previous statements)
                        let output = vm.output();
                        for line in output.iter().skip(self.prev_output_count) {
                            println!("{}", line);
                        }

                        // Update state on success (but don't accumulate auto-print expressions)
                        if !is_auto_print {
                            if is_decl {
                                self.accumulated_decls.push(actual_input.clone());
                            } else {
                                self.accumulated_stmts.push(actual_input.clone());
                            }
                        }
                        self.prev_output_count = output.len();
                    }
                    Err(e) => {
                        self.emit_diagnostic(e.into(), &source);
                    }
                }
            }
            Err(_) => {
                // Diagnostic already emitted by compile_program_source
            }
        }
    }

    /// Handle :profile command
    fn handle_profile_command(&mut self, args: &str) {
        match args.to_lowercase().as_str() {
            "on" => {
                self.profiling_enabled = true;
                // Reset profile data when enabling
                if let Ok(mut data) = self.profile_data.lock() {
                    *data = ProfileData::default();
                }
                println!("Profiling enabled.");
            }
            "off" => {
                self.profiling_enabled = false;
                // Remove debugger from VM
                self.vm.set_debugger(None);
                println!("Profiling disabled.");
            }
            "show" => {
                if let Ok(data) = self.profile_data.lock() {
                    let report = ProfileReport::new(&data);
                    print!("{}", report.to_text());
                }
            }
            "clear" => {
                if let Ok(mut data) = self.profile_data.lock() {
                    *data = ProfileData::default();
                }
                println!("Profile data cleared.");
            }
            "" => {
                println!("Usage: :profile on|off|show|clear");
                println!("  on    - Enable profiling");
                println!("  off   - Disable profiling");
                println!("  show  - Show current profile");
                println!("  clear - Clear profile data");
            }
            _ => {
                println!("Unknown profile command: {}", args);
                println!("Usage: :profile on|off|show|clear");
            }
        }
    }

    /// Benchmark an expression
    fn benchmark(&mut self, args: &str) {
        use std::time::Instant;

        // Parse arguments: expression [count]
        let parts: Vec<&str> = args.rsplitn(2, ' ').collect();
        let (expr, count) = if parts.len() == 2 {
            // Check if the last part is a number
            if let Ok(n) = parts[0].parse::<usize>() {
                (parts[1].trim(), n)
            } else {
                (args, 100) // Default count
            }
        } else {
            (args, 100)
        };

        if count == 0 {
            println!("Count must be greater than 0");
            return;
        }

        // Wrap expression in a minimal program
        let source = format!(
            "PROGRAM bench\nIMPLICIT NONE\nINTEGER :: bench_i\nDO bench_i = 1, {}\n{}\nEND DO\nEND PROGRAM",
            count, expr
        );

        // Compile
        let mut lexer = Lexer::new(&source);
        let tokens = match lexer.tokenize() {
            Ok(t) => t,
            Err(e) => {
                println!("Parse error: {}", e);
                return;
            }
        };

        let mut parser = Parser::new(tokens);
        let program = match parser.parse_program() {
            Ok(p) => p,
            Err(e) => {
                println!("Parse error: {}", e);
                return;
            }
        };

        let mut compiler = Compiler::new();
        let chunk = match compiler.compile(&program) {
            Ok(c) => c,
            Err(e) => {
                println!("Compile error: {}", e);
                return;
            }
        };

        // Run and time
        let mut vm = VM::new();
        let start = Instant::now();

        if let Err(e) = vm.run(chunk) {
            println!("Runtime error: {}", e);
            return;
        }

        let elapsed = start.elapsed();
        let total_ms = elapsed.as_secs_f64() * 1000.0;
        let avg_us = (elapsed.as_nanos() as f64 / count as f64) / 1000.0;

        println!("Benchmark: {} iterations", count);
        println!("  Total time: {:.3}ms", total_ms);
        println!("  Average:    {:.3}us per iteration", avg_us);
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
