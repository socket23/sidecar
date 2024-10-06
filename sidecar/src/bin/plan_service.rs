use futures::future::try_join_all;
use sidecar::agentic::tool::input::ToolInput;
use sidecar::agentic::tool::lsp::file_diagnostics::{FileDiagnostics, FileDiagnosticsInput};
use sidecar::agentic::tool::output::ToolOutput;
use sidecar::agentic::tool::r#type::Tool;
use std::io::{self, Write};
use std::{path::PathBuf, sync::Arc};

use tokio::fs::File;
use tokio::io::AsyncReadExt;

use llm_client::{
    broker::LLMBroker,
    clients::types::LLMType,
    config::LLMBrokerConfiguration,
    provider::{AnthropicAPIKey, LLMProvider, LLMProviderAPIKeys, OpenAIProvider},
};
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
            plan::plan::Plan,
            plan::plan_step::PlanStep,
            plan::service::PlanService,
        },
    },
    chunking::{editor_parsing::EditorParsing, languages::TSLanguageParsing},
    inline_completion::symbols_tracker::SymbolTrackerInline,
    user_context::types::{FileContentValue, UserContext},
};

fn default_index_dir() -> PathBuf {
    match directories::ProjectDirs::from("ai", "codestory", "sidecar") {
        Some(dirs) => dirs.data_dir().to_owned(),
        None => "codestory_sidecar".into(),
    }
}

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "plan_executor", about = "A simple plan execution tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Next,   // Execute the next step in the plan
    Append, // Append a new step to the plan
}

fn edit_step(step: &mut PlanStep) {
    println!("Current step title:");
    println!("{}", step.title());
    println!("\nEnter new title (press Enter to keep current title):");

    let mut new_title = String::new();
    io::stdin().read_line(&mut new_title).unwrap();
    new_title = new_title.trim().to_string();

    if !new_title.is_empty() {
        step.edit_title(new_title);
    }

    println!("\nCurrent step description:");
    println!("{}", step.description());
    println!("\nEnter new description (press Enter twice to finish):");

    let mut new_description = String::new();
    loop {
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        if input.trim().is_empty() {
            break;
        }
        new_description.push_str(&input);
    }

    step.edit_description(new_description.trim().to_string());
    println!("Step updated successfully.");
}

fn append_step(plan: &mut Plan) {
    println!("Appending a new step to the plan.");

    println!("Enter step title:");
    let mut title = String::new();
    io::stdin().read_line(&mut title).unwrap();
    let title = title.trim().to_string();

    println!("Enter step description:");
    let mut description = String::new();
    io::stdin().read_line(&mut description).unwrap();
    let description = description.trim().to_string();

    println!("Enter file to edit (path):");
    let mut file_to_edit = String::new();
    io::stdin().read_line(&mut file_to_edit).unwrap();
    let file_to_edit = file_to_edit.trim().to_string();

    let new_step = PlanStep::new(
        uuid::Uuid::new_v4().to_string(),
        plan.steps().len(),
        vec![file_to_edit],
        title,
        description,
        UserContext::new(vec![], vec![], None, vec![]),
    );

    plan.add_step(new_step);
    println!("New step appended successfully.");
}

