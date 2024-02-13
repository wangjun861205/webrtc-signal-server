-- Add migration script here
ALTER TABLE messages ADD COLUMN has_read BOOLEAN NOT NULL DEFAULT false;
