CREATE TABLE routing_rules (
    id UUID NOT NULL PRIMARY KEY,
    caller_category VARCHAR(20) NOT NULL,
    action_code VARCHAR(2) NOT NULL,
    ivr_flow_id UUID REFERENCES ivr_flows(id) ON DELETE SET NULL,
    priority INT NOT NULL DEFAULT 0,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    version INT NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT chk_routing_caller_category
        CHECK (caller_category IN ('spam', 'registered', 'unknown', 'anonymous')),
    CONSTRAINT chk_routing_action_code
        CHECK (action_code IN ('VB','VR','NR','RJ','BZ','AN','AR','VM','IV'))
);

CREATE INDEX idx_routing_rules_category
    ON routing_rules(caller_category, priority) WHERE is_active;
