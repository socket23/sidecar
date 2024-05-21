use crate::chunking::text_document::Range;

/// We need here the outline of the symbol which needs to be chaned so we can give
/// it some instruction or else a better thing to do here would be, to just show
/// the changes and why it was made and send that as a request to the symbol
/// This way the other symbol becomes responsible for handling the change as required
/// since it will also know the context in which the changes need to be made
#[derive(Debug, Clone)]
pub struct CodeFollowupEditDeicisionRequest {
    fs_file_path: String,
    // These are the portions for the followup code symbol
    code_above: Option<String>,
    code_below: Option<String>,
    code_in_selection: String,
    symbol_outline: Option<String>,
    symbol_name: String,
    // This range here refers to the portion of the code_in_selection
    // which needs to be highlighted as the reference symbol
    code_in_selection_highlight: Range,
    previous_code: String,
    instructions: String,
}
