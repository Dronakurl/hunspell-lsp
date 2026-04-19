//! Harper language server client for communicating with Harper process.
//!
//! This module provides functionality to spawn and communicate with the Harper
//! language server for enhanced English spell checking.

use anyhow::{Context, Result};
use lsp_types::{Diagnostic, PublishDiagnosticsParams};
use lsp_server::{Message, Notification, Request};
use serde_json::json;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child as TokioChild, Command as TokioCommand};
use tokio::task::JoinHandle;

/// Harper language server client with async communication.
pub struct HarperClient {
    /// Child process handle
    child: TokioChild,
    /// Task handle for the communication task
    _task_handle: JoinHandle<Result<()>>,
    /// Channel sender for communicating with Harper
    tx: tokio::sync::mpsc::UnboundedSender<Message>,
    /// Channel receiver for getting diagnostics from background task
    diagnostics_rx: tokio::sync::mpsc::UnboundedReceiver<Vec<Diagnostic>>,
}

impl HarperClient {
    /// Creates a new Harper client by spawning the harper-ls process.
    ///
    /// # Returns
    ///
    /// * `Ok(HarperClient)` - If Harper was spawned successfully
    /// * `Err` - If Harper could not be started
    pub async fn new() -> Result<Self> {
        // Spawn harper-ls process
        let mut child = TokioCommand::new("harper-ls")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn harper-ls process")?;

        let stdin = child.stdin.take().context("Failed to open stdin")?;
        let stdout = child.stdout.take().context("Failed to open stdout")?;
        let stderr = child.stderr.take().context("Failed to open stderr")?;

        // Spawn task to handle stderr logging
        tokio::spawn(async move {
            let mut reader = tokio::io::BufReader::new(stderr);
            let mut line = String::new();
            while reader.read_line(&mut line).await.is_ok() {
                eprintln!("Harper stderr: {}", line.trim());
                line.clear();
            }
        });

        // Spawn task to handle communication with Harper
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Message>();
        let (diagnostics_tx, diagnostics_rx) = tokio::sync::mpsc::unbounded_channel::<Vec<Diagnostic>>();

        let task_handle = tokio::spawn(async move {
            let mut reader = tokio::io::BufReader::new(stdout);
            let mut writer = tokio::io::BufWriter::new(stdin);

            // Handle messages from channel and also listen for Harper responses
            loop {
                tokio::select! {
                    // Handle outgoing messages to Harper
                    msg = rx.recv() => {
                        match msg {
                            Some(Message::Request(req)) => {
                                if let Err(e) = send_lsp_message(&mut writer, &req).await {
                                    eprintln!("Error sending request to Harper: {}", e);
                                    break;
                                }
                                // Read response from Harper
                                match read_lsp_message(&mut reader).await {
                                    Ok(_resp) => {
                                        // Response handled, continue
                                    }
                                    Err(e) => {
                                        eprintln!("Error reading response from Harper: {}", e);
                                        break;
                                    }
                                }
                            }
                            Some(Message::Notification(notif)) => {
                                if let Err(e) = send_lsp_message(&mut writer, &notif).await {
                                    eprintln!("Error sending notification to Harper: {}", e);
                                    break;
                                }
                                // After sending a notification, check if Harper publishes diagnostics
                                match tokio::time::timeout(
                                    tokio::time::Duration::from_millis(500),
                                    read_lsp_message(&mut reader)
                                ).await {
                                    Ok(Ok(Some(Message::Notification(inner_notif)))) => {
                                        if inner_notif.method == "textDocument/publishDiagnostics" {
                                            if let Ok(params) = serde_json::from_value::<PublishDiagnosticsParams>(inner_notif.params) {
                                                let _ = diagnostics_tx.send(params.diagnostics);
                                            }
                                        }
                                    }
                                    _ => {
                                        // No diagnostics or timeout, continue
                                    }
                                }
                            }
                            Some(Message::Response(resp)) => {
                                if let Err(e) = send_lsp_message(&mut writer, &resp).await {
                                    eprintln!("Error sending response to Harper: {}", e);
                                    break;
                                }
                            }
                            None => {
                                // Channel closed, exit loop
                                break;
                            }
                        }
                    }
                }
            }
            Ok::<(), anyhow::Error>(())
        });

        // Initialize Harper
        let init_req = Request {
            id: 1.into(),
            method: "initialize".to_string(),
            params: json!({
                "processId": null,
                "rootUri": null,
                "capabilities": {},
                "workspaceFolders": null
            }),
        };

        tx.send(Message::Request(init_req))
            .map_err(|e| anyhow::anyhow!("Failed to send initialize request: {}", e))?;

        // Wait a bit for initialization
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Send initialized notification
        let init_notif = Notification {
            method: "initialized".to_string(),
            params: json!(null),
        };

        tx.send(Message::Notification(init_notif))
            .map_err(|e| anyhow::anyhow!("Failed to send initialized notification: {}", e))?;

        Ok(HarperClient {
            child,
            _task_handle: task_handle,
            tx,
            diagnostics_rx,
        })
    }

