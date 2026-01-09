//! Debug session management
//!
//! This module provides the DebugSession struct that manages an interactive
//! debugging session, handling the command loop and VM execution.

use std::io::{self, BufRead, Write};

use crate::bytecode::Chunk;
use crate::vm::{VM, RuntimeError};

use super::commands::{parse_command, help_text, DebugCommand};
use super::display::SourceDisplay;
use super::eval::{EvalContext, parse_expr, evaluate};
use super::inspector::ValueFormatter;
use super::interactive::InteractiveDebugger;
use super::state::DebugState;
use super::trace::{Tracer, TraceMode};
use super::watch::WatchManager;

/// Result type for debug session operations
pub type DebugResult<T> = Result<T, DebugError>;

/// Debug session error
#[derive(Debug)]
pub enum DebugError {
    /// VM runtime error
    Runtime(RuntimeError),
    /// IO error (reading commands)
    Io(io::Error),
    /// User requested quit
    Quit,
}

impl std::fmt::Display for DebugError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DebugError::Runtime(e) => write!(f, "Runtime error: {}", e),
            DebugError::Io(e) => write!(f, "IO error: {}", e),
            DebugError::Quit => write!(f, "Quit"),
        }
    }
}

impl std::error::Error for DebugError {}

impl From<RuntimeError> for DebugError {
    fn from(e: RuntimeError) -> Self {
        DebugError::Runtime(e)
    }
}

impl From<io::Error> for DebugError {
    fn from(e: io::Error) -> Self {
        DebugError::Io(e)
    }
}

/// An interactive debugging session
pub struct DebugSession {
    /// The VM being debugged
    vm: VM,
    /// The interactive debugger
    debugger: InteractiveDebugger,
    /// Source display for showing code
    source_display: SourceDisplay,
    /// Whether to show output immediately
    show_output: bool,
    /// Last command (for repeat on empty input)
    last_command: Option<DebugCommand>,
    /// Watch expression manager
    watches: WatchManager,
    /// Execution tracer
    tracer: Tracer,
    /// Currently selected stack frame (0 = innermost)
    current_frame: usize,
    /// Copy of the original chunk for restart
    original_chunk: Option<Chunk>,
}

impl DebugSession {
    /// Create a new debug session
    pub fn new(source: &str, filename: Option<&str>) -> Self {
        let mut debugger = InteractiveDebugger::new();
        let source_display = match filename {
            Some(f) => {
                debugger.set_source_with_file(source, f);
                SourceDisplay::with_file(source, f)
            }
            None => {
                debugger.set_source(source);
                SourceDisplay::new(source)
            }
        };

        Self {
            vm: VM::new(),
            debugger,
            source_display,
            show_output: true,
            last_command: None,
            watches: WatchManager::new(),
            tracer: Tracer::new(),
            current_frame: 0,
            original_chunk: None,
        }
    }

    /// Run the debug session with the given chunk
    pub fn run(&mut self, chunk: Chunk) -> DebugResult<()> {
        // Store original chunk for restart
        self.original_chunk = Some(chunk.clone());

        // Initialize VM with chunk (but don't execute yet)
        self.vm.init(chunk.clone());

        println!("Program loaded. Type /help for commands.");
        println!();

        // Show initial position
        if let Some(loc) = self.vm.current_location() {
            self.debugger.update_location(Some(loc));
        }
        self.show_location();

        // Main debug loop
        self.debug_loop(chunk)
    }

