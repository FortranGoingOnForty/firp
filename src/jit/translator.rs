//! Bytecode to Cranelift IR translation
//!
//! This module handles the core compilation from FIRP bytecode to
//! Cranelift IR, which is then compiled to native code.

use super::context::{call_conv, pointer_type};
use super::{CompiledFn, JitError, JitRuntimeContext};
use crate::bytecode::{Chunk, Instruction, OpCode, Value};
use cranelift::prelude::*;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::JITModule;
use cranelift_module::{Linkage, Module};
use std::collections::HashMap;

/// Translate a procedure to native code
///
/// # Arguments
/// * `module` - The JIT module to compile into
/// * `builder_ctx` - Reusable function builder context
/// * `codegen_ctx` - Reusable codegen context
/// * `name` - Procedure name (for symbol table)
/// * `entry_ip` - Bytecode entry point
/// * `end_ip` - Bytecode end point (exclusive)
/// * `chunk` - The bytecode chunk containing instructions
/// * `debug_info` - Whether to include debug information
pub fn translate_procedure(
    module: &mut JITModule,
    builder_ctx: &mut FunctionBuilderContext,
    codegen_ctx: &mut codegen::Context,
    name: &str,
    entry_ip: usize,
    end_ip: usize,
    chunk: &Chunk,
    _debug_info: bool,
) -> Result<CompiledFn, JitError> {
    // Clear the codegen context for reuse
    codegen_ctx.clear();

    // Build the function signature: fn(*mut JitRuntimeContext) -> i64
    let ptr_type = pointer_type();
    codegen_ctx.func.signature.params.push(AbiParam::new(ptr_type));
    codegen_ctx
        .func
        .signature
        .returns
        .push(AbiParam::new(types::I64));
    codegen_ctx.func.signature.call_conv = call_conv();

    // Create function builder
    {
        let mut builder = FunctionBuilder::new(&mut codegen_ctx.func, builder_ctx);

        // Create entry block
        let entry_block = builder.create_block();
        builder.append_block_params_for_function_params(entry_block);
        builder.switch_to_block(entry_block);
        builder.seal_block(entry_block);

        // Get the context pointer parameter
        let ctx_ptr = builder.block_params(entry_block)[0];

        // Create the translator and compile
        let mut translator = BytecodeTranslator::new(chunk, entry_ip, end_ip, ctx_ptr);
        translator.translate(&mut builder)?;

        // Finalize the function
        builder.finalize();
    }

    // Declare the function in the module
    let func_id = module
        .declare_function(name, Linkage::Local, &codegen_ctx.func.signature)
        .map_err(|e| JitError::ModuleError(e.to_string()))?;

    // Define the function
    module
        .define_function(func_id, codegen_ctx)
        .map_err(|e| JitError::CompilationFailed(e.to_string()))?;

    // Finalize the module (generate machine code)
    module.finalize_definitions().map_err(|e| {
        JitError::CompilationFailed(format!("Failed to finalize: {}", e))
    })?;

    // Get the function pointer
    let func_ptr = module.get_finalized_function(func_id);

    // Cast to our function type
    let compiled_fn: CompiledFn = unsafe { std::mem::transmute(func_ptr) };

    Ok(compiled_fn)
}

/// Translates FIRP bytecode to Cranelift IR
struct BytecodeTranslator<'a> {
    chunk: &'a Chunk,
    entry_ip: usize,
    end_ip: usize,
    ctx_ptr: cranelift::prelude::Value,
    /// Maps bytecode IP to Cranelift block
    block_map: HashMap<usize, Block>,
    /// Current evaluation stack (SSA values)
    stack: Vec<cranelift::prelude::Value>,
    /// Variable slots (for local variables)
    locals: Vec<Variable>,
    /// Current instruction pointer
    current_ip: usize,
    /// Track if current block is terminated
    block_terminated: bool,
}

