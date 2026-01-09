//! JIT compilation for FIRP bytecode
//!
//! This module provides Just-In-Time compilation capability using Cranelift
//! as the code generation backend. It compiles hot procedures and loops
//! to native code for improved performance.

pub mod context;
pub mod translator;
pub mod runtime;

use crate::bytecode::{Chunk, OpCode, Value};
use cranelift_jit::JITModule;
use std::collections::HashMap;

/// Configuration for JIT compilation
#[derive(Debug, Clone)]
pub struct JitConfig {
    /// Number of calls before compiling a procedure
    pub call_threshold: usize,
    /// Number of iterations before compiling a loop
    pub loop_threshold: usize,
    /// Enable debug info in generated code
    pub debug_info: bool,
}

impl Default for JitConfig {
    fn default() -> Self {
        Self {
            call_threshold: 100,
            loop_threshold: 1000,
            debug_info: false,
        }
    }
}

/// Compiled native function pointer
/// Takes a pointer to JitRuntimeContext, returns error code (0 = success)
pub type CompiledFn = unsafe extern "C" fn(*mut JitRuntimeContext) -> i64;

/// Runtime context passed to JIT-compiled code
/// This struct is shared between Rust and native code, so it must be #[repr(C)]
#[repr(C)]
pub struct JitRuntimeContext {
    /// Pointer to variables array
    pub variables: *mut Value,
    /// Number of variables
    pub var_count: usize,
    /// Stack pointer (for evaluation stack)
    pub stack: *mut Value,
    /// Stack top index
    pub stack_top: usize,
    /// Stack capacity
    pub stack_capacity: usize,
    /// Constants pool pointer
    pub constants: *const Value,
    /// Constants count
    pub const_count: usize,
    /// Error flag (0 = success, nonzero = error code)
    pub error: i64,
    /// Return value storage
    pub return_value: i64,
}

impl Default for JitRuntimeContext {
    fn default() -> Self {
        Self {
            variables: std::ptr::null_mut(),
            var_count: 0,
            stack: std::ptr::null_mut(),
            stack_top: 0,
            stack_capacity: 0,
            constants: std::ptr::null(),
            const_count: 0,
            error: 0,
            return_value: 0,
        }
    }
}

/// JIT compilation errors
#[derive(Debug)]
pub enum JitError {
    /// Cranelift compilation failed
    CompilationFailed(String),
    /// Opcode not supported by JIT
    UnsupportedOpcode(OpCode),
    /// Invalid bytecode structure
    InvalidBytecode(String),
    /// Cranelift internal error
    CraneliftError(String),
    /// Module finalization error
    ModuleError(String),
}

impl std::fmt::Display for JitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JitError::CompilationFailed(msg) => write!(f, "JIT compilation failed: {}", msg),
            JitError::UnsupportedOpcode(op) => write!(f, "Unsupported opcode for JIT: {:?}", op),
            JitError::InvalidBytecode(msg) => write!(f, "Invalid bytecode: {}", msg),
            JitError::CraneliftError(msg) => write!(f, "Cranelift error: {}", msg),
            JitError::ModuleError(msg) => write!(f, "Module error: {}", msg),
        }
    }
}

impl std::error::Error for JitError {}

/// JIT compiler managing compilation and caching
pub struct JitCompiler {
    /// Cranelift JIT module
    module: JITModule,
    /// Compiled procedure cache: name -> function pointer
    cache: HashMap<String, CompiledFn>,
    /// Call counts for hotspot detection
    call_counts: HashMap<String, usize>,
    /// Loop iteration counts (keyed by bytecode address)
    loop_counts: HashMap<usize, usize>,
    /// Configuration
    config: JitConfig,
    /// Function builder context (reusable)
    builder_context: cranelift_frontend::FunctionBuilderContext,
    /// Codegen context (reusable)
    codegen_context: cranelift_codegen::Context,
}

impl JitCompiler {
    /// Create a new JIT compiler with the given configuration
    pub fn new(config: JitConfig) -> Result<Self, JitError> {
        let (module, codegen_context) = context::create_jit_module()?;

        Ok(Self {
            module,
            cache: HashMap::new(),
            call_counts: HashMap::new(),
            loop_counts: HashMap::new(),
            config,
            builder_context: cranelift_frontend::FunctionBuilderContext::new(),
            codegen_context,
        })
    }

    /// Record a procedure call, returns true if should compile now
    pub fn record_call(&mut self, name: &str) -> bool {
        let count = self.call_counts.entry(name.to_string()).or_insert(0);
        *count += 1;
        *count == self.config.call_threshold
    }

    /// Record loop iteration, returns true if should compile now
    pub fn record_loop(&mut self, addr: usize) -> bool {
        let count = self.loop_counts.entry(addr).or_insert(0);
        *count += 1;
        *count == self.config.loop_threshold
    }

    /// Get a compiled function if available
    pub fn get_compiled(&self, name: &str) -> Option<CompiledFn> {
        self.cache.get(name).copied()
    }

    /// Check if a procedure is compiled
    pub fn is_compiled(&self, name: &str) -> bool {
        self.cache.contains_key(name)
    }

    /// Compile a procedure to native code
    pub fn compile_procedure(
        &mut self,
        name: &str,
        entry_ip: usize,
        end_ip: usize,
        chunk: &Chunk,
    ) -> Result<CompiledFn, JitError> {
        // Check if already compiled
        if let Some(func) = self.cache.get(name) {
            return Ok(*func);
        }

        // Create the translator and compile
        let func_ptr = translator::translate_procedure(
            &mut self.module,
            &mut self.builder_context,
            &mut self.codegen_context,
            name,
            entry_ip,
            end_ip,
            chunk,
            self.config.debug_info,
        )?;

        // Cache the result
        self.cache.insert(name.to_string(), func_ptr);

        Ok(func_ptr)
    }

    /// Get statistics about the JIT compiler
    pub fn stats(&self) -> JitStats {
        JitStats {
            compiled_procedures: self.cache.len(),
            total_calls_tracked: self.call_counts.values().sum(),
            total_loops_tracked: self.loop_counts.values().sum(),
        }
    }
}

/// JIT compiler statistics
#[derive(Debug, Clone)]
pub struct JitStats {
    pub compiled_procedures: usize,
    pub total_calls_tracked: usize,
    pub total_loops_tracked: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jit_config_default() {
        let config = JitConfig::default();
        assert_eq!(config.call_threshold, 100);
        assert_eq!(config.loop_threshold, 1000);
        assert!(!config.debug_info);
    }

    #[test]
    fn test_call_counting() {
        let config = JitConfig {
            call_threshold: 3,
            ..Default::default()
        };
        let mut compiler = JitCompiler::new(config).expect("Failed to create JIT compiler");

        assert!(!compiler.record_call("test_proc"));
        assert!(!compiler.record_call("test_proc"));
        assert!(compiler.record_call("test_proc")); // 3rd call triggers
        assert!(!compiler.record_call("test_proc")); // Already past threshold
    }

    #[test]
    fn test_loop_counting() {
        let config = JitConfig {
            loop_threshold: 5,
            ..Default::default()
        };
        let mut compiler = JitCompiler::new(config).expect("Failed to create JIT compiler");

        for _ in 0..4 {
            assert!(!compiler.record_loop(100));
        }
        assert!(compiler.record_loop(100)); // 5th iteration triggers
    }
}
