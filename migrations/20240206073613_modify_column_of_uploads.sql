-- Add migration script here
ALTER TABLE uploads RENAME COLUMN file_path TO filepath;
