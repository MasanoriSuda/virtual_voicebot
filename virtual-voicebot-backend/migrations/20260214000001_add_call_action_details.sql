ALTER TABLE call_logs
    ADD COLUMN call_disposition VARCHAR(20) NOT NULL DEFAULT 'allowed',
    ADD COLUMN final_action VARCHAR(50),
    ADD COLUMN transfer_status VARCHAR(20) NOT NULL DEFAULT 'no_transfer',
    ADD COLUMN transfer_started_at TIMESTAMPTZ,
    ADD COLUMN transfer_answered_at TIMESTAMPTZ,
    ADD COLUMN transfer_ended_at TIMESTAMPTZ;

ALTER TABLE call_logs
    ADD CONSTRAINT chk_call_disposition
        CHECK (call_disposition IN ('allowed', 'denied', 'no_answer'));

ALTER TABLE call_logs
    ADD CONSTRAINT chk_call_final_action
        CHECK (
            final_action IS NULL OR final_action IN (
                'normal_call',
                'voicemail',
                'voicebot',
                'ivr',
                'announcement',
                'busy',
                'rejected',
                'announcement_deny'
            )
        );

ALTER TABLE call_logs
    ADD CONSTRAINT chk_call_transfer_status
        CHECK (transfer_status IN ('no_transfer', 'none', 'trying', 'answered', 'failed'));

CREATE INDEX idx_call_logs_disposition
    ON call_logs(call_disposition, started_at DESC);

CREATE INDEX idx_call_logs_transfer_status
    ON call_logs(transfer_status, started_at DESC);
