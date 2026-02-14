# STEER-179: IVR DTMF 遷移バグ修正

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-179 |
| タイトル | IVR DTMF 遷移バグ修正 |
| ステータス | Approved |
| 関連Issue | #179 |
| 優先度 | P0 |
| 作成日 | 2026-02-14 |

---

## 2. ストーリー（Why）

### 2.1 背景

**問題**: IVR 中に番号押下しても指定されたアクションが行えない

- **期待動作**: IVR で選択した番号を押下すると、アクションを行う
- **実際の動作**: IVR で選択した番号を押下しても、同じアナウンスが流れる

**原因**（Codex 調査結果）:

1. **セッション側**: IVR開始時の `keypad_node_id` を保持して使い続ける
   - セッション管理ファイル（`src/protocol/session/` 配下）

2. **serversync 側**: IVRノードを定期的に削除＋再作成して UUID を作り直す
   - `src/interface/sync/converters.rs` - IVR ノード構築ロジック
   - `src/interface/sync/frontend_pull.rs` - IVR フロー同期ロジック

3. **結果**: 通話中に同期が走った瞬間に保持中の `keypad_node_id` が古くなり、`no transition` になる

**ログ証跡**:
- `IVR DTMF matched key=1 ...` → DTMF 受信は正常
- `IVR invalid DTMF key=1 (no transition)` → 遷移テーブル参照に失敗
- DB では同じフローの `ivr_nodes` が 30秒ごとに新UUIDへ回転

### 2.2 目的

serversync による IVR ノード ID 再作成の影響を排除し、IVR 中の DTMF 遷移を正常に動作させる。

### 2.3 ユーザーストーリー

```
As a 通話者
I want to IVR で番号を押下したときに指定されたアクションが実行される
So that 意図した操作（転送、録音など）が正しく行われる

受入条件:
- [ ] IVR 中に番号押下してアクションが正しく実行される
- [ ] serversync が動作中でも DTMF 遷移が失敗しない
- [ ] 既存の IVR フローが引き続き動作する
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-14 |
| 起票理由 | IVR DTMF 遷移が失敗する不具合 |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Code (claude-sonnet-4-5) |
| 作成日 | 2026-02-14 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "Issue #179: IVR DTMF 遷移バグの修正仕様を作成" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| 1 | Codex (claude-opus-4-6) | 2026-02-14 | 要修正 | timeout/invalid 分岐も flow_id 化、RoutingPort 経由設計、受入条件拡充、ファイルパス修正 → 対応完了 |
| 2 | Codex (claude-opus-4-6) | 2026-02-14 | 要修正 | ファイルパス修正、RoutingFuture形式統一、keypad_node_id継続保持明記、SQL ORDER BY LIMIT 1追加 → 対応完了 |
| 3 | Codex (claude-opus-4-6) | 2026-02-14 | 要修正 | metadata_json→tts_text AS metadata_json修正、実装対象パス修正（handlers/mod.rs）、参考欄パス統一 → 対応完了 |
| 4 | Codex (claude-opus-4-6) | 2026-02-14 | OK | 前回指摘事項すべて解消、実装着手可能 |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | @MasanoriSuda |
| 承認日 | 2026-02-14 |
| 承認コメント | Codex レビュー4回を経て承認。flow_id ベース遷移検索による堅牢な設計を採用。 |

### 3.5 実装（該当する場合）

| 項目 | 値 |
|------|-----|
| 実装者 | Codex (GPT-5) |
| 実装日 | 2026-02-14 |
| 指示者 | @MasanoriSuda |
| 指示内容 | 「実装お願いします Refs #179」 |
| コードレビュー | 実施待ち（CodeRabbit/オーナー） |

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
| virtual-voicebot-backend/docs/design/detail/DD-xxx.md | 修正 | IVR DTMF 遷移ロジックの詳細設計 |
| virtual-voicebot-backend/docs/test/unit/UT-xxx.md | 追加 | IVR DTMF 遷移テスト |

### 4.2 影響するコード

| モジュール | 変更種別 | 概要 |
|-----------|---------|------|
| src/shared/ports/routing_port.rs | 追加 | flow_id ベース遷移検索 API 追加（find_ivr_dtmf_destination_by_flow 等） |
| src/interface/db/routing_repo.rs | 追加 | flow_id ベース遷移検索の実装 |
| src/protocol/session/handlers/mod.rs | 修正 | DTMF/timeout/invalid 遷移ロジックを flow_id ベースに変更 |

---

## 5. 差分仕様（What / How）

### 5.1 採用アプローチ

**DTMF/timeout/invalid 遷移を flow_id ベースで引く**

**概要**:
セッション側の IVR 遷移ロジックを `keypad_node_id` 固定ではなく、`flow_id` ベースで遷移先を検索するように変更する。DTMF キー遷移だけでなく、timeout/invalid 分岐も同様に統一する。

**理由**:
1. **堅牢性**: serversync が IVR ノードを再作成しても影響を受けない
2. **将来性**: Frontend JSON の UUID 変更にも対応できる
3. **設計の一貫性**: flow_id は不変なので、より安定した参照基準になる

**修正範囲**:
- **DTMF キー遷移**: 現在は `keypad_node_id` + DTMF キー → 変更後は `flow_id` + DTMF キー
- **timeout 遷移**: 現在は `keypad_node_id` + timeout → 変更後は `flow_id` + timeout
- **invalid 遷移**: 現在は `keypad_node_id` + invalid → 変更後は `flow_id` + invalid

---

### 5.2 実装箇所

#### 5.2.1 Session 側の状態保持

**ファイル**: `src/protocol/session/handlers/mod.rs` および関連セッション管理ファイル

**変更内容**:
- IVR 開始時に `keypad_node_id` と `ivr_flow_id` の両方を保持（既存の keypad_node_id は継続保持）
- **遷移検索のみ flow_id ベースに変更**（イベント記録や既存処理では keypad_node_id を使用）

#### 5.2.2 RoutingPort API 追加

**ファイル**: `src/shared/ports/routing_port.rs`

**追加 API**:
```rust
/// flow_id + DTMF キーで遷移先を検索
fn find_ivr_dtmf_destination_by_flow(
    &self,
    flow_id: Uuid,
    dtmf_key: &str,
) -> RoutingFuture<Option<IvrDestinationRow>>;

