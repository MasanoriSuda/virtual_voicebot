CREATE TABLE system_settings (
    id INT NOT NULL PRIMARY KEY DEFAULT 1,
    recording_retention_days INT NOT NULL DEFAULT 90,
    history_retention_days INT NOT NULL DEFAULT 365,
    sync_endpoint_url TEXT,
    default_action_code VARCHAR(2) NOT NULL DEFAULT 'IV',
    max_concurrent_calls INT NOT NULL DEFAULT 2,
    extra JSONB NOT NULL DEFAULT '{}',
    version INT NOT NULL DEFAULT 1,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT chk_single_row CHECK (id = 1),
    CONSTRAINT chk_retention_positive
        CHECK (recording_retention_days > 0 AND history_retention_days > 0),
    CONSTRAINT chk_settings_action_code
        CHECK (default_action_code IN ('VB','VR','NR','RJ','BZ','AN','AR','VM','IV'))
);
