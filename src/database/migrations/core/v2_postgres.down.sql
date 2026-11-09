
ALTER TABLE token_usage DROP CONSTRAINT token_usage_vendor_model_api_key_hash_date_hour_key;
ALTER TABLE token_usage ADD CONSTRAINT token_usage_vendor_model_date_hour_key UNIQUE (vendor, model, date, hour);
ALTER TABLE token_usage ADD COLUMN total_tokens BIGINT DEFAULT 0, ADD COLUMN prompt_tokens BIGINT DEFAULT 0, ADD COLUMN completion_tokens BIGINT DEFAULT 0, ADD COLUMN cost_usd_sum DOUBLE PRECISION DEFAULT 0.0;
UPDATE token_usage SET prompt_tokens = input_tokens, completion_tokens = output_tokens, total_tokens = input_tokens + output_tokens + cache_read_tokens + cache_creation_tokens, cost_usd_sum = input_cost + output_cost + cache_read_cost + cache_creation_cost;
ALTER TABLE token_usage DROP COLUMN api_key_hash, DROP COLUMN input_tokens, DROP COLUMN output_tokens, DROP COLUMN cache_read_tokens, DROP COLUMN cache_creation_tokens, DROP COLUMN input_cost, DROP COLUMN output_cost, DROP COLUMN cache_read_cost, DROP COLUMN cache_creation_cost;
DROP INDEX IF EXISTS idx_token_usage_date;
DROP INDEX IF EXISTS idx_token_usage_key_date;
