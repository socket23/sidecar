//! Identifier here represents how the code will look like if we have metadata and the
//! location for it
//! We can also use the tools along with this symbol to traverse the code graph

use std::collections::HashSet;

use derivative::Derivative;
use llm_client::{
    clients::types::LLMType,
    provider::{LLMProvider, LLMProviderAPIKeys},
};

use crate::chunking::{
    text_document::Range,
    types::{OutlineNode, OutlineNodeContent, OutlineNodeType},
};

#[derive(Debug, Clone)]
pub struct LLMProperties {
    llm: LLMType,
    provider: LLMProvider,
    api_key: LLMProviderAPIKeys,
}

impl LLMProperties {
    pub fn new(llm: LLMType, provider: LLMProvider, api_keys: LLMProviderAPIKeys) -> Self {
        Self {
            llm,
            provider,
            api_key: api_keys,
        }
    }

    pub fn llm(&self) -> &LLMType {
        &self.llm
    }

    pub fn provider(&self) -> &LLMProvider {
        &self.provider
    }

    pub fn api_key(&self) -> &LLMProviderAPIKeys {
        &self.api_key
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Snippet {
    range: Range,
    symbol_name: String,
    fs_file_path: String,
    content: String,
    language: Option<String>,
    // this represents completely a snippet of code which is a logical symbol
    outline_node_content: OutlineNodeContent,
}

impl Snippet {
    pub fn new(
        symbol_name: String,
        range: Range,
        fs_file_path: String,
        content: String,
        outline_node_content: OutlineNodeContent,
    ) -> Self {
        Self {
            symbol_name,
            range,
            fs_file_path,
            content,
            language: None,
            outline_node_content,
        }
    }

    // TODO(skcd): Fix the language over here and make it not None
    pub fn language(&self) -> String {
        self.language.clone().unwrap_or("".to_owned()).to_owned()
    }

    pub fn file_path(&self) -> &str {
        &self.fs_file_path
    }

    pub fn range(&self) -> &Range {
        &self.range
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    // we try to get the non overlapping lines from our content
    pub fn get_non_overlapping_content(&self, range: &[&Range]) -> Option<String> {
        let lines = self
            .content
            .lines()
            .into_iter()
            .enumerate()
            .map(|(idx, line)| (idx + self.range().start_line(), line.to_owned()))
            .filter(|(idx, _)| !range.into_iter().any(|range| range.contains_line(*idx)))
            .filter(|(_, line)| {
                // we want to filter out the lines which are not part of
                if line == "}" || line.is_empty() {
                    false
                } else {
                    true
                }
            })
            .map(|(_, line)| line)
            .collect::<Vec<String>>();
        if lines.is_empty() {
            None
        } else {
            Some(lines.join("\n"))
        }
    }

    pub fn to_xml(&self) -> String {
        let symbol_content = &self.outline_node_content;
        let name = &self.symbol_name;
        let file_path = self.file_path();
        let start_line = self.range().start_line();
        let end_line = self.range().end_line();
        let content = self.content();
        let language = self.language();
        format!(
            r#"<name>
{name}
</name>
<file_path>
{file_path}:{start_line}-{end_line}
</file_path>
<content>
```{language}
{content}
```
</content>"#
        )
        .to_owned()
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct SymbolIdentifier {
    symbol_name: String,
    fs_file_path: Option<String>,
}

impl SymbolIdentifier {
    pub fn new_symbol(symbol_name: &str) -> Self {
        Self {
            symbol_name: symbol_name.to_owned(),
            fs_file_path: None,
        }
    }

    pub fn symbol_name(&self) -> &str {
        &self.symbol_name
    }

    pub fn with_file_path(symbol_name: &str, fs_file_path: &str) -> Self {
        Self {
            symbol_name: symbol_name.to_owned(),
            fs_file_path: Some(fs_file_path.to_owned()),
        }
    }
}

#[derive(Debug)]
pub struct SnippetReRankInformation {
    idx: usize,
    range: Range,
    fs_file_path: String,
    is_outline: bool,
}

impl SnippetReRankInformation {
    pub fn new(idx: usize, range: Range, fs_file_path: String) -> Self {
        Self {
            idx,
            range,
            fs_file_path,
            is_outline: false,
        }
    }

    pub fn set_is_outline(mut self) -> Self {
        self.is_outline = true;
        self
    }
}

#[derive(Debug)]
pub struct MechaCodeSymbolThinking {
    symbol_name: String,
    steps: Vec<String>,
    is_new: bool,
    file_path: String,
    snippet: Option<Snippet>,
    implementations: Vec<Snippet>,
}

impl MechaCodeSymbolThinking {
    pub fn new(
        symbol_name: String,
        steps: Vec<String>,
        is_new: bool,
        file_path: String,
        snippet: Option<Snippet>,
        implementations: Vec<Snippet>,
    ) -> Self {
        Self {
            symbol_name,
            steps,
            is_new,
            file_path,
            snippet,
            implementations,
        }
    }

    pub fn steps(&self) -> &[String] {
        self.steps.as_slice()
    }

    pub fn is_new(&self) -> bool {
        self.is_new
    }

    pub fn symbol_name(&self) -> &str {
        &self.symbol_name
    }

    pub fn to_symbol_identifier(&self) -> SymbolIdentifier {
        if self.is_new {
            SymbolIdentifier::new_symbol(&self.symbol_name)
        } else {
            SymbolIdentifier::with_file_path(&self.symbol_name, &self.file_path)
        }
    }

    pub fn set_snippet(&mut self, snippet: Snippet) {
        self.snippet = Some(snippet);
    }

    pub fn get_snippet(&self) -> Option<&Snippet> {
        self.snippet.as_ref()
    }

    pub fn add_step(&mut self, step: &str) {
        self.steps.push(step.to_owned());
    }

    pub fn fs_file_path(&self) -> &str {
        &self.file_path
    }

    pub fn add_implementation(&mut self, implementation: Snippet) {
        self.implementations.push(implementation);
    }

    pub fn get_implementations(&self) -> &[Snippet] {
        self.implementations.as_slice()
    }

    pub fn set_implementations(&mut self, snippets: Vec<Snippet>) {
        self.implementations = snippets;
    }

    // To xml is a common way to say that the data object implements a way to be
    // written in a xml which is a standard way to represent it for a LLM
    // TODO(skcd): How do we get the symbols which need to be edited here
    // properly, can we ask the llm to put it out properly or we ask it for the section
    // index
    // in which case that might work with the caveat being that if the LLM gets confused
    // we will get a big threshold, another way would be that we ask the llm to also
    // reply in symbols and the indexes as well
    // we have to keep a mapping between the snippets and the indexes we are using
    // that's the hard part
    // we can reconstruct if nothing changes in between which is the initial case
    // anyways but might not be the case always
    // combining both would be better
    // we also need a mapping back here which will help us understand which snippet
    // to look at, the structure I can come up with is something like this:
    // idx -> (Range + FS_FILE_PATH + is_outline)
    // fin
    pub fn to_llm_request(&self) -> Option<(String, Vec<SnippetReRankInformation>)> {
        if let Some(snippet) = &self.snippet {
            let is_function = snippet
                .outline_node_content
                .outline_node_type()
                .is_function();
            if is_function {
                let function_body = snippet.to_xml();
                Some((
                    format!(
                        r#"<rerank_entry>
<id>
0
</id>
{function_body}
</rerank_entry>"#
                    ),
                    vec![SnippetReRankInformation::new(
                        0,
                        snippet.range.clone(),
                        snippet.fs_file_path.to_owned(),
                    )],
                ))
            } else {
                // and now we have the other symbols which might be a mix of the following
                // functions
                // class implementations
                // one of the problems we hvae have here is that we have to show
                // the llm all these sections and then show the llm on how to edit them
                // this is the ost interesting part since we do know what the implementation
                // block looks like with the functions removed, we can use huristics
                // to fix it or expose it as part of the outline nodes
                let class_implementations = self
                    .implementations
                    .iter()
                    .filter(|implementation| implementation.outline_node_content.is_class_type())
                    .collect::<Vec<_>>();
                let functions = self
                    .implementations
                    .iter()
                    .filter(|implemenation| implemenation.outline_node_content.is_function_type())
                    .collect::<Vec<_>>();
                let mut covered_function_idx: HashSet<usize> = Default::default();
                let class_covering_functions = class_implementations
                    .into_iter()
                    .map(|class_implementation| {
                        let class_range = class_implementation.range();
                        let filtered_functions = functions
                            .iter()
                            .enumerate()
                            .filter_map(|(idx, function)| {
                                if class_range.contains(function.range()) {
                                    covered_function_idx.insert(idx);
                                    Some(function)
                                } else {
                                    None
                                }
                            })
                            .collect::<Vec<_>>();
                        let class_non_overlap = class_implementation.get_non_overlapping_content(
                            filtered_functions
                                .iter()
                                .map(|filtered_function| filtered_function.range())
                                .collect::<Vec<_>>()
                                .as_slice(),
                        );
                        (class_implementation, filtered_functions, class_non_overlap)
                    })
                    .collect::<Vec<(&Snippet, Vec<&&Snippet>, Option<String>)>>();

                // now we will generate the code snippets over here
                // and give them a list
                // this list is a bit special cause it also has prefix in between
                // for some symbols
                let mut symbol_index = 0;
                // we are hedging on the fact that the agent will be able to pick
                // up the snippets properly, instead of going for the inner symbols
                // (kind of orthodox I know, the reason is that the starting part of
                // the symbol is also important and editable, so this approach should
                // in theory work)
                // ideally we will move it back to a range based edit later on
                let mut symbol_rerank_information = vec![];
                let symbol_list = class_covering_functions
                    .into_iter()
                    .map(|(class_snippet, functions, non_overlap_prefix)| {
                        let formatted_snippet = class_snippet.to_xml();
                        if class_snippet.outline_node_content.is_class_definition() {
                            let definition = format!(
                                r#"<rerank_entry>
<id>
{symbol_index}
</id>
{formatted_snippet}
</rerank_entry>"#
                            );
                            symbol_rerank_information.push(SnippetReRankInformation::new(
                                symbol_index,
                                class_snippet.range.clone(),
                                class_snippet.fs_file_path.to_owned(),
                            ));
                            symbol_index = symbol_index + 1;
                            definition
                        } else {
                            let overlap = if let Some(non_overlap_prefix) = non_overlap_prefix {
                                let file_path = class_snippet.file_path();
                                let overlapp_snippet = format!(
                                    r#"<rerank_entry>
<id>
{symbol_index}
</id>
<file_path>
{file_path}
</file_path>
<content>
{non_overlap_prefix}
</content>
</rerank_entry>"#
                                )
                                .to_owned();
                                symbol_rerank_information.push(
                                    SnippetReRankInformation::new(
                                        symbol_index,
                                        class_snippet.range.clone(),
                                        class_snippet.fs_file_path.to_owned(),
                                    )
                                    .set_is_outline(),
                                );
                                symbol_index = symbol_index + 1;
                                Some(overlapp_snippet)
                            } else {
                                None
                            };
                            let function_snippets = functions
                                .into_iter()
                                .map(|function| {
                                    let function_body = function.to_xml();
                                    let function_code_snippet = format!(
                                        r#"<rerank_entry>
<id>
{symbol_index}
</id>
{function_body}
</rerank_entry>"#
                                    );
                                    symbol_rerank_information.push(SnippetReRankInformation::new(
                                        symbol_index,
                                        function.range.clone(),
                                        function.fs_file_path.to_owned(),
                                    ));
                                    symbol_index = symbol_index + 1;
                                    function_code_snippet
                                })
                                .collect::<Vec<_>>()
                                .join("\n");

                            // now that we have the overlap we have to join it together
                            // with the functions
                            if let Some(overlap) = overlap {
                                format!(
                                    r#"{overlap}
{function_snippets}"#
                                )
                            } else {
                                function_snippets
                            }
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                // for functions which are inside trait boundaries we will do the following:
                // try to get the lines which are not covered by the functions from the outline
                // remove the } from the end of the string (always try and do class.end_line() - max(function.end_line()))
                // and then we put the functions, that way things turn out structured as we want
                // TODO(skcd): This will break in the future since we want to identify the property
                // identifiers, but for now this is completely fine
                // now for the functions which are not covered we will create separate prompts for them
                // cause those are not covered by any class implementation (which is suss...)
                // now we try to see which functions belong to a class
                let uncovered_functions = functions
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, snippet)| {
                        if covered_function_idx.contains(&idx) {
                            Some(snippet)
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();

                // we still have the uncovered functions which we want to sort
                // through
                let uncovered_functions = uncovered_functions
                    .into_iter()
                    .map(|uncovered_function| {
                        let formatted_content = uncovered_function.to_xml();
                        let llm_snippet = format!(
                            "<rerank_entry>
<id>
{symbol_index}
</id>
{formatted_content}
</rerank_entry>"
                        );
                        symbol_rerank_information.push(SnippetReRankInformation::new(
                            symbol_index,
                            uncovered_function.range.clone(),
                            uncovered_function.fs_file_path.to_owned(),
                        ));
                        symbol_index = symbol_index + 1;
                        llm_snippet
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                Some((
                    format!(
                        "<rerank_list>
{symbol_list}
{uncovered_functions}
</rerank_list>"
                    ),
                    symbol_rerank_information,
                ))
            }
        } else {
            None
        }
    }
}
