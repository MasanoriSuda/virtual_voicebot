-- Issue #138: 着信アクションルール（番号グループ評価用）テーブル作成
-- RD-004 FR-1.1: 段階2「番号グループ評価」で使用
-- Frontend IncomingRule を Backend DB に同期

CREATE TABLE call_action_rules (
    id UUID NOT NULL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    caller_group_id UUID,
    action_type VARCHAR(20) NOT NULL,
    action_config JSONB NOT NULL,
    priority INT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT chk_call_action_rules_action_type
        CHECK (action_type IN ('allow', 'deny')),
    CONSTRAINT chk_call_action_rules_priority
        CHECK (priority >= 0)
);

-- priority でソート（ルール評価順序）
CREATE INDEX idx_call_action_rules_priority
    ON call_action_rules(priority) WHERE is_active = TRUE;

-- caller_group_id で検索（番号グループに紐づくルール取得）
CREATE INDEX idx_call_action_rules_group_id
    ON call_action_rules(caller_group_id) WHERE caller_group_id IS NOT NULL AND is_active = TRUE;

COMMENT ON TABLE call_action_rules IS 'Frontend IncomingRule（着信アクションルール）。番号グループに対するアクション設定';
COMMENT ON COLUMN call_action_rules.caller_group_id IS 'registered_numbers.group_id を参照（FK なし、削除済みグループ対応）';
COMMENT ON COLUMN call_action_rules.action_config IS 'ActionCode + 詳細設定（JSONB）。例: {"actionCode":"BZ"} or {"actionCode":"IV","ivrFlowId":"uuid-v7"}';
COMMENT ON COLUMN call_action_rules.priority IS '評価優先順位（小さいほど優先、Frontend 配列順）';
