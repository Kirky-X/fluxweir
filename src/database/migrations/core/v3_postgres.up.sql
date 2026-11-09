
CREATE TABLE IF NOT EXISTS vault_keys (
    id TEXT PRIMARY KEY,
    vendor_id TEXT NOT NULL,
    label TEXT,
    payload BYTEA NOT NULL,
    mask TEXT NOT NULL,
    fingerprint TEXT NOT NULL,
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_vault_keys_vendor_fingerprint ON vault_keys (vendor_id, fingerprint);
CREATE INDEX IF NOT EXISTS idx_vault_keys_vendor ON vault_keys (vendor_id);
