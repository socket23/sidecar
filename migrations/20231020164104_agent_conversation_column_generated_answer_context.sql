-- Add migration script here
-- We want to store the generated answer context as well
ALTER TABLE agent_conversation_message ADD COLUMN generated_answer_context TEXT NULL;