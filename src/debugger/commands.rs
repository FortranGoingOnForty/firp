//! Debug command parsing for the interactive debugger
//!
//! This module provides parsing for debugger commands entered by the user.

/// Debug commands that can be executed
#[derive(Debug, Clone, PartialEq)]
pub enum DebugCommand {
    // === Execution Control ===

    /// Continue execution until next breakpoint or end
    Continue,
    /// Step into (execute one statement, entering functions)
    Step,
    /// Step over (execute one statement, skipping function internals)
    Next,
    /// Step out (run until current function returns)
    Finish,
    /// Run until specified line
    Until(usize),
    /// Run the program from the beginning
    Run,
    /// Stop execution
    Stop,

    // === Breakpoints ===

    /// Set a breakpoint
    Break {
        line: Option<usize>,
        function: Option<String>,
        condition: Option<String>,
    },
    /// List all breakpoints
    BreakList,
    /// Delete a breakpoint by ID
    BreakDelete(usize),
    /// Disable a breakpoint by ID
    BreakDisable(usize),
    /// Enable a breakpoint by ID
    BreakEnable(usize),
    /// Clear all breakpoints
    BreakClear,

    // === Variable Inspection ===

    /// Print a variable or expression
    Print(String),
    /// Show all local variables
    Locals,
    /// Show all global variables
    Globals,
    /// Show type of a variable
    Type(String),

    // === Watch Expressions ===

    /// Add a watch expression
    Watch(String),
    /// Remove a watch expression
    Unwatch(String),
    /// List all watch expressions
    WatchList,

    // === Call Stack ===

    /// Show call stack (backtrace)
    Backtrace { full: bool },
    /// Move up one stack frame
    Up,
    /// Move down one stack frame
    Down,
    /// Select a specific stack frame
    Frame(usize),

    // === Source Display ===

    /// Show current location
    Where,
    /// List source code
    List {
        start: Option<usize>,
        end: Option<usize>,
    },
    /// Show bytecode disassembly
    Disassemble,

    // === Variable Modification ===

    /// Set a variable value
    Set { var: String, value: String },

    // === Tracing ===

    /// Set trace mode
    Trace(String),

    // === Other ===

    /// Restart the program
    Restart,
    /// Show help
    Help,
    /// Quit the debugger
    Quit,
}

/// Error type for command parsing
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub message: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ParseError {}

