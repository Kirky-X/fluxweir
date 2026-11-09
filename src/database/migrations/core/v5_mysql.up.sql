
ALTER TABLE request_logs ADD COLUMN session_key VARCHAR(255);
ALTER TABLE request_logs ADD COLUMN context_window_tokens BIGINT;
ALTER TABLE request_logs ADD COLUMN cache_read_tokens BIGINT;
ALTER TABLE request_logs ADD COLUMN cache_write_tokens BIGINT;
