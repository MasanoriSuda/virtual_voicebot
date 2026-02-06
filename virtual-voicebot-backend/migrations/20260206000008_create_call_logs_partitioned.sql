CREATE TABLE call_logs (
    id UUID NOT NULL REFERENCES call_log_index(id),
    started_at TIMESTAMPTZ NOT NULL,

    PRIMARY KEY (id, started_at),

    external_call_id VARCHAR(64) NOT NULL,
    sip_call_id VARCHAR(255),
    caller_number VARCHAR(20),
    caller_category VARCHAR(20) NOT NULL DEFAULT 'unknown',
    action_code VARCHAR(2) NOT NULL,
    -- Intentionally no FK to ivr_flows(id):
    -- call_logs is an immutable audit/history table, and flow deletion/changes
    -- must not block or rewrite historical call records.
    ivr_flow_id UUID,
    answered_at TIMESTAMPTZ,
    ended_at TIMESTAMPTZ,
    duration_sec INT,
    end_reason VARCHAR(20) NOT NULL DEFAULT 'normal',
    version INT NOT NULL DEFAULT 1,
    synced_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT chk_call_caller_e164
        CHECK (caller_number IS NULL OR caller_number ~ '^\+[1-9][0-9]{1,14}$'),
    CONSTRAINT chk_call_category
        CHECK (caller_category IN ('spam', 'registered', 'unknown', 'anonymous')),
    CONSTRAINT chk_call_action_code
        CHECK (action_code IN ('VB','VR','NR','RJ','BZ','AN','AR','VM','IV')),
    CONSTRAINT chk_call_end_reason
        CHECK (end_reason IN ('normal', 'cancelled', 'rejected', 'timeout', 'error'))
) PARTITION BY RANGE (started_at);

CREATE UNIQUE INDEX uq_call_logs_external_id
    ON call_logs(external_call_id, started_at);

CREATE INDEX idx_call_logs_caller
    ON call_logs(caller_number, started_at DESC);

CREATE INDEX idx_call_logs_synced
    ON call_logs(started_at) WHERE synced_at IS NULL;
