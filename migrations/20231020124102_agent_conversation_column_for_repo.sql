-- Add migration script here
-- We need the repository reference here so we can load the previous conversation
-- properly
ALTER TABLE agent_conversation_message ADD COLUMN repo_ref TEXT NOT NULL;