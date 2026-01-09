//! Profile report generation
//!
//! This module generates human-readable and machine-readable profile reports
//! from collected profiling data.

use std::io::{self, Write};
use std::time::Duration;

use super::ProfileData;

/// Profile report generator
pub struct ProfileReport<'a> {
    data: &'a ProfileData,
}

impl<'a> ProfileReport<'a> {
    /// Create a new report from profile data
    pub fn new(data: &'a ProfileData) -> Self {
        Self { data }
    }

    /// Format a duration for display
    fn format_duration(d: Duration) -> String {
        let total_ns = d.as_nanos();

        if total_ns == 0 {
            return "0.000s".to_string();
        }

        let total_us = d.as_micros();
        let total_ms = d.as_millis();
        let total_secs = d.as_secs_f64();

        if total_secs >= 1.0 {
            format!("{:.3}s", total_secs)
        } else if total_ms >= 1 {
            format!("{:.3}ms", total_ms as f64 + (total_us % 1000) as f64 / 1000.0)
        } else if total_us >= 1 {
            format!("{:.3}us", total_us as f64 + (total_ns % 1000) as f64 / 1000.0)
        } else {
            format!("{}ns", total_ns)
        }
    }

    /// Calculate percentage of total time
    fn percentage(&self, d: Duration) -> f64 {
        if self.data.total_time.as_nanos() == 0 {
            0.0
        } else {
            (d.as_nanos() as f64 / self.data.total_time.as_nanos() as f64) * 100.0
        }
    }

    /// Write text report to a writer
    pub fn write_text<W: Write>(&self, w: &mut W) -> io::Result<()> {
        writeln!(w)?;
        writeln!(w, "Profile Report")?;
        writeln!(w, "==============")?;
        writeln!(w)?;
        writeln!(w, "Total execution time: {}", Self::format_duration(self.data.total_time))?;
        writeln!(w)?;

        // Top functions by self time
        self.write_top_functions(w)?;

        // Call graph
        self.write_call_graph(w)?;

        // Hot lines (if any)
        self.write_hot_lines(w)?;

        // Loop metrics (if any)
        self.write_loop_metrics(w)?;

        // Memory metrics (if any)
        self.write_memory_metrics(w)?;

        Ok(())
    }

    /// Write top functions section
    fn write_top_functions<W: Write>(&self, w: &mut W) -> io::Result<()> {
        let top_funcs = self.data.top_functions_by_self_time(10);

        if top_funcs.is_empty() {
            writeln!(w, "No function calls recorded.")?;
            return Ok(());
        }

        writeln!(w, "Top Functions by Self Time:")?;
        writeln!(w, "---------------------------")?;
        writeln!(w, "  {:<20} {:>10} {:>7} {:>8} {:>12}",
            "Function", "Self Time", "%", "Calls", "Avg/Call")?;
        writeln!(w, "  {:<20} {:>10} {:>7} {:>8} {:>12}",
            "-".repeat(20), "-".repeat(10), "-".repeat(7), "-".repeat(8), "-".repeat(12))?;

        for (name, metrics) in &top_funcs {
            let display_name = if name.len() > 20 {
                format!("{}...", &name[..17])
            } else {
                name.to_string()
            };

            writeln!(w, "  {:<20} {:>10} {:>6.1}% {:>8} {:>12}",
                display_name,
                Self::format_duration(metrics.self_time),
                self.percentage(metrics.self_time),
                metrics.call_count,
                Self::format_duration(metrics.avg_self_time()))?;
        }

        writeln!(w)?;
        Ok(())
    }

    /// Write call graph section
    fn write_call_graph<W: Write>(&self, w: &mut W) -> io::Result<()> {
        let edges = self.data.call_graph_edges();

        if edges.is_empty() {
            return Ok(());
        }

        writeln!(w, "Call Graph:")?;
        writeln!(w, "-----------")?;

        // Only show top 20 edges
        for ((caller, callee), count) in edges.iter().take(20) {
            writeln!(w, "  {} -> {} ({} calls)", caller, callee, count)?;
        }

        if edges.len() > 20 {
            writeln!(w, "  ... and {} more edges", edges.len() - 20)?;
        }

        writeln!(w)?;
        Ok(())
    }