impl<'a> BytecodeTranslator<'a> {
    fn new(
        chunk: &'a Chunk,
        entry_ip: usize,
        end_ip: usize,
        ctx_ptr: cranelift::prelude::Value,
    ) -> Self {
        Self {
            chunk,
            entry_ip,
            end_ip,
            ctx_ptr,
            block_map: HashMap::new(),
            stack: Vec::new(),
            locals: Vec::new(),
            current_ip: entry_ip,
            block_terminated: false,
        }
    }

    /// Main translation entry point
    fn translate(&mut self, builder: &mut FunctionBuilder) -> Result<(), JitError> {
        // Phase 1: Find basic block boundaries
        self.find_basic_blocks(builder)?;

        // Phase 2: Declare local variables
        self.declare_locals(builder)?;

        // Phase 3: Translate each instruction
        self.current_ip = self.entry_ip;
        self.block_terminated = false;

        while self.current_ip < self.end_ip {
            // Check if this IP starts a new block
            if let Some(&block) = self.block_map.get(&self.current_ip) {
                if self.current_ip > self.entry_ip {
                    // Jump to the new block if we're falling through
                    if !self.block_terminated {
                        builder.ins().jump(block, &[]);
                    }
                    builder.switch_to_block(block);
                    builder.seal_block(block);
                    self.block_terminated = false;
                }
            }

            let instr = &self.chunk.instructions[self.current_ip];
            self.translate_instruction(builder, instr)?;
            self.current_ip += 1;
        }

        // Ensure function ends with a return if not already
        if !self.block_terminated {
            let zero = builder.ins().iconst(types::I64, 0);
            builder.ins().return_(&[zero]);
        }

        Ok(())
    }

