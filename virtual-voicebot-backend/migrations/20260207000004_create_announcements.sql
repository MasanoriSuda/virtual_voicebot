CREATE TABLE announcements (
    id UUID NOT NULL PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    description TEXT,
    announcement_type VARCHAR(20) NOT NULL DEFAULT 'custom',
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    folder_id UUID REFERENCES folders(id) ON DELETE SET NULL,
    audio_file_url TEXT,
    tts_text TEXT,
    duration_sec INT,
    language VARCHAR(10) NOT NULL DEFAULT 'ja',
    version INT NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT chk_announcement_type
        CHECK (announcement_type IN (
            'greeting', 'hold', 'ivr', 'closed', 'recording_notice', 'custom'
        )),
    CONSTRAINT chk_announcement_has_source
        CHECK (audio_file_url IS NOT NULL OR tts_text IS NOT NULL)
);

CREATE INDEX idx_announcements_type ON announcements(announcement_type);
CREATE INDEX idx_announcements_folder ON announcements(folder_id);
