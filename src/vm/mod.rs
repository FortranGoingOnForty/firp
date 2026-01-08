//! Virtual Machine for bytecode execution
//!
//! This module implements a stack-based virtual machine that executes
//! compiled Fortran bytecode.

use crate::bytecode::{ArrayDim, Chunk, Instruction, Intrinsic, OpCode, Value};
use crate::lexer::SourceLocation;
use std::collections::HashMap;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{self, BufReader, Write};

/// Maximum stack size
const STACK_SIZE: usize = 256;

/// Maximum call stack depth
const MAX_CALL_DEPTH: usize = 64;

/// Call frame for tracking procedure calls
#[derive(Debug, Clone)]
pub struct CallFrame {
    /// Return address (instruction pointer to return to)
    pub return_address: usize,
    /// Base index for local variables
    pub locals_base: usize,
    /// Number of arguments passed
    pub arg_count: usize,
}

/// Slice specification for array section extraction
#[derive(Debug, Clone)]
pub enum SliceSpec {
    /// Single index (reduces dimension)
    Index(i64),
    /// Slice with optional start, end, step
    Slice {
        start: Option<i64>,
        end: Option<i64>,
        step: Option<i64>,
    },
}

/// Runtime error types
#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeError {
    /// Stack overflow
    StackOverflow {
        location: SourceLocation,
    },
    /// Stack underflow (tried to pop from empty stack)
    StackUnderflow {
        location: SourceLocation,
    },
    /// Division by zero
    DivisionByZero {
        location: SourceLocation,
    },
    /// Type error (invalid operation for type)
    TypeError {
        message: String,
        location: SourceLocation,
    },
    /// Invalid variable access
    InvalidVariable {
        index: usize,
        location: SourceLocation,
    },
    /// Invalid constant access
    InvalidConstant {
        index: usize,
        location: SourceLocation,
    },
    /// Invalid jump target
    InvalidJump {
        target: usize,
        location: SourceLocation,
    },
    /// Call stack overflow (too many nested calls)
    CallStackOverflow {
        location: SourceLocation,
    },
    /// Invalid procedure call
    InvalidProcedure {
        index: usize,
        location: SourceLocation,
    },
    /// Invalid instruction (missing operand or malformed)
    InvalidInstruction {
        message: String,
        location: SourceLocation,
    },
    /// Array index out of bounds
    IndexOutOfBounds {
        message: String,
        location: SourceLocation,
    },
    /// I/O error (file operations)
    IoError {
        message: String,
        location: SourceLocation,
    },
    /// Math error (e.g., sqrt of negative)
    MathError {
        message: String,
        location: SourceLocation,
    },
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeError::StackOverflow { location } => {
                write!(f, "Stack overflow at {}", location)
            }
            RuntimeError::StackUnderflow { location } => {
                write!(f, "Stack underflow at {}", location)
            }
            RuntimeError::DivisionByZero { location } => {
                write!(f, "Division by zero at {}", location)
            }
            RuntimeError::TypeError { message, location } => {
                write!(f, "Type error at {}: {}", location, message)
            }
            RuntimeError::InvalidVariable { index, location } => {
                write!(f, "Invalid variable index {} at {}", index, location)
            }
            RuntimeError::InvalidConstant { index, location } => {
                write!(f, "Invalid constant index {} at {}", index, location)
            }
            RuntimeError::InvalidJump { target, location } => {
                write!(f, "Invalid jump target {} at {}", target, location)
            }
            RuntimeError::CallStackOverflow { location } => {
                write!(f, "Call stack overflow at {}", location)
            }
            RuntimeError::InvalidProcedure { index, location } => {
                write!(f, "Invalid procedure index {} at {}", index, location)
            }
            RuntimeError::InvalidInstruction { message, location } => {
                write!(f, "Invalid instruction: {} at {}", message, location)
            }
            RuntimeError::IndexOutOfBounds { message, location } => {
                write!(f, "Index out of bounds: {} at {}", message, location)
            }
            RuntimeError::IoError { message, location } => {
                write!(f, "I/O error: {} at {}", message, location)
            }
            RuntimeError::MathError { message, location } => {
                write!(f, "Math error: {} at {}", message, location)
            }
        }
    }
}

impl std::error::Error for RuntimeError {}

pub type VMResult<T> = Result<T, RuntimeError>;

/// File action mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileAction {
    Read,
    Write,
    ReadWrite,
}

/// File handle wrapper for Fortran I/O
pub struct FileHandle {
    /// Path to the file
    path: String,
    /// Action mode
    action: FileAction,
    /// The underlying file (reopened as needed)
    file: Option<File>,
    /// Read buffer for line-by-line reading
    read_buffer: Vec<String>,
    /// Current position in read buffer
    read_pos: usize,
}

impl FileHandle {
    /// Create a new file handle
    pub fn new(path: String, action: FileAction) -> io::Result<Self> {
        let file = match action {
            FileAction::Read => OpenOptions::new().read(true).open(&path)?,
            FileAction::Write => OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&path)?,
            FileAction::ReadWrite => OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(&path)?,
        };

        Ok(Self {
            path,
            action,
            file: Some(file),
            read_buffer: Vec::new(),
            read_pos: 0,
        })
    }

    /// Check if file can be read
    pub fn can_read(&self) -> bool {
        matches!(self.action, FileAction::Read | FileAction::ReadWrite)
    }

    /// Check if file can be written
    pub fn can_write(&self) -> bool {
        matches!(self.action, FileAction::Write | FileAction::ReadWrite)
    }

    /// Write a line to the file
    pub fn write_line(&mut self, line: &str) -> io::Result<()> {
        if let Some(ref mut file) = self.file {
            writeln!(file, "{}", line)?;
            file.flush()?;
        }
        Ok(())
    }

    /// Read the next line from the file
    pub fn read_line(&mut self) -> io::Result<Option<String>> {
        // Load file into buffer on first read
        if self.read_buffer.is_empty() && self.read_pos == 0 {
            self.load_file_contents()?;
        }

        if self.read_pos < self.read_buffer.len() {
            let line = self.read_buffer[self.read_pos].clone();
            self.read_pos += 1;
            Ok(Some(line))
        } else {
            Ok(None)
        }
    }

    /// Load entire file contents into buffer for reading
    fn load_file_contents(&mut self) -> io::Result<()> {
        use std::io::BufRead;

        // Reopen file for reading from beginning
        let file = OpenOptions::new().read(true).open(&self.path)?;
        let reader = BufReader::new(file);

        self.read_buffer = reader.lines().collect::<io::Result<Vec<String>>>()?;
        self.read_pos = 0;
        Ok(())
    }

    /// Flush and close the file
    pub fn close(&mut self) -> io::Result<()> {
        if let Some(ref mut file) = self.file {
            file.flush()?;
        }
        self.file = None;
        Ok(())
    }
}

/// Virtual Machine for executing bytecode
pub struct VM {
    /// Instruction pointer
    ip: usize,
    /// Operand stack
    stack: Vec<Value>,
    /// Variable storage
    variables: Vec<Option<Value>>,
    /// Current chunk being executed
    chunk: Option<Chunk>,
    /// Trace mode for debugging
    trace: bool,
    /// Output buffer for PRINT statements
    output: Vec<String>,
    /// Call stack for tracking procedure calls
    call_stack: Vec<CallFrame>,
    /// Procedure entry points (address -> procedure index)
    procedure_addresses: Vec<usize>,
    /// Open file handles (unit number -> file handle)
    file_handles: HashMap<i64, FileHandle>,
    /// Input buffer for READ statements (for testing)
    input_buffer: Vec<String>,
    /// Input buffer position
    input_pos: usize,
}

impl VM {
    /// Create a new VM
    pub fn new() -> Self {
        Self {
            ip: 0,
            stack: Vec::with_capacity(STACK_SIZE),
            variables: Vec::new(),
            chunk: None,
            trace: false,
            output: Vec::new(),
            call_stack: Vec::with_capacity(MAX_CALL_DEPTH),
            procedure_addresses: Vec::new(),
            file_handles: HashMap::new(),
            input_buffer: Vec::new(),
            input_pos: 0,
        }
    }

    /// Enable or disable trace mode
    pub fn set_trace(&mut self, enabled: bool) {
        self.trace = enabled;
    }

    /// Set input buffer for READ statements (for testing)
    pub fn set_input(&mut self, input: Vec<String>) {
        self.input_buffer = input;
        self.input_pos = 0;
    }

    /// Reset the VM state
    pub fn reset(&mut self) {
        self.ip = 0;
        self.stack.clear();
        self.variables.clear();
        self.chunk = None;
        self.output.clear();
        self.call_stack.clear();
        self.procedure_addresses.clear();
        // Close all open files
        self.file_handles.clear();
        self.input_buffer.clear();
        self.input_pos = 0;
    }

    /// Run a compiled chunk
    pub fn run(&mut self, chunk: Chunk) -> VMResult<()> {
        // Save input state before reset
        let saved_input = std::mem::take(&mut self.input_buffer);
        let saved_pos = self.input_pos;

        self.reset();

        // Restore input state
        self.input_buffer = saved_input;
        self.input_pos = saved_pos;

        // Initialize variable storage
        self.variables = vec![None; chunk.variables.len()];
        self.chunk = Some(chunk);

        self.execute()
    }

    /// Get the output from PRINT statements
    pub fn output(&self) -> &[String] {
        &self.output
    }

    /// Get a variable value by name
    pub fn get_variable(&self, name: &str) -> Option<&Value> {
        let chunk = self.chunk.as_ref()?;
        let index = chunk.get_variable_index(name)?;
        self.variables.get(index)?.as_ref()
    }

    /// Get all variable names and values
    pub fn variables(&self) -> Vec<(String, Option<Value>)> {
        match &self.chunk {
            Some(chunk) => chunk
                .variables
                .iter()
                .enumerate()
                .map(|(i, name)| {
                    let value = self.variables.get(i).and_then(|v| v.clone());
                    (name.clone(), value)
                })
                .collect(),
            None => Vec::new(),
        }
    }

