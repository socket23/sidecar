use llm_client::clients::types::LLMType;
use sidecar::agentic::tool::code_symbol::tree::ImportantFilesFinderBroker;

fn main() {
    let llm = LLMType::GeminiProFlash;
    let tree = "some tree";
    let broker = ImportantFilesFinderBroker::new(tree.to_string(), llm);
}
