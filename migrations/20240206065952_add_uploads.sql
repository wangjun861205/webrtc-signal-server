-- Add migration script here
CREATE TABLE IF NOT EXISTS uploads (
    id UUID NOT NULL DEFAULT uuid_generate_v4() PRIMARY KEY,
    filename VARCHAR NOT NULL,
    mime_type VARCHAR NOT NULL,
    uploader_id VARCHAR NOT NULL,
    uploaded_at TIMESTAMP WITH TIME ZONE NOT NULL
);
