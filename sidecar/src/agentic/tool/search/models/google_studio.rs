use std::sync::Arc;

use llm_client::broker::LLMBroker;

use async_trait::async_trait;

use crate::agentic::{
    symbol::identifier::LLMProperties,
    tool::{
        code_symbol::{important::CodeSymbolImportantResponse, types::CodeSymbolError},
        search::types::{BigSearch, BigSearchRequest, SearchType},
    },
};

pub struct GoogleStudioBigSearch {
    _llm_client: Arc<LLMBroker>,
    _fail_over_llm: LLMProperties,
}

impl GoogleStudioBigSearch {
    pub fn new(llm_client: Arc<LLMBroker>, fail_over_llm: LLMProperties) -> Self {
        Self {
            _llm_client: llm_client,
            _fail_over_llm: fail_over_llm,
        }
    }
}

#[async_trait]
impl BigSearch for GoogleStudioBigSearch {
    async fn search(
        &self,
        input: BigSearchRequest,
    ) -> Result<CodeSymbolImportantResponse, CodeSymbolError> {
        match input.search_type() {
            SearchType::Tree(_tree_data) => {
                // Perform tree search calculation using tree_data
                // ...
            }
            SearchType::Repomap(_repomap_data) => {
                // Perform repomap search calculation using repomap_data
                // ...
            }
            SearchType::Both(tree_data, repomap_data) => {
                println!(
                    "Both tree and repomap search calculation using tree_data and repomap_data"
                );

                println!("tree_data: {:?}", tree_data);
                println!("repomap_data: {:?}", repomap_data);
                // Perform both tree and repomap search calculation using tree_data and repomap_data
                // ...
            }
        }
        todo!();
    }
}
