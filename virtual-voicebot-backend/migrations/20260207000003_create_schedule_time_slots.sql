CREATE TABLE schedule_time_slots (
    id UUID NOT NULL PRIMARY KEY,
    schedule_id UUID NOT NULL REFERENCES schedules(id) ON DELETE CASCADE,
    day_of_week SMALLINT,
    start_time TIME NOT NULL,
    end_time TIME NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT chk_day_of_week
        CHECK (day_of_week IS NULL OR (day_of_week >= 0 AND day_of_week <= 6)),
    CONSTRAINT chk_time_order
        CHECK (start_time < end_time)
);

CREATE INDEX idx_schedule_time_slots_schedule ON schedule_time_slots(schedule_id);
CREATE INDEX idx_schedule_time_slots_dow ON schedule_time_slots(day_of_week);
