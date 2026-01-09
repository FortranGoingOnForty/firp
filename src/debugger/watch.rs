//! Watch expression management
//!
//! This module provides watch expressions that trigger when their value changes.

use super::eval::{DebugExpr, EvalContext, evaluate, evaluate_condition, parse_expr};
use crate::bytecode::Value;

/// A watch expression definition
#[derive(Debug, Clone)]
pub struct WatchExpression {
    /// Unique watch identifier
    pub id: usize,
    /// The expression string (for display)
    pub expression: String,
    /// Parsed expression AST
    parsed: DebugExpr,
    /// Optional condition: only trigger if this is true when value changes
    pub condition: Option<String>,
    /// Last evaluated value
    pub last_value: Option<Value>,
    /// Whether this watch is enabled
    pub enabled: bool,
}

impl WatchExpression {
    /// Create a new watch expression
    pub fn new(id: usize, expression: String, parsed: DebugExpr) -> Self {
        Self {
            id,
            expression,
            parsed,
            condition: None,
            last_value: None,
            enabled: true,
        }
    }

    /// Add a condition to this watch
    pub fn with_condition(mut self, condition: String) -> Self {
        self.condition = Some(condition);
        self
    }

    /// Evaluate the current value
    pub fn evaluate(&self, ctx: &EvalContext) -> Option<Value> {
        evaluate(&self.parsed, ctx).ok()
    }

    /// Check if value has changed from last evaluation
    pub fn has_changed(&self, current: &Option<Value>) -> bool {
        self.last_value != *current
    }

    /// Update the stored value
    pub fn update_value(&mut self, new_value: Option<Value>) {
        self.last_value = new_value;
    }

    /// Check if condition is met (if any)
    pub fn condition_met(&self, ctx: &EvalContext) -> bool {
        match &self.condition {
            Some(cond) => evaluate_condition(cond, ctx).unwrap_or(false),
            None => true, // No condition = always trigger
        }
    }

    /// Format for display
    pub fn display(&self, formatter: &dyn Fn(&Value) -> String) -> String {
        let status = if self.enabled { "*" } else { " " };
        let val = self.last_value.as_ref()
            .map(formatter)
            .unwrap_or_else(|| "<unknown>".to_string());
        let cond = self.condition.as_ref()
            .map(|c| format!(" if {}", c))
            .unwrap_or_default();

        format!("{} {}: {} = {}{}", status, self.id, self.expression, val, cond)
    }
}

/// Manages watch expressions during debugging
#[derive(Debug, Default)]
pub struct WatchManager {
    /// All watch expressions
    watches: Vec<WatchExpression>,
    /// Next watch ID to assign
    next_id: usize,
}

/// Result of checking watches
pub struct WatchTrigger {
    /// ID of the watch that triggered
    pub id: usize,
    /// Old value before change
    pub old_value: Option<Value>,
    /// New value after change
    pub new_value: Option<Value>,
}

impl WatchManager {
    /// Create a new watch manager
    pub fn new() -> Self {
        Self {
            watches: Vec::new(),
            next_id: 1,
        }
    }

    /// Add a watch expression
    pub fn add(&mut self, expr: &str, condition: Option<String>) -> Result<usize, String> {
        let parsed = parse_expr(expr)?;
        let id = self.next_id;
        self.next_id += 1;

        let mut watch = WatchExpression::new(id, expr.to_string(), parsed);
        if let Some(cond) = condition {
            watch = watch.with_condition(cond);
        }

        self.watches.push(watch);
        Ok(id)
    }

    /// Remove a watch by ID
    pub fn remove(&mut self, id: usize) -> bool {
        if let Some(pos) = self.watches.iter().position(|w| w.id == id) {
            self.watches.remove(pos);
            true
        } else {
            false
        }
    }

    /// Enable a watch by ID
    pub fn enable(&mut self, id: usize) -> bool {
        if let Some(watch) = self.watches.iter_mut().find(|w| w.id == id) {
            watch.enabled = true;
            true
        } else {
            false
        }
    }

    /// Disable a watch by ID
    pub fn disable(&mut self, id: usize) -> bool {
        if let Some(watch) = self.watches.iter_mut().find(|w| w.id == id) {
            watch.enabled = false;
            true
        } else {
            false
        }
    }

