use llm_client::{
    clients::{
        fireworks::FireworksAIClient,
        groq::GroqClient,
        types::{LLMClient, LLMClientCompletionRequest, LLMClientMessage, LLMType},
    },
    provider::{FireworksAPIKey, GroqProviderAPIKey, LLMProviderAPIKeys},
};

#[tokio::main]
async fn main() {
    let system_message = r#"You are an expert software eningeer who never writes incorrect code and is tasked with selecting code symbols whose definitions you can use for editing.
The editor has stopped working for you, so we get no help with auto-complete when writing code, hence we want to make sure that we select all the code symbols which are necessary.
As a first step before making changes, you are tasked with collecting all the definitions of the various code symbols whose methods or parameters you will be using when editing the code in the selection.
- You will be given the original user query in <user_query>
- You will be provided the code snippet you will be editing in <code_snippet_to_edit> section.
- The various definitions of the class, method or function (just the high level outline of it) will be given to you as a list in <code_symbol_outline_list>. When writing code you will reuse the methods from here to make the edits, so be very careful when selecting the symbol outlines you are interested in.
- Pay attention to the <code_snippet_to_edit> section and select code symbols accordingly, do not select symbols which we will not be using for making edits.
- Each code_symbol_outline entry is in the following format:
```
<code_symbol>
<name>
{name of the code symbol over here}
</name>
<content>
{the outline content for the code symbol over here}
</content>
</code_symbol>
```
- You have to decide which code symbols you will be using when doing the edits and select those code symbols.
Your reply should be in the following format:
<reply>
<thinking>
</thinking>
<code_symbol_outline_list>
<code_symbol>
<name>
</name>
<file_path>
</file_path>
</code_symbol>
... more code_symbol sections over here as per your requirement
</code_symbol_outline_list>
<reply>

Now we will show you an example of how the output should look like:
<user_query>
We want to implement a new method on symbol event which exposes the initial request question
</user_query>
<code_snippet_to_edit>
```rust
#[derive(Debug, Clone, serde::Serialize)]
pub enum SymbolEvent {
    InitialRequest(InitialRequestData),
    AskQuestion(AskQuestionRequest),
    UserFeedback,
    Delete,
    Edit(SymbolToEditRequest),
    Outline,
    // Probe
    Probe(SymbolToProbeRequest),
}
```
</code_snippet_to_edit>
<code_symbol_outline_list>
<code_symbol>
<name>
InitialRequestData
</name>
<content>
FILEPATH: /Users/skcd/scratch/sidecar/sidecar/src/agentic/symbol/events/initial_request.rs
#[derive(Debug, Clone, serde::Serialize)]
pub struct InitialRequestData {
    original_question: String,
    plan_if_available: Option<String>,
    history: Vec<SymbolRequestHistoryItem>,
    /// We operate on the full symbol instead of the
    full_symbol_request: bool,
}

impl InitialRequestData {
    pub fn new(
        original_question: String,
        plan_if_available: Option<String>,
        history: Vec<SymbolRequestHistoryItem>,
        full_symbol_request: bool,
    ) -> Self
    
    pub fn full_symbol_request(&self) -> bool

    pub fn get_original_question(&self) -> &str

    pub fn get_plan(&self) -> Option<String>

    pub fn history(&self) -> &[SymbolRequestHistoryItem]
}
</content>
</code_symbol>
<code_symbol>
<name>
AskQuestionRequest
</name>
<content>
FILEPATH: /Users/skcd/scratch/sidecar/sidecar/src/agentic/symbol/events/edit.rs
#[derive(Debug, Clone, serde::Serialize)]
pub struct AskQuestionRequest {
    question: String,
}

impl AskQuestionRequest {
    pub fn new(question: String) -> Self

    pub fn get_question(&self) -> &str
}
</content>
</code_symbol>
<code_symbol>
<name>
SymbolToEditRequest
</name>
<content>
FILEPATH: /Users/skcd/scratch/sidecar/sidecar/src/agentic/symbol/events/edit.rs
#[derive(Debug, Clone, serde::Serialize)]
pub struct SymbolToEditRequest {
    symbols: Vec<SymbolToEdit>,
    symbol_identifier: SymbolIdentifier,
    history: Vec<SymbolRequestHistoryItem>,
}

impl SymbolToEditRequest {
    pub fn new(
        symbols: Vec<SymbolToEdit>,
        identifier: SymbolIdentifier,
        history: Vec<SymbolRequestHistoryItem>,
    ) -> Self

    pub fn symbols(self) -> Vec<SymbolToEdit>

    pub fn symbol_identifier(&self) -> &SymbolIdentifier

    pub fn history(&self) -> &[SymbolRequestHistoryItem]
}
</content>
</code_symbol>
<code_symbol>
<name>
SymbolToProbeRequest
</name>
<content>
FILEPATH: /Users/skcd/scratch/sidecar/sidecar/src/agentic/symbol/events/probe.rs
#[derive(Debug, Clone, serde::Serialize)]
pub struct SymbolToProbeRequest {
    symbol_identifier: SymbolIdentifier,
    probe_request: String,
    original_request: String,
    original_request_id: String,
    history: Vec<SymbolToProbeHistory>,
}

impl SymbolToProbeRequest {
    pub fn new(
        symbol_identifier: SymbolIdentifier,
        probe_request: String,
        original_request: String,
        original_request_id: String,
        history: Vec<SymbolToProbeHistory>,
    ) -> Self

    pub fn symbol_identifier(&self) -> &SymbolIdentifier

    pub fn original_request_id(&self) -> &str

    pub fn original_request(&self) -> &str

    pub fn probe_request(&self) -> &str

    pub fn history_slice(&self) -> &[SymbolToProbeHistory]

    pub fn history(&self) -> String
}
</content>
</code_symbol>
</code_symbol_outline_list>

Your reply should be:
<reply>
<thinking>
The request talks about implementing new methods for the initial request data, so we need to include the initial request data symbol in the context when trying to edit the code.
</thinking>
<code_symbol_outline_list>
<code_symbol>
<name>
InitialRequestData
</name>
<file_path>
/Users/skcd/scratch/sidecar/sidecar/src/agentic/symbol/events/initial_request.rs
</file_path>
</code_symbol>
</code_symbol_outline_list>
</reply>"#;
    let user_message = r#"<user_query>
Original user query:
Add support for mixtral model to LLMType

Edit selection reason:
The `Deserialize` implementation for `LLMType` needs to be updated to correctly deserialize the new Mixtral model.
</user_query>

<file_path>
/Users/skcd/test_repo/sidecar/llm_client/src/clients/types.rs
</file_path>



<code_snippet_to_edit>

impl<'de> Deserialize<'de> for LLMType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct LLMTypeVisitor;

        impl<'de> Visitor<'de> for LLMTypeVisitor {
            type Value = LLMType;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string representing an LLMType")
            }

            fn visit_str<E>(self, value: &str) -> Result<LLMType, E>
            where
                E: de::Error,
            {
                match value {
                    "Mixtral" => Ok(LLMType::Mixtral),
                    "MistralInstruct" => Ok(LLMType::MistralInstruct),
                    "Gpt4" => Ok(LLMType::Gpt4),
                    "Gpt4OMini" => Ok(LLMType::Gpt4OMini),
                    "GPT3_5_16k" => Ok(LLMType::GPT3_5_16k),
                    "Gpt4_32k" => Ok(LLMType::Gpt4_32k),
                    "Gpt4Turbo" => Ok(LLMType::Gpt4Turbo),
                    "DeepSeekCoder1.3BInstruct" => Ok(LLMType::DeepSeekCoder1_3BInstruct),
                    "DeepSeekCoder6BInstruct" => Ok(LLMType::DeepSeekCoder6BInstruct),
                    "CodeLLama70BInstruct" => Ok(LLMType::CodeLLama70BInstruct),
                    "CodeLlama13BInstruct" => Ok(LLMType::CodeLlama13BInstruct),
                    "CodeLlama7BInstruct" => Ok(LLMType::CodeLlama7BInstruct),
                    "DeepSeekCoder33BInstruct" => Ok(LLMType::DeepSeekCoder33BInstruct),
                    "ClaudeOpus" => Ok(LLMType::ClaudeOpus),
                    "ClaudeSonnet" => Ok(LLMType::ClaudeSonnet),
                    "ClaudeHaiku" => Ok(LLMType::ClaudeHaiku),
                    "PPLXSonnetSmall" => Ok(LLMType::PPLXSonnetSmall),
                    "CohereRerankV3" => Ok(LLMType::CohereRerankV3),
                    "GeminiPro1.5" => Ok(LLMType::GeminiPro),
                    "Llama3_8bInstruct" => Ok(LLMType::Llama3_8bInstruct),
                    "Llama3_1_8bInstruct" => Ok(LLMType::Llama3_1_8bInstruct),
                    "Llama3_1_70bInstruct" => Ok(LLMType::Llama3_1_70bInstruct),
                    "Gpt4O" => Ok(LLMType::Gpt4O),
                    "GeminiProFlash" => Ok(LLMType::GeminiProFlash),
                    "DeepSeekCoderV2" => Ok(LLMType::DeepSeekCoderV2),
                    _ => Ok(LLMType::Custom(value.to_string())),
                }
            }
        }

        deserializer.deserialize_string(LLMTypeVisitor)
    }
