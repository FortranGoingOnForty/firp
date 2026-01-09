//! Warning system configuration
//!
//! Provides configuration for enabling, suppressing, and promoting warnings.

use std::collections::HashSet;
use super::codes::ErrorCode;

/// Warning configuration level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WarningLevel {
    /// Warning is enabled (default)
    Warn,
    /// Warning is suppressed/allowed
    Allow,
    /// Warning is treated as an error
    Deny,
}

/// Configuration for which warnings are enabled
#[derive(Debug, Clone)]
pub struct WarningConfig {
    /// Default level for all warnings
    default_level: WarningLevel,
    /// Specific overrides for individual warning codes
    overrides: HashSet<(u16, WarningLevel)>,
    /// Warnings that have been emitted (for "emit once" behavior)
    emitted: HashSet<(u16, usize, usize)>, // (code, line, column)
}

impl Default for WarningConfig {
    fn default() -> Self {
        Self {
            default_level: WarningLevel::Warn,
            overrides: HashSet::new(),
            emitted: HashSet::new(),
        }
    }
}

impl WarningConfig {
    /// Create a new warning configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the default warning level
    pub fn set_default_level(&mut self, level: WarningLevel) {
        self.default_level = level;
    }

    /// Set level for a specific warning
    pub fn set_warning_level(&mut self, code: ErrorCode, level: WarningLevel) {
        if code.is_warning {
            // Remove any existing override
            self.overrides.retain(|(c, _)| *c != code.number);
            // Add new override
            self.overrides.insert((code.number, level));
        }
    }

    /// Enable a warning (set to Warn level)
    pub fn enable(&mut self, code: ErrorCode) {
        self.set_warning_level(code, WarningLevel::Warn);
    }

    /// Allow/suppress a warning
    pub fn allow(&mut self, code: ErrorCode) {
        self.set_warning_level(code, WarningLevel::Allow);
    }

    /// Deny a warning (treat as error)
    pub fn deny(&mut self, code: ErrorCode) {
        self.set_warning_level(code, WarningLevel::Deny);
    }

    /// Get the effective level for a warning
    pub fn get_level(&self, code: ErrorCode) -> WarningLevel {
        if !code.is_warning {
            return WarningLevel::Deny; // Errors are always denied
        }

        // Check for specific override
        for (c, level) in &self.overrides {
            if *c == code.number {
                return *level;
            }
        }

        self.default_level
    }

    /// Check if a warning should be emitted
    pub fn should_emit(&self, code: ErrorCode) -> bool {
        matches!(self.get_level(code), WarningLevel::Warn | WarningLevel::Deny)
    }

    /// Check if a warning should be treated as an error
    pub fn is_error(&self, code: ErrorCode) -> bool {
        self.get_level(code) == WarningLevel::Deny
    }

    /// Record that a warning was emitted at a location (for deduplication)
    pub fn record_emission(&mut self, code: ErrorCode, line: usize, column: usize) {
        if code.is_warning {
            self.emitted.insert((code.number, line, column));
        }
    }

    /// Check if a warning was already emitted at this location
    pub fn was_emitted(&self, code: ErrorCode, line: usize, column: usize) -> bool {
        code.is_warning && self.emitted.contains(&(code.number, line, column))
    }

    /// Clear all emission records (useful between files)
    pub fn clear_emissions(&mut self) {
        self.emitted.clear();
    }

    /// Enable all warnings
    pub fn enable_all(&mut self) {
        self.default_level = WarningLevel::Warn;
        self.overrides.clear();
    }

    /// Suppress all warnings
    pub fn suppress_all(&mut self) {
        self.default_level = WarningLevel::Allow;
        self.overrides.clear();
    }

    /// Treat all warnings as errors
    pub fn warnings_as_errors(&mut self) {
        self.default_level = WarningLevel::Deny;
    }
}

/// Standard warning presets
impl WarningConfig {
    /// Minimal warnings (only critical issues)
    pub fn minimal() -> Self {
        let mut config = Self::new();
        config.suppress_all();
        config
    }

    /// Standard warnings (recommended for development)
    pub fn standard() -> Self {
        Self::new() // All warnings enabled by default
    }

    /// Strict warnings (all warnings as errors)
    pub fn strict() -> Self {
        let mut config = Self::new();
        config.warnings_as_errors();
        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::codes::*;

    #[test]
    fn test_default_config() {
        let config = WarningConfig::new();
        assert_eq!(config.get_level(W0001_UNUSED_VARIABLE), WarningLevel::Warn);
        assert!(config.should_emit(W0001_UNUSED_VARIABLE));
        assert!(!config.is_error(W0001_UNUSED_VARIABLE));
    }

    #[test]
    fn test_allow_warning() {
        let mut config = WarningConfig::new();
        config.allow(W0001_UNUSED_VARIABLE);

        assert_eq!(config.get_level(W0001_UNUSED_VARIABLE), WarningLevel::Allow);
        assert!(!config.should_emit(W0001_UNUSED_VARIABLE));
    }

    #[test]
    fn test_deny_warning() {
        let mut config = WarningConfig::new();
        config.deny(W0001_UNUSED_VARIABLE);

        assert_eq!(config.get_level(W0001_UNUSED_VARIABLE), WarningLevel::Deny);
        assert!(config.should_emit(W0001_UNUSED_VARIABLE));
        assert!(config.is_error(W0001_UNUSED_VARIABLE));
    }

    #[test]
    fn test_emission_tracking() {
        let mut config = WarningConfig::new();

        assert!(!config.was_emitted(W0001_UNUSED_VARIABLE, 10, 5));

        config.record_emission(W0001_UNUSED_VARIABLE, 10, 5);

        assert!(config.was_emitted(W0001_UNUSED_VARIABLE, 10, 5));
        assert!(!config.was_emitted(W0001_UNUSED_VARIABLE, 10, 6)); // Different column
    }

    #[test]
    fn test_presets() {
        let minimal = WarningConfig::minimal();
        assert_eq!(minimal.get_level(W0001_UNUSED_VARIABLE), WarningLevel::Allow);

        let standard = WarningConfig::standard();
        assert_eq!(standard.get_level(W0001_UNUSED_VARIABLE), WarningLevel::Warn);

        let strict = WarningConfig::strict();
        assert_eq!(strict.get_level(W0001_UNUSED_VARIABLE), WarningLevel::Deny);
    }

    #[test]
    fn test_errors_are_always_deny() {
        let mut config = WarningConfig::new();
        config.suppress_all();

        // Errors should always be "denied" (treated as errors)
        assert_eq!(config.get_level(E0201_UNDECLARED_VARIABLE), WarningLevel::Deny);
    }
}
