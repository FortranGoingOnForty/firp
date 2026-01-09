//! Cranelift context and module initialization
//!
//! This module handles setting up the Cranelift JIT infrastructure
//! with appropriate settings for the target platform.

use super::runtime;
use super::JitError;
use cranelift::prelude::*;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{default_libcall_names, Module};

/// Create and configure a new JIT module
///
/// This sets up:
/// - Native target detection (x86_64 or aarch64)
/// - Optimization settings
/// - Runtime helper function registration
pub fn create_jit_module() -> Result<(JITModule, codegen::Context), JitError> {
    // Configure code generation flags
    let mut flag_builder = settings::builder();

    // Enable speed optimizations
    flag_builder
        .set("opt_level", "speed")
        .map_err(|e| JitError::CraneliftError(e.to_string()))?;

    // PIC mode depends on architecture
    // On x86_64: use PIC for better code generation
    // On ARM64: don't use PIC (PLT not supported yet in Cranelift JIT)
    #[cfg(target_arch = "x86_64")]
    {
        flag_builder
            .set("is_pic", "true")
            .map_err(|e| JitError::CraneliftError(e.to_string()))?;
    }

    // Detect native ISA (x86_64, aarch64, etc.)
    let isa_builder = cranelift_native::builder()
        .map_err(|e| JitError::CraneliftError(format!("Failed to detect native target: {}", e)))?;

    let isa = isa_builder
        .finish(settings::Flags::new(flag_builder))
        .map_err(|e| JitError::CraneliftError(e.to_string()))?;

    // Create JIT builder with the configured ISA
    let mut builder = JITBuilder::with_isa(isa, default_libcall_names());

    // Register runtime helper functions that JIT code can call
    register_runtime_symbols(&mut builder);

    // Create the JIT module
    let module = JITModule::new(builder);

    // Create a reusable codegen context
    let codegen_context = module.make_context();

    Ok((module, codegen_context))
}

/// Register runtime helper functions as symbols the JIT can call
fn register_runtime_symbols(builder: &mut JITBuilder) {
    // Integer printing
    builder.symbol("jit_print_integer", runtime::jit_print_integer as *const u8);

    // Real (floating point) printing
    builder.symbol("jit_print_real", runtime::jit_print_real as *const u8);

    // String printing
    builder.symbol("jit_print_string", runtime::jit_print_string as *const u8);

    // Newline printing
    builder.symbol("jit_print_newline", runtime::jit_print_newline as *const u8);

    // Integer power operation
    builder.symbol("jit_power_int", runtime::jit_power_int as *const u8);

    // Real power operation
    builder.symbol("jit_power_real", runtime::jit_power_real as *const u8);

    // Intrinsic function dispatcher
    builder.symbol(
        "jit_call_intrinsic",
        runtime::jit_call_intrinsic as *const u8,
    );
}

/// Get the pointer type for the current platform
pub fn pointer_type() -> types::Type {
    if cfg!(target_pointer_width = "64") {
        types::I64
    } else {
        types::I32
    }
}

/// Get the calling convention for the current platform
pub fn call_conv() -> isa::CallConv {
    if cfg!(target_os = "windows") {
        isa::CallConv::WindowsFastcall
    } else {
        isa::CallConv::SystemV
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_jit_module() {
        let result = create_jit_module();
        assert!(result.is_ok(), "Failed to create JIT module: {:?}", result.err());
    }

    #[test]
    fn test_pointer_type() {
        let ptr_type = pointer_type();
        // On 64-bit systems, pointer type should be I64
        #[cfg(target_pointer_width = "64")]
        assert_eq!(ptr_type, types::I64);
        #[cfg(target_pointer_width = "32")]
        assert_eq!(ptr_type, types::I32);
    }
}
