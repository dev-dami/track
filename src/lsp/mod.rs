use std::collections::HashMap;
use std::sync::Mutex;

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::checker::LinearChecker;

pub struct TrackLsp {
    client: Client,
    documents: Mutex<HashMap<Url, String>>,
}

impl TrackLsp {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: Mutex::new(HashMap::new()),
        }
    }

    fn analyze_source(&self, source: &str, _uri: &Url) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Try to tokenize
        let tokens = match Lexer::tokenize(source) {
            Ok(t) => t,
            Err(e) => {
                diagnostics.push(Diagnostic {
                    range: Range::new(Position::new(0, 0), Position::new(0, 0)),
                    severity: Some(DiagnosticSeverity::ERROR),
                    message: e,
                    ..Default::default()
                });
                return diagnostics;
            }
        };

        // Try to parse
        let mut parser = Parser::new(tokens, source.to_string());
        let program = match parser.parse_program() {
            Ok(p) => p,
            Err(e) => {
                diagnostics.push(Diagnostic {
                    range: Range::new(Position::new(0, 0), Position::new(0, 0)),
                    severity: Some(DiagnosticSeverity::ERROR),
                    message: e,
                    ..Default::default()
                });
                return diagnostics;
            }
        };

        // Try to type check
        let mut checker = LinearChecker::new();
        if let Err(e) = checker.check_program(&program) {
            diagnostics.push(Diagnostic {
                range: Range::new(Position::new(0, 0), Position::new(0, 0)),
                severity: Some(DiagnosticSeverity::ERROR),
                message: e,
                ..Default::default()
            });
        }

        diagnostics
    }

    fn extract_track_blocks(&self, markdown: &str) -> Vec<(Range, String)> {
        let mut blocks = Vec::new();
        let mut in_block = false;
        let mut block_start_line = 0;
        let mut block_content = String::new();

        for (line_idx, line) in markdown.lines().enumerate() {
            let trimmed = line.trim();

            if trimmed.starts_with("```track") || trimmed.starts_with("```trk") {
                in_block = true;
                block_start_line = line_idx + 1;
                block_content.clear();
            } else if trimmed == "```" && in_block {
                in_block = false;
                let range = Range::new(
                    Position::new(block_start_line as u32, 0),
                    Position::new(line_idx as u32, 0),
                );
                blocks.push((range, block_content.clone()));
            } else if in_block {
                if !block_content.is_empty() {
                    block_content.push('\n');
                }
                block_content.push_str(line);
            }
        }

        blocks
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for TrackLsp {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![".".to_string(), ":".to_string()]),
                    ..Default::default()
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Track LSP server initialized")
            .await;
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;

        self.documents
            .lock()
            .unwrap()
            .insert(uri.clone(), text.clone());

        let diagnostics = if uri.path().ends_with(".md") || uri.path().ends_with(".markdown") {
            let mut all_diagnostics = Vec::new();
            let blocks = self.extract_track_blocks(&text);

            for (range, block_source) in blocks {
                let block_diagnostics = self.analyze_source(&block_source, &uri);
                for mut diag in block_diagnostics {
                    // Offset the range by the block's start line
                    diag.range.start.line += range.start.line;
                    diag.range.end.line += range.start.line;
                    all_diagnostics.push(diag);
                }
            }
            all_diagnostics
        } else if uri.path().ends_with(".trk") {
            self.analyze_source(&text, &uri)
        } else {
            Vec::new()
        };

        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.content_changes.into_iter().next().map(|c| c.text).unwrap_or_default();

        self.documents
            .lock()
            .unwrap()
            .insert(uri.clone(), text.clone());

        let diagnostics = if uri.path().ends_with(".md") || uri.path().ends_with(".markdown") {
            let mut all_diagnostics = Vec::new();
            let blocks = self.extract_track_blocks(&text);

            for (range, block_source) in blocks {
                let block_diagnostics = self.analyze_source(&block_source, &uri);
                for mut diag in block_diagnostics {
                    diag.range.start.line += range.start.line;
                    diag.range.end.line += range.start.line;
                    all_diagnostics.push(diag);
                }
            }
            all_diagnostics
        } else if uri.path().ends_with(".trk") {
            self.analyze_source(&text, &uri)
        } else {
            Vec::new()
        };

        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri;
        if let Some(text) = params.text {
            self.documents
                .lock()
                .unwrap()
                .insert(uri.clone(), text.clone());

            let diagnostics = if uri.path().ends_with(".md") || uri.path().ends_with(".markdown") {
                let mut all_diagnostics = Vec::new();
                let blocks = self.extract_track_blocks(&text);

                for (range, block_source) in blocks {
                    let block_diagnostics = self.analyze_source(&block_source, &uri);
                    for mut diag in block_diagnostics {
                        diag.range.start.line += range.start.line;
                        diag.range.end.line += range.start.line;
                        all_diagnostics.push(diag);
                    }
                }
                all_diagnostics
            } else if uri.path().ends_with(".trk") {
                self.analyze_source(&text, &uri)
            } else {
                Vec::new()
            };

            self.client
                .publish_diagnostics(uri, diagnostics, None)
                .await;
        }
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = &params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        let documents = self.documents.lock().unwrap();
        let text = match documents.get(uri) {
            Some(t) => t.clone(),
            None => return Ok(None),
        };
        drop(documents);

        let lines: Vec<&str> = text.lines().collect();
        if position.line as usize >= lines.len() {
            return Ok(None);
        }

        let line = lines[position.line as usize];
        let cursor_pos = position.character as usize;

        // Get the word being typed
        let before_cursor = &line[..cursor_pos.min(line.len())];
        let word_start = before_cursor.rfind(|c: char| !c.is_alphanumeric() && c != '_')
            .map(|i| i + 1)
            .unwrap_or(0);
        let word = &before_cursor[word_start..];

        if word.is_empty() {
            return Ok(None);
        }

        let mut completions = Vec::new();

        // Keywords
        let keywords = vec![
            "let", "mut", "fn", "return", "if", "else", "while",
            "struct", "enum", "union", "match", "with", "const",
            "true", "false", "as",
        ];

        for kw in keywords {
            if kw.starts_with(word) {
                completions.push(CompletionItem::new_simple(
                    kw.to_string(),
                    "Keyword".to_string(),
                ));
            }
        }

        // Types
        let types = vec!["i32", "u32", "i64", "u64", "bool", "void", "ptr"];

        for ty in types {
            if ty.starts_with(word) {
                completions.push(CompletionItem {
                    label: ty.to_string(),
                    kind: Some(CompletionItemKind::TYPE_PARAMETER),
                    detail: Some("Type".to_string()),
                    ..Default::default()
                });
            }
        }

        // Built-in functions
        let builtins = vec!["print", "read"];

        for b in builtins {
            if b.starts_with(word) {
                completions.push(CompletionItem {
                    label: b.to_string(),
                    kind: Some(CompletionItemKind::FUNCTION),
                    detail: Some("Built-in function".to_string()),
                    ..Default::default()
                });
            }
        }

        // Macros
        let macros = vec!["bit", "pin", "register", "compile_error", "now", "fib_comptime"];

        for m in macros {
            if m.starts_with(word) {
                completions.push(CompletionItem {
                    label: format!("@{}", m),
                    kind: Some(CompletionItemKind::FUNCTION),
                    detail: Some("Macro".to_string()),
                    ..Default::default()
                });
            }
        }

        // Enum/Union variants from source
        let tokens = Lexer::tokenize(&text).unwrap_or_default();
        let mut parser = Parser::new(tokens, text.clone());
        if let Ok(program) = parser.parse_program() {
            for stmt in &program {
                match stmt {
                    crate::ast::Expr::EnumDef { name, variants, .. } => {
                        for (variant, _) in variants {
                            let full = format!("{}::{}", name, variant);
                            if full.starts_with(word) {
                                completions.push(CompletionItem {
                                    label: full,
                                    kind: Some(CompletionItemKind::ENUM_MEMBER),
                                    detail: Some(format!("Enum variant of {}", name)),
                                    ..Default::default()
                                });
                            }
                        }
                    }
                    crate::ast::Expr::UnionDef { name, variants, .. } => {
                        for (variant, _) in variants {
                            let full = format!("{}::{}", name, variant);
                            if full.starts_with(word) {
                                completions.push(CompletionItem {
                                    label: full,
                                    kind: Some(CompletionItemKind::ENUM_MEMBER),
                                    detail: Some(format!("Union variant of {}", name)),
                                    ..Default::default()
                                });
                            }
                        }
                    }
                    crate::ast::Expr::FnDef { name, .. } => {
                        if name.starts_with(word) {
                            completions.push(CompletionItem {
                                label: name.clone(),
                                kind: Some(CompletionItemKind::FUNCTION),
                                detail: Some("User-defined function".to_string()),
                                ..Default::default()
                            });
                        }
                    }
                    _ => {}
                }
            }
        }


        Ok(Some(CompletionResponse::Array(completions)))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let documents = self.documents.lock().unwrap();
        let text = match documents.get(uri) {
            Some(t) => t.clone(),
            None => return Ok(None),
        };
        drop(documents);

        let lines: Vec<&str> = text.lines().collect();
        if position.line as usize >= lines.len() {
            return Ok(None);
        }

        let line = lines[position.line as usize];
        let cursor_pos = position.character as usize;

        // Get the word under cursor
        let before_cursor = &line[..cursor_pos.min(line.len())];
        let after_cursor = &line[cursor_pos..];
        let word_start = before_cursor.rfind(|c: char| !c.is_alphanumeric() && c != '_')
            .map(|i| i + 1)
            .unwrap_or(0);
        let word_end = after_cursor.find(|c: char| !c.is_alphanumeric() && c != '_')
            .map(|i| cursor_pos + i)
            .unwrap_or(line.len());
        let word = &line[word_start..word_end];

        if word.is_empty() {
            return Ok(None);
        }

        // Check keywords
        let keyword_docs = HashMap::from([
            ("let", "Declare a variable\n\n```track\nlet x = 42;\n```"),
            ("mut", "Declare a mutable variable\n\n```track\nlet mut x = 42;\n```"),
            ("fn", "Define a function\n\n```track\nfn add(a: i32, b: i32) -> i32 {\n    return a + b;\n}\n```"),
            ("return", "Return from function"),
            ("if", "Conditional expression"),
            ("else", "Else branch"),
            ("while", "Loop"),
            ("struct", "Define a struct"),
            ("enum", "Define an enum\n\n```track\nenum Color {\n    Red,\n    Green,\n    Blue,\n}\n```"),
            ("union", "Define a tagged union\n\n```track\nunion Value {\n    Int(i32),\n    Float(f64),\n}\n```"),
            ("match", "Pattern matching"),
            ("with", "Lexical lens block"),
            ("const", "Compile-time constant"),
            ("true", "Boolean true"),
            ("false", "Boolean false"),
            ("as", "Alias import"),
        ]);

        if let Some(doc) = keyword_docs.get(word) {
            return Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: doc.to_string(),
                }),
                range: None,
            }));
        }

        // Check types
        let type_docs = HashMap::from([
            ("i32", "32-bit signed integer (copy type)"),
            ("u32", "32-bit unsigned integer (copy type)"),
            ("i64", "64-bit signed integer (copy type)"),
            ("u64", "64-bit unsigned integer (copy type)"),
            ("bool", "Boolean (copy type)"),
            ("void", "Unit type (copy type)"),
            ("ptr", "Raw pointer (copy type)\n\n```track\nlet p: ptr<i32>;\n```"),
        ]);

        if let Some(doc) = type_docs.get(word) {
            return Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: doc.to_string(),
                }),
                range: None,
            }));
        }

        // Check built-ins
        let builtin_docs = HashMap::from([
            ("print", "Print a value to stdout\n\n```track\nprint(42);\nprint(\"hello\");\n```"),
            ("read", "Read input\n\n```track\nlet value = read();\n```"),
        ]);

        if let Some(doc) = builtin_docs.get(word) {
            return Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: doc.to_string(),
                }),
                range: None,
            }));
        }

        // Check macros
        let macro_docs = HashMap::from([
            ("@bit", "Create a bit mask\n\n```track\nlet pin = @bit(5);  // 1 << 5\n```"),
            ("@pin", "Create a pin identifier\n\n```track\nlet led = @pin(1, 5);  // (1 << 8) | 5\n```"),
            ("@register", "Create a register address\n\n```track\nlet reg = @register(0x4000, 0xFF);\n```"),
            ("@compile_error", "Trigger a compile-time error"),
            ("@now", "Get current timestamp"),
            ("@fib_comptime", "Compute Fibonacci at compile time"),
        ]);

        if let Some(doc) = macro_docs.get(word) {
            return Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: doc.to_string(),
                }),
                range: None,
            }));
        }

        Ok(None)
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

pub async fn start_server() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, messages) = LspService::new(|client| TrackLsp::new(client));
    Server::new(stdin, stdout, messages)
        .serve(service)
        .await;
}