    /// Main execution loop
    fn execute(&mut self) -> VMResult<()> {
        loop {
            // Copy instruction data to avoid borrow conflicts
            let (opcode, operand, location) = {
                let chunk = self.chunk.as_ref().unwrap();
                if self.ip >= chunk.instructions.len() {
                    break;
                }
                let instruction = &chunk.instructions[self.ip];
                if self.trace {
                    self.trace_instruction(instruction);
                }
                (instruction.opcode, instruction.operand, instruction.location)
            };

            match opcode {
                OpCode::Halt => break,
                OpCode::Nop => {
                    self.ip += 1;
                }

                // Constants
                OpCode::LoadConst => {
                    let index = operand.unwrap_or(0);
                    let value = self.get_constant(index, location)?;
                    self.push(value, location)?;
                    self.ip += 1;
                }
                OpCode::LoadTrue => {
                    self.push(Value::Logical(true), location)?;
                    self.ip += 1;
                }
                OpCode::LoadFalse => {
                    self.push(Value::Logical(false), location)?;
                    self.ip += 1;
                }

                // Variables
                OpCode::LoadVar => {
                    let index = operand.unwrap_or(0);
                    let value = self.get_variable_by_index(index, location)?;
                    self.push(value, location)?;
                    self.ip += 1;
                }
                OpCode::StoreVar => {
                    let index = operand.unwrap_or(0);
                    let value = self.pop(location)?;
                    self.set_variable_by_index(index, value, location)?;
                    self.ip += 1;
                }
                OpCode::LoadRef => {
                    // Push a reference to a variable (for pass-by-reference)
                    let index = operand.unwrap_or(0);
                    // If this variable is itself a reference, follow it
                    let target_index = self.resolve_reference(index, location)?;
                    self.push(Value::Reference(target_index), location)?;
                    self.ip += 1;
                }
                OpCode::StoreParam => {
                    // Store a parameter value (possibly a reference) without resolving
                    // This is used at function/subroutine entry to set up parameter bindings
                    let index = operand.unwrap_or(0);
                    let value = self.pop(location)?;
                    // Store directly without resolving references
                    if index >= self.variables.len() {
                        self.variables.resize(index + 1, None);
                    }
                    self.variables[index] = Some(value);
                    self.ip += 1;
                }

                // Arithmetic
                OpCode::Add => {
                    self.binary_op(|a, b| Self::add_values(a, b), location)?;
                    self.ip += 1;
                }
                OpCode::Subtract => {
                    self.binary_op(|a, b| Self::subtract_values(a, b), location)?;
                    self.ip += 1;
                }
                OpCode::Multiply => {
                    self.binary_op(|a, b| Self::multiply_values(a, b), location)?;
                    self.ip += 1;
                }
                OpCode::Divide => {
                    self.binary_op_checked(
                        |a, b, loc| Self::divide_values(a, b, loc),
                        location,
                    )?;
                    self.ip += 1;
                }
                OpCode::Power => {
                    self.binary_op(|a, b| Self::power_values(a, b), location)?;
                    self.ip += 1;
                }
                OpCode::Negate => {
                    let value = self.pop(location)?;
                    let result = Self::negate_value(value, location)?;
                    self.push(result, location)?;
                    self.ip += 1;
                }

                // Comparison
                OpCode::Equal => {
                    self.comparison_op(|a, b| Self::values_equal(a, b), location)?;
                    self.ip += 1;
                }
                OpCode::NotEqual => {
                    self.comparison_op(|a, b| !Self::values_equal(a, b), location)?;
                    self.ip += 1;
                }
                OpCode::Less => {
                    self.comparison_op(|a, b| Self::values_less(a, b), location)?;
                    self.ip += 1;
                }
                OpCode::LessEqual => {
                    self.comparison_op(|a, b| Self::values_less_equal(a, b), location)?;
                    self.ip += 1;
                }
                OpCode::Greater => {
                    self.comparison_op(|a, b| Self::values_greater(a, b), location)?;
                    self.ip += 1;
                }
                OpCode::GreaterEqual => {
                    self.comparison_op(
                        |a, b| Self::values_greater_equal(a, b),
                        location,
                    )?;
                    self.ip += 1;
                }

                // Logical
                OpCode::And => {
                    let b = self.pop(location)?;
                    let a = self.pop(location)?;
                    let result = Self::logical_and(a, b, location)?;
                    self.push(result, location)?;
                    self.ip += 1;
                }
                OpCode::Or => {
                    let b = self.pop(location)?;
                    let a = self.pop(location)?;
                    let result = Self::logical_or(a, b, location)?;
                    self.push(result, location)?;
                    self.ip += 1;
                }
                OpCode::Not => {
                    let value = self.pop(location)?;
                    let result = Self::logical_not(value, location)?;
                    self.push(result, location)?;
                    self.ip += 1;
                }

                // Type conversions
                OpCode::IntToReal => {
                    let value = self.pop(location)?;
                    let result = match value {
                        Value::Integer(i) => Value::Real(i as f64),
                        _ => value,
                    };
                    self.push(result, location)?;
                    self.ip += 1;
                }
                OpCode::RealToInt => {
                    let value = self.pop(location)?;
                    let result = match value {
                        Value::Real(r) => Value::Integer(r as i64),
                        _ => value,
                    };
                    self.push(result, location)?;
                    self.ip += 1;
                }

                // Control flow
                OpCode::Jump => {
                    let target = operand.unwrap_or(0);
                    self.ip = target;
                }
                OpCode::JumpIfFalse => {
                    let condition = self.pop(location)?;
                    let target = operand.unwrap_or(0);
                    if !Self::is_truthy(&condition) {
                        self.ip = target;
                    } else {
                        self.ip += 1;
                    }
                }
                OpCode::JumpIfTrue => {
                    let condition = self.pop(location)?;
                    let target = operand.unwrap_or(0);
                    if Self::is_truthy(&condition) {
                        self.ip = target;
                    } else {
                        self.ip += 1;
                    }
                }

                // Stack operations
                OpCode::Pop => {
                    self.pop(location)?;
                    self.ip += 1;
                }
                OpCode::Dup => {
                    let value = self.peek(location)?.clone();
                    self.push(value, location)?;
                    self.ip += 1;
                }

                // I/O
                OpCode::Print => {
                    let count = operand.unwrap_or(0);
                    let mut values = Vec::with_capacity(count);
                    for _ in 0..count {
                        values.push(self.pop(location)?);
                    }
                    values.reverse();
                    let output: Vec<String> = values.iter().map(|v| format!("{}", v)).collect();
                    let line = output.join(" ");
                    self.output.push(line.clone());
                    if self.trace {
                        println!("OUTPUT: {}", line);
                    }
                    self.ip += 1;
                }

                OpCode::Read => {
                    let var_index = operand.unwrap_or(0);
                    // Read from input buffer (for testing) or stdin
                    let value = self.read_value(location)?;
                    self.set_variable_by_index(var_index, value, location)?;
                    self.ip += 1;
                }

                OpCode::OpenFile => {
                    // Stack: filename (bottom), unit (top)
                    let unit = match self.pop(location)? {
                        Value::Integer(n) => n,
                        _ => return Err(RuntimeError::TypeError {
                            message: "Unit number must be integer".to_string(),
                            location,
                        }),
                    };
                    let filename = match self.pop(location)? {
                        Value::Character(s) => s,
                        _ => return Err(RuntimeError::TypeError {
                            message: "Filename must be string".to_string(),
                            location,
                        }),
                    };

                    // Open file for reading and writing (default action)
                    let handle = FileHandle::new(filename.clone(), FileAction::ReadWrite)
                        .map_err(|e| RuntimeError::IoError {
                            message: format!("Cannot open '{}': {}", filename, e),
                            location,
                        })?;

                    self.file_handles.insert(unit, handle);
                    self.ip += 1;
                }

                OpCode::CloseFile => {
                    // Stack: unit (top)
                    let unit = match self.pop(location)? {
                        Value::Integer(n) => n,
                        _ => return Err(RuntimeError::TypeError {
                            message: "Unit number must be integer".to_string(),
                            location,
                        }),
                    };

                    // Remove and close file handle
                    if let Some(mut handle) = self.file_handles.remove(&unit) {
                        let _ = handle.close();
                    }
                    self.ip += 1;
                }

                OpCode::WriteFile => {
                    // Stack: values..., unit (top)
                    let count = operand.unwrap_or(0);

                    let unit = match self.pop(location)? {
                        Value::Integer(n) => n,
                        _ => return Err(RuntimeError::TypeError {
                            message: "Unit number must be integer".to_string(),
                            location,
                        }),
                    };

                    // Pop values
                    let mut values = Vec::with_capacity(count);
                    for _ in 0..count {
                        values.push(self.pop(location)?);
                    }
                    values.reverse();

                    // Format output line
                    let output: Vec<String> = values.iter().map(|v| format!("{}", v)).collect();
                    let line = output.join(" ");

                    // Write to file
                    if let Some(handle) = self.file_handles.get_mut(&unit) {
                        if !handle.can_write() {
                            return Err(RuntimeError::IoError {
                                message: format!("Unit {} not open for writing", unit),
                                location,
                            });
                        }
                        handle.write_line(&line).map_err(|e| RuntimeError::IoError {
                            message: format!("Write error: {}", e),
                            location,
                        })?;
                    } else {
                        return Err(RuntimeError::IoError {
                            message: format!("Unit {} not open", unit),
                            location,
                        });
                    }
                    self.ip += 1;
                }

                OpCode::ReadFile => {
                    // Stack: unit (top)
                    let var_index = operand.unwrap_or(0);

                    let unit = match self.pop(location)? {
                        Value::Integer(n) => n,
                        _ => return Err(RuntimeError::TypeError {
                            message: "Unit number must be integer".to_string(),
                            location,
                        }),
                    };

                    // Read from file
                    let value = self.read_from_file(unit, location)?;
                    self.set_variable_by_index(var_index, value, location)?;
                    self.ip += 1;
                }

                OpCode::Call => {
                    // operand is the procedure entry address
                    let proc_address = operand.ok_or(RuntimeError::InvalidProcedure {
                        index: 0,
                        location,
                    })?;

                    // Check call stack depth
                    if self.call_stack.len() >= MAX_CALL_DEPTH {
                        return Err(RuntimeError::CallStackOverflow { location });
                    }

                    // Create call frame with return address (next instruction)
                    let frame = CallFrame {
                        return_address: self.ip + 1,
                        locals_base: self.variables.len(),
                        arg_count: 0, // Arguments handled separately
                    };
                    self.call_stack.push(frame);

                    // Jump to procedure
                    self.ip = proc_address;
                }

                OpCode::Return => {
                    // Pop call frame and return
                    if let Some(frame) = self.call_stack.pop() {
                        self.ip = frame.return_address;
                    } else {
                        // Return from main program - halt
                        break;
                    }
                }

                OpCode::MethodCall => {
                    // Type-bound procedure call with virtual dispatch
                    // Stack: [args..., object, arg_count]
                    // Operand: index of binding name in constants

                    let name_idx = operand.ok_or(RuntimeError::InvalidInstruction {
                        message: "MethodCall requires binding name index".to_string(),
                        location,
                    })?;

                    // Get the binding name from constants
                    let binding_name = {
                        let chunk = self.chunk.as_ref().unwrap();
                        match chunk.constants.get(name_idx) {
                            Some(Value::Character(s)) => s.clone(),
                            _ => return Err(RuntimeError::InvalidInstruction {
                                message: "MethodCall binding name not found".to_string(),
                                location,
                            }),
                        }
                    };

                    // Pop arg count from stack
                    let arg_count = match self.pop(location)? {
                        Value::Integer(n) => n as usize,
                        _ => return Err(RuntimeError::TypeError {
                            message: "Expected integer arg count for MethodCall".to_string(),
                            location,
                        }),
                    };

                    // The object is the first argument (at bottom of args on stack)
                    // Peek at it to get the type info without removing it
                    let stack_len = self.stack.len();
                    if stack_len < arg_count {
                        return Err(RuntimeError::StackUnderflow { location });
                    }
                    let object_idx = stack_len - arg_count;

                    // The object might be a Reference (for pass-by-reference) - dereference it
                    let type_index = match &self.stack[object_idx] {
                        Value::Instance { type_index, .. } => *type_index,
                        Value::Reference(var_idx) => {
                            // Dereference to get the actual instance
                            let actual_value = self.variables.get(*var_idx)
                                .and_then(|v| v.as_ref())
                                .ok_or(RuntimeError::InvalidVariable {
                                    index: *var_idx,
                                    location,
                                })?;
                            match actual_value {
                                Value::Instance { type_index, .. } => *type_index,
                                _ => return Err(RuntimeError::TypeError {
                                    message: format!("MethodCall requires an instance, got {:?}", actual_value),
                                    location,
                                }),
                            }
                        }
                        _ => return Err(RuntimeError::TypeError {
                            message: format!("MethodCall requires an instance, got {:?}", self.stack[object_idx]),
                            location,
                        }),
                    };

                    // Resolve the actual procedure name and address
                    let proc_address = {
                        let chunk = self.chunk.as_ref().unwrap();
                        // Resolve the actual procedure name from the type's procedures map
                        let proc_name = chunk.resolve_procedure(type_index, &binding_name)
                            .ok_or_else(|| RuntimeError::InvalidInstruction {
                                message: format!("No method '{}' found on type", binding_name),
                                location,
                            })?;

                        // Get the procedure address
                        chunk.get_procedure_address(&proc_name)
                            .ok_or_else(|| RuntimeError::InvalidProcedure {
                                index: 0,
                                location,
                            })?
                    };

                    // Check call stack depth
                    if self.call_stack.len() >= MAX_CALL_DEPTH {
                        return Err(RuntimeError::CallStackOverflow { location });
                    }

                    // Create call frame with return address
                    let frame = CallFrame {
                        return_address: self.ip + 1,
                        locals_base: self.variables.len(),
                        arg_count: 0,
                    };
                    self.call_stack.push(frame);

                    // Jump to procedure
                    self.ip = proc_address;
                }

                OpCode::AllocArray => {
                    // Stack contains: num_dims, lower1, upper1, lower2, upper2, ...
                    let var_index = operand.ok_or(RuntimeError::InvalidInstruction {
                        message: "AllocArray requires variable index".to_string(),
                        location,
                    })?;

                    // Pop number of dimensions
                    let num_dims = match self.pop(location)? {
                        Value::Integer(n) => n as usize,
                        _ => return Err(RuntimeError::TypeError {
                            message: "Expected integer for array dimension count".to_string(),
                            location,
                        }),
                    };

                    // Pop bounds for each dimension (in reverse order)
                    let mut dims = Vec::with_capacity(num_dims);
                    for _ in 0..num_dims {
                        let upper = match self.pop(location)? {
                            Value::Integer(n) => n,
                            _ => return Err(RuntimeError::TypeError {
                                message: "Expected integer for array upper bound".to_string(),
                                location,
                            }),
                        };
                        let lower = match self.pop(location)? {
                            Value::Integer(n) => n,
                            _ => return Err(RuntimeError::TypeError {
                                message: "Expected integer for array lower bound".to_string(),
                                location,
                            }),
                        };
                        dims.push(ArrayDim::new(lower, upper));
                    }

                    // Reverse to get correct order
                    dims.reverse();

                    // Create array and store in variable
                    let array = Value::new_integer_array(dims);
                    self.set_variable_by_index(var_index, array, location)?;
                    self.ip += 1;
                }

                OpCode::LoadArrayElem => {
                    // Stack contains: num_indices, index1, index2, ...
                    let var_index = operand.ok_or(RuntimeError::InvalidInstruction {
                        message: "LoadArrayElem requires variable index".to_string(),
                        location,
                    })?;

                    // Pop indices
                    let indices = self.pop_indices(location)?;

                    // Get array from variable
                    let array = self.get_variable_by_index(var_index, location)?;

                    // Get element
                    let element = array.get_element(&indices).ok_or(RuntimeError::IndexOutOfBounds {
                        message: format!("Index {:?} out of bounds", indices),
                        location,
                    })?.clone();

                    self.push(element, location)?;
                    self.ip += 1;
                }

                OpCode::StoreArrayElem => {
                    // Stack contains: value, num_indices, index1, index2, ...
                    let var_index = operand.ok_or(RuntimeError::InvalidInstruction {
                        message: "StoreArrayElem requires variable index".to_string(),
                        location,
                    })?;

                    // Pop indices
                    let indices = self.pop_indices(location)?;

                    // Pop value
                    let value = self.pop(location)?;

                    // Get array from variable (mutably)
                    let array = self.variables
                        .get_mut(var_index)
                        .ok_or(RuntimeError::InvalidVariable { index: var_index, location })?
                        .as_mut()
                        .ok_or(RuntimeError::TypeError {
                            message: "Uninitialized array".to_string(),
                            location,
                        })?;

                    // Set element
                    array.set_element(&indices, value).ok_or(RuntimeError::IndexOutOfBounds {
                        message: format!("Index {:?} out of bounds", indices),
                        location,
                    })?;
                    self.ip += 1;
                }

                OpCode::LoadArraySection => {
                    // Stack contains: num_subscripts, then for each subscript:
                    //   - if index (marker=0): marker, value
                    //   - if slice (marker=1): marker, start, end, step, flags
                    let var_index = operand.ok_or(RuntimeError::InvalidInstruction {
                        message: "LoadArraySection requires variable index".to_string(),
                        location,
                    })?;

                    // Helper to extract integer from Value
                    let value_to_int = |v: &Value, loc: SourceLocation| -> Result<i64, RuntimeError> {
                        match v {
                            Value::Integer(n) => Ok(*n),
                            Value::Real(n) => Ok(*n as i64),
                            _ => Err(RuntimeError::TypeError {
                                message: "Expected numeric value".to_string(),
                                location: loc,
                            }),
                        }
                    };

                    // Pop subscript count
                    let num_subs_val = self.pop(location)?;
                    let num_subs = value_to_int(&num_subs_val, location)? as usize;

                    // Collect subscript specifications (in reverse order due to stack)
                    let mut subscript_specs: Vec<SliceSpec> = Vec::with_capacity(num_subs);
                    for _ in 0..num_subs {
                        subscript_specs.push(SliceSpec::Index(0)); // placeholder
                    }

                    // Pop subscripts in reverse order (last subscript first on stack)
                    for i in (0..num_subs).rev() {
                        // Pop potential flags (if slice) or value (if index)
                        let first_val = self.pop(location)?;
                        let first_int = value_to_int(&first_val, location)?;

                        // Check if this looks like slice flags (0-7)
                        if first_int >= 0 && first_int <= 7 {
                            // Could be slice flags - check further
                            let step_val = self.pop(location)?;
                            let end_val = self.pop(location)?;
                            let start_val = self.pop(location)?;
                            let marker_val = self.pop(location)?;
                            let marker_int = value_to_int(&marker_val, location)?;

                            if marker_int == 1 {
                                // This is a slice
                                let flags = first_int;
                                let has_start = (flags & 1) != 0;
                                let has_end = (flags & 2) != 0;
                                let has_step = (flags & 4) != 0;

                                subscript_specs[i] = SliceSpec::Slice {
                                    start: if has_start { Some(value_to_int(&start_val, location)?) } else { None },
                                    end: if has_end { Some(value_to_int(&end_val, location)?) } else { None },
                                    step: if has_step { Some(value_to_int(&step_val, location)?) } else { None },
                                };
                            } else {
                                // Not a slice - this was an index followed by other values
                                return Err(RuntimeError::TypeError {
                                    message: "Invalid array subscript format".to_string(),
                                    location,
                                });
                            }
                        } else {
                            // This is an index value
                            let marker_val = self.pop(location)?;
                            let marker_int = value_to_int(&marker_val, location)?;
                            if marker_int != 0 {
                                return Err(RuntimeError::TypeError {
                                    message: "Expected index marker".to_string(),
                                    location,
                                });
                            }
                            subscript_specs[i] = SliceSpec::Index(first_int);
                        }
                    }

                    // Get array from variable
                    let array = self.get_variable_by_index(var_index, location)?;

                    // Extract the slice
                    let section = self.extract_array_section(&array, &subscript_specs, location)?;

                    self.push(section, location)?;
                    self.ip += 1;
                }

                OpCode::BuildArray => {
                    // Stack: value1, value2, ..., valueN, count
                    // Pop count, then pop that many values, build array
                    let count_val = self.pop(location)?;
                    let count = match count_val {
                        Value::Integer(n) => n as usize,
                        _ => return Err(RuntimeError::TypeError {
                            message: "BuildArray expected integer count".to_string(),
                            location,
                        }),
                    };

                    // Pop values in reverse order
                    let mut elements = Vec::with_capacity(count);
                    for _ in 0..count {
                        elements.push(self.pop(location)?);
                    }
                    elements.reverse();

                    // Build the array
                    let dims = vec![ArrayDim { lower: 1, upper: count as i64 }];
                    let array = Value::Array { elements, dims };
                    self.push(array, location)?;
                    self.ip += 1;
                }

                OpCode::CallIntrinsic => {
                    // Decode operand: (intrinsic_id << 8) | arg_count
                    let encoded = operand.ok_or(RuntimeError::InvalidInstruction {
                        message: "CallIntrinsic requires operand".to_string(),
                        location,
                    })?;

                    let intrinsic_id = encoded >> 8;
                    let arg_count = encoded & 0xFF;

                    let intrinsic = Intrinsic::from_usize(intrinsic_id)
                        .ok_or(RuntimeError::InvalidInstruction {
                            message: format!("Unknown intrinsic ID: {}", intrinsic_id),
                            location,
                        })?;

                    // Execute the intrinsic
                    let result = self.execute_intrinsic(intrinsic, arg_count, location)?;
                    self.push(result, location)?;
                    self.ip += 1;
                }

                OpCode::CreateInstance => {
                    // operand = type index
                    let type_index = operand.ok_or(RuntimeError::InvalidInstruction {
                        message: "CreateInstance requires type index operand".to_string(),
                        location,
                    })?;

                    // Pop the argument count from stack
                    let arg_count = match self.pop(location)? {
                        Value::Integer(n) => n as usize,
                        _ => return Err(RuntimeError::TypeError {
                            message: "CreateInstance expected argument count".to_string(),
                            location,
                        }),
                    };

                    // Get the type definition
                    let chunk = self.chunk.as_ref().ok_or(RuntimeError::InvalidInstruction {
                        message: "No chunk loaded".to_string(),
                        location,
                    })?;
                    let type_def = chunk.get_type(type_index).ok_or(RuntimeError::TypeError {
                        message: format!("Unknown type index: {}", type_index),
                        location,
                    })?;

                    let num_components = type_def.components.len();

                    // Pop arguments (component values) in reverse order
                    let mut components = vec![Value::Integer(0); num_components];
                    for i in (0..arg_count.min(num_components)).rev() {
                        components[i] = self.pop(location)?;
                    }

                    // Create the instance
                    let instance = Value::Instance {
                        type_index,
                        components,
                    };

                    self.push(instance, location)?;
                    self.ip += 1;
                }

                OpCode::LoadComponent => {
                    // operand = constant index for component name
                    let name_index = operand.ok_or(RuntimeError::InvalidInstruction {
                        message: "LoadComponent requires component name index".to_string(),
                        location,
                    })?;

                    // Get component name from constants
                    let chunk = self.chunk.as_ref().ok_or(RuntimeError::InvalidInstruction {
                        message: "No chunk loaded".to_string(),
                        location,
                    })?;
                    let comp_name = match chunk.constants.get(name_index) {
                        Some(Value::Character(s)) => s.clone(),
                        _ => return Err(RuntimeError::TypeError {
                            message: "LoadComponent expected string component name".to_string(),
                            location,
                        }),
                    };

                    // Pop the instance from stack
                    let instance = self.pop(location)?;

                    match instance {
                        Value::Instance { type_index, components } => {
                            // Get the type definition to find component index
                            let chunk = self.chunk.as_ref().ok_or(RuntimeError::InvalidInstruction {
                                message: "No chunk loaded".to_string(),
                                location,
                            })?;
                            let type_def = chunk.get_type(type_index).ok_or(RuntimeError::TypeError {
                                message: format!("Unknown type index: {}", type_index),
                                location,
                            })?;

                            let comp_idx = type_def.component_index(&comp_name).ok_or(RuntimeError::TypeError {
                                message: format!("Unknown component: {}", comp_name),
                                location,
                            })?;

                            let value = components.get(comp_idx).cloned().unwrap_or(Value::Integer(0));
                            self.push(value, location)?;
                        }
                        _ => return Err(RuntimeError::TypeError {
                            message: "LoadComponent requires instance on stack".to_string(),
                            location,
                        }),
                    }
                    self.ip += 1;
                }

                OpCode::StoreComponent => {
                    // operand = constant index for component name
                    let name_index = operand.ok_or(RuntimeError::InvalidInstruction {
                        message: "StoreComponent requires component name index".to_string(),
                        location,
                    })?;

                    // Get component name from constants
                    let chunk = self.chunk.as_ref().ok_or(RuntimeError::InvalidInstruction {
                        message: "No chunk loaded".to_string(),
                        location,
                    })?;
                    let comp_name = match chunk.constants.get(name_index) {
                        Some(Value::Character(s)) => s.clone(),
                        _ => return Err(RuntimeError::TypeError {
                            message: "StoreComponent expected string component name".to_string(),
                            location,
                        }),
                    };

                    // Pop the instance from stack
                    let instance = self.pop(location)?;

                    // Pop the value to store
                    let value = self.pop(location)?;

                    match instance {
                        Value::Instance { type_index, mut components } => {
                            // Get the type definition to find component index
                            let chunk = self.chunk.as_ref().ok_or(RuntimeError::InvalidInstruction {
                                message: "No chunk loaded".to_string(),
                                location,
                            })?;
                            let type_def = chunk.get_type(type_index).ok_or(RuntimeError::TypeError {
                                message: format!("Unknown type index: {}", type_index),
                                location,
                            })?;

                            let comp_idx = type_def.component_index(&comp_name).ok_or(RuntimeError::TypeError {
                                message: format!("Unknown component: {}", comp_name),
                                location,
                            })?;

                            // Update the component
                            if comp_idx < components.len() {
                                components[comp_idx] = value;
                            }

                            // Push the modified instance back
                            let new_instance = Value::Instance { type_index, components };
                            self.push(new_instance, location)?;
                        }
                        _ => return Err(RuntimeError::TypeError {
                            message: "StoreComponent requires instance on stack".to_string(),
                            location,
                        }),
                    }
                    self.ip += 1;
                }
            }
        }

        Ok(())
    }