#[tokio::main]
async fn main() {
    let request_id = uuid::Uuid::new_v4();
    let request_id_str = request_id.to_string();
    let parea_url = format!(
        r#"https://app.parea.ai/logs?colViz=%7B%220%22%3Afalse%2C%221%22%3Afalse%2C%222%22%3Afalse%2C%223%22%3Afalse%2C%22error%22%3Afalse%2C%22deployment_id%22%3Afalse%2C%22feedback_score%22%3Afalse%2C%22time_to_first_token%22%3Afalse%2C%22scores%22%3Afalse%2C%22start_timestamp%22%3Afalse%2C%22user%22%3Afalse%2C%22session_id%22%3Afalse%2C%22target%22%3Afalse%2C%22experiment_uuid%22%3Afalse%2C%22dataset_references%22%3Afalse%2C%22in_dataset%22%3Afalse%2C%22event_type%22%3Afalse%2C%22request_type%22%3Afalse%2C%22evaluation_metric_names%22%3Afalse%2C%22request%22%3Afalse%2C%22calling_node%22%3Afalse%2C%22edges%22%3Afalse%2C%22metadata_evaluation_metric_names%22%3Afalse%2C%22metadata_event_type%22%3Afalse%2C%22metadata_0%22%3Afalse%2C%22metadata_calling_node%22%3Afalse%2C%22metadata_edges%22%3Afalse%2C%22metadata_root_id%22%3Afalse%7D&filter=%7B%22filter_field%22%3A%22meta_data%22%2C%22filter_operator%22%3A%22equals%22%2C%22filter_key%22%3A%22root_id%22%2C%22filter_value%22%3A%22{request_id_str}%22%7D&page=1&page_size=50&time_filter=1m"#
    );
    println!("===========================================\nRequest ID: {}\nParea AI: {}\n===========================================", request_id.to_string(), parea_url);
    let editor_url = "http://localhost:42427".to_owned();
    let anthropic_api_keys = LLMProviderAPIKeys::Anthropic(AnthropicAPIKey::new("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".to_owned()));
    let anthropic_llm_properties = LLMProperties::new(
        LLMType::ClaudeSonnet,
        LLMProvider::Anthropic,
        anthropic_api_keys.clone(),
    );

    let _o1_properties = LLMProperties::new(
        LLMType::O1Preview,
        LLMProvider::OpenAI,
        LLMProviderAPIKeys::OpenAI(OpenAIProvider::new("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".to_owned())),
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
        // for our testing workflow we want to apply the edits directly
        ToolBrokerConfiguration::new(None, true),
        LLMProperties::new(
            LLMType::Gpt4O,
            LLMProvider::OpenAI,
            LLMProviderAPIKeys::OpenAI(OpenAIProvider::new(
                "sk-proj-BLaSMsWvoO6FyNwo9syqT3BlbkFJo3yqCyKAxWXLm4AvePtt".to_owned(),
            )),
        ),
    ));

    let mut plan_storage_path = default_index_dir();
    plan_storage_path = plan_storage_path.join("plans");

    // check if the plan_storage_path_exists
    if tokio::fs::metadata(&plan_storage_path).await.is_err() {
        tokio::fs::create_dir(&plan_storage_path)
            .await
            .expect("directory creation to not fail");
    }

    let (_plan_storage_path, plan_id) = {
        let mut plan_storage_path = plan_storage_path.clone();
        // replace plan_id here with a static id if you want to reuse the plan loading
        let plan_id = uuid::Uuid::new_v4().to_string();
        plan_storage_path = plan_storage_path
            .join(plan_id.to_owned())
            .with_extension("json");
        (plan_storage_path, plan_id)
    };

    // edit step
    // let plan_storage_path = PathBuf::from("/Users/zi/Library/Application Support/ai.codestory.sidecar/plans/6e5eb8c1-054f-4510-aca4-61676c73168e.json");

    // add step
    // let plan_storage_path = PathBuf::from("/Users/zi/Library/Application Support/ai.codestory.sidecar/plans/c6580a8e-5d4f-4138-9fce-69d1a067bf72.json");

    // fix lsp
    // let plan_storage_path = PathBuf::from("/Users/zi/Library/Application Support/ai.codestory.sidecar/plans/c6580a8e-5d4f-4138-9fce-69d1a067bf72.json");

    // add file_path field
    // let plan_storage_path = PathBuf::from("/Users/zi/Library/Application Support/ai.codestory.sidecar/plans/65b85dcb-72e9-498f-912c-036b02845319.json");

    // fetch lsp diags
    // let plan_storage_path = PathBuf::from("/Users/zi/Library/Application Support/ai.codestory.sidecar/plans/93bdc6e5-4c22-4fe0-b341-b99e005d6f97.json");
    // let plan_storage_path = PathBuf::from("/Users/zi/Library/Application Support/ai.codestory.sidecar/plans/4fa3986c-1c78-4381-b6a0-0be7f8f88512.json");

    // file_diag tool
    // let plan_storage_path = PathBuf::from("/Users/zi/Library/Application Support/ai.codestory.sidecar/plans/848232c4-0c81-48a7-9901-967a1442b371.json");

    // massive dooby incoming
    let plan_storage_path = PathBuf::from("/Users/zi/Library/Application Support/ai.codestory.sidecar/plans/1bd5c95c-c4e6-4b4b-9ca1-d4269635afa7.json");

    let (sender, mut _receiver) = tokio::sync::mpsc::unbounded_channel();

    let event_properties = SymbolEventMessageProperties::new(
        SymbolEventRequestId::new(request_id_str.to_owned(), request_id_str.to_owned()),
        sender.clone(),
        editor_url.to_owned(),
        tokio_util::sync::CancellationToken::new(),
    );

    let symbol_manager = Arc::new(SymbolManager::new(
        tool_broker.clone(),
        symbol_broker.clone(),
        editor_parsing.clone(),
        anthropic_llm_properties.clone(),
    ));

    let tool_box = Arc::new(ToolBox::new(
        tool_broker.clone(),
        symbol_broker.clone(),
        editor_parsing.clone(),
    ));

    // let user_query =
    //     "add a command that fetches diagnostics for the last edited step's edited file path."
    //         .to_string();

    let user_query =
        "I want to fetch the lsp errors from each step's files_to_edit field (just take the first in the vec).

