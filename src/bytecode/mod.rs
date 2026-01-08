//! Bytecode instruction set and compiler
//!
//! This module defines the bytecode instruction set for the FIRP VM
//! and provides a compiler to transform AST into bytecode.

use crate::ast::*;
use crate::lexer::SourceLocation;
use std::collections::{HashMap, HashSet};
use std::fmt;

/// Bytecode operation codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpCode {
    // Constants
    /// Load a constant from the constant pool
    LoadConst,
    /// Load boolean true
    LoadTrue,
    /// Load boolean false
    LoadFalse,

    // Variables
    /// Load a variable onto the stack
    LoadVar,
    /// Store top of stack into a variable
    StoreVar,

    // Arithmetic operations
    /// Add two values
    Add,
    /// Subtract two values
    Subtract,
    /// Multiply two values
    Multiply,
    /// Divide two values
    Divide,
    /// Power (exponentiation)
    Power,
    /// Negate a value (unary minus)
    Negate,

    // Comparison operations
    /// Equality comparison
    Equal,
    /// Not equal comparison
    NotEqual,
    /// Less than comparison
    Less,
    /// Less than or equal comparison
    LessEqual,
    /// Greater than comparison
    Greater,
    /// Greater than or equal comparison
    GreaterEqual,

    // Logical operations
    /// Logical AND
    And,
    /// Logical OR
    Or,
    /// Logical NOT
    Not,

    // Type conversions
    /// Convert integer to real
    IntToReal,
    /// Convert real to integer
    RealToInt,

    // Control flow
    /// Unconditional jump
    Jump,
    /// Jump if top of stack is false
    JumpIfFalse,
    /// Jump if top of stack is true
    JumpIfTrue,

    // Stack operations
    /// Pop top of stack (discard value)
    Pop,
    /// Duplicate top of stack
    Dup,

    // I/O operations
    /// Print values to stdout (operand = count of values)
    Print,
    /// Read value from stdin into variable (operand = variable index)
    Read,
    /// Open a file (operand = unit number, expects filename on stack)
    OpenFile,
    /// Close a file (operand = unit number)
    CloseFile,
    /// Write values to file (operand = count of values, expects unit on stack)
    WriteFile,
    /// Read value from file into variable (operand = variable index, expects unit on stack)
    ReadFile,

    // Program control
    /// Halt execution
    Halt,
    /// No operation
    Nop,

    // Subroutine/function operations
    /// Call a subroutine/function (operand = procedure index)
    Call,
    /// Return from a subroutine/function
    Return,

    // Array operations
    /// Allocate array storage (operand = variable index, expects dims on stack)
    AllocArray,
    /// Load element from array (operand = variable index, expects indices on stack)
    LoadArrayElem,
    /// Store value into array element (operand = variable index, expects value and indices on stack)
    StoreArrayElem,

    // Intrinsic function operations
    /// Call an intrinsic function (operand = Intrinsic enum value, expects args on stack)
    CallIntrinsic,

    // Derived type operations
    /// Create a new instance of a derived type (operand = type index)
    CreateInstance,
    /// Load component from instance (operand = component index, expects instance on stack)
    LoadComponent,
    /// Store value to component (operand = component index, expects value and instance on stack)
    StoreComponent,
}

/// Intrinsic functions available in Fortran
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum Intrinsic {
    // Mathematical functions
    Sqrt = 0,
    Abs,
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    Atan2,
    Exp,
    Log,
    Log10,

    // Utility functions
    Mod,
    Modulo,
    Max,
    Min,
    Floor,
    Ceiling,
    Nint,
    Sign,

    // Type conversion functions
    Int,
    Real,
    Dble,

    // Array intrinsics
    Sum,
    Product,
    Size,
    Maxval,
    Minval,
    DotProduct,
}

impl Intrinsic {
    /// Get intrinsic from its numeric value
    pub fn from_usize(value: usize) -> Option<Intrinsic> {
        match value {
            0 => Some(Intrinsic::Sqrt),
            1 => Some(Intrinsic::Abs),
            2 => Some(Intrinsic::Sin),
            3 => Some(Intrinsic::Cos),
            4 => Some(Intrinsic::Tan),
            5 => Some(Intrinsic::Asin),
            6 => Some(Intrinsic::Acos),
            7 => Some(Intrinsic::Atan),
            8 => Some(Intrinsic::Atan2),
            9 => Some(Intrinsic::Exp),
            10 => Some(Intrinsic::Log),
            11 => Some(Intrinsic::Log10),
            12 => Some(Intrinsic::Mod),
            13 => Some(Intrinsic::Modulo),
            14 => Some(Intrinsic::Max),
            15 => Some(Intrinsic::Min),
            16 => Some(Intrinsic::Floor),
            17 => Some(Intrinsic::Ceiling),
            18 => Some(Intrinsic::Nint),
            19 => Some(Intrinsic::Sign),
            20 => Some(Intrinsic::Int),
            21 => Some(Intrinsic::Real),
            22 => Some(Intrinsic::Dble),
            23 => Some(Intrinsic::Sum),
            24 => Some(Intrinsic::Product),
            25 => Some(Intrinsic::Size),
            26 => Some(Intrinsic::Maxval),
            27 => Some(Intrinsic::Minval),
            28 => Some(Intrinsic::DotProduct),
            _ => None,
        }
    }

    /// Get intrinsic by name (case-insensitive)
    pub fn from_name(name: &str) -> Option<Intrinsic> {
        match name.to_uppercase().as_str() {
            "SQRT" => Some(Intrinsic::Sqrt),
            "ABS" | "IABS" | "DABS" => Some(Intrinsic::Abs),
            "SIN" | "DSIN" => Some(Intrinsic::Sin),
            "COS" | "DCOS" => Some(Intrinsic::Cos),
            "TAN" | "DTAN" => Some(Intrinsic::Tan),
            "ASIN" | "DASIN" => Some(Intrinsic::Asin),
            "ACOS" | "DACOS" => Some(Intrinsic::Acos),
            "ATAN" | "DATAN" => Some(Intrinsic::Atan),
            "ATAN2" | "DATAN2" => Some(Intrinsic::Atan2),
            "EXP" | "DEXP" => Some(Intrinsic::Exp),
            "LOG" | "ALOG" | "DLOG" => Some(Intrinsic::Log),
            "LOG10" | "ALOG10" | "DLOG10" => Some(Intrinsic::Log10),
            "MOD" => Some(Intrinsic::Mod),
            "MODULO" => Some(Intrinsic::Modulo),
            "MAX" | "MAX0" | "AMAX1" | "DMAX1" => Some(Intrinsic::Max),
            "MIN" | "MIN0" | "AMIN1" | "DMIN1" => Some(Intrinsic::Min),
            "FLOOR" => Some(Intrinsic::Floor),
            "CEILING" => Some(Intrinsic::Ceiling),
            "NINT" | "ANINT" | "DNINT" => Some(Intrinsic::Nint),
            "SIGN" | "ISIGN" | "DSIGN" => Some(Intrinsic::Sign),
            "INT" | "IFIX" | "IDINT" => Some(Intrinsic::Int),
            "REAL" | "FLOAT" | "SNGL" => Some(Intrinsic::Real),
            "DBLE" | "DFLOAT" => Some(Intrinsic::Dble),
            // Array intrinsics
            "SUM" => Some(Intrinsic::Sum),
            "PRODUCT" => Some(Intrinsic::Product),
            "SIZE" => Some(Intrinsic::Size),
            "MAXVAL" => Some(Intrinsic::Maxval),
            "MINVAL" => Some(Intrinsic::Minval),
            "DOT_PRODUCT" => Some(Intrinsic::DotProduct),
            _ => None,
        }
    }

    /// Get the number of required arguments for this intrinsic
    pub fn arg_count(&self) -> (usize, usize) {
        // Returns (min_args, max_args)
        match self {
            // Single argument functions
            Intrinsic::Sqrt | Intrinsic::Abs | Intrinsic::Sin | Intrinsic::Cos |
            Intrinsic::Tan | Intrinsic::Asin | Intrinsic::Acos | Intrinsic::Atan |
            Intrinsic::Exp | Intrinsic::Log | Intrinsic::Log10 |
            Intrinsic::Floor | Intrinsic::Ceiling | Intrinsic::Nint |
            Intrinsic::Int | Intrinsic::Real | Intrinsic::Dble => (1, 1),

            // Two argument functions
            Intrinsic::Mod | Intrinsic::Modulo | Intrinsic::Atan2 | Intrinsic::Sign => (2, 2),

            // Variable argument functions (at least 2)
            Intrinsic::Max | Intrinsic::Min => (2, 255),

            // Array intrinsics (single array argument)
            Intrinsic::Sum | Intrinsic::Product | Intrinsic::Size |
            Intrinsic::Maxval | Intrinsic::Minval => (1, 1),

            // Two array argument function
            Intrinsic::DotProduct => (2, 2),
        }
    }
}

impl fmt::Display for Intrinsic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Intrinsic::Sqrt => write!(f, "SQRT"),
            Intrinsic::Abs => write!(f, "ABS"),
            Intrinsic::Sin => write!(f, "SIN"),
            Intrinsic::Cos => write!(f, "COS"),
            Intrinsic::Tan => write!(f, "TAN"),
            Intrinsic::Asin => write!(f, "ASIN"),
            Intrinsic::Acos => write!(f, "ACOS"),
            Intrinsic::Atan => write!(f, "ATAN"),
            Intrinsic::Atan2 => write!(f, "ATAN2"),
            Intrinsic::Exp => write!(f, "EXP"),
            Intrinsic::Log => write!(f, "LOG"),
            Intrinsic::Log10 => write!(f, "LOG10"),
            Intrinsic::Mod => write!(f, "MOD"),
            Intrinsic::Modulo => write!(f, "MODULO"),
            Intrinsic::Max => write!(f, "MAX"),
            Intrinsic::Min => write!(f, "MIN"),
            Intrinsic::Floor => write!(f, "FLOOR"),
            Intrinsic::Ceiling => write!(f, "CEILING"),
            Intrinsic::Nint => write!(f, "NINT"),
            Intrinsic::Sign => write!(f, "SIGN"),
            Intrinsic::Int => write!(f, "INT"),
            Intrinsic::Real => write!(f, "REAL"),
            Intrinsic::Dble => write!(f, "DBLE"),
            Intrinsic::Sum => write!(f, "SUM"),
            Intrinsic::Product => write!(f, "PRODUCT"),
            Intrinsic::Size => write!(f, "SIZE"),
            Intrinsic::Maxval => write!(f, "MAXVAL"),
            Intrinsic::Minval => write!(f, "MINVAL"),
            Intrinsic::DotProduct => write!(f, "DOT_PRODUCT"),
        }
    }
}

