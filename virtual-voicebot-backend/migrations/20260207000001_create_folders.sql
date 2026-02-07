CREATE TABLE folders (
    id UUID NOT NULL PRIMARY KEY,
    parent_id UUID REFERENCES folders(id) ON DELETE CASCADE,
    entity_type VARCHAR(30) NOT NULL,
    name VARCHAR(100) NOT NULL,
    description TEXT,
    sort_order INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT chk_folder_entity_type
        CHECK (entity_type IN (
            'phone_number', 'routing_rule', 'ivr_flow',
            'schedule', 'announcement'
        ))
);

CREATE INDEX idx_folders_parent ON folders(parent_id);
CREATE INDEX idx_folders_entity_type ON folders(entity_type);
CREATE UNIQUE INDEX uq_folders_parent_name
    ON folders(COALESCE(parent_id, '00000000-0000-0000-0000-000000000000'), entity_type, name);
