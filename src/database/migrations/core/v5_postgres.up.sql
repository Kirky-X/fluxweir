
ALTER TABLE request_logs ADD COLUMN session_key TEXT;
ALTER TABLE request_logs ADD COLUMN context_window_tokens BIGINT;
ALTER TABLE request_logs ADD COLUMN cache_read_tokens BIGINT;
ALTER TABLE request_logs ADD COLUMN cache_write_tokens BIGINT;
CREATE INDEX IF NOT EXISTS idx_request_logs_session ON request_logs(session_key, timestamp);
