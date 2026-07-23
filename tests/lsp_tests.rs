use tower_lsp::lsp_types::*;
use tower_lsp::LanguageServer;

use track::lsp::TrackLsp;

#[tokio::test]
async fn test_lsp_initialization() {
    let (service, _) = tower_lsp::LspService::new(|client| TrackLsp::new(client));
    let server = service.inner();

    let init_params = InitializeParams::default();
    let init_result = server.initialize(init_params).await.unwrap();

    assert!(init_result.capabilities.completion_provider.is_some());
    assert!(init_result.capabilities.hover_provider.is_some());
}

#[tokio::test]
async fn test_lsp_completion() {
    let (service, _) = tower_lsp::LspService::new(|client| TrackLsp::new(client));
    let server = service.inner();

    let uri = Url::parse("file:///test.trk").unwrap();
    let text = "fn main() -> void {\n    le\n}".to_string();

    server
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "track".to_string(),
                version: 1,
                text,
            },
        })
        .await;

    let completion_params = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position::new(1, 6),
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
        context: None,
    };

    let response = server.completion(completion_params).await.unwrap();
    assert!(response.is_some());
    if let Some(CompletionResponse::Array(items)) = response {
        assert!(items.iter().any(|item| item.label == "let"));
    } else {
        panic!("Expected array completion response");
    }
}

#[tokio::test]
async fn test_lsp_hover() {
    let (service, _) = tower_lsp::LspService::new(|client| TrackLsp::new(client));
    let server = service.inner();

    let uri = Url::parse("file:///test.trk").unwrap();
    let text = "fn main() -> void {\n    print(42);\n}".to_string();

    server
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "track".to_string(),
                version: 1,
                text,
            },
        })
        .await;

    let hover_params = HoverParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position::new(1, 6),
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
    };

    let response = server.hover(hover_params).await.unwrap();
    assert!(response.is_some());
    if let Some(hover) = response {
        if let HoverContents::Markup(markup) = hover.contents {
            assert!(markup.value.contains("Print a value to stdout"));
        } else {
            panic!("Expected markup hover contents");
        }
    }
}
