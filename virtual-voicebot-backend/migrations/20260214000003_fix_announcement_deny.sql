UPDATE call_logs
SET final_action = 'announcement_deny'
WHERE final_action = 'announcement_reject';

ALTER TABLE call_logs
    DROP CONSTRAINT IF EXISTS chk_call_final_action;

ALTER TABLE call_logs
    ADD CONSTRAINT chk_call_final_action
        CHECK (
            final_action IS NULL OR final_action IN (
                'normal_call',
                'voicemail',
                'voicebot',
                'ivr',
                'announcement',
                'busy',
                'rejected',
                'announcement_deny'
            )
        );
