use async_trait::async_trait;
use futures::{stream, StreamExt};
use llm_client::{
    broker::LLMBroker,
    clients::types::{LLMClientCompletionRequest, LLMClientMessage},
};
use quick_xml::de::from_str;
use std::{sync::Arc, time::Instant};

use crate::{
    agentic::{
        symbol::{
            events::message_event::SymbolEventMessageProperties,
            identifier::LLMProperties,
            ui_event::{RelevantReference, UIEventWithID},
        },
        tool::{
            errors::ToolError, input::ToolInput, lsp::gotoreferences::AnchoredReference,
            output::ToolOutput, r#type::Tool,
        },
    },
    chunking::types::OutlineNode,
};

/// Represents a request for filtering references in the codebase.
#[derive(Debug, Clone)]
pub struct ReferenceFilterRequest {
    /// The instruction or query provided by the user.
    user_instruction: String,
    llm_properties: LLMProperties,
    /// The unique identifier for the root request.
    root_id: String,
    // we need ui_sender to fire events to ide
    message_properties: SymbolEventMessageProperties,
    anchored_references: Vec<AnchoredReference>,
}

impl ReferenceFilterRequest {
    pub fn new(
        user_instruction: String,
        llm_properties: LLMProperties,
        root_id: String,
        message_properties: SymbolEventMessageProperties,
        anchored_references: Vec<AnchoredReference>,
    ) -> Self {
        Self {
            user_instruction,
            llm_properties,
            root_id,
            message_properties,
            anchored_references,
        }
    }

    pub fn user_instruction(&self) -> &str {
        &self.user_instruction
    }

    pub fn llm_properties(&self) -> &LLMProperties {
        &self.llm_properties
    }

    pub fn root_id(&self) -> &str {
        &self.root_id
    }

    pub fn message_properties(&self) -> &SymbolEventMessageProperties {
        &self.message_properties
    }

    pub fn anchored_references(&self) -> &[AnchoredReference] {
        &self.anchored_references
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename = "response")]
pub struct ReferenceFilterResponse {
    #[serde(rename = "reason")]
    pub reason: String,
    #[serde(rename = "change_required")]
    pub change_required: bool,
}

impl ReferenceFilterResponse {}

pub struct ReferenceFilterBroker {
    llm_client: Arc<LLMBroker>,
    _fail_over_llm: LLMProperties,
}

impl ReferenceFilterBroker {
    pub fn new(llm_client: Arc<LLMBroker>, fail_over_llm: LLMProperties) -> Self {
        Self {
            llm_client,
            _fail_over_llm: fail_over_llm,
        }
    }

    // consider variants: tiny, regular, in-depth
    pub fn system_message(&self) -> String {
        format!(
            r#"You are an expert software engineer who is pair programming with another developer.
- The developer who you are helping with has selected some code which is present in <code_selected> and they intent to change it, the request for change will be provided to you in <user_query>.
- We found a reference for the code present in <code_selected> which is given to you in <reference> section. This means that any change made to <code_selected> might also require changes to the <reference> section.
- Given the changes which will be made to <code_selected> because of the <user_query> you need to decide if we need to change the code in <reference> section.
- Try to give back your reply in a single sentence if possible and keep it very high value.
- <user_query> which CAN lead to additional changes:
- - The user might be changing the function definition
- - The user might be adding a new parameter or removing a parameter for the class
- - Changing code from sync to async
- - and many more such cases which changes the structure and the meaning of the code, as these can be breaking changes.
- You have to decide and be certain if we are going to make a change as true or false, this should be put in a section called <change_required>
- Making a change requires a lot of effort, so be very certain if we should change the code in our selection in <code_selected> based on the <user_query>
- In your reply do not mention the <reference> as reference code, but instead talk about the code symbol.
- Your reason which you will put in the <reason> section of your reply, MUST contain the "WHY" for the change. We MUST explain to the user why the code in <reference> might require a change.

Your response must be in the following format:

<reply>
<response>
<reason>
your single sentence
</reason>
<change_required>
</change_required>
</response>
</reply>"#
        )
    }

