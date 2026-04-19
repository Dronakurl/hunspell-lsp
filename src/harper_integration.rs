//! Harper language server integration for enhanced English spell checking.
//!
//! This module handles detection of Harper installation and routing of English
//! language checking requests to Harper when available.

use anyhow::Result;
use std::process::Command;

/// Checks if Harper language server is available on the system.
///
/// # Returns
///
/// * `true` - If `harper-ls` command is found in PATH
/// * `false` - If Harper is not installed or not accessible
pub fn is_harper_available() -> bool {
    Command::new("harper-ls")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Gets the Harper command path if available.
///
/// # Returns
///
/// * `Some(String)` - Path to Harper executable if found
/// * `None` - If Harper is not available
pub fn get_harper_command() -> Option<String> {
    which::which("harper-ls")
        .ok()
        .and_then(|path| path.to_str().map(|s| s.to_string()))
}

/// Checks if Harper should be used for a specific language.
///
/// # Arguments
///
/// * `lang` - Language code to check
///
/// # Returns
///
/// * `true` - If language is English and Harper is available
/// * `false` - If language is not English or Harper is unavailable
pub fn should_use_harper(lang: &str) -> bool {
    crate::is_english_lang(lang) && is_harper_available()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_harper_available() {
        // This test will pass if Harper is installed, fail otherwise
        // We don't assert the result since Harper might not be installed
        let _available = is_harper_available();
        println!("Harper available: {}", _available);
    }

    #[test]
    fn test_should_use_harper_english() {
        // If Harper is installed, it should be used for English
        // If not installed, it should not be used
        let result = should_use_harper("en_US");
        println!("Should use Harper for en_US: {}", result);

        // Verify that the result is consistent with availability
        assert_eq!(result, is_harper_available());
    }

    #[test]
    fn test_should_not_use_harper_non_english() {
        // Harper should never be used for non-English languages
        assert!(!should_use_harper("de_DE"));
        assert!(!should_use_harper("fr_FR"));
        assert!(!should_use_harper("es_ES"));
        assert!(!should_use_harper("zh_CN"));
    }

    #[test]
    fn test_english_detection() {
        // These should all return the same result (based on Harper availability)
        let en_us_result = should_use_harper("en_US");
        let en_gb_result = should_use_harper("en_GB");
        let en_result = should_use_harper("en");

        assert_eq!(en_us_result, en_gb_result);
        assert_eq!(en_us_result, en_result);
    }
}
