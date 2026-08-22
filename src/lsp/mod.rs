use std::collections::HashMap;
use std::sync::Mutex;

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use crate::checker::LinearChecker;
use crate::lexer::Lexer;
use crate::parser::Parser;

pub struct TrackLsp {
    client: Client,
    documents: Mutex<HashMap<Url, String>>,
    ast_cache: Mutex<HashMap<Url, Vec<crate::ast::Expr>>>,
    doc_revisions: Mutex<HashMap<Url, usize>>,
}

impl TrackLsp {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: Mutex::new(HashMap::new()),
            ast_cache: Mutex::new(HashMap::new()),
            doc_revisions: Mutex::new(HashMap::new()),
        }
    }

    async fn analyze_document_async(&self, uri: Url, text: String, rev: usize) -> Vec<Diagnostic> {
        let uri_clone = uri.clone();
        let (ast_opt, diagnostics) = tokio::task::spawn_blocking(move || {
            if uri_clone.path().ends_with(".md") || uri_clone.path().ends_with(".markdown") {
                let mut all_diagnostics = Vec::new();
                let blocks = Self::extract_track_blocks_static(&text);

                for (range, block_source) in blocks {
                    let (_, block_diagnostics) = Self::analyze_source_static(&block_source);
                    for mut diag in block_diagnostics {
                        diag.range.start.line += range.start.line;
                        diag.range.end.line += range.start.line;
                        all_diagnostics.push(diag);
                    }
                }
                (None, all_diagnostics)
            } else if uri_clone.path().ends_with(".trk") {
                Self::analyze_source_static(&text)
            } else {
                (None, Vec::new())
            }
        })
        .await
        .unwrap_or((None, Vec::new()));

        if self.doc_revisions.lock().unwrap().get(&uri) != Some(&rev) {
            return Vec::new(); // Superseded by newer edit
        }

        if let Some(ast) = ast_opt {
            self.ast_cache.lock().unwrap().insert(uri, ast);
        }

        diagnostics
    }

    fn analyze_source_static(source: &str) -> (Option<Vec<crate::ast::Expr>>, Vec<Diagnostic>) {
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
                return (None, diagnostics);
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
                return (None, diagnostics);
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

        (Some(program), diagnostics)
    }

    fn extract_track_blocks_static(markdown: &str) -> Vec<(Range, String)> {
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

        let rev = {
            let mut map = self.doc_revisions.lock().unwrap();
            let entry = map.entry(uri.clone()).or_insert(0);
            *entry += 1;
            *entry
        };

        let diagnostics = self.analyze_document_async(uri.clone(), text, rev).await;

        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params
            .content_changes
            .into_iter()
            .next()
            .map(|c| c.text)
            .unwrap_or_default();

        self.documents
            .lock()
            .unwrap()
            .insert(uri.clone(), text.clone());

        let rev = {
            let mut map = self.doc_revisions.lock().unwrap();
            let entry = map.entry(uri.clone()).or_insert(0);
            *entry += 1;
            *entry
        };

        let diagnostics = self.analyze_document_async(uri.clone(), text, rev).await;

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

            let rev = {
                let mut map = self.doc_revisions.lock().unwrap();
                let entry = map.entry(uri.clone()).or_insert(0);
                *entry += 1;
                *entry
            };

            let diagnostics = self.analyze_document_async(uri.clone(), text, rev).await;

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
        let word_start = before_cursor
            .rfind(|c: char| !c.is_alphanumeric() && c != '_')
            .map(|i| i + 1)
            .unwrap_or(0);
        let word = &before_cursor[word_start..];

        if word.is_empty() {
            return Ok(None);
        }

        let mut completions = Vec::new();

        // Keywords
        let keywords = vec![
            "import", "use", "let", "mut", "fn", "return", "if", "else", "while", "for", "in",
            "struct", "enum", "union", "match", "with", "const", "type", "true", "false", "as",
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
        let types = vec!["i8", "u8", "i32", "u32", "i64", "u64", "bool", "void", "ptr"];

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
        let builtins = vec![
            "print",
            "println",
            "eprint",
            "read",
            "file_open",
            "file_close",
            "file_exists",
            "dir_exists",
            "file_copy",
            "clock_ms",
            "exit",
            "alloc",
            "dealloc",
            "env_get",
            "os_args_count",
            "os_arg",
            "process_spawn",
            "sys_exec",
            "sys_set_memory_limit",
            "sys_get_memory_used",
            "str_starts_with",
            "str_ends_with",
            "str_contains",
            "abort",
        ];

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
        let macros = vec!["macro", "now", "compile_error"];

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

        // Enum/Union variants from cached AST
        let cached_program = self.ast_cache.lock().unwrap().get(uri).cloned();
        let program = match cached_program {
            Some(p) => p,
            None => {
                let tokens = Lexer::tokenize(&text).unwrap_or_default();
                let mut parser = Parser::new(tokens, text.clone());
                parser.parse_program().unwrap_or_default()
            }
        };

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
                    crate::ast::Expr::FnDef { name, .. } if name.starts_with(word) => {
                        completions.push(CompletionItem {
                            label: name.clone(),
                            kind: Some(CompletionItemKind::FUNCTION),
                            detail: Some("User-defined function".to_string()),
                            ..Default::default()
                        });
                    }
                    _ => {}
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
        let word_start = before_cursor
            .rfind(|c: char| !c.is_alphanumeric() && c != '_')
            .map(|i| i + 1)
            .unwrap_or(0);
        let word_end = after_cursor
            .find(|c: char| !c.is_alphanumeric() && c != '_')
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
            ("for", "For-in loop\n\n```track\nfor i in 0..10 {\n    print(i);\n}\n```"),
            ("in", "Iteration in a for-in loop"),
            ("use", "Import items from a module (synonym for `import`)"),
            ("type", "Define a type alias\n\n```track\ntype Matrix = [i32; 16];\n```"),
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
            ("i8", "8-bit signed integer (copy type)"),
            ("u8", "8-bit unsigned integer (copy type)"),
            ("i32", "32-bit signed integer (copy type)"),
            ("u32", "32-bit unsigned integer (copy type)"),
            ("i64", "64-bit signed integer (copy type)"),
            ("u64", "64-bit unsigned integer (copy type)"),
            ("bool", "Boolean (copy type)"),
            ("void", "Unit type (copy type)"),
            (
                "ptr",
                "Raw pointer (copy type)\n\n```track\nlet p: ptr<i32>;\n```",
            ),
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
            (
                "print",
                "Print a value to stdout\n\n```track\nprint(42);\nprint(\"hello\");\n```",
            ),
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
            (
                "@macro",
                "Define a compile-time macro\n\n```track\n@macro double(n: i32) -> i32 {\n    return n * 2;\n}\n```",
            ),
            ("@compile_error", "Trigger a compile-time error"),
            ("@now", "Get current timestamp"),
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

    let (service, messages) = LspService::new(TrackLsp::new);
    Server::new(stdin, stdout, messages).serve(service).await;
}
