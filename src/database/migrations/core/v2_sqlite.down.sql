
ALTER TABLE token_usage RENAME TO token_usage_v2;
CREATE TABLE IF NOT EXISTS token_usage (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    vendor TEXT NOT NULL,
    model TEXT NOT NULL,
    date TEXT NOT NULL,
    hour INTEGER NOT NULL,
    request_count INTEGER DEFAULT 0,
    total_tokens INTEGER DEFAULT 0,
    prompt_tokens INTEGER DEFAULT 0,
    completion_tokens INTEGER DEFAULT 0,
    cost_usd_sum REAL DEFAULT 0.0,
    UNIQUE(vendor, model, date, hour)
);
INSERT INTO token_usage (vendor, model, date, hour, request_count, total_tokens, prompt_tokens, completion_tokens, cost_usd_sum)
SELECT vendor, model, date, hour, request_count, input_tokens + output_tokens + cache_read_tokens + cache_creation_tokens, input_tokens, output_tokens, input_cost + output_cost + cache_read_cost + cache_creation_cost FROM token_usage_v2;
DROP TABLE token_usage_v2;