    /// Execute an intrinsic function
    fn execute_intrinsic(
        &mut self,
        intrinsic: Intrinsic,
        arg_count: usize,
        location: SourceLocation,
    ) -> VMResult<Value> {
        match intrinsic {
            // Mathematical functions (single argument)
            Intrinsic::Sqrt => {
                let arg = self.pop_as_real(location)?;
                if arg < 0.0 {
                    return Err(RuntimeError::MathError {
                        message: "SQRT of negative number".to_string(),
                        location,
                    });
                }
                Ok(Value::Real(arg.sqrt()))
            }

            Intrinsic::Abs => {
                let arg = self.pop(location)?;
                match arg {
                    Value::Integer(n) => Ok(Value::Integer(n.abs())),
                    Value::Real(n) => Ok(Value::Real(n.abs())),
                    _ => Err(RuntimeError::TypeError {
                        message: "ABS requires numeric argument".to_string(),
                        location,
                    }),
                }
            }

            Intrinsic::Sin => {
                let arg = self.pop_as_real(location)?;
                Ok(Value::Real(arg.sin()))
            }

            Intrinsic::Cos => {
                let arg = self.pop_as_real(location)?;
                Ok(Value::Real(arg.cos()))
            }

            Intrinsic::Tan => {
                let arg = self.pop_as_real(location)?;
                Ok(Value::Real(arg.tan()))
            }

            Intrinsic::Asin => {
                let arg = self.pop_as_real(location)?;
                if arg < -1.0 || arg > 1.0 {
                    return Err(RuntimeError::MathError {
                        message: "ASIN argument out of range [-1, 1]".to_string(),
                        location,
                    });
                }
                Ok(Value::Real(arg.asin()))
            }

            Intrinsic::Acos => {
                let arg = self.pop_as_real(location)?;
                if arg < -1.0 || arg > 1.0 {
                    return Err(RuntimeError::MathError {
                        message: "ACOS argument out of range [-1, 1]".to_string(),
                        location,
                    });
                }
                Ok(Value::Real(arg.acos()))
            }

            Intrinsic::Atan => {
                let arg = self.pop_as_real(location)?;
                Ok(Value::Real(arg.atan()))
            }

            Intrinsic::Atan2 => {
                let x = self.pop_as_real(location)?;
                let y = self.pop_as_real(location)?;
                Ok(Value::Real(y.atan2(x)))
            }

            Intrinsic::Exp => {
                let arg = self.pop_as_real(location)?;
                Ok(Value::Real(arg.exp()))
            }

            Intrinsic::Log => {
                let arg = self.pop_as_real(location)?;
                if arg <= 0.0 {
                    return Err(RuntimeError::MathError {
                        message: "LOG of non-positive number".to_string(),
                        location,
                    });
                }
                Ok(Value::Real(arg.ln()))
            }

            Intrinsic::Log10 => {
                let arg = self.pop_as_real(location)?;
                if arg <= 0.0 {
                    return Err(RuntimeError::MathError {
                        message: "LOG10 of non-positive number".to_string(),
                        location,
                    });
                }
                Ok(Value::Real(arg.log10()))
            }

            // Utility functions
            Intrinsic::Mod => {
                let b = self.pop(location)?;
                let a = self.pop(location)?;
                match (&a, &b) {
                    (Value::Integer(a), Value::Integer(b)) => {
                        if *b == 0 {
                            return Err(RuntimeError::DivisionByZero { location });
                        }
                        Ok(Value::Integer(a % b))
                    }
                    _ => {
                        let a = self.value_to_real(&a, location)?;
                        let b = self.value_to_real(&b, location)?;
                        if b == 0.0 {
                            return Err(RuntimeError::DivisionByZero { location });
                        }
                        Ok(Value::Real(a % b))
                    }
                }
            }

            Intrinsic::Modulo => {
                // MODULO differs from MOD for negative numbers
                // MODULO(a, b) = a - FLOOR(a/b) * b
                let b = self.pop(location)?;
                let a = self.pop(location)?;
                match (&a, &b) {
                    (Value::Integer(a), Value::Integer(b)) => {
                        if *b == 0 {
                            return Err(RuntimeError::DivisionByZero { location });
                        }
                        Ok(Value::Integer(a.rem_euclid(*b)))
                    }
                    _ => {
                        let a = self.value_to_real(&a, location)?;
                        let b = self.value_to_real(&b, location)?;
                        if b == 0.0 {
                            return Err(RuntimeError::DivisionByZero { location });
                        }
                        Ok(Value::Real(a.rem_euclid(b)))
                    }
                }
            }

            Intrinsic::Max => {
                let mut max_val = self.pop(location)?;
                for _ in 1..arg_count {
                    let val = self.pop(location)?;
                    max_val = self.compare_max(max_val, val, location)?;
                }
                Ok(max_val)
            }

            Intrinsic::Min => {
                let mut min_val = self.pop(location)?;
                for _ in 1..arg_count {
                    let val = self.pop(location)?;
                    min_val = self.compare_min(min_val, val, location)?;
                }
                Ok(min_val)
            }

            Intrinsic::Floor => {
                let arg = self.pop_as_real(location)?;
                Ok(Value::Integer(arg.floor() as i64))
            }

            Intrinsic::Ceiling => {
                let arg = self.pop_as_real(location)?;
                Ok(Value::Integer(arg.ceil() as i64))
            }

            Intrinsic::Nint => {
                let arg = self.pop_as_real(location)?;
                Ok(Value::Integer(arg.round() as i64))
            }

            Intrinsic::Sign => {
                let b = self.pop(location)?;
                let a = self.pop(location)?;
                match (&a, &b) {
                    (Value::Integer(a), Value::Integer(b)) => {
                        let sign = if *b >= 0 { 1 } else { -1 };
                        Ok(Value::Integer(a.abs() * sign))
                    }
                    _ => {
                        let a = self.value_to_real(&a, location)?;
                        let b = self.value_to_real(&b, location)?;
                        let sign = if b >= 0.0 { 1.0 } else { -1.0 };
                        Ok(Value::Real(a.abs() * sign))
                    }
                }
            }

            // Type conversion functions
            Intrinsic::Int => {
                let arg = self.pop(location)?;
                match arg {
                    Value::Integer(n) => Ok(Value::Integer(n)),
                    Value::Real(n) => Ok(Value::Integer(n.trunc() as i64)),
                    Value::Logical(b) => Ok(Value::Integer(if b { 1 } else { 0 })),
                    _ => Err(RuntimeError::TypeError {
                        message: "INT requires numeric argument".to_string(),
                        location,
                    }),
                }
            }

            Intrinsic::Real => {
                let arg = self.pop(location)?;
                match arg {
                    Value::Integer(n) => Ok(Value::Real(n as f64)),
                    Value::Real(n) => Ok(Value::Real(n)),
                    Value::Logical(b) => Ok(Value::Real(if b { 1.0 } else { 0.0 })),
                    _ => Err(RuntimeError::TypeError {
                        message: "REAL requires numeric argument".to_string(),
                        location,
                    }),
                }
            }

            Intrinsic::Dble => {
                // DBLE is same as REAL in our implementation (we use f64)
                let arg = self.pop(location)?;
                match arg {
                    Value::Integer(n) => Ok(Value::Real(n as f64)),
                    Value::Real(n) => Ok(Value::Real(n)),
                    _ => Err(RuntimeError::TypeError {
                        message: "DBLE requires numeric argument".to_string(),
                        location,
                    }),
                }
            }

            // Array intrinsics
            Intrinsic::Sum => {
                let arg = self.pop(location)?;
                match arg {
                    Value::Array { elements, .. } => {
                        let mut sum = 0.0;
                        let mut is_integer = true;
                        for elem in &elements {
                            match elem {
                                Value::Integer(n) => sum += *n as f64,
                                Value::Real(n) => {
                                    sum += n;
                                    is_integer = false;
                                }
                                _ => return Err(RuntimeError::TypeError {
                                    message: "SUM requires numeric array".to_string(),
                                    location,
                                }),
                            }
                        }
                        if is_integer {
                            Ok(Value::Integer(sum as i64))
                        } else {
                            Ok(Value::Real(sum))
                        }
                    }
                    _ => Err(RuntimeError::TypeError {
                        message: "SUM requires array argument".to_string(),
                        location,
                    }),
                }
            }

            Intrinsic::Product => {
                let arg = self.pop(location)?;
                match arg {
                    Value::Array { elements, .. } => {
                        let mut product = 1.0;
                        let mut is_integer = true;
                        for elem in &elements {
                            match elem {
                                Value::Integer(n) => product *= *n as f64,
                                Value::Real(n) => {
                                    product *= n;
                                    is_integer = false;
                                }
                                _ => return Err(RuntimeError::TypeError {
                                    message: "PRODUCT requires numeric array".to_string(),
                                    location,
                                }),
                            }
                        }
                        if is_integer {
                            Ok(Value::Integer(product as i64))
                        } else {
                            Ok(Value::Real(product))
                        }
                    }
                    _ => Err(RuntimeError::TypeError {
                        message: "PRODUCT requires array argument".to_string(),
                        location,
                    }),
                }
            }

            Intrinsic::Size => {
                let arg = self.pop(location)?;
                match arg {
                    Value::Array { elements, .. } => {
                        Ok(Value::Integer(elements.len() as i64))
                    }
                    _ => Err(RuntimeError::TypeError {
                        message: "SIZE requires array argument".to_string(),
                        location,
                    }),
                }
            }

            Intrinsic::Maxval => {
                let arg = self.pop(location)?;
                match arg {
                    Value::Array { elements, .. } => {
                        if elements.is_empty() {
                            return Err(RuntimeError::TypeError {
                                message: "MAXVAL: empty array".to_string(),
                                location,
                            });
                        }
                        let mut max_val: f64 = f64::NEG_INFINITY;
                        let mut is_integer = true;
                        for elem in &elements {
                            match elem {
                                Value::Integer(n) => {
                                    if (*n as f64) > max_val {
                                        max_val = *n as f64;
                                    }
                                }
                                Value::Real(n) => {
                                    if *n > max_val {
                                        max_val = *n;
                                    }
                                    is_integer = false;
                                }
                                _ => return Err(RuntimeError::TypeError {
                                    message: "MAXVAL requires numeric array".to_string(),
                                    location,
                                }),
                            }
                        }
                        if is_integer {
                            Ok(Value::Integer(max_val as i64))
                        } else {
                            Ok(Value::Real(max_val))
                        }
                    }
                    _ => Err(RuntimeError::TypeError {
                        message: "MAXVAL requires array argument".to_string(),
                        location,
                    }),
                }
            }

            Intrinsic::Minval => {
                let arg = self.pop(location)?;
                match arg {
                    Value::Array { elements, .. } => {
                        if elements.is_empty() {
                            return Err(RuntimeError::TypeError {
                                message: "MINVAL: empty array".to_string(),
                                location,
                            });
                        }
                        let mut min_val: f64 = f64::INFINITY;
                        let mut is_integer = true;
                        for elem in &elements {
                            match elem {
                                Value::Integer(n) => {
                                    if (*n as f64) < min_val {
                                        min_val = *n as f64;
                                    }
                                }
                                Value::Real(n) => {
                                    if *n < min_val {
                                        min_val = *n;
                                    }
                                    is_integer = false;
                                }
                                _ => return Err(RuntimeError::TypeError {
                                    message: "MINVAL requires numeric array".to_string(),
                                    location,
                                }),
                            }
                        }
                        if is_integer {
                            Ok(Value::Integer(min_val as i64))
                        } else {
                            Ok(Value::Real(min_val))
                        }
                    }
                    _ => Err(RuntimeError::TypeError {
                        message: "MINVAL requires array argument".to_string(),
                        location,
                    }),
                }
            }

            Intrinsic::DotProduct => {
                let arr2 = self.pop(location)?;
                let arr1 = self.pop(location)?;
                match (&arr1, &arr2) {
                    (Value::Array { elements: e1, .. }, Value::Array { elements: e2, .. }) => {
                        if e1.len() != e2.len() {
                            return Err(RuntimeError::TypeError {
                                message: format!(
                                    "DOT_PRODUCT: arrays must have same size ({} vs {})",
                                    e1.len(), e2.len()
                                ),
                                location,
                            });
                        }
                        let mut dot = 0.0;
                        let mut is_integer = true;
                        for (v1, v2) in e1.iter().zip(e2.iter()) {
                            let n1 = match v1 {
                                Value::Integer(n) => *n as f64,
                                Value::Real(n) => { is_integer = false; *n }
                                _ => return Err(RuntimeError::TypeError {
                                    message: "DOT_PRODUCT requires numeric arrays".to_string(),
                                    location,
                                }),
                            };
                            let n2 = match v2 {
                                Value::Integer(n) => *n as f64,
                                Value::Real(n) => { is_integer = false; *n }
                                _ => return Err(RuntimeError::TypeError {
                                    message: "DOT_PRODUCT requires numeric arrays".to_string(),
                                    location,
                                }),
                            };
                            dot += n1 * n2;
                        }
                        if is_integer {
                            Ok(Value::Integer(dot as i64))
                        } else {
                            Ok(Value::Real(dot))
                        }
                    }
                    _ => Err(RuntimeError::TypeError {
                        message: "DOT_PRODUCT requires two array arguments".to_string(),
                        location,
                    }),
                }
            }

            Intrinsic::Reshape => {
                // RESHAPE(source, shape) - reshape array to new shape
                let shape_arr = self.pop(location)?;
                let source = self.pop(location)?;

                // Get shape as vector of dimensions
                let shape = match &shape_arr {
                    Value::Array { elements, .. } => {
                        let mut dims = Vec::new();
                        for elem in elements {
                            match elem {
                                Value::Integer(n) => dims.push(*n as usize),
                                _ => return Err(RuntimeError::TypeError {
                                    message: "RESHAPE: shape must be integer array".to_string(),
                                    location,
                                }),
                            }
                        }
                        dims
                    }
                    _ => return Err(RuntimeError::TypeError {
                        message: "RESHAPE: shape must be an array".to_string(),
                        location,
                    }),
                };

                // Get source elements
                let source_elements = match source {
                    Value::Array { elements, .. } => elements,
                    _ => return Err(RuntimeError::TypeError {
                        message: "RESHAPE: source must be an array".to_string(),
                        location,
                    }),
                };

                // Calculate total size of new shape
                let new_size: usize = shape.iter().product();
                if new_size != source_elements.len() {
                    return Err(RuntimeError::TypeError {
                        message: format!(
                            "RESHAPE: source has {} elements but shape requires {}",
                            source_elements.len(), new_size
                        ),
                        location,
                    });
                }

                // Create new array with the shape
                let dims: Vec<ArrayDim> = shape.iter()
                    .map(|&s| ArrayDim::new(1, s as i64))
                    .collect();

                Ok(Value::Array {
                    elements: source_elements,
                    dims,
                })
            }

            Intrinsic::Transpose => {
                // TRANSPOSE(matrix) - transpose a 2D array
                let matrix = self.pop(location)?;

                match matrix {
                    Value::Array { elements, dims } => {
                        if dims.len() != 2 {
                            return Err(RuntimeError::TypeError {
                                message: format!(
                                    "TRANSPOSE: requires 2D array, got {}D",
                                    dims.len()
                                ),
                                location,
                            });
                        }

                        let rows = dims[0].size() as usize;
                        let cols = dims[1].size() as usize;

                        // Create transposed array (swap rows and cols)
                        let mut transposed = vec![Value::Integer(0); elements.len()];
                        for i in 0..rows {
                            for j in 0..cols {
                                let old_idx = i * cols + j;
                                let new_idx = j * rows + i;
                                transposed[new_idx] = elements[old_idx].clone();
                            }
                        }

                        let new_dims = vec![
                            ArrayDim::new(1, cols as i64),
                            ArrayDim::new(1, rows as i64),
                        ];

                        Ok(Value::Array {
                            elements: transposed,
                            dims: new_dims,
                        })
                    }
                    _ => Err(RuntimeError::TypeError {
                        message: "TRANSPOSE: requires array argument".to_string(),
                        location,
                    }),
                }
            }

            Intrinsic::Matmul => {
                // MATMUL(matrix_a, matrix_b) - matrix multiplication
                let matrix_b = self.pop(location)?;
                let matrix_a = self.pop(location)?;

                match (&matrix_a, &matrix_b) {
                    (
                        Value::Array { elements: a_elems, dims: a_dims },
                        Value::Array { elements: b_elems, dims: b_dims },
                    ) => {
                        // Support 2D x 2D, 2D x 1D (vector), and 1D x 2D
                        let (a_rows, a_cols) = if a_dims.len() == 2 {
                            (a_dims[0].size() as usize, a_dims[1].size() as usize)
                        } else if a_dims.len() == 1 {
                            (1, a_dims[0].size() as usize)
                        } else {
                            return Err(RuntimeError::TypeError {
                                message: "MATMUL: first argument must be 1D or 2D array".to_string(),
                                location,
                            });
                        };

                        let (b_rows, b_cols) = if b_dims.len() == 2 {
                            (b_dims[0].size() as usize, b_dims[1].size() as usize)
                        } else if b_dims.len() == 1 {
                            (b_dims[0].size() as usize, 1)
                        } else {
                            return Err(RuntimeError::TypeError {
                                message: "MATMUL: second argument must be 1D or 2D array".to_string(),
                                location,
                            });
                        };

                        // Check dimensions are compatible for multiplication
                        if a_cols != b_rows {
                            return Err(RuntimeError::TypeError {
                                message: format!(
                                    "MATMUL: incompatible dimensions ({},{}) x ({},{})",
                                    a_rows, a_cols, b_rows, b_cols
                                ),
                                location,
                            });
                        }

                        // Perform matrix multiplication
                        // Result is stored in column-major order for Fortran
                        let mut result = vec![Value::Integer(0); a_rows * b_cols];
                        let mut is_integer = true;

                        for i in 0..a_rows {
                            for j in 0..b_cols {
                                let mut sum = 0.0;
                                for k in 0..a_cols {
                                    // Fortran uses column-major order
                                    let a_idx = i + k * a_rows;
                                    let b_idx = k + j * b_rows;

                                    let a_val = match &a_elems[a_idx] {
                                        Value::Integer(n) => *n as f64,
                                        Value::Real(n) => { is_integer = false; *n }
                                        _ => return Err(RuntimeError::TypeError {
                                            message: "MATMUL: requires numeric arrays".to_string(),
                                            location,
                                        }),
                                    };
                                    let b_val = match &b_elems[b_idx] {
                                        Value::Integer(n) => *n as f64,
                                        Value::Real(n) => { is_integer = false; *n }
                                        _ => return Err(RuntimeError::TypeError {
                                            message: "MATMUL: requires numeric arrays".to_string(),
                                            location,
                                        }),
                                    };
                                    sum += a_val * b_val;
                                }
                                // Store result in column-major order
                                let result_idx = i + j * a_rows;
                                if is_integer {
                                    result[result_idx] = Value::Integer(sum as i64);
                                } else {
                                    result[result_idx] = Value::Real(sum);
                                }
                            }
                        }

                        // Determine result dimensions
                        let result_dims = if a_dims.len() == 1 && b_dims.len() == 1 {
                            // 1D x 1D = scalar (but we return 1D with one element)
                            vec![ArrayDim::new(1, 1)]
                        } else if a_dims.len() == 1 {
                            // 1D x 2D = 1D
                            vec![ArrayDim::new(1, b_cols as i64)]
                        } else if b_dims.len() == 1 {
                            // 2D x 1D = 1D
                            vec![ArrayDim::new(1, a_rows as i64)]
                        } else {
                            // 2D x 2D = 2D
                            vec![
                                ArrayDim::new(1, a_rows as i64),
                                ArrayDim::new(1, b_cols as i64),
                            ]
                        };

                        Ok(Value::Array {
                            elements: result,
                            dims: result_dims,
                        })
                    }
                    _ => Err(RuntimeError::TypeError {
                        message: "MATMUL: requires two array arguments".to_string(),
                        location,
                    }),
                }
            }

            // Character intrinsics
            Intrinsic::Len => {
                let arg = self.pop(location)?;
                match arg {
                    Value::Character(s) => Ok(Value::Integer(s.len() as i64)),
                    _ => Err(RuntimeError::TypeError {
                        message: "LEN requires character argument".to_string(),
                        location,
                    }),
                }
            }

            Intrinsic::LenTrim => {
                let arg = self.pop(location)?;
                match arg {
                    Value::Character(s) => Ok(Value::Integer(s.trim_end().len() as i64)),
                    _ => Err(RuntimeError::TypeError {
                        message: "LEN_TRIM requires character argument".to_string(),
                        location,
                    }),
                }
            }

            Intrinsic::Trim => {
                let arg = self.pop(location)?;
                match arg {
                    Value::Character(s) => Ok(Value::Character(s.trim_end().to_string())),
                    _ => Err(RuntimeError::TypeError {
                        message: "TRIM requires character argument".to_string(),
                        location,
                    }),
                }
            }

            Intrinsic::Adjustl => {
                let arg = self.pop(location)?;
                match arg {
                    Value::Character(s) => {
                        let trimmed = s.trim_start();
                        let padding = s.len() - trimmed.len();
                        Ok(Value::Character(format!("{}{}", trimmed, " ".repeat(padding))))
                    }
                    _ => Err(RuntimeError::TypeError {
                        message: "ADJUSTL requires character argument".to_string(),
                        location,
                    }),
                }
            }

            Intrinsic::Adjustr => {
                let arg = self.pop(location)?;
                match arg {
                    Value::Character(s) => {
                        let trimmed = s.trim_end();
                        let padding = s.len() - trimmed.len();
                        Ok(Value::Character(format!("{}{}", " ".repeat(padding), trimmed)))
                    }
                    _ => Err(RuntimeError::TypeError {
                        message: "ADJUSTR requires character argument".to_string(),
                        location,
                    }),
                }
            }

            Intrinsic::Index => {
                let substring = self.pop(location)?;
                let string = self.pop(location)?;
                match (&string, &substring) {
                    (Value::Character(s), Value::Character(sub)) => {
                        let pos = s.find(sub).map(|p| p + 1).unwrap_or(0);
                        Ok(Value::Integer(pos as i64))
                    }
                    _ => Err(RuntimeError::TypeError {
                        message: "INDEX requires two character arguments".to_string(),
                        location,
                    }),
                }
            }

            Intrinsic::Repeat => {
                let ncopies = self.pop(location)?;
                let string = self.pop(location)?;
                match (&string, &ncopies) {
                    (Value::Character(s), Value::Integer(n)) => {
                        if *n < 0 {
                            return Err(RuntimeError::TypeError {
                                message: "REPEAT: ncopies must be non-negative".to_string(),
                                location,
                            });
                        }
                        Ok(Value::Character(s.repeat(*n as usize)))
                    }
                    _ => Err(RuntimeError::TypeError {
                        message: "REPEAT requires character and integer arguments".to_string(),
                        location,
                    }),
                }
            }

            Intrinsic::CharFn => {
                let arg = self.pop(location)?;
                match arg {
                    Value::Integer(i) => {
                        if i < 0 || i > 127 {
                            return Err(RuntimeError::TypeError {
                                message: format!("CHAR: value {} out of ASCII range", i),
                                location,
                            });
                        }
                        Ok(Value::Character((i as u8 as char).to_string()))
                    }
                    _ => Err(RuntimeError::TypeError {
                        message: "CHAR requires integer argument".to_string(),
                        location,
                    }),
                }
            }

            Intrinsic::Ichar => {
                let arg = self.pop(location)?;
                match arg {
                    Value::Character(s) => {
                        if s.is_empty() {
                            return Err(RuntimeError::TypeError {
                                message: "ICHAR requires non-empty string".to_string(),
                                location,
                            });
                        }
                        Ok(Value::Integer(s.chars().next().unwrap() as i64))
                    }
                    _ => Err(RuntimeError::TypeError {
                        message: "ICHAR requires character argument".to_string(),
                        location,
                    }),
                }
            }

            // Bit manipulation intrinsics
            Intrinsic::Iand => {
                let j = self.pop(location)?;
                let i = self.pop(location)?;
                match (&i, &j) {
                    (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a & b)),
                    _ => Err(RuntimeError::TypeError {
                        message: "IAND requires integer arguments".to_string(),
                        location,
                    }),
                }
            }

            Intrinsic::Ior => {
                let j = self.pop(location)?;
                let i = self.pop(location)?;
                match (&i, &j) {
                    (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a | b)),
                    _ => Err(RuntimeError::TypeError {
                        message: "IOR requires integer arguments".to_string(),
                        location,
                    }),
                }
            }

