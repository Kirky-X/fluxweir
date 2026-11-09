
CREATE TABLE IF NOT EXISTS affinity_sticks (
    tenant VARCHAR(64) NOT NULL DEFAULT '',
    session_key VARCHAR(255) NOT NULL,
    vendor_id VARCHAR(64) NOT NULL,
    worth_tokens BIGINT NOT NULL DEFAULT 0,
    expires_at BIGINT NOT NULL,
    PRIMARY KEY (tenant, session_key)
) ENGINE = InnoDB;
