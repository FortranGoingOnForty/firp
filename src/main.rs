//! FIRP - Fortran Interpreter
//!
//! Usage:
//!   firp              Start interactive REPL
//!   firp <file>       Execute a Fortran file
//!   firp --help       Show help

use firp::bytecode::Compiler;
use firp::diagnostic::{
    codes, Diagnostic, DiagnosticRenderer, RenderStyle, SourceMap, WarningConfig,
};
use firp::lexer::Lexer;
use firp::parser::Parser;
use firp::repl::Repl;
use firp::vm::VM;
use std::env;
use std::fs;
use std::process;
use std::sync::Arc;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// CLI configuration
struct Config {
    render_style: RenderStyle,
    color_enabled: bool,
    file: Option<String>,
    warning_config: WarningConfig,
    debug_mode: bool,
    profile_mode: bool,
    jit_mode: bool,
    jit_threshold: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            render_style: RenderStyle::Rich,
            color_enabled: true,
            file: None,
            warning_config: WarningConfig::new(),
            debug_mode: false,
            profile_mode: false,
            jit_mode: false,
            jit_threshold: 100,
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    // Parse arguments
    let mut config = Config::default();
    let mut i = 1;

    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "--help" | "-h" => {
                print_help();
                return;
            }
            "--version" | "-v" => {
                print_version();
                return;
            }
            "--explain" => {
                if i + 1 < args.len() {
                    explain_error(&args[i + 1]);
                    return;
                } else {
                    eprintln!("--explain requires an error code (e.g., E0201)");
                    process::exit(1);
                }
            }
            "--error-format=rich" => {
                config.render_style = RenderStyle::Rich;
            }
            "--error-format=compact" => {
                config.render_style = RenderStyle::Compact;
            }
            "--error-format=json" => {
                config.render_style = RenderStyle::Json;
            }
            "--color=auto" | "--color" => {
                config.color_enabled = true;
            }
            "--color=always" => {
                config.color_enabled = true;
            }
            "--color=never" => {
                config.color_enabled = false;
            }
            "-W" => {
                if i + 1 < args.len() {
                    i += 1;
                    if let Some(code) = parse_warning_code(&args[i]) {
                        config.warning_config.enable(code);
                    } else {
                        eprintln!("Unknown warning: {}", args[i]);
                        process::exit(1);
                    }
                } else {
                    eprintln!("-W requires a warning name or code");
                    process::exit(1);
                }
            }
            "-A" => {
                if i + 1 < args.len() {
                    i += 1;
                    if let Some(code) = parse_warning_code(&args[i]) {
                        config.warning_config.allow(code);
                    } else {
                        eprintln!("Unknown warning: {}", args[i]);
                        process::exit(1);
                    }
                } else {
                    eprintln!("-A requires a warning name or code");
                    process::exit(1);
                }
            }
            "-D" => {
                if i + 1 < args.len() {
                    i += 1;
                    if let Some(code) = parse_warning_code(&args[i]) {
                        config.warning_config.deny(code);
                    } else {
                        eprintln!("Unknown warning: {}", args[i]);
                        process::exit(1);
                    }
                } else {
                    eprintln!("-D requires a warning name or code");
                    process::exit(1);
                }
            }
            "-Werror" | "--warnings-as-errors" => {
                config.warning_config.warnings_as_errors();
            }
            "--debug" | "-g" => {
                config.debug_mode = true;
            }
            "--profile" => {
                config.profile_mode = true;
            }
            "--jit" => {
                config.jit_mode = true;
            }
            "--jit-threshold" => {
                if i + 1 < args.len() {
                    i += 1;
                    if let Ok(threshold) = args[i].parse::<usize>() {
                        config.jit_threshold = threshold;
                    } else {
                        eprintln!("--jit-threshold requires a number");
                        process::exit(1);
                    }
                } else {
                    eprintln!("--jit-threshold requires a value");
                    process::exit(1);
                }
            }
            _ if arg.starts_with("--") || arg.starts_with("-") && !arg.starts_with("-") => {
                eprintln!("Unknown option: {}", arg);
                eprintln!("Try 'firp --help' for more information.");
                process::exit(1);
            }
            _ => {
                // Assume it's a filename
                config.file = Some(arg.clone());
            }
        }
        i += 1;
    }

    match config.file {
        Some(ref filename) => run_file(filename, &config),
        None => run_repl(),
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
    println!("Options:");
    println!("  --error-format=<format>   Set error output format:");
    println!("                            rich (default), compact, json");
    println!("  --color=<when>            Color output: auto, always, never");
    println!("  --explain <code>          Explain an error code (e.g., E0201)");
    println!("  --debug, -g               Enable interactive debugging");
    println!("  --profile                 Enable performance profiling");
    println!("  --jit                     Enable JIT compilation for hot code");
    println!("  --jit-threshold <n>       Set call count threshold for JIT (default: 100)");
    println!();
    println!("Warning Options:");
    println!("  -W <warning>              Enable warning");
    println!("  -A <warning>              Allow (suppress) warning");
    println!("  -D <warning>              Deny (treat as error)");
    println!("  -Werror                   Treat all warnings as errors");
    println!();
    println!("Warning Names:");
    println!("  unused-variable           W0001 - Variable declared but never used");
    println!("  unused-parameter          W0002 - Function parameter never used");
    println!("  shadowed-variable         W0003 - Variable shadows outer scope");
    println!("  implicit-conversion       W0004 - Implicit type conversion");
    println!("  unreachable-code          W0005 - Unreachable code detected");
    println!("  deprecated                W0006 - Use of deprecated feature");
    println!("  mixed-type-comparison     W0007 - Comparison of different types");
    println!();
    println!("REPL Commands:");
    println!("  :help             Show REPL help");
    println!("  :quit             Exit the REPL");
    println!("  :vars             Show defined variables");
    println!("  :load <file>      Load and execute a file");
    println!();
    println!("Error Code Ranges:");
    println!("  E0001-E0099       Lexer errors");
    println!("  E0101-E0199       Parser errors");
    println!("  E0201-E0299       Semantic errors");
    println!("  E0301-E0399       Compile errors");
    println!("  E0401-E0499       Runtime errors");
    println!("  W0001-W0099       Warnings");
    println!();
    println!("For more information, visit: https://github.com/FortranGoingOnForty/firp");
}

