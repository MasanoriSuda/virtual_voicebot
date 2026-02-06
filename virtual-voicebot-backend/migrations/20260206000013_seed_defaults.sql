INSERT INTO routing_rules (id, caller_category, action_code, priority, is_active) VALUES
    ('019503a0-0000-7000-8000-000000000001', 'spam',       'RJ', 0, TRUE),
    ('019503a0-0000-7000-8000-000000000002', 'registered', 'VR', 0, TRUE),
    ('019503a0-0000-7000-8000-000000000003', 'unknown',    'IV', 0, TRUE),
    ('019503a0-0000-7000-8000-000000000004', 'anonymous',  'IV', 0, TRUE);

INSERT INTO system_settings (id) VALUES (1);
