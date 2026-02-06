CREATE TABLE sync_outbox (
    id BIGSERIAL PRIMARY KEY,
    entity_type VARCHAR(30) NOT NULL,
    entity_id UUID NOT NULL,
    payload JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    processed_at TIMESTAMPTZ
);

CREATE INDEX idx_outbox_pending
    ON sync_outbox(created_at) WHERE processed_at IS NULL;