    /// Write hot lines section
    fn write_hot_lines<W: Write>(&self, w: &mut W) -> io::Result<()> {
        let hot_lines = self.data.top_hot_lines(10);

        if hot_lines.is_empty() || hot_lines.iter().all(|(_, count)| *count <= 1) {
            return Ok(());
        }

        writeln!(w, "Hot Lines:")?;
        writeln!(w, "----------")?;

        for (line, count) in &hot_lines {
            if *count > 1 {
                writeln!(w, "  Line {:>5}: {} executions", line, count)?;
            }
        }

        writeln!(w)?;
        Ok(())
    }

    /// Write loop metrics section
    fn write_loop_metrics<W: Write>(&self, w: &mut W) -> io::Result<()> {
        if self.data.loops.is_empty() {
            return Ok(());
        }

        writeln!(w, "Loop Metrics:")?;
        writeln!(w, "-------------")?;

        // Sort by total time
        let mut loops: Vec<_> = self.data.loops.values().collect();
        loops.sort_by(|a, b| b.total_time.cmp(&a.total_time));

        for metrics in loops.iter().take(10) {
            writeln!(w, "  Line {:>5}: {} iterations ({} entries), {} total",
                metrics.line,
                metrics.iterations,
                metrics.entry_count,
                Self::format_duration(metrics.total_time))?;
        }

        writeln!(w)?;
        Ok(())
    }

    /// Write memory metrics section
    fn write_memory_metrics<W: Write>(&self, w: &mut W) -> io::Result<()> {
        if self.data.memory.allocation_count == 0 {
            return Ok(());
        }

        writeln!(w, "Memory Allocations:")?;
        writeln!(w, "-------------------")?;
        writeln!(w, "  Total allocated: {} bytes ({} allocations)",
            self.data.memory.total_allocated,
            self.data.memory.allocation_count)?;
        writeln!(w, "  Peak usage: {} bytes", self.data.memory.peak_usage)?;

        if !self.data.memory.allocations.is_empty() {
            writeln!(w)?;
            writeln!(w, "  Allocations:")?;
            for alloc in self.data.memory.allocations.iter().take(10) {
                let name = alloc.name.as_deref().unwrap_or("<unnamed>");
                writeln!(w, "    Line {:>5}: {} bytes ({})",
                    alloc.line, alloc.size, name)?;
            }
            if self.data.memory.allocations.len() > 10 {
                writeln!(w, "    ... and {} more allocations",
                    self.data.memory.allocations.len() - 10)?;
            }
        }

        writeln!(w)?;
        Ok(())
    }

    /// Generate text report as string
    pub fn to_text(&self) -> String {
        let mut buf = Vec::new();
        self.write_text(&mut buf).unwrap();
        String::from_utf8(buf).unwrap_or_default()
    }

    /// Generate JSON report
    pub fn to_json(&self) -> String {
        let mut result = String::from("{\n");

        // Total time
        result.push_str(&format!(
            "  \"total_time_ms\": {:.3},\n",
            self.data.total_time.as_secs_f64() * 1000.0
        ));

        // Functions
        result.push_str("  \"functions\": [\n");
        let funcs: Vec<_> = self.data.functions.iter().collect();
        for (i, (name, metrics)) in funcs.iter().enumerate() {
            result.push_str("    {\n");
            result.push_str(&format!("      \"name\": \"{}\",\n", name));
            result.push_str(&format!("      \"call_count\": {},\n", metrics.call_count));
            result.push_str(&format!(
                "      \"total_time_ms\": {:.6},\n",
                metrics.total_time.as_secs_f64() * 1000.0
            ));
            result.push_str(&format!(
                "      \"self_time_ms\": {:.6}\n",
                metrics.self_time.as_secs_f64() * 1000.0
            ));
            result.push_str("    }");
            if i < funcs.len() - 1 {
                result.push(',');
            }
            result.push('\n');
        }
        result.push_str("  ],\n");

        // Call graph
        result.push_str("  \"call_graph\": [\n");
        let edges: Vec<_> = self.data.call_graph.iter().collect();
        for (i, ((caller, callee), count)) in edges.iter().enumerate() {
            result.push_str(&format!(
                "    {{\"caller\": \"{}\", \"callee\": \"{}\", \"count\": {}}}",
                caller, callee, count
            ));
            if i < edges.len() - 1 {
                result.push(',');
            }
            result.push('\n');
        }
        result.push_str("  ]\n");

        result.push('}');
        result
    }

