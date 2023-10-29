use std::collections::HashSet;

use super::{
    javascript::javascript_language_config, rust::rust_language_config,
    typescript::typescript_language_config,
};

fn naive_chunker(buffer: &str, line_count: usize, overlap: usize) -> Vec<Span> {
    let mut chunks: Vec<Span> = vec![];
    let current_chunk = buffer
        .lines()
        .into_iter()
        .map(|line| line.to_owned())
        .collect::<Vec<_>>();
    let chunk_length = current_chunk.len();
    let mut start = 0;
    while start < chunk_length {
        let end = (start + line_count).min(chunk_length);
        let chunk = current_chunk[start..end].to_owned();
        let span = Span::new(start, end, None, Some(chunk.join("\n")));
        chunks.push(span);
        start += line_count - overlap;
    }
    chunks
}

/// We are going to use tree-sitter to parse the code and get the chunks for the
/// code. we are going to use the algo sweep uses for tree-sitter
///
#[derive(Debug, Clone)]
pub struct TSLanguageConfig {
    /// A list of language names that can be processed by these scope queries
    /// e.g.: ["Typescript", "TSX"], ["Rust"]
    pub language_ids: &'static [&'static str],

    /// Extensions that can help classify the file: rs, js, tx, py, etc
    pub file_extensions: &'static [&'static str],

    /// tree-sitter grammar for this language
    pub grammar: fn() -> tree_sitter::Language,

    /// Namespaces defined by this language,
    /// E.g.: type namespace, variable namespace, function namespace
    pub namespaces: Vec<String>,

    /// The documentation query which will be used by this language
    pub documentation_query: Vec<String>,

    /// The queries to get the function body for the language
    pub function_query: Vec<String>,
}

impl TSLanguageConfig {
    pub fn get_language(&self) -> Option<String> {
        self.language_ids.first().map(|s| s.to_string())
    }
}

#[derive(Clone)]
pub struct TSLanguageParsing {
    configs: Vec<TSLanguageConfig>,
}

impl TSLanguageParsing {
    pub fn init() -> Self {
        Self {
            configs: vec![
                javascript_language_config(),
                typescript_language_config(),
                rust_language_config(),
            ],
        }
    }

    pub fn for_lang(&self, language: &str) -> Option<&TSLanguageConfig> {
        self.configs
            .iter()
            .find(|config| config.language_ids.contains(&language))
    }

    /// We will use this to chunk the file to pieces which can be used for
    /// searching
    pub fn chunk_file(
        &self,
        file_path: &str,
        buffer: &str,
        file_extension: Option<&str>,
    ) -> Vec<Span> {
        if file_extension.is_none() {
            // We use naive chunker here which just splits on the number
            // of lines
            return naive_chunker(buffer, 30, 15);
        }
        // We try to find which language config we should use for this file
        let language_config_maybe = self
            .configs
            .iter()
            .find(|config| config.file_extensions.contains(&file_extension.unwrap()));
        if let Some(language_config) = language_config_maybe {
            // We use tree-sitter to parse the file and get the chunks
            // for the file
            let language = language_config.grammar;
            let mut parser = tree_sitter::Parser::new();
            parser.set_language(language()).unwrap();
            let tree = parser.parse(buffer.as_bytes(), None).unwrap();
            // we allow for 1500 characters and 100 character coalesce
            let chunks = chunk_tree(&tree, language_config, 1500, 100, &buffer);
            chunks
        } else {
            // use naive chunker here which just splits the file into parts
            return naive_chunker(buffer, 30, 15);
        }
    }

    pub fn parse_documentation(&self, code: &str, language: &str) -> Vec<String> {
        let language_config_maybe = self
            .configs
            .iter()
            .find(|config| config.language_ids.contains(&language));
        if let None = language_config_maybe {
            return Default::default();
        }
        let language_config = language_config_maybe.expect("if let None check above to hold");
        let grammar = language_config.grammar;
        let documentation_queries = language_config.documentation_query.to_vec();
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(grammar()).unwrap();
        let parsed_data = parser.parse(code, None).unwrap();
        let node = parsed_data.root_node();
        let mut nodes = vec![];
        documentation_queries
            .into_iter()
            .for_each(|documentation_query| {
                let query = tree_sitter::Query::new(grammar(), &documentation_query)
                    .expect("documentation queries are well formed");
                let mut cursor = tree_sitter::QueryCursor::new();
                cursor
                    .captures(&query, node, code.as_bytes())
                    .into_iter()
                    .for_each(|capture| {
                        capture.0.captures.into_iter().for_each(|capture| {
                            nodes.push(capture.node);
                        })
                    });
            });

        // Now we only want to keep the unique ranges which we have captured
        // from the nodes
        let mut node_ranges: HashSet<tree_sitter::Range> = Default::default();
        let nodes = nodes
            .into_iter()
            .filter(|capture| {
                let range = capture.range();
                if node_ranges.contains(&range) {
                    return false;
                }
                node_ranges.insert(range);
                true
            })
            .collect::<Vec<_>>();

        // Now that we have the nodes, we also want to merge them together,
        // for that we need to first order the nodes
        get_merged_documentation_nodes(nodes, code)
    }
}