    /// The main debug command loop
    fn debug_loop(&mut self, chunk: Chunk) -> DebugResult<()> {
        let stdin = io::stdin();
        let mut stdout = io::stdout();

        loop {
            // Check if program finished
            if self.vm.is_finished() {
                println!();
                println!("Program finished.");
                self.print_output();
                return Ok(());
            }

            // Print prompt
            let prompt = match self.debugger.get_state() {
                DebugState::Paused(_) => "(debug) ",
                DebugState::Running => "(running) ",
                DebugState::Stepping(_) => "(stepping) ",
                DebugState::Finished => "(finished) ",
            };
            print!("{}", prompt);
            stdout.flush()?;

            // Read command
            let mut line = String::new();
            if stdin.lock().read_line(&mut line)? == 0 {
                // EOF
                return Ok(());
            }

            let line = line.trim();

            // Handle empty input - repeat last command
            let command = if line.is_empty() {
                match &self.last_command {
                    Some(cmd) => cmd.clone(),
                    None => continue,
                }
            } else {
                match parse_command(line) {
                    Ok(cmd) => {
                        self.last_command = Some(cmd.clone());
                        cmd
                    }
                    Err(e) => {
                        println!("Error: {}", e);
                        continue;
                    }
                }
            };

            // Execute command
            match self.execute_command(command, &chunk)? {
                CommandResult::Continue => {}
                CommandResult::Quit => return Err(DebugError::Quit),
                CommandResult::Run => {
                    // Run VM until next pause
                    self.run_until_pause(&chunk)?;
                }
            }
        }
    }

    /// Run the VM until it pauses or finishes
    fn run_until_pause(&mut self, _chunk: &Chunk) -> DebugResult<()> {
        // Save breakpoints before running
        let breakpoints: Vec<_> = self.debugger.breakpoints().list().to_vec();

        // Create a fresh debugger for the VM with same breakpoints
        let mut vm_debugger = InteractiveDebugger::new();

        // Copy breakpoints to VM debugger
        for bp in &breakpoints {
            if let super::breakpoint::BreakpointLocation::Line { line, .. } = &bp.location {
                let id = vm_debugger.breakpoints().add_line(*line, bp.condition.clone());
                if !bp.enabled {
                    vm_debugger.breakpoints().disable(id);
                }
            }
        }

        // Set current location on VM debugger (needed for step to know where to stop)
        if let Some(loc) = self.vm.current_location() {
            vm_debugger.update_location(Some(loc));
        }

        // Set step mode from our debugger
        match self.debugger.get_state() {
            DebugState::Running => vm_debugger.continue_execution(),
            DebugState::Stepping(mode) => match mode {
                super::state::StepMode::Into => vm_debugger.step_into(),
                super::state::StepMode::Over => vm_debugger.step_over(),
                super::state::StepMode::Out => vm_debugger.step_out(),
            },
            _ => vm_debugger.continue_execution(),
        }

        // Set debugger on VM and run
        self.vm.set_debugger(Some(Box::new(vm_debugger)));
        let result = self.vm.resume();

        // Clear VM debugger (we've used it)
        self.vm.set_debugger(None);

        // Update our debugger's location from VM
        if let Some(loc) = self.vm.current_location() {
            self.debugger.update_location(Some(loc));
        }

        // Reset our debugger to paused state
        self.debugger.pause(super::state::PauseReason::Step);

        match result {
            Ok(()) => {
                // VM paused or finished
                if self.vm.is_finished() {
                    println!();
                    println!("Program finished.");
                    self.print_output();
                } else {
                    // Paused - show location
                    self.show_location();
                }
                Ok(())
            }
            Err(e) => {
                println!("Runtime error: {}", e);
                Err(DebugError::Runtime(e))
            }
        }
    }