/// Parse a debug command from user input
pub fn parse_command(input: &str) -> Result<DebugCommand, ParseError> {
    let input = input.trim();

    // Empty input - repeat last command (caller handles this)
    if input.is_empty() {
        return Err(ParseError {
            message: "Empty command".to_string(),
        });
    }

    // Commands must start with /
    let input = if input.starts_with('/') {
        &input[1..]
    } else {
        // Treat as print expression
        return Ok(DebugCommand::Print(input.to_string()));
    };

    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.is_empty() {
        return Err(ParseError {
            message: "Empty command".to_string(),
        });
    }

    let cmd = parts[0].to_lowercase();
    let args = &parts[1..];

    match cmd.as_str() {
        // Execution control
        "c" | "continue" => Ok(DebugCommand::Continue),
        "s" | "step" => Ok(DebugCommand::Step),
        "n" | "next" => Ok(DebugCommand::Next),
        "f" | "finish" => Ok(DebugCommand::Finish),
        "until" => {
            let line = args.get(0)
                .ok_or_else(|| ParseError {
                    message: "Usage: /until <line>".to_string(),
                })?
                .parse::<usize>()
                .map_err(|_| ParseError {
                    message: "Invalid line number".to_string(),
                })?;
            Ok(DebugCommand::Until(line))
        }
        "run" | "r" => Ok(DebugCommand::Run),
        "stop" => Ok(DebugCommand::Stop),

        // Breakpoints
        "b" | "break" => parse_break_command(args),
        "bl" | "breakpoints" => Ok(DebugCommand::BreakList),
        "d" | "delete" => {
            let id = args.get(0)
                .ok_or_else(|| ParseError {
                    message: "Usage: /delete <breakpoint-id>".to_string(),
                })?
                .parse::<usize>()
                .map_err(|_| ParseError {
                    message: "Invalid breakpoint ID".to_string(),
                })?;
            Ok(DebugCommand::BreakDelete(id))
        }
        "disable" => {
            let id = args.get(0)
                .ok_or_else(|| ParseError {
                    message: "Usage: /disable <breakpoint-id>".to_string(),
                })?
                .parse::<usize>()
                .map_err(|_| ParseError {
                    message: "Invalid breakpoint ID".to_string(),
                })?;
            Ok(DebugCommand::BreakDisable(id))
        }
        "enable" => {
            let id = args.get(0)
                .ok_or_else(|| ParseError {
                    message: "Usage: /enable <breakpoint-id>".to_string(),
                })?
                .parse::<usize>()
                .map_err(|_| ParseError {
                    message: "Invalid breakpoint ID".to_string(),
                })?;
            Ok(DebugCommand::BreakEnable(id))
        }
        "clear" => Ok(DebugCommand::BreakClear),

        // Inspection
        "p" | "print" => {
            if args.is_empty() {
                return Err(ParseError {
                    message: "Usage: /print <expression>".to_string(),
                });
            }
            Ok(DebugCommand::Print(args.join(" ")))
        }
        "l" | "locals" => Ok(DebugCommand::Locals),
        "g" | "globals" => Ok(DebugCommand::Globals),
        "type" => {
            let var = args.get(0)
                .ok_or_else(|| ParseError {
                    message: "Usage: /type <variable>".to_string(),
                })?;
            Ok(DebugCommand::Type((*var).to_string()))
        }

        // Watch
        "watch" => {
            if args.is_empty() {
                return Err(ParseError {
                    message: "Usage: /watch <expression>".to_string(),
                });
            }
            Ok(DebugCommand::Watch(args.join(" ")))
        }
        "unwatch" => {
            if args.is_empty() {
                return Err(ParseError {
                    message: "Usage: /unwatch <expression>".to_string(),
                });
            }
            Ok(DebugCommand::Unwatch(args.join(" ")))
        }
        "watches" => Ok(DebugCommand::WatchList),

        // Stack
        "bt" | "backtrace" => {
            let full = args.get(0).map(|a| *a == "full").unwrap_or(false);
            Ok(DebugCommand::Backtrace { full })
        }
        "up" => Ok(DebugCommand::Up),
        "down" => Ok(DebugCommand::Down),
        "frame" => {
            let n = args.get(0)
                .ok_or_else(|| ParseError {
                    message: "Usage: /frame <number>".to_string(),
                })?
                .parse::<usize>()
                .map_err(|_| ParseError {
                    message: "Invalid frame number".to_string(),
                })?;
            Ok(DebugCommand::Frame(n))
        }

        // Source display
        "w" | "where" => Ok(DebugCommand::Where),
        "list" => parse_list_command(args),
        "disasm" | "disassemble" => Ok(DebugCommand::Disassemble),

        // Variable modification
        "set" => {
            if args.len() < 3 || args[1] != "=" {
                return Err(ParseError {
                    message: "Usage: /set <var> = <value>".to_string(),
                });
            }
            Ok(DebugCommand::Set {
                var: args[0].to_string(),
                value: args[2..].join(" "),
            })
        }

        // Tracing
        "trace" => {
            if args.is_empty() {
                return Err(ParseError {
                    message: "Usage: /trace <off|calls|all|vars> or /trace var <name>".to_string(),
                });
            }
            Ok(DebugCommand::Trace(args.join(" ")))
        }

        // Restart
        "restart" => Ok(DebugCommand::Restart),

        // Help and quit
        "h" | "help" => Ok(DebugCommand::Help),
        "q" | "quit" | "exit" => Ok(DebugCommand::Quit),

        _ => Err(ParseError {
            message: format!("Unknown command: /{}", cmd),
        }),
    }
}

