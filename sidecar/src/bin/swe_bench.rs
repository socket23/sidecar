use clap::Parser;
use llm_client::{
    broker::LLMBroker,
    clients::types::LLMType,
    config::LLMBrokerConfiguration,
    provider::{AnthropicAPIKey, GoogleAIStudioKey, LLMProvider, LLMProviderAPIKeys},
};
use serde::{Deserialize, Serialize};
use sidecar::{
    agentic::{
        symbol::{
            events::{input::SymbolEventRequestId, message_event::SymbolEventMessageProperties},
            identifier::LLMProperties,
            manager::SymbolManager,
            tool_box::ToolBox,
        },
        tool::{
            broker::{ToolBroker, ToolBrokerConfiguration},
            code_edit::models::broker::CodeEditBroker,
            session::service::SessionService,
        },
    },
    chunking::{editor_parsing::EditorParsing, languages::TSLanguageParsing},
    inline_completion::symbols_tracker::SymbolTrackerInline,
    repo::types::RepoRef,
};
use std::{path::PathBuf, sync::Arc};

/// Define the command-line arguments
#[derive(Parser, Debug)]
#[command(author = "skcd", version = "1.0", about = "SWE-Bench Sidecar Runner")]
struct CliArgs {
    /// Git directory name
    #[arg(long)]
    timeout: usize,

    /// Endpoint URL
    #[arg(long)]
    editor_url: String,

    /// Timeout in seconds
    #[arg(long)]
    input: PathBuf,

    /// Anthropic api key
    #[arg(long)]
    anthropic_api_key: String,

    /// The run id for the current run
    #[arg(long)]
    run_id: String,

    #[arg(long)]
    repo_name: String,
}

