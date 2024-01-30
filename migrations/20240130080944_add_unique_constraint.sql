-- Add migration script here
ALTER TABLE friend_requests ADD CONSTRAINT uni_from_to UNIQUE ("from", "to");
