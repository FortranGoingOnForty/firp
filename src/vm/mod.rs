//! Virtual Machine for bytecode execution
//!
//! This module implements a stack-based virtual machine that executes
//! compiled Fortran bytecode.

use crate::bytecode::{Chunk, Instruction, OpCode, Value};
use crate::lexer::SourceLocation;
use std::fmt;

/// Maximum stack size
const STACK_SIZE: usize = 256;

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
        }
    }
}

impl std::error::Error for RuntimeError {}

pub type VMResult<T> = Result<T, RuntimeError>;

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
        }
    }

    /// Enable or disable trace mode
    pub fn set_trace(&mut self, enabled: bool) {
        self.trace = enabled;
    }

    /// Reset the VM state
    pub fn reset(&mut self) {
        self.ip = 0;
        self.stack.clear();
        self.variables.clear();
        self.chunk = None;
        self.output.clear();
    }

    /// Run a compiled chunk
    pub fn run(&mut self, chunk: Chunk) -> VMResult<()> {
        self.reset();

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
            }
        }

        Ok(())
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

    fn get_variable_by_index(&self, index: usize, location: SourceLocation) -> VMResult<Value> {
        let value = self
            .variables
            .get(index)
            .ok_or(RuntimeError::InvalidVariable { index, location })?
            .clone()
            .unwrap_or(Value::Integer(0)); // Uninitialized variables default to 0
        Ok(value)
    }

    fn set_variable_by_index(
        &mut self,
        index: usize,
        value: Value,
        _location: SourceLocation,
    ) -> VMResult<()> {
        if index >= self.variables.len() {
            // Extend if needed
            self.variables.resize(index + 1, None);
        }
        self.variables[index] = Some(value);
        Ok(())
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
        }
    }

    fn to_real(value: &Value) -> f64 {
        match value {
            Value::Integer(i) => *i as f64,
            Value::Real(r) => *r,
            Value::Logical(b) => if *b { 1.0 } else { 0.0 },
            Value::Character(_) => 0.0,
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
