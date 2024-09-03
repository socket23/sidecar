use futures::{stream, StreamExt};
use llm_client::{
    clients::{
        anthropic::AnthropicClient,
        types::{LLMClient, LLMClientCompletionRequest, LLMClientMessage, LLMClientRole, LLMType},
    },
    provider::{AnthropicAPIKey, LLMProviderAPIKeys},
};

#[tokio::main]
async fn main() {
    let anthropic_api_key = "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".to_owned();
    let anthropic_client = AnthropicClient::new();
    let api_key = LLMProviderAPIKeys::Anthropic(AnthropicAPIKey::new(anthropic_api_key));
    let system_prompt = r#"Act as an expert software developer.
Always use best practices when coding.
Respect and use existing conventions, libraries, etc that are already present in the code base.
You are diligent and tireless!
You NEVER leave comments describing code without implementing it!
You always COMPLETELY IMPLEMENT the needed code!
You will be presented with a single file and the code which you can EDIT will be given in a <code_to_edit_section>.
You will be also provided with some extra data, which contains various definitions of symbols which you can use to use the call the correct functions and re-use existing functionality in the code.
Take requests for changes to the supplied code.
If the request is ambiguous, ask questions.

Always reply to the user in the same language they are using.

Once you understand the request you MUST:
1. Decide if you need to propose *SEARCH/REPLACE* edits to any files that haven't been added to the chat. You can create new files without asking. But if you need to propose edits to existing files not already added to the chat, you *MUST* tell the user their full path names and ask them to *add the files to the chat*. End your reply and wait for their approval. You can keep asking if you then decide you need to edit more files.
2. Think step-by-step and explain the needed changes with a numbered list of short sentences put this in a xml tag called <thinking> at the very start of your answer.
3. Describe each change with a *SEARCH/REPLACE block* per the examples below. All changes to files must use this *SEARCH/REPLACE block* format. ONLY EVER RETURN CODE IN A *SEARCH/REPLACE BLOCK*!

All changes to files must use the *SEARCH/REPLACE block* format.

# *SEARCH/REPLACE block* Rules:

Every *SEARCH/REPLACE block* must use this format:
1. The file path alone on a line, verbatim. No bold asterisks, no quotes around it, no escaping of characters, etc.
2. The opening fence and code language, eg: ```{language}
3. The start of search block: <<<<<<< SEARCH
4. A contiguous chunk of lines to search for in the existing source code
5. The dividing line: =======
6. The lines to replace into the source code
7. The end of the replace block: >>>>>>> REPLACE
8. The closing fence: ```

Every *SEARCH* section must *EXACTLY MATCH* the existing source code, character for character, including all comments, docstrings, etc.


*SEARCH/REPLACE* blocks will replace *all* matching occurrences.
Include enough lines to make the SEARCH blocks uniquely match the lines to change.

Keep *SEARCH/REPLACE* blocks concise.
Break large *SEARCH/REPLACE* blocks into a series of smaller blocks that each change a small portion of the file.
Include just the changing lines, and a few surrounding lines if needed for uniqueness.
Do not include long runs of unchanging lines in *SEARCH/REPLACE* blocks.

Only create *SEARCH/REPLACE* blocks for files that the user has added to the chat!

To move code within a file, use 2 *SEARCH/REPLACE* blocks: 1 to delete it from its current location, 1 to insert it in the new location.

If you want to put code in a new file, use a *SEARCH/REPLACE block* with:
- A new file path, including dir name if needed
- An empty `SEARCH` section
- The new file's contents in the `REPLACE` section

You are diligent and tireless!
You NEVER leave comments describing code without implementing it!
You always COMPLETELY IMPLEMENT the needed code!
ONLY EVER RETURN CODE IN A *SEARCH/REPLACE BLOCK*!
You always put your thinking in <thinking> section before you suggest *SEARCH/REPLACE* blocks"#;
    fn example_messages() -> Vec<LLMClientMessage> {
        vec![
            LLMClientMessage::user(r#"Change get_factorial() to use math.factorial"#.to_owned()),
            LLMClientMessage::assistant(
                r#"<thinking>
To make this change we need to modify `mathweb/flask/app.py` to:

1. Import the math package.
2. Remove the existing factorial() function.
3. Update get_factorial() to call math.factorial instead.
</thinking>

Here are the *SEARCH/REPLACE* blocks:

mathweb/flask/app.py
```python
<<<<<<< SEARCH
from flask import Flask
=======
import math
from flask import Flask
>>>>>>> REPLACE
```

mathweb/flask/app.py
```python
<<<<<<< SEARCH
def factorial(n):
"compute factorial"

if n == 0:
    return 1
else:
    return n * factorial(n-1)

=======
>>>>>>> REPLACE
```

mathweb/flask/app.py
```python
<<<<<<< SEARCH
return str(factorial(n))
=======
return str(math.factorial(n))
>>>>>>> REPLACE
```"#
                    .to_owned(),
            )
            .cache_point(),
        ]
    }
    let user_request = r#"<extra_symbols_will_be_created>
<symbol>
FILEPATH: /Users/skcd/test_repo/sidecar/sidecar/src/webserver/agentic.rs
code_editing_stop
</symbol>
</extra_symbols_will_be_created>This is the extra data which you can use:
<extra_data>
FILEPATH: /Users/skcd/test_repo/sidecar/sidecar/src/webserver/agentic.rs
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProbeStopRequest {
    request_id: String,
}
</extra_data>
<code_to_edit_selection>
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProbeStopRequest {
    request_id: String,
}
</code_to_edit_selection>
Only edit the code in <code_to_edit_selection> my instructions are:
<user_instruction>
We can reuse the existing ProbeStopRequest struct for the new API as it already contains the necessary request_id field.
</user_insturction>

<fs_file_path>
/Users/skcd/test_repo/sidecar/sidecar/src/webserver/agentic.rs
</fs_file_path>"#;
    let file_paths = vec![
        "/Users/skcd/scratch/sidecar/sidecar/src/webserver/agentic.rs".to_owned(),
        "/Users/skcd/scratch/sidecar/sidecar/src/bin/webserver.rs".to_owned(),
    ];
    let file_content_prompt = stream::iter(file_paths)
        .map(|file_path| async move {
            let file_content = tokio::fs::read(file_path.to_owned())
                .await
                .expect("to work");
            let file_content_str = String::from_utf8(file_content).expect("to work");
            format!(
                r#"<fs_file_path>
{file_path}
</fs_file_path>
<file_content>
{file_content_str}
</file_content>"#
            )
        })
        .buffer_unordered(1)
        .collect::<Vec<_>>()
        .await
        .join("\n");
    let context_message = vec![
        LLMClientMessage::user(format!(
            r#"You can use the code in these files are inspiration for the coding style and writing out the code in the same way as present in the codebase:
{file_content_prompt}"#
        )),
        LLMClientMessage::assistant(
            "I will use these files and follow the coding style present in them".to_owned(),
        )
        .cache_point(),
    ];
    let request = LLMClientCompletionRequest::new(
        LLMType::ClaudeSonnet,
        vec![LLMClientMessage::new(
            LLMClientRole::System,
            "You are an expert at saying hi, you have to say hi 10 times".to_owned(),
        )]
        .into_iter()
        .chain(vec![LLMClientMessage::user(
            "say something to me".to_owned(),
        )])
        .collect::<Vec<_>>(),
        0.1,
        None,
    );
    println!("we are over here");
    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
    let start_instant = std::time::Instant::now();
    let mut response = Box::pin(anthropic_client.stream_completion(api_key, request, sender));
    println!("{}", start_instant.elapsed().as_millis());
    loop {
        tokio::select! {
            stream_msg = receiver.recv() => {
                match stream_msg {
                    Some(msg) => {
                        println!("whats the delta: {:?}", msg.delta());
                    }
                    None => {
                        break;
                    }
                }
            }
            response = &mut response => {
                println!("finished streaming");
                println!("final response: {:?}", response);
            }
        }
    }
    // let client = Client::new();
    // let url = "https://api.anthropic.com/v1/messages";
    // let api_key = "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";

    // let response = client
    //     .post(url)
    //     .header("x-api-key", api_key)
    //     .header("anthropic-version", "2023-06-01")
    //     .header("content-type", "application/json")
    //     .json(&json!({
    //         "model": "claude-3-opus-20240229",
    //         "max_tokens": 1024,
    //         "messages": [
    //             {
    //                 "role": "user",
    //                 "content": "Repeat the following content 5 times"
    //             }
    //         ],
    //         "stream": true
    //     }))
    //     .send()
    //     .await
    //     .expect("to work");

    // if response.status().is_success() {
    //     let body = response.text().await.expect("to work");
    //     println!("Response Body: {}", body);
    // } else {
    //     println!("Request failed with status: {}", response.status());
    // }
}
