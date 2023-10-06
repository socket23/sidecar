use std::collections::HashMap;

use super::schema::Payload;

impl Payload {
    pub fn into_qdrant(self) -> HashMap<String, qdrant_client::qdrant::Value> {
        HashMap::from([
            ("lang".into(), self.lang.to_ascii_lowercase().into()),
            ("repo_name".into(), self.repo_name.into()),
            ("repo_ref".into(), self.repo_ref.into()),
            ("relative_path".into(), self.relative_path.into()),
            ("content_hash".into(), self.content_hash.into()),
            ("snippet".into(), self.text.into()),
            ("start_line".into(), self.start_line.to_string().into()),
            ("end_line".into(), self.end_line.to_string().into()),
            ("start_byte".into(), self.start_byte.to_string().into()),
            ("end_byte".into(), self.end_byte.to_string().into()),
            ("branches".into(), self.branches.into()),
            ("commit_hash".into(), self.commit_hash.into()),
        ])
    }
}
