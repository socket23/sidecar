use async_trait::async_trait;
use thiserror::Error;

#[async_trait]
pub trait GenerateSearchPlan {
    async fn generate_search_plan(
        &self,
        query: &str,
        context: SearchPlanContext,
    ) -> Result<String, GenerateSearchPlanError>;
}

pub enum SearchPlanContext {
    RepoTree(String),
}

#[derive(Debug, Error)]
pub enum GenerateSearchPlanError {
    #[error("generic error: {0}")]
    Generic(String),
}