            Intrinsic::Ieor => {
                let j = self.pop(location)?;
                let i = self.pop(location)?;
                match (&i, &j) {
                    (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a ^ b)),
                    _ => Err(RuntimeError::TypeError {
                        message: "IEOR requires integer arguments".to_string(),
                        location,
                    }),
                }
            }

            Intrinsic::Not => {
                let arg = self.pop(location)?;
                match arg {
                    Value::Integer(i) => Ok(Value::Integer(!i)),
                    _ => Err(RuntimeError::TypeError {
                        message: "NOT requires integer argument".to_string(),
                        location,
                    }),
                }
            }

            Intrinsic::Btest => {
                let pos = self.pop(location)?;
                let i = self.pop(location)?;
                match (&i, &pos) {
                    (Value::Integer(val), Value::Integer(bit)) => {
                        if *bit < 0 || *bit >= 64 {
                            return Err(RuntimeError::TypeError {
                                message: "BTEST: bit position out of range".to_string(),
                                location,
                            });
                        }
                        Ok(Value::Logical((val >> bit) & 1 == 1))
                    }
                    _ => Err(RuntimeError::TypeError {
                        message: "BTEST requires integer arguments".to_string(),
                        location,
                    }),
                }
            }

            Intrinsic::Ibset => {
                let pos = self.pop(location)?;
                let i = self.pop(location)?;
                match (&i, &pos) {
                    (Value::Integer(val), Value::Integer(bit)) => {
                        if *bit < 0 || *bit >= 64 {
                            return Err(RuntimeError::TypeError {
                                message: "IBSET: bit position out of range".to_string(),
                                location,
                            });
                        }
                        Ok(Value::Integer(val | (1 << bit)))
                    }
                    _ => Err(RuntimeError::TypeError {
                        message: "IBSET requires integer arguments".to_string(),
                        location,
                    }),
                }
            }

            Intrinsic::Ibclr => {
                let pos = self.pop(location)?;
                let i = self.pop(location)?;
                match (&i, &pos) {
                    (Value::Integer(val), Value::Integer(bit)) => {
                        if *bit < 0 || *bit >= 64 {
                            return Err(RuntimeError::TypeError {
                                message: "IBCLR: bit position out of range".to_string(),
                                location,
                            });
                        }
                        Ok(Value::Integer(val & !(1 << bit)))
                    }
                    _ => Err(RuntimeError::TypeError {
                        message: "IBCLR requires integer arguments".to_string(),
                        location,
                    }),
                }
            }

            Intrinsic::Ishft => {
                let shift = self.pop(location)?;
                let i = self.pop(location)?;
                match (&i, &shift) {
                    (Value::Integer(val), Value::Integer(sh)) => {
                        let result = if *sh >= 0 {
                            val << sh
                        } else {
                            val >> (-sh)
                        };
                        Ok(Value::Integer(result))
                    }
                    _ => Err(RuntimeError::TypeError {
                        message: "ISHFT requires integer arguments".to_string(),
                        location,
                    }),
                }
            }

            Intrinsic::ThisImage => {
                // In single-image mode, always return 1
                // In multi-image mode, this would return the current image index
                Ok(Value::Integer(1))
            }

            Intrinsic::NumImages => {
                // In single-image mode, always return 1
                // In multi-image mode, this would return the total number of images
                Ok(Value::Integer(1))
            }

            Intrinsic::Present => {
                // PRESENT(a) - check if optional argument is present
                // In full implementation, this would check if argument was passed
                // For now, return true if argument is not Null
                let arg = self.pop(location)?;
                // Check if the value is "present" (not null)
                // For basic implementation, any non-null value is "present"
                let is_present = !matches!(arg, Value::Null);
                Ok(Value::Logical(is_present))
            }

            Intrinsic::Associated => {
                // ASSOCIATED(ptr) or ASSOCIATED(ptr, target)
                // Check if a pointer is associated with a target
                if arg_count == 1 {
                    let ptr = self.pop(location)?;
                    // A pointer is associated if it's a Reference (not Null)
                    let is_associated = matches!(ptr, Value::Reference(_));
                    Ok(Value::Logical(is_associated))
                } else {
                    // ASSOCIATED(ptr, target) - check if ptr points to target
                    let target = self.pop(location)?;
                    let ptr = self.pop(location)?;
                    // For now, just check if both are the same reference
                    let is_associated = match (ptr, target) {
                        (Value::Reference(p), Value::Reference(t)) => p == t,
                        _ => false,
                    };
                    Ok(Value::Logical(is_associated))
                }
            }

            Intrinsic::Null => {
                // NULL() or NULL(mold)
                // Return a null pointer value
                // If mold is provided, we just pop and ignore it (for type compatibility)
                if arg_count == 1 {
                    let _mold = self.pop(location)?;
                }
                Ok(Value::Null)
            }

            // ==================== Sprint 17: Numeric Inquiry Intrinsics ====================

            Intrinsic::Huge => {
                let arg = self.pop(location)?;
                match arg {
                    Value::Integer(_) => Ok(Value::Integer(i64::MAX)),
                    Value::Real(_) => Ok(Value::Real(f64::MAX)),
                    _ => Err(RuntimeError::TypeError {
                        message: "HUGE requires numeric argument".to_string(),
                        location,
                    }),
                }
            }

            Intrinsic::Tiny => {
                let _arg = self.pop(location)?;
                // TINY returns smallest positive real
                Ok(Value::Real(f64::MIN_POSITIVE))
            }

            Intrinsic::Epsilon => {
                let _arg = self.pop(location)?;
                // Machine epsilon for f64
                Ok(Value::Real(f64::EPSILON))
            }

            Intrinsic::Digits => {
                let arg = self.pop(location)?;
                match arg {
                    Value::Integer(_) => Ok(Value::Integer(63)),  // i64 has 63 value bits (1 sign)
                    Value::Real(_) => Ok(Value::Integer(53)),     // f64 mantissa has 53 bits
                    _ => Err(RuntimeError::TypeError {
                        message: "DIGITS requires numeric argument".to_string(),
                        location,
                    }),
                }
            }

            Intrinsic::Precision => {
                let _arg = self.pop(location)?;
                // f64 has approximately 15-17 decimal digits of precision
                Ok(Value::Integer(15))
            }

            Intrinsic::Range => {
                let arg = self.pop(location)?;
                match arg {
                    Value::Integer(_) => Ok(Value::Integer(18)),   // log10(2^63) ~ 18
                    Value::Real(_) => Ok(Value::Integer(307)),     // f64 exponent range
                    _ => Err(RuntimeError::TypeError {
                        message: "RANGE requires numeric argument".to_string(),
                        location,
                    }),
                }
            }

            Intrinsic::Radix => {
                let _arg = self.pop(location)?;
                // Binary representation
                Ok(Value::Integer(2))
            }

            Intrinsic::BitSize => {
                let arg = self.pop(location)?;
                match arg {
                    Value::Integer(_) => Ok(Value::Integer(64)),
                    _ => Err(RuntimeError::TypeError {
                        message: "BIT_SIZE requires integer argument".to_string(),
                        location,
                    }),
                }
            }

            // ==================== Sprint 17: Additional Bit Intrinsics ====================

            Intrinsic::Ishftc => {
                // ISHFTC(i, shift, [size]) - circular bit shift
                let size = if arg_count == 3 {
                    match self.pop(location)? {
                        Value::Integer(s) => s as u32,
                        _ => return Err(RuntimeError::TypeError {
                            message: "ISHFTC size must be integer".to_string(),
                            location,
                        }),
                    }
                } else {
                    64  // Default to full width
                };

                let shift = match self.pop(location)? {
                    Value::Integer(s) => s,
                    _ => return Err(RuntimeError::TypeError {
                        message: "ISHFTC shift must be integer".to_string(),
                        location,
                    }),
                };

                let i = match self.pop(location)? {
                    Value::Integer(v) => v as u64,
                    _ => return Err(RuntimeError::TypeError {
                        message: "ISHFTC requires integer argument".to_string(),
                        location,
                    }),
                };

                // Circular shift within 'size' bits
                let size = size.min(64);
                if size == 0 {
                    return Ok(Value::Integer(i as i64));
                }
                let mask = if size >= 64 { !0u64 } else { (1u64 << size) - 1 };
                let val = i & mask;
                let shift_mod = ((shift % size as i64) + size as i64) % size as i64;
                let result = if shift_mod == 0 {
                    val
                } else {
                    let left = (val << shift_mod) & mask;
                    let right = val >> (size as i64 - shift_mod);
                    left | right
                };
                Ok(Value::Integer(result as i64))
            }

            Intrinsic::Mvbits => {
                // MVBITS(from, frompos, len, to, topos)
                // Move len bits from position frompos in 'from' to position topos in 'to'
                let topos = match self.pop(location)? {
                    Value::Integer(v) => v as u32,
                    _ => return Err(RuntimeError::TypeError {
                        message: "MVBITS topos must be integer".to_string(),
                        location,
                    }),
                };
                let to = match self.pop(location)? {
                    Value::Integer(v) => v as u64,
                    _ => return Err(RuntimeError::TypeError {
                        message: "MVBITS to must be integer".to_string(),
                        location,
                    }),
                };
                let len = match self.pop(location)? {
                    Value::Integer(v) => v as u32,
                    _ => return Err(RuntimeError::TypeError {
                        message: "MVBITS len must be integer".to_string(),
                        location,
                    }),
                };
                let frompos = match self.pop(location)? {
                    Value::Integer(v) => v as u32,
                    _ => return Err(RuntimeError::TypeError {
                        message: "MVBITS frompos must be integer".to_string(),
                        location,
                    }),
                };
                let from = match self.pop(location)? {
                    Value::Integer(v) => v as u64,
                    _ => return Err(RuntimeError::TypeError {
                        message: "MVBITS from must be integer".to_string(),
                        location,
                    }),
                };

                // Extract bits from 'from' starting at frompos
                let mask = if len >= 64 { !0u64 } else { (1u64 << len) - 1 };
                let bits = (from >> frompos) & mask;
                // Clear target bits and set new ones
                let clear_mask = !(mask << topos);
                let result = (to & clear_mask) | (bits << topos);
                Ok(Value::Integer(result as i64))
            }

            // ==================== Sprint 17: Additional Character Intrinsics ====================

            Intrinsic::Scan => {
                // SCAN(string, set) - find first char from set
                let set = self.pop(location)?;
                let string = self.pop(location)?;
                match (&string, &set) {
                    (Value::Character(s), Value::Character(chars)) => {
                        let pos = s.chars()
                            .position(|c| chars.contains(c))
                            .map(|p| p + 1)  // Fortran 1-based indexing
                            .unwrap_or(0);
                        Ok(Value::Integer(pos as i64))
                    }
                    _ => Err(RuntimeError::TypeError {
                        message: "SCAN requires character arguments".to_string(),
                        location,
                    }),
                }
            }

            Intrinsic::Verify => {
                // VERIFY(string, set) - find first char NOT in set
                let set = self.pop(location)?;
                let string = self.pop(location)?;
                match (&string, &set) {
                    (Value::Character(s), Value::Character(chars)) => {
                        let pos = s.chars()
                            .position(|c| !chars.contains(c))
                            .map(|p| p + 1)  // Fortran 1-based indexing
                            .unwrap_or(0);
                        Ok(Value::Integer(pos as i64))
                    }
                    _ => Err(RuntimeError::TypeError {
                        message: "VERIFY requires character arguments".to_string(),
                        location,
                    }),
                }
            }

            Intrinsic::Achar => {
                // ACHAR(i) - ASCII code to character
                let arg = self.pop(location)?;
                match arg {
                    Value::Integer(i) => {
                        if i < 0 || i > 127 {
                            return Err(RuntimeError::TypeError {
                                message: format!("ACHAR: value {} out of ASCII range (0-127)", i),
                                location,
                            });
                        }
                        Ok(Value::Character((i as u8 as char).to_string()))
                    }
                    _ => Err(RuntimeError::TypeError {
                        message: "ACHAR requires integer argument".to_string(),
                        location,
                    }),
                }
            }

            Intrinsic::Iachar => {
                // IACHAR(c) - character to ASCII code
                let arg = self.pop(location)?;
                match arg {
                    Value::Character(s) => {
                        if s.is_empty() {
                            return Err(RuntimeError::TypeError {
                                message: "IACHAR requires non-empty string".to_string(),
                                location,
                            });
                        }
                        Ok(Value::Integer(s.chars().next().unwrap() as i64))
                    }
                    _ => Err(RuntimeError::TypeError {
                        message: "IACHAR requires character argument".to_string(),
                        location,
                    }),
                }
            }

            // ==================== Sprint 16: Array Inquiry Intrinsics ====================

            Intrinsic::Shape => {
                let arg = self.pop(location)?;
                match arg {
                    Value::Array { dims, .. } => {
                        let shape_elements: Vec<Value> = dims.iter()
                            .map(|d| Value::Integer(d.size() as i64))
                            .collect();
                        let shape_dims = vec![ArrayDim::new(1, shape_elements.len() as i64)];
                        Ok(Value::Array {
                            elements: shape_elements,
                            dims: shape_dims,
                        })
                    }
                    _ => {
                        // Scalar has empty shape
                        Ok(Value::Array {
                            elements: vec![],
                            dims: vec![ArrayDim::new(1, 0)],
                        })
                    }
                }
            }

            Intrinsic::Lbound => {
                let dim_arg = if arg_count == 2 {
                    Some(self.pop(location)?)
                } else {
                    None
                };
                let arr = self.pop(location)?;

                match arr {
                    Value::Array { dims, .. } => {
                        if let Some(Value::Integer(d)) = dim_arg {
                            // Return single dimension's lower bound
                            let idx = (d - 1) as usize;
                            if idx >= dims.len() {
                                return Err(RuntimeError::TypeError {
                                    message: format!("LBOUND: dimension {} out of range (1-{})", d, dims.len()),
                                    location,
                                });
                            }
                            Ok(Value::Integer(dims[idx].lower))
                        } else {
                            // Return array of all lower bounds
                            let bounds: Vec<Value> = dims.iter()
                                .map(|d| Value::Integer(d.lower))
                                .collect();
                            Ok(Value::Array {
                                elements: bounds.clone(),
                                dims: vec![ArrayDim::new(1, bounds.len() as i64)],
                            })
                        }
                    }
                    _ => Err(RuntimeError::TypeError {
                        message: "LBOUND requires array argument".to_string(),
                        location,
                    }),
                }
            }

            Intrinsic::Ubound => {
                let dim_arg = if arg_count == 2 {
                    Some(self.pop(location)?)
                } else {
                    None
                };
                let arr = self.pop(location)?;

                match arr {
                    Value::Array { dims, .. } => {
                        if let Some(Value::Integer(d)) = dim_arg {
                            // Return single dimension's upper bound
                            let idx = (d - 1) as usize;
                            if idx >= dims.len() {
                                return Err(RuntimeError::TypeError {
                                    message: format!("UBOUND: dimension {} out of range (1-{})", d, dims.len()),
                                    location,
                                });
                            }
                            Ok(Value::Integer(dims[idx].upper))
                        } else {
                            // Return array of all upper bounds
                            let bounds: Vec<Value> = dims.iter()
                                .map(|d| Value::Integer(d.upper))
                                .collect();
                            Ok(Value::Array {
                                elements: bounds.clone(),
                                dims: vec![ArrayDim::new(1, bounds.len() as i64)],
                            })
                        }
                    }
                    _ => Err(RuntimeError::TypeError {
                        message: "UBOUND requires array argument".to_string(),
                        location,
                    }),
                }
            }

            Intrinsic::Rank => {
                let arg = self.pop(location)?;
                match arg {
                    Value::Array { dims, .. } => Ok(Value::Integer(dims.len() as i64)),
                    _ => Ok(Value::Integer(0)),  // Scalars have rank 0
                }
            }

            Intrinsic::Any => {
                let arg = self.pop(location)?;
                match arg {
                    Value::Array { elements, .. } => {
                        let result = elements.iter().any(|e| match e {
                            Value::Logical(b) => *b,
                            _ => false,
                        });
                        Ok(Value::Logical(result))
                    }
                    Value::Logical(b) => Ok(Value::Logical(b)),
                    _ => Err(RuntimeError::TypeError {
                        message: "ANY requires logical array".to_string(),
                        location,
                    }),
                }
            }

            Intrinsic::All => {
                let arg = self.pop(location)?;
                match arg {
                    Value::Array { elements, .. } => {
                        let result = elements.iter().all(|e| match e {
                            Value::Logical(b) => *b,
                            _ => false,
                        });
                        Ok(Value::Logical(result))
                    }
                    Value::Logical(b) => Ok(Value::Logical(b)),
                    _ => Err(RuntimeError::TypeError {
                        message: "ALL requires logical array".to_string(),
                        location,
                    }),
                }
            }

            Intrinsic::Count => {
                let arg = self.pop(location)?;
                match arg {
                    Value::Array { elements, .. } => {
                        let count = elements.iter().filter(|e| match e {
                            Value::Logical(b) => *b,
                            _ => false,
                        }).count();
                        Ok(Value::Integer(count as i64))
                    }
                    Value::Logical(b) => Ok(Value::Integer(if b { 1 } else { 0 })),
                    _ => Err(RuntimeError::TypeError {
                        message: "COUNT requires logical array".to_string(),
                        location,
                    }),
                }
            }

            Intrinsic::Maxloc => {
                let arg = self.pop(location)?;
                match arg {
                    Value::Array { elements, dims, .. } => {
                        if elements.is_empty() {
                            return Err(RuntimeError::TypeError {
                                message: "MAXLOC: empty array".to_string(),
                                location,
                            });
                        }

                        let mut max_idx = 0;
                        let mut max_val = f64::NEG_INFINITY;

                        for (i, elem) in elements.iter().enumerate() {
                            let val = match elem {
                                Value::Integer(n) => *n as f64,
                                Value::Real(n) => *n,
                                _ => continue,
                            };
                            if val > max_val {
                                max_val = val;
                                max_idx = i;
                            }
                        }

                        // Convert flat index to multi-dimensional indices (Fortran 1-based)
                        let indices = self.flat_to_multidim(max_idx, &dims);
                        let result: Vec<Value> = indices.iter()
                            .map(|i| Value::Integer(*i))
                            .collect();

                        Ok(Value::Array {
                            elements: result.clone(),
                            dims: vec![ArrayDim::new(1, result.len() as i64)],
                        })
                    }
                    _ => Err(RuntimeError::TypeError {
                        message: "MAXLOC requires array argument".to_string(),
                        location,
                    }),
                }
            }

            Intrinsic::Minloc => {
                let arg = self.pop(location)?;
                match arg {
                    Value::Array { elements, dims, .. } => {
                        if elements.is_empty() {
                            return Err(RuntimeError::TypeError {
                                message: "MINLOC: empty array".to_string(),
                                location,
                            });
                        }

                        let mut min_idx = 0;
                        let mut min_val = f64::INFINITY;

                        for (i, elem) in elements.iter().enumerate() {
                            let val = match elem {
                                Value::Integer(n) => *n as f64,
                                Value::Real(n) => *n,
                                _ => continue,
                            };
                            if val < min_val {
                                min_val = val;
                                min_idx = i;
                            }
                        }

                        // Convert flat index to multi-dimensional indices (Fortran 1-based)
                        let indices = self.flat_to_multidim(min_idx, &dims);
                        let result: Vec<Value> = indices.iter()
                            .map(|i| Value::Integer(*i))
                            .collect();

                        Ok(Value::Array {
                            elements: result.clone(),
                            dims: vec![ArrayDim::new(1, result.len() as i64)],
                        })
                    }
                    _ => Err(RuntimeError::TypeError {
                        message: "MINLOC requires array argument".to_string(),
                        location,
                    }),
                }
            }

            Intrinsic::Allocated => {
                let arg = self.pop(location)?;
                // For our current implementation, arrays are always "allocated" once they exist
                let is_allocated = matches!(arg, Value::Array { .. });
                Ok(Value::Logical(is_allocated))
            }
        }
    }

    /// Helper function to convert flat index to multi-dimensional indices
    fn flat_to_multidim(&self, flat_idx: usize, dims: &[ArrayDim]) -> Vec<i64> {
        let mut indices = vec![0i64; dims.len()];
        let mut remaining = flat_idx;

        for (i, dim) in dims.iter().enumerate() {
            let size = dim.size();
            if size > 0 {
                indices[i] = (remaining % size) as i64 + dim.lower;
                remaining /= size;
            } else {
                indices[i] = dim.lower;
            }
        }

        indices
    }

    /// Pop a value from the stack and convert to f64
    fn pop_as_real(&mut self, location: SourceLocation) -> VMResult<f64> {
        let val = self.pop(location)?;
        self.value_to_real(&val, location)
    }

    /// Convert a value to f64
    fn value_to_real(&self, val: &Value, location: SourceLocation) -> VMResult<f64> {
        match val {
            Value::Integer(n) => Ok(*n as f64),
            Value::Real(n) => Ok(*n),
            _ => Err(RuntimeError::TypeError {
                message: "Expected numeric value".to_string(),
                location,
            }),
        }
    }

    /// Compare two values and return the maximum
    fn compare_max(&self, a: Value, b: Value, location: SourceLocation) -> VMResult<Value> {
        match (&a, &b) {
            (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer((*a).max(*b))),
            (Value::Real(a), Value::Real(b)) => Ok(Value::Real(a.max(*b))),
            (Value::Integer(a), Value::Real(b)) => Ok(Value::Real((*a as f64).max(*b))),
            (Value::Real(a), Value::Integer(b)) => Ok(Value::Real(a.max(*b as f64))),
            _ => Err(RuntimeError::TypeError {
                message: "MAX requires numeric arguments".to_string(),
                location,
            }),
        }
    }

    /// Compare two values and return the minimum
    fn compare_min(&self, a: Value, b: Value, location: SourceLocation) -> VMResult<Value> {
        match (&a, &b) {
            (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer((*a).min(*b))),
            (Value::Real(a), Value::Real(b)) => Ok(Value::Real(a.min(*b))),
            (Value::Integer(a), Value::Real(b)) => Ok(Value::Real((*a as f64).min(*b))),
            (Value::Real(a), Value::Integer(b)) => Ok(Value::Real(a.min(*b as f64))),
            _ => Err(RuntimeError::TypeError {
                message: "MIN requires numeric arguments".to_string(),
                location,
            }),
        }
    }

    // Stack operations

    fn push(&mut self, value: Value, location: SourceLocation) -> VMResult<()> {
        if self.stack.len() >= STACK_SIZE {
            return Err(RuntimeError::StackOverflow { location });
        }
        self.stack.push(value);
        Ok(())
    }

    fn pop(&mut self, location: SourceLocation) -> VMResult<Value> {
        self.stack
            .pop()
            .ok_or(RuntimeError::StackUnderflow { location })
    }

    fn peek(&self, location: SourceLocation) -> VMResult<&Value> {
        self.stack
            .last()
            .ok_or(RuntimeError::StackUnderflow { location })
    }

    // Variable operations

    /// Resolve a reference chain - if the variable at `index` contains a Reference,
    /// follow it to get the target variable index.
    fn resolve_reference(&self, index: usize, location: SourceLocation) -> VMResult<usize> {
        if let Some(Some(Value::Reference(target))) = self.variables.get(index) {
            // This variable is a reference, follow it
            Ok(*target)
        } else {
            // Not a reference, return the original index
            Ok(index)
        }
    }

    fn get_variable_by_index(&self, index: usize, location: SourceLocation) -> VMResult<Value> {
        self.get_variable_by_index_with_depth(index, location, 0)
    }

    fn get_variable_by_index_with_depth(&self, index: usize, location: SourceLocation, depth: usize) -> VMResult<Value> {
        // Prevent infinite recursion in case of reference cycles
        if depth > 100 {
            return Err(RuntimeError::TypeError {
                message: "Reference cycle detected".to_string(),
                location,
            });
        }

        // First resolve any reference
        let actual_index = self.resolve_reference(index, location)?;

        let value = self
            .variables
            .get(actual_index)
            .ok_or(RuntimeError::InvalidVariable { index: actual_index, location })?
            .clone()
            .unwrap_or(Value::Integer(0)); // Uninitialized variables default to 0

        // If the value is itself a Reference, follow it (shouldn't happen after resolve)
        match value {
            Value::Reference(target) => self.get_variable_by_index_with_depth(target, location, depth + 1),
            _ => Ok(value),
        }
    }

    fn set_variable_by_index(
        &mut self,
        index: usize,
        value: Value,
        location: SourceLocation,
    ) -> VMResult<()> {
        // First resolve any reference
        let actual_index = self.resolve_reference(index, location)?;

        if actual_index >= self.variables.len() {
            // Extend if needed
            self.variables.resize(actual_index + 1, None);
        }
        self.variables[actual_index] = Some(value);
        Ok(())
    }

    /// Pop array indices from stack
    /// Stack should contain: num_indices, index1, index2, ...
    fn pop_indices(&mut self, location: SourceLocation) -> VMResult<Vec<i64>> {
        // Pop indices in reverse order
        let mut indices = Vec::new();

        // First, collect all indices from stack
        // The stack has: ... index_n, index_n-1, ..., index_1, num_indices
        // We need to pop num_indices first
        let num_indices = match self.pop(location)? {
            Value::Integer(n) => n as usize,
            _ => return Err(RuntimeError::TypeError {
                message: "Expected integer for index count".to_string(),
                location,
            }),
        };

        // Pop each index (they come in reverse order)
        for _ in 0..num_indices {
            let idx = match self.pop(location)? {
                Value::Integer(n) => n,
                _ => return Err(RuntimeError::TypeError {
                    message: "Expected integer for array index".to_string(),
                    location,
                }),
            };
            indices.push(idx);
        }

        // Reverse to get correct order
        indices.reverse();

        Ok(indices)
    }

    /// Extract an array section based on slice specifications
    fn extract_array_section(&self, array: &Value, specs: &[SliceSpec], location: SourceLocation) -> VMResult<Value> {
        match array {
            Value::Array { elements, dims } => {
                // Get array dimensions info
                let dim_bounds: Vec<(i64, i64)> = dims.iter()
                    .map(|d| (d.lower, d.upper))
                    .collect();

                if specs.len() != dim_bounds.len() {
                    return Err(RuntimeError::IndexOutOfBounds {
                        message: format!("Expected {} subscripts, got {}", dim_bounds.len(), specs.len()),
                        location,
                    });
                }

                // Process each dimension and determine resulting shape
                let mut result_dims: Vec<ArrayDim> = Vec::new();
                let mut slice_ranges: Vec<Vec<i64>> = Vec::new();

                for (spec, (lower, upper)) in specs.iter().zip(dim_bounds.iter()) {
                    match spec {
                        SliceSpec::Index(idx) => {
                            // Single index - this dimension is eliminated
                            if *idx < *lower || *idx > *upper {
                                return Err(RuntimeError::IndexOutOfBounds {
                                    message: format!("Index {} out of bounds [{}, {}]", idx, lower, upper),
                                    location,
                                });
                            }
                            slice_ranges.push(vec![*idx]);
                        }
                        SliceSpec::Slice { start, end, step } => {
                            // Slice - dimension is preserved (possibly with different size)
                            let start_idx = start.unwrap_or(*lower);
                            let end_idx = end.unwrap_or(*upper);
                            let step_val = step.unwrap_or(1);

                            if step_val == 0 {
                                return Err(RuntimeError::TypeError {
                                    message: "Slice step cannot be zero".to_string(),
                                    location,
                                });
                            }

                            // Generate indices for this slice
                            let mut indices = Vec::new();
                            if step_val > 0 {
                                let mut i = start_idx;
                                while i <= end_idx {
                                    if i >= *lower && i <= *upper {
                                        indices.push(i);
                                    }
                                    i += step_val;
                                }
                            } else {
                                let mut i = start_idx;
                                while i >= end_idx {
                                    if i >= *lower && i <= *upper {
                                        indices.push(i);
                                    }
                                    i += step_val;
                                }
                            }

                            if !indices.is_empty() {
                                result_dims.push(ArrayDim {
                                    lower: 1,
                                    upper: indices.len() as i64,
                                });
                            }
                            slice_ranges.push(indices);
                        }
                    }
                }

                // Extract elements based on computed ranges
                let mut result_elements = Vec::new();
                self.extract_elements_recursive(elements, dims, &slice_ranges, 0, &mut result_elements);

                // If all dimensions were eliminated (all single indices), return scalar
                if result_dims.is_empty() {
                    if result_elements.len() == 1 {
                        return Ok(result_elements.into_iter().next().unwrap());
                    }
                }

                Ok(Value::Array {
                    elements: result_elements,
                    dims: result_dims,
                })
            }
            _ => Err(RuntimeError::TypeError {
                message: "Cannot slice non-array value".to_string(),
                location,
            }),
        }
    }

    /// Recursively extract elements from an array based on slice ranges
    fn extract_elements_recursive(
        &self,
        elements: &[Value],
        dimensions: &[ArrayDim],
        slice_ranges: &[Vec<i64>],
        dim_idx: usize,
        result: &mut Vec<Value>,
    ) {
        if dim_idx >= dimensions.len() {
            return;
        }

        let dim = &dimensions[dim_idx];
        let range = &slice_ranges[dim_idx];

        // Calculate stride for this dimension
        let stride: usize = dimensions[dim_idx + 1..].iter()
            .map(|d| (d.upper - d.lower + 1) as usize)
            .product::<usize>()
            .max(1);

        for &idx in range {
            let offset = ((idx - dim.lower) as usize) * stride;

            if dim_idx == dimensions.len() - 1 {
                // Last dimension - copy elements directly
                if offset < elements.len() {
                    result.push(elements[offset].clone());
                }
            } else {
                // Recurse for inner dimensions
                let sub_elements = if offset + stride <= elements.len() {
                    &elements[offset..offset + stride]
                } else if offset < elements.len() {
                    &elements[offset..]
                } else {
                    &[]
                };
                self.extract_elements_recursive(sub_elements, &dimensions[dim_idx + 1..], &slice_ranges[dim_idx + 1..], 0, result);
            }
        }
    }

    // I/O operations

    /// Read a value from the input buffer or stdin
    fn read_value(&mut self, location: SourceLocation) -> VMResult<Value> {
        let input = if self.input_pos < self.input_buffer.len() {
            // Read from input buffer (for testing)
            let line = self.input_buffer[self.input_pos].clone();
            self.input_pos += 1;
            line
        } else {
            // Read from stdin
            let mut line = String::new();
            io::stdin().read_line(&mut line).map_err(|e| RuntimeError::IoError {
                message: format!("Read error: {}", e),
                location,
            })?;
            line.trim().to_string()
        };

        // Try to parse as integer, then real, then keep as string
        self.parse_value(&input)
    }

    /// Read a value from a file
    fn read_from_file(&mut self, unit: i64, location: SourceLocation) -> VMResult<Value> {
        if let Some(handle) = self.file_handles.get_mut(&unit) {
            if !handle.can_read() {
                return Err(RuntimeError::IoError {
                    message: format!("Unit {} not open for reading", unit),
                    location,
                });
            }

            match handle.read_line() {
                Ok(Some(line)) => self.parse_value(&line),
                Ok(None) => Err(RuntimeError::IoError {
                    message: format!("End of file on unit {}", unit),
                    location,
                }),
                Err(e) => Err(RuntimeError::IoError {
                    message: format!("Read error on unit {}: {}", unit, e),
                    location,
                }),
            }
        } else {
            Err(RuntimeError::IoError {
                message: format!("Unit {} not open", unit),
                location,
            })
        }
    }

    /// Parse a string value into the appropriate type
    fn parse_value(&self, input: &str) -> VMResult<Value> {
        let trimmed = input.trim();

        // Try integer first
        if let Ok(i) = trimmed.parse::<i64>() {
            return Ok(Value::Integer(i));
        }

        // Try real
        if let Ok(r) = trimmed.parse::<f64>() {
            return Ok(Value::Real(r));
        }

        // Try logical
        let upper = trimmed.to_uppercase();
        if upper == ".TRUE." || upper == "T" || upper == "TRUE" {
            return Ok(Value::Logical(true));
        }
        if upper == ".FALSE." || upper == "F" || upper == "FALSE" {
            return Ok(Value::Logical(false));
        }

        // Keep as character
        Ok(Value::Character(trimmed.to_string()))
    }

    // Constant operations

    fn get_constant(&self, index: usize, location: SourceLocation) -> VMResult<Value> {
        let chunk = self.chunk.as_ref().unwrap();
        chunk
            .constants
            .get(index)
            .cloned()
            .ok_or(RuntimeError::InvalidConstant { index, location })
    }

    // Binary operations helper

    fn binary_op<F>(&mut self, op: F, location: SourceLocation) -> VMResult<()>
    where
        F: FnOnce(Value, Value) -> Value,
    {
        let b = self.pop(location)?;
        let a = self.pop(location)?;
        let result = op(a, b);
        self.push(result, location)
    }

    fn binary_op_checked<F>(&mut self, op: F, location: SourceLocation) -> VMResult<()>
    where
        F: FnOnce(Value, Value, SourceLocation) -> VMResult<Value>,
    {
        let b = self.pop(location)?;
        let a = self.pop(location)?;
        let result = op(a, b, location)?;
        self.push(result, location)
    }

    fn comparison_op<F>(&mut self, op: F, location: SourceLocation) -> VMResult<()>
    where
        F: FnOnce(&Value, &Value) -> bool,
    {
        let b = self.pop(location)?;
        let a = self.pop(location)?;
        let result = Value::Logical(op(&a, &b));
        self.push(result, location)
    }

    // Arithmetic operations

    fn add_values(a: Value, b: Value) -> Value {
        match (a, b) {
            (Value::Integer(x), Value::Integer(y)) => Value::Integer(x + y),
            (Value::Real(x), Value::Real(y)) => Value::Real(x + y),
            (Value::Integer(x), Value::Real(y)) => Value::Real(x as f64 + y),
            (Value::Real(x), Value::Integer(y)) => Value::Real(x + y as f64),
            (Value::Character(x), Value::Character(y)) => Value::Character(x + &y),
            (a, b) => {
                // Fallback: try to convert to real
                let av = Self::to_real(&a);
                let bv = Self::to_real(&b);
                Value::Real(av + bv)
            }
        }
    }

    fn subtract_values(a: Value, b: Value) -> Value {
        match (a, b) {
            (Value::Integer(x), Value::Integer(y)) => Value::Integer(x - y),
            (Value::Real(x), Value::Real(y)) => Value::Real(x - y),
            (Value::Integer(x), Value::Real(y)) => Value::Real(x as f64 - y),
            (Value::Real(x), Value::Integer(y)) => Value::Real(x - y as f64),
            (a, b) => {
                let av = Self::to_real(&a);
                let bv = Self::to_real(&b);
                Value::Real(av - bv)
            }
        }
    }

    fn multiply_values(a: Value, b: Value) -> Value {
        match (a, b) {
            (Value::Integer(x), Value::Integer(y)) => Value::Integer(x * y),
            (Value::Real(x), Value::Real(y)) => Value::Real(x * y),
            (Value::Integer(x), Value::Real(y)) => Value::Real(x as f64 * y),
            (Value::Real(x), Value::Integer(y)) => Value::Real(x * y as f64),
            (a, b) => {
                let av = Self::to_real(&a);
                let bv = Self::to_real(&b);
                Value::Real(av * bv)
            }
        }
    }

    fn divide_values(a: Value, b: Value, location: SourceLocation) -> VMResult<Value> {
        // Check for division by zero
        let is_zero = match &b {
            Value::Integer(0) => true,
            Value::Real(x) if *x == 0.0 => true,
            _ => false,
        };

        if is_zero {
            return Err(RuntimeError::DivisionByZero { location });
        }

        let result = match (a, b) {
            (Value::Integer(x), Value::Integer(y)) => Value::Integer(x / y),
            (Value::Real(x), Value::Real(y)) => Value::Real(x / y),
            (Value::Integer(x), Value::Real(y)) => Value::Real(x as f64 / y),
            (Value::Real(x), Value::Integer(y)) => Value::Real(x / y as f64),
            (a, b) => {
                let av = Self::to_real(&a);
                let bv = Self::to_real(&b);
                Value::Real(av / bv)
            }
        };

        Ok(result)
    }

    fn power_values(a: Value, b: Value) -> Value {
        match (a, b) {
            (Value::Integer(x), Value::Integer(y)) => {
                if y >= 0 {
                    Value::Integer(x.pow(y as u32))
                } else {
                    Value::Real((x as f64).powi(y as i32))
                }
            }
            (Value::Real(x), Value::Integer(y)) => Value::Real(x.powi(y as i32)),
            (Value::Integer(x), Value::Real(y)) => Value::Real((x as f64).powf(y)),
            (Value::Real(x), Value::Real(y)) => Value::Real(x.powf(y)),
            (a, b) => {
                let av = Self::to_real(&a);
                let bv = Self::to_real(&b);
                Value::Real(av.powf(bv))
            }
        }
    }

    fn negate_value(value: Value, location: SourceLocation) -> VMResult<Value> {
        match value {
            Value::Integer(x) => Ok(Value::Integer(-x)),
            Value::Real(x) => Ok(Value::Real(-x)),
            _ => Err(RuntimeError::TypeError {
                message: format!("Cannot negate {}", value.type_name()),
                location,
            }),
        }
    }

    // Comparison operations

    fn values_equal(a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Integer(x), Value::Integer(y)) => x == y,
            (Value::Real(x), Value::Real(y)) => (x - y).abs() < f64::EPSILON,
            (Value::Integer(x), Value::Real(y)) => (*x as f64 - y).abs() < f64::EPSILON,
            (Value::Real(x), Value::Integer(y)) => (x - *y as f64).abs() < f64::EPSILON,
            (Value::Logical(x), Value::Logical(y)) => x == y,
            (Value::Character(x), Value::Character(y)) => x == y,
            _ => false,
        }
    }

    fn values_less(a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Integer(x), Value::Integer(y)) => x < y,
            (Value::Real(x), Value::Real(y)) => x < y,
            (Value::Integer(x), Value::Real(y)) => (*x as f64) < *y,
            (Value::Real(x), Value::Integer(y)) => *x < (*y as f64),
            (Value::Character(x), Value::Character(y)) => x < y,
            _ => false,
        }
    }

    fn values_less_equal(a: &Value, b: &Value) -> bool {
        Self::values_less(a, b) || Self::values_equal(a, b)
    }

    fn values_greater(a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Integer(x), Value::Integer(y)) => x > y,
            (Value::Real(x), Value::Real(y)) => x > y,
            (Value::Integer(x), Value::Real(y)) => (*x as f64) > *y,
            (Value::Real(x), Value::Integer(y)) => *x > (*y as f64),
            (Value::Character(x), Value::Character(y)) => x > y,
            _ => false,
        }
    }

    fn values_greater_equal(a: &Value, b: &Value) -> bool {
        Self::values_greater(a, b) || Self::values_equal(a, b)
    }

    // Logical operations

    fn logical_and(a: Value, b: Value, location: SourceLocation) -> VMResult<Value> {
        match (a, b) {
            (Value::Logical(x), Value::Logical(y)) => Ok(Value::Logical(x && y)),
            _ => Err(RuntimeError::TypeError {
                message: "AND requires logical operands".to_string(),
                location,
            }),
        }
    }

    fn logical_or(a: Value, b: Value, location: SourceLocation) -> VMResult<Value> {
        match (a, b) {
            (Value::Logical(x), Value::Logical(y)) => Ok(Value::Logical(x || y)),
            _ => Err(RuntimeError::TypeError {
                message: "OR requires logical operands".to_string(),
                location,
            }),
        }
    }

    fn logical_not(value: Value, location: SourceLocation) -> VMResult<Value> {
        match value {
            Value::Logical(x) => Ok(Value::Logical(!x)),
            _ => Err(RuntimeError::TypeError {
                message: "NOT requires logical operand".to_string(),
                location,
            }),
        }
    }

    // Helper functions

    fn is_truthy(value: &Value) -> bool {
        match value {
            Value::Logical(b) => *b,
            Value::Integer(i) => *i != 0,
            Value::Real(r) => *r != 0.0,
            Value::Character(s) => !s.is_empty(),
            Value::Array { elements, .. } => !elements.is_empty(),
            Value::Instance { .. } => true, // Instances are always truthy
            Value::Null => false, // Null is falsy
            Value::Reference(_) => true, // References are truthy (they exist)
        }
    }

    fn to_real(value: &Value) -> f64 {
        match value {
            Value::Integer(i) => *i as f64,
            Value::Real(r) => *r,
            Value::Logical(b) => if *b { 1.0 } else { 0.0 },
            Value::Character(_) => 0.0,
            Value::Array { .. } => 0.0, // Arrays can't be converted to real
            Value::Instance { .. } => 0.0, // Instances can't be converted to real
            Value::Null => 0.0, // Null can't be converted to real
            Value::Reference(_) => 0.0, // References can't be converted to real
        }
    }

    // Debugging

    fn trace_instruction(&self, instruction: &Instruction) {
        let chunk = self.chunk.as_ref().unwrap();

        // Print stack
        print!("          ");
        for value in &self.stack {
            print!("[ {} ]", value);
        }
        println!();

        // Print instruction
        let operand_str = match instruction.operand {
            Some(op) => {
                match instruction.opcode {
                    OpCode::LoadConst => {
                        if let Some(val) = chunk.constants.get(op) {
                            format!("{} ; {}", op, val)
                        } else {
                            format!("{}", op)
                        }
                    }
                    OpCode::LoadVar | OpCode::StoreVar => {
                        if let Some(name) = chunk.variables.get(op) {
                            format!("{} ; {}", op, name)
                        } else {
                            format!("{}", op)
                        }
                    }
                    _ => format!("{}", op),
                }
            }
            None => String::new(),
        };

        println!("{:04} {:12} {}", self.ip, instruction.opcode, operand_str);
    }

    /// Dump the current stack state
    pub fn dump_stack(&self) -> String {
        let mut output = String::from("Stack:\n");
        for (i, value) in self.stack.iter().enumerate() {
            output.push_str(&format!("  [{}] = {}\n", i, value));
        }
        output
    }

    /// Dump all variables
    pub fn dump_variables(&self) -> String {
        let mut output = String::from("Variables:\n");
        if let Some(chunk) = &self.chunk {
            for (i, name) in chunk.variables.iter().enumerate() {
                let value = self
                    .variables
                    .get(i)
                    .and_then(|v| v.as_ref())
                    .map(|v| format!("{}", v))
                    .unwrap_or_else(|| "<uninitialized>".to_string());
                output.push_str(&format!("  {} = {}\n", name, value));
            }
        }
        output
    }
}

