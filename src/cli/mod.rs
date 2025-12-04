//! CLI argument parsing and validation interfaces

use crate::{PolyglotError, PolyglotResult};

/// Validation result for polyglot files
#[derive(Debug, PartialEq)]
pub enum ValidationResult {
    /// File is a valid PNG/ZIP polyglot
    Valid,
    /// Invalid PNG with error message
    InvalidPng(String),
    /// Invalid ZIP with error message
    InvalidZip(String),
    /// Both PNG and ZIP are invalid
    InvalidBoth(String, String),
}

// Additional CLI-related functions can be added here
// Currently, most CLI logic is in main.rs with clap

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_result_display() {
        assert!(matches!(ValidationResult::Valid, ValidationResult::Valid));

        let invalid_png = ValidationResult::InvalidPng("test".to_string());
        assert!(matches!(invalid_png, ValidationResult::InvalidPng(_)));
    }
}
