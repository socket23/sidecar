use crate::agentic::tool::code_symbol::important::CodeSymbolImportantResponse;

/// Contains the caching utility for long context query with the initial search
pub struct LongContextSearchCache {
    // contains the cache which goes from the instance_id to the content
    cache_location: String,
}

impl LongContextSearchCache {
    pub fn new() -> Self {
        Self {
            cache_location: "/Users/skcd/scratch/swe_bench/chat-logs/full---gpt-4o".to_owned(),
        }
    }
    fn file_path(&self, instance_id: &str) -> String {
        self.cache_location.to_owned() + &format!("/{instance_id}")
    }

    pub async fn update_cache(
        &self,
        instance_id: Option<String>,
        content: &CodeSymbolImportantResponse,
    ) {
        if instance_id.is_none() {
            return;
        }
        let instance_id = instance_id.expect("is_none to hold");
        let file_path = self.file_path(&instance_id);
        let _ = tokio::fs::write(
            file_path,
            serde_json::to_string(content).expect("to always work"),
        )
        .await;
        return;
    }

    pub async fn check_cache(&self, instance_id: &str) -> Option<CodeSymbolImportantResponse> {
        let file_path = self.file_path(instance_id);
        let contents = tokio::fs::read(file_path).await.map(|output| {
            String::from_utf8(output)
                .map(|str_output| serde_json::from_str::<CodeSymbolImportantResponse>(&str_output))
        });
        match contents {
            // please don't laugh at me
            Ok(Ok(Ok(contents))) => Some(contents),
            _ => None,
        }
    }
}
