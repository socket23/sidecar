use std::process::Stdio;

use serde_json::json;
use tokio::{io::AsyncReadExt, process::Command};

async fn get_diff_patch(git_dname: &str) -> String {
    let mut child = Command::new("git")
        .arg("-C")
        .arg(git_dname)
        .arg("--no-pager") // Add this line to disable the pager
        .arg("diff")
        .stdout(Stdio::piped())
        .spawn()
        .expect("to work");
    let _ = child.wait().await;
    let mut stdout = child.stdout.take().expect("Failed to get stdout");
    let mut output = Vec::new();
    stdout.read_to_end(&mut output).await.expect("to work");

    let output_string = String::from_utf8_lossy(&output);
    println!("Output: {}", output_string);
    output_string.to_string()
}

#[tokio::main]
async fn main() {
    let instance_id = "sympy__sympy-18532".to_owned();
    let folder_path = "/var/folders/bq/1dbw218x1zq3r3c5_gqxgdgr0000gn/T/tmp21sxtah_".to_owned();
    let git_diff = get_diff_patch(&folder_path).await;
    println!("Whats the git diff\n");
    println!("{}", git_diff);
    let prediction_json = json!({
        "instance_id": instance_id.to_owned(),
        "model_name_or_path": "codestory-mixed".to_owned(),
        "model_patch": get_diff_patch(&folder_path).await,
    });

    let prediction_output = "/Users/skcd/scratch/swe_bench/predictions/full---gpt-4o/".to_owned()
        + &instance_id
        + ".jsonl";

    let _ = dbg!(
        tokio::fs::write(
            prediction_output,
            serde_json::to_string(&prediction_json).expect("serde to not fail"),
        )
        .await
    );
}
