//! Debug expression evaluator for conditions and watch expressions
//!
//! This module provides a lightweight expression parser and evaluator for
//! debugger conditions, watch expressions, and variable modifications.

use crate::bytecode::Value;
use std::collections::HashMap;

/// Evaluation context - maps variable names to values
pub type EvalContext = HashMap<String, Value>;

/// Expression AST for debugger
#[derive(Debug, Clone, PartialEq)]
pub enum DebugExpr {
    /// Integer literal
    Integer(i64),
    /// Real (floating-point) literal
    Real(f64),
    /// Logical literal (.TRUE. or .FALSE.)
    Logical(bool),
    /// String literal
    String(String),
    /// Variable reference
    Variable(String),
    /// Array access: arr(index) or arr(i, j)
    ArrayAccess {
        array: String,
        indices: Vec<DebugExpr>,
    },
    /// Derived type member: point%x
    Member {
        object: Box<DebugExpr>,
        field: String,
    },
    /// Binary operation
    BinaryOp {
        left: Box<DebugExpr>,
        op: BinaryOp,
        right: Box<DebugExpr>,
    },
    /// Unary operation
    UnaryOp {
        op: UnaryOp,
        operand: Box<DebugExpr>,
    },
    /// Parenthesized expression
    Grouped(Box<DebugExpr>),
}

/// Binary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    // Comparison
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    // Logical
    And,
    Or,
    Eqv,
    Neqv,
}

/// Unary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
}

