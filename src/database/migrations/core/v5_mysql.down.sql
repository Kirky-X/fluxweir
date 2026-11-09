
ALTER TABLE request_logs DROP INDEX idx_request_logs_session;
ALTER TABLE request_logs DROP COLUMN session_key;
ALTER TABLE request_logs DROP COLUMN context_window_tokens;
ALTER TABLE request_logs DROP COLUMN cache_read_tokens;
ALTER TABLE request_logs DROP COLUMN cache_write_tokens;