/// Parse break command arguments
fn parse_break_command(args: &[&str]) -> Result<DebugCommand, ParseError> {
    if args.is_empty() {
        return Err(ParseError {
            message: "Usage: /break <line> or /break <function> [if <condition>]".to_string(),
        });
    }

    // Check for "if" condition
    let if_pos = args.iter().position(|&a| a.to_lowercase() == "if");
    let (main_args, condition) = if let Some(pos) = if_pos {
        let cond = args[pos + 1..].join(" ");
        (&args[..pos], if cond.is_empty() { None } else { Some(cond) })
    } else {
        (args, None)
    };

    if main_args.is_empty() {
        return Err(ParseError {
            message: "Usage: /break <line> or /break <function>".to_string(),
        });
    }

    // Try to parse as line number first
    if let Ok(line) = main_args[0].parse::<usize>() {
        Ok(DebugCommand::Break {
            line: Some(line),
            function: None,
            condition,
        })
    } else {
        // Treat as function name
        Ok(DebugCommand::Break {
            line: None,
            function: Some(main_args[0].to_string()),
            condition,
        })
    }
}

/// Parse list command arguments
fn parse_list_command(args: &[&str]) -> Result<DebugCommand, ParseError> {
    if args.is_empty() {
        return Ok(DebugCommand::List { start: None, end: None });
    }

    // Check for range (e.g., "10-20")
    if let Some(dash_pos) = args[0].find('-') {
        let start = args[0][..dash_pos].parse::<usize>()
            .map_err(|_| ParseError {
                message: "Invalid start line".to_string(),
            })?;
        let end = args[0][dash_pos + 1..].parse::<usize>()
            .map_err(|_| ParseError {
                message: "Invalid end line".to_string(),
            })?;
        return Ok(DebugCommand::List {
            start: Some(start),
            end: Some(end),
        });
    }

    // Single line number
    let line = args[0].parse::<usize>()
        .map_err(|_| ParseError {
            message: "Invalid line number".to_string(),
        })?;

    Ok(DebugCommand::List {
        start: Some(line),
        end: None,
    })
}

/// Generate help text for debug commands
pub fn help_text() -> &'static str {
    r#"Debug Commands:

Execution Control:
  /c, /continue    - Continue execution
  /s, /step        - Step into (execute one statement)
  /n, /next        - Step over (skip function internals)
  /f, /finish      - Step out (run until function returns)
  /until <line>    - Run until reaching line
  /run             - Run program from beginning
  /stop            - Stop execution

Breakpoints:
  /b, /break <line>           - Set breakpoint at line
  /b, /break <func>           - Set breakpoint at function
  /break <line> if <cond>     - Conditional breakpoint
  /bl, /breakpoints           - List all breakpoints
  /d, /delete <id>            - Delete breakpoint
  /disable <id>               - Disable breakpoint
  /enable <id>                - Enable breakpoint
  /clear                      - Clear all breakpoints

Inspection:
  /p, /print <expr>  - Print variable or expression
  /l, /locals        - Show local variables
  /g, /globals       - Show global variables
  /type <var>        - Show type of variable

Watch Expressions:
  /watch <expr>      - Add watch expression
  /unwatch <expr>    - Remove watch expression
  /watches           - List watch expressions

Call Stack:
  /bt, /backtrace    - Show call stack
  /bt full           - Show stack with locals
  /up                - Move up one frame
  /down              - Move down one frame
  /frame <n>         - Select frame n

Source Display:
  /w, /where         - Show current location
  /list              - List source around current line
  /list <n>          - List source around line n
  /list <n>-<m>      - List lines n through m
  /disasm            - Show bytecode disassembly

Tracing:
  /trace off         - Disable tracing
  /trace calls       - Trace function entry/exit
  /trace vars        - Trace variable changes
  /trace all         - Trace all statements
  /trace var <name>  - Add variable to trace

