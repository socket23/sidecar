use llm_client::clients::types::LLMType;

pub struct ImportantFilesFinderBroker {
    tree: String,
    llm: LLMType,
}

impl ImportantFilesFinderBroker {
    pub fn new(tree: String, llm: LLMType) -> Self {
        Self { tree, llm }
    }
}
