//! We are going to log the UI events, this is mostly for
//! debugging and having better visibility to what ever is happening
//! in the symbols

use crate::{agentic::tool::input::ToolInput, chunking::text_document::Range};

use super::{
    events::input::SymbolInputEvent,
    identifier::SymbolIdentifier,
    types::{SymbolEventRequest, SymbolLocation},
};

#[derive(Debug, serde::Serialize)]
pub struct UIEventWithID {
    request_id: String,
    event: UIEvent,
}

impl UIEventWithID {
    pub fn finish_edit_request(request_id: String) -> Self {
        Self {
            request_id: request_id.to_owned(),
            event: UIEvent::EditRequestFinished(request_id),
        }
    }
    pub fn from_tool_event(request_id: String, input: ToolInput) -> Self {
        Self {
            request_id,
            event: UIEvent::from(input),
        }
    }

    pub fn from_symbol_event(request_id: String, input: SymbolEventRequest) -> Self {
        Self {
            request_id,
            event: UIEvent::SymbolEvent(input),
        }
    }

    pub fn for_codebase_event(request_id: String, input: SymbolInputEvent) -> Self {
        Self {
            request_id,
            event: UIEvent::CodebaseEvent(input),
        }
    }

    pub fn symbol_location(request_id: String, symbol_location: SymbolLocation) -> Self {
        Self {
            request_id,
            event: UIEvent::SymbolLoctationUpdate(symbol_location),
        }
    }

    pub fn sub_symbol_step(
        request_id: String,
        sub_symbol_request: SymbolEventSubStepRequest,
    ) -> Self {
        Self {
            request_id,
            event: UIEvent::SymbolEventSubStep(sub_symbol_request),
        }
    }

    pub fn probe_answer_event(
        request_id: String,
        symbol_identifier: SymbolIdentifier,
        probe_answer: String,
    ) -> Self {
        Self {
            request_id,
            event: UIEvent::SymbolEventSubStep(SymbolEventSubStepRequest::new(
                symbol_identifier,
                SymbolEventSubStep::Probe(SymbolEventProbeRequest::ProbeAnswer(probe_answer)),
            )),
        }
    }

    pub fn probing_started_event(request_id: String) -> Self {
        Self {
            request_id,
            event: UIEvent::RequestEvent(RequestEvents::ProbingStart),
        }
    }

    pub fn probing_finished_event(request_id: String, response: String) -> Self {
        Self {
            request_id,
            event: UIEvent::RequestEvent(RequestEvents::ProbeFinished(
                RequestEventProbeFinished::new(response),
            )),
        }
    }

    pub fn range_selection_for_edit(
        request_id: String,
        symbol_identifier: SymbolIdentifier,
        range: Range,
        fs_file_path: String,
    ) -> Self {
        Self {
            request_id,
            event: UIEvent::SymbolEventSubStep(
                SymbolEventSubStepRequest::range_selection_for_edit(
                    symbol_identifier,
                    fs_file_path,
                    range,
                ),
            ),
        }
    }

    pub fn edited_code(
        request_id: String,
        symbol_identifier: SymbolIdentifier,
        range: Range,
        fs_file_path: String,
        edited_code: String,
    ) -> Self {
        Self {
            request_id,
            event: UIEvent::SymbolEventSubStep(SymbolEventSubStepRequest::edited_code(
                symbol_identifier,
                range,
                fs_file_path,
                edited_code,
            )),
        }
    }

