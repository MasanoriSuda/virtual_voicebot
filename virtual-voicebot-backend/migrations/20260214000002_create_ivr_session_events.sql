CREATE TABLE ivr_session_events (
    id UUID PRIMARY KEY,
    call_log_id UUID NOT NULL,
    sequence INT NOT NULL CHECK (sequence >= 0),
    event_type VARCHAR(20) NOT NULL CHECK (
        event_type IN (
            'node_enter',
            'dtmf_input',
            'transition',
            'timeout',
            'invalid_input',
            'exit'
        )
    ),
    occurred_at TIMESTAMPTZ NOT NULL,
    node_id UUID,
    dtmf_key VARCHAR(1),
    transition_id UUID,
    exit_action VARCHAR(2),
    exit_reason VARCHAR(50),
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT fk_ivr_session_events_call_log
        FOREIGN KEY (call_log_id) REFERENCES call_log_index(id)
        ON DELETE CASCADE,
    CONSTRAINT uq_ivr_session_events_call_log_sequence
        UNIQUE (call_log_id, sequence)
);

CREATE INDEX idx_ivr_session_events_occurred_at
    ON ivr_session_events(occurred_at);

CREATE INDEX idx_ivr_session_events_event_type
    ON ivr_session_events(event_type);
