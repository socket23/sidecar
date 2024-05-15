use async_trait::async_trait;
use std::{collections::HashMap, sync::Arc};

use llm_client::broker::LLMBroker;

use crate::{
    chunking::languages::TSLanguageParsing, inline_completion::symbols_tracker::SymbolTrackerInline,
};

use super::{
    base::{Tool, ToolType},
    code_edit::{
        find::FindCodeSectionsToEdit, models::broker::CodeEditBroker, types::CodeEditingTool,
    },
    code_symbol::important::CodeSymbolImportantBroker,
    editor::apply::EditorApply,
    errors::ToolError,
    filtering::broker::CodeToEditFormatterBroker,
    grep::file::FindInFile,
    input::ToolInput,
    lsp::{
        diagnostics::LSPDiagnostics, gotodefintion::LSPGoToDefinition,
        gotoimplementations::LSPGoToImplementation, open_file::LSPOpenFile,
    },
    output::ToolOutput,
    rerank::base::ReRankBroker,
};

// TODO(skcd): We want to use a different serializer and deserializer for this
// since we are going to be storing an array of tools over here, we have to make
// sure that we do not store everything about the tool but a representation of it
pub struct ToolBroker {
    tools: HashMap<ToolType, Box<dyn Tool + Send + Sync>>,
}

impl ToolBroker {
    pub fn new(
        llm_client: Arc<LLMBroker>,
        code_edit_broker: Arc<CodeEditBroker>,
        symbol_tracking: Arc<SymbolTrackerInline>,
        language_broker: Arc<TSLanguageParsing>,
    ) -> Self {
        let mut tools: HashMap<ToolType, Box<dyn Tool + Send + Sync>> = Default::default();
        tools.insert(
            ToolType::CodeEditing,
            Box::new(CodeEditingTool::new(
                llm_client.clone(),
                code_edit_broker.clone(),
            )),
        );
        tools.insert(ToolType::LSPDiagnostics, Box::new(LSPDiagnostics::new()));
        tools.insert(
            ToolType::FindCodeSnippets,
            Box::new(FindCodeSectionsToEdit::new(
                symbol_tracking,
                language_broker,
                code_edit_broker.clone(),
                llm_client.clone(),
            )),
        );
        tools.insert(
            ToolType::ReRank,
            Box::new(ReRankBroker::new(llm_client.clone())),
        );
        tools.insert(
            ToolType::RequestImportantSymbols,
            Box::new(CodeSymbolImportantBroker::new(llm_client.clone())),
        );
        tools.insert(
            ToolType::FindCodeSymbolsCodeBaseWide,
            Box::new(CodeSymbolImportantBroker::new(llm_client.clone())),
        );
        tools.insert(
            ToolType::GoToDefinitions,
            Box::new(LSPGoToDefinition::new()),
        );
        tools.insert(ToolType::OpenFile, Box::new(LSPOpenFile::new()));
        tools.insert(ToolType::GrepInFile, Box::new(FindInFile::new()));
        tools.insert(
            ToolType::GoToImplementations,
            Box::new(LSPGoToImplementation::new()),
        );
        tools.insert(
            ToolType::FilterCodeSnippetsForEditing,
            Box::new(CodeToEditFormatterBroker::new(llm_client.clone())),
        );
        tools.insert(
            ToolType::FilterCodeSnippetsSingleSymbolForEditing,
            Box::new(CodeToEditFormatterBroker::new(llm_client)),
        );
        tools.insert(ToolType::EditorApplyEdits, Box::new(EditorApply::new()));
        // we also want to add the re-ranking tool here, so we invoke it freely
        Self { tools }
    }
}

#[async_trait]
impl Tool for ToolBroker {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let tool_type = input.tool_type();
        let time_start = std::time::Instant::now();
        if let Some(tool) = self.tools.get(&tool_type) {
            let result = tool.invoke(input).await;
            println!("Tool(OK): time taken: {:?}", time_start.elapsed());
            result
        } else {
            let result = Err(ToolError::MissingTool);
            println!("Tool(Err): time taken: {:?}", time_start.elapsed());
            result
        }
    }
}
