use hunspell_lsp::{extract_lang, load_dict};
use lsp_server::{Connection, Message, Notification};
use lsp_types::*;

fn main() {
    let (connection, io_threads) = Connection::stdio();

    let server_capabilities = serde_json::to_value(ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        ..Default::default()
    })
    .unwrap();

    connection
        .initialize(server_capabilities)
        .expect("init failed");

    while let Ok(msg) = connection.receiver.recv() {
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req).unwrap() {
                    break;
                }
            }

            Message::Notification(notif) => {
                if notif.method == "textDocument/didOpen"
                    || notif.method == "textDocument/didChange"
                {
                    let params = if notif.method == "textDocument/didOpen" {
                        let open: DidOpenTextDocumentParams = serde_json::from_value(notif.params).unwrap();
                        DidChangeTextDocumentParams {
                            text_document: VersionedTextDocumentIdentifier {
                                uri: open.text_document.uri,
                                version: 1,
                            },
                            content_changes: vec![TextDocumentContentChangeEvent {
                                range: None,
                                range_length: None,
                                text: open.text_document.text,
                            }],
                        }
                    } else {
                        serde_json::from_value(notif.params).unwrap()
                    };

                    let uri = params.text_document.uri;
                    let text = &params.content_changes[0].text;

                    let lang = extract_lang(text).unwrap_or("en_US".into());
                    let dict = load_dict(&lang);

                    let mut diagnostics = vec![];

                    if let Some(dict) = dict {
                        for (line_idx, line) in text.lines().enumerate() {
                            for (col_idx, word) in line.split_whitespace().enumerate() {
                                let clean = word.trim_matches(|c: char| !c.is_alphabetic());
                                if !clean.is_empty() && !dict.check(clean) {
                                    diagnostics.push(Diagnostic {
                                        range: Range {
                                            start: Position {
                                                line: line_idx as u32,
                                                character: col_idx as u32,
                                            },
                                            end: Position {
                                                line: line_idx as u32,
                                                character: (col_idx + clean.len()) as u32,
                                            },
                                        },
                                        severity: Some(DiagnosticSeverity::WARNING),
                                        message: format!("Possibly misspelled: {}", clean),
                                        ..Default::default()
                                    });
                                }
                            }
                        }
                    }

                    let params = PublishDiagnosticsParams {
                        uri,
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
