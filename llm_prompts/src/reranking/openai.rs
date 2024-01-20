use llm_client::clients::types::{LLMClientCompletionRequest, LLMClientMessage};

use super::types::{
    CodeSpan, ReRankCodeSpan, ReRankCodeSpanError, ReRankCodeSpanRequest, ReRankCodeSpanResponse,
    ReRankPointWisePrompt, ReRankStrategy,
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
                format!("{identifier}\n```\n{data}\n```\n")
            })
            .collect::<Vec<String>>()
            .join("\n");
        // Now we create the prompt for this reranking
        let prompt = format!(
            r#"You are an expert at ranking the code snippets for the user query. You have the order the list of code snippets from the most relevant to the least relevant. As an example
<code_snippets>
add.rs::0
```
fn add(a: i32, b: i32) -> i32 {{
    a + b
}}
```

subtract.rs::0
```
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

The user has asked the following query: {user_query}
<code_snippets>
{code_snippets}
</code_snippets>

The final reranking:
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
}
