
CREATE INDEX IF NOT EXISTS idx_request_logs_ts_id ON request_logs(timestamp, id);
