//! Runtime helper functions for JIT-compiled code
//!
//! These functions are called from native code when the JIT needs to
//! perform operations that are easier to implement in Rust, such as
//! I/O operations, complex math, and intrinsic functions.
//!
//! All functions use the C calling convention for compatibility with
//! Cranelift-generated code.

use std::ffi::c_char;

/// Print an integer value to stdout
///
/// # Safety
/// This function is called from JIT-compiled code with C calling convention
#[no_mangle]
pub extern "C" fn jit_print_integer(value: i64) {
    print!("{}", value);
}

/// Print a real (f64) value to stdout
///
/// # Safety
/// This function is called from JIT-compiled code with C calling convention
#[no_mangle]
pub extern "C" fn jit_print_real(value: f64) {
    // Format similar to Fortran default formatting
    if value.abs() < 0.0001 || value.abs() >= 10000000.0 {
        print!("{:E}", value);
    } else {
        print!("{}", value);
    }
}

/// Print a null-terminated string to stdout
///
/// # Safety
/// This function is called from JIT-compiled code.
/// The `ptr` must be a valid pointer to a null-terminated string.
#[no_mangle]
pub unsafe extern "C" fn jit_print_string(ptr: *const c_char) {
    if ptr.is_null() {
        return;
    }
    let c_str = std::ffi::CStr::from_ptr(ptr);
    if let Ok(s) = c_str.to_str() {
        print!("{}", s);
    }
}

/// Print a newline to stdout
///
/// # Safety
/// This function is called from JIT-compiled code with C calling convention
#[no_mangle]
pub extern "C" fn jit_print_newline() {
    println!();
}

/// Compute integer power: base^exp
///
/// # Safety
/// This function is called from JIT-compiled code with C calling convention
#[no_mangle]
pub extern "C" fn jit_power_int(base: i64, exp: i64) -> i64 {
    if exp < 0 {
        // Integer division by power would truncate to 0 for base > 1
        if base == 1 {
            1
        } else if base == -1 {
            if exp % 2 == 0 {
                1
            } else {
                -1
            }
        } else {
            0
        }
    } else if exp == 0 {
        1
    } else {
        let mut result: i64 = 1;
        let mut b = base;
        let mut e = exp as u64;

        // Fast exponentiation by squaring
        while e > 0 {
            if e & 1 == 1 {
                result = result.wrapping_mul(b);
            }
            e >>= 1;
            b = b.wrapping_mul(b);
        }
        result
    }
}

/// Compute real power: base^exp
///
/// # Safety
/// This function is called from JIT-compiled code with C calling convention
#[no_mangle]
pub extern "C" fn jit_power_real(base: f64, exp: f64) -> f64 {
    base.powf(exp)
}

/// Call an intrinsic function by ID
///
/// Intrinsic IDs correspond to the VM's intrinsic numbering.
/// Returns the result as an i64 (reinterpret for f64 if needed).
///
/// # Safety
/// This function is called from JIT-compiled code with C calling convention
#[no_mangle]
pub extern "C" fn jit_call_intrinsic(intrinsic_id: i64, arg1: i64, arg2: i64) -> i64 {
    // Intrinsic dispatch based on ID
    // For now, implement a subset of commonly-used intrinsics
    match intrinsic_id {
        // ABS (integer)
        0 => arg1.abs(),

        // MOD (integer)
        1 => {
            if arg2 == 0 {
                0 // Avoid division by zero
            } else {
                arg1 % arg2
            }
        }

        // MAX (integer, 2 args)
        2 => std::cmp::max(arg1, arg2),

        // MIN (integer, 2 args)
        3 => std::cmp::min(arg1, arg2),

        // SIGN (integer)
        4 => {
            if arg1 >= 0 {
                arg2.abs()
            } else {
                -arg2.abs()
            }
        }

        // For unknown intrinsics, return 0
        _ => 0,
    }
}

/// Call a real intrinsic function by ID
///
/// # Safety
/// This function is called from JIT-compiled code with C calling convention
#[no_mangle]
pub extern "C" fn jit_call_intrinsic_real(intrinsic_id: i64, arg1: f64, arg2: f64) -> f64 {
    match intrinsic_id {
        // ABS
        0 => arg1.abs(),

        // SQRT
        1 => arg1.sqrt(),

        // SIN
        2 => arg1.sin(),

        // COS
        3 => arg1.cos(),

        // TAN
        4 => arg1.tan(),

        // EXP
        5 => arg1.exp(),

        // LOG
        6 => arg1.ln(),

        // LOG10
        7 => arg1.log10(),

        // MAX
        8 => arg1.max(arg2),

        // MIN
        9 => arg1.min(arg2),

        // FLOOR
        10 => arg1.floor(),

        // CEILING
        11 => arg1.ceil(),

        // MOD
        12 => arg1 % arg2,

        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_power_int() {
        assert_eq!(jit_power_int(2, 0), 1);
        assert_eq!(jit_power_int(2, 1), 2);
        assert_eq!(jit_power_int(2, 10), 1024);
        assert_eq!(jit_power_int(3, 4), 81);
        assert_eq!(jit_power_int(-2, 3), -8);
        assert_eq!(jit_power_int(-2, 4), 16);
        assert_eq!(jit_power_int(5, -1), 0);
        assert_eq!(jit_power_int(1, -5), 1);
        assert_eq!(jit_power_int(-1, -5), -1);
    }

    #[test]
    fn test_power_real() {
        assert!((jit_power_real(2.0, 3.0) - 8.0).abs() < 1e-10);
        assert!((jit_power_real(4.0, 0.5) - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_intrinsics_int() {
        // ABS
        assert_eq!(jit_call_intrinsic(0, -5, 0), 5);
        // MOD
        assert_eq!(jit_call_intrinsic(1, 17, 5), 2);
        // MAX
        assert_eq!(jit_call_intrinsic(2, 3, 7), 7);
        // MIN
        assert_eq!(jit_call_intrinsic(3, 3, 7), 3);
    }

    #[test]
    fn test_intrinsics_real() {
        // ABS
        assert!((jit_call_intrinsic_real(0, -5.5, 0.0) - 5.5).abs() < 1e-10);
        // SQRT
        assert!((jit_call_intrinsic_real(1, 16.0, 0.0) - 4.0).abs() < 1e-10);
    }
}
