
ALTER TABLE vault_keys
    ADD COLUMN version INTEGER NOT NULL DEFAULT 1,
    ADD COLUMN superseded_at_ms BIGINT NULL;