    /// Execute a debug command
    fn execute_command(&mut self, cmd: DebugCommand, chunk: &Chunk) -> DebugResult<CommandResult> {
        match cmd {
            // Execution control
            DebugCommand::Continue => {
                self.debugger.continue_execution();
                Ok(CommandResult::Run)
            }
            DebugCommand::Step => {
                self.debugger.step_into();
                Ok(CommandResult::Run)
            }
            DebugCommand::Next => {
                self.debugger.step_over();
                Ok(CommandResult::Run)
            }
            DebugCommand::Finish => {
                self.debugger.step_out();
                Ok(CommandResult::Run)
            }
            DebugCommand::Run => {
                self.debugger.continue_execution();
                Ok(CommandResult::Run)
            }
            DebugCommand::Stop => {
                self.debugger.stop();
                println!("Execution stopped.");
                Ok(CommandResult::Continue)
            }
            DebugCommand::Until(line) => {
                // Set temporary breakpoint and continue
                self.debugger.breakpoints().add_temporary(line);
                self.debugger.continue_execution();
                Ok(CommandResult::Run)
            }

            // Breakpoints
            DebugCommand::Break { line, function, condition } => {
                if let Some(l) = line {
                    let id = self.debugger.breakpoints().add_line(l, condition);
                    println!("Breakpoint {} set at line {}", id, l);
                } else if let Some(f) = function {
                    let id = self.debugger.breakpoints().add_function(&f);
                    println!("Breakpoint {} set at function '{}'", id, f);
                }
                Ok(CommandResult::Continue)
            }
            DebugCommand::BreakList => {
                let bps = self.debugger.breakpoints().list();
                if bps.is_empty() {
                    println!("No breakpoints set.");
                } else {
                    println!("Breakpoints:");
                    for bp in bps {
                        println!("  {}", bp.display());
                    }
                }
                Ok(CommandResult::Continue)
            }
            DebugCommand::BreakDelete(id) => {
                if self.debugger.breakpoints().remove(id) {
                    println!("Breakpoint {} deleted.", id);
                } else {
                    println!("No breakpoint with ID {}.", id);
                }
                Ok(CommandResult::Continue)
            }
            DebugCommand::BreakDisable(id) => {
                if self.debugger.breakpoints().disable(id) {
                    println!("Breakpoint {} disabled.", id);
                } else {
                    println!("No breakpoint with ID {}.", id);
                }
                Ok(CommandResult::Continue)
            }
            DebugCommand::BreakEnable(id) => {
                if self.debugger.breakpoints().enable(id) {
                    println!("Breakpoint {} enabled.", id);
                } else {
                    println!("No breakpoint with ID {}.", id);
                }
                Ok(CommandResult::Continue)
            }
            DebugCommand::BreakClear => {
                self.debugger.breakpoints().clear();
                println!("All breakpoints cleared.");
                Ok(CommandResult::Continue)
            }

            // Variable inspection
            DebugCommand::Print(expr) => {
                self.print_expression(&expr);
                Ok(CommandResult::Continue)
            }
            DebugCommand::Locals => {
                self.print_locals();
                Ok(CommandResult::Continue)
            }
            DebugCommand::Globals => {
                // For now, same as locals (Fortran doesn't have clear global/local distinction)
                self.print_locals();
                Ok(CommandResult::Continue)
            }
            DebugCommand::Type(var) => {
                self.print_type(&var);
                Ok(CommandResult::Continue)
            }

            // Watch expressions
            DebugCommand::Watch(expr) => {
                match self.watches.add(&expr, None) {
                    Ok(id) => {
                        // Initialize with current value
                        let ctx = self.build_eval_context();
                        self.watches.initialize(&ctx);
                        println!("Watch {} added: {}", id, expr);
                    }
                    Err(e) => println!("Error adding watch: {}", e),
                }
                Ok(CommandResult::Continue)
            }
            DebugCommand::Unwatch(expr) => {
                // Try parsing as ID first, then as expression
                if let Ok(id) = expr.parse::<usize>() {
                    if self.watches.remove(id) {
                        println!("Watch {} removed.", id);
                    } else {
                        println!("No watch with ID {}.", id);
                    }
                } else {
                    println!("Usage: /unwatch <id>");
                }
                Ok(CommandResult::Continue)
            }
            DebugCommand::WatchList => {
                let watches = self.watches.list();
                if watches.is_empty() {
                    println!("No watch expressions.");
                } else {
                    println!("Watch expressions:");
                    for w in watches {
                        println!("  {}", w.display(&|v| ValueFormatter::format(v)));
                    }
                }
                Ok(CommandResult::Continue)
            }

            // Call stack
            DebugCommand::Backtrace { full } => {
                self.print_backtrace(full);
                Ok(CommandResult::Continue)
            }
            DebugCommand::Up => {
                let trace = self.vm.stack_trace();
                if self.current_frame + 1 < trace.len() {
                    self.current_frame += 1;
                    self.show_frame_info();
                } else {
                    println!("Already at outermost frame.");
                }
                Ok(CommandResult::Continue)
            }
            DebugCommand::Down => {
                if self.current_frame > 0 {
                    self.current_frame -= 1;
                    self.show_frame_info();
                } else {
                    println!("Already at innermost frame.");
                }
                Ok(CommandResult::Continue)
            }
            DebugCommand::Frame(n) => {
                let trace = self.vm.stack_trace();
                if n < trace.len() {
                    self.current_frame = n;
                    self.show_frame_info();
                } else if trace.is_empty() {
                    println!("No stack frames available.");
                } else {
                    println!("Frame {} doesn't exist (0-{} available).", n, trace.len() - 1);
                }
                Ok(CommandResult::Continue)
            }

            // Source display
            DebugCommand::Where => {
                self.show_location();
                Ok(CommandResult::Continue)
            }
            DebugCommand::List { start, end } => {
                self.list_source(start, end);
                Ok(CommandResult::Continue)
            }
            DebugCommand::Disassemble => {
                self.disassemble(chunk);
                Ok(CommandResult::Continue)
            }

            // Variable modification
            DebugCommand::Set { var, value } => {
                let var_name = var.trim().to_uppercase();

                // Parse and evaluate the value expression
                match parse_expr(&value) {
                    Ok(expr) => {
                        let ctx = self.build_eval_context();
                        match evaluate(&expr, &ctx) {
                            Ok(new_value) => {
                                match self.vm.set_variable(&var_name, new_value.clone()) {
                                    Ok(()) => println!("{} = {}", var_name, ValueFormatter::format(&new_value)),
                                    Err(e) => println!("Error: {}", e),
                                }
                            }
                            Err(e) => println!("Error evaluating '{}': {}", value, e),
                        }
                    }
                    Err(e) => println!("Error parsing '{}': {}", value, e),
                }
                Ok(CommandResult::Continue)
            }

            // Tracing
            DebugCommand::Trace(mode_str) => {
                let parts: Vec<&str> = mode_str.split_whitespace().collect();
                match parts.get(0).map(|s| s.to_lowercase()).as_deref() {
                    Some("off") => {
                        self.tracer.set_mode(TraceMode::Off);
                        println!("Tracing disabled.");
                    }
                    Some("calls") => {
                        self.tracer.set_mode(TraceMode::Calls);
                        println!("Tracing function calls.");
                    }
                    Some("vars") | Some("variables") => {
                        self.tracer.set_mode(TraceMode::Variables);
                        println!("Tracing variable changes.");
                    }
                    Some("all") => {
                        self.tracer.set_mode(TraceMode::All);
                        println!("Tracing all statements.");
                    }
                    Some("var") => {
                        if let Some(name) = parts.get(1) {
                            self.tracer.add_traced_var(name);
                            self.tracer.set_mode(TraceMode::Variables);
                            println!("Tracing variable: {}", name.to_uppercase());
                        } else {
                            println!("Usage: /trace var <name>");
                        }
                    }
                    Some("output") => {
                        if let Some(path) = parts.get(1) {
                            match self.tracer.set_output_file(path) {
                                Ok(()) => println!("Trace output: {}", path),
                                Err(e) => println!("Error opening file: {}", e),
                            }
                        } else {
                            println!("Usage: /trace output <file>");
                        }
                    }
                    _ => {
                        println!("Usage: /trace <off|calls|vars|all>");
                        println!("       /trace var <name>");
                        println!("       /trace output <file>");
                    }
                }
                Ok(CommandResult::Continue)
            }

            // Restart
            DebugCommand::Restart => {
                if let Some(ref chunk) = self.original_chunk {
                    self.vm.init(chunk.clone());
                    self.current_frame = 0;
                    self.debugger = InteractiveDebugger::new();
                    self.debugger.set_source(&self.source_display.source());

                    if let Some(loc) = self.vm.current_location() {
                        self.debugger.update_location(Some(loc));
                    }

                    println!("Program restarted.");
                    self.show_location();
                } else {
                    println!("No program loaded to restart.");
                }
                Ok(CommandResult::Continue)
            }

            // Help and quit
            DebugCommand::Help => {
                println!("{}", help_text());
                Ok(CommandResult::Continue)
            }
            DebugCommand::Quit => {
                Ok(CommandResult::Quit)
            }
        }
    }