impl Default for VM {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_chunk() -> Chunk {
        Chunk::new()
    }

    #[test]
    fn test_vm_new() {
        let vm = VM::new();
        assert!(vm.stack.is_empty());
        assert!(vm.variables.is_empty());
        assert_eq!(vm.ip, 0);
    }

    #[test]
    fn test_load_const() {
        let mut chunk = make_chunk();
        let loc = SourceLocation { line: 1, column: 1 };

        let idx = chunk.add_constant(Value::Integer(42));
        chunk.emit_with_operand(OpCode::LoadConst, idx, loc);
        chunk.emit(OpCode::Halt, loc);

        let mut vm = VM::new();
        vm.run(chunk).unwrap();

        assert_eq!(vm.stack.len(), 1);
        assert_eq!(vm.stack[0], Value::Integer(42));
    }

    #[test]
    fn test_store_load_var() {
        let mut chunk = make_chunk();
        let loc = SourceLocation { line: 1, column: 1 };

        let const_idx = chunk.add_constant(Value::Integer(100));
        let var_idx = chunk.add_variable("X".to_string());

        chunk.emit_with_operand(OpCode::LoadConst, const_idx, loc);
        chunk.emit_with_operand(OpCode::StoreVar, var_idx, loc);
        chunk.emit_with_operand(OpCode::LoadVar, var_idx, loc);
        chunk.emit(OpCode::Halt, loc);

        let mut vm = VM::new();
        vm.run(chunk).unwrap();

        assert_eq!(vm.stack.len(), 1);
        assert_eq!(vm.stack[0], Value::Integer(100));
        assert_eq!(vm.get_variable("X"), Some(&Value::Integer(100)));
    }

