use std::collections::HashSet;

use anyhow::Context;
use gix::{
    bstr::ByteSlice,
    credentials::helper::Action,
    diff::blob::{sink::Counter, UnifiedDiffBuilder},
    objs::tree::EntryMode,
    Commit, Id,
};
use sidecar::repo::types::{Backend, RepoRef};

/// getting statistics out of git commits

#[derive(Debug, Default)]
pub struct CommitStatistics {
    author: String,
    file_insertions: usize,
    file_deletions: usize,
    title: String,
    body: Option<String>,
    git_diff: String,
    files_modified: HashSet<String>,
    line_insertions: u32,
    line_deletions: u32,
}

struct GitCommitIterator<'a> {
    commit: Commit<'a>,
    parent: Option<Id<'a>>,
}

#[derive(Debug)]
struct CommitError;

impl std::fmt::Display for CommitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unreachable!("commit error should not happen");
    }
}

impl std::error::Error for CommitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

impl<'a> Iterator for GitCommitIterator<'a> {
    type Item = CommitStatistics;

    fn next(&mut self) -> Option<Self::Item> {
        let Some(parent_id) = self.parent else {
            return None;
        };

        let parent_commit = parent_id.object().unwrap().into_commit();
        let commit_message = self
            .commit
            .message()
            .unwrap()
            .body()
            .map(|body| body.to_string());
        let commit_title = self.commit.message().unwrap().title.to_string();
        let mut commit_statistics = CommitStatistics {
            body: commit_message,
            title: commit_title,
            ..Default::default()
        };

        _ = self
            .commit
            .tree()
            .unwrap()
            .changes()
            .unwrap()
            .track_path()
            .for_each_to_obtain_tree(&parent_commit.tree().unwrap(), |change| {
                let ext = change
                    .location
                    .to_path_lossy()
                    .extension()
                    .map(|ext| ext.to_string_lossy().to_string());

                let location = change.location.to_str_lossy();
                commit_statistics
                    .files_modified
                    .insert(location.to_string());

                match &change.event {
                    // We only want git blobs and nothing else
                    gix::object::tree::diff::change::Event::Addition { entry_mode, id }
                        if matches!(entry_mode, EntryMode::Blob) =>
                    {
                        commit_statistics.file_insertions += 1;
                    }
                    gix::object::tree::diff::change::Event::Deletion { entry_mode, id }
                        if matches!(entry_mode, EntryMode::Blob) =>
                    {
                        commit_statistics.file_deletions += 1;
                    }
                    gix::object::tree::diff::change::Event::Modification {
                        previous_entry_mode,
                        previous_id,
                        entry_mode,
                        id,
                    } if matches!(entry_mode, EntryMode::Blob)
                        && matches!(previous_entry_mode, EntryMode::Blob) => {}
                    gix::object::tree::diff::change::Event::Rewrite {
                        source_location,
                        source_entry_mode,
                        source_id,
                        diff,
                        entry_mode,
                        id,
                        copy,
                    } if matches!(entry_mode, EntryMode::Blob) => {}
                    _ => {}
                }

                Ok::<gix::object::tree::diff::Action, CommitError>(
                    gix::object::tree::diff::Action::Continue,
                )
            })
            .unwrap();

        self.commit = parent_commit;
        self.parent = self.commit.parent_ids().next();
        Some(commit_statistics)
    }
}

fn add_diff(
    location: &str,
    extension: &Option<&str>,
    old: std::borrow::Cow<'_, str>,
    new: std::borrow::Cow<'_, str>,
    commit_statistics: &mut CommitStatistics,
) {
    let input = gix::diff::blob::intern::InternedInput::new(old.as_ref(), new.as_ref());
    commit_statistics.git_diff += &format!(
        r#"diff --git a/{location} b/{location}"
--- a/{location}
--- b/{location}
"#
    );
    let diff = gix::diff::blob::diff(
        gix::diff::blob::Algorithm::Histogram,
        &input,
        Counter::new(UnifiedDiffBuilder::new(&input)),
    );

    if let Some(ext) = extension {
        commit_statistics.line_insertions += &diff.removals;
        commit_statistics.line_deletions += &diff.insertions;
    }

    commit_statistics.git_diff += diff.wrapped.as_str();
    commit_statistics.git_diff += "\n";
}

pub fn get_commit_statistics_for_local_checkout(
    repo_ref: RepoRef,
) -> anyhow::Result<Vec<CommitStatistics>> {
    // This only works for the local path right now, but thats fine
    let repo = gix::open(repo_ref.local_path().expect("local path to be present"))?;
    let head_commit = repo
        .head()
        .context("invalid branch name")?
        .into_fully_peeled_id()
        .context("git errors")?
        .context("git errors")?
        .object()
        .context("git errors")?
        .into_commit();
    let parent = head_commit.parent_ids().next();
    Ok(GitCommitIterator {
        commit: head_commit,
        parent,
    }
    .take(300)
    .collect::<Vec<_>>())
}

#[tokio::main]
async fn main() {
    let reporef = RepoRef::new(Backend::Local, "/Users/skcd/scratch/ide").expect("this to work");
    // Here we will try to run the code and see if we can figure something out
    // start a new tokio task and mark this as blocking because it uses a lot of
    // IO
    let results = {
        let reporef_cloned = reporef.clone();
        tokio::task::spawn_blocking(|| get_commit_statistics_for_local_checkout(reporef))
            .await
            .context("threads error")
    };
    dbg!(results);
}