    pub fn code_correctness_action(
        request_id: String,
        symbol_identifier: SymbolIdentifier,
        range: Range,
        fs_file_path: String,
        tool_use_thinking: String,
    ) -> Self {
        Self {
            request_id,
            event: UIEvent::SymbolEventSubStep(SymbolEventSubStepRequest::code_correctness_action(
                symbol_identifier,
                range,
                fs_file_path,
                tool_use_thinking,
            )),
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub enum UIEvent {
    SymbolEvent(SymbolEventRequest),
    ToolEvent(ToolInput),
    CodebaseEvent(SymbolInputEvent),
    SymbolLoctationUpdate(SymbolLocation),
    SymbolEventSubStep(SymbolEventSubStepRequest),
    RequestEvent(RequestEvents),
    EditRequestFinished(String),
}

impl From<SymbolEventRequest> for UIEvent {
    fn from(req: SymbolEventRequest) -> Self {
        UIEvent::SymbolEvent(req)
    }
}

impl From<ToolInput> for UIEvent {
    fn from(input: ToolInput) -> Self {
        UIEvent::ToolEvent(input)
    }
}

#[derive(Debug, serde::Serialize)]
pub enum SymbolEventProbeRequest {
    SubSymbolSelection,
    ProbeDeeperSymbol,
    /// The final answer for the probe is sent via this event
    ProbeAnswer(String),
}

#[derive(Debug, serde::Serialize)]
pub struct SymbolEventGoToDefinitionRequest {
    fs_file_path: String,
    range: Range,
    thinking: String,
}

impl SymbolEventGoToDefinitionRequest {
    fn new(fs_file_path: String, range: Range, thinking: String) -> Self {
        Self {
            fs_file_path,
            range,
            thinking,
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct RangeSelectionForEditRequest {
    range: Range,
    fs_file_path: String,
}

impl RangeSelectionForEditRequest {
    pub fn new(range: Range, fs_file_path: String) -> Self {
        Self {
            range,
            fs_file_path,
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct InsertCodeForEditRequest {
    range: Range,
    fs_file_path: String,
}

#[derive(Debug, serde::Serialize)]
pub struct EditedCodeForEditRequest {
    range: Range,
    fs_file_path: String,
    new_code: String,
}

impl EditedCodeForEditRequest {
    pub fn new(range: Range, fs_file_path: String, new_code: String) -> Self {
        Self {
            range,
            fs_file_path,
            new_code,
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct CodeCorrectionToolSelection {
    range: Range,
    fs_file_path: String,
    tool_use_thinking: String,
}

impl CodeCorrectionToolSelection {
    pub fn new(range: Range, fs_file_path: String, tool_use_thinking: String) -> Self {
        Self {
            range,
            fs_file_path,
            tool_use_thinking,
        }
    }
}

/// We have range selection and then the edited code, we should also show the
/// events which the AI is using for the tool correction and whats it is planning
/// on doing for that
#[derive(Debug, serde::Serialize)]
pub enum SymbolEventEditRequest {
    RangeSelectionForEdit(RangeSelectionForEditRequest),
    /// We might be inserting code at a line which is a new symbol by itself
    InsertCode(InsertCodeForEditRequest),
    EditCode(EditedCodeForEditRequest),
    CodeCorrectionTool(CodeCorrectionToolSelection),
}

#[derive(Debug, serde::Serialize)]
pub enum SymbolEventSubStep {
    Probe(SymbolEventProbeRequest),
    GoToDefinition(SymbolEventGoToDefinitionRequest),
    Edit(SymbolEventEditRequest),
}

#[derive(Debug, serde::Serialize)]
pub struct SymbolEventSubStepRequest {
    symbol_identifier: SymbolIdentifier,
    event: SymbolEventSubStep,
}

impl SymbolEventSubStepRequest {
    pub fn new(symbol_identifier: SymbolIdentifier, event: SymbolEventSubStep) -> Self {
        Self {
            symbol_identifier,
            event,
        }
    }

    pub fn probe_answer(symbol_identifier: SymbolIdentifier, answer: String) -> Self {
        Self {
            symbol_identifier,
            event: SymbolEventSubStep::Probe(SymbolEventProbeRequest::ProbeAnswer(answer)),
        }
    }

    pub fn go_to_definition_request(
        symbol_identifier: SymbolIdentifier,
        fs_file_path: String,
        range: Range,
        thinking: String,
    ) -> Self {
        Self {
            symbol_identifier,
            event: SymbolEventSubStep::GoToDefinition(SymbolEventGoToDefinitionRequest::new(
                fs_file_path,
                range,
                thinking,
            )),
        }
    }

    pub fn range_selection_for_edit(
        symbol_identifier: SymbolIdentifier,
        fs_file_path: String,
        range: Range,
    ) -> Self {
        Self {
            symbol_identifier,
            event: SymbolEventSubStep::Edit(SymbolEventEditRequest::RangeSelectionForEdit(
                RangeSelectionForEditRequest::new(range, fs_file_path),
            )),
        }
    }

    pub fn edited_code(
        symbol_identifier: SymbolIdentifier,
        range: Range,
        fs_file_path: String,
        edited_code: String,
    ) -> Self {
        Self {
            symbol_identifier,
            event: SymbolEventSubStep::Edit(SymbolEventEditRequest::EditCode(
                EditedCodeForEditRequest::new(range, fs_file_path, edited_code),
            )),
        }
    }

    pub fn code_correctness_action(
        symbol_identifier: SymbolIdentifier,
        range: Range,
        fs_file_path: String,
        tool_use_thinking: String,
    ) -> Self {
        Self {
            symbol_identifier,
            event: SymbolEventSubStep::Edit(SymbolEventEditRequest::CodeCorrectionTool(
                CodeCorrectionToolSelection::new(range, fs_file_path, tool_use_thinking),
            )),
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct RequestEventProbeFinished {
    reply: String,
}

impl RequestEventProbeFinished {
    pub fn new(reply: String) -> Self {
        Self { reply }
    }
}

#[derive(Debug, serde::Serialize)]
pub enum RequestEvents {
    ProbingStart,
    ProbeFinished(RequestEventProbeFinished),
}
