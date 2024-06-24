use std::{path::PathBuf, sync::Arc};

use llm_client::{
    broker::LLMBroker,
    clients::types::LLMType,
    config::LLMBrokerConfiguration,
    provider::{GeminiProAPIKey, GoogleAIStudioKey, LLMProvider, LLMProviderAPIKeys},
};
use sidecar::agentic::{symbol::identifier::LLMProperties, tool::r#type::Tool};
use sidecar::{
    agentic::tool::{
        broker::ToolBroker, code_edit::models::broker::CodeEditBroker,
        code_symbol::important::CodeSymbolImportantWideSearch, input::ToolInput,
    },
    chunking::{editor_parsing::EditorParsing, languages::TSLanguageParsing},
    inline_completion::symbols_tracker::SymbolTrackerInline,
    user_context::types::UserContext,
};

fn default_index_dir() -> PathBuf {
    match directories::ProjectDirs::from("ai", "codestory", "sidecar") {
        Some(dirs) => dirs.data_dir().to_owned(),
        None => "codestory_sidecar".into(),
    }
}

#[tokio::main]
async fn main() {
    let gemini_pro_api_key = LLMProviderAPIKeys::GeminiPro(GeminiProAPIKey::new("ya29.a0AXooCgsyayMRlJE8xsuPvO2GYGUDzJNtNCXIqDIWowqoa7jzLH8oleEDqmhMkmYGdeB14Yezkv4OF6jhnFQime_Zo3ZVYM3kMMSbGk2b5Jo1mhv8No-nsnymFWUpCyZQrPgyOQpPc44JiEqf7IRwmNLOEoMMQ02I0cpWPxJT954aCgYKAeUSARESFQHGX2Mijj50U7MmN8j0vtQQvo_zhA0178".to_owned(), "anton-390822".to_owned()));
    let user_context = UserContext::new(
        vec![],
        vec![],
        None,
        // vec![],
        vec!["/var/folders/bq/1dbw218x1zq3r3c5_gqxgdgr0000gn/T/tmpyb7d6owx".to_owned()],
    );
    let user_query = "".to_owned();
    let _editor_url = "http://localhost:42423".to_owned();
    let editor_parsing = Arc::new(EditorParsing::default());
    let symbol_broker = Arc::new(SymbolTrackerInline::new(editor_parsing.clone()));
    let tool_broker = Arc::new(ToolBroker::new(
        Arc::new(
            LLMBroker::new(LLMBrokerConfiguration::new(default_index_dir()))
                .await
                .expect("to initialize properly"),
        ),
        Arc::new(CodeEditBroker::new()),
        symbol_broker.clone(),
        Arc::new(TSLanguageParsing::init()),
        None,
        LLMProperties::new(
            LLMType::GeminiPro,
            LLMProvider::GoogleAIStudio,
            LLMProviderAPIKeys::GoogleAIStudio(GoogleAIStudioKey::new(
                "AIzaSyCMkKfNkmjF8rTOWMg53NiYmz0Zv6xbfsE".to_owned(),
            )),
        ),
    ));
    let code_wide_search = ToolInput::RequestImportantSybmolsCodeWide(
        CodeSymbolImportantWideSearch::new(
            user_context,
            user_query,
            LLMType::GeminiProFlash,
            LLMProvider::GeminiPro,
            gemini_pro_api_key,
        )
        .set_file_extension_fitler("py".to_owned()),
    );
    let output = tool_broker.invoke(code_wide_search).await;
    println!("{:?}", output);
}
