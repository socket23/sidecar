use std::collections::HashMap;

use llm_client::clients::types::{LLMClientCompletionRequest, LLMClientMessage};

use super::types::{
    CodeSpan, CodeSpanDigest, ReRankCodeSpan, ReRankCodeSpanError, ReRankCodeSpanRequest,
    ReRankCodeSpanResponse, ReRankListWiseResponse, ReRankPointWisePrompt, ReRankStrategy,
};

pub struct OpenAIReRank {}

impl OpenAIReRank {
    pub fn new() -> Self {
        Self {}
    }
}

impl OpenAIReRank {
    pub fn pointwise_reranking(&self, request: ReRankCodeSpanRequest) -> ReRankCodeSpanResponse {
        let code_span_digests = CodeSpan::to_digests(request.code_spans().to_vec());
        // Now we query the LLM for the pointwise reranking here
        let user_query = request.user_query().to_owned();
        let prompts = code_span_digests
            .into_iter()
            .map(|code_span_digest| {
                let user_query = user_query.to_owned();
                let hash = code_span_digest.hash();
                let data = code_span_digest.data();
                let prompt = format!(r#"You are an expert software developer responsible for helping detect whether the retrieved snippet of code is relevant to the query. For a given input, you need to output a single word: "Yes" or "No" indicating the retrieved snippet is relevant to the query.
Query: Where is the FastAPI server?
Code Snippet:
```/Users/skcd/server/main.py
from fastapi import FastAPI
app = FastAPI()
@app.get("/")
def read_root():
    return {{"Hello": "World"}}
```
Relevant: Yes

Query: Where in the documentation does it talk about the UI?
Snippet:
```/Users/skcd/bubble_sort/src/lib.rs
fn bubble_sort<T: Ord>(arr: &mut [T]) {{
    for i in 0..arr.len() {{
        for j in 1..arr.len() - i {{
            if arr[j - 1] > arr[j] {{
                arr.swap(j - 1, j);
            }}
        }}
    }}
}}
```
Relevant: No

Query: {user_query}
Snippet:
```{hash}
{data}
```
Relevant:"#);
                let llm_prompt = LLMClientCompletionRequest::from_messages(
                    vec![LLMClientMessage::system(prompt)],
                    request.llm_type().clone(),
                );
                ReRankPointWisePrompt::new_message_request(llm_prompt, code_span_digest)
            })
            .collect();

        ReRankCodeSpanResponse::PointWise(prompts)
    }

    pub fn listwise_reranking(&self, request: ReRankCodeSpanRequest) -> ReRankCodeSpanResponse {
        // First we get the code spans which are present here cause they are important
        let code_spans = request.code_spans().to_vec();
        let user_query = request.user_query().to_owned();
        // Now we need to generate the prompt for this
        let code_span_digests = CodeSpan::to_digests(code_spans);
        let code_snippets = code_span_digests
            .iter()
            .map(|code_span_digest| {
                let identifier = code_span_digest.hash();
                let data = code_span_digest.data();
                let span_identifier = code_span_digest.get_span_identifier();
                format!("{identifier}\n```\n{span_identifier}\n{data}\n```\n")
            })
            .collect::<Vec<String>>()
            .join("\n");
        // Now we create the prompt for this reranking
        let prompt = format!(
            r#"You are an expert at ranking the code snippets for the user query. You have the order the list of code snippets from the most relevant to the least relevant. As an example
<code_snippets>
add.rs::0
```
// FILEPATH: add.rs:0-2
fn add(a: i32, b: i32) -> i32 {{
    a + b
}}
```

subtract.rs::0
```
// FILEPATH: subtract.rs:0-2
fn subtract(a: i32, b: i32) -> i32 {{
    a - b
}}
```
</code_snippets>

And if you thought the code snippet add.rs::0 is more relevant than subtract.rs::0 then you would rank it as:
<ranking>
add.rs::0
subtract.rs::0
</ranking>

The user query might contain a selection of line ranges in the following format:
[#file:foo.rs:4-10](values:file:foo.rs:4-10) this means the line range from 4 to 10 is selected by the user in the file foo.rs

The user has asked the following query: {user_query}
<code_snippets>
{code_snippets}
</code_snippets>

As a reminder the user query is:
<user_query>
{user_query}
</user_query>

The final reranking ordered from the most relevant to the least relevant is:
<ranking>"#
        );
        let llm_prompt = LLMClientCompletionRequest::from_messages(
            vec![LLMClientMessage::system(prompt)],
            request.llm_type().clone(),
        );
        ReRankCodeSpanResponse::listwise_message(llm_prompt, code_span_digests)
    }
}

impl ReRankCodeSpan for OpenAIReRank {
    fn rerank_prompt(
        &self,
        request: ReRankCodeSpanRequest,
    ) -> Result<ReRankCodeSpanResponse, ReRankCodeSpanError> {
        Ok(match request.strategy() {
            ReRankStrategy::ListWise => self.listwise_reranking(request),
            ReRankStrategy::PointWise => {
                // We need to generate the prompt for this
                self.pointwise_reranking(request)
            }
        })
    }

    fn parse_listwise_output(
        &self,
        llm_output: String,
        rerank_request: ReRankListWiseResponse,
    ) -> Result<Vec<CodeSpanDigest>, ReRankCodeSpanError> {
        // In case of OpenAI things are a bit easier, since the list is properly formatted
        // almost always and we can just grab the ids from the list and rank the
        // code snippets based that.
        let mut output = llm_output.split("\n");
        let mut code_spans_mapping: HashMap<String, CodeSpanDigest> = rerank_request
            .code_span_digests
            .into_iter()
            .map(|code_span_digest| (code_span_digest.hash().to_owned(), code_span_digest))
            .collect();
        let mut reranked_code_snippets: Vec<CodeSpanDigest> = vec![];
        while let Some(line) = output.next() {
            let line_output = line.trim();
            if line_output.contains("</ranking>") {
                break;
            }
            let possible_id = line.trim();
            if let Some(code_span) = code_spans_mapping.remove(possible_id) {
                reranked_code_snippets.push(code_span);
            }
        }
        // Add back the remaining code snippets to the list
        code_spans_mapping.into_iter().for_each(|(_, code_span)| {
            reranked_code_snippets.push(code_span);
        });
        Ok(reranked_code_snippets)
    }
}
