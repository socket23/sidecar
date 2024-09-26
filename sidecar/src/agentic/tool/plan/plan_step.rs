use uuid::Uuid;

use crate::user_context::types::UserContext;

#[derive(Debug, Clone)]
pub struct PlanStep {
    id: Uuid,
    index: usize,
    title: String,
    files_to_edit: Vec<String>, // todo(zi): consider whether this should be constrained to just one; paths of files that step may execute against
    description: String,        // we want to keep the step's edit as deterministic as possible
    user_context: Option<UserContext>, // 'Some' if user provides step specific context
}

impl PlanStep {
    pub fn new(
        index: usize,
        files_to_edit: Vec<String>,
        title: String,
        description: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            index,
            title,
            files_to_edit,
            description,
            user_context: None,
        }
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn edit_title(&mut self, new_title: String) {
        self.title = new_title;
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn edit_description(&mut self, new_description: String) {
        self.description = new_description;
    }

    pub fn user_context(&self) -> Option<&UserContext> {
        self.user_context.as_ref()
    }

    pub fn files_to_edit(&self) -> &[String] {
        &self.files_to_edit.as_slice()
    }

    /// Returns first file in Vec. Temporary measure until we decide whether files_to_edit should be an vec.
    pub fn file_to_edit(&self) -> String {
        self.files_to_edit
            .get(0)
            .map(String::as_str)
            .unwrap_or("")
            .to_string()
    }

    pub fn with_user_context(mut self, user_context: UserContext) -> Self {
        self.user_context = Some(user_context);
        self
    }
}

#[derive(Debug, Clone)]
pub struct StepExecutionContext {
    description: String,
    user_context: Option<UserContext>,
}

impl StepExecutionContext {
    pub fn new(description: String, user_context: Option<UserContext>) -> Self {
        Self {
            description,
            user_context,
        }
    }

    pub fn from_plan_step(plan_step: &PlanStep) -> Self {
        Self {
            description: plan_step.description.clone(),
            user_context: plan_step.user_context.clone(),
        }
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn user_context(&self) -> Option<&UserContext> {
        self.user_context.as_ref()
    }

    pub fn update_description(&mut self, new_description: String) {
        self.description = new_description;
    }

    pub fn update_user_context(&mut self, new_user_context: Option<UserContext>) {
        self.user_context = new_user_context;
    }
}

impl From<&PlanStep> for StepExecutionContext {
    fn from(plan_step: &PlanStep) -> Self {
        Self::from_plan_step(plan_step)
    }
}
