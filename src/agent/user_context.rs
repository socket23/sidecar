//! We are going to implement how tht agent is going to use user context

use anyhow::Result;
use futures::stream;
use futures::StreamExt;

use super::{llm_funcs::LlmClient, types::Agent};

impl Agent {
    pub async fn truncate_user_context(&mut self, query: &str) -> Result<String> {
        // We get different levels of context here from the user:
        // - @file full files (which we have to truncate and get relevant values)
        // - @selection selection ranges from the user (these we have to include including expanding them a bit on each side)
        // - @code code symbols which the user is interested in, which we also keep as it is cause they might be useful
        // so our rough maths is as follows:
        // - @selection (we always keep)
        // - @code (we keep unless its a class in which case we can truncate it a bit)
        // - @file (we have to truncate if it does not fit in the context window)
        let user_context = self
            .user_context
            .as_ref()
            .expect("user_context to be there")
            .clone();
        let user_variables = user_context.variables;
        let user_files = user_context.file_content_map;

        // we want to be fast af, so let's parallelize the lexical search on each
        // of these files and get the queries
        // stream::iter(user_files.into_iter())
        //     .map(|value| {
        //         let fs_file_path = value.file_path;
        //         let file_content = value.file_content;
        //     })
        //     .collect()
        //     .await;
        unimplemented!();
    }
}
