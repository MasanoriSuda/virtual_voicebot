CREATE TABLE spam_numbers (
    id UUID NOT NULL PRIMARY KEY,
    phone_number VARCHAR(20) NOT NULL,
    reason VARCHAR(255),
    source VARCHAR(50) NOT NULL DEFAULT 'manual',
    deleted_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT chk_spam_phone_e164
        CHECK (phone_number ~ '^\+[1-9][0-9]{1,14}$'),
    CONSTRAINT chk_spam_source
        CHECK (source IN ('manual', 'import', 'report'))
);

CREATE UNIQUE INDEX uq_spam_numbers_phone
    ON spam_numbers(phone_number) WHERE deleted_at IS NULL;
