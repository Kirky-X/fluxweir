
CREATE TABLE IF NOT EXISTS quota_ledger (
    api_key_hash TEXT NOT NULL,
    window_id TEXT NOT NULL,
    window_start INTEGER NOT NULL,
    tokens_used INTEGER NOT NULL DEFAULT 0,
    cost_used_millicents INTEGER NOT NULL DEFAULT 0,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (api_key_hash, window_id, window_start)
);
