-- Add migration script here
ALTER TABLE messages ALTER COLUMN sent_at SET DEFAULT now();
