//! Profile data collection during execution
//!
//! This module provides the ProfilerDebugger which implements the Debugger
//! trait to collect profiling data during program execution.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::bytecode::Value;
use crate::debugger::traits::Debugger;
use crate::debugger::state::DebugState;
use crate::lexer::SourceLocation;

use super::{ProfileData, LoopMetrics};

/// Active call on the profile stack
struct ProfileCall {
    /// Function/procedure name
    name: String,
    /// When the call started
    start_time: Instant,
    /// Time spent in child calls (for calculating self time)
    child_time: Duration,
}

/// Profiler that implements Debugger trait to collect execution metrics
pub struct ProfilerDebugger {
    /// Collected profile data (shared for extraction)
    data: Arc<Mutex<ProfileData>>,
    /// Active call stack with timestamps
    call_stack: Vec<ProfileCall>,
    /// Start time of profiling
    start_time: Option<Instant>,
    /// Track if we're actively profiling
    enabled: bool,
    /// Current call depth
    current_depth: usize,
    /// Track loop entry times (line -> entry time)
    loop_times: std::collections::HashMap<usize, Instant>,
}

impl Default for ProfilerDebugger {
    fn default() -> Self {
        Self::new()
    }
}

impl ProfilerDebugger {
    /// Create a new profiler (disabled by default)
    pub fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(ProfileData::new())),
            call_stack: Vec::with_capacity(64), // Pre-allocate for typical call depth
            start_time: None,
            enabled: false,
            current_depth: 0,
            loop_times: std::collections::HashMap::new(),
        }
    }

    /// Start profiling
    pub fn start(&mut self) {
        self.enabled = true;
        self.start_time = Some(Instant::now());

        // Push a synthetic "main" call to represent top-level execution
        self.call_stack.push(ProfileCall {
            name: "<main>".to_string(),
            start_time: Instant::now(),
            child_time: Duration::ZERO,
        });
    }

    /// Stop profiling and finalize metrics
    pub fn stop(&mut self) {
        if !self.enabled {
            return;
        }

        // Pop all remaining calls (shouldn't happen normally, but handle it)
        while let Some(call) = self.call_stack.pop() {
            self.record_return(&call.name, call.start_time, call.child_time);
        }

        // Record total time
        if let Some(start) = self.start_time {
            if let Ok(mut data) = self.data.lock() {
                data.total_time = start.elapsed();
            }
        }

        self.enabled = false;
    }

    /// Get reference to shared profile data
    pub fn data(&self) -> Arc<Mutex<ProfileData>> {
        Arc::clone(&self.data)
    }

    /// Take the profile data (consumes profiler)
    pub fn take_data(mut self) -> ProfileData {
        // Stop profiling first to finalize metrics
        self.stop();

        // Clone the data out of the Arc/Mutex
        if let Ok(data) = self.data.lock() {
            data.clone()
        } else {
            ProfileData::default()
        }
    }

    /// Check if profiling is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Record a function return
    fn record_return(&self, name: &str, start_time: Instant, child_time: Duration) {
        let elapsed = start_time.elapsed();
        let self_time = elapsed.saturating_sub(child_time);

        if let Ok(mut data) = self.data.lock() {
            let metrics = data.functions
                .entry(name.to_string())
                .or_default();

            metrics.call_count += 1;
            metrics.total_time += elapsed;
            metrics.self_time += self_time;

            // Update min/max
            if metrics.min_time == Duration::ZERO || elapsed < metrics.min_time {
                metrics.min_time = elapsed;
            }
            if elapsed > metrics.max_time {
                metrics.max_time = elapsed;
            }
        }
    }

    /// Record a line execution
    fn record_line(&self, line: usize) {
        if let Ok(mut data) = self.data.lock() {
            *data.hot_lines.entry(line).or_default() += 1;
        }
    }

    /// Record a call graph edge
    fn record_call_edge(&self, caller: &str, callee: &str) {
        if let Ok(mut data) = self.data.lock() {
            let edge = (caller.to_string(), callee.to_string());
            *data.call_graph.entry(edge).or_default() += 1;
        }
    }

}

impl Debugger for ProfilerDebugger {
    fn on_instruction(&mut self, _ip: usize, location: Option<SourceLocation>) -> bool {
        if !self.enabled {
            return true;
        }

        // Track hot lines
        if let Some(loc) = location {
            self.record_line(loc.line);
        }

        true // Always continue execution
    }

    fn on_function_call(&mut self, name: &str, _args: &[Value]) {
        if !self.enabled {
            return;
        }

        // Record caller -> callee edge
        if let Some(caller) = self.call_stack.last() {
            self.record_call_edge(&caller.name, name);
        }

        // Push new call onto stack
        self.call_stack.push(ProfileCall {
            name: name.to_string(),
            start_time: Instant::now(),
            child_time: Duration::ZERO,
        });
    }

    fn on_function_return(&mut self, _name: &str, _result: Option<&Value>) {
        if !self.enabled {
            return;
        }

        if let Some(call) = self.call_stack.pop() {
            let elapsed = call.start_time.elapsed();

            // Update parent's child_time
            if let Some(parent) = self.call_stack.last_mut() {
                parent.child_time += elapsed;
            }

            // Record the metrics
            self.record_return(&call.name, call.start_time, call.child_time);
        }
    }

    fn on_variable_changed(&mut self, _name: &str, _old: Option<&Value>, _new: &Value) {
        // Could track variable changes for advanced profiling
        // For now, do nothing
    }

    fn should_pause(&self) -> bool {
        false // Profiler never pauses execution
    }

    fn state(&self) -> DebugState {
        DebugState::Running // Always running
    }