fn print_version() {
    println!("FIRP v{}", VERSION);
}

/// Parse a warning code from string (e.g., "W0001" or "unused-variable")
fn parse_warning_code(s: &str) -> Option<codes::ErrorCode> {
    // Parse numeric warning codes like "W0001"
    if (s.starts_with('W') || s.starts_with('w')) && s.len() > 1 {
        if let Ok(num) = s[1..].parse::<u16>() {
            return Some(codes::ErrorCode::warning(num));
        }
    }

    // Parse named warnings
    match s {
        "unused-variable" | "unused_variable" => Some(codes::W0001_UNUSED_VARIABLE),
        "unused-parameter" | "unused_parameter" => Some(codes::W0002_UNUSED_PARAMETER),
        "shadowed-variable" | "shadowed_variable" => Some(codes::W0003_SHADOWED_VARIABLE),
        "implicit-conversion" | "implicit_conversion" => Some(codes::W0004_IMPLICIT_CONVERSION),
        "unreachable-code" | "unreachable_code" => Some(codes::W0005_UNREACHABLE_CODE),
        "deprecated" => Some(codes::W0006_DEPRECATED),
        "mixed-type-comparison" | "mixed_type_comparison" => Some(codes::W0007_MIXED_TYPE_COMPARISON),
        "all" => {
            // Special case: return any warning to trigger "all" behavior
            // The caller can detect this and use warnings_as_errors() or enable_all()
            Some(codes::ErrorCode::warning(0))
        }
        _ => None,
    }
}

