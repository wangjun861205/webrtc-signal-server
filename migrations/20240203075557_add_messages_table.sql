-- Add migration script here
CREATE TABLE IF NOT EXISTS messages (
    id UUID NOT NULL DEFAULT uuid_generate_v4(),
    "from" UUID NOT NULL,
    "to" UUID NOT NULL,
    sent_at TIMESTAMP WITH TIME ZONE  NOT NULL,
    message TEXT NOT NULL,
    PRIMARY KEY (id)
);
