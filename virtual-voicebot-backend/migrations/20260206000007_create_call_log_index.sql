CREATE TABLE call_log_index (
    id UUID NOT NULL PRIMARY KEY,
    started_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_call_log_index_started_at ON call_log_index(started_at);