fn get_merged_documentation_nodes(matches: Vec<tree_sitter::Node>, source: &str) -> Vec<String> {
    let mut comments = Vec::new();
    let mut current_index = 0;

    while current_index < matches.len() {
        let mut lines = Vec::new();
        lines.push(get_text_from_source(
            source,
            &matches[current_index].range(),
        ));

        while current_index + 1 < matches.len()
            && matches[current_index].range().end_point.row + 1
                == matches[current_index + 1].range().start_point.row
        {
            current_index += 1;
            lines.push(get_text_from_source(
                source,
                &matches[current_index].range(),
            ));
        }

        comments.push(lines.join("\n"));
        current_index += 1;
    }
    comments
}

fn get_text_from_source(source: &str, range: &tree_sitter::Range) -> String {
    source[range.start_byte..range.end_byte].to_owned()
}

#[derive(Clone, Debug, PartialEq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub language: Option<String>,
    pub data: Option<String>,
}

impl Span {
    fn new(start: usize, end: usize, language: Option<String>, data: Option<String>) -> Self {
        Self {
            start,
            end,
            language,
            data,
        }
    }

    fn len(&self) -> usize {
        self.end - self.start
    }
}

fn chunk_node(
    mut node: tree_sitter::Node,
    language: &TSLanguageConfig,
    max_chars: usize,
) -> Vec<Span> {
    let mut chunks: Vec<Span> = vec![];
    let mut current_chunk = Span::new(
        node.start_byte(),
        node.start_byte(),
        language.get_language(),
        None,
    );
    let mut node_walker = node.walk();
    let current_node_children = node.children(&mut node_walker);
    for child in current_node_children {
        if child.end_byte() - child.start_byte() > max_chars {
            chunks.push(current_chunk.clone());
            current_chunk = Span::new(
                child.end_byte(),
                child.end_byte(),
                language.get_language(),
                None,
            );
            chunks.extend(chunk_node(child, language, max_chars));
        } else if child.end_byte() - child.start_byte() + current_chunk.len() > max_chars {
            chunks.push(current_chunk.clone());
            current_chunk = Span::new(
                child.start_byte(),
                child.end_byte(),
                language.get_language(),
                None,
            );
        } else {
            current_chunk.end = child.end_byte();
        }
    }
    chunks.push(current_chunk);
    chunks
}

/// We want to get back the non whitespace length of the string
fn non_whitespace_len(s: &str) -> usize {
    s.chars().filter(|c| !c.is_whitespace()).count()
}

fn get_line_number(byte_position: usize, split_lines: &[&str]) -> usize {
    let mut line_number = 0;
    let mut current_position = 0;
    for line in split_lines {
        if current_position + line.len() > byte_position {
            return line_number;
        }
        current_position += line.len();
        line_number += 1;
    }
    line_number
}

