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

    while let Ok(msg) = connection.receiver.recv() {
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req).unwrap() {
                    break;
                }

                // Handle code action requests
                if req.method == "textDocument/codeAction" {
                    let params: CodeActionParams = serde_json::from_value(req.params).unwrap();
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

                    connection.sender.send(Message::Response(response)).unwrap();
                }
            }

            Message::Notification(notif) => {
                if notif.method == "textDocument/didOpen"
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
                    let dict = load_dict(&lang);

                    let mut doc_state = DocumentState::new();
                    doc_state.text = text.clone();

                    let mut diagnostics = vec![];

                    if let Some(dict) = dict {
                        let word_re = Regex::new(r"\b[\w']+\b").unwrap();
                        for (line_idx, line) in text.lines().enumerate() {
                            for mat in word_re.find_iter(&line) {
                                let word = mat.as_str();
                                let clean = word.trim_matches(|c: char| !c.is_alphabetic());
                                if !clean.is_empty() && !should_ignore_word(clean, line) && dict.check(clean) != CheckResult::FoundInDictionary {
                                    let suggestions = dict.suggest(clean);

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
                                }
                            }
                        }
                    }

                    documents.insert(uri, doc_state);

                    let params = PublishDiagnosticsParams {
                        uri: params.text_document.uri,
                        diagnostics,
                        version: None,
                    };

                    connection
                        .sender
                        .send(Message::Notification(Notification {
                            method: "textDocument/publishDiagnostics".into(),
                            params: serde_json::to_value(params).unwrap(),
                        }))
                        .unwrap();
                }
            }

            _ => {}
        }
    }

    io_threads.join().unwrap();
}
