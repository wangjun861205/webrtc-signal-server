-- Add migration script here
ALTER TABLE users ADD COLUMN session_key VARCHAR;

