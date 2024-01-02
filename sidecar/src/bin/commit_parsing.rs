//! We are going to try and parse the commits, we want to understand what sections
//! of the code has changed in a maaningful way and if we are able to do this
//! properly we will be able to create a code graph of the changes on the code
//! symbols and how they relate to each other

use std::sync::Arc;

use sidecar::chunking::{
    languages::TSLanguageParsing,
    scope_graph::ScopeGraph,
    text_document::{Position, Range},
    tree_sitter_file::TreeSitterFile,
};
use tokio::process::Command;

#[tokio::main]
async fn main() {
    // Today instead of the base and the next commit we pick up the files which have
    // changed here and use that to get the commit delta and see how things are working
    let head_path_str = "/Users/skcd/scratch/sidecar/sidecar/src/bin/test_files/bloop_answer_b.rs";
    let parent_path_str =
        "/Users/skcd/scratch/sidecar/sidecar/src/bin/test_files/bloop_answer_a.rs";
    let head_contents =
        std::fs::read("/Users/skcd/scratch/sidecar/sidecar/src/bin/test_files/bloop_answer_b.rs")
            .expect("to not fail");
    let parent_contents =
        std::fs::read("/Users/skcd/scratch/sidecar/sidecar/src/bin/test_files/bloop_answer_a.rs")
            .expect("to not fail");

    // Now we can run git diff on this and see how things are working
    let output = git_diff_command(parent_path_str, head_path_str).await;
    let (deleted_lines, added_lines) = parse_git_diff_output(&output.0);

    // Now we want to see what function or class or namespace it belongs to and
    // how are we going to get that out
    let language_parsing = Arc::new(TSLanguageParsing::init());
    let scope_graph_parent = get_scope_graph(
        &String::from_utf8(parent_contents.to_vec()).expect("to not fail"),
        "rust",
        language_parsing,
    )
    .await;

    // We first try to get position for the parent contents
    // and then we try to get the position for the head contents
    deleted_lines.into_iter().for_each(|deleted_line| {
        let position = get_range_for_line(
            String::from_utf8(parent_contents.to_vec())
                .expect("to not fail")
                .lines()
                .into_iter()
                .collect::<Vec<_>>(),
            deleted_line,
        );
        let position = position.expect("position to be present");
        dbg!(&position);
        let node_index =
            scope_graph_parent.tightest_node_for_range(position.start_byte(), position.end_byte());
        // Now we are going to get the hoverable ranges from this file
        let hoverable_ranges = hoverable_ranges(
            "rust",
            Arc::new(TSLanguageParsing::init()),
            parent_contents.to_vec(),
        )
        .unwrap_or_default()
        .into_iter()
        .filter(|range| {
            // we only want to get the hoverable ranges which are contained
            // within this position of change
            position.contains(&range)
        })
        .map(|range| {
            dbg!("checking range", &range);
            let start_byte = range.start_byte();
            let end_byte = range.end_byte();
            let node = scope_graph_parent
                .node_by_range(start_byte, end_byte)
                .map(|idx| &scope_graph_parent.graph[idx]);
            if let Some(node) = node {
                dbg!("found node information", &node);
            }
        })
        .collect::<Vec<_>>();
        dbg!("hoverable ranges", hoverable_ranges);
        dbg!("left_side", &position, deleted_line);
    });

    // Now we want to get position for the delta line and see what we are getting
    // here
    // added_lines.into_iter().for_each(|added_line| {
    //     let position = get_range_for_line(
    //         String::from_utf8(head_contents.to_vec())
    //             .expect("to not fail")
    //             .lines()
    //             .into_iter()
    //             .collect::<Vec<_>>(),
    //         added_line,
    //     );
    //     dbg!("right_side", &position, added_line);
    // });

    // Try to experiment with the scope graph here and figure out if we can
    // get something useful over here
    // bytes here are tricky to get because we can have functions which start
    // at the middle of the line or in between and not at the start of the line
    // so we will have to be careful about how we are getting the start_byte
    // and the end_byte here
    let start_byte = 0;
    let end_byte = 0;
    scope_graph_parent.node_by_range(start_byte, end_byte);
}

async fn git_diff_command(parent_file_path: &str, head_file_path: &str) -> (String, String) {
    let mut command_parent = Command::new("git");
    let command = command_parent.args(vec![
        "diff",
        "--unified=1000",
        "--no-index",
        parent_file_path,
        head_file_path,
    ]);
    let output = command.output().await.expect("git diff to not fail");
    (
        String::from_utf8(output.stdout.to_owned()).expect("to work"),
        String::from_utf8(output.stderr.to_owned()).expect("to work"),
    )
}

fn parse_git_diff_output(output: &str) -> (Vec<usize>, Vec<usize>) {
    let lines = output.lines().into_iter().collect::<Vec<_>>();
    let mut added_lines = vec![];
    let mut deleted_lines = vec![];
    let mut start_index = None;
    let mut l_index = 0;
    let mut r_index = 0;
    lines.into_iter().enumerate().for_each(|(idx, line)| {
        if line.starts_with("@@") {
            start_index = Some(idx);
            return;
        }
        if let None = start_index {
            return;
        }

        if !line.starts_with("-") && !line.starts_with("+") {
            l_index = l_index + 1;
            r_index = r_index + 1;
        }
        if line.starts_with("-") {
            l_index = l_index + 1;
            deleted_lines.push(l_index);
        }
        if line.starts_with("+") {
            r_index = r_index + 1;
            added_lines.push(r_index);
        }
    });
    let move_by_one = |lines: Vec<usize>| -> Vec<usize> {
        lines
            .into_iter()
            .map(|line| line - 1)
            .collect::<Vec<usize>>()
    };
    (move_by_one(deleted_lines), move_by_one(added_lines))
}

async fn get_scope_graph(
    buffer: &str,
    language: &str,
    language_parsing: Arc<TSLanguageParsing>,
) -> ScopeGraph {
    let scope_graph =
        TreeSitterFile::try_build(buffer.as_bytes(), &language, language_parsing.clone())
            .and_then(TreeSitterFile::scope_graph);
    scope_graph.expect("no errors while testing")
}

/// We are going to ask for the range which starts from the line start and end
/// in this range, this will be useful to check which nodes we want to pick up
/// here and use go-to-reference to see where the changed symbols are defined
fn get_range_for_line(lines: Vec<&str>, line_index: usize) -> Option<Range> {
    let mut byte_counts = 0;
    let mut start_position = None;
    let mut end_position = None;
    for (index, line) in lines.iter().enumerate() {
        if index == line_index {
            start_position = Some(Position::new(index, 0, byte_counts));
            end_position = Some(Position::new(
                index,
                line.len(),
                byte_counts + line.as_bytes().len(),
            ));
            break;
        }
        // adding a 1 here because thats the \n character
        byte_counts = byte_counts + line.as_bytes().len() + '\n'.to_string().as_bytes().len();
    }

    match (start_position, end_position) {
        (Some(start_position), Some(end_position)) => {
            Some(Range::new(start_position, end_position))
        }
        _ => None,
    }
}

fn hoverable_ranges(
    language: &str,
    language_parsing: Arc<TSLanguageParsing>,
    content: Vec<u8>,
) -> Option<Vec<Range>> {
    TreeSitterFile::try_build(content.as_slice(), language, language_parsing)
        .and_then(TreeSitterFile::hoverable_ranges)
        .ok()
}
