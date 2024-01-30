-- Add migration script here

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE IF NOT EXISTS friend_requests (
    id UUID NOT NULL DEFAULT uuid_generate_v4() PRIMARY KEY,
    "from" UUID NOT NULL,
    "to" UUID NOT NULL,
    status VARCHAR NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);