impl fmt::Display for OpCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OpCode::LoadConst => write!(f, "LoadConst"),
            OpCode::LoadTrue => write!(f, "LoadTrue"),
            OpCode::LoadFalse => write!(f, "LoadFalse"),
            OpCode::LoadVar => write!(f, "LoadVar"),
            OpCode::StoreVar => write!(f, "StoreVar"),
            OpCode::Add => write!(f, "Add"),
            OpCode::Subtract => write!(f, "Subtract"),
            OpCode::Multiply => write!(f, "Multiply"),
            OpCode::Divide => write!(f, "Divide"),
            OpCode::Power => write!(f, "Power"),
            OpCode::Negate => write!(f, "Negate"),
            OpCode::Equal => write!(f, "Equal"),
            OpCode::NotEqual => write!(f, "NotEqual"),
            OpCode::Less => write!(f, "Less"),
            OpCode::LessEqual => write!(f, "LessEqual"),
            OpCode::Greater => write!(f, "Greater"),
            OpCode::GreaterEqual => write!(f, "GreaterEqual"),
            OpCode::And => write!(f, "And"),
            OpCode::Or => write!(f, "Or"),
            OpCode::Not => write!(f, "Not"),
            OpCode::IntToReal => write!(f, "IntToReal"),
            OpCode::RealToInt => write!(f, "RealToInt"),
            OpCode::Jump => write!(f, "Jump"),
            OpCode::JumpIfFalse => write!(f, "JumpIfFalse"),
            OpCode::JumpIfTrue => write!(f, "JumpIfTrue"),
            OpCode::Pop => write!(f, "Pop"),
            OpCode::Dup => write!(f, "Dup"),
            OpCode::Print => write!(f, "Print"),
            OpCode::Read => write!(f, "Read"),
            OpCode::OpenFile => write!(f, "OpenFile"),
            OpCode::CloseFile => write!(f, "CloseFile"),
            OpCode::WriteFile => write!(f, "WriteFile"),
            OpCode::ReadFile => write!(f, "ReadFile"),
            OpCode::Halt => write!(f, "Halt"),
            OpCode::Nop => write!(f, "Nop"),
            OpCode::Call => write!(f, "Call"),
            OpCode::Return => write!(f, "Return"),
            OpCode::AllocArray => write!(f, "AllocArray"),
            OpCode::LoadArrayElem => write!(f, "LoadArrayElem"),
            OpCode::StoreArrayElem => write!(f, "StoreArrayElem"),
            OpCode::CallIntrinsic => write!(f, "CallIntrinsic"),
            OpCode::CreateInstance => write!(f, "CreateInstance"),
            OpCode::LoadComponent => write!(f, "LoadComponent"),
            OpCode::StoreComponent => write!(f, "StoreComponent"),
        }
    }
}

/// Array dimension information for runtime bounds checking
#[derive(Debug, Clone, PartialEq)]
pub struct ArrayDim {
    pub lower: i64,
    pub upper: i64,
}

impl ArrayDim {
    pub fn new(lower: i64, upper: i64) -> Self {
        Self { lower, upper }
    }

    pub fn size(&self) -> usize {
        (self.upper - self.lower + 1).max(0) as usize
    }
}

/// Runtime derived type definition
#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeTypeDef {
    /// Type name
    pub name: String,
    /// Component names in order
    pub components: Vec<String>,
    /// Default values for each component (None if no default)
    pub defaults: Vec<Option<Value>>,
}

impl RuntimeTypeDef {
    pub fn new(name: String) -> Self {
        Self {
            name,
            components: Vec::new(),
            defaults: Vec::new(),
        }
    }

    pub fn add_component(&mut self, name: String, default: Option<Value>) {
        self.components.push(name);
        self.defaults.push(default);
    }

    pub fn component_index(&self, name: &str) -> Option<usize> {
        self.components.iter().position(|c| c == name)
    }
}

/// Runtime value types
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Integer(i64),
    Real(f64),
    Logical(bool),
    Character(String),
    /// Array with elements and dimension info
    Array {
        elements: Vec<Value>,
        dims: Vec<ArrayDim>,
    },
    /// Instance of a derived type
    Instance {
        /// Index into the type registry
        type_index: usize,
        /// Component values in order matching type definition
        components: Vec<Value>,
    },
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Integer(_) => "INTEGER",
            Value::Real(_) => "REAL",
            Value::Logical(_) => "LOGICAL",
            Value::Character(_) => "CHARACTER",
            Value::Array { .. } => "ARRAY",
            Value::Instance { .. } => "DERIVED TYPE",
        }
    }

    /// Create a new array with given dimensions, initialized to default values
    pub fn new_array(dims: Vec<ArrayDim>, default: Value) -> Self {
        let total_size: usize = dims.iter().map(|d| d.size()).product();
        let elements = vec![default; total_size];
        Value::Array { elements, dims }
    }

    /// Create a new integer array with given dimensions, initialized to 0
    pub fn new_integer_array(dims: Vec<ArrayDim>) -> Self {
        Self::new_array(dims, Value::Integer(0))
    }

    /// Create a new real array with given dimensions, initialized to 0.0
    pub fn new_real_array(dims: Vec<ArrayDim>) -> Self {
        Self::new_array(dims, Value::Real(0.0))
    }

    /// Check if this value is an array
    pub fn is_array(&self) -> bool {
        matches!(self, Value::Array { .. })
    }

    /// Get array rank (number of dimensions)
    pub fn rank(&self) -> Option<usize> {
        match self {
            Value::Array { dims, .. } => Some(dims.len()),
            _ => None,
        }
    }

    /// Calculate linear index from multi-dimensional indices (column-major order like Fortran)
    pub fn linear_index(&self, indices: &[i64]) -> Option<usize> {
        match self {
            Value::Array { dims, .. } => {
                if indices.len() != dims.len() {
                    return None;
                }

                let mut linear_idx = 0usize;
                let mut stride = 1usize;

                for (idx, dim) in indices.iter().zip(dims.iter()) {
                    // Check bounds
                    if *idx < dim.lower || *idx > dim.upper {
                        return None;
                    }
                    linear_idx += ((idx - dim.lower) as usize) * stride;
                    stride *= dim.size();
                }

                Some(linear_idx)
            }
            _ => None,
        }
    }

    /// Get an element from an array by indices
    pub fn get_element(&self, indices: &[i64]) -> Option<&Value> {
        match self {
            Value::Array { elements, .. } => {
                let linear_idx = self.linear_index(indices)?;
                elements.get(linear_idx)
            }
            _ => None,
        }
    }

    /// Set an element in an array by indices (returns None if out of bounds or not an array)
    pub fn set_element(&mut self, indices: &[i64], value: Value) -> Option<()> {
        match self {
            Value::Array { elements, dims } => {
                // Calculate linear index inline to avoid borrow issues
                if indices.len() != dims.len() {
                    return None;
                }

                let mut linear_idx = 0usize;
                let mut stride = 1usize;

                for (idx, dim) in indices.iter().zip(dims.iter()) {
                    if *idx < dim.lower || *idx > dim.upper {
                        return None;
                    }
                    linear_idx += ((idx - dim.lower) as usize) * stride;
                    stride *= dim.size();
                }

                if linear_idx < elements.len() {
                    elements[linear_idx] = value;
                    Some(())
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Integer(v) => write!(f, "{}", v),
            Value::Real(v) => write!(f, "{}", v),
            Value::Logical(true) => write!(f, ".TRUE."),
            Value::Logical(false) => write!(f, ".FALSE."),
            Value::Character(s) => write!(f, "'{}'", s),
            Value::Array { elements, dims } => {
                // Format like Fortran array output
                write!(f, "[")?;
                for (i, elem) in elements.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", elem)?;
                }
                write!(f, "]")?;
                // Show shape
                write!(f, " shape(")?;
                for (i, dim) in dims.iter().enumerate() {
                    if i > 0 {
                        write!(f, ",")?;
                    }
                    write!(f, "{}", dim.size())?;
                }
                write!(f, ")")
            }
            Value::Instance { type_index, components } => {
                write!(f, "Instance(type={}, ", type_index)?;
                for (i, comp) in components.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", comp)?;
                }
                write!(f, ")")
            }
        }
    }
}

/// A single bytecode instruction
#[derive(Debug, Clone, PartialEq)]
pub struct Instruction {
    pub opcode: OpCode,
    pub operand: Option<usize>,
    pub location: SourceLocation,
}

impl Instruction {
    pub fn new(opcode: OpCode, location: SourceLocation) -> Self {
        Self {
            opcode,
            operand: None,
            location,
        }
    }

    pub fn with_operand(opcode: OpCode, operand: usize, location: SourceLocation) -> Self {
        Self {
            opcode,
            operand: Some(operand),
            location,
        }
    }
}

/// Constant pool for storing literal values
#[derive(Debug, Clone, Default)]
pub struct ConstantPool {
    constants: Vec<Value>,
    index_map: HashMap<String, usize>, // For deduplication
}

impl ConstantPool {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a constant to the pool, returns its index
    /// Deduplicates identical constants
    pub fn add(&mut self, value: Value) -> usize {
        let key = format!("{:?}", value);

        if let Some(&index) = self.index_map.get(&key) {
            return index;
        }

        let index = self.constants.len();
        self.constants.push(value);
        self.index_map.insert(key, index);
        index
    }

    /// Get a constant by index
    pub fn get(&self, index: usize) -> Option<&Value> {
        self.constants.get(index)
    }

    /// Get all constants
    pub fn constants(&self) -> &[Value] {
        &self.constants
    }

    /// Number of constants
    pub fn len(&self) -> usize {
        self.constants.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.constants.is_empty()
    }
}

/// A chunk of bytecode with its constant pool
#[derive(Debug, Clone, Default)]
pub struct Chunk {
    pub instructions: Vec<Instruction>,
    pub constants: ConstantPool,
    pub variables: Vec<String>, // Variable names by index
    /// Procedure entry points (name -> instruction address)
    pub procedures: HashMap<String, usize>,
    /// Derived type definitions
    pub types: Vec<RuntimeTypeDef>,
    /// Type name to index mapping
    pub type_indices: HashMap<String, usize>,
    /// Operator overloading interfaces (operator -> list of procedure names)
    pub operator_interfaces: HashMap<String, Vec<String>>,
    /// Assignment interfaces (list of procedure names)
    pub assignment_interfaces: Vec<String>,
    /// Generic interfaces (generic name -> list of procedure names)
    pub generic_interfaces: HashMap<String, Vec<String>>,
}

impl Chunk {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an instruction without operand
    pub fn emit(&mut self, opcode: OpCode, location: SourceLocation) -> usize {
        let index = self.instructions.len();
        self.instructions.push(Instruction::new(opcode, location));
        index
    }