fn explain_error(code_str: &str) {
    // Parse error code string (e.g., "E0201" or "W0001")
    let (is_warning, number) = if code_str.len() >= 2 {
        let prefix = code_str.chars().next().unwrap_or(' ');
        let number_str = &code_str[1..];
        let is_warning = prefix == 'W' || prefix == 'w';
        let is_error = prefix == 'E' || prefix == 'e';

        if (is_warning || is_error) && number_str.chars().all(|c| c.is_ascii_digit()) {
            if let Ok(num) = number_str.parse::<u16>() {
                (is_warning, num)
            } else {
                (false, 0)
            }
        } else {
            (false, 0)
        }
    } else {
        (false, 0)
    };

    if number == 0 {
        eprintln!("Invalid error code: {}", code_str);
        eprintln!("Error codes should be like E0201 or W0001");
        process::exit(1);
    }

    let code = if is_warning {
        codes::ErrorCode::warning(number)
    } else {
        codes::ErrorCode::error(number)
    };

    let description = codes::describe_error(code);

    println!("{}: {}", code, description);
    println!();

    if let Some(explanation) = codes::explain_error(code) {
        println!("{}", explanation);
    } else {
        println!("No detailed explanation available for this error code.");
    }
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

fn run_file(filename: &str, config: &Config) {
    // Read source file
    let source = match fs::read_to_string(filename) {
        Ok(contents) => contents,
        Err(e) => {
            eprintln!("Error reading file '{}': {}", filename, e);
            process::exit(1);
        }
    };

    // Create source map for rich error display
    let mut source_map = SourceMap::new();
    source_map.add_file(filename, &source);
    let source_map = Arc::new(source_map);

    // Create renderer for error display
    let renderer = DiagnosticRenderer::new(source_map)
        .with_color(config.color_enabled)
        .with_style(config.render_style);

    // Tokenize
    let mut lexer = Lexer::with_filename(&source, filename);
    let tokens = match lexer.tokenize() {
        Ok(tokens) => tokens,
        Err(e) => {
            let diag: Diagnostic = e.into();
            let _ = renderer.emit(&diag);
            process::exit(1);
        }
    };

    // Parse
    let mut parser = Parser::new(tokens);
    let program = match parser.parse_program() {
        Ok(program) => program,
        Err(e) => {
            let diag: Diagnostic = e.into();
            let _ = renderer.emit(&diag);
            process::exit(1);
        }
    };

    // Compile
    let mut compiler = Compiler::new();
    let chunk = match compiler.compile(&program) {
        Ok(chunk) => chunk,
        Err(e) => {
            let diag: Diagnostic = e.into();
            let _ = renderer.emit(&diag);
            process::exit(1);
        }
    };

    // Execute
    if config.debug_mode {
        // Run in debug mode with interactive session
        use firp::debugger::{DebugSession, DebugError};

        println!("FIRP Debugger v{}", VERSION);
        println!();

        let mut session = DebugSession::new(&source, Some(filename));

        match session.run(chunk) {
            Ok(()) => {}
            Err(DebugError::Quit) => {
                println!("Debugger exited.");
            }
            Err(DebugError::Runtime(e)) => {
                let diag: Diagnostic = e.into();
                let _ = renderer.emit(&diag);
                process::exit(1);
            }
            Err(DebugError::Io(e)) => {
                eprintln!("IO error: {}", e);
                process::exit(1);
            }
        }
    } else if config.profile_mode {
        // Execution with profiling enabled
        use firp::profiler::{ProfilerDebugger, ProfileReport};

        let mut profiler = ProfilerDebugger::new();
        profiler.start();
        let profile_data = profiler.data(); // Get shared reference to data

        let mut vm = VM::new();
        vm.set_debugger(Some(Box::new(profiler)));

        let result = vm.run(chunk);

        // Print program output first
        for line in vm.output() {
            println!("{}", line);
        }

        // Handle execution result
        if let Err(ref e) = result {
            let diag: Diagnostic = e.clone().into();
            let _ = renderer.emit(&diag);

            // Print stack trace if available
            let stack_trace = vm.format_stack_trace();
            if !stack_trace.is_empty() {
                eprintln!("\nStack trace:");
                eprint!("{}", stack_trace);
            }
        }

        // Generate and display profile report
        if let Ok(data) = profile_data.lock() {
            let report = ProfileReport::new(&data);
            print!("{}", report.to_text());
        }

        if result.is_err() {
            process::exit(1);
        }
    } else {
        // Normal execution without debugger
        let mut vm = VM::new();

        // Enable JIT if requested
        if config.jit_mode {
            use firp::jit::JitConfig;
            let jit_config = JitConfig {
                call_threshold: config.jit_threshold,
                loop_threshold: config.jit_threshold * 10, // Higher threshold for loops
                debug_info: false,
            };
            if let Err(e) = vm.enable_jit(jit_config) {
                eprintln!("Warning: Failed to enable JIT: {}", e);
                eprintln!("Continuing with interpreter-only execution.");
            }
        }

        match vm.run(chunk) {
            Ok(()) => {
                // Print output
                for line in vm.output() {
                    println!("{}", line);
                }
            }
            Err(e) => {
                let diag: Diagnostic = e.into();
                let _ = renderer.emit(&diag);

                // Print stack trace if available
                let stack_trace = vm.format_stack_trace();
                if !stack_trace.is_empty() {
                    eprintln!("\nStack trace:");
                    eprint!("{}", stack_trace);
                }

                process::exit(1);
            }
        }
    }
}