    /// Find basic block boundaries by scanning for jump targets
    fn find_basic_blocks(&mut self, builder: &mut FunctionBuilder) -> Result<(), JitError> {
        for ip in self.entry_ip..self.end_ip {
            let instr = &self.chunk.instructions[ip];
            match instr.opcode {
                OpCode::Jump | OpCode::JumpIfFalse | OpCode::JumpIfTrue => {
                    if let Some(target) = instr.operand {
                        if target >= self.entry_ip && target < self.end_ip {
                            // Target is a block start
                            self.block_map
                                .entry(target)
                                .or_insert_with(|| builder.create_block());
                        }
                        // Instruction after conditional jump is also a block start
                        if matches!(instr.opcode, OpCode::JumpIfFalse | OpCode::JumpIfTrue) {
                            let next_ip = ip + 1;
                            if next_ip < self.end_ip {
                                self.block_map
                                    .entry(next_ip)
                                    .or_insert_with(|| builder.create_block());
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Declare local variables for the function
    fn declare_locals(&mut self, builder: &mut FunctionBuilder) -> Result<(), JitError> {
        // Count variables used in this procedure
        let mut max_var = 0usize;
        for ip in self.entry_ip..self.end_ip {
            let instr = &self.chunk.instructions[ip];
            match instr.opcode {
                OpCode::LoadVar | OpCode::StoreVar => {
                    if let Some(var_idx) = instr.operand {
                        max_var = max_var.max(var_idx + 1);
                    }
                }
                _ => {}
            }
        }

        // Create Cranelift variables
        for i in 0..max_var {
            let var = Variable::new(i);
            builder.declare_var(var, types::I64);
            // Initialize to zero
            let zero = builder.ins().iconst(types::I64, 0);
            builder.def_var(var, zero);
            self.locals.push(var);
        }

        Ok(())
    }

    /// Translate a single instruction
    fn translate_instruction(
        &mut self,
        builder: &mut FunctionBuilder,
        instr: &Instruction,
    ) -> Result<(), JitError> {
        match instr.opcode {
            // === Constants ===
            OpCode::LoadConst => {
                let const_idx = instr.operand.ok_or_else(|| {
                    JitError::InvalidBytecode("LoadConst missing operand".into())
                })?;

                let value = self.load_constant(builder, const_idx)?;
                self.stack.push(value);
            }

            OpCode::LoadTrue => {
                let value = builder.ins().iconst(types::I64, 1);
                self.stack.push(value);
            }

            OpCode::LoadFalse => {
                let value = builder.ins().iconst(types::I64, 0);
                self.stack.push(value);
            }

            // === Variables ===
            OpCode::LoadVar => {
                let var_idx = instr.operand.ok_or_else(|| {
                    JitError::InvalidBytecode("LoadVar missing operand".into())
                })?;

                if var_idx < self.locals.len() {
                    let value = builder.use_var(self.locals[var_idx]);
                    self.stack.push(value);
                } else {
                    // Fall back: load from context
                    let value = self.load_var_from_context(builder, var_idx)?;
                    self.stack.push(value);
                }
            }

            OpCode::StoreVar | OpCode::StoreParam => {
                // StoreParam is effectively the same as StoreVar for JIT
                // Both store a value from the stack into a variable slot
                let var_idx = instr.operand.ok_or_else(|| {
                    JitError::InvalidBytecode("StoreVar/StoreParam missing operand".into())
                })?;

                let value = self.pop_stack()?;

                if var_idx < self.locals.len() {
                    builder.def_var(self.locals[var_idx], value);
                } else {
                    // Store to context
                    self.store_var_to_context(builder, var_idx, value)?;
                }
            }

            OpCode::LoadRef => {
                // LoadRef loads a reference to a variable
                // For JIT, we treat this as loading the variable index as a value
                let var_idx = instr.operand.ok_or_else(|| {
                    JitError::InvalidBytecode("LoadRef missing operand".into())
                })?;

                // For now, treat reference as loading the variable value
                // (simplified - full reference semantics would need more work)
                if var_idx < self.locals.len() {
                    let value = builder.use_var(self.locals[var_idx]);
                    self.stack.push(value);
                } else {
                    let value = self.load_var_from_context(builder, var_idx)?;
                    self.stack.push(value);
                }
            }

            // === Arithmetic ===
            OpCode::Add => {
                let b = self.pop_stack()?;
                let a = self.pop_stack()?;
                let result = builder.ins().iadd(a, b);
                self.stack.push(result);
            }

            OpCode::Subtract => {
                let b = self.pop_stack()?;
                let a = self.pop_stack()?;
                let result = builder.ins().isub(a, b);
                self.stack.push(result);
            }

            OpCode::Multiply => {
                let b = self.pop_stack()?;
                let a = self.pop_stack()?;
                let result = builder.ins().imul(a, b);
                self.stack.push(result);
            }

            OpCode::Divide => {
                let b = self.pop_stack()?;
                let a = self.pop_stack()?;
                // Use signed division for integers
                let result = builder.ins().sdiv(a, b);
                self.stack.push(result);
            }

            OpCode::Negate => {
                let a = self.pop_stack()?;
                let result = builder.ins().ineg(a);
                self.stack.push(result);
            }

            // === Comparisons ===
            OpCode::Less => {
                let b = self.pop_stack()?;
                let a = self.pop_stack()?;
                let cmp = builder.ins().icmp(IntCC::SignedLessThan, a, b);
                let result = builder.ins().uextend(types::I64, cmp);
                self.stack.push(result);
            }

            OpCode::LessEqual => {
                let b = self.pop_stack()?;
                let a = self.pop_stack()?;
                let cmp = builder.ins().icmp(IntCC::SignedLessThanOrEqual, a, b);
                let result = builder.ins().uextend(types::I64, cmp);
                self.stack.push(result);
            }

            OpCode::Greater => {
                let b = self.pop_stack()?;
                let a = self.pop_stack()?;
                let cmp = builder.ins().icmp(IntCC::SignedGreaterThan, a, b);
                let result = builder.ins().uextend(types::I64, cmp);
                self.stack.push(result);
            }

            OpCode::GreaterEqual => {
                let b = self.pop_stack()?;
                let a = self.pop_stack()?;
                let cmp = builder.ins().icmp(IntCC::SignedGreaterThanOrEqual, a, b);
                let result = builder.ins().uextend(types::I64, cmp);
                self.stack.push(result);
            }

            OpCode::Equal => {
                let b = self.pop_stack()?;
                let a = self.pop_stack()?;
                let cmp = builder.ins().icmp(IntCC::Equal, a, b);
                let result = builder.ins().uextend(types::I64, cmp);
                self.stack.push(result);
            }

            OpCode::NotEqual => {
                let b = self.pop_stack()?;
                let a = self.pop_stack()?;
                let cmp = builder.ins().icmp(IntCC::NotEqual, a, b);
                let result = builder.ins().uextend(types::I64, cmp);
                self.stack.push(result);
            }

            // === Logical ===
            OpCode::And => {
                let b = self.pop_stack()?;
                let a = self.pop_stack()?;
                let result = builder.ins().band(a, b);
                self.stack.push(result);
            }

            OpCode::Or => {
                let b = self.pop_stack()?;
                let a = self.pop_stack()?;
                let result = builder.ins().bor(a, b);
                self.stack.push(result);
            }

            OpCode::Not => {
                let a = self.pop_stack()?;
                // Logical NOT: if a == 0 then 1 else 0
                let zero = builder.ins().iconst(types::I64, 0);
                let cmp = builder.ins().icmp(IntCC::Equal, a, zero);
                let result = builder.ins().uextend(types::I64, cmp);
                self.stack.push(result);
            }

            // === Control Flow ===
            OpCode::Jump => {
                let target = instr
                    .operand
                    .ok_or_else(|| JitError::InvalidBytecode("Jump missing operand".into()))?;

                if let Some(&block) = self.block_map.get(&target) {
                    builder.ins().jump(block, &[]);
                    self.block_terminated = true;
                } else {
                    // Jump outside our compiled range - return error
                    let error_code = builder.ins().iconst(types::I64, 1);
                    builder.ins().return_(&[error_code]);
                    self.block_terminated = true;
                }
            }

            OpCode::JumpIfFalse => {
                let target = instr.operand.ok_or_else(|| {
                    JitError::InvalidBytecode("JumpIfFalse missing operand".into())
                })?;

                let cond = self.pop_stack()?;
                let zero = builder.ins().iconst(types::I64, 0);
                let is_false = builder.ins().icmp(IntCC::Equal, cond, zero);

                if let Some(&target_block) = self.block_map.get(&target) {
                    let next_ip = self.current_ip + 1;
                    if let Some(&fallthrough_block) = self.block_map.get(&next_ip) {
                        builder.ins().brif(
                            is_false,
                            target_block,
                            &[],
                            fallthrough_block,
                            &[],
                        );
                        self.block_terminated = true;
                    } else {
                        // Create fallthrough block
                        let fallthrough = builder.create_block();
                        self.block_map.insert(next_ip, fallthrough);
                        builder.ins().brif(is_false, target_block, &[], fallthrough, &[]);
                        self.block_terminated = true;
                    }
                }
            }

            OpCode::JumpIfTrue => {
                let target = instr.operand.ok_or_else(|| {
                    JitError::InvalidBytecode("JumpIfTrue missing operand".into())
                })?;

                let cond = self.pop_stack()?;
                let zero = builder.ins().iconst(types::I64, 0);
                let is_true = builder.ins().icmp(IntCC::NotEqual, cond, zero);

                if let Some(&target_block) = self.block_map.get(&target) {
                    let next_ip = self.current_ip + 1;
                    if let Some(&fallthrough_block) = self.block_map.get(&next_ip) {
                        builder.ins().brif(
                            is_true,
                            target_block,
                            &[],
                            fallthrough_block,
                            &[],
                        );
                        self.block_terminated = true;
                    } else {
                        let fallthrough = builder.create_block();
                        self.block_map.insert(next_ip, fallthrough);
                        builder.ins().brif(is_true, target_block, &[], fallthrough, &[]);
                        self.block_terminated = true;
                    }
                }
            }

            // === Stack Operations ===
            OpCode::Pop => {
                let _ = self.pop_stack()?;
            }

            OpCode::Dup => {
                if let Some(&top) = self.stack.last() {
                    self.stack.push(top);
                } else {
                    return Err(JitError::InvalidBytecode("Dup on empty stack".into()));
                }
            }

            // === Return ===
            OpCode::Return => {
                let ret_val = if !self.stack.is_empty() {
                    self.pop_stack()?
                } else {
                    builder.ins().iconst(types::I64, 0)
                };
                builder.ins().return_(&[ret_val]);
                self.block_terminated = true;
            }

            // === Loop Markers (no-op for JIT) ===
            OpCode::LoopStart | OpCode::LoopIteration | OpCode::LoopEnd => {
                // These are profiling markers, no code needed
            }

            // === No-op ===
            OpCode::Nop | OpCode::Halt => {
                // No code needed
            }

            // === Unsupported opcodes ===
            _ => {
                return Err(JitError::UnsupportedOpcode(instr.opcode.clone()));
            }
        }

        Ok(())
    }

    /// Load a constant from the chunk's constant pool
    fn load_constant(
        &mut self,
        builder: &mut FunctionBuilder,
        idx: usize,
    ) -> Result<cranelift::prelude::Value, JitError> {
        let const_value = self.chunk.constants.get(idx).ok_or_else(|| {
            JitError::InvalidBytecode(format!("Constant index {} out of bounds", idx))
        })?;

        match const_value {
            Value::Integer(i) => Ok(builder.ins().iconst(types::I64, *i)),
            Value::Real(r) => {
                // For reals, we store as i64 bit pattern
                let bits = r.to_bits() as i64;
                Ok(builder.ins().iconst(types::I64, bits))
            }
            Value::Logical(b) => {
                let val = if *b { 1i64 } else { 0i64 };
                Ok(builder.ins().iconst(types::I64, val))
            }
            _ => Err(JitError::InvalidBytecode(format!(
                "Unsupported constant type: {:?}",
                const_value.type_name()
            ))),
        }
    }

    /// Load a variable from the runtime context
    fn load_var_from_context(
        &mut self,
        builder: &mut FunctionBuilder,
        var_idx: usize,
    ) -> Result<cranelift::prelude::Value, JitError> {
        let ptr_type = pointer_type();
        let offset = std::mem::offset_of!(JitRuntimeContext, variables) as i32;

        // Load variables pointer from context
        let vars_ptr = builder.ins().load(
            ptr_type,
            MemFlags::trusted(),
            self.ctx_ptr,
            offset,
        );

        // Calculate offset: var_idx * sizeof(Value)
        // Value is an enum, assuming it's 24 bytes (discriminant + max variant)
        let value_size = std::mem::size_of::<Value>() as i64;
        let byte_offset = builder
            .ins()
            .iconst(types::I64, var_idx as i64 * value_size);

        let elem_ptr = builder.ins().iadd(vars_ptr, byte_offset);

        // Load the value (just the integer portion for now)
        // This is simplified - real implementation would handle Value enum properly
        let value = builder
            .ins()
            .load(types::I64, MemFlags::trusted(), elem_ptr, 8);

        Ok(value)
    }

    /// Store a variable to the runtime context
    fn store_var_to_context(
        &mut self,
        builder: &mut FunctionBuilder,
        var_idx: usize,
        value: cranelift::prelude::Value,
    ) -> Result<(), JitError> {
        let ptr_type = pointer_type();
        let offset = std::mem::offset_of!(JitRuntimeContext, variables) as i32;

        // Load variables pointer from context
        let vars_ptr = builder.ins().load(
            ptr_type,
            MemFlags::trusted(),
            self.ctx_ptr,
            offset,
        );

        // Calculate offset
        let value_size = std::mem::size_of::<Value>() as i64;
        let byte_offset = builder
            .ins()
            .iconst(types::I64, var_idx as i64 * value_size);

        let elem_ptr = builder.ins().iadd(vars_ptr, byte_offset);

        // Store the value
        builder
            .ins()
            .store(MemFlags::trusted(), value, elem_ptr, 8);

        Ok(())
    }

    /// Pop a value from the translation stack
    fn pop_stack(&mut self) -> Result<cranelift::prelude::Value, JitError> {
        self.stack
            .pop()
            .ok_or_else(|| JitError::InvalidBytecode("Stack underflow".into()))
    }
}

#[cfg(test)]
mod tests {
    // Integration tests will be added when we have full VM integration
}