</code_snippet_to_edit>

<code_symbol_outline_list>
<code_symbol>
<name>
LLMType
</name>
<file_path>
/Users/skcd/test_repo/sidecar/llm_client/src/clients/types.rs
</file_path>
<content>
FILEPATH: /Users/skcd/test_repo/sidecar/llm_client/src/clients/types.rs
#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub enum LLMType {
    Mixtral,
    MistralInstruct,
    Gpt4,
    GPT3_5_16k,
    Gpt4_32k,
    Gpt4O,
    Gpt4OMini,
    Gpt4Turbo,
    DeepSeekCoder1_3BInstruct,
    DeepSeekCoder33BInstruct,
    DeepSeekCoder6BInstruct,
    DeepSeekCoderV2,
    CodeLLama70BInstruct,
    CodeLlama13BInstruct,
    CodeLlama7BInstruct,
    Llama3_8bInstruct,
    Llama3_1_8bInstruct,
    Llama3_1_70bInstruct,
    ClaudeOpus,
    ClaudeSonnet,
    ClaudeHaiku,
    PPLXSonnetSmall,
    CohereRerankV3,
    GeminiPro,
    GeminiProFlash,
    Custom(String),
}
</content>
</code_symbol>
<code_symbol>
<name>
LLMType
</name>
<file_path>
/Users/skcd/test_repo/sidecar/llm_client/src/clients/types.rs
</file_path>
<content>
FILEPATH: /Users/skcd/test_repo/sidecar/llm_client/src/clients/types.rs
impl Serialize for LLMType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
}
</content>
</code_symbol>
<code_symbol>
<name>
LLMType
</name>
<file_path>
/Users/skcd/test_repo/sidecar/llm_client/src/clients/types.rs
</file_path>
<content>
FILEPATH: /Users/skcd/test_repo/sidecar/llm_client/src/clients/types.rs
impl<'de> Deserialize<'de> for LLMType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            fn visit_str<E>(self, value: &str) -> Result<LLMType, E>
            where
                E: de::Error,
            {
}
</content>
</code_symbol>
<code_symbol>
<name>
LLMType
</name>
<file_path>
/Users/skcd/test_repo/sidecar/llm_client/src/clients/types.rs
</file_path>
<content>
FILEPATH: /Users/skcd/test_repo/sidecar/llm_client/src/clients/types.rs
impl LLMType {
    pub fn is_openai(&self) -> bool {
    pub fn is_custom(&self) -> bool {
    pub fn is_anthropic(&self) -> bool {
    pub fn is_openai_gpt4o(&self) -> bool {
    pub fn is_gemini_model(&self) -> bool {
    pub fn is_gemini_pro(&self) -> bool {
    pub fn is_togetherai_model(&self) -> bool {
}
</content>
</code_symbol>
<code_symbol>
<name>
LLMType
</name>
<file_path>
/Users/skcd/test_repo/sidecar/llm_client/src/clients/types.rs
</file_path>
<content>
FILEPATH: /Users/skcd/test_repo/sidecar/llm_client/src/clients/types.rs
impl fmt::Display for LLMType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
}
</content>
</code_symbol>
<code_symbol>
<name>
LLMClientRole
</name>
<file_path>
/Users/skcd/test_repo/sidecar/llm_client/src/clients/types.rs
</file_path>
<content>
FILEPATH: /Users/skcd/test_repo/sidecar/llm_client/src/clients/types.rs
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub enum LLMClientRole {
    System,
    User,
    Assistant,
    // function calling is weird, its only supported by openai right now
    // and not other LLMs, so we are going to make this work with the formatters
    // and still keep it as it is
    Function,
}
</content>
</code_symbol>
<code_symbol>
<name>
LLMClientRole
</name>
<file_path>
/Users/skcd/test_repo/sidecar/llm_client/src/clients/types.rs
</file_path>
<content>
FILEPATH: /Users/skcd/test_repo/sidecar/llm_client/src/clients/types.rs
impl LLMClientRole {
    pub fn is_system(&self) -> bool {
    pub fn is_user(&self) -> bool {
    pub fn is_assistant(&self) -> bool {
    pub fn is_function(&self) -> bool {
    pub fn to_string(&self) -> String {
}
</content>
</code_symbol>
<code_symbol>
<name>
LLMClientMessageFunctionCall
</name>
<file_path>
/Users/skcd/test_repo/sidecar/llm_client/src/clients/types.rs
</file_path>
<content>
FILEPATH: /Users/skcd/test_repo/sidecar/llm_client/src/clients/types.rs
#[derive(serde::Serialize, Debug, Clone)]
pub struct LLMClientMessageFunctionCall {
    name: String,
    // arguments are generally given as a JSON string, so we keep it as a string
    // here, validate in the upper handlers for this
    arguments: String,
}
</content>
</code_symbol>
<code_symbol>
<name>
LLMClientMessageFunctionCall
</name>
<file_path>
/Users/skcd/test_repo/sidecar/llm_client/src/clients/types.rs
</file_path>
<content>
FILEPATH: /Users/skcd/test_repo/sidecar/llm_client/src/clients/types.rs
impl LLMClientMessageFunctionCall {
    pub fn name(&self) -> &str {
    pub fn arguments(&self) -> &str {
}
</content>
</code_symbol>
<code_symbol>
<name>
LLMClientMessageFunctionReturn
</name>
<file_path>
/Users/skcd/test_repo/sidecar/llm_client/src/clients/types.rs
</file_path>
<content>
FILEPATH: /Users/skcd/test_repo/sidecar/llm_client/src/clients/types.rs
#[derive(serde::Serialize, Debug, Clone)]
pub struct LLMClientMessageFunctionReturn {
    name: String,
    content: String,
}
</content>
</code_symbol>
<code_symbol>
<name>
LLMClientMessageFunctionReturn
</name>
<file_path>
/Users/skcd/test_repo/sidecar/llm_client/src/clients/types.rs
</file_path>
<content>
FILEPATH: /Users/skcd/test_repo/sidecar/llm_client/src/clients/types.rs
impl LLMClientMessageFunctionReturn {
    pub fn name(&self) -> &str {
    pub fn content(&self) -> &str {
}
</content>
</code_symbol>
<code_symbol>
<name>
LLMClientMessage
</name>
<file_path>
/Users/skcd/test_repo/sidecar/llm_client/src/clients/types.rs
</file_path>
<content>
FILEPATH: /Users/skcd/test_repo/sidecar/llm_client/src/clients/types.rs
#[derive(serde::Serialize, Debug, Clone)]
pub struct LLMClientMessage {
    role: LLMClientRole,
    message: String,
    function_call: Option<LLMClientMessageFunctionCall>,
    function_return: Option<LLMClientMessageFunctionReturn>,
}
</content>
</code_symbol>
<code_symbol>
<name>
LLMClientMessage
</name>
<file_path>
/Users/skcd/test_repo/sidecar/llm_client/src/clients/types.rs
</file_path>
<content>
FILEPATH: /Users/skcd/test_repo/sidecar/llm_client/src/clients/types.rs
impl LLMClientMessage {
    pub fn new(role: LLMClientRole, message: String) -> Self {
    pub fn concat_message(&mut self, message: &str) {
    pub fn concat(self, other: Self) -> Self {
    pub fn function_call(name: String, arguments: String) -> Self {
    pub fn function_return(name: String, content: String) -> Self {
    pub fn user(message: String) -> Self {
    pub fn assistant(message: String) -> Self {
    pub fn system(message: String) -> Self {
    pub fn content(&self) -> &str {
    pub fn set_empty_content(&mut self) {
    pub fn function(message: String) -> Self {
    pub fn role(&self) -> &LLMClientRole {
    pub fn get_function_call(&self) -> Option<&LLMClientMessageFunctionCall> {
    pub fn get_function_return(&self) -> Option<&LLMClientMessageFunctionReturn> {
}
</content>
</code_symbol>
<code_symbol>
<name>
LLMClientCompletionRequest
</name>
<file_path>
/Users/skcd/test_repo/sidecar/llm_client/src/clients/types.rs
</file_path>
<content>
FILEPATH: /Users/skcd/test_repo/sidecar/llm_client/src/clients/types.rs
#[derive(Clone, Debug)]
pub struct LLMClientCompletionRequest {
    model: LLMType,
    messages: Vec<LLMClientMessage>,
    temperature: f32,
    frequency_penalty: Option<f32>,
    stop_words: Option<Vec<String>>,
    max_tokens: Option<usize>,
}
</content>
</code_symbol>
<code_symbol>
<name>
LLMClientCompletionStringRequest
</name>
<file_path>
/Users/skcd/test_repo/sidecar/llm_client/src/clients/types.rs
</file_path>
<content>
FILEPATH: /Users/skcd/test_repo/sidecar/llm_client/src/clients/types.rs
#[derive(Clone)]
pub struct LLMClientCompletionStringRequest {
    model: LLMType,
    prompt: String,
    temperature: f32,
    frequency_penalty: Option<f32>,
    stop_words: Option<Vec<String>>,
    max_tokens: Option<usize>,
}
</content>
</code_symbol>
<code_symbol>
<name>
LLMClientCompletionStringRequest
</name>
<file_path>
/Users/skcd/test_repo/sidecar/llm_client/src/clients/types.rs
</file_path>
<content>
FILEPATH: /Users/skcd/test_repo/sidecar/llm_client/src/clients/types.rs
impl LLMClientCompletionStringRequest {
    pub fn new(
        model: LLMType,
        prompt: String,
        temperature: f32,
        frequency_penalty: Option<f32>,
    ) -> Self {
    pub fn set_stop_words(mut self, stop_words: Vec<String>) -> Self {
    pub fn model(&self) -> &LLMType {
    pub fn temperature(&self) -> f32 {
    pub fn frequency_penalty(&self) -> Option<f32> {
    pub fn prompt(&self) -> &str {
    pub fn stop_words(&self) -> Option<&[String]> {
    pub fn set_max_tokens(mut self, max_tokens: usize) -> Self {
    pub fn get_max_tokens(&self) -> Option<usize> {
}
</content>
</code_symbol>
<code_symbol>
<name>
LLMClientCompletionRequest
</name>
<file_path>
/Users/skcd/test_repo/sidecar/llm_client/src/clients/types.rs
</file_path>
<content>
FILEPATH: /Users/skcd/test_repo/sidecar/llm_client/src/clients/types.rs
impl LLMClientCompletionRequest {
    pub fn new(
        model: LLMType,
        messages: Vec<LLMClientMessage>,
        temperature: f32,
        frequency_penalty: Option<f32>,
    ) -> Self {
    pub fn set_llm(mut self, llm: LLMType) -> Self {
    pub fn fix_message_structure(mut self: Self) -> Self {
    pub fn from_messages(messages: Vec<LLMClientMessage>, model: LLMType) -> Self {
    pub fn set_temperature(mut self, temperature: f32) -> Self {
    pub fn messages(&self) -> &[LLMClientMessage] {
    pub fn temperature(&self) -> f32 {
    pub fn frequency_penalty(&self) -> Option<f32> {
    pub fn model(&self) -> &LLMType {
    pub fn stop_words(&self) -> Option<&[String]> {
    pub fn set_max_tokens(mut self, max_tokens: usize) -> Self {
    pub fn get_max_tokens(&self) -> Option<usize> {
}
</content>
</code_symbol>
<code_symbol>
<name>
LLMClientCompletionResponse
</name>
<file_path>
/Users/skcd/test_repo/sidecar/llm_client/src/clients/types.rs
</file_path>
<content>
FILEPATH: /Users/skcd/test_repo/sidecar/llm_client/src/clients/types.rs
#[derive(Debug)]
pub struct LLMClientCompletionResponse {
    answer_up_until_now: String,
    delta: Option<String>,
    model: String,
}
</content>
</code_symbol>
<code_symbol>
<name>
LLMClientCompletionResponse
</name>
<file_path>
/Users/skcd/test_repo/sidecar/llm_client/src/clients/types.rs
</file_path>
<content>
FILEPATH: /Users/skcd/test_repo/sidecar/llm_client/src/clients/types.rs
impl LLMClientCompletionResponse {
    pub fn new(answer_up_until_now: String, delta: Option<String>, model: String) -> Self {
    pub fn get_answer_up_until_now(self) -> String {
    pub fn answer_up_until_now(&self) -> &str {
    pub fn delta(&self) -> Option<&str> {
    pub fn model(&self) -> &str {
}
</content>
</code_symbol>
<code_symbol>
<name>
LLMClientError
</name>
<file_path>
/Users/skcd/test_repo/sidecar/llm_client/src/clients/types.rs
</file_path>
<content>
FILEPATH: /Users/skcd/test_repo/sidecar/llm_client/src/clients/types.rs
#[derive(Error, Debug)]
pub enum LLMClientError {
    #[error("Failed to get response from LLM")]
    FailedToGetResponse,

    #[error("Reqwest error: {0}")]
    ReqwestError(#[from] reqwest::Error),

    #[error("serde failed: {0}")]
    SerdeError(#[from] serde_json::Error),

    #[error("send error over channel: {0}")]
    SendError(#[from] tokio::sync::mpsc::error::SendError<LLMClientCompletionResponse>),

    #[error("unsupported model")]
    UnSupportedModel,

    #[error("OpenAI api error: {0}")]
    OpenAPIError(#[from] async_openai::error::OpenAIError),

    #[error("Wrong api key type")]
    WrongAPIKeyType,

    #[error("OpenAI does not support completion")]
    OpenAIDoesNotSupportCompletion,

    #[error("Sqlite setup error")]
    SqliteSetupError,

    #[error("tokio mspc error")]
    TokioMpscSendError,

    #[error("Failed to store in sqlite DB")]
    FailedToStoreInDB,

    #[error("Sqlx erorr: {0}")]
    SqlxError(#[from] sqlx::Error),

    #[error("Function calling role but not function call present")]
    FunctionCallNotPresent,

    #[error("Gemini pro does not support prompt completion")]
    GeminiProDoesNotSupportPromptCompletion,
}
</content>
</code_symbol>
<code_symbol>
<name>
LLMClient
</name>
<file_path>
/Users/skcd/test_repo/sidecar/llm_client/src/clients/types.rs
</file_path>
<content>
FILEPATH: /Users/skcd/test_repo/sidecar/llm_client/src/clients/types.rs
#[async_trait]
pub trait LLMClient {
    fn client(&self) -> &LLMProvider;
    async fn stream_completion(
        &self,
        api_key: LLMProviderAPIKeys,
        request: LLMClientCompletionRequest,
        sender: UnboundedSender<LLMClientCompletionResponse>,
    ) -> Result<String, LLMClientError>;
    async fn completion(
        &self,
        api_key: LLMProviderAPIKeys,
        request: LLMClientCompletionRequest,
    ) -> Result<String, LLMClientError>;
    async fn stream_prompt_completion(
        &self,
        api_key: LLMProviderAPIKeys,
        request: LLMClientCompletionStringRequest,
        sender: UnboundedSender<LLMClientCompletionResponse>,
    ) -> Result<String, LLMClientError>;

}
</content>
</code_symbol>
<code_symbol>
<name>
test_llm_type_from_string
</name>
<file_path>
/Users/skcd/test_repo/sidecar/llm_client/src/clients/types.rs
</file_path>
<content>
FILEPATH: /Users/skcd/test_repo/sidecar/llm_client/src/clients/types.rs
    fn test_llm_type_from_string() {
</content>
</code_symbol>
<code_symbol>
<name>
LLMType
</name>
<file_path>
/Users/skcd/test_repo/sidecar/llm_client/src/clients/ollama.rs
</file_path>
<content>
FILEPATH: /Users/skcd/test_repo/sidecar/llm_client/src/clients/ollama.rs
impl LLMType {
    pub fn to_ollama_model(&self) -> Result<String, LLMClientError> {
}
</content>
</code_symbol>
<code_symbol>
<name>
LLMProvider
</name>
<file_path>
/Users/skcd/test_repo/sidecar/llm_client/src/provider.rs
</file_path>
<content>
FILEPATH: /Users/skcd/test_repo/sidecar/llm_client/src/provider.rs
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, Hash, PartialEq, Eq)]
pub enum LLMProvider {
    OpenAI,
    TogetherAI,
    Ollama,
    LMStudio,
    CodeStory(CodeStoryLLMTypes),
    Azure(AzureOpenAIDeploymentId),
    OpenAICompatible,
    Anthropic,
    FireworksAI,
    GeminiPro,
    GoogleAIStudio,
    OpenRouter,
}
</content>
</code_symbol>
<code_symbol>
<name>
LLMProvider
</name>
<file_path>
/Users/skcd/test_repo/sidecar/llm_client/src/provider.rs
</file_path>
<content>
FILEPATH: /Users/skcd/test_repo/sidecar/llm_client/src/provider.rs
impl std::fmt::Display for LLMProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
}
</content>
</code_symbol>
<code_symbol>
<name>
LLMProvider
</name>
<file_path>
/Users/skcd/test_repo/sidecar/llm_client/src/provider.rs
</file_path>
<content>
FILEPATH: /Users/skcd/test_repo/sidecar/llm_client/src/provider.rs
impl LLMProvider {
    pub fn is_codestory(&self) -> bool {
    pub fn is_anthropic_api_key(&self) -> bool {
}
</content>
</code_symbol>
<code_symbol>
<name>
LLMProviderAPIKeys
</name>
<file_path>
/Users/skcd/test_repo/sidecar/llm_client/src/provider.rs
</file_path>
<content>
FILEPATH: /Users/skcd/test_repo/sidecar/llm_client/src/provider.rs
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum LLMProviderAPIKeys {
    OpenAI(OpenAIProvider),
    TogetherAI(TogetherAIProvider),
    Ollama(OllamaProvider),
    OpenAIAzureConfig(AzureConfig),
    LMStudio(LMStudioConfig),
    OpenAICompatible(OpenAICompatibleConfig),
    CodeStory,
    Anthropic(AnthropicAPIKey),
    FireworksAI(FireworksAPIKey),
    GeminiPro(GeminiProAPIKey),
    GoogleAIStudio(GoogleAIStudioKey),
    OpenRouter(OpenRouterAPIKey),
}
</content>
</code_symbol>
<code_symbol>
<name>
LLMProviderAPIKeys
</name>
<file_path>
/Users/skcd/test_repo/sidecar/llm_client/src/provider.rs
</file_path>
<content>
FILEPATH: /Users/skcd/test_repo/sidecar/llm_client/src/provider.rs
impl LLMProviderAPIKeys {
    // Gets the relevant key from the llm provider
    pub fn is_openai(&self) -> bool {
    pub fn provider_type(&self) -> LLMProvider {
    pub fn key(&self, llm_provider: &LLMProvider) -> Option<Self> {
}
</content>
</code_symbol>
<code_symbol>
<name>
Error
</name>
<file_path>
/Users/skcd/test_repo/sidecar/sidecar/src/webserver/types.rs
</file_path>
<content>
FILEPATH: /Users/skcd/test_repo/sidecar/sidecar/src/webserver/types.rs
#[derive(Debug)]
pub struct Error {
    status: StatusCode,
    body: EndpointError<'static>,
}
</content>
</code_symbol>
<code_symbol>
<name>
Error
</name>
<file_path>
/Users/skcd/test_repo/sidecar/sidecar/src/webserver/types.rs
</file_path>
<content>
FILEPATH: /Users/skcd/test_repo/sidecar/sidecar/src/webserver/types.rs
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
}
</content>
</code_symbol>
<code_symbol>
<name>
Error
</name>
<file_path>
/Users/skcd/test_repo/sidecar/sidecar/src/webserver/types.rs
</file_path>
<content>
FILEPATH: /Users/skcd/test_repo/sidecar/sidecar/src/webserver/types.rs
impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
}
</content>
</code_symbol>
<code_symbol>
<name>
Error
</name>
<file_path>
/Users/skcd/test_repo/sidecar/sidecar/src/webserver/types.rs
</file_path>
<content>
FILEPATH: /Users/skcd/test_repo/sidecar/sidecar/src/webserver/types.rs
impl Error {
    pub fn internal<S: std::fmt::Display>(message: S) -> Self {
}
</content>
</code_symbol>
<code_symbol>
<name>
Error
</name>
<file_path>
/Users/skcd/test_repo/sidecar/sidecar/src/webserver/types.rs
</file_path>
<content>
FILEPATH: /Users/skcd/test_repo/sidecar/sidecar/src/webserver/types.rs
impl From<anyhow::Error> for Error {
    fn from(value: anyhow::Error) -> Self {
}
</content>
</code_symbol>
</code_symbol_outline_list>"#;
    let llm_request = LLMClientCompletionRequest::new(
        LLMType::Llama3_1_8bInstruct,
        vec![
            LLMClientMessage::system(system_message.to_owned()),
            LLMClientMessage::user(user_message.to_owned()),
        ],
        0.2,
        None,
    );
    let client = GroqClient::new();
    let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
    let start_instant = std::time::Instant::now();
    let response = client
        .stream_completion(
            LLMProviderAPIKeys::GroqProvider(GroqProviderAPIKey::new(
                "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".to_owned(),
            )),
            llm_request,
            sender,
        )
        .await;
    println!(
        "response {}:\n{}",
        start_instant.elapsed().as_millis(),
        response.expect("to work always")
    );
}
