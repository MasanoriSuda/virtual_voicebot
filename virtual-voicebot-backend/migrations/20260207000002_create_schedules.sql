CREATE TABLE schedules (
    id UUID NOT NULL PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    description TEXT,
    schedule_type VARCHAR(20) NOT NULL DEFAULT 'business',
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    folder_id UUID REFERENCES folders(id) ON DELETE SET NULL,
    date_range_start DATE,
    date_range_end DATE,
    action_type VARCHAR(20) NOT NULL,
    action_target UUID,
    -- Intentionally no FK for polymorphic reference.
    -- route -> routing_rules.id, announcement -> announcements.id.
    action_code VARCHAR(2),
    version INT NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT chk_schedule_type
        CHECK (schedule_type IN ('business', 'holiday', 'special', 'override')),
    CONSTRAINT chk_schedule_action_type
        CHECK (action_type IN ('route', 'voicemail', 'announcement', 'closed')),
    CONSTRAINT chk_schedule_action_code
        CHECK (
            action_code IS NULL
            OR action_code IN ('VB','VR','NR','RJ','BZ','AN','AR','VM','IV')
        )
);

CREATE INDEX idx_schedules_active ON schedules(is_active) WHERE is_active;
CREATE INDEX idx_schedules_folder ON schedules(folder_id);