    /// Checks text for spelling issues using Harper.
    ///
    /// # Arguments
    ///
    /// * `text` - Text to check
    /// * `uri` - Document URI
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<Diagnostic>)` - List of diagnostics from Harper
    /// * `Err` - If the check failed
    pub async fn check_text(&mut self, text: &str, uri: &str) -> Result<Vec<Diagnostic>> {
        // First, send didClose notification to clear any previous state
        let did_close = Notification {
            method: "textDocument/didClose".to_string(),
            params: json!({
                "textDocument": {
                    "uri": uri
                }
            }),
        };

        self.send_message(Message::Notification(did_close))
            .await?;

        // Then send didOpen notification with the new text
        let did_open = Notification {
            method: "textDocument/didOpen".to_string(),
            params: json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": "markdown",
                    "version": 1,
                    "text": text
                }
            }),
        };

        self.send_message(Message::Notification(did_open))
            .await?;

        // Wait for diagnostics from Harper
        match tokio::time::timeout(
            tokio::time::Duration::from_secs(5),
            self.diagnostics_rx.recv()
        ).await {
            Ok(Some(diagnostics)) => Ok(diagnostics),
            Ok(None) => {
                eprintln!("Harper diagnostics channel closed");
                Ok(vec![])
            }
            Err(_) => {
                eprintln!("Timeout waiting for Harper diagnostics");
                Ok(vec![])
            }
        }
    }

    /// Send a message to the Harper process.
    async fn send_message(&self, message: Message) -> Result<()> {
        self.tx
            .send(message)
            .map_err(|e| anyhow::anyhow!("Failed to send message to Harper: {}", e))?;
        Ok(())
    }
}

/// Sends an LSP message with proper Content-Length header.
async fn send_lsp_message<W>(writer: &mut W, message: &impl serde::Serialize) -> Result<()>
where
    W: AsyncWriteExt + Unpin + std::marker::Unpin,
{
    let content = serde_json::to_string(message)?;
    let content_bytes = content.as_bytes();
    let header = format!("Content-Length: {}\r\n\r\n", content_bytes.len());

    writer.write_all(header.as_bytes()).await?;
    writer.write_all(content_bytes).await?;
    writer.flush().await?;

    Ok(())
}

/// Reads an LSP message with proper Content-Length header.
async fn read_lsp_message<R>(reader: &mut R) -> Result<Option<Message>>
where
    R: tokio::io::AsyncBufReadExt + Unpin,
{
    // Read Content-Length header line
    let mut header_line = String::new();
    let mut byte = [0u8; 1];

    loop {
        let n = reader.read(&mut byte[..]).await?;
        if n == 0 {
            return Ok(None);
        }
        if byte[0] == b'\n' {
            break;
        }
        header_line.push(byte[0] as char);
        if header_line.len() > 100 {
            return Ok(None); // Prevent infinite loop
        }
    }

    if !header_line.starts_with("Content-Length:") {
        return Ok(None);
    }

    let length: usize = header_line["Content-Length:".len()..]
        .trim()
        .parse()
        .context("Invalid Content-Length")?;

    // Read blank line (CRLF)
    let mut blank = [0u8; 2];
    reader.read_exact(&mut blank[..]).await?;

    // Read content
    let mut content = vec![0u8; length];
    reader.read_exact(&mut content[..]).await?;

    let content_str = String::from_utf8(content)?;
    let message = serde_json::from_str(&content_str)?;

    Ok(Some(message))
}

impl Drop for HarperClient {
    fn drop(&mut self) {
        // Send shutdown request to Harper
        let shutdown_req = Request {
            id: 2.into(),
            method: "shutdown".to_string(),
            params: json!(null),
        };

        let _ = tokio::runtime::Handle::try_current()
            .ok()
            .map(|handle| handle.block_on(async {
                let _ = self.send_message(Message::Request(shutdown_req)).await;

                // Send exit notification
                let exit_notif = Notification {
                    method: "exit".to_string(),
                    params: json!(null),
                };

                let _ = self.send_message(Message::Notification(exit_notif)).await;

                // Give Harper time to exit gracefully
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                // Kill the child process
                let _ = self.child.kill().await;
            }));
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
