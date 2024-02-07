-- Add migration script here
ALTER TABLE users ADD COLUMN avatar VARCHAR;
ALTER TABLE users ADD CONSTRAINT fk_avatar FOREIGN KEY (avatar) REFERENCES uploads(id);