    /// Generate flat profile (just function times, sorted)
    pub fn flat_profile(&self) -> String {
        let mut result = String::new();

        result.push_str(&format!("Flat profile: {} total\n\n", Self::format_duration(self.data.total_time)));

        let top_funcs = self.data.top_functions_by_self_time(100);

        result.push_str(&format!("{:>7} {:>7} {:>8} {:>10}  {}\n",
            "% time", "self", "calls", "self/call", "name"));

        for (name, metrics) in &top_funcs {
            result.push_str(&format!(
                "{:>6.1}% {:>7} {:>8} {:>10}  {}\n",
                self.percentage(metrics.self_time),
                Self::format_duration(metrics.self_time),
                metrics.call_count,
                Self::format_duration(metrics.avg_self_time()),
                name
            ));
        }

        result
    }

    /// Generate DOT format call graph for Graphviz
    pub fn to_dot(&self) -> String {
        let mut result = String::from("digraph callgraph {\n");
        result.push_str("  rankdir=TB;\n");
        result.push_str("  node [shape=box];\n\n");

        for ((caller, callee), count) in &self.data.call_graph {
            result.push_str(&format!(
                "  \"{}\" -> \"{}\" [label=\"{}\"];\n",
                caller, callee, count
            ));
        }

        result.push_str("}\n");
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profiler::FunctionMetrics;

    fn make_test_data() -> ProfileData {
        let mut data = ProfileData::new();
        data.total_time = Duration::from_millis(1000);

        data.functions.insert("fast_func".to_string(), FunctionMetrics {
            call_count: 100,
            total_time: Duration::from_millis(100),
            self_time: Duration::from_millis(50),
            min_time: Duration::from_micros(400),
            max_time: Duration::from_millis(2),
        });

        data.functions.insert("slow_func".to_string(), FunctionMetrics {
            call_count: 10,
            total_time: Duration::from_millis(800),
            self_time: Duration::from_millis(600),
            min_time: Duration::from_millis(50),
            max_time: Duration::from_millis(100),
        });

        data.call_graph.insert(
            ("<main>".to_string(), "slow_func".to_string()),
            10,
        );
        data.call_graph.insert(
            ("slow_func".to_string(), "fast_func".to_string()),
            100,
        );

        data
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(ProfileReport::format_duration(Duration::from_secs(2)), "2.000s");
        assert_eq!(ProfileReport::format_duration(Duration::from_millis(100)), "100.000ms");
        assert_eq!(ProfileReport::format_duration(Duration::from_micros(500)), "500.000us");
        assert_eq!(ProfileReport::format_duration(Duration::from_nanos(100)), "100ns");
    }

    #[test]
    fn test_text_report() {
        let data = make_test_data();
        let report = ProfileReport::new(&data);
        let text = report.to_text();

        assert!(text.contains("Profile Report"));
        assert!(text.contains("Total execution time:"));
        assert!(text.contains("slow_func"));
        assert!(text.contains("fast_func"));
    }

    #[test]
    fn test_json_report() {
        let data = make_test_data();
        let report = ProfileReport::new(&data);
        let json = report.to_json();

        assert!(json.contains("\"total_time_ms\""));
        assert!(json.contains("\"functions\""));
        assert!(json.contains("\"call_graph\""));
        assert!(json.contains("\"slow_func\""));
    }

    #[test]
    fn test_flat_profile() {
        let data = make_test_data();
        let report = ProfileReport::new(&data);
        let flat = report.flat_profile();

        assert!(flat.contains("Flat profile"));
        assert!(flat.contains("% time"));
    }

    #[test]
    fn test_dot_export() {
        let data = make_test_data();
        let report = ProfileReport::new(&data);
        let dot = report.to_dot();

        assert!(dot.contains("digraph callgraph"));
        assert!(dot.contains("->"));
    }

    #[test]
    fn test_percentage() {
        let data = make_test_data();
        let report = ProfileReport::new(&data);

        let pct = report.percentage(Duration::from_millis(500));
        assert!((pct - 50.0).abs() < 0.1);
    }
}
