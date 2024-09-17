//! The diff recent changes which we have made and which are
//! more static than others, we can maintain a l1 and l2 cache
//! style changes, l1 is ONLY including the files which is being
//! edited and the l2 is the part which is long term and more static

use llm_client::clients::types::LLMClientMessage;

/// Contains the diff recent changes, with the caveat that the l1_changes are
/// the variable one and the l2_changes are the static one
#[derive(Debug, Clone, serde::Serialize)]
pub struct DiffRecentChanges {
    l1_changes: String,
    l2_changes: String,
}

impl DiffRecentChanges {
    pub fn new(l1_changes: String, l2_changes: String) -> Self {
        Self {
            l1_changes,
            l2_changes,
        }
    }

    pub fn l1_changes(&self) -> &str {
        &self.l1_changes
    }

    pub fn l2_changes(&self) -> &str {
        &self.l2_changes
    }

    pub fn to_llm_client_message(&self) -> Vec<LLMClientMessage> {
        let l1_changes = self.l1_changes();
        let l2_changes = self.l2_changes();
        let first_part_message = format!(
            r#"
These are the git diff from the files which were recently edited sorted by the least recent to the most recent:
<diff_recent_changes>
{l2_changes}
"#
        );
        let second_part_message = format!(
            r#"{l1_changes}
</diff_recent_changes>
"#
        );
        vec![
            LLMClientMessage::user(first_part_message).cache_point(),
            LLMClientMessage::user(second_part_message),
        ]
    }
}