/// Parser for debug expressions
pub struct ExprParser<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> ExprParser<'a> {
    /// Create a new parser
    pub fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    /// Parse the full expression
    pub fn parse(&mut self) -> Result<DebugExpr, String> {
        self.skip_whitespace();
        let expr = self.parse_or()?;
        self.skip_whitespace();
        if self.pos < self.input.len() {
            return Err(format!(
                "Unexpected characters at position {}: '{}'",
                self.pos,
                &self.input[self.pos..]
            ));
        }
        Ok(expr)
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() {
            let c = self.input[self.pos..].chars().next().unwrap();
            if c.is_whitespace() {
                self.pos += c.len_utf8();
            } else {
                break;
            }
        }
    }

    fn peek_char(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn peek_str(&self, len: usize) -> &str {
        let end = (self.pos + len).min(self.input.len());
        &self.input[self.pos..end]
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.peek_char()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    fn consume(&mut self, s: &str) -> bool {
        if self.input[self.pos..].to_uppercase().starts_with(&s.to_uppercase()) {
            self.pos += s.len();
            true
        } else {
            false
        }
    }

    /// Parse logical OR: .OR.
    fn parse_or(&mut self) -> Result<DebugExpr, String> {
        let mut left = self.parse_and()?;

        loop {
            self.skip_whitespace();
            if self.consume(".OR.") {
                self.skip_whitespace();
                let right = self.parse_and()?;
                left = DebugExpr::BinaryOp {
                    left: Box::new(left),
                    op: BinaryOp::Or,
                    right: Box::new(right),
                };
            } else if self.consume(".EQV.") {
                self.skip_whitespace();
                let right = self.parse_and()?;
                left = DebugExpr::BinaryOp {
                    left: Box::new(left),
                    op: BinaryOp::Eqv,
                    right: Box::new(right),
                };
            } else if self.consume(".NEQV.") {
                self.skip_whitespace();
                let right = self.parse_and()?;
                left = DebugExpr::BinaryOp {
                    left: Box::new(left),
                    op: BinaryOp::Neqv,
                    right: Box::new(right),
                };
            } else {
                break;
            }
        }

        Ok(left)
    }

    /// Parse logical AND: .AND.
    fn parse_and(&mut self) -> Result<DebugExpr, String> {
        let mut left = self.parse_comparison()?;

        loop {
            self.skip_whitespace();
            if self.consume(".AND.") {
                self.skip_whitespace();
                let right = self.parse_comparison()?;
                left = DebugExpr::BinaryOp {
                    left: Box::new(left),
                    op: BinaryOp::And,
                    right: Box::new(right),
                };
            } else {
                break;
            }
        }

        Ok(left)
    }

    /// Parse comparison operators
    fn parse_comparison(&mut self) -> Result<DebugExpr, String> {
        let left = self.parse_additive()?;
        self.skip_whitespace();

        // Try various comparison operators
        let op = if self.consume("==") || self.consume(".EQ.") {
            Some(BinaryOp::Eq)
        } else if self.consume("/=") || self.consume(".NE.") {
            Some(BinaryOp::Ne)
        } else if self.consume("<=") || self.consume(".LE.") {
            Some(BinaryOp::Le)
        } else if self.consume(">=") || self.consume(".GE.") {
            Some(BinaryOp::Ge)
        } else if self.consume("<") || self.consume(".LT.") {
            Some(BinaryOp::Lt)
        } else if self.consume(">") || self.consume(".GT.") {
            Some(BinaryOp::Gt)
        } else {
            None
        };

        if let Some(op) = op {
            self.skip_whitespace();
            let right = self.parse_additive()?;
            Ok(DebugExpr::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            })
        } else {
            Ok(left)
        }
    }

    /// Parse addition and subtraction
    fn parse_additive(&mut self) -> Result<DebugExpr, String> {
        let mut left = self.parse_multiplicative()?;

        loop {
            self.skip_whitespace();
            let op = if self.peek_char() == Some('+') {
                self.advance();
                Some(BinaryOp::Add)
            } else if self.peek_char() == Some('-') {
                self.advance();
                Some(BinaryOp::Sub)
            } else {
                None
            };

            if let Some(op) = op {
                self.skip_whitespace();
                let right = self.parse_multiplicative()?;
                left = DebugExpr::BinaryOp {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                };
            } else {
                break;
            }
        }

        Ok(left)
    }

    /// Parse multiplication and division
    fn parse_multiplicative(&mut self) -> Result<DebugExpr, String> {
        let mut left = self.parse_power()?;

        loop {
            self.skip_whitespace();
            let op = if self.peek_char() == Some('*') && self.peek_str(2) != "**" {
                self.advance();
                Some(BinaryOp::Mul)
            } else if self.peek_char() == Some('/') && self.peek_str(2) != "/=" {
                self.advance();
                Some(BinaryOp::Div)
            } else {
                None
            };

            if let Some(op) = op {
                self.skip_whitespace();
                let right = self.parse_power()?;
                left = DebugExpr::BinaryOp {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                };
            } else {
                break;
            }
        }

        Ok(left)
    }

    /// Parse power operator **
    fn parse_power(&mut self) -> Result<DebugExpr, String> {
        let left = self.parse_unary()?;
        self.skip_whitespace();

        if self.consume("**") {
            self.skip_whitespace();
            let right = self.parse_power()?; // Right-associative
            Ok(DebugExpr::BinaryOp {
                left: Box::new(left),
                op: BinaryOp::Pow,
                right: Box::new(right),
            })
        } else {
            Ok(left)
        }
    }

    /// Parse unary operators
    fn parse_unary(&mut self) -> Result<DebugExpr, String> {
        self.skip_whitespace();

        if self.peek_char() == Some('-') {
            self.advance();
            self.skip_whitespace();
            let operand = self.parse_unary()?;
            Ok(DebugExpr::UnaryOp {
                op: UnaryOp::Neg,
                operand: Box::new(operand),
            })
        } else if self.peek_char() == Some('+') {
            self.advance();
            self.parse_unary()
        } else if self.consume(".NOT.") {
            self.skip_whitespace();
            let operand = self.parse_unary()?;
            Ok(DebugExpr::UnaryOp {
                op: UnaryOp::Not,
                operand: Box::new(operand),
            })
        } else {
            self.parse_postfix()
        }
    }

    /// Parse postfix operators (member access, array indexing)
    fn parse_postfix(&mut self) -> Result<DebugExpr, String> {
        let mut expr = self.parse_primary()?;

        loop {
            self.skip_whitespace();

            if self.peek_char() == Some('%') {
                // Member access
                self.advance();
                self.skip_whitespace();
                let field = self.parse_identifier()?;
                expr = DebugExpr::Member {
                    object: Box::new(expr),
                    field,
                };
            } else {
                break;
            }
        }

        Ok(expr)
    }

    /// Parse primary expressions
    fn parse_primary(&mut self) -> Result<DebugExpr, String> {
        self.skip_whitespace();

        // Parenthesized expression
        if self.peek_char() == Some('(') {
            self.advance();
            self.skip_whitespace();
            let expr = self.parse_or()?;
            self.skip_whitespace();
            if self.peek_char() != Some(')') {
                return Err("Expected ')'".to_string());
            }
            self.advance();
            return Ok(DebugExpr::Grouped(Box::new(expr)));
        }

        // Logical literals
        if self.consume(".TRUE.") {
            return Ok(DebugExpr::Logical(true));
        }
        if self.consume(".FALSE.") {
            return Ok(DebugExpr::Logical(false));
        }

        // String literal
        if self.peek_char() == Some('\'') || self.peek_char() == Some('"') {
            return self.parse_string_literal();
        }

        // Number (integer or real)
        if let Some(c) = self.peek_char() {
            if c.is_ascii_digit() || c == '.' {
                return self.parse_number();
            }
        }

        // Variable or array access
        if let Some(c) = self.peek_char() {
            if c.is_alphabetic() || c == '_' {
                let name = self.parse_identifier()?;
                self.skip_whitespace();

                // Check for array access
                if self.peek_char() == Some('(') {
                    self.advance();
                    let indices = self.parse_index_list()?;
                    self.skip_whitespace();
                    if self.peek_char() != Some(')') {
                        return Err("Expected ')' in array access".to_string());
                    }
                    self.advance();
                    return Ok(DebugExpr::ArrayAccess {
                        array: name,
                        indices,
                    });
                }

                return Ok(DebugExpr::Variable(name));
            }
        }

        Err(format!(
            "Unexpected token at position {}: '{}'",
            self.pos,
            self.peek_char().unwrap_or(' ')
        ))
    }

    fn parse_identifier(&mut self) -> Result<String, String> {
        let start = self.pos;

        if let Some(c) = self.peek_char() {
            if !c.is_alphabetic() && c != '_' {
                return Err("Expected identifier".to_string());
            }
        } else {
            return Err("Unexpected end of input".to_string());
        }

        while let Some(c) = self.peek_char() {
            if c.is_alphanumeric() || c == '_' {
                self.advance();
            } else {
                break;
            }
        }

        Ok(self.input[start..self.pos].to_uppercase())
    }

    fn parse_number(&mut self) -> Result<DebugExpr, String> {
        let start = self.pos;
        let mut has_dot = false;
        let mut has_exp = false;

        // Leading digits
        while let Some(c) = self.peek_char() {
            if c.is_ascii_digit() {
                self.advance();
            } else {
                break;
            }
        }

        // Decimal point
        if self.peek_char() == Some('.') {
            // Make sure it's not a Fortran operator like .AND.
            let rest = &self.input[self.pos..];
            if !rest[1..].starts_with(|c: char| c.is_alphabetic()) {
                has_dot = true;
                self.advance();

                // Fractional digits
                while let Some(c) = self.peek_char() {
                    if c.is_ascii_digit() {
                        self.advance();
                    } else {
                        break;
                    }
                }
            }
        }

        // Exponent
        if let Some(c) = self.peek_char() {
            if c == 'e' || c == 'E' || c == 'd' || c == 'D' {
                has_exp = true;
                self.advance();

                // Optional sign
                if let Some(c) = self.peek_char() {
                    if c == '+' || c == '-' {
                        self.advance();
                    }
                }

                // Exponent digits
                while let Some(c) = self.peek_char() {
                    if c.is_ascii_digit() {
                        self.advance();
                    } else {
                        break;
                    }
                }
            }
        }

        let num_str = &self.input[start..self.pos];

        if has_dot || has_exp {
            // Parse as real
            let num_str = num_str.replace(['d', 'D'], "e");
            match num_str.parse::<f64>() {
                Ok(n) => Ok(DebugExpr::Real(n)),
                Err(_) => Err(format!("Invalid real number: {}", num_str)),
            }
        } else {
            // Parse as integer
            match num_str.parse::<i64>() {
                Ok(n) => Ok(DebugExpr::Integer(n)),
                Err(_) => Err(format!("Invalid integer: {}", num_str)),
            }
        }
    }

    fn parse_string_literal(&mut self) -> Result<DebugExpr, String> {
        let quote = self.advance().unwrap();
        let mut value = String::new();

        loop {
            match self.peek_char() {
                Some(c) if c == quote => {
                    self.advance();
                    // Check for escaped quote (doubled)
                    if self.peek_char() == Some(quote) {
                        value.push(quote);
                        self.advance();
                    } else {
                        break;
                    }
                }
                Some(c) => {
                    value.push(c);
                    self.advance();
                }
                None => return Err("Unterminated string literal".to_string()),
            }
        }

        Ok(DebugExpr::String(value))
    }

    fn parse_index_list(&mut self) -> Result<Vec<DebugExpr>, String> {
        let mut indices = Vec::new();

        self.skip_whitespace();
        if self.peek_char() == Some(')') {
            return Ok(indices);
        }

        loop {
            self.skip_whitespace();
            let expr = self.parse_or()?;
            indices.push(expr);

            self.skip_whitespace();
            if self.peek_char() == Some(',') {
                self.advance();
            } else {
                break;
            }
        }

        Ok(indices)
    }
}