    /// Add an instruction with operand
    pub fn emit_with_operand(
        &mut self,
        opcode: OpCode,
        operand: usize,
        location: SourceLocation,
    ) -> usize {
        let index = self.instructions.len();
        self.instructions
            .push(Instruction::with_operand(opcode, operand, location));
        index
    }

    /// Add a constant and return its index
    pub fn add_constant(&mut self, value: Value) -> usize {
        self.constants.add(value)
    }

    /// Add a variable and return its index
    pub fn add_variable(&mut self, name: String) -> usize {
        // Check if variable already exists
        if let Some(pos) = self.variables.iter().position(|v| v == &name) {
            return pos;
        }
        let index = self.variables.len();
        self.variables.push(name);
        index
    }

    /// Get variable index by name
    pub fn get_variable_index(&self, name: &str) -> Option<usize> {
        self.variables.iter().position(|v| v == name)
    }

    /// Check if a variable has been declared
    pub fn has_variable(&self, name: &str) -> bool {
        self.variables.iter().any(|v| v == name)
    }

    /// Register a procedure entry point
    pub fn add_procedure(&mut self, name: String, address: usize) {
        self.procedures.insert(name, address);
    }

    /// Get procedure entry address by name
    pub fn get_procedure_address(&self, name: &str) -> Option<usize> {
        self.procedures.get(name).copied()
    }

    /// Register a derived type definition
    pub fn add_type(&mut self, type_def: RuntimeTypeDef) -> usize {
        let name = type_def.name.clone();
        let index = self.types.len();
        self.types.push(type_def);
        self.type_indices.insert(name, index);
        index
    }

    /// Get type definition by name
    pub fn get_type_index(&self, name: &str) -> Option<usize> {
        self.type_indices.get(name).copied()
    }

    /// Get type definition by index
    pub fn get_type(&self, index: usize) -> Option<&RuntimeTypeDef> {
        self.types.get(index)
    }

    /// Register an operator interface (for operator overloading)
    pub fn register_operator_interface(&mut self, op: OverloadableOperator, proc_name: String) {
        let op_key = format!("{:?}", op);
        self.operator_interfaces
            .entry(op_key)
            .or_insert_with(Vec::new)
            .push(proc_name);
    }

    /// Register an assignment interface
    pub fn register_assignment_interface(&mut self, proc_name: String) {
        self.assignment_interfaces.push(proc_name);
    }

    /// Register a generic interface
    pub fn register_generic_interface(&mut self, generic_name: String, proc_name: String) {
        self.generic_interfaces
            .entry(generic_name)
            .or_insert_with(Vec::new)
            .push(proc_name);
    }

    /// Get operator interface procedures for a given operator
    pub fn get_operator_procedures(&self, op: &OverloadableOperator) -> Option<&Vec<String>> {
        let op_key = format!("{:?}", op);
        self.operator_interfaces.get(&op_key)
    }

    /// Patch a jump instruction with the actual target
    pub fn patch_jump(&mut self, instruction_index: usize, target: usize) {
        if let Some(instr) = self.instructions.get_mut(instruction_index) {
            instr.operand = Some(target);
        }
    }

    /// Current instruction count (for jump targets)
    pub fn current_offset(&self) -> usize {
        self.instructions.len()
    }

    /// Disassemble the chunk into a readable string
    pub fn disassemble(&self, name: &str) -> String {
        let mut output = format!("== {} ==\n", name);

        // Show constants
        if !self.constants.is_empty() {
            output.push_str("Constants:\n");
            for (i, constant) in self.constants.constants().iter().enumerate() {
                output.push_str(&format!("  {:04} = {}\n", i, constant));
            }
            output.push('\n');
        }

        // Show variables
        if !self.variables.is_empty() {
            output.push_str("Variables:\n");
            for (i, var) in self.variables.iter().enumerate() {
                output.push_str(&format!("  {:04} = {}\n", i, var));
            }
            output.push('\n');
        }

        // Show instructions
        output.push_str("Instructions:\n");
        for (i, instr) in self.instructions.iter().enumerate() {
            output.push_str(&self.disassemble_instruction(i, instr));
        }

        output
    }

    fn disassemble_instruction(&self, offset: usize, instr: &Instruction) -> String {
        let operand_str = match instr.operand {
            Some(op) => {
                match instr.opcode {
                    OpCode::LoadConst => {
                        if let Some(val) = self.constants.get(op) {
                            format!("{:4} ; {}", op, val)
                        } else {
                            format!("{:4}", op)
                        }
                    }
                    OpCode::LoadVar | OpCode::StoreVar | OpCode::Read => {
                        if let Some(name) = self.variables.get(op) {
                            format!("{:4} ; {}", op, name)
                        } else {
                            format!("{:4}", op)
                        }
                    }
                    OpCode::Print => format!("{:4} ; count", op),
                    _ => format!("{:4}", op),
                }
            }
            None => String::new(),
        };

        format!(
            "  {:04} {:12} {}\n",
            offset, instr.opcode, operand_str
        )
    }
}

/// Compiler error types
#[derive(Debug, Clone, PartialEq)]
pub enum CompileError {
    UndeclaredVariable {
        name: String,
        location: SourceLocation,
    },
    InternalError {
        message: String,
        location: SourceLocation,
    },
    InvalidOperation {
        message: String,
        location: SourceLocation,
    },
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompileError::UndeclaredVariable { name, location } => {
                write!(f, "Undeclared variable '{}' at {}", name, location)
            }
            CompileError::InternalError { message, location } => {
                write!(f, "Internal compiler error at {}: {}", location, message)
            }
            CompileError::InvalidOperation { message, location } => {
                write!(f, "Invalid operation at {}: {}", location, message)
            }
        }
    }
}

impl std::error::Error for CompileError {}

pub type CompileResult<T> = Result<T, CompileError>;

/// Information about an active loop during compilation
#[derive(Debug, Clone)]
struct LoopContext {
    /// Instruction offset where the loop condition/start is
    #[allow(dead_code)]
    loop_start: usize,
    /// List of EXIT jump instruction offsets that need to be patched to loop end
    exit_jumps: Vec<usize>,
    /// List of CYCLE jump instruction offsets that need to be patched to continue point
    cycle_jumps: Vec<usize>,
    /// Optional loop name for named EXIT/CYCLE (reserved for future use)
    #[allow(dead_code)]
    name: Option<String>,
}

/// Type of symbol exported by a module
#[derive(Debug, Clone, PartialEq)]
pub enum ModuleSymbolKind {
    /// Variable with its chunk variable index
    Variable(usize),
    /// Parameter (constant) with its chunk variable index
    Parameter(usize),
    /// Procedure with its entry address
    Procedure(usize),
}

/// A symbol exported by a module
#[derive(Debug, Clone)]
pub struct ModuleSymbol {
    pub name: String,
    pub kind: ModuleSymbolKind,
    pub visibility: Visibility,
}

/// Registry of compiled modules and their exports
#[derive(Debug, Clone, Default)]
pub struct ModuleRegistry {
    /// Map of module name -> list of exported symbols
    modules: HashMap<String, Vec<ModuleSymbol>>,
    /// Map of module name -> operator interfaces (operator key -> procedure names)
    module_interfaces: HashMap<String, HashMap<String, Vec<String>>>,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a module with its exported symbols
    pub fn register_module(&mut self, name: String, symbols: Vec<ModuleSymbol>) {
        self.modules.insert(name, symbols);
    }

    /// Get symbols from a module
    pub fn get_module(&self, name: &str) -> Option<&Vec<ModuleSymbol>> {
        self.modules.get(name)
    }

    /// Get a specific symbol from a module by name
    pub fn get_symbol(&self, module_name: &str, symbol_name: &str) -> Option<&ModuleSymbol> {
        self.modules.get(module_name)?
            .iter()
            .find(|s| s.name == symbol_name)
    }

    /// Register operator interfaces for a module
    pub fn register_interfaces(&mut self, module_name: String, interfaces: HashMap<String, Vec<String>>) {
        self.module_interfaces.insert(module_name, interfaces);
    }

    /// Get operator interfaces from a module
    pub fn get_module_interfaces(&self, module_name: &str) -> Option<&HashMap<String, Vec<String>>> {
        self.module_interfaces.get(module_name)
    }
}

