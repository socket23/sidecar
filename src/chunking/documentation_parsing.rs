/// We want to parse the documentation here for a given code block
use std::collections::{HashMap, HashSet};

async fn documentation_queries() -> HashMap<String, String> {
    vec![(
        "javascript".to_owned(),
        "((comment) @comment
        (#match? @comment \"^\\\\/\\\\*\\\\*\")) @docComment"
            .to_owned(),
    )]
    .into_iter()
    .collect()
}

pub fn parse_documentation_for_typescript_code(code: &str) -> Vec<String> {
    let query = tree_sitter::Query::new(
        tree_sitter_typescript::language_tsx(),
        "((comment) @comment
    (#match? @comment \"^\\\\/\\\\*\\\\*\")) @docComment",
    )
    .expect("this to work");
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(tree_sitter_typescript::language_tsx())
        .unwrap();
    let parsed_data = parser.parse(code, None).unwrap();
    let node = parsed_data.root_node();
    let mut cursor = tree_sitter::QueryCursor::new();
    let nodes = cursor
        .matches(&query, node, code.as_bytes())
        .flat_map(|m| m.captures)
        .collect::<Vec<_>>();
    // we want to keep the unique nodes here for the documentation which has
    // been generated
    let mut node_ranges: HashSet<tree_sitter::Range> = Default::default();
    let nodes = nodes
        .into_iter()
        .filter(|capture| {
            let range = capture.node.range();
            if node_ranges.contains(&range) {
                return false;
            }
            node_ranges.insert(range);
            true
        })
        .collect::<Vec<_>>();
    get_merged_comments(nodes, code)
}

fn get_text_from_source(source: &str, range: &tree_sitter::Range) -> String {
    source[range.start_byte..range.end_byte].to_owned()
}

fn get_merged_comments(matches: Vec<&tree_sitter::QueryCapture>, source: &str) -> Vec<String> {
    let mut comments = Vec::new();
    let mut currentIndex = 0;

    while currentIndex < matches.len() {
        let mut lines = Vec::new();
        lines.push(get_text_from_source(
            source,
            &matches[currentIndex].node.range(),
        ));

        while currentIndex + 1 < matches.len()
            && matches[currentIndex].node.range().end_point.row + 1
                == matches[currentIndex + 1].node.range().start_point.row
        {
            currentIndex += 1;
            lines.push(get_text_from_source(
                source,
                &matches[currentIndex].node.range(),
            ));
        }

        comments.push(lines.join("\n"));
        currentIndex += 1;
    }

    comments
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_typescript_parsing() {
        let query = tree_sitter::Query::new(
            tree_sitter_typescript::language_tsx(),
            "((comment) @comment
        (#match? @comment \"^\\\\/\\\\*\\\\*\")) @docComment",
        )
        .expect("this to work");
        let source_code = r#"
        /**
         * Represents the response object returned by the Open AI chat completion API.
         */
        interface Response {
          id: string;
          created: number;
          model: string;
          object: string;
          choices: Array<{
            finish_reason: 'stop' | 'length' | 'function_call';
            index: number;
            message: MessageOutput;
          }>;
          usage?: {
            completion_tokens: number;
            prompt_tokens: number;
            total_tokens: number;
          };
        }

        /**
         * Something over here
        "#;
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(tree_sitter_typescript::language_tsx())
            .unwrap();
        let parsed_data = parser.parse(source_code, None).unwrap();
        let node = parsed_data.root_node();
        let mut cursor = tree_sitter::QueryCursor::new();
        let data = cursor
            .matches(&query, node, source_code.as_bytes())
            .flat_map(|m| m.captures)
            .for_each(|capture| {
                dbg!(capture.node.kind());
                dbg!(capture.node.range());
                dbg!(capture);
            });
        assert!(false);
    }
}