/// Parse a debug expression string
pub fn parse_expr(input: &str) -> Result<DebugExpr, String> {
    let mut parser = ExprParser::new(input);
    parser.parse()
}

/// Evaluate an expression against a variable context
pub fn evaluate(expr: &DebugExpr, ctx: &EvalContext) -> Result<Value, String> {
    match expr {
        DebugExpr::Integer(n) => Ok(Value::Integer(*n)),
        DebugExpr::Real(n) => Ok(Value::Real(*n)),
        DebugExpr::Logical(b) => Ok(Value::Logical(*b)),
        DebugExpr::String(s) => Ok(Value::Character(s.clone())),

        DebugExpr::Variable(name) => {
            ctx.get(&name.to_uppercase())
                .cloned()
                .ok_or_else(|| format!("Unknown variable: {}", name))
        }

        DebugExpr::ArrayAccess { array, indices } => {
            let arr_value = ctx.get(&array.to_uppercase())
                .ok_or_else(|| format!("Unknown array: {}", array))?;

            match arr_value {
                Value::Array { elements, dims } => {
                    // Evaluate indices
                    let mut idx_values = Vec::new();
                    for idx_expr in indices {
                        let idx = evaluate(idx_expr, ctx)?;
                        match idx {
                            Value::Integer(i) => idx_values.push(i),
                            _ => return Err("Array index must be integer".to_string()),
                        }
                    }

                    if idx_values.len() != dims.len() {
                        return Err(format!(
                            "Wrong number of indices: expected {}, got {}",
                            dims.len(),
                            idx_values.len()
                        ));
                    }

                    // Calculate linear index
                    let mut linear_idx = 0usize;
                    let mut stride = 1usize;
                    for (i, (idx, dim)) in idx_values.iter().zip(dims.iter()).enumerate() {
                        let lb = dim.lower;
                        let ub = dim.upper;
                        if *idx < lb as i64 || *idx > ub as i64 {
                            return Err(format!(
                                "Array index {} out of bounds ({}:{})",
                                idx, lb, ub
                            ));
                        }
                        linear_idx += ((idx - lb as i64) as usize) * stride;
                        if i + 1 < dims.len() {
                            stride *= dim.size();
                        }
                    }

                    elements.get(linear_idx)
                        .cloned()
                        .ok_or_else(|| "Array index out of bounds".to_string())
                }
                _ => Err(format!("{} is not an array", array)),
            }
        }

        DebugExpr::Member { object, field } => {
            let obj_value = evaluate(object, ctx)?;
            match obj_value {
                Value::Instance { .. } => {
                    // For now, we can't easily look up component names without type registry
                    // Return error for this MVP
                    Err(format!("Member access to '{}' not yet supported in debugger", field))
                }
                _ => Err("Member access on non-instance value".to_string()),
            }
        }

        DebugExpr::BinaryOp { left, op, right } => {
            let l = evaluate(left, ctx)?;
            let r = evaluate(right, ctx)?;
            apply_binary_op(&l, *op, &r)
        }

        DebugExpr::UnaryOp { op, operand } => {
            let v = evaluate(operand, ctx)?;
            apply_unary_op(*op, &v)
        }

        DebugExpr::Grouped(inner) => evaluate(inner, ctx),
    }
}

