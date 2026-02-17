ALTER TABLE call_logs
    ADD COLUMN status VARCHAR(20) NOT NULL DEFAULT 'ringing';

ALTER TABLE call_logs
    ADD CONSTRAINT chk_call_status
        CHECK (status IN ('ringing', 'in_call', 'ended', 'error'));

CREATE INDEX idx_call_logs_status
    ON call_logs(status, started_at DESC);