/// Define the SWEbenchInstance struct for serialization
#[derive(Debug, Serialize, Deserialize)]
struct SWEbenchInstance {
    repo: String,
    instance_id: String,
    base_commit: String,
    patch: String,
    test_patch: String,
    problem_statement: String,
    hints_text: String,
    created_at: String,
    version: String,
    #[serde(rename = "FAIL_TO_PASS")]
    fail_to_pass: String,
    #[serde(rename = "PASS_TO_PASS")]
    pass_to_pass: String,
    environment_setup_commit: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct InputParts {
    git_drname: String,
    instance: SWEbenchInstance,
}

fn default_index_dir() -> PathBuf {
    match directories::ProjectDirs::from("ai", "codestory", "sidecar") {
        Some(dirs) => dirs.data_dir().to_owned(),
        None => "codestory_sidecar".into(),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command-line arguments
    let args = CliArgs::parse();

    let editor_parsing = Arc::new(EditorParsing::default());
    let symbol_broker = Arc::new(SymbolTrackerInline::new(editor_parsing.clone()));
    let llm_broker = Arc::new(
        LLMBroker::new(LLMBrokerConfiguration::new(default_index_dir()))
            .await
            .expect("to initialize properly"),
    );
    let tool_broker = Arc::new(ToolBroker::new(
        llm_broker.clone(),
        Arc::new(CodeEditBroker::new()),
        symbol_broker.clone(),
        Arc::new(TSLanguageParsing::init()),
        ToolBrokerConfiguration::new(None, true),
        LLMProperties::new(
            LLMType::GeminiPro,
            LLMProvider::GoogleAIStudio,
            LLMProviderAPIKeys::GoogleAIStudio(GoogleAIStudioKey::new("".to_owned())),
        ),
    ));

    let symbol_tracker = Arc::new(SymbolTrackerInline::new(editor_parsing.clone()));

    let symbol_manager = Arc::new(SymbolManager::new(
        tool_broker.clone(),
        symbol_tracker.clone(),
        editor_parsing.clone(),
        LLMProperties::new(
            LLMType::ClaudeSonnet,
            LLMProvider::Anthropic,
            LLMProviderAPIKeys::Anthropic(AnthropicAPIKey::new("".to_owned())),
        ),
    ));

    let tool_box = Arc::new(ToolBox::new(tool_broker, symbol_broker, editor_parsing));

    let editor_url = args.editor_url.to_owned();
    let _timeout = args.timeout;
    let input_path = args.input;
    let run_id = args.run_id.to_owned();
    let repo_name = args.repo_name.to_owned();
    let anthropic_api_key = args.anthropic_api_key.to_owned();
    let input_content = tokio::fs::read(input_path).await.expect("path content");
    let input_parts: InputParts =
        serde_json::from_slice(&input_content).expect("Parse the serde json");

    let model_configuration = LLMProperties::new(
        LLMType::ClaudeSonnet,
        LLMProvider::Anthropic,
        LLMProviderAPIKeys::Anthropic(AnthropicAPIKey::new(anthropic_api_key)),
    );

    let session_id = format!(
        "{}-{}",
        input_parts.instance.instance_id,
        run_id.to_string()
    );

    println!("session_id:{}", &session_id);

    // Creates the unique path for the session
    let session_path = default_index_dir().join("session");
    // check if the plan_storage_path_exists
    if tokio::fs::metadata(&session_path).await.is_err() {
        tokio::fs::create_dir(&session_path)
            .await
            .expect("directory creation to not fail");
    }
    let session_path = session_path.join(session_id.to_owned());
    let storage_path = session_path
        .to_str()
        .expect("path conversion to work on all platforms")
        .to_owned();

    let initial_exchange_id = 0;

    let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
    let cancellation_token = tokio_util::sync::CancellationToken::new();
    let message_properties = SymbolEventMessageProperties::new(
        SymbolEventRequestId::new(
            initial_exchange_id.to_string().to_owned(),
            run_id.to_string(),
        ),
        sender.clone(),
        editor_url,
        cancellation_token.clone(),
        model_configuration,
    );

    let problem_with_test = format!(
        "GitHub issue: {}\n\nTest to pass: {}",
        input_parts.instance.problem_statement,
        r#" class FilterableFieldTests(TestCase):
    @classmethod
    def setUpTestData(cls):
        cls.metadata_type = ProductMetaDataType.objects.create(
            label='brand',
            filterable=True
        )
        cls.metadata = ProductMetaData.objects.create(
            value='Dark Vador',
            metadata_type=cls.metadata_type
        )

    def test_filterable_field_raises_error(self):
        """
        Test that filtering on a foreign key to a model with a field named 'filterable'
        raises NotSupportedError.
        """
        from django.db.utils import NotSupportedError
        msg = 'ProductMetaDataType is disallowed in the filter clause.'
        with self.assertRaisesMessage(NotSupportedError, msg):
            # Filter both directly on metadata_type and through the relationship
            list(ProductMetaData.objects.filter(
                value='Dark Vador',
                metadata_type=self.metadata_type,
                metadata_type__filterable=True
            ))

    def test_filterable_field_renamed_works(self):
        """
        Test that filtering works when the field is renamed to something else.
        """
        # Temporarily rename the field for this test
        old_field = ProductMetaDataType._meta.get_field('filterable')
        try:
            old_field.name = 'filterable_test'
            # This should not raise NotSupportedError
            qs = ProductMetaData.objects.filter(
                value='Dark Vador',
                metadata_type=self.metadata_type
            )
            self.assertEqual(len(qs), 1)
            self.assertEqual(qs[0].value, 'Dark Vador')
        finally:
            # Restore the field name
            old_field.name = 'filterable'

    
    def test_filterable_field_raises_error(self):
        """
        Test that filtering on a foreign key to a model with a field named 'filterable'
        raises NotSupportedError.
        """
        from django.db.utils import NotSupportedError
        msg = 'ProductMetaDataType is disallowed in the filter clause.'
        with self.assertRaisesMessage(NotSupportedError, msg):
            # Filter both directly on metadata_type and through the relationship
            list(ProductMetaData.objects.filter(
                value='Dark Vador',
                metadata_type=self.metadata_type,
                metadata_type__filterable=True
            ))
"#
    );

    let session_service = SessionService::new(tool_box.clone(), symbol_manager);
    println!("session_service::tool_use_agentic_swe_bench");
    // generate tests to test out the code gen output
    let _ = session_service
        .tool_use_agentic_swe_bench(
            session_id,
            storage_path,
            repo_name,
            problem_with_test,
            initial_exchange_id.to_string(),
            vec![],
            vec![],
            "bash".to_owned(),
            vec![],
            RepoRef::local(&input_parts.git_drname).expect("to work"),
            input_parts.git_drname.to_owned(),
            tool_box,
            llm_broker,
            message_properties,
        )
        .await;
    Ok(())
}
