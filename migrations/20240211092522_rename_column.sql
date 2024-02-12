-- Add migration script here
ALTER TABLE users RENAME COLUMN apns_token TO fcm_token;