    #[test]
    fn test_arithmetic_add() {
        let mut chunk = make_chunk();
        let loc = SourceLocation { line: 1, column: 1 };

        let idx1 = chunk.add_constant(Value::Integer(5));
        let idx2 = chunk.add_constant(Value::Integer(3));
        chunk.emit_with_operand(OpCode::LoadConst, idx1, loc);
        chunk.emit_with_operand(OpCode::LoadConst, idx2, loc);
        chunk.emit(OpCode::Add, loc);
        chunk.emit(OpCode::Halt, loc);

        let mut vm = VM::new();
        vm.run(chunk).unwrap();

        assert_eq!(vm.stack[0], Value::Integer(8));
    }

    #[test]
    fn test_arithmetic_subtract() {
        let mut chunk = make_chunk();
        let loc = SourceLocation { line: 1, column: 1 };

        let idx1 = chunk.add_constant(Value::Integer(10));
        let idx2 = chunk.add_constant(Value::Integer(3));
        chunk.emit_with_operand(OpCode::LoadConst, idx1, loc);
        chunk.emit_with_operand(OpCode::LoadConst, idx2, loc);
        chunk.emit(OpCode::Subtract, loc);
        chunk.emit(OpCode::Halt, loc);

        let mut vm = VM::new();
        vm.run(chunk).unwrap();

        assert_eq!(vm.stack[0], Value::Integer(7));
    }

