/// We are going to test out the mistral model here and try and figure out
/// if we can get the inference to work properly
use async_openai::{
    config::{AzureConfig, OpenAIConfig},
    types::{ChatCompletionRequestMessageArgs, CreateChatCompletionRequestArgs, Role},
    Client,
};
use futures::StreamExt;

#[tokio::main]
async fn main() {
    let api_base = "".to_owned();
    let api_key = "".to_owned();
    let api_version = "".to_owned();
    // This is as easy as it is to get this to work
    // now we try to see what features we can power
    let mistral_config = OpenAIConfig::new().with_api_base("http://localhost:1234/v1".to_owned());
    let mistral_azure_config =
        AzureConfig::new().with_api_base("http://localhost:1234/v1".to_owned());
    let mut request_args = CreateChatCompletionRequestArgs::default();
    // let system_message = "\nYou are an AI programming assistant.\nWhen asked for your name, you must respond with \"Aide\".\nFollow the user's requirements carefully & to the letter.\n- Each code block must ALWAYS STARTS and include ```typescript and // FILEPATH\n- You always answer with typescript code.\n- When the user asks you to document something, you must answer in the form of a typescript code block.\n- Your documentation should not include just the name of the function, think about what the function is really doing.\n- When generating the documentation, be sure to understand what the function is doing and include that as part of the documentation and then generate the documentation.\n- DO NOT modify the code which you will be generating\n    ";
    // let mut system_message = ChatCompletionRequestMessageArgs::default()
    //     .role(Role::System)
    //     .content(system_message)
    //     .build()
    //     .unwrap();
    // let user_message = ChatCompletionRequestMessageArgs::default()
    //     .role(Role::User)
    //     .content("I have the following code in the selection:\n```typescript\n// FILEPATH: /Users/skcd/test_repo/axflow/packages/models/src/ollama/generation.ts\nfunction noop(chunk: OllamaGenerationTypes.Chunk) {\n  return chunk;\n}\n```\n")
    //     .build()
    //     .unwrap();
    // let user_message2 = ChatCompletionRequestMessageArgs::default()
    //     .role(Role::User)
    //     .content("Please add a TSDoc comment for noop. can you add comments to this function. Do not forget to include the FILEPATH marker in your generated code.")
    //     .build()
    //     .unwrap();
    let system_message = ChatCompletionRequestMessageArgs::default()
        .role(Role::System)
        .content("\nWhen asked for your name, you must respond with \"Aide\".\nFollow the user's requirements carefully & to the letter.\nYour responses should be informative and logical.\nYou should always adhere to technical information.\nIf the user asks for code or technical questions, you must provide code suggestions and adhere to technical information.\nIf the question is related to a developer, you must respond with content related to a developer.\n\nA software developer is using an AI chatbot in a code editor.\nThe developer added the following request to the chat and your goal is to select a function to perform the request.\n\nRequest: can you add a comment to this?\n\nAvailable functions:\nFunction Id: code\nFunction Description: Add code to an already existing code base\n\nFunction Id: doc\nFunction Description: Add documentation comment for this symbol\n\nFunction Id: edit\nFunction Description: Refactors the selected code based on requirements provided by the user\n\nFunction Id: tests\nFunction Description: Generate unit tests for the selected code\n\nFunction Id: fix\nFunction Description: Propose a fix for the problems in the selected code\n\nFunction Id: explain\nFunction Description: Explain how the selected code works\n\nFunction Id: unknown\nFunction Description: Intent of this command is unclear or is not related to information technologies\n\n\nHere are some examples to make the instructions clearer:\nRequest: Add a function that returns the sum of two numbers\nResponse: code\n\nRequest: Add jsdoc to this method\nResponse: doc\n\nRequest: Change this method to use async/await\nResponse: edit\n\nRequest: Write a set of detailed unit test functions for the code above.\nResponse: tests\n\nRequest: There is a problem in this code. Rewrite the code to show it with the bug fixed.\nResponse: fix\n\nRequest: Write an explanation for the code above as paragraphs of text.\nResponse: explain\n\nRequest: Add a dog to this comment.\nResponse: unknown\n\nRequest: can you add a comment to this?\nResponse:\n    ")
        .build()
        .unwrap();
    let azure_client = Client::with_config(mistral_azure_config);
    let client = Client::with_config(mistral_config);
    let max_tokens: u16 = 1000;
    let chat_request_args = request_args
        .messages(vec![system_message])
        .temperature(0.0)
        .max_tokens(max_tokens)
        .build()
        .unwrap();
    dbg!(&chat_request_args);
    dbg!("sending request here");
    let stream_messages = client
        .chat()
        .create_stream(chat_request_args)
        .await
        .expect("to work");
    dbg!("stream created");
    // We are going to stream the messages here
    let mut buffered_answer = "".to_owned();
    stream_messages
        .for_each(|message| {
            match message {
                Ok(message) => {
                    buffered_answer = buffered_answer.to_owned()
                        + message.choices[0]
                            .delta
                            .content
                            .as_ref()
                            .expect("to be present");
                    println!("{}", buffered_answer.to_owned());
                }
                Err(_) => {}
            }
            futures::future::ready(())
        })
        .await;
}
