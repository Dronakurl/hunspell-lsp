//! Harper language server client stub for future enhancement.
//!
//! This module provides placeholder functionality for communicating with Harper.
//! The full async LSP client implementation will be added in a future update.

use anyhow::Result;

/// Harper language server client (stub implementation).
pub struct HarperClient;

impl HarperClient {
    /// Creates a new Harper client (placeholder).
    pub async fn new() -> Result<Self> {
        Ok(HarperClient)
    }

    /// Checks text for spelling issues using Harper (placeholder).
    pub async fn check_text(&self, _text: &str, _uri: &str) -> Result<Vec<lsp_types::Diagnostic>> {
        // Placeholder: Return empty diagnostics
        // In a full implementation, this would communicate with the Harper process
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_harper_client_creation() {
        // This test requires Harper to be installed
        if !crate::harper_integration::is_harper_available() {
            println!("Skipping test - Harper not installed");
            return;
        }

        match HarperClient::new().await {
            Ok(_client) => {
                println!("Harper client created successfully");
            }
            Err(e) => {
                println!("Failed to create Harper client: {}", e);
            }
        }
    }
}
