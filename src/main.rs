mod config;
mod dictionary_io;
mod harper_client;
mod harper_integration;

use config::{get_user_dict_path, get_workspace_dict_path};
use hunspell_lsp::is_english_lang;

use hunspell_lsp::{extract_lang, load_dict, should_ignore_word};
use hunspell_rs::CheckResult;
use lsp_server::{Connection, Message, Notification, Response};
use lsp_types::*;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SpellCheckerData {
    uri: String,
    word: String,
    suggestions: Vec<String>,
    range: Range,
}

struct DocumentState {
    text: String,
    diagnostics: HashMap<String, SpellCheckerData>, // key: diagnostic identifier
}

impl DocumentState {
    fn new() -> Self {
        Self {
            text: String::new(),
            diagnostics: HashMap::new(),
        }
    }
}

fn main() {
    // Create async runtime
    let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");

    // Run the LSP server within the async runtime
    rt.block_on(async {
        run_lsp_server().await;
    });
}

async fn run_lsp_server() {
    let (connection, io_threads) = Connection::stdio();

    let server_capabilities = serde_json::to_value(ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
        ..Default::default()
    })
    .unwrap();

    connection
        .initialize(server_capabilities)
        .expect("init failed");

    let mut documents: HashMap<String, DocumentState> = HashMap::new();
    let mut shutdown_requested = false;

    loop {
        let msg = match connection.receiver.recv() {
            Ok(msg) => msg,
            Err(_) => {
                // Connection closed, exit gracefully
                break;
            }
        };

        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req).unwrap() {
                    shutdown_requested = true;
                    continue;
                }

                if shutdown_requested {
                    // Reject all other requests after shutdown
                    let response = Response {
                        id: req.id,
                        result: None,
                        error: Some(lsp_server::ResponseError {
                            code: lsp_server::ErrorCode::ServerNotInitialized as i32,
                            message: "Server is shutting down".to_string(),
                            data: None,
                        }),
                    };
                    let _ = connection.sender.send(Message::Response(response));
                    continue;
                }

                // Handle code action requests
                if req.method == "textDocument/codeAction" {
                    let params: CodeActionParams = match serde_json::from_value(req.params) {
                        Ok(p) => p,
                        Err(e) => {
                            let response = Response {
                                id: req.id,
                                result: None,
                                error: Some(lsp_server::ResponseError {
                                    code: lsp_server::ErrorCode::InvalidRequest as i32,
                                    message: format!("Failed to parse CodeActionParams: {}", e),
                                    data: None,
                                }),
                            };
                            let _ = connection.sender.send(Message::Response(response));
                            continue;
                        }
                    };
                    let uri = params.text_document.uri.to_string();

                    let mut code_actions = vec![];

                    if let Some(doc_state) = documents.get(&uri) {
                        // Get the cursor line number
                        let cursor_line = params.range.start.line;

                        // Check if cursor is on a misspelled word
                        let mut cursor_on_misspelled = false;
                        for (_diag_id, spell_data) in &doc_state.diagnostics {
                            let ranges_intersect = spell_data.range.start.line
                                <= params.range.end.line
                                && spell_data.range.end.line >= params.range.start.line
                                && spell_data.range.start.character <= params.range.end.character
                                && spell_data.range.end.character >= params.range.start.character;

                            if ranges_intersect {
                                cursor_on_misspelled = true;
                                break;
                            }
                        }

                        if cursor_on_misspelled {
                            // Cursor is on a misspelled word - only show suggestions for this word
                            for (diag_id, spell_data) in &doc_state.diagnostics {
                                let ranges_intersect = spell_data.range.start.line
                                    <= params.range.end.line
                                    && spell_data.range.end.line >= params.range.start.line
                                    && spell_data.range.start.character
                                        <= params.range.end.character
                                    && spell_data.range.end.character
                                        >= params.range.start.character;

                                if ranges_intersect {
                                    // Create code actions for each suggestion
                                    for suggestion in &spell_data.suggestions {
                                        let action = CodeAction {
                                            title: format!(
                                                // "Replace '{}' with '{}'",
                                                "--> '{}'",
                                                // spell_data.word, suggestion
                                                suggestion
                                            ),
                                            kind: Some(CodeActionKind::QUICKFIX),
                                            diagnostics: None,
                                            edit: Some(WorkspaceEdit {
                                                changes: Some(
                                                    vec![(
                                                        params.text_document.uri.clone(),
                                                        vec![TextEdit {
                                                            range: spell_data.range.clone(),
                                                            new_text: suggestion.clone(),
                                                        }],
                                                    )]
                                                    .into_iter()
                                                    .collect(),
                                                ),
                                                document_changes: None,
                                                change_annotations: None,
                                            }),
                                            command: None,
                                            is_preferred: None,
                                            disabled: None,
                                            data: Some(serde_json::to_value(diag_id).unwrap()),
                                        };
                                        code_actions.push(action);
                                    }

                                    // Add dictionary actions
                                    let user_dict_action = CodeAction {
                                        title: format!("Add '{}' to user dictionary", spell_data.word),
                                        kind: Some(CodeActionKind::QUICKFIX),
                                        diagnostics: None,
                                        edit: None,
                                        command: Some(Command {
                                            title: format!("Add '{}' to user dictionary", spell_data.word),
                                            command: "hunspell-lsp.addToUserDict".to_string(),
                                            arguments: Some(vec![
                                                serde_json::to_value(spell_data.word.clone()).unwrap(),
                                                serde_json::to_value(params.text_document.uri.to_string()).unwrap(),
                                            ]),
                                        }),
                                        is_preferred: None,
                                        disabled: None,
                                        data: None,
                                    };
                                    code_actions.push(user_dict_action);

                                    let workspace_dict_action = CodeAction {
                                        title: format!("Add '{}' to workspace dictionary", spell_data.word),
                                        kind: Some(CodeActionKind::QUICKFIX),
                                        diagnostics: None,
                                        edit: None,
                                        command: Some(Command {
                                            title: format!("Add '{}' to workspace dictionary", spell_data.word),
                                            command: "hunspell-lsp.addToWorkspaceDict".to_string(),
                                            arguments: Some(vec![
                                                serde_json::to_value(spell_data.word.clone()).unwrap(),
                                                serde_json::to_value(params.text_document.uri.to_string()).unwrap(),
                                            ]),
                                        }),
                                        is_preferred: None,
                                        disabled: None,
                                        data: None,
                                    };
                                    code_actions.push(workspace_dict_action);
                                }
                            }
                        } else {
                            // Cursor is not on a misspelled word - show all misspelled words in the line
                            for (diag_id, spell_data) in &doc_state.diagnostics {
                                // Only include diagnostics on the same line as the cursor
                                if spell_data.range.start.line == cursor_line {
                                    // Create code actions for each suggestion
                                    for suggestion in &spell_data.suggestions {
                                        let action = CodeAction {
                                            title: format!(
                                                "'{}' --> '{}'",
                                                spell_data.word,
                                                // spell_data.range.start.line + 1,
                                                // spell_data.range.start.character + 1,
                                                suggestion
                                            ),
                                            kind: Some(CodeActionKind::QUICKFIX),
                                            diagnostics: None,
                                            edit: Some(WorkspaceEdit {
                                                changes: Some(
                                                    vec![(
                                                        params.text_document.uri.clone(),
                                                        vec![TextEdit {
                                                            range: spell_data.range.clone(),
                                                            new_text: suggestion.clone(),
                                                        }],
                                                    )]
                                                    .into_iter()
                                                    .collect(),
                                                ),
                                                document_changes: None,
                                                change_annotations: None,
                                            }),
                                            command: None,
                                            is_preferred: None,
                                            disabled: None,
                                            data: Some(serde_json::to_value(diag_id).unwrap()),
                                        };
                                        code_actions.push(action);
                                    }

                                    // Add dictionary actions
                                    let user_dict_action = CodeAction {
                                        title: format!("Add '{}' to user dictionary", spell_data.word),
                                        kind: Some(CodeActionKind::QUICKFIX),
                                        diagnostics: None,
                                        edit: None,
                                        command: Some(Command {
                                            title: format!("Add '{}' to user dictionary", spell_data.word),
                                            command: "hunspell-lsp.addToUserDict".to_string(),
                                            arguments: Some(vec![
                                                serde_json::to_value(spell_data.word.clone()).unwrap(),
                                                serde_json::to_value(params.text_document.uri.to_string()).unwrap(),
                                            ]),
                                        }),
                                        is_preferred: None,
                                        disabled: None,
                                        data: None,
                                    };
                                    code_actions.push(user_dict_action);

                                    let workspace_dict_action = CodeAction {
                                        title: format!("Add '{}' to workspace dictionary", spell_data.word),
                                        kind: Some(CodeActionKind::QUICKFIX),
                                        diagnostics: None,
                                        edit: None,
                                        command: Some(Command {
                                            title: format!("Add '{}' to workspace dictionary", spell_data.word),
                                            command: "hunspell-lsp.addToWorkspaceDict".to_string(),
                                            arguments: Some(vec![
                                                serde_json::to_value(spell_data.word.clone()).unwrap(),
                                                serde_json::to_value(params.text_document.uri.to_string()).unwrap(),
                                            ]),
                                        }),
                                        is_preferred: None,
                                        disabled: None,
                                        data: None,
                                    };
                                    code_actions.push(workspace_dict_action);
                                }
                            }
                        }
                    }

                    let result = serde_json::to_value(&code_actions).unwrap();
                    let response = Response {
                        id: req.id,
                        result: Some(result),
                        error: None,
                    };

                    let _ = connection.sender.send(Message::Response(response));
                } else if req.method == "hunspell-lsp.addToUserDict" {
                    // Handle adding word to user dictionary
                    handle_add_to_dictionary(&connection, req, &get_user_dict_path()).await;
                } else if req.method == "hunspell-lsp.addToWorkspaceDict" {
                    // Handle adding word to workspace dictionary
                    let workspace_uri = params_for_add_to_dict(&req);
                    let workspace_path = workspace_uri
                        .and_then(|uri| config::Config::get_workspace_root(&uri))
                        .unwrap_or_else(|| std::path::PathBuf::from("."));

                    let workspace_dict_path = get_workspace_dict_path(&workspace_path);
                    handle_add_to_dictionary(&connection, req, &workspace_dict_path).await;
                } else {
                    // Respond to unsupported requests with MethodNotFound error
                    let response = Response {
                        id: req.id,
                        result: None,
                        error: Some(lsp_server::ResponseError {
                            code: lsp_server::ErrorCode::MethodNotFound as i32,
                            message: format!("Method '{}' not supported", req.method),
                            data: None,
                        }),
                    };
                    let _ = connection.sender.send(Message::Response(response));
                }
            }

            Message::Notification(notif) => {
                if notif.method == "exit" {
                    if shutdown_requested {
                        break;
                    } else {
                        // Exit notification without shutdown request - exit with error
                        std::process::exit(1);
                    }
                } else if notif.method == "textDocument/didOpen"
                    || notif.method == "textDocument/didChange"
                {
                    let params = if notif.method == "textDocument/didOpen" {
                        let open: DidOpenTextDocumentParams =
                            serde_json::from_value(notif.params).unwrap();
                        DidChangeTextDocumentParams {
                            text_document: VersionedTextDocumentIdentifier {
                                uri: open.text_document.uri.clone(),
                                version: 1,
                            },
                            content_changes: vec![TextDocumentContentChangeEvent {
                                range: None,
                                range_length: None,
                                text: open.text_document.text.clone(),
                            }],
                        }
                    } else {
                        serde_json::from_value(notif.params).unwrap()
                    };

                    let uri = params.text_document.uri.to_string();
                    let text = params.content_changes[0].text.clone();

                    let lang = extract_lang(&text).unwrap_or("en_US".into());

                    // Check if we should use Harper for English
                    if is_english_lang(&lang) && harper_integration::is_harper_available() {
                        // Route to Harper for enhanced English checking (placeholder)
                        eprintln!("Routing English document to Harper: {} (full integration coming soon)", lang);
                        if let Err(e) = handle_harper_check(&connection, &text, &uri).await {
                            eprintln!("Harper placeholder failed, using Hunspell fallback: {}", e);
                            // Continue with Hunspell processing below
                        } else {
                            // Skip Hunspell processing if Harper succeeded (placeholder always returns Ok)
                            continue;
                        }
                    }

                    let dict = load_dict(&lang);

                    let mut doc_state = DocumentState::new();
                    doc_state.text = text.clone();

                    let mut diagnostics = vec![];

                    if let Some(dict) = dict {
                        let word_re = Regex::new(r"\b[\w']+\b").unwrap();
                        let mut suggestion_cache: HashMap<String, Vec<String>> = HashMap::new();
                        const MAX_DIAGNOSTICS: usize = 1000; // Limit total diagnostics for performance
                        let mut diag_count = 0;

                        for (line_idx, line) in text.lines().enumerate() {
                            for mat in word_re.find_iter(&line) {
                                let word = mat.as_str();
                                let clean = word.trim_matches(|c: char| !c.is_alphabetic());
                                if !clean.is_empty() && !should_ignore_word(clean, line) && dict.check(clean) != CheckResult::FoundInDictionary {
                                    // Use cached suggestions if available, otherwise get new ones
                                    let suggestions = if let Some(cached) = suggestion_cache.get(clean) {
                                        cached.clone()
                                    } else {
                                        let sugg = dict.suggest(clean);
                                        // Limit to 10 suggestions per word for performance
                                        let limited: Vec<String> = sugg.into_iter().take(10).collect();
                                        suggestion_cache.insert(clean.to_string(), limited.clone());
                                        limited
                                    };

                                    // Convert byte positions to character positions for UTF-8 support
                                    let word_start = line[..mat.start()].chars().count();
                                    let word_end = word_start + clean.chars().count();

                                    let diag_id = format!("{}:{}:{}", uri, line_idx, word_start);

                                    let spell_data = SpellCheckerData {
                                        uri: uri.clone(),
                                        word: clean.to_string(),
                                        suggestions: suggestions.clone(),
                                        range: Range {
                                            start: Position {
                                                line: line_idx as u32,
                                                character: word_start as u32,
                                            },
                                            end: Position {
                                                line: line_idx as u32,
                                                character: word_end as u32,
                                            },
                                        },
                                    };

                                    doc_state.diagnostics.insert(diag_id.clone(), spell_data);

                                    let message = if suggestions.is_empty() {
                                        format!("No suggestions for: {}", clean)
                                    } else {
                                        format!("Typo: {}", suggestions.join(", "))
                                    };

                                    diagnostics.push(Diagnostic {
                                        range: Range {
                                            start: Position {
                                                line: line_idx as u32,
                                                character: word_start as u32,
                                            },
                                            end: Position {
                                                line: line_idx as u32,
                                                character: word_end as u32,
                                            },
                                        },
                                        severity: Some(DiagnosticSeverity::HINT),
                                        message,
                                        data: Some(serde_json::to_value(diag_id).unwrap()),
                                        ..Default::default()
                                    });

                                    diag_count += 1;
                                    if diag_count >= MAX_DIAGNOSTICS {
                                        break; // Stop processing after max diagnostics reached
                                    }
                                }
                            }
                            if diag_count >= MAX_DIAGNOSTICS {
                                break; // Stop processing lines after max diagnostics reached
                            }
                        }
                    }

                    documents.insert(uri, doc_state);

                    let params = PublishDiagnosticsParams {
                        uri: params.text_document.uri,
                        diagnostics,
                        version: None,
                    };

                    let _ = connection
                        .sender
                        .send(Message::Notification(Notification {
                            method: "textDocument/publishDiagnostics".into(),
                            params: serde_json::to_value(params).unwrap(),
                        }));
                }
            }

            _ => {}
        }
    }

    // Drop connection before joining threads to avoid blocking
    drop(connection);
    let _ = io_threads.join();
}