/// flow_id + timeout で遷移先を検索
fn find_ivr_timeout_destination_by_flow(
    &self,
    flow_id: Uuid,
) -> RoutingFuture<Option<IvrDestinationRow>>;

/// flow_id + invalid で遷移先を検索
fn find_ivr_invalid_destination_by_flow(
    &self,
    flow_id: Uuid,
) -> RoutingFuture<Option<IvrDestinationRow>>;
```

**備考**:
- 現行の `RoutingFuture<T>` = `Pin<Box<dyn Future<Output = Result<T, RoutingPortError>> + Send>>` を使用
- 既存の `IvrDestinationRow` 構造体を返す

#### 5.2.3 RoutingRepo 実装

**ファイル**: `src/interface/db/routing_repo.rs`

**実装イメージ**:
```rust
fn find_ivr_dtmf_destination_by_flow(
    &self,
    flow_id: Uuid,
    dtmf_key: &str,
) -> RoutingFuture<Option<IvrDestinationRow>> {
    let pool = self.pool.clone();
    let dtmf_key = dtmf_key.to_string();
    Box::pin(async move {
        let row = sqlx::query(
            "SELECT t.id AS transition_id, dn.id AS node_id, dn.action_code,
                    dn.audio_file_url, dn.tts_text AS metadata_json
             FROM ivr_transitions t
             JOIN ivr_nodes n ON t.from_node_id = n.id
             JOIN ivr_nodes dn ON t.to_node_id = dn.id
             WHERE n.flow_id = $1
               AND n.node_type = 'KEYPAD'
               AND t.input_type = 'DTMF'
               AND t.dtmf_key = $2
             ORDER BY t.id ASC
             LIMIT 1",
        )
        .bind(flow_id)
        .bind(dtmf_key)
        .fetch_optional(&pool)
        .await
        .map_err(map_read_err)?;

        let Some(row) = row else {
            return Ok(None);
        };

        Ok(Some(IvrDestinationRow {
            transition_id: row.try_get("transition_id").map_err(map_read_err)?,
            node_id: row.try_get("node_id").map_err(map_read_err)?,
            action_code: row.try_get("action_code").map_err(map_read_err)?,
            audio_file_url: row.try_get("audio_file_url").map_err(map_read_err)?,
            metadata_json: row.try_get("metadata_json").map_err(map_read_err)?,
        }))
    })
}

