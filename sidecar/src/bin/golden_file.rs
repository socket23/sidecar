use std::{path::PathBuf, sync::Arc};

use serde::Deserialize;
use std::error::Error;
use std::fs::File;

use futures::{stream, StreamExt};
use llm_client::{
    broker::LLMBroker,
    clients::types::LLMType,
    config::LLMBrokerConfiguration,
    provider::{AnthropicAPIKey, LLMProvider, LLMProviderAPIKeys, OpenAIProvider},
};
use sidecar::{
    agentic::{
        symbol::{
            events::input::SymbolInputEvent, identifier::LLMProperties, manager::SymbolManager,
        },
        tool::{
            broker::{ToolBroker, ToolBrokerConfiguration},
            code_edit::models::broker::CodeEditBroker,
        },
    },
    chunking::{editor_parsing::EditorParsing, languages::TSLanguageParsing},
    inline_completion::symbols_tracker::SymbolTrackerInline,
    user_context::types::{FileContentValue, UserContext},
};

#[derive(Debug, Deserialize, Clone)]
struct Task {
    golden_file: String,
    problem_statement: String,
}

fn default_index_dir() -> PathBuf {
    match directories::ProjectDirs::from("ai", "codestory", "sidecar") {
        Some(dirs) => dirs.data_dir().to_owned(),
        None => "codestory_sidecar".into(),
    }
}

#[tokio::main]
async fn main() {
    let csv_path = "/Users/zi/codestory/sidecar/sidecar/src/bin/swe_lite_formula.csv";
    let repo = "sqlfluff/sqlfluff";

    let problems = read_problems_from_csv(csv_path, repo);
}

