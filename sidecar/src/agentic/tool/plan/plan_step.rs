use uuid::Uuid;

#[derive(Debug)]
pub struct PlanStep {
    // file_paths: Vec<String>,
    id: Uuid,
    index: usize,
    content: String,
    context: Vec<String>,
    // user_context: UserContext,
}

impl PlanStep {
    pub fn new(content: String, index: usize) -> Self {
        PlanStep {
            id: Uuid::new_v4(),
            index,
            content,
            context: Vec::new(),
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn edit_content(&mut self, new_content: String) {
        self.content = new_content;
    }

    pub fn add_context(&mut self, new_context: String) {
        self.context.push(new_context)
    }
}