fn find_ivr_timeout_destination_by_flow(
    &self,
    flow_id: Uuid,
) -> RoutingFuture<Option<IvrDestinationRow>> {
    let pool = self.pool.clone();
    Box::pin(async move {
        let row = sqlx::query(
            "SELECT t.id AS transition_id, dn.id AS node_id, dn.action_code,
                    dn.audio_file_url, dn.tts_text AS metadata_json
             FROM ivr_transitions t
             JOIN ivr_nodes n ON t.from_node_id = n.id
             JOIN ivr_nodes dn ON t.to_node_id = dn.id
             WHERE n.flow_id = $1
               AND n.node_type = 'KEYPAD'
               AND t.input_type = 'TIMEOUT'
             ORDER BY t.id ASC
             LIMIT 1",
        )
        .bind(flow_id)
        .fetch_optional(&pool)
        .await
        .map_err(map_read_err)?;

        let Some(row) = row else {
            return Ok(None);
        };

        Ok(Some(IvrDestinationRow {
            transition_id: row.try_get("transition_id").map_err(map_read_err)?,
            node_id: row.try_get("node_id").map_err(map_read_err)?,
            action_code: row.try_get("action_code").map_err(map_read_err)?,
            audio_file_url: row.try_get("audio_file_url").map_err(map_read_err)?,
            metadata_json: row.try_get("metadata_json").map_err(map_read_err)?,
        }))
    })
}

fn find_ivr_invalid_destination_by_flow(
    &self,
    flow_id: Uuid,
) -> RoutingFuture<Option<IvrDestinationRow>> {
    let pool = self.pool.clone();
    Box::pin(async move {
        let row = sqlx::query(
            "SELECT t.id AS transition_id, dn.id AS node_id, dn.action_code,
                    dn.audio_file_url, dn.tts_text AS metadata_json
             FROM ivr_transitions t
             JOIN ivr_nodes n ON t.from_node_id = n.id
             JOIN ivr_nodes dn ON t.to_node_id = dn.id
             WHERE n.flow_id = $1
               AND n.node_type = 'KEYPAD'
               AND t.input_type = 'INVALID'
             ORDER BY t.id ASC
             LIMIT 1",
        )
        .bind(flow_id)
        .fetch_optional(&pool)
        .await
        .map_err(map_read_err)?;

        let Some(row) = row else {
            return Ok(None);
        };

        Ok(Some(IvrDestinationRow {
            transition_id: row.try_get("transition_id").map_err(map_read_err)?,
            node_id: row.try_get("node_id").map_err(map_read_err)?,
            action_code: row.try_get("action_code").map_err(map_read_err)?,
            audio_file_url: row.try_get("audio_file_url").map_err(map_read_err)?,
            metadata_json: row.try_get("metadata_json").map_err(map_read_err)?,
        }))
    })
}
```

**備考**:
- 現行の `RoutingRepoImpl` パターンに合わせて `Box::pin(async move { ... })` を使用
- 複数 KEYPAD ノード対応時の曖昧性対策として `ORDER BY t.id ASC LIMIT 1` を追加
- Phase 4-A の制約（1フロー1 KEYPAD ノード）では常に1件のみ返る

#### 5.2.4 Session の DTMF/timeout/invalid ハンドリング

**ファイル**: `src/protocol/session/handlers/mod.rs`

**変更内容**:
```rust
// DTMF 遷移
async fn handle_dtmf(&mut self, key: &str) {
    // 変更前: keypad_node_id で遷移先を検索
    // let destination = self.routing_port
    //     .find_ivr_dtmf_destination(self.keypad_node_id, key)
    //     .await;

    // 変更後: flow_id + key で遷移先を検索
    let destination = self.routing_port
        .find_ivr_dtmf_destination_by_flow(self.ivr_flow_id, key)
        .await;

    // ...既存ロジック
}

// timeout 遷移
async fn handle_timeout(&mut self) {
    // 変更前: keypad_node_id で遷移先を検索
    // let destination = self.routing_port
    //     .find_ivr_timeout_destination(self.keypad_node_id)
    //     .await;

    // 変更後: flow_id で timeout 遷移を検索
    let destination = self.routing_port
        .find_ivr_timeout_destination_by_flow(self.ivr_flow_id)
        .await;

    // ...既存ロジック
}