    #[test]
    fn test_arithmetic_multiply() {
        let mut chunk = make_chunk();
        let loc = SourceLocation { line: 1, column: 1 };

        let idx1 = chunk.add_constant(Value::Integer(6));
        let idx2 = chunk.add_constant(Value::Integer(7));
        chunk.emit_with_operand(OpCode::LoadConst, idx1, loc);
        chunk.emit_with_operand(OpCode::LoadConst, idx2, loc);
        chunk.emit(OpCode::Multiply, loc);
        chunk.emit(OpCode::Halt, loc);

        let mut vm = VM::new();
        vm.run(chunk).unwrap();

        assert_eq!(vm.stack[0], Value::Integer(42));
    }

    #[test]
    fn test_arithmetic_divide() {
        let mut chunk = make_chunk();
        let loc = SourceLocation { line: 1, column: 1 };

        let idx1 = chunk.add_constant(Value::Integer(20));
        let idx2 = chunk.add_constant(Value::Integer(4));
        chunk.emit_with_operand(OpCode::LoadConst, idx1, loc);
        chunk.emit_with_operand(OpCode::LoadConst, idx2, loc);
        chunk.emit(OpCode::Divide, loc);
        chunk.emit(OpCode::Halt, loc);

        let mut vm = VM::new();
        vm.run(chunk).unwrap();

        assert_eq!(vm.stack[0], Value::Integer(5));
    }