    /// Show current source location
    fn show_location(&self) {
        if let Some(loc) = self.debugger.current_location() {
            let file = "<input>";
            println!("Stopped at {}:{}:{}", file, loc.line, loc.column);

            // Show source context
            let listing = self.source_display.list_around(loc.line, Some(2));
            print!("{}", listing);
        } else if let Some(loc) = self.vm.current_location() {
            println!("At line {}:{}", loc.line, loc.column);
            let listing = self.source_display.list_around(loc.line, Some(2));
            print!("{}", listing);
        } else {
            println!("Location unknown");
        }
    }

    /// List source code
    fn list_source(&self, start: Option<usize>, end: Option<usize>) {
        let current_line = self.debugger.current_location()
            .map(|l| l.line)
            .or_else(|| self.vm.current_location().map(|l| l.line));

        match (start, end) {
            (Some(s), Some(e)) => {
                print!("{}", self.source_display.format_lines(s, e, current_line));
            }
            (Some(s), None) => {
                print!("{}", self.source_display.list_around(s, None));
            }
            (None, None) => {
                let line = current_line.unwrap_or(1);
                print!("{}", self.source_display.list_around(line, None));
            }
            (None, Some(e)) => {
                print!("{}", self.source_display.format_lines(1, e, current_line));
            }
        }
    }

