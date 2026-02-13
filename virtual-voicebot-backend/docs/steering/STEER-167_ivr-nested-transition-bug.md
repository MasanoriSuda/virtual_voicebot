# STEER-167: IVR 次の層への遷移失敗バグ

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-167 |
| タイトル | IVR 次の層への遷移失敗バグ（調査結果：バグではない） |
| ステータス | Closed |
| 関連Issue | #167（Closed） |
| 親ステアリング | - |
| 優先度 | - |
| 作成日 | 2026-02-12 |
| クローズ日 | 2026-02-12 |

---

## 2. ストーリー（Why）

### 2.1 背景

IVR フロー実行中、ユーザーが「次の層へ行く」キーを押下したときに、次の層へ遷移せず「不正な入力」として扱われる不具合が発生している。

**発生条件**:
- IVR フローの1層目でキー（例: "1"）を押下
- Destination の action_code が "IV"（次の IVR フローへ遷移）
- Destination の metadata に `ivrFlowId` が設定されている

**期待動作**:
- 次の層（2層目）の IVR フローに遷移する
- 2層目のメニュー音声が再生される
- 2層目のキー入力を受け付ける

**実動作**:
- 次の層へ遷移せず、「不正な入力」として `handle_db_ivr_retry("INVALID")` が呼ばれる
- 現在の層（1層目）のメニュー音声が再生される
- リトライカウントが増加する

**根本原因**:

