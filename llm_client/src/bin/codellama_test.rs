use llm_client::clients::togetherai::TogetherAIClient;
use llm_client::clients::types::LLMClientCompletionStringRequest;
use llm_client::{
    clients::types::{LLMClient, LLMClientCompletionRequest, LLMClientMessage},
    provider::TogetherAIProvider,
};

#[tokio::main]
async fn main() {
    let togetherai = TogetherAIClient::new();
    let api_key = llm_client::provider::LLMProviderAPIKeys::TogetherAI(TogetherAIProvider {
        api_key: "cc10d6774e67efef2004b85efdb81a3c9ba0b7682cc33d59c30834183502208d".to_owned(),
    });
    let message = r#"<s>Source: system

 #### USER SELECTED CONTEXT ####
####
Your job is to answer the user query.

When referring to code, you must provide an example in a code block.

- You are given the following project labels which are associated with the codebase:
- cargo
- rust


Respect these rules at all times:
- When asked for your name, you must respond with "Aide".
- Follow the user's requirements carefully & to the letter.
- Minimize any other prose.
- Unless directed otherwise, the user is expecting for you to edit their selected code.
- Link ALL paths AND code symbols (functions, methods, fields, classes, structs, types, variables, values, definitions, directories, etc) by embedding them in a markdown link, with the URL corresponding to the full path, and the anchor following the form `LX` or `LX-LY`, where X represents the starting line number, and Y represents the ending line number, if the reference is more than one line.
    - For example, to refer to lines 50 to 78 in a sentence, respond with something like: The compiler is initialized in [`src/foo.rs`](/Users/nareshr/github/codestory/sidecarsrc/foo.rs#L50-L78)
    - For example, to refer to the `new` function on a struct, respond with something like: The [`new`](/Users/nareshr/github/codestory/sidecarsrc/bar.rs#L26-53) function initializes the struct
    - For example, to refer to the `foo` field on a struct and link a single line, respond with something like: The [`foo`](/Users/nareshr/github/codestory/sidecarsrc/foo.rs#L138) field contains foos. Do not respond with something like [`foo`](/Users/nareshr/github/codestory/sidecarsrc/foo.rs#L138-L138)
    - For example, to refer to a folder `foo`, respond with something like: The files can be found in [`foo`](/Users/nareshr/github/codestory/sidecarpath/to/foo/) folder
- Do not print out line numbers directly, only in a link
- Do not refer to more lines than necessary when creating a line range, be precise
- Do NOT output bare symbols. ALL symbols must include a link
    - E.g. Do not simply write `Bar`, write [`Bar`](/Users/nareshr/github/codestory/sidecarsrc/bar.rs#L100-L105).
    - E.g. Do not simply write "Foos are functions that create `Foo` values out of thin air." Instead, write: "Foos are functions that create [`Foo`](/Users/nareshr/github/codestory/sidecarsrc/foo.rs#L80-L120) values out of thin air."
- Link all fields
    - E.g. Do not simply write: "It has one main field: `foo`." Instead, write: "It has one main field: [`foo`](/Users/nareshr/github/codestory/sidecarsrc/foo.rs#L193)."
- Do NOT link external urls not present in the context, do NOT link urls from the internet
- Link all symbols, even when there are multiple in one sentence
    - E.g. Do not simply write: "Bars are [`Foo`]( that return a list filled with `Bar` variants." Instead, write: "Bars are functions that return a list filled with [`Bar`](/Users/nareshr/github/codestory/sidecarsrc/bar.rs#L38-L57) variants."
- Code blocks MUST be displayed to the user using markdown
- Code blocks MUST be displayed to the user using markdown and must NEVER include the line numbers
- If you are going to not edit sections of the code, leave "// rest of code .." as the placeholder string
- Do NOT write the line number in the codeblock
    - E.g. Do not write:
    ```rust
    1. // rest of code ..
    2. // rest of code ..
    ```
    Here the codeblock has line numbers 1 and 2, do not write the line numbers in the codeblock
- You are given the code which the user has selected explicitly in the USER SELECTED CODE section
- Pay special attention to the USER SELECTED CODE as these code snippets are specially selected by the user in their query <step> Source: user

Write me a function in Rust to add 2 numbers <step> Source: assistant
Destination: user

 "#;

    let messages = vec![
        LLMClientMessage::system("You are a helpful coding assistant.".to_owned()),
        LLMClientMessage::user(
            "Can you help me write a function in rust which adds 2 numbers".to_owned(),
        ),
    ];
    let message_request = LLMClientCompletionRequest::new(
        llm_client::clients::types::LLMType::CodeLLama70BInstruct,
        messages,
        1.0,
        None,
    );
    let request = LLMClientCompletionStringRequest::new(
        llm_client::clients::types::LLMType::CodeLLama70BInstruct,
        message.to_owned(),
        1.0,
        None,
    );
    // let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
    // let response = togetherai
    //     .stream_completion(api_key, message_request, sender)
    //     .await;
    // dbg!(&request);
    let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
    let response = togetherai
        .stream_prompt_completion(api_key, request, sender)
        .await;
    dbg!(&response);
}