    /// Get all watches
    pub fn list(&self) -> &[WatchExpression] {
        &self.watches
    }

    /// Get a watch by ID
    pub fn get(&self, id: usize) -> Option<&WatchExpression> {
        self.watches.iter().find(|w| w.id == id)
    }

    /// Check if any watch triggered (value changed)
    /// Returns trigger info if a watch was triggered
    pub fn check(&mut self, ctx: &EvalContext) -> Option<WatchTrigger> {
        for watch in &mut self.watches {
            if !watch.enabled {
                continue;
            }

            let current = watch.evaluate(ctx);

            // Check if value changed
            if watch.has_changed(&current) {
                let old_value = watch.last_value.take();
                watch.last_value = current.clone();

                // Check condition if present
                if !watch.condition_met(ctx) {
                    continue;
                }

                return Some(WatchTrigger {
                    id: watch.id,
                    old_value,
                    new_value: current,
                });
            }
        }
        None
    }

    /// Initialize watch values without triggering
    /// Call this at the start of debugging to set baseline values
    pub fn initialize(&mut self, ctx: &EvalContext) {
        for watch in &mut self.watches {
            watch.last_value = watch.evaluate(ctx);
        }
    }

    /// Clear all watches
    pub fn clear(&mut self) {
        self.watches.clear();
    }

    /// Get the number of watches
    pub fn count(&self) -> usize {
        self.watches.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_ctx() -> EvalContext {
        let mut ctx = HashMap::new();
        ctx.insert("X".to_string(), Value::Integer(10));
        ctx.insert("Y".to_string(), Value::Integer(5));
        ctx
    }

    #[test]
    fn test_add_watch() {
        let mut manager = WatchManager::new();
        let id = manager.add("x", None).unwrap();
        assert_eq!(id, 1);
        assert_eq!(manager.count(), 1);
    }

    #[test]
    fn test_invalid_expression() {
        let mut manager = WatchManager::new();
        assert!(manager.add("@#$", None).is_err());
    }

    #[test]
    fn test_watch_trigger() {
        let mut manager = WatchManager::new();
        let id = manager.add("x", None).unwrap();

        // Initialize with first context
        let ctx1 = make_ctx();
        manager.initialize(&ctx1);

        // Same value - no trigger
        assert!(manager.check(&ctx1).is_none());

        // Changed value - trigger
        let mut ctx2 = make_ctx();
        ctx2.insert("X".to_string(), Value::Integer(20));

        let trigger = manager.check(&ctx2).unwrap();
        assert_eq!(trigger.id, id);
        assert_eq!(trigger.old_value, Some(Value::Integer(10)));
        assert_eq!(trigger.new_value, Some(Value::Integer(20)));
    }

    #[test]
    fn test_watch_condition() {
        let mut manager = WatchManager::new();
        manager.add("x", Some("x > 15".to_string())).unwrap();

        let ctx1 = make_ctx();
        manager.initialize(&ctx1);

        // Value changes but condition not met
        let mut ctx2 = make_ctx();
        ctx2.insert("X".to_string(), Value::Integer(12));
        assert!(manager.check(&ctx2).is_none());

        // Value changes and condition met
        let mut ctx3 = make_ctx();
        ctx3.insert("X".to_string(), Value::Integer(20));
        assert!(manager.check(&ctx3).is_some());
    }

    #[test]
    fn test_disable_watch() {
        let mut manager = WatchManager::new();
        let id = manager.add("x", None).unwrap();

        let ctx = make_ctx();
        manager.initialize(&ctx);
        manager.disable(id);

        let mut ctx2 = make_ctx();
        ctx2.insert("X".to_string(), Value::Integer(100));

        // Disabled, so no trigger
        assert!(manager.check(&ctx2).is_none());

        // Re-enable
        manager.enable(id);
        assert!(manager.check(&ctx2).is_some());
    }

    #[test]
    fn test_remove_watch() {
        let mut manager = WatchManager::new();
        let id = manager.add("x", None).unwrap();

        assert_eq!(manager.count(), 1);
        assert!(manager.remove(id));
        assert_eq!(manager.count(), 0);
        assert!(!manager.remove(id)); // Already removed
    }
}
