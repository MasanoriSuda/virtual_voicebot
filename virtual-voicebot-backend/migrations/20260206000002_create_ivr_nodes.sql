CREATE TABLE ivr_nodes (
    id UUID NOT NULL PRIMARY KEY,
    flow_id UUID NOT NULL REFERENCES ivr_flows(id) ON DELETE CASCADE,
    parent_id UUID REFERENCES ivr_nodes(id) ON DELETE CASCADE,
    node_type VARCHAR(20) NOT NULL,
    action_code VARCHAR(2),
    audio_file_url TEXT,
    tts_text TEXT,
    timeout_sec INT NOT NULL DEFAULT 10,
    max_retries INT NOT NULL DEFAULT 3,
    depth SMALLINT NOT NULL DEFAULT 0,
    exit_action VARCHAR(2) NOT NULL DEFAULT 'IE',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT chk_node_type
        CHECK (node_type IN ('ANNOUNCE', 'KEYPAD', 'FORWARD', 'TRANSFER', 'RECORD', 'EXIT')),
    CONSTRAINT chk_node_depth
        CHECK (depth >= 0 AND depth <= 3),
    CONSTRAINT chk_node_exit_action
        CHECK (exit_action IN ('VB','VR','NR','RJ','BZ','AN','AR','VM','IV',
                               'IA','IR','IK','IW','IF','IT','IB','IE'))
);

CREATE INDEX idx_ivr_nodes_flow ON ivr_nodes(flow_id);
CREATE INDEX idx_ivr_nodes_flow_parent ON ivr_nodes(flow_id, parent_id);
