use crate::user_context::types::UserContext;

struct PlanStep {
    // file_paths: Vec<String>,
    content: String,
    context: Vec<String>,
    // user_context: UserContext,
}

impl PlanStep {
    pub fn new(content: String) -> Self {
        PlanStep {
            content,
            context: Vec::new(),
        }
    }

    pub fn edit_content(&mut self, new_content: String) {
        self.content = new_content;
    }

    pub fn add_context(&mut self, new_context: String) {
        self.context.push(new_context)
    }
}
