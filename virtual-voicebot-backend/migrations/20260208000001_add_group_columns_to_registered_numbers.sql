-- Issue #138: registered_numbers に番号グループ情報を追加
-- RD-004 FR-1.4: Frontend CallerGroup との対応

-- group_id: 番号グループの不変ID（Frontend CallerGroup.id）
-- group_name: 番号グループの表示名（Frontend CallerGroup.name）
ALTER TABLE registered_numbers
    ADD COLUMN group_id UUID,
    ADD COLUMN group_name VARCHAR(255);

-- group_id にインデックスを作成（call_action_rules との照合用）
CREATE INDEX idx_registered_numbers_group_id
    ON registered_numbers(group_id) WHERE group_id IS NOT NULL;

COMMENT ON COLUMN registered_numbers.group_id IS 'Frontend CallerGroup の不変ID（UUID）。FK なし（削除済みグループ対応）';
COMMENT ON COLUMN registered_numbers.group_name IS 'Frontend CallerGroup の表示名。リネーム時に更新される';
