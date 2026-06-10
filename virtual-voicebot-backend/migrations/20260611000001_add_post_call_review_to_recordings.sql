ALTER TABLE recordings
  ADD COLUMN transcript_json JSONB,
  ADD COLUMN summary_text TEXT,
  ADD COLUMN review_json JSONB,
  ADD COLUMN review_status VARCHAR(20) NOT NULL DEFAULT 'pending',
  ADD COLUMN review_error TEXT,
  ADD COLUMN reviewed_at TIMESTAMPTZ;

ALTER TABLE recordings
  ADD CONSTRAINT chk_recording_review_status
  CHECK (review_status IN ('pending', 'processing', 'completed', 'failed', 'skipped'));

CREATE INDEX idx_recordings_review_pending
  ON recordings(created_at)
  WHERE review_status IN ('pending', 'failed') AND upload_status = 'uploaded';
