use async_trait::async_trait;

use super::agentic::{GenerateSearchPlan, GenerateSearchPlanError, SearchPlanContext};

struct GoogleStudioPlanGenerator {}

#[async_trait]
impl GenerateSearchPlan for GoogleStudioPlanGenerator {
    async fn generate_search_plan(
        &self,
        query: &str,
        context: SearchPlanContext,
    ) -> Result<String, GenerateSearchPlanError> {
        todo!();
    }
}