fn apply_binary_op(left: &Value, op: BinaryOp, right: &Value) -> Result<Value, String> {
    match op {
        BinaryOp::Add => binary_arithmetic(left, right, |a, b| a + b, |a, b| a + b),
        BinaryOp::Sub => binary_arithmetic(left, right, |a, b| a - b, |a, b| a - b),
        BinaryOp::Mul => binary_arithmetic(left, right, |a, b| a * b, |a, b| a * b),
        BinaryOp::Div => {
            // Check for division by zero
            match right {
                Value::Integer(0) => return Err("Division by zero".to_string()),
                Value::Real(r) if *r == 0.0 => return Err("Division by zero".to_string()),
                _ => {}
            }
            binary_arithmetic(left, right, |a, b| a / b, |a, b| a / b)
        }
        BinaryOp::Pow => {
            match (left, right) {
                (Value::Integer(a), Value::Integer(b)) => {
                    if *b >= 0 {
                        Ok(Value::Integer(a.pow(*b as u32)))
                    } else {
                        Ok(Value::Real((*a as f64).powi(*b as i32)))
                    }
                }
                (Value::Integer(a), Value::Real(b)) => {
                    Ok(Value::Real((*a as f64).powf(*b)))
                }
                (Value::Real(a), Value::Integer(b)) => {
                    Ok(Value::Real(a.powi(*b as i32)))
                }
                (Value::Real(a), Value::Real(b)) => {
                    Ok(Value::Real(a.powf(*b)))
                }
                _ => Err(format!("Cannot apply ** to {} and {}", left.type_name(), right.type_name())),
            }
        }

        // Comparison operators
        BinaryOp::Eq => binary_comparison(left, right, |a, b| a == b, |a, b| a == b, |a, b| a == b, |a, b| a == b),
        BinaryOp::Ne => binary_comparison(left, right, |a, b| a != b, |a, b| a != b, |a, b| a != b, |a, b| a != b),
        BinaryOp::Lt => binary_comparison(left, right, |a, b| a < b, |a, b| a < b, |_a, _b| false, |a, b| a < b),
        BinaryOp::Le => binary_comparison(left, right, |a, b| a <= b, |a, b| a <= b, |_a, _b| false, |a, b| a <= b),
        BinaryOp::Gt => binary_comparison(left, right, |a, b| a > b, |a, b| a > b, |_a, _b| false, |a, b| a > b),
        BinaryOp::Ge => binary_comparison(left, right, |a, b| a >= b, |a, b| a >= b, |_a, _b| false, |a, b| a >= b),

        // Logical operators
        BinaryOp::And => {
            match (left, right) {
                (Value::Logical(a), Value::Logical(b)) => Ok(Value::Logical(*a && *b)),
                _ => Err(format!("Cannot apply .AND. to {} and {}", left.type_name(), right.type_name())),
            }
        }
        BinaryOp::Or => {
            match (left, right) {
                (Value::Logical(a), Value::Logical(b)) => Ok(Value::Logical(*a || *b)),
                _ => Err(format!("Cannot apply .OR. to {} and {}", left.type_name(), right.type_name())),
            }
        }
        BinaryOp::Eqv => {
            match (left, right) {
                (Value::Logical(a), Value::Logical(b)) => Ok(Value::Logical(*a == *b)),
                _ => Err(format!("Cannot apply .EQV. to {} and {}", left.type_name(), right.type_name())),
            }
        }
        BinaryOp::Neqv => {
            match (left, right) {
                (Value::Logical(a), Value::Logical(b)) => Ok(Value::Logical(*a != *b)),
                _ => Err(format!("Cannot apply .NEQV. to {} and {}", left.type_name(), right.type_name())),
            }
        }
    }
}

