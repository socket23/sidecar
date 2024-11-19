use clap::{Args as ClapArgs, Parser};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Define the command-line arguments
#[derive(Parser, Debug)]
#[command(
    author = "Your Name",
    version = "1.0",
    about = "SWE-Bench Sidecar Runner"
)]
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
}

/// Define the SWEbenchInstance arguments
#[derive(ClapArgs, Debug)]
struct SWEbenchInstanceArgs {
    /// Repository URL
    #[arg(long)]
    repo: String,

    /// Instance ID
    #[arg(long)]
    instance_id: String,

    /// Base commit hash
    #[arg(long)]
    base_commit: String,

    /// Patch content
    #[arg(long)]
    patch: String,

    /// Test patch content
    #[arg(long)]
    test_patch: String,

    /// Problem statement
    #[arg(long)]
    problem_statement: String,

    /// Hints text
    #[arg(long)]
    hints_text: String,

    /// Creation timestamp
    #[arg(long)]
    created_at: String,

    /// Version
    #[arg(long)]
    version: String,

    /// Fail-to-pass code
    #[arg(long)]
    fail_to_pass: String,

    /// Pass-to-pass code
    #[arg(long)]
    pass_to_pass: String,

    /// Environment setup commit hash
    #[arg(long)]
    environment_setup_commit: String,
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command-line arguments
    let args = CliArgs::parse();
    let editor_url = args.editor_url.to_owned();
    let timeout = args.timeout;
    let input_path = args.input;
    let input_content = tokio::fs::read(input_path).await.expect("path content");
    let input_parts: InputParts =
        serde_json::from_slice(&input_content).expect("Parse the serde json");
    println!("args:{:?}", input_parts);
    println!("timeout:{}", timeout);
    println!("input_pargs:{:?}", editor_url);
    Ok(())
}
