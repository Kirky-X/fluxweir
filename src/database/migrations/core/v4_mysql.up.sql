
CREATE TABLE IF NOT EXISTS response_store (
    id VARCHAR(64) PRIMARY KEY,
    tenant_hash VARCHAR(64) NOT NULL,
    conversation_id VARCHAR(128),
    parent_response_id VARCHAR(64),
    model VARCHAR(255) NOT NULL,
    status VARCHAR(24) NOT NULL,
    store_flag BIGINT NOT NULL DEFAULT 1,
    background BIGINT NOT NULL DEFAULT 0,
    created_at BIGINT NOT NULL,
    expires_at BIGINT NOT NULL,
    input_items LONGTEXT NOT NULL,
    output_items LONGTEXT NOT NULL,
    response_json LONGTEXT NOT NULL
) ENGINE = InnoDB;
CREATE INDEX idx_response_store_tenant ON response_store(tenant_hash, created_at);
CREATE INDEX idx_response_store_conversation ON response_store(conversation_id, created_at);
CREATE INDEX idx_response_store_expires ON response_store(expires_at);
