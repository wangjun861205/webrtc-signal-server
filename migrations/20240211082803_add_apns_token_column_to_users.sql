-- Add migration script here
ALTER TABLE users ADD COLUMN apns_token VARCHAR;
