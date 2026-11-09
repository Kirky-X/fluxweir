
CREATE TABLE IF NOT EXISTS response_store (
    id TEXT PRIMARY KEY,
    tenant_hash TEXT NOT NULL,
    conversation_id TEXT,
    parent_response_id TEXT,
    model TEXT NOT NULL,
    status TEXT NOT NULL,
    store_flag BIGINT NOT NULL DEFAULT 1,
    background BIGINT NOT NULL DEFAULT 0,
    created_at BIGINT NOT NULL,
    expires_at BIGINT NOT NULL,
    input_items TEXT NOT NULL DEFAULT '[]',
    output_items TEXT NOT NULL DEFAULT '[]',
    response_json TEXT NOT NULL DEFAULT '{}'
);
CREATE INDEX IF NOT EXISTS idx_response_store_tenant ON response_store(tenant_hash, created_at);
CREATE INDEX IF NOT EXISTS idx_response_store_conversation ON response_store(conversation_id, created_at);
CREATE INDEX IF NOT EXISTS idx_response_store_expires ON response_store(expires_at);
