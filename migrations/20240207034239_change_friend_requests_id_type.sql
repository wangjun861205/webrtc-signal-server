-- Add migration script here
ALTER TABLE friend_requests ALTER COLUMN id TYPE VARCHAR;
ALTER TABLE friend_requests ALTER COLUMN id DROP DEFAULT;
ALTER TABLE friend_requests ALTER COLUMN "from" TYPE VARCHAR;
ALTER TABLE friend_requests ALTER COLUMN "from" DROP DEFAULT;
ALTER TABLE friend_requests ALTER COLUMN "to" TYPE VARCHAR;
ALTER TABLE friend_requests ALTER COLUMN "to" DROP DEFAULT;
