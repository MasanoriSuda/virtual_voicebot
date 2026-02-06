CREATE TABLE recordings (
    id UUID NOT NULL PRIMARY KEY,
    call_log_id UUID NOT NULL REFERENCES call_log_index(id) ON DELETE CASCADE,
    recording_type VARCHAR(20) NOT NULL DEFAULT 'full_call',
    sequence_number SMALLINT NOT NULL DEFAULT 1,
    file_path TEXT NOT NULL,
    s3_url TEXT,
    upload_status VARCHAR(20) NOT NULL DEFAULT 'local_only',
    duration_sec INT,
    format VARCHAR(10) NOT NULL DEFAULT 'wav',
    file_size_bytes BIGINT,
    started_at TIMESTAMPTZ NOT NULL,
    ended_at TIMESTAMPTZ,
    synced_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT chk_recording_type
        CHECK (recording_type IN ('full_call', 'ivr_segment', 'voicemail', 'transfer', 'one_way')),
    CONSTRAINT chk_upload_status
        CHECK (upload_status IN ('local_only', 'uploading', 'uploaded', 'upload_failed')),
    CONSTRAINT chk_recording_format
        CHECK (format IN ('wav', 'mp3'))
);

CREATE INDEX idx_recordings_call_log_id ON recordings(call_log_id);
CREATE INDEX idx_recordings_synced ON recordings(synced_at) WHERE synced_at IS NULL;
CREATE INDEX idx_recordings_upload ON recordings(upload_status) WHERE upload_status != 'uploaded';
