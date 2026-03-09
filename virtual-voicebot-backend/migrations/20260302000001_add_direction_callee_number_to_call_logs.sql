ALTER TABLE call_logs
    ADD COLUMN direction TEXT NOT NULL DEFAULT 'inbound'
        CHECK (direction IN ('inbound', 'outbound')),
    ADD COLUMN callee_number TEXT;
