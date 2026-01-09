//! Variable inspection for the interactive debugger
//!
//! This module provides utilities for inspecting and formatting
//! variable values during debugging.

use crate::bytecode::Value;

/// Maximum number of array elements to display
const MAX_ARRAY_DISPLAY: usize = 20;

/// Formats a Value for display in the debugger
pub struct ValueFormatter;

impl ValueFormatter {
    /// Format a value for display
    pub fn format(value: &Value) -> String {
        Self::format_with_indent(value, 0)
    }

    /// Format a value with indentation
    pub fn format_with_indent(value: &Value, indent: usize) -> String {
        match value {
            Value::Integer(n) => format!("{}", n),
            Value::Real(r) => {
                // Format real numbers nicely
                if r.abs() < 0.0001 || r.abs() >= 1e6 {
                    format!("{:E}", r)
                } else {
                    format!("{}", r)
                }
            }
            Value::Logical(b) => {
                if *b { ".TRUE.".to_string() } else { ".FALSE.".to_string() }
            }
            Value::Character(s) => format!("'{}'", s),
            Value::Array { elements, dims } => {
                Self::format_array(elements, dims, indent)
            }
            Value::Instance { type_index, components } => {
                Self::format_instance(*type_index, components, indent)
            }
            Value::Null => "NULL()".to_string(),
            Value::Reference(idx) => format!("<ref:{}>", idx),
        }
    }

    /// Format an array value
    fn format_array(elements: &[Value], dims: &[crate::bytecode::ArrayDim], indent: usize) -> String {
        let mut result = String::new();

        // Show dimensions
        let dim_str: Vec<String> = dims.iter()
            .map(|d| format!("{}:{}", d.lower, d.upper))
            .collect();
        result.push_str(&format!("ARRAY({})", dim_str.join(", ")));

        if elements.is_empty() {
            return result;
        }

        // Show elements
        result.push_str(" = [");

        let show_count = elements.len().min(MAX_ARRAY_DISPLAY);
        let elem_strs: Vec<String> = elements[..show_count]
            .iter()
            .map(|v| Self::format_with_indent(v, indent + 1))
            .collect();

        if elements.len() <= 10 {
            // Single line for small arrays
            result.push_str(&elem_strs.join(", "));
        } else {
            // Multi-line for larger arrays
            result.push('\n');
            let prefix = "  ".repeat(indent + 1);
            for (i, elem_str) in elem_strs.iter().enumerate() {
                result.push_str(&format!("{}  [{}] = {}\n", prefix, i + 1, elem_str));
            }
            result.push_str(&"  ".repeat(indent));
        }

        if elements.len() > MAX_ARRAY_DISPLAY {
            result.push_str(&format!("... ({} more elements)", elements.len() - MAX_ARRAY_DISPLAY));
        }

        result.push(']');
        result
    }

    /// Format an instance (derived type) value
    fn format_instance(type_index: usize, components: &[Value], indent: usize) -> String {
        let mut result = format!("<TYPE#{}>", type_index);
        result.push_str(" {\n");

        let prefix = "  ".repeat(indent + 1);
        for (i, comp) in components.iter().enumerate() {
            result.push_str(&format!("{}  component_{}: {}\n",
                prefix, i + 1, Self::format_with_indent(comp, indent + 1)));
        }

        result.push_str(&format!("{}}}", "  ".repeat(indent)));
        result
    }

    /// Get type name for a value
    pub fn type_name(value: &Value) -> &'static str {
        value.type_name()
    }

    /// Format a value with type information
    pub fn format_with_type(value: &Value) -> String {
        format!("{}: {} = {}",
            Self::type_name(value),
            match value {
                Value::Array { dims, .. } => {
                    let dim_str: Vec<String> = dims.iter()
                        .map(|d| format!("{}", d.size()))
                        .collect();
                    format!("({})", dim_str.join(","))
                }
                _ => String::new(),
            },
            Self::format(value)
        )
    }
}

/// Information about a variable for debugging
#[derive(Debug, Clone)]
pub struct VariableInfo {
    /// Variable name
    pub name: String,
    /// Variable slot index
    pub slot: usize,
    /// Current value (if available)
    pub value: Option<Value>,
}

impl VariableInfo {
    /// Format for display
    pub fn display(&self) -> String {
        match &self.value {
            Some(val) => format!("  {}: {} = {}",
                self.name,
                ValueFormatter::type_name(val),
                ValueFormatter::format(val)),
            None => format!("  {}: <uninitialized>", self.name),
        }
    }
}

/// Extracts debug information from VM state
pub struct DebugInspector;

impl DebugInspector {
    /// Format a list of variables for display
    pub fn format_variables(vars: &[(String, Option<Value>)]) -> String {
        if vars.is_empty() {
            return "  <no variables>".to_string();
        }

        let mut result = String::new();
        for (name, value) in vars {
            match value {
                Some(val) => {
                    result.push_str(&format!("  {}: {} = {}\n",
                        name,
                        ValueFormatter::type_name(val),
                        ValueFormatter::format(val)));
                }
                None => {
                    result.push_str(&format!("  {}: <uninitialized>\n", name));
                }
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::ArrayDim;

    #[test]
    fn test_format_integer() {
        let val = Value::Integer(42);
        assert_eq!(ValueFormatter::format(&val), "42");
    }

    #[test]
    fn test_format_real() {
        let val = Value::Real(3.14159);
        assert!(ValueFormatter::format(&val).contains("3.14"));
    }

    #[test]
    fn test_format_logical() {
        assert_eq!(ValueFormatter::format(&Value::Logical(true)), ".TRUE.");
        assert_eq!(ValueFormatter::format(&Value::Logical(false)), ".FALSE.");
    }

    #[test]
    fn test_format_character() {
        let val = Value::Character("hello".to_string());
        assert_eq!(ValueFormatter::format(&val), "'hello'");
    }

    #[test]
    fn test_format_array() {
        let val = Value::Array {
            elements: vec![Value::Integer(1), Value::Integer(2), Value::Integer(3)],
            dims: vec![ArrayDim { lower: 1, upper: 3 }],
        };
        let formatted = ValueFormatter::format(&val);
        assert!(formatted.contains("ARRAY"));
        assert!(formatted.contains("1:3"));
    }

    #[test]
    fn test_format_null() {
        assert_eq!(ValueFormatter::format(&Value::Null), "NULL()");
    }

    #[test]
    fn test_format_reference() {
        let val = Value::Reference(5);
        assert_eq!(ValueFormatter::format(&val), "<ref:5>");
    }

    #[test]
    fn test_type_name() {
        assert_eq!(ValueFormatter::type_name(&Value::Integer(0)), "INTEGER");
        assert_eq!(ValueFormatter::type_name(&Value::Real(0.0)), "REAL");
        assert_eq!(ValueFormatter::type_name(&Value::Logical(true)), "LOGICAL");
        assert_eq!(ValueFormatter::type_name(&Value::Character(String::new())), "CHARACTER");
    }
}
