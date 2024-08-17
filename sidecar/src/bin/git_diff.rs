//! Generates the git diff given a repo location and file path
//! This should move to the editor and it will

#[tokio::main]
async fn main() {
    let root_directory = "/Users/skcd/scratch/sidecar";
    let fs_file_path = "/Users/skcd/scratch/sidecar/sidecar/src/bin/git_diff.rs";
    let mut output = "".to_owned();
    let _ = run_command(&root_directory, fs_file_path, &mut output).await;
    println!("output:\n{}", &output);
}

use std::fs::File as StdFile;
use std::process::Stdio;
use tempfile::NamedTempFile;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

async fn run_command(
    root_directory: &str,
    _fs_file_path: &str,
    output: &mut String,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create a temporary file
    let tmpfile = NamedTempFile::new_in("/tmp")?;
    let tmpfile_path = tmpfile.path().to_path_buf();
    println!("tmpfile: {:?}", &tmpfile_path);

    // Run the git diff command, directing stdout to the temporary file
    let status = Command::new("git")
        .current_dir(root_directory)
        .arg("diff")
        .arg("--no-prefix")
        .arg("-U1000")
        .stdout(Stdio::from(StdFile::create(&tmpfile_path)?))
        .status()
        .await?;

    if !status.success() {
        return Err(format!("Command failed with exit code: {:?}", status.code()).into());
    }

    // Asynchronously read the temporary file and stream the contentletfile = File::open(tmpfile_path)?;
    let file = File::open(tmpfile_path).await?;
    let mut reader = BufReader::new(file).lines();

    while let Some(line) = reader.next_line().await? {
        output.push_str(&line);
        output.push('\n');
    }

    Ok(())
}