Then, we should use the generator.rs logic to generate steps. Write a convenience method somewhere to handle a LSP-based input to the user_message, updating followup_chat in agent.rs accordingly. UserContext should have a new boolean field to communicate that we are doing an LSP-related run.

Additionally, we need to do some wiring in webserver.rs, to create endpoint.

overall, we need an endpoint that, when hit, fetchs all diagnostic messages present in the files_to_edit of the steps up to checkpoint, then feed that into a user_query for generator.rs in order to generate new steps from this context.
"
            .to_string();

    let _initial_context = String::from("");

    // let context_files = vec![
    //     "/Users/zi/codestory/sidecar/sidecar/src/bin/get_diagnostics.rs",
    //     "/Users/zi/codestory/sidecar/sidecar/src/agentic/tool/plan/plan_step.rs",
    //     "/Users/zi/codestory/sidecar/sidecar/src/agentic/tool/plan/plan.rs",
    //     "/Users/zi/codestory/sidecar/sidecar/src/agentic/tool/plan/service.rs",
    //     "/Users/zi/codestory/sidecar/sidecar/src/bin/plan_service.rs",
    //     "/Users/zi/codestory/sidecar/sidecar/src/agentic/tool/lsp/diagnostics.rs",
    // ];

    let context_files = vec![
        "/Users/zi/codestory/sidecar/sidecar/src/bin/webserver.rs",
        "/Users/zi/codestory/sidecar/sidecar/src/webserver/agent.rs",
        "/Users/zi/codestory/sidecar/sidecar/src/user_context/types.rs",
        "/Users/zi/codestory/sidecar/sidecar/src/agentic/tool/plan/generator.rs",
        "/Users/zi/codestory/sidecar/sidecar/src/agentic/tool/lsp/file_diagnostics.rs",
        "/Users/zi/codestory/sidecar/sidecar/src/agentic/tool/plan/service.rs",
    ];

    let file_futures: Vec<_> = context_files
        .into_iter()
        .map(|path| read_file(PathBuf::from(path)))
        .collect();

    let file_contents = try_join_all(file_futures).await.unwrap();

    let user_context = UserContext::new(vec![], file_contents, None, vec![]); // this is big, should be passed using references

    let _ui_sender = event_properties.ui_sender();

    let plan_service = PlanService::new(tool_box.clone(), symbol_manager);

    // let path = "/Users/skcd/scratch/sidecar/sidecar/src/bin/plan.json";

    // when adding variables to the JSON, just use file_content_map (copy what you see in global context)

    let plan_storage_path_str = plan_storage_path
        .clone()
        .to_str()
        .expect("to work")
        .to_owned();

    println!("Plan Storage Path:\n{}", &plan_storage_path_str);

    // toggle this to use o1-preview
    let is_deep_reasoning = false;

    let plan = if tokio::fs::metadata(plan_storage_path.clone()).await.is_ok() {
        plan_service
            .load_plan(plan_storage_path.to_str().expect("to work"))
            .await
            .unwrap()
    } else {
        plan_service
            .create_plan(
                plan_id,
                user_query,
                user_context,
                is_deep_reasoning,
                plan_storage_path
                    .to_str()
                    .map(|plan_str| plan_str.to_owned())
                    .expect("PathBuf to str conversion to not fail on platforms"),
                event_properties.clone(),
            )
            .await
            .expect("Failed to create new plan")
    };

    let _ = plan_service.save_plan(&plan, &plan_storage_path_str).await;

    println!("Welcome to Agentic Planning.");
    println!();
    println!(
        "Your plan has {} steps. We are at step {}.",
        &plan.steps().len(),
        &plan.checkpoint().unwrap_or_default() + 1,
    );
    println!();

    loop {
        let plan = plan_service
            .load_plan(&plan_storage_path_str)
            .await
            .unwrap();
        let _steps = plan.steps();
        let checkpoint = plan.checkpoint().unwrap_or_default();
        let context = plan_service.prepare_context(plan.steps(), checkpoint).await;

        let mut plan = plan_service
            .load_plan(&plan_storage_path_str)
            .await
            .unwrap();
        let step_to_execute = plan.steps_mut().get_mut(checkpoint).unwrap();

        println!("Next: {}", step_to_execute.title());

        println!("[1] Execute");
        println!("[2] Edit");
        println!("[3] Show Description");
        println!("[4] Append Step");
        println!(
            "[5] Fetch Diagnostics for {}",
            &step_to_execute
                .file_to_edit()
                .unwrap_or("No file to edit".to_owned())
        );
        println!("[6] Exit");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        match input.trim() {
            "1" | "execute" => {
                // using file as store for Plan
                let _ = match plan_service
                    .execute_step(step_to_execute, context, event_properties.clone())
                    .await
                {
                    Ok(_) => {
                        if let Some(index) = plan.checkpoint() {
                            println!("Checkpoint {} complete", index);
                            plan.increment_checkpoint();
                        } else {
                            plan.set_checkpoint(1) // this is a hack
                        }

                        // if plan.checkpoint() < plan.final_checkpoint() {

                        // } else {
                        //     println!("Reached final checkpoint. Plan execution complete.");
                        // }

                        // save!
                        if let Err(e) = plan_service.save_plan(&plan, &plan_storage_path_str).await
                        {
                            eprintln!("Error saving plan: {}", e)
                        }
                    }
                    Err(e) => println!("Error executing step: {}", e),
                };
            }
            "2" | "edit" => {
                edit_step(step_to_execute);
                if let Err(e) = plan_service.save_plan(&plan, &plan_storage_path_str).await {
                    eprintln!("Error saving plan: {}", e);
                }
            }
            "3" | "show" => {
                println!("\nStep Description:");
                println!("{}", step_to_execute.description());
                println!(); // Add a blank line for readability
            }
            "4" | "append" => {
                append_step(&mut plan);
                if let Err(e) = plan_service.save_plan(&plan, &plan_storage_path_str).await {
                    eprintln!("Error saving plan: {}", e);
                }
            }
            "5" | "fetch" => {
                if let Some(file_path) = step_to_execute.file_to_edit() {
                    println!("Fetching diagnostics for file: {}", file_path);

                    let file_diagnostics_input =
                        FileDiagnosticsInput::new(file_path.to_string(), editor_url.clone(), true);

                    let diagnostics_client = FileDiagnostics::new();
                    match diagnostics_client
                        .invoke(ToolInput::FileDiagnostics(file_diagnostics_input))
                        .await
                    {
                        Ok(ToolOutput::FileDiagnostics(output)) => {
                            let diagnostics = output.get_diagnostics();
                            if diagnostics.is_empty() {
                                println!("No diagnostics found.");
                            } else {
                                println!("Diagnostics:");
                                for (i, diagnostic) in diagnostics.iter().enumerate() {
                                    println!(
                                        "{}. {} (at {:?})",
                                        i + 1,
                                        diagnostic.message(),
                                        diagnostic.range()
                                    );
                                }
                            }
                        }
                        Err(e) => println!("Error fetching diagnostics: {}", e),
                        _ => println!("Unexpected output type from FileDiagnostics"),
                    }
                } else {
                    println!("No file to edit available for the current step.");
                }
            }

            "6" | "exit" => break,
            _ => println!("Invalid command. Please try again."),
        }

        println!(); // Add a blank line for readability
    }

    println!("Exiting program. Check plan's checkpoint value in JSON before next run");
}

async fn read_file(path: PathBuf) -> Result<FileContentValue, std::io::Error> {
    let mut file = File::open(&path).await?;
    let mut content = String::new();
    file.read_to_string(&mut content).await?;
    Ok(FileContentValue::new(
        path.to_string_lossy().into_owned(),
        content,
        "rs".to_owned(),
    ))
}
