-- Add migration script here
ALTER TABLE messages RENAME COLUMN message TO content;