// invalid 遷移
async fn handle_invalid(&mut self) {
    // 変更前: keypad_node_id で遷移先を検索
    // let destination = self.routing_port
    //     .find_ivr_invalid_destination(self.keypad_node_id)
    //     .await;

    // 変更後: flow_id で invalid 遷移を検索
    let destination = self.routing_port
        .find_ivr_invalid_destination_by_flow(self.ivr_flow_id)
        .await;

    // ...既存ロジック
}
```

**備考**:
- keypad_node_id は継続保持（イベント記録や既存処理で使用）
- 遷移検索のみ flow_id ベースに変更

---

### 5.3 受入条件

#### 5.3.1 基本動作

- [ ] IVR 中に番号押下してアクションが正しく実行される
- [ ] 既存の IVR フロー（Phase 4-A のシンプル IVR）が引き続き動作する
- [ ] DTMF キー入力から遷移先ノード検索までのロジックが正しく実装される

#### 5.3.2 serversync 中の動作

- [ ] serversync が動作中（30秒間隔）でも DTMF 遷移が失敗しない
- [ ] **通話中に同期が1回以上走った場合でも、DTMF 遷移が正常に動作する**
- [ ] serversync によるノード ID 再作成の影響を受けない

#### 5.3.3 エラーケース

- [ ] エラーケース（遷移先なし、無効なキー）が正しくハンドリングされる
- [ ] **timeout 遷移が同期中でも正常に動作する**
- [ ] **invalid 遷移が同期中でも正常に動作する**

#### 5.3.4 単体テスト

- [ ] 単体テストで以下をカバー:
  - DTMF 遷移が正しく動作する
  - timeout 遷移が正しく動作する
  - invalid 遷移が正しく動作する
  - serversync 後も DTMF 遷移が動作する（flow_id ベース検索の検証）
  - 無効なキーが入力された場合のハンドリング
  - **2階層目 IVR で有効キーが成功する**（将来の拡張を見据えた検証）

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #179 | STEER-179 | 起票 |
| STEER-179 | Backend DD-xxx | 詳細設計 |
| STEER-179 | Backend UT-xxx | 単体テスト |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [ ] 要件の記述が明確か
- [ ] 詳細設計で実装者が迷わないか
- [ ] テストケースが網羅的か
- [ ] 既存仕様（contract.md, IVR 設計）との整合性があるか
- [ ] トレーサビリティが維持されているか
- [ ] Codex 調査結果と矛盾していないか

### 7.2 マージ前チェック（Approved → Merged）

- [ ] 実装が完了している
- [ ] コードレビューを受けている
- [ ] 関連テストがPASS
- [ ] 手動テストで IVR DTMF 遷移が正常に動作することを確認

---

## 8. 備考

### 8.1 すぐの回避策

実装までの間、以下の回避策で対処可能:

1. **FRONTEND_SYNC_INTERVAL_SEC を長くする**（例: 300〜600秒）
   - serversync の実行間隔を長くすることで、通話中に同期が走る確率を下げる

2. **検証中だけ serversync 停止**
   - serversync プロセスを停止することで、IVR ノード ID の再作成を防ぐ

### 8.2 技術的注意点

- **Phase 4-A の制約**: 現在のシンプル IVR パターンは **1フロー1 KEYPAD ノード** を前提としている
  - この前提により、flow_id + key で一意に遷移先を特定できる
  - SQL に `ORDER BY t.id ASC LIMIT 1` を追加することで、複数候補がある場合でも確定的に動作する
  - Phase 5+ で複数 KEYPAD ノードをサポートする場合は、階層パスやノード順序を考慮する必要がある

- **パフォーマンス**: flow_id ベース検索では DTMF 入力ごとに DB クエリが発生する
  - 現行の keypad_node_id ベース検索も同様に DB クエリを発行しているため、パフォーマンス特性は同等
  - 将来的にセッション開始時に遷移テーブルをキャッシュすることを検討

- **keypad_node_id の保持**: 既存の keypad_node_id は継続保持する
  - イベント記録や既存処理で使用されているため、削除せず遷移検索のみ flow_id 化

### 8.3 参考情報

- Codex 調査ログ: `IVR DTMF matched key=1 ...` → `IVR invalid DTMF key=1 (no transition)`
- 影響範囲:
  - セッション側: `src/protocol/session/handlers/mod.rs`（IVR 状態保持、DTMF/timeout/invalid ハンドリング）
  - serversync 側: `src/interface/sync/converters.rs`, `src/interface/sync/frontend_pull.rs`
  - ルーティング側: `src/shared/ports/routing_port.rs`, `src/interface/db/routing_repo.rs`

---

## 9. 未確定点・質問

なし（アプローチ2で確定、migration 不要）

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-14 | 初版作成 | Claude Code (claude-sonnet-4-5) |
| 2026-02-14 | Codex レビュー#1対応：アプローチ2確定、timeout/invalid含む、RoutingPort経由設計、受入条件拡充 | Claude Code (claude-sonnet-4-5) |
| 2026-02-14 | Codex レビュー#2対応：ファイルパス修正、RoutingFuture形式統一、keypad_node_id継続保持明記、SQL ORDER BY LIMIT 1追加 | Claude Code (claude-sonnet-4-5) |
| 2026-02-14 | Codex レビュー#3対応：metadata_json→tts_text AS metadata_json修正、実装対象パス修正（handlers/mod.rs）、参考欄パス統一 | Claude Code (claude-sonnet-4-5) |
| 2026-02-14 | Codex レビュー#4 OK、@MasanoriSuda 承認、Status: Approved | Claude Code (claude-sonnet-4-5) |
