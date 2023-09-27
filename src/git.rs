use anyhow::anyhow;
// We are going to use this to try and load a repository and see what the last
// timestamp of the file was when it was committed. This will help us get
// more context and help improve the search even more
use tokio::process::Command;

pub async fn get_last_commit_timestamp(
    repo_path: &str,
    file_path: &str,
) -> Result<String, anyhow::Error> {
    // we need to execute this command on the cli to get the value and set the
    // current working directory to the root path of the repository
    // git log -1 --format=%ct -- {file_path} (cwd: repo_path)
    let output = Command::new("git")
        .arg("log")
        .arg("-1")
        .arg("--format=%ct")
        .arg("--")
        .arg(file_path)
        .current_dir(repo_path)
        .output()
        .await
        .map_err(|op| {
            anyhow!(
                "Failed to execute git log -1 --format=%ct -- {file_path} (cwd: repo_path): {}",
                op,
            )
        })?;

    if output.status.success() {
        let timestamp = String::from_utf8_lossy(&output.stdout);
        println!("Timestamp: {}", timestamp.trim());
        Ok(timestamp.trim().to_owned())
    } else {
        eprintln!("Error: {:?}", String::from_utf8_lossy(&output.stderr));
        Err(anyhow!(
            "Failed to execute git log -1 --format=%ct -- {file_path} (cwd: repo_path)"
        ))
    }
}