fn binary_arithmetic<F, G>(left: &Value, right: &Value, int_op: F, real_op: G) -> Result<Value, String>
where
    F: Fn(i64, i64) -> i64,
    G: Fn(f64, f64) -> f64,
{
    match (left, right) {
        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(int_op(*a, *b))),
        (Value::Integer(a), Value::Real(b)) => Ok(Value::Real(real_op(*a as f64, *b))),
        (Value::Real(a), Value::Integer(b)) => Ok(Value::Real(real_op(*a, *b as f64))),
        (Value::Real(a), Value::Real(b)) => Ok(Value::Real(real_op(*a, *b))),
        _ => Err(format!("Cannot perform arithmetic on {} and {}", left.type_name(), right.type_name())),
    }
}

fn binary_comparison<F, G, H, I>(
    left: &Value,
    right: &Value,
    int_op: F,
    real_op: G,
    logical_op: H,
    str_op: I,
) -> Result<Value, String>
where
    F: Fn(i64, i64) -> bool,
    G: Fn(f64, f64) -> bool,
    H: Fn(bool, bool) -> bool,
    I: Fn(&str, &str) -> bool,
{
    match (left, right) {
        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Logical(int_op(*a, *b))),
        (Value::Integer(a), Value::Real(b)) => Ok(Value::Logical(real_op(*a as f64, *b))),
        (Value::Real(a), Value::Integer(b)) => Ok(Value::Logical(real_op(*a, *b as f64))),
        (Value::Real(a), Value::Real(b)) => Ok(Value::Logical(real_op(*a, *b))),
        (Value::Logical(a), Value::Logical(b)) => Ok(Value::Logical(logical_op(*a, *b))),
        (Value::Character(a), Value::Character(b)) => Ok(Value::Logical(str_op(a, b))),
        _ => Err(format!("Cannot compare {} and {}", left.type_name(), right.type_name())),
    }
}

