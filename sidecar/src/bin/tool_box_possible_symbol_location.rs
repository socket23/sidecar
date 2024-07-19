use std::{collections::HashSet, path::PathBuf, sync::Arc};

use llm_client::{
    broker::LLMBroker,
    clients::types::LLMType,
    config::LLMBrokerConfiguration,
    provider::{GoogleAIStudioKey, LLMProvider, LLMProviderAPIKeys},
};
use sidecar::{
    agentic::{
        symbol::{identifier::LLMProperties, tool_box::ToolBox},
        tool::{
            broker::{ToolBroker, ToolBrokerConfiguration},
            code_edit::models::broker::CodeEditBroker,
        },
    },
    chunking::{editor_parsing::EditorParsing, languages::TSLanguageParsing},
    inline_completion::symbols_tracker::SymbolTrackerInline,
};
use tree_sitter::Parser;

fn default_index_dir() -> PathBuf {
    match directories::ProjectDirs::from("ai", "codestory", "sidecar") {
        Some(dirs) => dirs.data_dir().to_owned(),
        None => "codestory_sidecar".into(),
    }
}

#[tokio::main]
async fn main() {
    let editor_url = "http://localhost:42450".to_owned();
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
        // apply the edits directly
        ToolBrokerConfiguration::new(None, true),
        LLMProperties::new(
            LLMType::GeminiPro,
            LLMProvider::GoogleAIStudio,
            LLMProviderAPIKeys::GoogleAIStudio(GoogleAIStudioKey::new(
                "AIzaSyCMkKfNkmjF8rTOWMg53NiYmz0Zv6xbfsE".to_owned(),
            )),
        ),
    ));

    let ts_language_parsing = TSLanguageParsing::init();
    let language_config = ts_language_parsing.for_lang("rust").expect("to work");
    let fs_file_path = "/Users/skcd/scratch/sidecar/llm_client/src/broker.rs".to_owned();
    let source_code = tokio::fs::read(fs_file_path.to_owned())
        .await
        .expect("to work");
    let mut parser = Parser::new();
    let grammar = language_config.grammar;
    parser.set_language(grammar()).unwrap();
    let tree = parser.parse(source_code.as_slice(), None).unwrap();
    let import_nodes =
        language_config.generate_import_identifier_nodes(source_code.as_slice(), &tree);
    let hoverable_nodes = language_config.hoverable_nodes(source_code.as_slice());
    println!("What are the import nodes: {:?}", &import_nodes);
    let clickable_nodes = hoverable_nodes
        .into_iter()
        .filter(|hoverable_node| {
            import_nodes
                .iter()
                .any(|(_, range)| range.contains(&hoverable_node))
        })
        .collect::<Vec<_>>();
    println!("What are the clickable nodes::????");
    println!("{:?}", &clickable_nodes);

    let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();

    let tool_box = Arc::new(ToolBox::new(
        tool_broker,
        symbol_broker,
        editor_parsing,
        editor_url,
        sender,
        "".to_owned(),
    ));

    let mut final_files = vec![];
    let fs_file_path_ref = &fs_file_path;
    for clickable_node in clickable_nodes.into_iter() {
        let go_to_definition_response = tool_box
            .go_to_definition(fs_file_path_ref, clickable_node.end_position(), "")
            .await;
        if let Ok(go_to_definition) = go_to_definition_response {
            final_files.extend(
                go_to_definition
                    .definitions()
                    .into_iter()
                    .map(|definition| definition.file_path().to_owned()),
            );
        }
    }

    final_files = final_files
        .into_iter()
        .collect::<HashSet<String>>()
        .into_iter()
        .filter(|file_path| !file_path.contains("rustup") && !file_path.contains("cargo"))
        .collect::<Vec<_>>();

    // Now we want to create a prompt which will give us the outline present in the file
    // so we can prompt the AI to figure out where it wants to make those changes
    let _outline_nodes: Vec<String> = vec![];
    for file_path in final_files.into_iter() {
        let _file_content = tokio::fs::read(file_path.to_owned())
            .await
            .expect("to work");
        let mut parser = Parser::new();
        let grammar = language_config.grammar;
        parser.set_language(grammar()).unwrap();
        let tree = parser.parse(source_code.as_slice(), None).unwrap();
        let _outline_nodes = ts_language_parsing
            .for_lang("rust")
            .expect("to work")
            .generate_outline(source_code.as_slice(), &tree, file_path);
    }
}