    /// Print a variable or expression
    fn print_expression(&self, expr: &str) {
        let var_name = expr.trim().to_uppercase();

        // Try direct lookup first
        if let Some(value) = self.vm.get_variable(&var_name) {
            println!("{} = {}", var_name, ValueFormatter::format(value));
            return;
        }

        // Try searching through all variables (case-insensitive)
        let vars = self.vm.variables();
        for (name, value) in &vars {
            if name.to_uppercase() == var_name {
                match value {
                    Some(v) => println!("{} = {}", name, ValueFormatter::format(v)),
                    None => println!("{} = <uninitialized>", name),
                }
                return;
            }
        }

        // Variable not found - suggest similar names
        let names: Vec<&str> = vars.iter().map(|(n, _)| n.as_str()).collect();
        if let Some(similar) = find_similar(&var_name, &names) {
            println!("Variable '{}' not found. Did you mean '{}'?", expr, similar);
        } else {
            println!("Variable '{}' not found.", expr);
        }
    }

    /// Print all local variables
    fn print_locals(&self) {
        let vars = self.vm.variables();

        if vars.is_empty() {
            println!("No local variables.");
            return;
        }

        println!("Local variables:");
        for (name, value) in &vars {
            match value {
                Some(v) => {
                    println!("  {}: {} = {}", name, ValueFormatter::type_name(v), ValueFormatter::format(v));
                }
                None => {
                    println!("  {}: <uninitialized>", name);
                }
            }
        }
    }

    /// Print type of a variable
    fn print_type(&self, var: &str) {
        let var_name = var.trim().to_uppercase();

        if let Some(value) = self.vm.get_variable(&var_name) {
            println!("{}: {}", var_name, ValueFormatter::type_name(value));
        } else {
            println!("Variable '{}' not found.", var);
        }
    }

