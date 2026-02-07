ALTER TABLE spam_numbers
    ADD COLUMN folder_id UUID REFERENCES folders(id) ON DELETE SET NULL;

ALTER TABLE registered_numbers
    ADD COLUMN folder_id UUID REFERENCES folders(id) ON DELETE SET NULL;

ALTER TABLE routing_rules
    ADD COLUMN folder_id UUID REFERENCES folders(id) ON DELETE SET NULL;

ALTER TABLE ivr_flows
    ADD COLUMN folder_id UUID REFERENCES folders(id) ON DELETE SET NULL;