pub fn chunk_tree(
    tree: &tree_sitter::Tree,
    language: &TSLanguageConfig,
    max_characters_per_chunk: usize,
    coalesce: usize,
    buffer_content: &str,
) -> Vec<Span> {
    let mut chunks: Vec<Span> = vec![];
    let root_node = tree.root_node();
    let split_lines = buffer_content.split("\n").collect::<Vec<_>>();
    chunks = chunk_node(root_node, language, max_characters_per_chunk);

    if chunks.len() == 0 {
        return Default::default();
    }
    if chunks.len() < 2 {
        return vec![Span::new(
            0,
            get_line_number(chunks[0].end, split_lines.as_slice()),
            language.get_language(),
            Some(buffer_content.to_owned()),
        )];
    }
    for (prev, curr) in chunks.to_vec().iter_mut().zip(chunks.iter_mut().skip(1)) {
        prev.end = curr.start;
    }

    let mut new_chunks: Vec<Span> = Default::default();
    let mut current_chunk = Span::new(0, 0, language.get_language(), None);
    for chunk in chunks.iter() {
        current_chunk = Span::new(
            current_chunk.start,
            chunk.end,
            language.get_language(),
            None,
        );
        if non_whitespace_len(buffer_content[current_chunk.start..current_chunk.end].trim())
            > coalesce
        {
            new_chunks.push(current_chunk.clone());
            current_chunk = Span::new(chunk.end, chunk.end, language.get_language(), None);
        }
    }

    if current_chunk.len() > 0 {
        new_chunks.push(current_chunk.clone());
    }

    let mut line_chunks = new_chunks
        .iter()
        .map(|chunk| {
            let start_line = get_line_number(chunk.start, split_lines.as_slice());
            let end_line = get_line_number(chunk.end, split_lines.as_slice());
            Span::new(start_line, end_line, language.get_language(), None)
        })
        .filter(|span| span.len() > 0)
        .collect::<Vec<Span>>();

    if line_chunks.len() > 1 && line_chunks.last().unwrap().len() < coalesce {
        let chunks_len = line_chunks.len();
        let last_chunk = line_chunks.last().unwrap().clone();
        let prev_chunk = line_chunks.get_mut(chunks_len - 2).unwrap();
        prev_chunk.end = last_chunk.end;
        line_chunks.pop();
    }

    let split_buffer = buffer_content.split("\n").collect::<Vec<_>>();

    line_chunks
        .into_iter()
        .map(|line_chunk| {
            let data: String = split_buffer[line_chunk.start..line_chunk.end].join("\n");
            Span {
                start: line_chunk.start,
                end: line_chunk.end,
                language: line_chunk.language,
                data: Some(data),
            }
        })
        .collect::<Vec<_>>()
}

#[cfg(test)]
mod tests {

    use std::collections::HashSet;

    use crate::chunking::types::FunctionInformation;
    use crate::chunking::types::FunctionNodeType;

    use super::naive_chunker;
    use super::TSLanguageParsing;