    fn set_current_location(&mut self, _location: Option<SourceLocation>) {
        // Not used by profiler
    }

    fn call_depth(&self) -> usize {
        self.current_depth
    }

    fn enter_function(&mut self) {
        self.current_depth += 1;
    }

    fn leave_function(&mut self) {
        if self.current_depth > 0 {
            self.current_depth -= 1;
        }
    }

    fn on_loop_start(&mut self, line: usize) {
        if !self.enabled {
            return;
        }

        self.loop_times.insert(line, Instant::now());

        if let Ok(mut data) = self.data.lock() {
            let metrics = data.loops.entry(line).or_insert_with(|| LoopMetrics {
                line,
                ..Default::default()
            });
            metrics.entry_count += 1;
        }
    }

    fn on_loop_iteration(&mut self, line: usize) {
        if !self.enabled {
            return;
        }

        if let Ok(mut data) = self.data.lock() {
            let metrics = data.loops.entry(line).or_insert_with(|| LoopMetrics {
                line,
                ..Default::default()
            });
            metrics.iterations += 1;
        }
    }

    fn on_loop_end(&mut self, line: usize) {
        if !self.enabled {
            return;
        }

        if let Some(start_time) = self.loop_times.remove(&line) {
            let elapsed = start_time.elapsed();

            if let Ok(mut data) = self.data.lock() {
                if let Some(metrics) = data.loops.get_mut(&line) {
                    metrics.total_time += elapsed;
                }
            }
        }
    }

    fn on_allocation(&mut self, line: usize, size: usize, name: Option<&str>) {
        if !self.enabled {
            return;
        }

        if let Ok(mut data) = self.data.lock() {
            data.memory.total_allocated += size;
            data.memory.current_usage += size;
            data.memory.allocation_count += 1;

            if data.memory.current_usage > data.memory.peak_usage {
                data.memory.peak_usage = data.memory.current_usage;
            }

            data.memory.allocations.push(super::AllocationRecord {
                line,
                size,
                name: name.map(|s| s.to_string()),
            });
        }
    }
}

impl Drop for ProfilerDebugger {
    fn drop(&mut self) {
        // Automatically finalize profiling when dropped
        if self.enabled {
            self.stop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profiler_new() {
        let profiler = ProfilerDebugger::new();
        assert!(!profiler.is_enabled());
    }

    #[test]
    fn test_profiler_start_stop() {
        let mut profiler = ProfilerDebugger::new();
        profiler.start();
        assert!(profiler.is_enabled());

        std::thread::sleep(Duration::from_millis(10));

        profiler.stop();
        assert!(!profiler.is_enabled());

        let data = profiler.take_data();
        assert!(data.total_time >= Duration::from_millis(10));
    }

    #[test]
    fn test_profiler_function_timing() {
        let mut profiler = ProfilerDebugger::new();
        profiler.start();

        // Simulate a function call
        profiler.on_function_call("test_func", &[]);
        std::thread::sleep(Duration::from_millis(5));
        profiler.on_function_return("test_func", None);

        profiler.stop();

        let data = profiler.take_data();
        assert!(data.functions.contains_key("test_func"));

        let metrics = &data.functions["test_func"];
        assert_eq!(metrics.call_count, 1);
        assert!(metrics.total_time >= Duration::from_millis(5));
    }

    #[test]
    fn test_profiler_call_graph() {
        let mut profiler = ProfilerDebugger::new();
        profiler.start();

        // main calls outer
        profiler.on_function_call("outer", &[]);
        // outer calls inner
        profiler.on_function_call("inner", &[]);
        profiler.on_function_return("inner", None);
        profiler.on_function_return("outer", None);

        profiler.stop();

        let data = profiler.take_data();

        // Check call graph edges
        assert!(data.call_graph.contains_key(&("<main>".to_string(), "outer".to_string())));
        assert!(data.call_graph.contains_key(&("outer".to_string(), "inner".to_string())));
    }

    #[test]
    fn test_profiler_nested_calls_self_time() {
        let mut profiler = ProfilerDebugger::new();
        profiler.start();

        // outer (total = 20ms, self = 10ms)
        profiler.on_function_call("outer", &[]);
        std::thread::sleep(Duration::from_millis(5));

        // inner (total = 10ms, self = 10ms)
        profiler.on_function_call("inner", &[]);
        std::thread::sleep(Duration::from_millis(10));
        profiler.on_function_return("inner", None);

        std::thread::sleep(Duration::from_millis(5));
        profiler.on_function_return("outer", None);

        profiler.stop();

        let data = profiler.take_data();

        let outer = &data.functions["outer"];
        let inner = &data.functions["inner"];

        // Inner's self time should be close to its total time
        assert!(inner.self_time.as_millis() >= 8); // Allow some slack

        // Outer's self time should be less than total time
        assert!(outer.self_time < outer.total_time);
    }

    #[test]
    fn test_profiler_hot_lines() {
        let mut profiler = ProfilerDebugger::new();
        profiler.start();

        // Simulate executing line 10 multiple times
        for _ in 0..100 {
            profiler.on_instruction(0, Some(SourceLocation { line: 10, column: 1 }));
        }

        profiler.stop();

        let data = profiler.take_data();
        assert_eq!(data.hot_lines.get(&10), Some(&100));
    }

    #[test]
    fn test_profiler_disabled() {
        let mut profiler = ProfilerDebugger::new();
        // Don't call start()

        profiler.on_function_call("should_not_record", &[]);
        profiler.on_function_return("should_not_record", None);

        let data = profiler.take_data();
        assert!(data.functions.is_empty());
    }
}
