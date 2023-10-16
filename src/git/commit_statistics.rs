use std::collections::HashSet;

use crate::repo::types::{Backend, RepoRef};
use anyhow::Context;
use gix::{
    bstr::ByteSlice,
    diff::blob::{sink::Counter, UnifiedDiffBuilder},
    object::blob::diff::Platform,
    objs::tree::EntryMode,
    Commit, Id,
};

/// getting statistics out of git commits

#[derive(Debug, Default)]
pub struct CommitStatistics {
    author: Option<String>,
    file_insertions: usize,
    file_deletions: usize,
    title: String,
    body: Option<String>,
    git_diff: String,
    files_modified: HashSet<String>,
    line_insertions: u32,
    line_deletions: u32,
    commit_timestamp: i64,
    commit_hash: String,
    // This is the repo-reference which we will use to tag the repository
    // as unique
    repo_ref: String,
}

struct GitCommitIterator<'a> {
    commit: Commit<'a>,
    parent: Option<Id<'a>>,
    repo_ref: &'a RepoRef,
}

#[derive(Debug)]
struct CommitError;

impl std::fmt::Display for CommitError {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
        commit_statistics.author = self
            .commit
            .author()
            .map(|author| author.name.to_string())
            .ok();
        // This is the commit timestamp from the unix epoch
        commit_statistics.commit_timestamp = self.commit.time().unwrap().seconds;
        commit_statistics.commit_hash = self.commit.id().to_string();
        commit_statistics.repo_ref = self.repo_ref.to_string();

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
                        add_diff(
                            &location,
                            &ext.as_deref(),
                            "".into(),
                            id.object().unwrap().data.as_bstr().to_str_lossy(),
                            &mut commit_statistics,
                        );
                    }
                    gix::object::tree::diff::change::Event::Deletion { entry_mode, id }
                        if matches!(entry_mode, EntryMode::Blob) =>
                    {
                        commit_statistics.file_deletions += 1;
                        add_diff(
                            &location,
                            &ext.as_deref(),
                            id.object().unwrap().data.as_bstr().to_str_lossy(),
                            "".into(),
                            &mut commit_statistics,
                        );
                    }
                    gix::object::tree::diff::change::Event::Modification {
                        previous_entry_mode,
                        previous_id,
                        entry_mode,
                        id,
                    } if matches!(entry_mode, EntryMode::Blob)
                        && matches!(previous_entry_mode, EntryMode::Blob) =>
                    {
                        let platform = Platform::from_ids(previous_id, id).unwrap();
                        let old = platform.old.data.as_bstr().to_str_lossy();
                        let new = platform.new.data.as_bstr().to_str_lossy();
                        add_diff(&location, &ext.as_deref(), old, new, &mut commit_statistics);
                    }
                    gix::object::tree::diff::change::Event::Rewrite {
                        source_id,
                        entry_mode,
                        id,
                        ..
                    } if matches!(entry_mode, EntryMode::Blob) => {
                        let platform = Platform::from_ids(source_id, id).unwrap();
                        let old = platform.old.data.as_bstr().to_str_lossy();
                        let new = platform.new.data.as_bstr().to_str_lossy();
                        add_diff(&location, &ext.as_deref(), old, new, &mut commit_statistics);
                    }
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

    if let Some(_) = extension {
        // Here we have to guard against the extensions which we know we don't
        // care about
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
        repo_ref: &repo_ref,
    }
    .take(300)
    .collect::<Vec<_>>())
}