/// Extract parameters from a dictionary add command request.
fn params_for_add_to_dict(req: &lsp_server::Request) -> Option<String> {
    if let Ok(params) = serde_json::from_value::<serde_json::Value>(req.params.clone()) {
        if let Some(arr) = params.as_array() {
            if arr.len() >= 2 {
                if let Some(uri_str) = arr[1].as_str() {
                    return Some(uri_str.to_string());
                }
            }
        }
    }
    None
}

/// Handle adding a word to a dictionary.
async fn handle_add_to_dictionary(connection: &Connection, req: lsp_server::Request, dict_path: &std::path::Path) {
    let word_and_uri = match serde_json::from_value::<serde_json::Value>(req.params.clone()) {
        Ok(params) => params,
        Err(_) => {
            let response = Response {
                id: req.id,
                result: None,
                error: Some(lsp_server::ResponseError {
                    code: lsp_server::ErrorCode::InvalidRequest as i32,
                    message: "Invalid parameters for addToDictionary".to_string(),
                    data: None,
                }),
            };
            let _ = connection.sender.send(Message::Response(response));
            return;
        }
    };

    let arr = match word_and_uri.as_array() {
        Some(arr) if arr.len() >= 2 => arr,
        _ => {
            let response = Response {
                id: req.id,
                result: None,
                error: Some(lsp_server::ResponseError {
                    code: lsp_server::ErrorCode::InvalidRequest as i32,
                    message: "Expected [word, uri] parameters".to_string(),
                    data: None,
                }),
            };
            let _ = connection.sender.send(Message::Response(response));
            return;
        }
    };

    let word = match arr[0].as_str() {
        Some(w) => w,
        None => {
            let response = Response {
                id: req.id,
                result: None,
                error: Some(lsp_server::ResponseError {
                    code: lsp_server::ErrorCode::InvalidRequest as i32,
                    message: "Word parameter must be a string".to_string(),
                    data: None,
                }),
            };
            let _ = connection.sender.send(Message::Response(response));
            return;
        }
    };

    let uri = match arr[1].as_str() {
        Some(u) => u,
        None => {
            let response = Response {
                id: req.id,
                result: None,
                error: Some(lsp_server::ResponseError {
                    code: lsp_server::ErrorCode::InvalidRequest as i32,
                    message: "URI parameter must be a string".to_string(),
                    data: None,
                }),
            };
            let _ = connection.sender.send(Message::Response(response));
            return;
        }
    };

    // Add word to dictionary
    match dictionary_io::add_word_to_dict(dict_path, word).await {
        Ok(true) => {
            // Word was added successfully - just send success response
            // Note: In a full implementation, we would trigger diagnostic refresh here
            let response = Response {
                id: req.id,
                result: Some(serde_json::to_value(true).unwrap()),
                error: None,
            };
            let _ = connection.sender.send(Message::Response(response));

            // Note: In a real implementation, you would trigger a diagnostic refresh here
            // This would involve re-publishing diagnostics for the affected document
        }
        Ok(false) => {
            // Word already exists in dictionary
            let response = Response {
                id: req.id,
                result: Some(serde_json::to_value(false).unwrap()),
                error: None,
            };
            let _ = connection.sender.send(Message::Response(response));
        }
        Err(e) => {
            // Error adding word
            eprintln!("Error adding word to dictionary: {}", e);
            let response = Response {
                id: req.id,
                result: None,
                error: Some(lsp_server::ResponseError {
                    code: lsp_server::ErrorCode::InternalError as i32,
                    message: format!("Failed to add word to dictionary: {}", e),
                    data: None,
                }),
            };
            let _ = connection.sender.send(Message::Response(response));
        }
    }
}

/// Handle spell checking using Harper language server (placeholder).
async fn handle_harper_check(
    _connection: &Connection,
    _text: &str,
    uri: &str,
) -> anyhow::Result<()> {
    // For now, this is a placeholder that demonstrates the routing logic
    // The full Harper integration will be implemented in a future update
    eprintln!("Would use Harper for document: {} (full integration coming soon)", uri);

    // Skip Harper processing for now, return Ok to continue with Hunspell fallback
    Ok(())
}