fn read_problems_from_csv(path: &str, repo: &str) -> Result<Vec<Task>, Box<dyn Error>> {
    let file = File::open(path)?;
    let mut rdr = csv::Reader::from_reader(file);
    let mut problems = Vec::new();

    Ok(problems)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROOT_DIR: &str = "/Users/zi/codestory/testing/sqlfluff";

    #[tokio::test]
    async fn test_sqlfluff_031() {
        let task = Task {
            golden_file: "src/sqlfluff/rules/L031.py".to_string(),
            problem_statement: r#""TSQL - L031 incorrectly triggers ""Avoid using aliases in join condition"" when no join present
            ## Expected Behaviour
            
            Both of these queries should pass, the only difference is the addition of a table alias 'a':
            
            1/ no alias
            
            ```
            SELECT [hello]
            FROM
                mytable
            ```
            
            2/ same query with alias
            
            ```
            SELECT a.[hello]
            FROM
                mytable AS a
            ```
            
            ## Observed Behaviour
            
            1/ passes
            2/ fails with: L031: Avoid using aliases in join condition.
            
            But there is no join condition :-)
            
            ## Steps to Reproduce
            
            Lint queries above
            
            ## Dialect
            
            TSQL
            
            ## Version
            
            sqlfluff 0.6.9
            Python 3.6.9
            
            ## Configuration
            
            N/A
            "#.to_string(),
        };
        test_one(&task).await;
    }

    #[tokio::test]
    async fn test_sqlfluff_060() {
        let task = Task {
            golden_file: "src/sqlfluff/rules/L060.py".to_string(),
            problem_statement: r#""Rule L060 could give a specific error message
            At the moment rule L060 flags something like this:
            
            ```
            L:  21 | P:   9 | L060 | Use 'COALESCE' instead of 'IFNULL' or 'NVL'.
            ```
            
            Since we likely know the wrong word, it might be nice to actually flag that instead of both `IFNULL` and `NVL` - like most of the other rules do.
            
            That is it should flag this:
            
            ```
            L:  21 | P:   9 | L060 | Use 'COALESCE' instead of 'IFNULL'.
            ```
             Or this:
            
            ```
            L:  21 | P:   9 | L060 | Use 'COALESCE' instead of 'NVL'.
            ```
            
            As appropriate.
            
            What do you think @jpy-git ?
            
            ""#.to_string(),
        };
        test_one(&task).await;
    }

    async fn test_one(task: &Task) {
        let request_id = uuid::Uuid::new_v4();
        let request_id_str = request_id.to_string();
        let parea_url = format!(
            r#"https://app.parea.ai/logs?colViz=%7B%220%22%3Afalse%2C%221%22%3Afalse%2C%222%22%3Afalse%2C%223%22%3Afalse%2C%22error%22%3Afalse%2C%22deployment_id%22%3Afalse%2C%22feedback_score%22%3Afalse%2C%22time_to_first_token%22%3Afalse%2C%22scores%22%3Afalse%2C%22start_timestamp%22%3Afalse%2C%22user%22%3Afalse%2C%22session_id%22%3Afalse%2C%22target%22%3Afalse%2C%22experiment_uuid%22%3Afalse%2C%22dataset_references%22%3Afalse%2C%22in_dataset%22%3Afalse%2C%22event_type%22%3Afalse%2C%22request_type%22%3Afalse%2C%22evaluation_metric_names%22%3Afalse%2C%22request%22%3Afalse%2C%22calling_node%22%3Afalse%2C%22edges%22%3Afalse%2C%22metadata_evaluation_metric_names%22%3Afalse%2C%22metadata_event_type%22%3Afalse%2C%22metadata_0%22%3Afalse%2C%22metadata_calling_node%22%3Afalse%2C%22metadata_edges%22%3Afalse%2C%22metadata_root_id%22%3Afalse%7D&filter=%7B%22filter_field%22%3A%22meta_data%22%2C%22filter_operator%22%3A%22equals%22%2C%22filter_key%22%3A%22root_id%22%2C%22filter_value%22%3A%22{request_id_str}%22%7D&page=1&page_size=50&time_filter=1m"#
        );
        println!("===========================================\nRequest ID: {}\nParea AI: {}\n===========================================", request_id.to_string(), parea_url);
        let editor_url = "http://localhost:42424".to_owned();
        let anthropic_api_keys = LLMProviderAPIKeys::Anthropic(AnthropicAPIKey::new("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".to_owned()));
        let anthropic_llm_properties = LLMProperties::new(
            LLMType::ClaudeSonnet,
            LLMProvider::Anthropic,
            anthropic_api_keys.clone(),
        );
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
            ToolBrokerConfiguration::new(None, true),
            LLMProperties::new(
                LLMType::Gpt4O,
                LLMProvider::OpenAI,
                LLMProviderAPIKeys::OpenAI(OpenAIProvider::new(
                    "sk-proj-BLaSMsWvoO6FyNwo9syqT3BlbkFJo3yqCyKAxWXLm4AvePtt".to_owned(),
                )),
            ),
        ));

        let user_context = UserContext::new(vec![], vec![], None, vec![]);

        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();

        let symbol_manager = SymbolManager::new(
            tool_broker.clone(),
            symbol_broker.clone(),
            editor_parsing,
            editor_url.to_owned(),
            sender,
            anthropic_llm_properties.clone(),
            user_context.clone(),
            request_id.to_string(),
        );

        let initial_request = SymbolInputEvent::new(
            user_context,
            LLMType::ClaudeSonnet,
            LLMProvider::Anthropic,
            anthropic_api_keys,
            task.problem_statement.clone(),
            request_id.to_string(),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            true, // full_symbol_edit
            true, // codebase search
            Some(ROOT_DIR.to_string()),
        );

        let mut initial_request_task = Box::pin(symbol_manager.test_golden_file(initial_request));

        loop {
            tokio::select! {
                event = receiver.recv() => {
                    if event.is_none() {
                        break; // Receiver closed, exit the loop
                    }
                }
                result = &mut initial_request_task => {
                    match result {
                        Ok(symbols) => {
                            assert!(!symbols.is_empty(), "Expected non-empty vector of symbols");
                            assert!(
                                symbols.iter().any(|symbol| symbol.file_path().ends_with(&task.golden_file)),
                                "Expected golden file '{}' not found in the returned symbols",
                                task.golden_file,
                            );
                            break
                        }
                        Err(e) => {
                            eprintln!("Error in initial_request_task: {}", e);
                            break;
                        }
                    }
                }
            }
        }
    }
}
