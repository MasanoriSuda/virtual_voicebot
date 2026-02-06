CREATE TABLE registered_numbers (
    id UUID NOT NULL PRIMARY KEY,
    phone_number VARCHAR(20) NOT NULL,
    name VARCHAR(100),
    category VARCHAR(50) NOT NULL DEFAULT 'general',
    action_code VARCHAR(2) NOT NULL DEFAULT 'VR',
    ivr_flow_id UUID REFERENCES ivr_flows(id) ON DELETE SET NULL,
    recording_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    announce_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    notes TEXT,
    version INT NOT NULL DEFAULT 1,
    deleted_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT chk_registered_phone_e164
        CHECK (phone_number ~ '^\+[1-9][0-9]{1,14}$'),
    CONSTRAINT chk_registered_category
        CHECK (category IN ('vip', 'customer', 'partner', 'general')),
    CONSTRAINT chk_registered_action_code
        CHECK (action_code IN ('VB','VR','NR','RJ','BZ','AN','AR','VM','IV'))
);

CREATE UNIQUE INDEX uq_registered_numbers_phone
    ON registered_numbers(phone_number) WHERE deleted_at IS NULL;
