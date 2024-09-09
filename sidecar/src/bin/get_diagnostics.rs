use sidecar::{
    agentic::tool::{
        input::ToolInput,
        lsp::diagnostics::{LSPDiagnostics, LSPDiagnosticsInput},
        r#type::Tool,
    },
    chunking::text_document::{Position, Range},
};

#[tokio::main]
async fn main() {
    let path = "/Users/zi/codestory/sidecar/sidecar/src/agentic/symbol/events/input.rs";
    let range = Range::new(Position::new(28, 0, 0), Position::new(33, 1, 0));
    let editor_url = "http://localhost:42427".to_owned();

    let diagnostics_client = LSPDiagnostics::new();

    let lsp_diagnostics_input =
        LSPDiagnosticsInput::new(path.to_string(), range, editor_url.to_string());

    let _ = diagnostics_client
        .invoke(ToolInput::LSPDiagnostics(lsp_diagnostics_input))
        .await;
}
