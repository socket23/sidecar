use uuid::Uuid;

use crate::user_context::types::UserContext;

#[derive(Debug, Clone)]
pub struct PlanStep {
    id: Uuid,
    index: usize,
    file_paths: Vec<String>,
    description: String, // we want to keep the step's edit as deterministic as possible
    context: Vec<String>,
    user_context: UserContext, // @symbols, @files, @last_edits etc.
                               // possibly, edits made
                               // i.e. step 1: edit x made in file y
}

impl PlanStep {
    pub fn new(
        description: String,
        index: usize,
        file_paths: Vec<String>,
        user_context: UserContext,
    ) -> Self {
        PlanStep {
            id: Uuid::new_v4(),
            index,
            description,
            context: Vec::new(),
            file_paths,
            user_context,
        }
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

    pub fn add_context(&mut self, new_context: String) {
        self.context.push(new_context)
    }

    pub fn user_context(&self) -> &UserContext {
        &self.user_context
    }

    pub fn file_paths(&self) -> &[String] {
        &self.file_paths.as_slice()
    }
}

// given a step,

// and whatever context,

// the step should be updated

// ren