Other:
  /set <var> = <val> - Set variable value
  /restart           - Restart program from beginning
  /h, /help          - Show this help
  /q, /quit          - Quit debugger
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_continue() {
        assert_eq!(parse_command("/c").unwrap(), DebugCommand::Continue);
        assert_eq!(parse_command("/continue").unwrap(), DebugCommand::Continue);
    }

    #[test]
    fn test_parse_step() {
        assert_eq!(parse_command("/s").unwrap(), DebugCommand::Step);
        assert_eq!(parse_command("/step").unwrap(), DebugCommand::Step);
    }

    #[test]
    fn test_parse_next() {
        assert_eq!(parse_command("/n").unwrap(), DebugCommand::Next);
        assert_eq!(parse_command("/next").unwrap(), DebugCommand::Next);
    }

    #[test]
    fn test_parse_finish() {
        assert_eq!(parse_command("/f").unwrap(), DebugCommand::Finish);
        assert_eq!(parse_command("/finish").unwrap(), DebugCommand::Finish);
    }

    #[test]
    fn test_parse_break_line() {
        assert_eq!(
            parse_command("/b 10").unwrap(),
            DebugCommand::Break { line: Some(10), function: None, condition: None }
        );
    }

    #[test]
    fn test_parse_break_function() {
        assert_eq!(
            parse_command("/break compute").unwrap(),
            DebugCommand::Break { line: None, function: Some("compute".to_string()), condition: None }
        );
    }

    #[test]
    fn test_parse_break_conditional() {
        assert_eq!(
            parse_command("/b 10 if x > 5").unwrap(),
            DebugCommand::Break {
                line: Some(10),
                function: None,
                condition: Some("x > 5".to_string())
            }
        );
    }

    #[test]
    fn test_parse_print() {
        assert_eq!(
            parse_command("/p x").unwrap(),
            DebugCommand::Print("x".to_string())
        );
        assert_eq!(
            parse_command("/print x + y").unwrap(),
            DebugCommand::Print("x + y".to_string())
        );
    }

    #[test]
    fn test_parse_locals() {
        assert_eq!(parse_command("/l").unwrap(), DebugCommand::Locals);
        assert_eq!(parse_command("/locals").unwrap(), DebugCommand::Locals);
    }

    #[test]
    fn test_parse_backtrace() {
        assert_eq!(
            parse_command("/bt").unwrap(),
            DebugCommand::Backtrace { full: false }
        );
        assert_eq!(
            parse_command("/bt full").unwrap(),
            DebugCommand::Backtrace { full: true }
        );
    }

    #[test]
    fn test_parse_list() {
        assert_eq!(
            parse_command("/list").unwrap(),
            DebugCommand::List { start: None, end: None }
        );
        assert_eq!(
            parse_command("/list 10").unwrap(),
            DebugCommand::List { start: Some(10), end: None }
        );
        assert_eq!(
            parse_command("/list 10-20").unwrap(),
            DebugCommand::List { start: Some(10), end: Some(20) }
        );
    }

    #[test]
    fn test_parse_where() {
        assert_eq!(parse_command("/w").unwrap(), DebugCommand::Where);
        assert_eq!(parse_command("/where").unwrap(), DebugCommand::Where);
    }

    #[test]
    fn test_parse_help() {
        assert_eq!(parse_command("/h").unwrap(), DebugCommand::Help);
        assert_eq!(parse_command("/help").unwrap(), DebugCommand::Help);
    }

    #[test]
    fn test_parse_quit() {
        assert_eq!(parse_command("/q").unwrap(), DebugCommand::Quit);
        assert_eq!(parse_command("/quit").unwrap(), DebugCommand::Quit);
    }

    #[test]
    fn test_parse_set() {
        assert_eq!(
            parse_command("/set x = 10").unwrap(),
            DebugCommand::Set { var: "x".to_string(), value: "10".to_string() }
        );
    }

    #[test]
    fn test_parse_delete() {
        assert_eq!(
            parse_command("/d 1").unwrap(),
            DebugCommand::BreakDelete(1)
        );
    }

    #[test]
    fn test_parse_frame() {
        assert_eq!(
            parse_command("/frame 2").unwrap(),
            DebugCommand::Frame(2)
        );
    }

    #[test]
    fn test_implicit_print() {
        // Without /, treat as print
        assert_eq!(
            parse_command("x + y").unwrap(),
            DebugCommand::Print("x + y".to_string())
        );
    }

    #[test]
    fn test_unknown_command() {
        assert!(parse_command("/unknown").is_err());
    }

    #[test]
    fn test_empty_command() {
        assert!(parse_command("").is_err());
        assert!(parse_command("  ").is_err());
    }
}