    fn get_naive_chunking_test_string<'a>() -> &'a str {
        r#"
        # @axflow/models/azure-openai/chat

        Interface with [Azure-OpenAI's Chat Completions API](https://learn.microsoft.com/en-us/azure/ai-services/openai/reference) using this module.
        
        Note that this is very close to the vanilla openAI interface, with some subtle minor differences (the return types contain content filter results, see the `AzureOpenAIChatTypes.ContentFilterResults` type ).
        
        In addition, the streaming methods sometimes return objects with empty `choices` arrays. This is automatically handled if you use the `streamTokens()` method.
        
        ```ts
        import { AzureOpenAIChat } from '@axflow/models/azure-openai/chat';
        import type { AzureOpenAIChatTypes } from '@axflow/models/azure-openai/chat';
        ```
        
        ```ts
        declare class AzureOpenAIChat {
          static run: typeof run;
          static stream: typeof stream;
          static streamBytes: typeof streamBytes;
          static streamTokens: typeof streamTokens;
        }
        ```
        
        ## `run`
        
        ```ts
        /**
         * Run a chat completion against the Azure-openAI API.
         *
         * @see https://learn.microsoft.com/en-us/azure/ai-services/openai/reference#chat-completions
         *
         * @param request The request body sent to Azure. See Azure's documentation for all available parameters.
         * @param options
         * @param options.apiKey Azure API key.
         * @param options.resourceName Azure resource name.
         * @param options.deploymentId Azure deployment id.
         * @param options.apiUrl The url of the OpenAI (or compatible) API. If this is passed, resourceName and deploymentId are ignored.
         * @param options.fetch A custom implementation of fetch. Defaults to globalThis.fetch.
         * @param options.headers Optionally add additional HTTP headers to the request.
         * @param options.signal An AbortSignal that can be used to abort the fetch request.
         *
         * @returns an Azure OpenAI chat completion. See Azure's documentation for /chat/completions
         */
        declare function run(
          request: AzureOpenAIChatTypes.Request,
          options: AzureOpenAIChatTypes.RequestOptions
        ): Promise<AzureOpenAIChatTypes.Response>;
        ```
        
        ## `streamBytes`
        
        ```ts
        /**
         * Run a streaming chat completion against the Azure-openAI API. The resulting stream is the raw unmodified bytes from the API.
         *
         * @see https://learn.microsoft.com/en-us/azure/ai-services/openai/reference#chat-completions
         *
         * @param request The request body sent to Azure. See Azure's documentation for all available parameters.
         * @param options
         * @param options.apiKey Azure API key.
         * @param options.resourceName Azure resource name.
         * @param options.deploymentId Azure deployment id.
         * @param options.apiUrl The url of the OpenAI (or compatible) API. If this is passed, resourceName and deploymentId are ignored.
         * @param options.fetch A custom implementation of fetch. Defaults to globalThis.fetch.
         * @param options.headers Optionally add additional HTTP headers to the request.
         * @param options.signal An AbortSignal that can be used to abort the fetch request.
         *
         * @returns A stream of bytes directly from the API.
         */
        declare function streamBytes(
          request: AzureOpenAIChatTypes.Request,
          options: AzureOpenAIChatTypes.RequestOptions
        ): Promise<ReadableStream<Uint8Array>>;
        ```
        
        ## `stream`
        
        ```ts
        /**
         * Run a streaming chat completion against the Azure-openAI API. The resulting stream is the parsed stream data as JavaScript objects.
         *
         * @see https://learn.microsoft.com/en-us/azure/ai-services/openai/reference#chat-completions
         *
         * Example object:
         * {"id":"chatcmpl-864d71dHehdlb2Vjq7WP5nHz10LRO","object":"chat.completion.chunk","created":1696458457,"model":"gpt-4","choices":[{"index":0,"finish_reason":null,"delta":{"content":" me"}}],"usage":null}
         *
         * @param request The request body sent to Azure. See Azure's documentation for all available parameters.
         * @param options
         * @param options.apiKey Azure API key.
         * @param options.resourceName Azure resource name.
         * @param options.deploymentId Azure deployment id.
         * @param options.apiUrl The url of the OpenAI (or compatible) API. If this is passed, resourceName and deploymentId are ignored.
         * @param options.fetch A custom implementation of fetch. Defaults to globalThis.fetch.
         * @param options.headers Optionally add additional HTTP headers to the request.
         * @param options.signal An AbortSignal that can be used to abort the fetch request.
         *
         * @returns A stream of objects representing each chunk from the API.
         */
        declare function stream(
          request: AzureOpenAIChatTypes.Request,
          options: AzureOpenAIChatTypes.RequestOptions
        ): Promise<ReadableStream<AzureOpenAIChatTypes.Chunk>>;
        ```
        
        ## `streamTokens`
        
        ```ts
        /**
         * Run a streaming chat completion against the Azure-openAI API. The resulting stream emits only the string tokens.
         *
         * @see https://learn.microsoft.com/en-us/azure/ai-services/openai/reference#chat-completions
         *
         * @param request The request body sent to Azure. See Azure's documentation for all available parameters.
         * @param options
         * @param options.apiKey Azure API key.
         * @param options.resourceName Azure resource name.
         * @param options.deploymentId Azure deployment id.
         * @param options.apiUrl The url of the OpenAI (or compatible) API. If this is passed, resourceName and deploymentId are ignored.
         * @param options.fetch A custom implementation of fetch. Defaults to globalThis.fetch.
         * @param options.headers Optionally add additional HTTP headers to the request.
         * @param options.signal An AbortSignal that can be used to abort the fetch request.
         *
         * @returns A stream of tokens from the API.
         */
        declare function streamTokens(
          request: AzureOpenAIChatTypes.Request,
          options: AzureOpenAIChatTypes.RequestOptions
        ): Promise<ReadableStream<string>>;
        ```        
        "#
    }

    #[test]
    fn test_naive_chunker() {
        // The test buffer has a total length of 128, with a chunk of size 30
        // and overlap of 15 we get 9 chunks, its easy maths. ceil(128/15) == 9
        let chunks = naive_chunker(get_naive_chunking_test_string(), 30, 15);
        assert_eq!(chunks.len(), 9);
    }

    #[test]
    fn test_documentation_parsing_rust() {
        let source_code = r#"
/// Some comment
/// Some other comment
fn blah_blah() {

}

/// something else
struct A {
    /// something over here
    pub a: string,
}
        "#;
        let tree_sitter_parsing = TSLanguageParsing::init();
        let documentation = tree_sitter_parsing.parse_documentation(source_code, "rust");
        assert_eq!(
            documentation,
            vec![
                "/// Some comment\n/// Some other comment",
                "/// something else",
                "/// something over here",
            ]
        );
    }

    #[test]
    fn test_documentation_parsing_rust_another() {
        let source_code = "/// Returns the default user ID as a `String`.\n///\n/// The default user ID is set to \"codestory\".\nfn default_user_id() -> String {\n    \"codestory\".to_owned()\n}";
        let tree_sitter_parsing = TSLanguageParsing::init();
        let documentation = tree_sitter_parsing.parse_documentation(source_code, "rust");
        assert_eq!(
            documentation,
            vec![
                "/// Returns the default user ID as a `String`.\n///\n/// The default user ID is set to \"codestory\".",
            ],
        );
    }

    #[test]
    fn test_documentation_parsing_typescript() {
        let source_code = r#"
        /**
         * Run a streaming chat completion against the Azure-openAI API. The resulting stream emits only the string tokens.
         *
         * @see https://learn.microsoft.com/en-us/azure/ai-services/openai/reference#chat-completions
         *
         * @param request The request body sent to Azure. See Azure's documentation for all available parameters.
         * @param options
         * @param options.apiKey Azure API key.
         * @param options.resourceName Azure resource name.
         * @param options.deploymentId Azure deployment id.
         * @param options.apiUrl The url of the OpenAI (or compatible) API. If this is passed, resourceName and deploymentId are ignored.
         * @param options.fetch A custom implementation of fetch. Defaults to globalThis.fetch.
         * @param options.headers Optionally add additional HTTP headers to the request.
         * @param options.signal An AbortSignal that can be used to abort the fetch request.
         *
         * @returns A stream of tokens from the API.
         */
        declare function streamTokens(
          request: AzureOpenAIChatTypes.Request,
          options: AzureOpenAIChatTypes.RequestOptions
        ): Promise<ReadableStream<string>>;
        "#;

        let tree_sitter_parsing = TSLanguageParsing::init();
        let documentation = tree_sitter_parsing.parse_documentation(source_code, "typescript");
        assert_eq!(
            documentation,
            vec![
    "/**\n         * Run a streaming chat completion against the Azure-openAI API. The resulting stream emits only the string tokens.\n         *\n         * @see https://learn.microsoft.com/en-us/azure/ai-services/openai/reference#chat-completions\n         *\n         * @param request The request body sent to Azure. See Azure's documentation for all available parameters.\n         * @param options\n         * @param options.apiKey Azure API key.\n         * @param options.resourceName Azure resource name.\n         * @param options.deploymentId Azure deployment id.\n         * @param options.apiUrl The url of the OpenAI (or compatible) API. If this is passed, resourceName and deploymentId are ignored.\n         * @param options.fetch A custom implementation of fetch. Defaults to globalThis.fetch.\n         * @param options.headers Optionally add additional HTTP headers to the request.\n         * @param options.signal An AbortSignal that can be used to abort the fetch request.\n         *\n         * @returns A stream of tokens from the API.\n         */",
            ],
        );
    }

    #[test]
    fn test_function_body_parsing_rust() {
        let source_code = r#"
/// Some comment
/// Some other comment
fn blah_blah() {

}

/// something else
struct A {
    /// something over here
    pub a: string,
}

impl A {
    fn something_else() -> Option<String> {
        None
    }
}
        "#;

        let tree_sitter_parsing = TSLanguageParsing::init();
        let language_config = tree_sitter_parsing.for_lang("rust").unwrap();
        let function_queries = language_config.function_query.to_vec();

        // Now we need to run the tree sitter query on this and get back the
        // answer
        let grammar = language_config.grammar;
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(grammar()).unwrap();
        let parsed_data = parser.parse(source_code.as_bytes(), None).unwrap();
        let node = parsed_data.root_node();
        let mut function_nodes = vec![];
        let mut unique_ranges: HashSet<tree_sitter::Range> = Default::default();
        function_queries.into_iter().for_each(|function_query| {
            let query = tree_sitter::Query::new(grammar(), &function_query)
                .expect("function queries are well formed");
            let mut cursor = tree_sitter::QueryCursor::new();
            cursor
                .captures(&query, node, source_code.as_bytes())
                .into_iter()
                .for_each(|capture| {
                    capture.0.captures.into_iter().for_each(|capture| {
                        let capture_name = query
                            .capture_names()
                            .to_vec()
                            .remove(capture.index.try_into().unwrap());
                        let capture_type = FunctionNodeType::from_str(&capture_name);
                        if let Some(capture_type) = capture_type {
                            function_nodes
                                .push(FunctionInformation::new(capture.node, capture_type));
                        }
                    })
                });
        });
        function_nodes = function_nodes
            .into_iter()
            .filter_map(|function_node| {
                let range = function_node.node().range();
                if unique_ranges.contains(&range) {
                    return None;
                }
                unique_ranges.insert(range);
                dbg!(function_node.r#type());
                dbg!(function_node.node().range());
                dbg!(function_node.node().kind());
                dbg!(source_code
                    [function_node.node().start_byte()..function_node.node().end_byte()]
                    .to_owned());
                Some(function_node)
            })
            .collect();
        // we should get back 2 function nodes here and since we capture 3 pieces
        // of information for each function block, in total that is 6
        assert_eq!(function_nodes.len(), 6);
    }
}
