CREATE TABLE ivr_transitions (
    id UUID NOT NULL PRIMARY KEY,
    from_node_id UUID NOT NULL REFERENCES ivr_nodes(id) ON DELETE CASCADE,
    input_type VARCHAR(20) NOT NULL,
    dtmf_key VARCHAR(5),
    to_node_id UUID REFERENCES ivr_nodes(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT chk_transition_input_type
        CHECK (input_type IN ('DTMF', 'TIMEOUT', 'INVALID', 'COMPLETE')),
    CONSTRAINT chk_dtmf_key_required
        CHECK (input_type != 'DTMF' OR dtmf_key IS NOT NULL)
);

CREATE INDEX idx_ivr_transitions_from ON ivr_transitions(from_node_id);