    /// Print call stack
    fn print_backtrace(&self, _full: bool) {
        let trace = self.vm.stack_trace();

        if trace.is_empty() {
            println!("  <empty stack>");
            return;
        }

        println!("Call stack:");
        for (i, entry) in trace.iter().enumerate() {
            let loc_str = match &entry.location {
                Some(loc) => format!("line {}:{}", loc.line, loc.column),
                None => "<unknown>".to_string(),
            };
            if i == 0 {
                println!("  #{} {} at {}", i, entry.procedure_name, loc_str);
            } else {
                println!("  #{} {} called from {}", i, entry.procedure_name, loc_str);
            }
        }
    }

    /// Show bytecode disassembly
    fn disassemble(&self, chunk: &Chunk) {
        println!("Bytecode disassembly:");
        println!("  IP: {}", self.vm.ip());
        println!();

        let current_ip = self.vm.ip();
        let start = current_ip.saturating_sub(5);
        let end = (current_ip + 10).min(chunk.instructions.len());

        for i in start..end {
            let instr = &chunk.instructions[i];
            let marker = if i == current_ip { "=>" } else { "  " };
            let operand = instr.operand
                .map(|o| format!(" {}", o))
                .unwrap_or_default();
            println!("{} {:4} {:?}{}", marker, i, instr.opcode, operand);
        }
    }

    /// Print any accumulated output
    fn print_output(&self) {
        let output = self.vm.output();
        if !output.is_empty() {
            println!();
            println!("Program output:");
            for line in output {
                println!("  {}", line);
            }
        }
    }

    /// Build evaluation context from current VM variables
    fn build_eval_context(&self) -> EvalContext {
        let mut ctx = EvalContext::new();
        for (name, value) in self.vm.variables() {
            if let Some(v) = value {
                ctx.insert(name.to_uppercase(), v);
            }
        }
        ctx
    }

    /// Show current frame info
    fn show_frame_info(&self) {
        let trace = self.vm.stack_trace();
        if let Some(frame) = trace.get(self.current_frame) {
            let loc_str = frame.location.as_ref()
                .map(|l| format!("line {}:{}", l.line, l.column))
                .unwrap_or_else(|| "<unknown>".to_string());
            println!("#{} {} at {}", self.current_frame, frame.procedure_name, loc_str);
        }
    }
}

/// Result of executing a command
enum CommandResult {
    /// Continue reading commands
    Continue,
    /// Quit the debugger
    Quit,
    /// Run the VM (continue/step/etc)
    Run,
}

/// Find a similar string (simple Levenshtein-like matching)
fn find_similar<'a>(target: &str, candidates: &[&'a str]) -> Option<&'a str> {
    let target = target.to_uppercase();

    for &candidate in candidates {
        let c = candidate.to_uppercase();
        // Simple prefix match
        if c.starts_with(&target) || target.starts_with(&c) {
            return Some(candidate);
        }
        // Simple edit distance check (within 2 edits)
        if levenshtein(&target, &c) <= 2 {
            return Some(candidate);
        }
    }
    None
}

/// Simple Levenshtein distance
fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();

    let m = a.len();
    let n = b.len();

    if m == 0 { return n; }
    if n == 0 { return m; }

    let mut prev: Vec<usize> = (0..=n).collect();
    let mut curr = vec![0; n + 1];

    for i in 1..=m {
        curr[0] = i;
        for j in 1..=n {
            let cost = if a[i-1] == b[j-1] { 0 } else { 1 };
            curr[j] = (prev[j] + 1)
                .min(curr[j-1] + 1)
                .min(prev[j-1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[n]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_similar() {
        let candidates = vec!["INTEGER", "REAL", "CHARACTER"];
        assert_eq!(find_similar("INTGER", &candidates), Some("INTEGER"));
        assert_eq!(find_similar("REA", &candidates), Some("REAL"));
        assert_eq!(find_similar("XYZ", &candidates), None);
    }

    #[test]
    fn test_levenshtein() {
        assert_eq!(levenshtein("", ""), 0);
        assert_eq!(levenshtein("abc", "abc"), 0);
        assert_eq!(levenshtein("abc", "abd"), 1);
        assert_eq!(levenshtein("abc", "ab"), 1);
        assert_eq!(levenshtein("abc", "abcd"), 1);
    }
}
