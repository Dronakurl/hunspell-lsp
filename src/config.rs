//! Configuration management for hunspell-lsp.
//!
//! This module handles configuration for dictionary paths and LSP settings.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use url::Url;

// Re-export dictionary functions for convenience
pub use crate::dictionary_io::{get_user_dict_path, get_workspace_dict_path};

/// Configuration for hunspell-lsp
#[derive(Debug, Clone)]
pub struct Config {
    /// Path to user dictionary
    pub user_dict_path: PathBuf,
    /// Path to workspace dictionary
    pub workspace_dict_path: PathBuf,
    /// Whether to enable Harper integration for English
    pub enable_harper: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            user_dict_path: get_user_dict_path(),
            workspace_dict_path: PathBuf::from(".hunspell-dictionary.txt"),
            enable_harper: true,
        }
    }
}

impl Config {
    /// Creates configuration from LSP initialization options.
    ///
    /// # Arguments
    ///
    /// * `workspace_uri` - Optional workspace root URI as string
    /// * `init_options` - Optional initialization options from LSP client
    ///
    /// # Returns
    ///
    /// * `Ok(Config)` - Configuration with applied settings
    /// * `Err` - If workspace path cannot be determined
    pub fn from_lsp_config(
        workspace_uri: Option<&str>,
        init_options: Option<&serde_json::Value>,
    ) -> Result<Self> {
        let mut config = Self::default();

        // Set workspace dictionary path if workspace URI is provided
        if let Some(uri_str) = workspace_uri {
            if let Ok(uri) = Url::parse(uri_str) {
                if let Ok(path) = uri.to_file_path() {
                    config.workspace_dict_path = get_workspace_dict_path(&path);
                }
            }
        }

        // Apply initialization options if provided
        if let Some(options) = init_options {
            if let Some(user_dict_path) = options.get("userDictionaryPath") {
                if let Some(path_str) = user_dict_path.as_str() {
                    config.user_dict_path = PathBuf::from(path_str);
                }
            }

            if let Some(enable_harper) = options.get("enableHarper") {
                if let Some(enabled) = enable_harper.as_bool() {
                    config.enable_harper = enabled;
                }
            }
        }

        Ok(config)
    }

    /// Gets the workspace root path from a document URI string.
    ///
    /// # Arguments
    ///
    /// * `document_uri` - URI string of the current document
    ///
    /// # Returns
    ///
    /// * `Some(PathBuf)` - Workspace root path if found
    /// * `None` - If workspace cannot be determined
    pub fn get_workspace_root(document_uri: &str) -> Option<PathBuf> {
        if let Ok(uri) = Url::parse(document_uri) {
            if let Ok(path) = uri.to_file_path() {
                // Start from the document directory and search upward
                let mut current: &Path = path.parent()?;

            // Search up the directory tree for workspace indicators
            // Common workspace indicators: .git, Cargo.toml, package.json, etc.
            const MAX_DEPTH: usize = 10;
            for _ in 0..MAX_DEPTH {
                // Check for common workspace indicators
                let workspace_indicators = [
                    ".git",
                    ".hg",
                    ".svn",
                    "Cargo.toml",
                    "package.json",
                    "tsconfig.json",
                    "pyproject.toml",
                    "go.mod",
                ];

                for indicator in &workspace_indicators {
                    let indicator_path = current.join(indicator);
                    if indicator_path.exists() {
                        return Some(current.to_path_buf());
                    }
                }

                // Move up one directory
                match current.parent() {
                    Some(parent) => current = parent,
                    None => break,
                }
            }

                // If no workspace found, use the document's parent directory
                path.parent().map(|p: &Path| p.to_path_buf())
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Creates configuration for a specific document.
    ///
    /// # Arguments
    ///
    /// * `document_uri` - URI string of the current document
    /// * `init_options` - Optional initialization options from LSP client
    ///
    /// # Returns
    ///
    /// * `Ok(Config)` - Configuration with workspace-specific settings
    /// * `Err` - If configuration cannot be created
    pub fn for_document(
        document_uri: &str,
        init_options: Option<&serde_json::Value>,
    ) -> Result<Self> {
        let workspace_root = Self::get_workspace_root(document_uri);

        // Convert workspace root to URI string
        let workspace_uri = workspace_root
            .and_then(|path| url::Url::from_file_path(path).ok())
            .as_ref()
            .map(url::Url::as_str)
            .map(|s| s.to_string());

        Self::from_lsp_config(workspace_uri.as_deref(), init_options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(config.user_dict_path.ends_with("user_dictionary.txt"));
        assert_eq!(config.workspace_dict_path, PathBuf::from(".hunspell-dictionary.txt"));
        assert!(config.enable_harper);
    }

    #[test]
    fn test_config_from_lsp_options() {
        let options = serde_json::json!({
            "userDictionaryPath": "/custom/path/dict.txt",
            "enableHarper": false
        });

        let config = Config::from_lsp_config(None, Some(&options)).unwrap();
        assert_eq!(config.user_dict_path, PathBuf::from("/custom/path/dict.txt"));
        assert!(!config.enable_harper);
    }

    #[test]
    fn test_workspace_root_detection() {
        // This test would require setting up a mock filesystem
        // For now, we just test that the function exists and returns None for invalid URIs
        assert!(Config::get_workspace_root("data:text/plain,test").is_none());
        assert!(Config::get_workspace_root("not-a-uri").is_none());
    }
}