/// Bytecode compiler that transforms AST to bytecode
pub struct Compiler {
    chunk: Chunk,
    /// Stack of active loops for EXIT/CYCLE handling
    loop_stack: Vec<LoopContext>,
    /// Registry of compiled modules
    module_registry: ModuleRegistry,
    /// Current function parameter names (to avoid reinitializing them)
    current_function_params: HashSet<String>,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            chunk: Chunk::new(),
            loop_stack: Vec::new(),
            module_registry: ModuleRegistry::new(),
            current_function_params: HashSet::new(),
        }
    }

    /// Compile a complete compilation unit (modules + program)
    pub fn compile_unit(&mut self, unit: &CompilationUnit) -> CompileResult<Chunk> {
        let default_loc = SourceLocation { line: 1, column: 1 };

        // First pass: compile module declarations (these need to run to initialize constants)
        for module in &unit.modules {
            self.compile_module_declarations(module)?;
        }

        // If we have module procedures and a program, emit a jump to skip procedure code
        let has_module_procs = unit.modules.iter().any(|m| !m.procedures.is_empty());
        let jump_to_program = if has_module_procs && unit.program.is_some() {
            Some(self.chunk.emit_with_operand(OpCode::Jump, 0, default_loc))
        } else {
            None
        };

        // Second pass: compile module procedures
        for module in &unit.modules {
            self.compile_module_procedures(module)?;
        }

        // Patch jump to program entry point
        if let Some(jump_idx) = jump_to_program {
            self.chunk.patch_jump(jump_idx, self.chunk.current_offset());
        }

        // Compile the main program if present
        if let Some(program) = &unit.program {
            // Process USE statements first
            for use_stmt in &program.uses {
                self.process_use_statement(use_stmt, program.location)?;
            }
            self.compile(program)
        } else {
            // No program - just return the chunk with module code
            self.chunk.emit(OpCode::Halt, default_loc);
            Ok(std::mem::take(&mut self.chunk))
        }
    }

    /// Compile module declarations (first pass - runs at program start)
    fn compile_module_declarations(&mut self, module: &ModuleDef) -> CompileResult<()> {
        let mut exported_symbols = Vec::new();

        // Build visibility map from visibility statements
        let mut explicit_visibility: HashMap<String, Visibility> = HashMap::new();
        for vis_stmt in &module.visibility_stmts {
            for name in &vis_stmt.names {
                explicit_visibility.insert(name.clone(), vis_stmt.visibility);
            }
        }

        // Compile module-level declarations (constants and variables)
        for decl in &module.declarations {
            match decl {
                Declaration::Variable { entities, .. } => {
                    for entity in entities {
                        let var_index = self.chunk.add_variable(entity.name.clone());
                        let visibility = explicit_visibility.get(&entity.name)
                            .copied()
                            .unwrap_or(module.default_visibility);

                        exported_symbols.push(ModuleSymbol {
                            name: entity.name.clone(),
                            kind: ModuleSymbolKind::Variable(var_index),
                            visibility,
                        });
                    }
                    self.compile_declaration(decl)?;
                }
                Declaration::Parameter { name, .. } => {
                    let var_index = self.chunk.add_variable(name.clone());
                    let visibility = explicit_visibility.get(name)
                        .copied()
                        .unwrap_or(module.default_visibility);

                    exported_symbols.push(ModuleSymbol {
                        name: name.clone(),
                        kind: ModuleSymbolKind::Parameter(var_index),
                        visibility,
                    });
                    self.compile_declaration(decl)?;
                }
                Declaration::ImplicitNone { .. } => {
                    // No symbol export needed
                }
                Declaration::DerivedType(type_def) => {
                    // Register derived type definition
                    let mut runtime_type = RuntimeTypeDef::new(type_def.name.clone());
                    for component in &type_def.components {
                        runtime_type.add_component(component.name.clone(), None);
                    }
                    self.chunk.add_type(runtime_type);
                }
                Declaration::Interface(interface) => {
                    // Register interface for operator overloading
                    self.compile_interface(interface)?;
                }
            }
        }

        // Register module symbols (procedures will be added in second pass)
        self.module_registry.register_module(module.name.clone(), exported_symbols);

        // Register module interfaces for operator overloading
        if !self.chunk.operator_interfaces.is_empty() {
            self.module_registry.register_interfaces(
                module.name.clone(),
                self.chunk.operator_interfaces.clone()
            );
        }

        Ok(())
    }

    /// Compile module procedures (second pass - code that is jumped over)
    fn compile_module_procedures(&mut self, module: &ModuleDef) -> CompileResult<()> {
        // Build visibility map
        let mut explicit_visibility: HashMap<String, Visibility> = HashMap::new();
        for vis_stmt in &module.visibility_stmts {
            for name in &vis_stmt.names {
                explicit_visibility.insert(name.clone(), vis_stmt.visibility);
            }
        }

        // Compile module procedures and update the registry with entry addresses
        for proc in &module.procedures {
            let proc_name = match proc {
                Procedure::Subroutine(sub) => sub.name.clone(),
                Procedure::Function(func) => func.name.clone(),
            };

            let entry_address = self.chunk.current_offset();
            self.compile_procedure(proc)?;

            let visibility = explicit_visibility.get(&proc_name)
                .copied()
                .unwrap_or(module.default_visibility);

            // Add procedure symbol to module registry
            if let Some(symbols) = self.module_registry.modules.get_mut(&module.name) {
                symbols.push(ModuleSymbol {
                    name: proc_name.clone(),
                    kind: ModuleSymbolKind::Procedure(entry_address),
                    visibility,
                });
            }

            // Also register in chunk procedures for direct lookup
            self.chunk.add_procedure(proc_name, entry_address);
        }

        Ok(())
    }

    /// Compile a module definition (legacy single-pass - kept for compatibility)
    #[allow(dead_code)]
    fn compile_module(&mut self, module: &ModuleDef) -> CompileResult<()> {
        self.compile_module_declarations(module)?;
        self.compile_module_procedures(module)?;
        Ok(())
    }

    /// Process a USE statement to import symbols from a module
    fn process_use_statement(&mut self, use_stmt: &UseStatement, location: SourceLocation) -> CompileResult<()> {
        let module_symbols = self.module_registry.get_module(&use_stmt.module_name)
            .ok_or_else(|| CompileError::InvalidOperation {
                message: format!("Module '{}' not found", use_stmt.module_name),
                location,
            })?
            .clone(); // Clone to avoid borrow issues

        match &use_stmt.only {
            None => {
                // Import all public symbols
                for symbol in &module_symbols {
                    if symbol.visibility == Visibility::Public {
                        self.import_symbol(symbol, &symbol.name)?;
                    }
                }
            }
            Some(items) => {
                // Import only specified symbols
                for item in items {
                    let original_name = item.original_name.as_ref().unwrap_or(&item.local_name);
                    let symbol = module_symbols.iter()
                        .find(|s| s.name == *original_name)
                        .ok_or_else(|| CompileError::InvalidOperation {
                            message: format!("Symbol '{}' not found in module '{}'", original_name, use_stmt.module_name),
                            location,
                        })?;

                    self.import_symbol(symbol, &item.local_name)?;
                }
            }
        }

        // Import operator interfaces from the module
        if let Some(interfaces) = self.module_registry.get_module_interfaces(&use_stmt.module_name).cloned() {
            for (op_key, proc_names) in interfaces {
                for proc_name in proc_names {
                    self.chunk.operator_interfaces
                        .entry(op_key.clone())
                        .or_insert_with(Vec::new)
                        .push(proc_name);
                }
            }
        }

        Ok(())
    }

    /// Import a symbol from a module (make it available under the given local name)
    fn import_symbol(&mut self, symbol: &ModuleSymbol, local_name: &str) -> CompileResult<()> {
        match &symbol.kind {
            ModuleSymbolKind::Variable(var_idx) | ModuleSymbolKind::Parameter(var_idx) => {
                // Map local name to the same variable index
                // Since variables are shared, we just ensure the name maps to the same slot
                if !self.chunk.has_variable(local_name) {
                    // Add an alias: the local name points to the module variable
                    self.chunk.variables.push(local_name.to_string());
                }
                // Note: This is a simplified approach. In a full implementation,
                // we'd need a symbol table mapping local names to their actual indices.
                let _ = var_idx; // Acknowledge the index (it's already in chunk.variables)
            }
            ModuleSymbolKind::Procedure(addr) => {
                // Register the procedure under the local name
                self.chunk.add_procedure(local_name.to_string(), *addr);
            }
        }
        Ok(())
    }

    /// Compile a complete program
    pub fn compile(&mut self, program: &Program) -> CompileResult<Chunk> {
        // Process declarations first (allocate variable slots)
        for decl in &program.declarations {
            self.compile_declaration(decl)?;
        }

        // Jump over procedures to main code if there are procedures
        let loc = program.location;
        let jump_to_main = if !program.procedures.is_empty() {
            Some(self.chunk.emit_with_operand(OpCode::Jump, 0, loc))
        } else {
            None
        };

        // Compile procedures first (so their addresses are known)
        for proc in &program.procedures {
            self.compile_procedure(proc)?;
        }

        // Patch jump to main
        if let Some(jump_idx) = jump_to_main {
            self.chunk.patch_jump(jump_idx, self.chunk.current_offset());
        }

        // Compile main statements
        for stmt in &program.statements {
            self.compile_statement(stmt)?;
        }

        // Emit halt at end of main program
        self.chunk.emit(OpCode::Halt, loc);

        Ok(std::mem::take(&mut self.chunk))
    }

    /// Compile a procedure definition
    fn compile_procedure(&mut self, proc: &Procedure) -> CompileResult<()> {
        match proc {
            Procedure::Subroutine(sub) => self.compile_subroutine(sub),
            Procedure::Function(func) => self.compile_function(func),
        }
    }

    /// Compile a subroutine definition
    fn compile_subroutine(&mut self, sub: &SubroutineDef) -> CompileResult<()> {
        let entry_address = self.chunk.current_offset();

        // Register procedure entry point
        self.chunk.add_procedure(sub.name.clone(), entry_address);

        // Track subroutine parameters to avoid reinitializing them
        self.current_function_params.clear();
        for param in &sub.parameters {
            self.current_function_params.insert(param.name.clone());
        }

        // Allocate parameter slots and pop arguments into them (in reverse order)
        let param_indices: Vec<_> = sub.parameters.iter()
            .map(|param| self.chunk.add_variable(param.name.clone()))
            .collect();

        // Pop arguments from stack into parameters (reverse order because stack is LIFO)
        for &param_idx in param_indices.iter().rev() {
            self.chunk.emit_with_operand(OpCode::StoreVar, param_idx, sub.location);
        }

        // Compile local declarations
        for decl in &sub.declarations {
            self.compile_declaration(decl)?;
        }

        // Compile body statements
        for stmt in &sub.body {
            self.compile_statement(stmt)?;
        }

        // Emit return at end
        self.chunk.emit(OpCode::Return, sub.location);

        // Clear parameter tracking
        self.current_function_params.clear();

        Ok(())
    }

    /// Compile a function definition
    fn compile_function(&mut self, func: &FunctionDef) -> CompileResult<()> {
        let entry_address = self.chunk.current_offset();

        // Register procedure entry point
        self.chunk.add_procedure(func.name.clone(), entry_address);

        // Track function parameters to avoid reinitializing them
        self.current_function_params.clear();
        for param in &func.parameters {
            self.current_function_params.insert(param.name.clone());
        }

        // Allocate parameter slots and pop arguments into them (in reverse order)
        let param_indices: Vec<_> = func.parameters.iter()
            .map(|param| self.chunk.add_variable(param.name.clone()))
            .collect();

        // Pop arguments from stack into parameters (reverse order because stack is LIFO)
        for &param_idx in param_indices.iter().rev() {
            self.chunk.emit_with_operand(OpCode::StoreVar, param_idx, func.location);
        }

        // Allocate result variable (function name or RESULT variable)
        let result_name = func.result_name.as_ref().unwrap_or(&func.name);
        self.chunk.add_variable(result_name.clone());

        // Compile local declarations
        for decl in &func.declarations {
            self.compile_declaration(decl)?;
        }

        // Compile body statements
        for stmt in &func.body {
            self.compile_statement(stmt)?;
        }

        // Load result value onto stack before return
        let result_idx = self.chunk.get_variable_index(result_name)
            .ok_or(CompileError::InvalidOperation {
                message: format!("Result variable {} not found", result_name),
                location: func.location,
            })?;
        self.chunk.emit_with_operand(OpCode::LoadVar, result_idx, func.location);

        // Emit return at end
        self.chunk.emit(OpCode::Return, func.location);

        // Clear parameter tracking
        self.current_function_params.clear();

        Ok(())
    }

    /// Compile a declaration
    fn compile_declaration(&mut self, decl: &Declaration) -> CompileResult<()> {
        match decl {
            Declaration::Variable { type_spec, entities, location, .. } => {
                for entity in entities {
                    // Check if this is a function parameter (should not be reinitialized)
                    let is_param = self.current_function_params.contains(&entity.name);

                    // Allocate variable slot (returns existing index if already declared)
                    let var_index = self.chunk.add_variable(entity.name.clone());

                    // Check if this is an array declaration
                    if let Some(array_spec) = &entity.array_spec {
                        // Emit array allocation
                        // Push lower and upper bounds for each dimension FIRST
                        for dim in &array_spec.dimensions {
                            // Lower bound (default to 1 if not specified)
                            if let Some(lower) = &dim.lower {
                                self.compile_expression(lower)?;
                            } else {
                                let one_idx = self.chunk.add_constant(Value::Integer(1));
                                self.chunk.emit_with_operand(OpCode::LoadConst, one_idx, *location);
                            }

                            // Upper bound
                            self.compile_expression(&dim.upper)?;
                        }

                        // Push number of dimensions LAST (so it's on top of stack)
                        let num_dims = array_spec.dimensions.len();
                        let dims_idx = self.chunk.add_constant(Value::Integer(num_dims as i64));
                        self.chunk.emit_with_operand(OpCode::LoadConst, dims_idx, *location);

                        // Emit AllocArray with variable index
                        self.chunk.emit_with_operand(OpCode::AllocArray, var_index, *location);
                    } else if let Some(init_expr) = &entity.init {
                        // Scalar with initialization
                        self.compile_expression(init_expr)?;
                        self.chunk
                            .emit_with_operand(OpCode::StoreVar, var_index, *location);
                    } else if let TypeSpec::Derived { name, .. } = type_spec {
                        // Derived type without initialization - create a default instance
                        // ONLY if this is not a function parameter (params get values from caller)
                        if !is_param {
                            if let Some(type_index) = self.chunk.get_type_index(name) {
                                // Get the number of components
                                let num_components = self.chunk.types[type_index].components.len();

                                // Push default values (0) for each component
                                for _ in 0..num_components {
                                    let zero_idx = self.chunk.add_constant(Value::Integer(0));
                                    self.chunk.emit_with_operand(OpCode::LoadConst, zero_idx, *location);
                                }

                                // Push argument count
                                let count_idx = self.chunk.add_constant(Value::Integer(num_components as i64));
                                self.chunk.emit_with_operand(OpCode::LoadConst, count_idx, *location);

                                // Create the instance
                                self.chunk.emit_with_operand(OpCode::CreateInstance, type_index, *location);

                                // Store to variable
                                self.chunk.emit_with_operand(OpCode::StoreVar, var_index, *location);
                            }
                        }
                    }
                    // Primitive scalars without initialization don't need bytecode
                }
                Ok(())
            }
            Declaration::ImplicitNone { .. } => {
                // No bytecode needed for IMPLICIT NONE
                Ok(())
            }
            Declaration::Parameter { name, value, location, .. } => {
                // Treat parameters like initialized constants
                let var_index = self.chunk.add_variable(name.clone());
                self.compile_expression(value)?;
                self.chunk
                    .emit_with_operand(OpCode::StoreVar, var_index, *location);
                Ok(())
            }
            Declaration::DerivedType(type_def) => {
                // Register the derived type definition
                let mut runtime_type = RuntimeTypeDef::new(type_def.name.clone());

                for component in &type_def.components {
                    // Add component with its name (defaults are not supported yet)
                    runtime_type.add_component(component.name.clone(), None);
                }

                self.chunk.add_type(runtime_type);
                Ok(())
            }
            Declaration::Interface(interface) => {
                // Register interface for operator overloading
                self.compile_interface(interface)
            }
        }
    }

    /// Compile an interface block for operator overloading
    fn compile_interface(&mut self, interface: &InterfaceBlock) -> CompileResult<()> {
        // Register the operator/assignment interface for later use during expression compilation
        match &interface.kind {
            InterfaceKind::Operator(op) => {
                // Store operator -> procedure mapping
                for proc in &interface.procedures {
                    if let Some(proc_name) = &proc.module_procedure {
                        self.chunk.register_operator_interface(op.clone(), proc_name.clone());
                    }
                }
            }
            InterfaceKind::Assignment => {
                // Store assignment interface
                for proc in &interface.procedures {
                    if let Some(proc_name) = &proc.module_procedure {
                        self.chunk.register_assignment_interface(proc_name.clone());
                    }
                }
            }
            InterfaceKind::Generic(name) => {
                // Store generic interface
                for proc in &interface.procedures {
                    if let Some(proc_name) = &proc.module_procedure {
                        self.chunk.register_generic_interface(name.clone(), proc_name.clone());
                    }
                }
            }
            InterfaceKind::Abstract => {
                // Abstract interfaces don't need runtime registration
            }
        }
        Ok(())
    }

    /// Compile a statement
    fn compile_statement(&mut self, stmt: &Statement) -> CompileResult<()> {
        match stmt {
            Statement::Assignment { target, indices, components, value, location } => {
                if !components.is_empty() {
                    // Component assignment: obj%comp = value or obj%comp1%comp2 = value
                    // Compile the value first
                    self.compile_expression(value)?;

                    // Load the base object
                    let var_index = self.chunk.add_variable(target.clone());
                    self.chunk.emit_with_operand(OpCode::LoadVar, var_index, *location);

                    // For chained access like obj%a%b, we need to navigate to the parent
                    // and store in the final component
                    for (i, comp) in components.iter().enumerate() {
                        if i < components.len() - 1 {
                            // Navigate to intermediate component
                            let comp_idx = self.chunk.add_constant(Value::Character(comp.clone()));
                            self.chunk.emit_with_operand(OpCode::LoadComponent, comp_idx, *location);
                        } else {
                            // Store into final component
                            let comp_idx = self.chunk.add_constant(Value::Character(comp.clone()));
                            self.chunk.emit_with_operand(OpCode::StoreComponent, comp_idx, *location);
                        }
                    }

                    // Store the modified instance back
                    self.chunk.emit_with_operand(OpCode::StoreVar, var_index, *location);
                } else if let Some(idx_exprs) = indices {
                    // Array element assignment: arr(i, j, ...) = value
                    // Stack order for StoreArrayElem: value, index1, index2, ..., num_indices (top)

                    // Push value first (goes to bottom)
                    self.compile_expression(value)?;

                    // Push each index
                    for idx in idx_exprs {
                        self.compile_expression(idx)?;
                    }

                    // Push number of indices LAST (so it's on top of stack)
                    let num_idx = idx_exprs.len();
                    let idx_count = self.chunk.add_constant(Value::Integer(num_idx as i64));
                    self.chunk.emit_with_operand(OpCode::LoadConst, idx_count, *location);

                    // Get variable index
                    let var_index = self.chunk.add_variable(target.clone());

                    // Emit StoreArrayElem
                    self.chunk.emit_with_operand(OpCode::StoreArrayElem, var_index, *location);
                } else {
                    // Scalar assignment
                    // Compile the value expression
                    self.compile_expression(value)?;

                    // Get or create variable index
                    let var_index = self.chunk.add_variable(target.clone());
                    self.chunk
                        .emit_with_operand(OpCode::StoreVar, var_index, *location);
                }
                Ok(())
            }

            Statement::Print { values, location, .. } => {
                // Compile each value expression
                for value in values {
                    self.compile_expression(value)?;
                }
                // Emit print with count of values
                self.chunk
                    .emit_with_operand(OpCode::Print, values.len(), *location);
                Ok(())
            }

            Statement::If {
                condition,
                then_block,
                else_if_blocks,
                else_block,
                location,
            } => {
                self.compile_if_statement(
                    condition,
                    then_block,
                    else_if_blocks,
                    else_block,
                    *location,
                )
            }

            Statement::DoLoop {
                variable,
                start,
                end,
                step,
                body,
                location,
            } => {
                self.compile_do_loop(variable, start, end, step.as_ref(), body, *location)
            }

            Statement::DoWhile {
                condition,
                body,
                location,
            } => {
                self.compile_do_while(condition, body, *location)
            }

            Statement::DoInfinite { body, location } => {
                self.compile_do_infinite(body, *location)
            }

            Statement::SelectCase {
                selector,
                cases,
                default,
                location,
            } => {
                self.compile_select_case(selector, cases, default.as_ref(), *location)
            }

            Statement::Exit { location } => {
                // Exit jumps to end of innermost loop
                if let Some(loop_ctx) = self.loop_stack.last_mut() {
                    // Emit jump with placeholder, record for patching
                    let jump_idx = self.chunk.emit_with_operand(OpCode::Jump, 0, *location);
                    loop_ctx.exit_jumps.push(jump_idx);
                    Ok(())
                } else {
                    Err(CompileError::InvalidOperation {
                        message: "EXIT statement outside of loop".to_string(),
                        location: *location,
                    })
                }
            }

            Statement::Cycle { location } => {
                // Cycle jumps to loop continuation (increment/condition check)
                if let Some(loop_ctx) = self.loop_stack.last_mut() {
                    // Emit jump with placeholder, record for patching
                    let jump_idx = self.chunk.emit_with_operand(OpCode::Jump, 0, *location);
                    loop_ctx.cycle_jumps.push(jump_idx);
                    Ok(())
                } else {
                    Err(CompileError::InvalidOperation {
                        message: "CYCLE statement outside of loop".to_string(),
                        location: *location,
                    })
                }
            }

            Statement::Continue { location } => {
                // Continue is a no-op
                self.chunk.emit(OpCode::Nop, *location);
                Ok(())
            }

            Statement::Call { name, arguments, location } => {
                // Compile arguments (push onto stack)
                for arg in arguments {
                    self.compile_expression(arg)?;
                }

                // Look up procedure address
                let proc_address = self.chunk.get_procedure_address(name)
                    .ok_or(CompileError::InvalidOperation {
                        message: format!("Undefined procedure: {}", name),
                        location: *location,
                    })?;

                // Emit call instruction with procedure address
                self.chunk.emit_with_operand(OpCode::Call, proc_address, *location);
                Ok(())
            }

            Statement::Return { value, location } => {
                // If there's a return value, compile it
                if let Some(val) = value {
                    self.compile_expression(val)?;
                }

                // Emit return instruction
                self.chunk.emit(OpCode::Return, *location);
                Ok(())
            }

            Statement::Write { unit, values, location, .. } => {
                match unit {
                    None => {
                        // WRITE(*, *) - write to stdout (same as PRINT)
                        for value in values {
                            self.compile_expression(value)?;
                        }
                        self.chunk
                            .emit_with_operand(OpCode::Print, values.len(), *location);
                    }
                    Some(unit_expr) => {
                        // WRITE(unit, *) - write to file
                        // First compile and push all values
                        for value in values {
                            self.compile_expression(value)?;
                        }
                        // Then compile unit expression (pushed last, on top)
                        self.compile_expression(unit_expr)?;
                        // Emit WriteFile with value count
                        self.chunk
                            .emit_with_operand(OpCode::WriteFile, values.len(), *location);
                    }
                }
                Ok(())
            }

            Statement::Read { unit, variables, location, .. } => {
                match unit {
                    None => {
                        // READ(*, *) - read from stdin
                        for var_name in variables {
                            let var_index = self.chunk.add_variable(var_name.clone());
                            self.chunk.emit_with_operand(OpCode::Read, var_index, *location);
                        }
                    }
                    Some(unit_expr) => {
                        // READ(unit, *) - read from file
                        for var_name in variables {
                            // Push unit expression for each read
                            self.compile_expression(unit_expr)?;
                            let var_index = self.chunk.add_variable(var_name.clone());
                            self.chunk.emit_with_operand(OpCode::ReadFile, var_index, *location);
                        }
                    }
                }
                Ok(())
            }

            Statement::Open { unit, file, location, .. } => {
                // Push filename if provided, otherwise empty string
                if let Some(file_expr) = file {
                    self.compile_expression(file_expr)?;
                } else {
                    // No filename - push empty string (scratch file)
                    let idx = self.chunk.add_constant(Value::Character(String::new()));
                    self.chunk.emit_with_operand(OpCode::LoadConst, idx, *location);
                }

                // Compile unit expression to get the unit number
                self.compile_expression(unit)?;

                // Emit OpenFile
                self.chunk.emit(OpCode::OpenFile, *location);
                Ok(())
            }

            Statement::Close { unit, location, .. } => {
                // Compile unit expression
                self.compile_expression(unit)?;

                // Emit CloseFile
                self.chunk.emit(OpCode::CloseFile, *location);
                Ok(())
            }
        }
    }

    /// Compile an IF statement
    fn compile_if_statement(
        &mut self,
        condition: &Expr,
        then_block: &[Statement],
        else_if_blocks: &[(Expr, Vec<Statement>)],
        else_block: &Option<Vec<Statement>>,
        location: SourceLocation,
    ) -> CompileResult<()> {
        // Compile condition
        self.compile_expression(condition)?;

        // Jump to else/elif if false
        let jump_to_else = self.chunk.emit_with_operand(OpCode::JumpIfFalse, 0, location);

        // Compile then block
        for stmt in then_block {
            self.compile_statement(stmt)?;
        }

        // Jump over else blocks
        let mut end_jumps = vec![];
        if !else_if_blocks.is_empty() || else_block.is_some() {
            end_jumps.push(self.chunk.emit_with_operand(OpCode::Jump, 0, location));
        }

        // Patch jump to first else/elif
        self.chunk.patch_jump(jump_to_else, self.chunk.current_offset());

        // Compile else if blocks
        for (elif_cond, elif_body) in else_if_blocks {
            self.compile_expression(elif_cond)?;
            let jump_to_next = self.chunk.emit_with_operand(OpCode::JumpIfFalse, 0, location);

            for stmt in elif_body {
                self.compile_statement(stmt)?;
            }

            end_jumps.push(self.chunk.emit_with_operand(OpCode::Jump, 0, location));
            self.chunk.patch_jump(jump_to_next, self.chunk.current_offset());
        }

        // Compile else block
        if let Some(else_stmts) = else_block {
            for stmt in else_stmts {
                self.compile_statement(stmt)?;
            }
        }

        // Patch all end jumps
        let end_offset = self.chunk.current_offset();
        for jump in end_jumps {
            self.chunk.patch_jump(jump, end_offset);
        }

        Ok(())
    }

    /// Compile a counted DO loop
    fn compile_do_loop(
        &mut self,
        variable: &str,
        start: &Expr,
        end: &Expr,
        step: Option<&Expr>,
        body: &[Statement],
        location: SourceLocation,
    ) -> CompileResult<()> {
        let var_index = self.chunk.add_variable(variable.to_string());

        // Initialize loop variable
        self.compile_expression(start)?;
        self.chunk.emit_with_operand(OpCode::StoreVar, var_index, location);

        // Loop start (condition check)
        let loop_start = self.chunk.current_offset();

        // Determine if step is negative (compile-time check for literal steps)
        // For runtime-determined steps, we'd need more complex logic
        // For now, we check at compile time if step is a negative literal
        let is_negative_step = match step {
            Some(Expr::IntegerLiteral(n, _)) => *n < 0,
            Some(Expr::UnaryOp { op: UnaryOperator::Minus, operand, .. }) => {
                matches!(operand.as_ref(), Expr::IntegerLiteral(_, _))
            }
            _ => false,
        };

        // Check condition: variable <= end (or >= for negative step)
        self.chunk.emit_with_operand(OpCode::LoadVar, var_index, location);
        self.compile_expression(end)?;
        if is_negative_step {
            self.chunk.emit(OpCode::GreaterEqual, location);
        } else {
            self.chunk.emit(OpCode::LessEqual, location);
        }

        // Jump out if condition is false
        let exit_jump = self.chunk.emit_with_operand(OpCode::JumpIfFalse, 0, location);

        // Push loop context
        self.loop_stack.push(LoopContext {
            loop_start,
            exit_jumps: Vec::new(),
            cycle_jumps: Vec::new(),
            name: None,
        });

        // Compile body
        for stmt in body {
            self.compile_statement(stmt)?;
        }

        // Continue target is the increment section (where CYCLE should go)
        let continue_target = self.chunk.current_offset();

        // Increment loop variable
        self.chunk.emit_with_operand(OpCode::LoadVar, var_index, location);
        if let Some(step_expr) = step {
            self.compile_expression(step_expr)?;
        } else {
            // Default step is 1
            let one_index = self.chunk.add_constant(Value::Integer(1));
            self.chunk.emit_with_operand(OpCode::LoadConst, one_index, location);
        }
        self.chunk.emit(OpCode::Add, location);
        self.chunk.emit_with_operand(OpCode::StoreVar, var_index, location);

        // Jump back to loop start
        self.chunk.emit_with_operand(OpCode::Jump, loop_start, location);

        // Loop end - patch condition exit jump
        let loop_end = self.chunk.current_offset();
        self.chunk.patch_jump(exit_jump, loop_end);

        // Pop loop context and patch EXIT/CYCLE jumps
        if let Some(loop_ctx) = self.loop_stack.pop() {
            // Patch EXIT jumps to loop end
            for exit_jump_idx in loop_ctx.exit_jumps {
                self.chunk.patch_jump(exit_jump_idx, loop_end);
            }
            // Patch CYCLE jumps to continue target (increment section)
            for cycle_jump_idx in loop_ctx.cycle_jumps {
                self.chunk.patch_jump(cycle_jump_idx, continue_target);
            }
        }

        Ok(())
    }

    /// Compile a DO WHILE loop
    fn compile_do_while(
        &mut self,
        condition: &Expr,
        body: &[Statement],
        location: SourceLocation,
    ) -> CompileResult<()> {
        let loop_start = self.chunk.current_offset();

        // Check condition
        self.compile_expression(condition)?;
        let exit_jump = self.chunk.emit_with_operand(OpCode::JumpIfFalse, 0, location);

        // Push loop context
        self.loop_stack.push(LoopContext {
            loop_start,
            exit_jumps: Vec::new(),
            cycle_jumps: Vec::new(),
            name: None,
        });

        // Compile body
        for stmt in body {
            self.compile_statement(stmt)?;
        }

        // Jump back to loop start
        self.chunk.emit_with_operand(OpCode::Jump, loop_start, location);

        // Loop end
        let loop_end = self.chunk.current_offset();
        self.chunk.patch_jump(exit_jump, loop_end);

        // Pop loop context and patch EXIT/CYCLE jumps
        if let Some(loop_ctx) = self.loop_stack.pop() {
            // Patch EXIT jumps to loop end
            for exit_jump_idx in loop_ctx.exit_jumps {
                self.chunk.patch_jump(exit_jump_idx, loop_end);
            }
            // For DO WHILE, CYCLE jumps back to condition check (loop_start)
            for cycle_jump_idx in loop_ctx.cycle_jumps {
                self.chunk.patch_jump(cycle_jump_idx, loop_start);
            }
        }

        Ok(())
    }

    /// Compile an infinite DO loop
    fn compile_do_infinite(
        &mut self,
        body: &[Statement],
        location: SourceLocation,
    ) -> CompileResult<()> {
        let loop_start = self.chunk.current_offset();

        // Push loop context
        self.loop_stack.push(LoopContext {
            loop_start,
            exit_jumps: Vec::new(),
            cycle_jumps: Vec::new(),
            name: None,
        });

        // Compile body
        for stmt in body {
            self.compile_statement(stmt)?;
        }

        // Jump back to loop start
        self.chunk.emit_with_operand(OpCode::Jump, loop_start, location);

        // Loop end
        let loop_end = self.chunk.current_offset();

        // Pop loop context and patch EXIT/CYCLE jumps
        if let Some(loop_ctx) = self.loop_stack.pop() {
            // Patch EXIT jumps to loop end
            for exit_jump_idx in loop_ctx.exit_jumps {
                self.chunk.patch_jump(exit_jump_idx, loop_end);
            }
            // For infinite loops, CYCLE jumps back to loop start
            for cycle_jump_idx in loop_ctx.cycle_jumps {
                self.chunk.patch_jump(cycle_jump_idx, loop_start);
            }
        }

        Ok(())
    }

    /// Compile a SELECT CASE statement
    fn compile_select_case(
        &mut self,
        selector: &Expr,
        cases: &[CaseClause],
        default: Option<&Vec<Statement>>,
        location: SourceLocation,
    ) -> CompileResult<()> {
        // Compile selector and store in temp
        self.compile_expression(selector)?;

        let mut end_jumps = vec![];

        // Compile each case
        for case in cases {
            // Duplicate selector value for comparison
            self.chunk.emit(OpCode::Dup, location);

            // Compile case selector comparison
            match &case.selector {
                CaseSelector::Value(expr) => {
                    self.compile_expression(expr)?;
                    self.chunk.emit(OpCode::Equal, location);
                }
                CaseSelector::Values(exprs) => {
                    // Check if selector equals any value
                    for (i, expr) in exprs.iter().enumerate() {
                        if i > 0 {
                            self.chunk.emit(OpCode::Dup, location);
                        }
                        self.compile_expression(expr)?;
                        self.chunk.emit(OpCode::Equal, location);
                        if i > 0 {
                            self.chunk.emit(OpCode::Or, location);
                        }
                    }
                }
                CaseSelector::Range(start, end) => {
                    // Check if selector >= start AND selector <= end
                    // Stack after initial Dup at line 852: [..., selector, selector_copy]
                    // We need another copy for the second comparison
                    self.chunk.emit(OpCode::Dup, location);  // [..., sel, copy1, copy2]

                    // First comparison: copy2 >= start
                    self.compile_expression(start)?;
                    self.chunk.emit(OpCode::GreaterEqual, location); // [..., sel, copy1, bool_ge]

                    // Store bool_ge in temp variable (since we can't swap on stack)
                    let temp_idx = self.chunk.add_variable("__range_temp".to_string());
                    self.chunk.emit_with_operand(OpCode::StoreVar, temp_idx, location);
                    // Stack: [..., sel, copy1]

                    // Second comparison: copy1 <= end
                    self.compile_expression(end)?;
                    self.chunk.emit(OpCode::LessEqual, location); // [..., sel, bool_le]

                    // Load back bool_ge and AND the results
                    self.chunk.emit_with_operand(OpCode::LoadVar, temp_idx, location);
                    self.chunk.emit(OpCode::And, location); // [..., sel, (bool_ge AND bool_le)]
                }
            }

            // Jump to next case if not matching
            let next_case = self.chunk.emit_with_operand(OpCode::JumpIfFalse, 0, location);

            // Pop the duplicated selector
            self.chunk.emit(OpCode::Pop, location);

            // Compile case body
            for stmt in &case.body {
                self.compile_statement(stmt)?;
            }

            // Jump to end
            end_jumps.push(self.chunk.emit_with_operand(OpCode::Jump, 0, location));

            // Patch next case jump
            self.chunk.patch_jump(next_case, self.chunk.current_offset());
        }

        // Pop selector value
        self.chunk.emit(OpCode::Pop, location);

        // Compile default case
        if let Some(default_stmts) = default {
            for stmt in default_stmts {
                self.compile_statement(stmt)?;
            }
        }

        // Patch end jumps
        let end_offset = self.chunk.current_offset();
        for jump in end_jumps {
            self.chunk.patch_jump(jump, end_offset);
        }

        Ok(())
    }

    /// Compile an expression
    fn compile_expression(&mut self, expr: &Expr) -> CompileResult<()> {
        match expr {
            Expr::IntegerLiteral(value, location) => {
                let index = self.chunk.add_constant(Value::Integer(*value));
                self.chunk.emit_with_operand(OpCode::LoadConst, index, *location);
                Ok(())
            }

            Expr::RealLiteral(value, location) => {
                let index = self.chunk.add_constant(Value::Real(*value));
                self.chunk.emit_with_operand(OpCode::LoadConst, index, *location);
                Ok(())
            }

            Expr::StringLiteral(value, location) => {
                let index = self.chunk.add_constant(Value::Character(value.clone()));
                self.chunk.emit_with_operand(OpCode::LoadConst, index, *location);
                Ok(())
            }

            Expr::LogicalLiteral(value, location) => {
                if *value {
                    self.chunk.emit(OpCode::LoadTrue, *location);
                } else {
                    self.chunk.emit(OpCode::LoadFalse, *location);
                }
                Ok(())
            }

            Expr::Identifier(name, location) => {
                let var_index = self.chunk.add_variable(name.clone());
                self.chunk.emit_with_operand(OpCode::LoadVar, var_index, *location);
                Ok(())
            }

            Expr::BinaryOp { op, left, right, location } => {
                // Check for operator overloading only when operands are derived type variables
                // (not component accesses, literals, or other expressions)
                let use_overloaded = if self.is_derived_type_expr(left) && self.is_derived_type_expr(right) {
                    let overloadable_op = Self::binary_to_overloadable(op);
                    if let Some(procedures) = self.chunk.get_operator_procedures(&overloadable_op) {
                        if let Some(proc_name) = procedures.first() {
                            if let Some(proc_addr) = self.chunk.get_procedure_address(proc_name) {
                                // Compile arguments (left, right)
                                self.compile_expression(left)?;
                                self.compile_expression(right)?;
                                // Call the overloaded operator procedure
                                self.chunk.emit_with_operand(OpCode::Call, proc_addr, *location);
                                return Ok(());
                            }
                        }
                    }
                    false
                } else {
                    false
                };

                if use_overloaded {
                    return Ok(());
                }

                // Fall back to built-in operator
                // Compile left operand
                self.compile_expression(left)?;
                // Compile right operand
                self.compile_expression(right)?;
                // Emit operation
                let opcode = match op {
                    BinaryOperator::Add => OpCode::Add,
                    BinaryOperator::Subtract => OpCode::Subtract,
                    BinaryOperator::Multiply => OpCode::Multiply,
                    BinaryOperator::Divide => OpCode::Divide,
                    BinaryOperator::Power => OpCode::Power,
                    BinaryOperator::Equal => OpCode::Equal,
                    BinaryOperator::NotEqual => OpCode::NotEqual,
                    BinaryOperator::Less => OpCode::Less,
                    BinaryOperator::LessEqual => OpCode::LessEqual,
                    BinaryOperator::Greater => OpCode::Greater,
                    BinaryOperator::GreaterEqual => OpCode::GreaterEqual,
                    BinaryOperator::And => OpCode::And,
                    BinaryOperator::Or => OpCode::Or,
                    BinaryOperator::Eqv => OpCode::Equal, // Logical equivalence
                    BinaryOperator::Neqv => OpCode::NotEqual, // Logical non-equivalence
                };
                self.chunk.emit(opcode, *location);
                Ok(())
            }

            Expr::UnaryOp { op, operand, location } => {
                self.compile_expression(operand)?;
                let opcode = match op {
                    UnaryOperator::Minus => OpCode::Negate,
                    UnaryOperator::Plus => return Ok(()), // Unary plus is a no-op
                    UnaryOperator::Not => OpCode::Not,
                };
                self.chunk.emit(opcode, *location);
                Ok(())
            }

            Expr::Parenthesized(inner, _) => {
                self.compile_expression(inner)
            }

            Expr::FunctionCall { name, arguments, location } => {
                // Check for intrinsic function FIRST
                if let Some(intrinsic) = Intrinsic::from_name(name) {
                    // Validate argument count
                    let (min_args, max_args) = intrinsic.arg_count();
                    let arg_count = arguments.len();
                    if arg_count < min_args || arg_count > max_args {
                        return Err(CompileError::InvalidOperation {
                            message: format!(
                                "Intrinsic {} expects {}-{} arguments, got {}",
                                intrinsic, min_args, max_args, arg_count
                            ),
                            location: *location,
                        });
                    }

                    // Compile all arguments (push onto stack)
                    for arg in arguments {
                        self.compile_expression(arg)?;
                    }

                    // Encode operand: (intrinsic_id << 8) | arg_count
                    let operand = ((intrinsic as usize) << 8) | arg_count;
                    self.chunk.emit_with_operand(OpCode::CallIntrinsic, operand, *location);
                } else if self.chunk.get_procedure_address(name).is_some() {
                    // This is a user-defined function call
                    // Compile arguments (push onto stack)
                    for arg in arguments {
                        self.compile_expression(arg)?;
                    }

                    // Look up function address
                    let func_address = self.chunk.get_procedure_address(name).unwrap();

                    // Emit call instruction with function address
                    // The function will leave its return value on the stack
                    self.chunk.emit_with_operand(OpCode::Call, func_address, *location);
                } else if self.chunk.has_variable(name) {
                    // This is array access (variable exists but not a procedure)
                    // Push each index FIRST
                    for arg in arguments {
                        self.compile_expression(arg)?;
                    }

                    // Push number of indices LAST (so it's on top of stack)
                    let num_idx = arguments.len();
                    let idx_count = self.chunk.add_constant(Value::Integer(num_idx as i64));
                    self.chunk.emit_with_operand(OpCode::LoadConst, idx_count, *location);

                    // Get variable index
                    let var_index = self.chunk.add_variable(name.clone());

                    // Emit LoadArrayElem
                    self.chunk.emit_with_operand(OpCode::LoadArrayElem, var_index, *location);
                } else if self.chunk.get_type_index(name).is_some() {
                    // This is a type constructor (structure constructor)
                    let type_index = self.chunk.get_type_index(name).unwrap();

                    // Push all arguments (component values) onto the stack
                    for arg in arguments {
                        self.compile_expression(arg)?;
                    }

                    // Push number of arguments
                    let arg_count = arguments.len();
                    let count_idx = self.chunk.add_constant(Value::Integer(arg_count as i64));
                    self.chunk.emit_with_operand(OpCode::LoadConst, count_idx, *location);

                    // Emit CreateInstance with type index
                    self.chunk.emit_with_operand(OpCode::CreateInstance, type_index, *location);
                } else {
                    // Unknown identifier - could be a function not yet compiled or an error
                    return Err(CompileError::InvalidOperation {
                        message: format!("Undefined function or array: {}", name),
                        location: *location,
                    });
                }
                Ok(())
            }

            Expr::ArrayAccess { name, indices, location } => {
                // Array element access: arr(i, j, ...)
                // Push each index FIRST
                for idx in indices {
                    self.compile_expression(idx)?;
                }

                // Push number of indices LAST (so it's on top of stack)
                let num_idx = indices.len();
                let idx_count = self.chunk.add_constant(Value::Integer(num_idx as i64));
                self.chunk.emit_with_operand(OpCode::LoadConst, idx_count, *location);

                // Get variable index
                let var_index = self.chunk.add_variable(name.clone());

                // Emit LoadArrayElem
                self.chunk.emit_with_operand(OpCode::LoadArrayElem, var_index, *location);
                Ok(())
            }

            Expr::ComponentAccess { object, component, location } => {
                // Compile the object expression (puts instance on stack)
                self.compile_expression(object)?;

                // Store component name as constant for runtime lookup
                let comp_idx = self.chunk.add_constant(Value::Character(component.clone()));

                // Emit LoadComponent with component name index
                self.chunk.emit_with_operand(OpCode::LoadComponent, comp_idx, *location);
                Ok(())
            }

            Expr::TypeConstructor { type_name, arguments, location } => {
                // Look up the type definition
                let type_index = self.chunk.get_type_index(type_name).ok_or_else(|| {
                    CompileError::InvalidOperation {
                        message: format!("Unknown derived type: {}", type_name),
                        location: *location,
                    }
                })?;

                // Push all arguments (component values) onto the stack
                for arg in arguments {
                    self.compile_expression(arg)?;
                }

                // Push number of arguments
                let arg_count = arguments.len();
                let count_idx = self.chunk.add_constant(Value::Integer(arg_count as i64));
                self.chunk.emit_with_operand(OpCode::LoadConst, count_idx, *location);

                // Emit CreateInstance with type index
                self.chunk.emit_with_operand(OpCode::CreateInstance, type_index, *location);
                Ok(())
            }
        }
    }

    /// Get the compiled chunk (consumes the compiler)
    pub fn into_chunk(self) -> Chunk {
        self.chunk
    }

    /// Convert BinaryOperator to OverloadableOperator for operator overloading lookup
    fn binary_to_overloadable(op: &BinaryOperator) -> OverloadableOperator {
        match op {
            BinaryOperator::Add => OverloadableOperator::Add,
            BinaryOperator::Subtract => OverloadableOperator::Subtract,
            BinaryOperator::Multiply => OverloadableOperator::Multiply,
            BinaryOperator::Divide => OverloadableOperator::Divide,
            BinaryOperator::Power => OverloadableOperator::Power,
            BinaryOperator::Equal => OverloadableOperator::Equal,
            BinaryOperator::NotEqual => OverloadableOperator::NotEqual,
            BinaryOperator::Less => OverloadableOperator::Less,
            BinaryOperator::LessEqual => OverloadableOperator::LessEqual,
            BinaryOperator::Greater => OverloadableOperator::Greater,
            BinaryOperator::GreaterEqual => OverloadableOperator::GreaterEqual,
            BinaryOperator::And => OverloadableOperator::And,
            BinaryOperator::Or => OverloadableOperator::Or,
            BinaryOperator::Eqv => OverloadableOperator::Eqv,
            BinaryOperator::Neqv => OverloadableOperator::Neqv,
        }
    }

    /// Check if an expression is a derived type variable (not a component access or primitive)
    fn is_derived_type_expr(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Identifier(name, ..) => {
                // Check if this variable is declared as a derived type
                // We check if there are any non-primitive types registered
                self.chunk.types.iter().any(|t| {
                    // If we have a variable with this name and there's a derived type
                    self.chunk.get_variable_index(name).is_some()
                        && t.name.to_uppercase() != "INTEGER"
                        && t.name.to_uppercase() != "REAL"
                        && t.name.to_uppercase() != "LOGICAL"
                        && t.name.to_uppercase() != "CHARACTER"
                })
            }
            // Component accesses like p%x are NOT derived types (they're primitive components)
            Expr::ComponentAccess { .. } => false,
            // Literals are not derived types
            Expr::IntegerLiteral(..)
            | Expr::RealLiteral(..)
            | Expr::LogicalLiteral(..)
            | Expr::StringLiteral(..) => false,
            // Binary operations result in primitives (unless they're overloaded, but we can't know that here)
            Expr::BinaryOp { .. } | Expr::UnaryOp { .. } => false,
            // Function calls could return derived types, but we don't have type info
            Expr::FunctionCall { .. } => false,
            // Type constructors return derived types
            Expr::TypeConstructor { .. } => true,
            // Default to false for safety
            _ => false,
        }
    }
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opcode_display() {
        assert_eq!(format!("{}", OpCode::LoadConst), "LoadConst");
        assert_eq!(format!("{}", OpCode::Add), "Add");
        assert_eq!(format!("{}", OpCode::JumpIfFalse), "JumpIfFalse");
    }

    #[test]
    fn test_value_display() {
        assert_eq!(format!("{}", Value::Integer(42)), "42");
        assert_eq!(format!("{}", Value::Real(3.14)), "3.14");
        assert_eq!(format!("{}", Value::Logical(true)), ".TRUE.");
        assert_eq!(format!("{}", Value::Character("hello".into())), "'hello'");
    }

    #[test]
    fn test_constant_pool_deduplication() {
        let mut pool = ConstantPool::new();

        let idx1 = pool.add(Value::Integer(42));
        let idx2 = pool.add(Value::Integer(42));
        let idx3 = pool.add(Value::Integer(100));

        assert_eq!(idx1, idx2); // Same value, same index
        assert_ne!(idx1, idx3); // Different value, different index
        assert_eq!(pool.len(), 2);
    }

    #[test]
    fn test_chunk_emit_instructions() {
        let mut chunk = Chunk::new();
        let loc = SourceLocation { line: 1, column: 1 };

        let idx = chunk.add_constant(Value::Integer(42));
        chunk.emit_with_operand(OpCode::LoadConst, idx, loc);
        chunk.emit(OpCode::Negate, loc);
        chunk.emit(OpCode::Halt, loc);

        assert_eq!(chunk.instructions.len(), 3);
        assert_eq!(chunk.instructions[0].opcode, OpCode::LoadConst);
        assert_eq!(chunk.instructions[0].operand, Some(0));
        assert_eq!(chunk.instructions[1].opcode, OpCode::Negate);
        assert_eq!(chunk.instructions[2].opcode, OpCode::Halt);
    }

    #[test]
    fn test_chunk_variables() {
        let mut chunk = Chunk::new();

        let idx1 = chunk.add_variable("X".to_string());
        let idx2 = chunk.add_variable("Y".to_string());
        let idx3 = chunk.add_variable("X".to_string()); // Same as idx1

        assert_eq!(idx1, 0);
        assert_eq!(idx2, 1);
        assert_eq!(idx3, idx1); // Reuses existing variable
        assert_eq!(chunk.variables.len(), 2);
    }

    #[test]
    fn test_chunk_patch_jump() {
        let mut chunk = Chunk::new();
        let loc = SourceLocation { line: 1, column: 1 };

        let jump_idx = chunk.emit_with_operand(OpCode::Jump, 0, loc);
        chunk.emit(OpCode::Nop, loc);
        chunk.emit(OpCode::Nop, loc);
        let target = chunk.current_offset();
        chunk.patch_jump(jump_idx, target);

        assert_eq!(chunk.instructions[0].operand, Some(3));
    }

    #[test]
    fn test_disassemble() {
        let mut chunk = Chunk::new();
        let loc = SourceLocation { line: 1, column: 1 };

        let const_idx = chunk.add_constant(Value::Integer(42));
        let var_idx = chunk.add_variable("X".to_string());

        chunk.emit_with_operand(OpCode::LoadConst, const_idx, loc);
        chunk.emit_with_operand(OpCode::StoreVar, var_idx, loc);
        chunk.emit(OpCode::Halt, loc);

        let output = chunk.disassemble("test");
        assert!(output.contains("LoadConst"));
        assert!(output.contains("42"));
        assert!(output.contains("StoreVar"));
        assert!(output.contains("X"));
    }

    // Array tests

    #[test]
    fn test_array_dim() {
        let dim = ArrayDim::new(1, 10);
        assert_eq!(dim.size(), 10);

        let dim2 = ArrayDim::new(-5, 5);
        assert_eq!(dim2.size(), 11);

        let dim3 = ArrayDim::new(0, 0);
        assert_eq!(dim3.size(), 1);
    }

    #[test]
    fn test_new_integer_array() {
        let dims = vec![ArrayDim::new(1, 5)];
        let arr = Value::new_integer_array(dims);

        assert!(arr.is_array());
        assert_eq!(arr.rank(), Some(1));
        assert_eq!(arr.type_name(), "ARRAY");

        if let Value::Array { elements, .. } = &arr {
            assert_eq!(elements.len(), 5);
            assert!(elements.iter().all(|e| *e == Value::Integer(0)));
        } else {
            panic!("Expected array");
        }
    }

    #[test]
    fn test_new_real_array() {
        let dims = vec![ArrayDim::new(1, 3)];
        let arr = Value::new_real_array(dims);

        if let Value::Array { elements, .. } = &arr {
            assert_eq!(elements.len(), 3);
            assert!(elements.iter().all(|e| *e == Value::Real(0.0)));
        } else {
            panic!("Expected array");
        }
    }

    #[test]
    fn test_2d_array() {
        let dims = vec![ArrayDim::new(1, 3), ArrayDim::new(1, 4)];
        let arr = Value::new_integer_array(dims);

        assert_eq!(arr.rank(), Some(2));

        if let Value::Array { elements, .. } = &arr {
            // 3 * 4 = 12 elements
            assert_eq!(elements.len(), 12);
        } else {
            panic!("Expected array");
        }
    }

    #[test]
    fn test_array_linear_index_1d() {
        let dims = vec![ArrayDim::new(1, 5)];
        let arr = Value::new_integer_array(dims);

        // Indices [1] through [5] should map to 0..4
        assert_eq!(arr.linear_index(&[1]), Some(0));
        assert_eq!(arr.linear_index(&[3]), Some(2));
        assert_eq!(arr.linear_index(&[5]), Some(4));

        // Out of bounds
        assert_eq!(arr.linear_index(&[0]), None);
        assert_eq!(arr.linear_index(&[6]), None);
    }

    #[test]
    fn test_array_linear_index_2d_column_major() {
        // 3x4 array (Fortran column-major)
        let dims = vec![ArrayDim::new(1, 3), ArrayDim::new(1, 4)];
        let arr = Value::new_integer_array(dims);

        // In column-major order:
        // (1,1)->0, (2,1)->1, (3,1)->2, (1,2)->3, (2,2)->4, (3,2)->5, ...
        assert_eq!(arr.linear_index(&[1, 1]), Some(0));
        assert_eq!(arr.linear_index(&[2, 1]), Some(1));
        assert_eq!(arr.linear_index(&[3, 1]), Some(2));
        assert_eq!(arr.linear_index(&[1, 2]), Some(3));
        assert_eq!(arr.linear_index(&[2, 2]), Some(4));
        assert_eq!(arr.linear_index(&[3, 4]), Some(11));
    }

    #[test]
    fn test_array_get_set_element() {
        let dims = vec![ArrayDim::new(1, 5)];
        let mut arr = Value::new_integer_array(dims);

        // Set element
        assert!(arr.set_element(&[3], Value::Integer(42)).is_some());

        // Get element
        assert_eq!(arr.get_element(&[3]), Some(&Value::Integer(42)));
        assert_eq!(arr.get_element(&[1]), Some(&Value::Integer(0)));

        // Out of bounds
        assert_eq!(arr.get_element(&[0]), None);
        assert!(arr.set_element(&[6], Value::Integer(99)).is_none());
    }

    #[test]
    fn test_array_display() {
        let dims = vec![ArrayDim::new(1, 3)];
        let mut arr = Value::new_integer_array(dims);
        arr.set_element(&[1], Value::Integer(10));
        arr.set_element(&[2], Value::Integer(20));
        arr.set_element(&[3], Value::Integer(30));

        let display = format!("{}", arr);
        assert!(display.contains("[10, 20, 30]"));
        assert!(display.contains("shape(3)"));
    }

    #[test]
    fn test_array_non_unit_lower_bound() {
        // Array with non-1 lower bound: arr(-2:2) has 5 elements
        let dims = vec![ArrayDim::new(-2, 2)];
        let mut arr = Value::new_integer_array(dims);

        assert_eq!(arr.linear_index(&[-2]), Some(0));
        assert_eq!(arr.linear_index(&[0]), Some(2));
        assert_eq!(arr.linear_index(&[2]), Some(4));

        arr.set_element(&[-1], Value::Integer(100));
        assert_eq!(arr.get_element(&[-1]), Some(&Value::Integer(100)));
    }
}