    #[test]
    fn test_division_by_zero() {
        let mut chunk = make_chunk();
        let loc = SourceLocation { line: 1, column: 1 };

        let idx1 = chunk.add_constant(Value::Integer(10));
        let idx2 = chunk.add_constant(Value::Integer(0));
        chunk.emit_with_operand(OpCode::LoadConst, idx1, loc);
        chunk.emit_with_operand(OpCode::LoadConst, idx2, loc);
        chunk.emit(OpCode::Divide, loc);
        chunk.emit(OpCode::Halt, loc);

        let mut vm = VM::new();
        let result = vm.run(chunk);

        assert!(matches!(result, Err(RuntimeError::DivisionByZero { .. })));
    }

    #[test]
    fn test_mixed_type_arithmetic() {
        let mut chunk = make_chunk();
        let loc = SourceLocation { line: 1, column: 1 };

        // 5 + 2.5 = 7.5
        let idx1 = chunk.add_constant(Value::Integer(5));
        let idx2 = chunk.add_constant(Value::Real(2.5));
        chunk.emit_with_operand(OpCode::LoadConst, idx1, loc);
        chunk.emit_with_operand(OpCode::LoadConst, idx2, loc);
        chunk.emit(OpCode::Add, loc);
        chunk.emit(OpCode::Halt, loc);

        let mut vm = VM::new();
        vm.run(chunk).unwrap();

        assert_eq!(vm.stack[0], Value::Real(7.5));
    }

    #[test]
    fn test_comparison_greater() {
        let mut chunk = make_chunk();
        let loc = SourceLocation { line: 1, column: 1 };

        let idx1 = chunk.add_constant(Value::Integer(10));
        let idx2 = chunk.add_constant(Value::Integer(5));
        chunk.emit_with_operand(OpCode::LoadConst, idx1, loc);
        chunk.emit_with_operand(OpCode::LoadConst, idx2, loc);
        chunk.emit(OpCode::Greater, loc);
        chunk.emit(OpCode::Halt, loc);

        let mut vm = VM::new();
        vm.run(chunk).unwrap();

        assert_eq!(vm.stack[0], Value::Logical(true));
    }

    #[test]
    fn test_logical_and() {
        let mut chunk = make_chunk();
        let loc = SourceLocation { line: 1, column: 1 };

        chunk.emit(OpCode::LoadTrue, loc);
        chunk.emit(OpCode::LoadFalse, loc);
        chunk.emit(OpCode::And, loc);
        chunk.emit(OpCode::Halt, loc);

        let mut vm = VM::new();
        vm.run(chunk).unwrap();

        assert_eq!(vm.stack[0], Value::Logical(false));
    }

    #[test]
    fn test_jump_if_false() {
        let mut chunk = make_chunk();
        let loc = SourceLocation { line: 1, column: 1 };

        // if false, jump to end; else push 1
        let idx1 = chunk.add_constant(Value::Integer(1));
        let idx2 = chunk.add_constant(Value::Integer(2));
        chunk.emit(OpCode::LoadFalse, loc);            // 0
        chunk.emit_with_operand(OpCode::JumpIfFalse, 4, loc); // 1 -> jump to 4
        chunk.emit_with_operand(OpCode::LoadConst, idx1, loc); // 2 (skipped)
        chunk.emit_with_operand(OpCode::Jump, 5, loc); // 3 (skipped)
        chunk.emit_with_operand(OpCode::LoadConst, idx2, loc); // 4
        chunk.emit(OpCode::Halt, loc);                 // 5

        let mut vm = VM::new();
        vm.run(chunk).unwrap();

        assert_eq!(vm.stack[0], Value::Integer(2));
    }

    #[test]
    fn test_print() {
        let mut chunk = make_chunk();
        let loc = SourceLocation { line: 1, column: 1 };

        let idx1 = chunk.add_constant(Value::Integer(42));
        let idx2 = chunk.add_constant(Value::Character("Hello".to_string()));
        chunk.emit_with_operand(OpCode::LoadConst, idx1, loc);
        chunk.emit_with_operand(OpCode::LoadConst, idx2, loc);
        chunk.emit_with_operand(OpCode::Print, 2, loc);
        chunk.emit(OpCode::Halt, loc);

        let mut vm = VM::new();
        vm.run(chunk).unwrap();

        assert_eq!(vm.output().len(), 1);
        assert_eq!(vm.output()[0], "42 'Hello'");
    }
}
