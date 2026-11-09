
ALTER TABLE token_usage DROP INDEX uk_token_usage_v2, ADD UNIQUE KEY vendor (vendor, model, date, hour), MODIFY date VARCHAR(255) NOT NULL;
ALTER TABLE token_usage ADD COLUMN total_tokens BIGINT DEFAULT 0, ADD COLUMN prompt_tokens BIGINT DEFAULT 0, ADD COLUMN completion_tokens BIGINT DEFAULT 0, ADD COLUMN cost_usd_sum DOUBLE DEFAULT 0.0;
UPDATE token_usage SET prompt_tokens = input_tokens, completion_tokens = output_tokens, total_tokens = input_tokens + output_tokens + cache_read_tokens + cache_creation_tokens, cost_usd_sum = input_cost + output_cost + cache_read_cost + cache_creation_cost;
ALTER TABLE token_usage DROP COLUMN api_key_hash, DROP COLUMN input_tokens, DROP COLUMN output_tokens, DROP COLUMN cache_read_tokens, DROP COLUMN cache_creation_tokens, DROP COLUMN input_cost, DROP COLUMN output_cost, DROP COLUMN cache_read_cost, DROP COLUMN cache_creation_cost;
