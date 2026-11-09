
ALTER TABLE token_usage RENAME TO token_usage_v1;
CREATE TABLE IF NOT EXISTS token_usage (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    vendor TEXT NOT NULL,
    model TEXT NOT NULL,
    api_key_hash TEXT NOT NULL DEFAULT '',
    date TEXT NOT NULL,
    hour INTEGER NOT NULL,
    request_count INTEGER DEFAULT 0,
    input_tokens INTEGER DEFAULT 0,
    output_tokens INTEGER DEFAULT 0,
    cache_read_tokens INTEGER DEFAULT 0,
    cache_creation_tokens INTEGER DEFAULT 0,
    input_cost REAL DEFAULT 0.0,
    output_cost REAL DEFAULT 0.0,
    cache_read_cost REAL DEFAULT 0.0,
    cache_creation_cost REAL DEFAULT 0.0,
    UNIQUE(vendor, model, api_key_hash, date, hour)
);
INSERT INTO token_usage (vendor, model, api_key_hash, date, hour, request_count, input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens, input_cost, output_cost, cache_read_cost, cache_creation_cost)
SELECT vendor, model, '', date, hour, request_count, prompt_tokens, completion_tokens, 0, 0, cost_usd_sum, 0.0, 0.0, 0.0 FROM token_usage_v1;
DROP TABLE token_usage_v1;
CREATE INDEX IF NOT EXISTS idx_token_usage_date ON token_usage(date);
CREATE INDEX IF NOT EXISTS idx_token_usage_key_date ON token_usage(api_key_hash, date);
