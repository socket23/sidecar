//! Generates the git diff given a repo location and file path
//! This should move to the editor and it will

#[tokio::main]
async fn main() {
    let root_directory = "/Users/skcd/scratch/sidecar";
    let fs_file_path = "/Users/skcd/scratch/sidecar/sidecar/src/bin/git_diff.rs";
    let mut output = "".to_owned();
    let _ = run_command(&root_directory, fs_file_path, &mut output).await;
    // println!("output:\n{}", &output);
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

    // now we parse the git-diff in a very very hacky way, bear with me
    // example output:
    println!("git_diff_output\n{}", &output);
    let results = parse_git_diff_output_full_length(&output);
    println!("{:?}", results);

    Ok(())
}

#[derive(Debug)]
struct GitDiffOutput {
    fs_file_path: String,
    old_version: String,
    new_version: String,
}

// exmaple output:
// diff --git a/sidecar/src/bin/git_diff.rs b/sidecar/src/bin/git_diff.rs
// index 1c5d775c..c78ff82f 100644
// --- a/sidecar/src/bin/git_diff.rs
// +++ b/sidecar/src/bin/git_diff.rs
// @@ -50,5 +50,8 @@ async fn run_command(
//          output.push('\n');
//      }

// +    // now we parse the git-diff in a very very hacky way, bear with me
// +    //
// +
//      Ok(())
//  }

fn parse_git_diff_output_full_length(git_diff: &str) -> Vec<GitDiffOutput> {
    let mut diff_outputs = Vec::new();
    let git_diff_lines = git_diff.lines().into_iter().collect::<Vec<_>>();
    println!("git_diff_lines_len({})", git_diff_lines.len());
    // let sections = git_diff.split("diff --git").skip(1);

    for (mut idx, _git_diff_line) in git_diff_lines.iter().enumerate() {
        // let lines: Vec<&str> = section.lines().collect();
        if !is_valid_diff_section(idx, &git_diff_lines) {
            continue;
        }

        // now we want to go until we find the next such index
        let index_limit = (idx + 1..=git_diff_lines.len() - 1)
            .find(|idx| is_valid_diff_section(*idx, &git_diff_lines));
        println!("index_limit({:?})::idx({})", index_limit, idx);
        let section_limit = if index_limit.is_none() {
            // this implies the end of the section
            git_diff_lines.len() - 1
        } else {
            index_limit.expect("is_none check above to hold") - 1
        };

        let fs_file_path = extract_file_path(&git_diff_lines[idx]);
        // we start after the diff git -- and index {crap} lines
        let slice_limits = &git_diff_lines[idx + 2..=section_limit];
        let (old_version, new_version) = extract_versions(slice_limits);

        diff_outputs.push(GitDiffOutput {
            fs_file_path,
            old_version,
            new_version,
        });
        idx = section_limit;
    }

    diff_outputs
}

fn is_valid_diff_section(line_index: usize, git_diff_lines: &Vec<&str>) -> bool {
    line_index + 1 < git_diff_lines.len()
        && git_diff_lines[line_index].starts_with("diff --git")
        && git_diff_lines[line_index + 1].starts_with("index")
}

fn extract_file_path(line: &str) -> String {
    println!("extract_file_path::({})", line);
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() >= 3 {
        if parts[2].starts_with("a/") {
            parts[2].trim_start_matches("a/").to_owned()
        } else {
            parts[2].trim().to_owned()
        }
    } else {
        String::new()
    }
}

fn extract_versions(lines: &[&str]) -> (String, String) {
    let mut old_version = String::new();
    let mut new_version = String::new();
    for line in lines {
        if line.starts_with("@@") {
            continue;
        }
        if line.starts_with("---") {
            continue;
        }
        if line.starts_with("+++") {
            continue;
        }

        if line.starts_with('-') {
            old_version.push_str(&line[1..]);
            old_version.push('\n');
        } else if line.starts_with('+') {
            new_version.push_str(&line[1..]);
            new_version.push('\n');
        } else {
            old_version.push_str(&line[1..]);
            old_version.push_str("\n");
            new_version.push_str(&line[1..]);
            new_version.push_str("\n");
        }
    }

    (
        old_version.trim().to_string(),
        new_version.trim().to_string(),
    )
}
