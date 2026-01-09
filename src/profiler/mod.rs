//! Profiling infrastructure for FIRP
//!
//! This module provides profiling capabilities to identify performance
//! bottlenecks in Fortran programs. It integrates with the VM through
//! the Debugger trait.
//!
//! # Usage
//!
//! ```ignore
//! use firp::profiler::ProfilerDebugger;
//! use firp::vm::VM;
//!
//! let mut profiler = ProfilerDebugger::new();
//! profiler.start();
//! vm.set_debugger(Some(Box::new(profiler)));
//! vm.run(chunk)?;
//! // Get profile data back and generate report
//! ```

pub mod collector;
pub mod report;

use std::collections::HashMap;
use std::time::Duration;

pub use collector::ProfilerDebugger;
pub use report::ProfileReport;

/// Per-function profiling metrics
#[derive(Debug, Clone, Default)]
pub struct FunctionMetrics {
    /// Number of times called
    pub call_count: usize,
    /// Total time spent (inclusive - includes callees)
    pub total_time: Duration,
    /// Time spent in function only (exclusive - excludes callees)
    pub self_time: Duration,
    /// Minimum single-call time
    pub min_time: Duration,
    /// Maximum single-call time
    pub max_time: Duration,
}

impl FunctionMetrics {
    /// Calculate average time per call
    pub fn avg_time(&self) -> Duration {
        if self.call_count > 0 {
            self.total_time / self.call_count as u32
        } else {
            Duration::ZERO
        }
    }

    /// Calculate average self time per call
    pub fn avg_self_time(&self) -> Duration {
        if self.call_count > 0 {
            self.self_time / self.call_count as u32
        } else {
            Duration::ZERO
        }
    }
}

/// Per-loop profiling metrics
#[derive(Debug, Clone, Default)]
pub struct LoopMetrics {
    /// Source location (line number)
    pub line: usize,
    /// Total iterations across all executions
    pub iterations: usize,
    /// Number of times the loop was entered
    pub entry_count: usize,
    /// Total time spent in loop
    pub total_time: Duration,
}

impl LoopMetrics {
    /// Calculate average iterations per entry
    pub fn avg_iterations(&self) -> f64 {
        if self.entry_count > 0 {
            self.iterations as f64 / self.entry_count as f64
        } else {
            0.0
        }
    }
}

/// Memory allocation record
#[derive(Debug, Clone)]
pub struct AllocationRecord {
    /// Source line where allocation occurred
    pub line: usize,
    /// Size in bytes (approximate)
    pub size: usize,
    /// Variable name if known
    pub name: Option<String>,
}

/// Memory profiling metrics
#[derive(Debug, Clone, Default)]
pub struct MemoryMetrics {
    /// Total bytes allocated
    pub total_allocated: usize,
    /// Peak memory usage
    pub peak_usage: usize,
    /// Current usage
    pub current_usage: usize,
    /// Number of allocations
    pub allocation_count: usize,
    /// Individual allocation records
    pub allocations: Vec<AllocationRecord>,
}

/// Complete profile data collected during execution
#[derive(Debug, Default, Clone)]
pub struct ProfileData {
    /// Total execution time
    pub total_time: Duration,
    /// Per-function metrics (key = function name)
    pub functions: HashMap<String, FunctionMetrics>,
    /// Per-loop metrics (key = line number)
    pub loops: HashMap<usize, LoopMetrics>,
    /// Bytecode instruction counts (key = opcode name)
    pub instruction_counts: HashMap<String, usize>,
    /// Call graph edges: (caller, callee) -> call count
    pub call_graph: HashMap<(String, String), usize>,
    /// Hot lines (line -> execution count)
    pub hot_lines: HashMap<usize, usize>,
    /// Memory metrics (if enabled)
    pub memory: MemoryMetrics,
}

impl ProfileData {
    /// Create empty profile data
    pub fn new() -> Self {
        Self::default()
    }

    /// Get top N functions by self time
    pub fn top_functions_by_self_time(&self, n: usize) -> Vec<(&str, &FunctionMetrics)> {
        let mut funcs: Vec<_> = self.functions.iter()
            .map(|(name, metrics)| (name.as_str(), metrics))
            .collect();
        funcs.sort_by(|a, b| b.1.self_time.cmp(&a.1.self_time));
        funcs.truncate(n);
        funcs
    }

    /// Get top N functions by total time
    pub fn top_functions_by_total_time(&self, n: usize) -> Vec<(&str, &FunctionMetrics)> {
        let mut funcs: Vec<_> = self.functions.iter()
            .map(|(name, metrics)| (name.as_str(), metrics))
            .collect();
        funcs.sort_by(|a, b| b.1.total_time.cmp(&a.1.total_time));
        funcs.truncate(n);
        funcs
    }

    /// Get top N functions by call count
    pub fn top_functions_by_calls(&self, n: usize) -> Vec<(&str, &FunctionMetrics)> {
        let mut funcs: Vec<_> = self.functions.iter()
            .map(|(name, metrics)| (name.as_str(), metrics))
            .collect();
        funcs.sort_by(|a, b| b.1.call_count.cmp(&a.1.call_count));
        funcs.truncate(n);
        funcs
    }

    /// Get top N hot lines by execution count
    pub fn top_hot_lines(&self, n: usize) -> Vec<(usize, usize)> {
        let mut lines: Vec<_> = self.hot_lines.iter()
            .map(|(&line, &count)| (line, count))
            .collect();
        lines.sort_by(|a, b| b.1.cmp(&a.1));
        lines.truncate(n);
        lines
    }

    /// Get call graph edges sorted by call count
    pub fn call_graph_edges(&self) -> Vec<((&str, &str), usize)> {
        let mut edges: Vec<_> = self.call_graph.iter()
            .map(|((caller, callee), &count)| ((caller.as_str(), callee.as_str()), count))
            .collect();
        edges.sort_by(|a, b| b.1.cmp(&a.1));
        edges
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_metrics_avg() {
        let metrics = FunctionMetrics {
            call_count: 10,
            total_time: Duration::from_millis(100),
            self_time: Duration::from_millis(50),
            min_time: Duration::from_millis(5),
            max_time: Duration::from_millis(20),
        };

        assert_eq!(metrics.avg_time(), Duration::from_millis(10));
        assert_eq!(metrics.avg_self_time(), Duration::from_millis(5));
    }

    #[test]
    fn test_function_metrics_zero_calls() {
        let metrics = FunctionMetrics::default();
        assert_eq!(metrics.avg_time(), Duration::ZERO);
        assert_eq!(metrics.avg_self_time(), Duration::ZERO);
    }

    #[test]
    fn test_profile_data_top_functions() {
        let mut data = ProfileData::new();
        data.functions.insert("fast".to_string(), FunctionMetrics {
            call_count: 100,
            self_time: Duration::from_millis(10),
            ..Default::default()
        });
        data.functions.insert("slow".to_string(), FunctionMetrics {
            call_count: 1,
            self_time: Duration::from_millis(1000),
            ..Default::default()
        });
        data.functions.insert("medium".to_string(), FunctionMetrics {
            call_count: 10,
            self_time: Duration::from_millis(100),
            ..Default::default()
        });

        let top = data.top_functions_by_self_time(2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].0, "slow");
        assert_eq!(top[1].0, "medium");
    }

    #[test]
    fn test_loop_metrics_avg() {
        let metrics = LoopMetrics {
            line: 10,
            iterations: 1000,
            entry_count: 10,
            total_time: Duration::from_secs(1),
        };

        assert_eq!(metrics.avg_iterations(), 100.0);
    }
}