fn apply_unary_op(op: UnaryOp, value: &Value) -> Result<Value, String> {
    match op {
        UnaryOp::Neg => {
            match value {
                Value::Integer(n) => Ok(Value::Integer(-n)),
                Value::Real(n) => Ok(Value::Real(-n)),
                _ => Err(format!("Cannot negate {}", value.type_name())),
            }
        }
        UnaryOp::Not => {
            match value {
                Value::Logical(b) => Ok(Value::Logical(!b)),
                _ => Err(format!("Cannot apply .NOT. to {}", value.type_name())),
            }
        }
    }
}

/// Evaluate a condition string (returns bool)
pub fn evaluate_condition(condition: &str, ctx: &EvalContext) -> Result<bool, String> {
    let expr = parse_expr(condition)?;
    let value = evaluate(&expr, ctx)?;

    match value {
        Value::Logical(b) => Ok(b),
        Value::Integer(n) => Ok(n != 0),
        Value::Real(n) => Ok(n != 0.0),
        _ => Err(format!("Condition evaluated to {} (expected logical)", value.type_name())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ctx() -> EvalContext {
        let mut ctx = EvalContext::new();
        ctx.insert("X".to_string(), Value::Integer(10));
        ctx.insert("Y".to_string(), Value::Integer(5));
        ctx.insert("R".to_string(), Value::Real(3.14));
        ctx.insert("FLAG".to_string(), Value::Logical(true));
        ctx.insert("NAME".to_string(), Value::Character("test".to_string()));
        ctx
    }

    #[test]
    fn test_parse_integer() {
        let expr = parse_expr("42").unwrap();
        assert_eq!(expr, DebugExpr::Integer(42));
    }

    #[test]
    fn test_parse_real() {
        let expr = parse_expr("3.14").unwrap();
        assert_eq!(expr, DebugExpr::Real(3.14));
    }

    #[test]
    fn test_parse_logical() {
        assert_eq!(parse_expr(".TRUE.").unwrap(), DebugExpr::Logical(true));
        assert_eq!(parse_expr(".FALSE.").unwrap(), DebugExpr::Logical(false));
    }

    #[test]
    fn test_parse_variable() {
        let expr = parse_expr("myVar").unwrap();
        assert_eq!(expr, DebugExpr::Variable("MYVAR".to_string()));
    }

    #[test]
    fn test_parse_arithmetic() {
        let expr = parse_expr("x + y * 2").unwrap();
        // Should parse as x + (y * 2) due to precedence
        match expr {
            DebugExpr::BinaryOp { op: BinaryOp::Add, .. } => {}
            _ => panic!("Expected addition at top level"),
        }
    }

    #[test]
    fn test_parse_comparison() {
        let expr = parse_expr("x > 5").unwrap();
        match expr {
            DebugExpr::BinaryOp { op: BinaryOp::Gt, .. } => {}
            _ => panic!("Expected greater-than"),
        }
    }

    #[test]
    fn test_parse_logical_ops() {
        let expr = parse_expr("x > 0 .AND. y < 10").unwrap();
        match expr {
            DebugExpr::BinaryOp { op: BinaryOp::And, .. } => {}
            _ => panic!("Expected AND"),
        }
    }

    #[test]
    fn test_evaluate_literal() {
        let ctx = EvalContext::new();
        assert_eq!(evaluate(&parse_expr("42").unwrap(), &ctx).unwrap(), Value::Integer(42));
        assert_eq!(evaluate(&parse_expr("3.14").unwrap(), &ctx).unwrap(), Value::Real(3.14));
    }

    #[test]
    fn test_evaluate_variable() {
        let ctx = make_ctx();
        assert_eq!(evaluate(&parse_expr("x").unwrap(), &ctx).unwrap(), Value::Integer(10));
        assert_eq!(evaluate(&parse_expr("Y").unwrap(), &ctx).unwrap(), Value::Integer(5));
    }

    #[test]
    fn test_evaluate_arithmetic() {
        let ctx = make_ctx();
        assert_eq!(
            evaluate(&parse_expr("x + y").unwrap(), &ctx).unwrap(),
            Value::Integer(15)
        );
        assert_eq!(
            evaluate(&parse_expr("x * 2").unwrap(), &ctx).unwrap(),
            Value::Integer(20)
        );
        assert_eq!(
            evaluate(&parse_expr("x - y").unwrap(), &ctx).unwrap(),
            Value::Integer(5)
        );
    }

    #[test]
    fn test_evaluate_comparison() {
        let ctx = make_ctx();
        assert_eq!(
            evaluate(&parse_expr("x > 5").unwrap(), &ctx).unwrap(),
            Value::Logical(true)
        );
        assert_eq!(
            evaluate(&parse_expr("x < 5").unwrap(), &ctx).unwrap(),
            Value::Logical(false)
        );
        assert_eq!(
            evaluate(&parse_expr("x == 10").unwrap(), &ctx).unwrap(),
            Value::Logical(true)
        );
    }

    #[test]
    fn test_evaluate_logical() {
        let ctx = make_ctx();
        assert_eq!(
            evaluate(&parse_expr("x > 0 .AND. y > 0").unwrap(), &ctx).unwrap(),
            Value::Logical(true)
        );
        assert_eq!(
            evaluate(&parse_expr("x < 0 .OR. y > 0").unwrap(), &ctx).unwrap(),
            Value::Logical(true)
        );
        assert_eq!(
            evaluate(&parse_expr(".NOT. flag").unwrap(), &ctx).unwrap(),
            Value::Logical(false)
        );
    }

    #[test]
    fn test_evaluate_condition() {
        let ctx = make_ctx();
        assert!(evaluate_condition("x > 5", &ctx).unwrap());
        assert!(!evaluate_condition("x < 5", &ctx).unwrap());
        assert!(evaluate_condition("x > 0 .AND. y > 0", &ctx).unwrap());
    }

    #[test]
    fn test_parse_array_access() {
        let expr = parse_expr("arr(5)").unwrap();
        match expr {
            DebugExpr::ArrayAccess { array, indices } => {
                assert_eq!(array, "ARR");
                assert_eq!(indices.len(), 1);
            }
            _ => panic!("Expected array access"),
        }
    }

    #[test]
    fn test_unary_negation() {
        let ctx = make_ctx();
        assert_eq!(
            evaluate(&parse_expr("-x").unwrap(), &ctx).unwrap(),
            Value::Integer(-10)
        );
    }

    #[test]
    fn test_power_operator() {
        let ctx = make_ctx();
        assert_eq!(
            evaluate(&parse_expr("2 ** 3").unwrap(), &ctx).unwrap(),
            Value::Integer(8)
        );
    }

    #[test]
    fn test_parenthesized_expression() {
        let ctx = make_ctx();
        assert_eq!(
            evaluate(&parse_expr("(x + y) * 2").unwrap(), &ctx).unwrap(),
            Value::Integer(30)
        );
    }
}
