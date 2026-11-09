
CREATE TABLE IF NOT EXISTS affinity_sticks (
    tenant TEXT NOT NULL DEFAULT '',
    session_key TEXT NOT NULL,
    vendor_id TEXT NOT NULL,
    worth_tokens BIGINT NOT NULL DEFAULT 0,
    expires_at BIGINT NOT NULL,
    PRIMARY KEY (tenant, session_key)
);
CREATE INDEX IF NOT EXISTS idx_affinity_sticks_expires ON affinity_sticks(expires_at);
