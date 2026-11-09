
CREATE TABLE IF NOT EXISTS vault_keys (
    id VARCHAR(64) PRIMARY KEY,
    vendor_id VARCHAR(255) NOT NULL,
    label VARCHAR(255),
    payload VARBINARY(8192) NOT NULL,
    mask VARCHAR(64) NOT NULL,
    fingerprint VARCHAR(16) NOT NULL,
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL
);
CREATE UNIQUE INDEX idx_vault_keys_vendor_fingerprint ON vault_keys (vendor_id, fingerprint);
CREATE INDEX idx_vault_keys_vendor ON vault_keys (vendor_id);
