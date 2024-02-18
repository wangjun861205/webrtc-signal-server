-- Add migration script here
ALTER TABLE messages ADD COLUMN IF NOT EXISTS mime_type VARCHAR NOT NULL;
