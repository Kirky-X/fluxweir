
CREATE TABLE IF NOT EXISTS request_logs (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    request_id VARCHAR(255) UNIQUE NOT NULL,
    timestamp BIGINT NOT NULL,
    endpoint TEXT NOT NULL,
    input_format TEXT NOT NULL,
    abstract_model TEXT,
    messages TEXT NOT NULL,
    actual_vendor VARCHAR(255) NOT NULL,
    actual_model VARCHAR(255) NOT NULL,
    routing_strategy TEXT NOT NULL,
    status_code BIGINT,
    response TEXT,
    error_message TEXT,
    prompt_tokens BIGINT,
    completion_tokens BIGINT,
    total_tokens BIGINT,
    duration_ms BIGINT,
    cost_usd DOUBLE DEFAULT 0.0
) ENGINE = InnoDB;
CREATE TABLE IF NOT EXISTS token_usage (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    vendor VARCHAR(255) NOT NULL,
    model VARCHAR(255) NOT NULL,
    date VARCHAR(255) NOT NULL,
    hour INTEGER NOT NULL,
    request_count BIGINT DEFAULT 0,
    total_tokens BIGINT DEFAULT 0,
    prompt_tokens BIGINT DEFAULT 0,
    completion_tokens BIGINT DEFAULT 0,
    cost_usd_sum DOUBLE DEFAULT 0.0,
    UNIQUE(vendor, model, date, hour)
) ENGINE = InnoDB;
CREATE TABLE IF NOT EXISTS vendor_status (
    vendor VARCHAR(255) PRIMARY KEY,
    status TEXT NOT NULL,
    last_updated BIGINT NOT NULL,
    window_used_tokens BIGINT DEFAULT 0,
    window_reset_at BIGINT,
    error_count BIGINT DEFAULT 0
) ENGINE = InnoDB;
CREATE TABLE IF NOT EXISTS model_stats (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    model VARCHAR(255) NOT NULL,
    vendor VARCHAR(255) NOT NULL,
    date VARCHAR(255) NOT NULL,
    request_count BIGINT DEFAULT 0,
    success_count BIGINT DEFAULT 0,
    error_count BIGINT DEFAULT 0,
    avg_duration_ms DOUBLE,
    UNIQUE(model, vendor, date)
) ENGINE = InnoDB;
