-- Add migration script here
ALTER TABLE uploads ADD COLUMN file_path VARCHAR NOT NULL;
