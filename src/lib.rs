//! FIRP - Fortran Interpreter
//!
//! A modern Fortran interpreter supporting Fortran 2003+ with bytecode VM execution.

pub mod lexer;
pub mod parser;
pub mod ast;
pub mod semantic;
pub mod bytecode;
pub mod vm;
pub mod runtime;
