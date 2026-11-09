
ALTER TABLE token_usage ADD COLUMN api_key_hash TEXT NOT NULL DEFAULT '', ADD COLUMN input_tokens BIGINT DEFAULT 0, ADD COLUMN output_tokens BIGINT DEFAULT 0, ADD COLUMN cache_read_tokens BIGINT DEFAULT 0, ADD COLUMN cache_creation_tokens BIGINT DEFAULT 0, ADD COLUMN input_cost DOUBLE PRECISION DEFAULT 0.0, ADD COLUMN output_cost DOUBLE PRECISION DEFAULT 0.0, ADD COLUMN cache_read_cost DOUBLE PRECISION DEFAULT 0.0, ADD COLUMN cache_creation_cost DOUBLE PRECISION DEFAULT 0.0;
UPDATE token_usage SET api_key_hash = '', input_tokens = prompt_tokens, output_tokens = completion_tokens, input_cost = cost_usd_sum;
ALTER TABLE token_usage DROP COLUMN total_tokens, DROP COLUMN prompt_tokens, DROP COLUMN completion_tokens, DROP COLUMN cost_usd_sum;
ALTER TABLE token_usage DROP CONSTRAINT token_usage_vendor_model_date_hour_key;
ALTER TABLE token_usage ADD CONSTRAINT token_usage_vendor_model_api_key_hash_date_hour_key UNIQUE (vendor, model, api_key_hash, date, hour);
CREATE INDEX IF NOT EXISTS idx_token_usage_date ON token_usage(date);
CREATE INDEX IF NOT EXISTS idx_token_usage_key_date ON token_usage(api_key_hash, date);