**コード箇所**: [handlers/mod.rs:1122-1136](virtual-voicebot-backend/src/protocol/session/handlers/mod.rs#L1122-L1136)

```rust
"IV" => {
    if let Some(next_flow_id) = self.ivr_flow_id {
        if !self.enter_db_ivr_flow(next_flow_id).await {  // ← false が返る
            warn!(
                "[session {}] failed to start nested IVR flow id={}, replaying current menu",
                self.call_id, next_flow_id
            );
            // 失敗したら前の状態に戻して現在のメニューを再生
            self.ivr_flow_id = previous_ivr_flow_id;
            self.ivr_keypad_node_id = previous_ivr_keypad_node_id;
            self.replay_current_ivr_menu().await;
        }
    }
}
```

**`enter_db_ivr_flow` が `false` を返す理由**:

[handlers/mod.rs:894-910](virtual-voicebot-backend/src/protocol/session/handlers/mod.rs#L894-L910)

```rust
async fn enter_db_ivr_flow(&mut self, ivr_flow_id: Uuid) -> bool {
    let menu = match self.routing_port.find_ivr_menu(ivr_flow_id).await {
        Ok(Some(row)) => row,
        Ok(None) => {
            warn!(
                "[session {}] IVR flow not found or inactive id={}",
                self.call_id, ivr_flow_id
            );
            return false;  // ← IVR フローが見つからない、または無効
        }
        Err(err) => {
            warn!(
                "[session {}] failed to read IVR flow id={} error={}",
                self.call_id, ivr_flow_id, err
            );
            return false;  // ← DB クエリエラー
        }
    };
    // ...
}
```

**原因の詳細**:

1. **DB に次の層の IVR フローが存在しない**:
   - Frontend で IVR フロー設定時、destination の `ivrFlowId` が誤って設定されている
   - または、参照先の IVR フローが削除されている

2. **IVR フローが無効（inactive）**:
   - `find_ivr_menu` クエリは `is_active=true` の IVR フローのみ取得する
   - 参照先の IVR フローが無効化されている

3. **DB クエリエラー**:
   - `find_ivr_menu` の SQL クエリが失敗している
   - DB 接続エラー、またはテーブル構造の不整合

**時系列の補足（重要）**:

調査の結果、以下の時系列が明らかになった：

| 時刻 | イベント | 状態 |
|------|---------|------|
| 2026-02-12 14:00 | ログ出力（flows=1） | 1層目のみ存在 |
| 2026-02-12 14:32:47+00 | 2層目フロー作成 | DB に2層目が追加 |
| 現在 | ivr-flows.json 確認 | 2階層構成、参照解決可能 |

**結論**: 当時の失敗は「**まだ次層フローが存在しない/同期前**」だった可能性が高い。現在は DB データが正常に存在しており、問題は解決済み。

**影響**:
- ~~IVR フローの多層構造（ネスト）が機能しない~~ → 現在は正常動作
- ~~ユーザーは次の層へ進めず、1層目のメニューがループする~~ → 現在は正常動作
- **運用上の課題**: データ同期のタイミングによっては、同様の問題が再発する可能性がある

### 2.2 目的

IVR フローの次の層への遷移を正常に動作させ、以下を達成する：

1. **次の層の IVR フローが正しく取得される**
2. **遷移失敗時のエラーハンドリングを改善する**（ログ強化、フォールバック処理）
3. **Frontend-Backend 間の IVR フロー参照整合性を確保する**

### 2.3 ユーザーストーリー

```
As a システム管理者
I want to IVR フローで「次の層へ行く」を選択したときに正しく遷移する
So that 多層 IVR フロー（ネスト構造）が正常に動作する

受入条件:
- [ ] AC-1: IVR 1層目でキー押下時、destination の action_code="IV" + ivrFlowId が設定されていれば、次の層へ遷移する
- [ ] AC-2: 次の層の IVR フローが存在しない場合、エラーログが出力され、適切なフォールバック処理が実行される
- [ ] AC-3: 次の層の IVR フローが無効（inactive）の場合、エラーログが出力され、適切なフォールバック処理が実行される
- [ ] AC-4: Backend ログに遷移失敗の理由（IVR フロー ID、エラー内容）が明確に記録される
- [ ] AC-5: Frontend で設定した IVR フロー参照が Backend で正しく解決される
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-12 |
| 起票理由 | IVR フロー実行中に次の層へ遷移できない不具合を発見 |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Code (claude-sonnet-4-5-20250929) |
| 作成日 | 2026-02-12 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "Issue #167 のステアリングファイルを作成、不具合1のみを対象とする" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| - | - | - | - | - |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | - |
| 承認日 | - |
| 承認コメント | - |

### 3.5 実装（該当する場合）

| 項目 | 値 |
|------|-----|
| 実装者 | Codex |
| 実装日 | - |
| 指示者 | @MasanoriSuda |
| 指示内容 | - |
| コードレビュー | - |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ実行者 | - |
| マージ日 | - |
| マージ先 | - |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| virtual-voicebot-backend/docs/design/DD-xxx.md | 確認 | IVR フロー遷移仕様を確認 |

### 4.2 影響するコード

| モジュール | 変更種別 | 概要 |
|-----------|---------|------|
| **Backend** | | |
| virtual-voicebot-backend/src/protocol/session/handlers/mod.rs | 修正 | `execute_db_ivr_destination` の "IV" 分岐のエラーハンドリング改善 |
| virtual-voicebot-backend/src/protocol/session/handlers/mod.rs | 修正 | `enter_db_ivr_flow` のログ強化、デバッグ情報追加 |
| virtual-voicebot-backend/src/interface/db/routing_repo.rs | 確認 | `find_ivr_menu` クエリの確認 |
| **Frontend** | | |
| virtual-voicebot-frontend/storage/db/ivr-flows.json | 確認 | IVR フロー設定の整合性確認 |

---

## 5. 差分仕様（What / How）

### 5.1 調査事項（実装前）

実装前に以下を調査・確認する必要がある：

#### 5.1.1 DB データの確認

**目的**: 次の層の IVR フローが DB に存在するか確認する

**手順**:
1. Frontend の `ivr-flows.json` を確認
   - 1層目の destination に設定されている `ivrFlowId` を特定
2. Backend の `find_ivr_menu` 相当のクエリを手動実行（[routing_repo.rs:215](virtual-voicebot-backend/src/interface/db/routing_repo.rs#L215) 参照）
   ```sql
   -- 次層の IVR フローが存在するか確認
   SELECT
     f.flow_id,
     f.flow_name,
     f.is_active,
     n_announce.node_id AS announce_node_id,
     n_keypad.node_id AS keypad_node_id,
     n_keypad.timeout_sec,
     n_keypad.max_retries,
     n_announce.audio_file_url
   FROM ivr_flows f
   INNER JOIN ivr_nodes n_announce ON n_announce.flow_id = f.flow_id AND n_announce.node_type = 'ANNOUNCE'
   INNER JOIN ivr_nodes n_keypad ON n_keypad.flow_id = f.flow_id AND n_keypad.node_type = 'KEYPAD'
   WHERE f.flow_id = '<ivrFlowId>'
     AND f.is_active = true;
   ```
3. 結果を確認
   - 行が返らない → IVR フローが存在しない、または無効、または ANNOUNCE/KEYPAD ノードが欠落
   - 行が返る → DB データは正常

**注意**: `ivr_menus` テーブルは存在しません。実際は `ivr_flows` + `ivr_nodes` (ANNOUNCE/KEYPAD) の結合クエリで確認します。

#### 5.1.2 Frontend 設定の確認

**目的**: Frontend で IVR フロー参照が正しく設定されているか確認する

**手順**:
1. Frontend UI で IVR フロー詳細を開く
2. 1層目の destination 設定を確認
   - action_code が "IV" であること
   - metadata に `ivrFlowId` が設定されていること
   - `ivrFlowId` が実在する IVR フローの UUID であること
3. 参照先の IVR フローを確認
   - 2層目の IVR フローが存在すること
   - `is_active=true` であること

#### 5.1.3 Backend ログの確認

**目的**: 遷移失敗時のエラーログを確認する

**手順**:
1. Backend ログで以下のメッセージを検索
   - `"failed to start nested IVR flow id={}"` → `enter_db_ivr_flow` が false を返した
   - `"IVR flow not found or inactive id={}"` → IVR フローが存在しない、または無効
   - `"failed to read IVR flow id={} error={}"` → DB クエリエラー
2. エラーの原因を特定
   - IVR フロー ID が誤っている
   - IVR フローが無効化されている
   - DB 接続エラー

### 5.2 修正方針（確定）

**調査結果**: 現在は DB データが正常に存在しており、問題は解決済み。当時の失敗は「データ同期のタイミング問題」だった。

**修正方針**: **A案（運用手順の整備）を優先**

#### A案: 運用手順の整備（データ同期順序の確認）

**対象**: 運用ドキュメント、エラーハンドリング改善

**内容**:
1. **運用確認 SQL セットの提供**
   - IVR フロー参照の整合性を確認する SQL を用意
   - データ同期前後で実行し、参照が解決可能か確認

2. **データ同期順序の明確化**
   - IVR フローを作成する際、参照先のフロー（2層目）を**先に作成**する
   - その後、参照元のフロー（1層目）の destination を設定する

3. **エラーログの改善**
   - 遷移失敗時に、どのフロー ID が見つからないかを明示
   - `warn` → `error` に変更し、運用監視で検知しやすくする

**運用確認 SQL セット**:
```sql
-- 1. IVR フロー一覧と階層構造の確認
SELECT
  f.flow_id,
  f.flow_name,
  f.is_active,
  f.created_at,
  COUNT(n.node_id) AS node_count
FROM ivr_flows f
LEFT JOIN ivr_nodes n ON n.flow_id = f.flow_id
GROUP BY f.flow_id, f.flow_name, f.is_active, f.created_at
ORDER BY f.created_at;

-- 2. IVR 遷移参照の整合性確認（次層フローが存在するか）
SELECT
  n_src.node_id AS source_node,
  n_src.node_type AS source_type,
  n_src.tts_text AS metadata,
  CAST(n_src.tts_text::json->>'ivrFlowId' AS UUID) AS referenced_flow_id,
  f_dst.flow_id AS destination_flow_id,
  f_dst.flow_name AS destination_flow_name,
  f_dst.is_active AS destination_is_active,
  CASE
    WHEN f_dst.flow_id IS NULL THEN '❌ 参照先フローが存在しない'
    WHEN f_dst.is_active = false THEN '⚠️ 参照先フローが無効'
    ELSE '✅ OK'
  END AS status
FROM ivr_nodes n_src
LEFT JOIN ivr_flows f_dst ON f_dst.flow_id = CAST(n_src.tts_text::json->>'ivrFlowId' AS UUID)
WHERE n_src.tts_text::json ? 'ivrFlowId'
ORDER BY n_src.flow_id, n_src.node_id;

-- 3. 特定の IVR フローが find_ivr_menu 条件を満たすか確認
SELECT
  f.flow_id,
  f.flow_name,
  f.is_active,
  n_announce.node_id AS announce_node_id,
  n_keypad.node_id AS keypad_node_id,
  n_keypad.timeout_sec,
  n_keypad.max_retries,
  n_announce.audio_file_url
FROM ivr_flows f
LEFT JOIN ivr_nodes n_announce ON n_announce.flow_id = f.flow_id AND n_announce.node_type = 'ANNOUNCE'
LEFT JOIN ivr_nodes n_keypad ON n_keypad.flow_id = f.flow_id AND n_keypad.node_type = 'KEYPAD'
WHERE f.flow_id = '<ivrFlowId>'  -- ← 確認したいフロー ID を指定
  AND f.is_active = true;
```

#### B案: Backend ログ強化（デバッグ情報不足の場合）

**対象**: Backend コード修正

**修正箇所**:
- `execute_db_ivr_destination` の "IV" 分岐
- `enter_db_ivr_flow`

**修正内容**:
```rust
// execute_db_ivr_destination の "IV" 分岐
"IV" => {
    if let Some(next_flow_id) = self.ivr_flow_id {
        info!(
            "[session {}] attempting to enter nested IVR flow id={}",
            self.call_id, next_flow_id
        );
        if !self.enter_db_ivr_flow(next_flow_id).await {
            error!(  // warn から error に変更
                "[session {}] FAILED to enter nested IVR flow id={} (see previous error logs)",
                self.call_id, next_flow_id
            );
            // フォールバック処理
            self.ivr_flow_id = previous_ivr_flow_id;
            self.ivr_keypad_node_id = previous_ivr_keypad_node_id;
            self.replay_current_ivr_menu().await;
        }
    } else {
        error!(
            "[session {}] IVR destination action_code=IV but ivrFlowId is missing in metadata",
            self.call_id
        );
        // Voicebot モードへフォールバック
        self.transition_to_voicebot_mode(Some(super::VOICEBOT_INTRO_WAV_PATH.to_string()))
            .await;
    }
}
```

#### C案: エラーハンドリング改善（フォールバック処理が不適切な場合）

**対象**: Backend コード修正

**修正内容**:
- 遷移失敗時、現在のメニューを再生するのではなく、明示的なエラーアナウンスを再生
- または、Voicebot モードへフォールバック

### 5.3 未確定点（解消済み）

| ID | 質問 | 決定 | 理由 |
|----|------|------|------|
| Q1 | 調査の結果、IVR フローが存在しない場合はどう修正するか？ | **A: 運用手順の整備** | データ同期順序を明確化し、参照先フローを先に作成する運用ルールを策定 |
| Q2 | 遷移失敗時のフォールバック処理は？ | **A: 現在のメニュー再生（現行維持）** | 既存実装で問題なし、エラーログ改善で対応 |
| Q3 | Backend ログレベルは？ | **B: error に変更** | 遷移失敗は運用監視で検知すべき重大なエラー |

**決定日**: 2026-02-12
**決定者**: @MasanoriSuda（調査結果に基づく）

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #167 | STEER-167 | 起票 |
| handlers/mod.rs:1122-1136 | STEER-167 §2.1 | 不具合箇所 |
| handlers/mod.rs:894-910 | STEER-167 §2.1 | 根本原因 |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [ ] 調査手順が明確か
- [ ] 修正方針（A/B/C案）が適切か
- [ ] 未確定点 Q1/Q2/Q3 が解消されているか
- [ ] トレーサビリティが維持されているか

### 7.2 マージ前チェック（Approved → Merged）

- [ ] 調査が完了している
- [ ] 修正が完了している
- [ ] E2E テストで動作確認が取れている
- [ ] コードレビューを受けている

---

## 8. 備考

### 調査結果：バグではなかった

**結論**: 次層への遷移は正常に動作していた。ログ確認により以下が判明：

```
2026-02-12 14:52 - IVR DTMF matched key=2 ... action_code=IV
2026-02-12 14:52 - starting DB IVR flow id=03805f87-...
```

**1層目→2層目への遷移は成功している**。

当初の報告「次層へ遷移しない」は、**データ同期のタイミング問題**（2層目フローがまだ存在しない時期）に起因していた可能性が高い。現在は DB データが正常に存在しており、遷移も正常に動作している。

### 記録として残す理由

- 調査過程で有用な情報（SQL セット、ログ分析手法）を記録
- データ同期のタイミングによる問題の再発防止のための参考資料
- 同様の報告があった場合の切り分け手順として活用

### 関連情報

- Issue #167 は Closed
- 本ステアリングも Closed として記録を残す

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-12 | 初版作成（Draft） | Claude Code (claude-sonnet-4-5-20250929) |
| 2026-02-12 | 調査結果反映：時系列補足追加、SQL修正、運用手順整備、Q1/Q2/Q3解消 | Claude Code (claude-sonnet-4-5-20250929) |
| 2026-02-12 | Status を Closed に変更、調査結果「バグではない」を記録 | Claude Code (claude-sonnet-4-5-20250929) |
