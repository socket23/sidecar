use llm_client::{
    clients::{
        anthropic::AnthropicClient,
        types::{LLMClient, LLMClientCompletionRequest, LLMClientMessage, LLMClientRole, LLMType},
    },
    provider::{AnthropicAPIKey, LLMProviderAPIKeys},
};
use reqwest::Client;
use serde_json::json;

#[tokio::main]
async fn main() {
    let anthropic_api_key = "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".to_owned();
    let anthropic_client = AnthropicClient::new();
    let api_key = LLMProviderAPIKeys::Anthropic(AnthropicAPIKey::new(anthropic_api_key));
    let request = LLMClientCompletionRequest::new(
        LLMType::ClaudeOpus,
        vec![
            LLMClientMessage::new(
                LLMClientRole::System,
                "you are an expert software engineer".to_owned(),
            ),
            // LLMClientMessage::new(LLMClientRole::System, "##### PATHS #####\n/Users/skcd/scratch/sidecar/llm_client/src/bin/anthropic.rs\n#### USER SELECTED CONTEXT ####\n\n##### CODE CHUNKS #####\n\n### /Users/skcd/scratch/sidecar/llm_client/src/bin/anthropic.rs ###\n2     clients::{\n3         anthropic::AnthropicClient,\n4         types::{LLMClient, LLMClientCompletionRequest, LLMClientMessage, LLMClientRole, LLMType},\n5     },\n6     provider::{AnthropicAPIKey, LLMProviderAPIKeys},\n7 };\n8 use reqwest::Client;\n9 use serde_json::json;\n10 \n11 #[tokio::main]\n12 async fn main() {\n13     let anthropic_api_key = \"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\".to_owned();\n14     let anthropic_client = AnthropicClient::new();\n15     let api_key = LLMProviderAPIKeys::Anthropic(AnthropicAPIKey::new(anthropic_api_key));\n16     let request = LLMClientCompletionRequest::new(\n17         LLMType::ClaudeOpus,\n18         vec![\n19             LLMClientMessage::new(LLMClientRole::System, \"you are an expert\".to_owned()),\n20             LLMClientMessage::new(LLMClientRole::User, \"Can you say 5, 5 times\".to_owned()),\n21         ],\n22         0.1,\n23         None,\n24     )\n25     .set_max_tokens(50000);\n26     let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();\n27     let response = anthropic_client\n28         .stream_completion(api_key, request, sender)\n29         .await;\n30     println!(\"{:?}\", response);\n31     // let client = Client::new();\n32     // let url = \"https://api.anthropic.com/v1/messages\";\n33     // let api_key = \"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\";\n34 \n35     // let response = client\n36     //     .post(url)\n37     //     .header(\"x-api-key\", api_key)\n38     //     .header(\"anthropic-version\", \"2023-06-01\")\n39     //     .header(\"content-type\", \"application/json\")\n40     //     .json(&json!({\n41     //         \"model\": \"claude-3-opus-20240229\",\n42     //         \"max_tokens\": 1024,\n43     //         \"messages\": [\n44     //             {\n45     //                 \"role\": \"user\",\n46     //                 \"content\": \"Repeat the following content 5 times\"\n47     //             }\n48     //         ],\n49     //         \"stream\": true\n50     //     }))\n51     //     .send()\n52     //     .await\n53     //     .expect(\"to work\");\n\n\n####\nYour job is to answer the user query.\n\nWhen referring to code, you must provide an example in a code block.\n\n- You are given the following project labels which are associated with the codebase:\n- rust\n- cargo\n\n\nRespect these rules at all times:\n- When asked for your name, you must respond with \"Aide\".\n- Follow the user's requirements carefully & to the letter.\n- Minimize any other prose.\n- Unless directed otherwise, the user is expecting for you to edit their selected code.\n- Link ALL paths AND code symbols (functions, methods, fields, classes, structs, types, variables, values, definitions, directories, etc) by embedding them in a markdown link, with the URL corresponding to the full path, and the anchor following the form `LX` or `LX-LY`, where X represents the starting line number, and Y represents the ending line number, if the reference is more than one line.\n    - For example, to refer to lines 50 to 78 in a sentence, respond with something like: The compiler is initialized in [`src/foo.rs`](/Users/skcd/scratch/sidecarsrc/foo.rs#L50-L78)\n    - For example, to refer to the `new` function on a struct, respond with something like: The [`new`](/Users/skcd/scratch/sidecarsrc/bar.rs#L26-53) function initializes the struct\n    - For example, to refer to the `foo` field on a struct and link a single line, respond with something like: The [`foo`](/Users/skcd/scratch/sidecarsrc/foo.rs#L138) field contains foos. Do not respond with something like [`foo`](/Users/skcd/scratch/sidecarsrc/foo.rs#L138-L138)\n    - For example, to refer to a folder `foo`, respond with something like: The files can be found in [`foo`](/Users/skcd/scratch/sidecarpath/to/foo/) folder\n- Do not print out line numbers directly, only in a link\n- Do not refer to more lines than necessary when creating a line range, be precise\n- Do NOT output bare symbols. ALL symbols must include a link\n    - E.g. Do not simply write `Bar`, write [`Bar`](/Users/skcd/scratch/sidecarsrc/bar.rs#L100-L105).\n    - E.g. Do not simply write \"Foos are functions that create `Foo` values out of thin air.\" Instead, write: \"Foos are functions that create [`Foo`](/Users/skcd/scratch/sidecarsrc/foo.rs#L80-L120) values out of thin air.\"\n- Link all fields\n    - E.g. Do not simply write: \"It has one main field: `foo`.\" Instead, write: \"It has one main field: [`foo`](/Users/skcd/scratch/sidecarsrc/foo.rs#L193).\"\n- Do NOT link external urls not present in the context, do NOT link urls from the internet\n- Link all symbols, even when there are multiple in one sentence\n    - E.g. Do not simply write: \"Bars are [`Foo`]( that return a list filled with `Bar` variants.\" Instead, write: \"Bars are functions that return a list filled with [`Bar`](/Users/skcd/scratch/sidecarsrc/bar.rs#L38-L57) variants.\"\n- Code blocks MUST be displayed to the user using markdown\n- Code blocks MUST be displayed to the user using markdown and must NEVER include the line numbers\n- If you are going to not edit sections of the code, leave \"// rest of code ..\" as the placeholder string\n- Do NOT write the line number in the codeblock\n    - E.g. Do not write:\n    ```rust\n    1. // rest of code ..\n    2. // rest of code ..\n    ```\n    Here the codeblock has line numbers 1 and 2, do not write the line numbers in the codeblock\n- You are given the code which the user has selected explicitly in the USER SELECTED CODE section\n- Pay special attention to the USER SELECTED CODE as these code snippets are specially selected by the user in their query".to_owned()),
            LLMClientMessage::new(LLMClientRole::User, "tell me add numbers in".to_owned()),
            // LLMClientMessage::new(LLMClientRole::User, "rust".to_owned()),
        ],
        0.1,
        None,
    )
    .set_max_tokens(50000);
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
    let response = anthropic_client
        .stream_completion(api_key, request, sender)
        .await;
    println!("{:?}", response);
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