    pub fn user_message(
        &self,
        anchored_symbol_prompt: &str,
        user_query: &str,
        reference: &OutlineNode,
    ) -> String {
        format!(
            r#"<user_query>
{}
</user_query>

<code_selected>
{}
</code_selected>

<reference>
{}
</reference>"#,
            user_query,
            anchored_symbol_prompt,
            {
                let name = reference.name();
                let fs_file_path = reference.fs_file_path();
                let content = reference.content().content();

                format!("{} in {}\n{}", name, fs_file_path, content)
            }
        )
    }

    pub fn parse_response(response: &str) -> String {
        // println!("parse_response::response: {}", response);
        let answer = response
            .lines()
            .skip_while(|l| !l.contains("<reply>"))
            .skip(1)
            .take_while(|l| !l.contains("</reply>"))
            .collect::<Vec<&str>>()
            .join("\n");

        answer
    }
}

#[async_trait]
impl Tool for ReferenceFilterBroker {
    async fn invoke(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let context = input.filter_references_request()?;
        let llm_properties = context.llm_properties.clone();
        let root_request_id = context.root_id.to_owned();

        let system_message = LLMClientMessage::system(self.system_message());

        let user_query = context.user_instruction();

        let anchored_references = context.anchored_references().to_vec();

        println!(
            "anchored_references::count: {:?}",
            &anchored_references.len()
        );

        let relevant_references =
            stream::iter(anchored_references.into_iter().map(|anchored_reference| {
                // the user message contains:
                // 1. anchored symbol contents
                // 2. its reference's outline_node
                let user_message = self.user_message(
                    &anchored_reference.anchored_symbol().content(), // content of the anchored symbol
                    user_query,
                    &anchored_reference.ref_outline_node(), // outline node of the reference
                );

                let fs_file_path_for_reference = anchored_reference
                    .reference_location()
                    .fs_file_path()
                    .to_owned();
                let ref_symbol_name = anchored_reference.ref_outline_node().name().to_owned();
                (
                    LLMClientCompletionRequest::new(
                        llm_properties.llm().clone(),
                        vec![system_message.clone(), LLMClientMessage::user(user_message)],
                        0.2,
                        None,
                    ),
                    self.llm_client.clone(),
                    llm_properties.clone(),
                    root_request_id.to_owned(),
                    context.clone(),
                    fs_file_path_for_reference,
                    ref_symbol_name,
                )
            }))
            .map(
                |(
                    request,
                    llm_client,
                    llm_properties,
                    root_request_id,
                    context,
                    fs_file_path_reference,
                    ref_symbol_name_reference,
                )| async move {
                    let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
                    let start = Instant::now();

                    let response = llm_client
                        .stream_completion(
                            llm_properties.api_key().clone(),
                            request,
                            llm_properties.provider().clone(),
                            vec![
                                ("event_type".to_owned(), "filter_references".to_owned()),
                                ("root_id".to_owned(), root_request_id.to_owned()),
                            ]
                            .into_iter()
                            .collect(),
                            sender,
                        )
                        .await;
                    println!(
                        "reference_check::stream_completion::elapsed: {:?}",
                        start.elapsed()
                    );

                    let parsed_response = match response {
                        Ok(response) => {
                            from_str::<ReferenceFilterResponse>(&Self::parse_response(&response))
                                .ok()
                        }
                        Err(_) => None,
                    };

                    if let Some(parsed_response) = parsed_response {
                        if parsed_response.change_required {
                            let ui_sender = context.message_properties().ui_sender();
                            let _ = ui_sender.send(UIEventWithID::relevant_reference(
                                root_request_id.to_owned(),
                                &fs_file_path_reference,
                                &ref_symbol_name_reference,
                                &parsed_response.reason,
                            ));

                            Some(RelevantReference::new(
                                &fs_file_path_reference,
                                &ref_symbol_name_reference,
                                &parsed_response.reason,
                            ))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                },
            )
            .buffer_unordered(200)
            .filter_map(|result| async move { result })
            .collect::<Vec<_>>()
            .await;

        Ok(ToolOutput::ReferencesFilter(relevant_references))
    }
}
