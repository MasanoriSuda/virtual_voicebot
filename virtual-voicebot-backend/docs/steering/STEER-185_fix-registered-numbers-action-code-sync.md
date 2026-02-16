# STEER-185: registered_numbers の action_code 固定化問題の修正

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-185 |
| タイトル | registered_numbers の action_code 固定化問題の修正 |
| ステータス | Approved |
| 関連Issue | #185 |
| 優先度 | P1 |
| 作成日 | 2026-02-16 |

---

## 2. ストーリー（Why）

### 2.1 背景

**問題**:
- 一旦登録した電話番号からかけると、着信アクションを変更してもアクションが変わらない
- 番号グループを変更・削除してもアクションが変わらない
- 番号を削除してもアクションが変わらない

**原因（Codex 調査結果）**:
1. **registered_numbers.action_code が固定化**
   - Frontend 同期時に action_code='VR' 固定値で INSERT ([converters.rs:191](../../src/interface/sync/converters.rs#L191))
   - ON CONFLICT 時も action_code は更新しない ([converters.rs:194-196](../../src/interface/sync/converters.rs#L194-L196))
   - グループ削除時も group_id を NULL にするだけで action_code は残る ([converters.rs:162](../../src/interface/sync/converters.rs#L162))

2. **ルート評価の優先順位**
   - Stage1 (registered_numbers) が Stage2 (call_action_rules) より先に評価 ([evaluator.rs:180](../../src/service/routing/evaluator.rs#L180))
   - registered_numbers にヒットすると、その action_code が返されて終了
   - call_action_rules の設定が無視される

**影響**:
- Frontend で着信アクションを変更しても実際の通話に反映されない
- ユーザーが設定変更できない（機能が事実上無効化）
- 番号グループ管理の価値が失われる

### 2.2 目的

Frontend で設定した着信アクション（番号グループの action）が実際の通話に正しく反映されるようにする。

### 2.3 ユーザーストーリー

```
As a システム管理者
I want to Frontend で番号グループの着信アクションを変更したら、実際の通話にすぐ反映される
So that 着信ごとに異なる対応（録音あり/なし、拒否など）を柔軟に設定できる

受入条件:
- [ ] Frontend で番号グループ「VIP」の ActionCode を VR → BZ に変更すると、次の着信から BZ で処理される
- [ ] Frontend で番号グループを削除すると、その番号からの着信はデフォルトアクションで処理される
- [ ] Frontend で番号を削除すると、その番号は registered_numbers で論理削除され、別のルール（Stage3 以降）で処理される
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-16 |
| 起票理由 | Frontend 設定変更が Backend 着信処理に反映されない |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Code (claude-sonnet-4-5) |
| 作成日 | 2026-02-16 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "Issue #185 のステアリングファイルを作成" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| - | - | - | - | |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | @MasanoriSuda |
| 承認日 | 2026-02-16 |
| 承認コメント | Codex レビュー3回実施、全指摘対応完了（重大1件、中3件、軽6件）。実装フェーズへ |

### 3.5 実装（該当する場合）

| 項目 | 値 |
|------|-----|
| 実装者 | Codex |
| 実装日 | |
| 指示者 | |
| 指示内容 | |
| コードレビュー | |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ実行者 | |
| マージ日 | |
| マージ先 | BD-004, DD-009 (sync), contract.md |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| [BD-004](../design/basic/BD-004_call-routing-db.md) | 修正 | registered_numbers スキーマ仕様の明確化 |
| [STEER-096](STEER-096_serversync.md) | 参照 | Frontend 同期ロジックの設計（converters.rs の実装参照） |
| [STEER-140](STEER-140_rule-evaluation-engine.md) | 参照 | ルール評価エンジンの設計確認 |

### 4.2 影響するコード

| モジュール | 変更種別 | 概要 |
|-----------|---------|------|
| src/interface/sync/converters.rs | 修正 | registered_numbers 同期ロジック変更（論理削除、復活処理） |
| src/service/routing/evaluator.rs | 修正 | Stage1 評価ロジック変更（group_id がある場合は Stage2 優先） |
| src/shared/ports/routing_port.rs | 修正 | RegisteredNumberRow に group_id フィールド追加 |

---

## 5. 差分仕様（What / How）

### 5.1 仕様案（複数オプション）

**現状の設計矛盾**:
- **registered_numbers.action_code**: 番号個別の設定を想定
- **Frontend 運用**: 番号グループ単位で action を管理（call_action_rules に保存）
- **同期処理**: registered_numbers.action_code を固定値 'VR' で INSERT

**Option A: registered_numbers.action_code を call_action_rules と同期**

- Frontend 同期時に call_action_rules の action_config から action_code を抽出して registered_numbers に設定
- ON CONFLICT 時も action_code を更新
- グループ削除時は action_code をデフォルト値にリセット

**メリット**:
- Stage1 評価ロジックをそのまま維持
- パフォーマンス良好（Stage1 で終了）

**デメリット**:
- call_action_rules と registered_numbers の二重管理
- 同期処理が複雑化（call_action_rules → registered_numbers の逆引き）

**実装概要**:
```rust
// converters.rs
async fn convert_phone_groups(...) {
    for group in groups {
        // call_action_rules から action_config を取得
        let action_code = fetch_action_code_for_group(group.id).await?;

        for phone_number in &group.phone_numbers {
            sqlx::query(
                "INSERT INTO registered_numbers (..., action_code, ...)
                 VALUES (..., $4, ...)
                 ON CONFLICT (phone_number) WHERE deleted_at IS NULL
                 DO UPDATE SET
                    group_id = $2,
                    group_name = $3,
                    action_code = $4,  -- 追加
                    updated_at = NOW()",
            )
            .bind(normalized)
            .bind(group.id)
            .bind(group.name.clone())
            .bind(action_code)  // 追加
            .execute(&mut **tx)
            .await?;
        }
    }
}
```

---

**Option B: Stage1 評価を廃止し、常に call_action_rules を参照**

- registered_numbers は電話番号と group_id のマッピングのみ保持
- Stage1 では group_id のみ取得し、action_code は取得しない
- 常に Stage2 (call_action_rules) で action_code を決定

**メリット**:
- 単一の真実の源泉（call_action_rules）
- 同期処理がシンプル（registered_numbers は group_id のみ管理）

**デメリット**:
- 番号個別の action_code 設定ができなくなる（仕様変更）
- STEER-140 の設計意図と矛盾

**実装概要**:
```rust
// evaluator.rs
async fn evaluate(&self, caller_id: &str, call_id: &str) -> Result<ActionConfig> {
    // 0. 非通知判定（既存）
    // ...

    // 1. 電話番号正規化（既存）
    let normalized_caller_id = self.normalize_phone_number(caller_id)?;

    // 2. 段階1: 番号完全一致（group_id のみ取得）
    let group_id = self.routing_port.find_caller_group(&normalized_caller_id).await?;

    if let Some(group_id) = group_id {
        // 3. 段階2: call_action_rules を検索
        if let Some(action) = self.match_caller_group_by_id(&group_id, call_id).await? {
            return Ok(action);
        }
    }

    // 4. 段階3: カテゴリ評価（既存）
    // ...

    // 5. 段階4: デフォルトアクション（既存）
    // ...
}
```

---

**Option C: group_id がある場合は Stage2 を優先評価**

- registered_numbers にマッチしても、group_id が設定されている場合は Stage2 (call_action_rules) を優先
- group_id が NULL の場合のみ registered_numbers.action_code を使用
- 評価順序を変更するだけで実装可能

**メリット**:
- 最小の変更で問題を解決
- 番号個別設定とグループ設定の両立（将来対応可能）
- 同期処理の変更不要

**デメリット**:
- 評価ロジックが複雑化（条件分岐が増える）
- Stage1 と Stage2 の役割が曖昧化

**実装概要**:
```rust
// evaluator.rs
async fn match_registered_number(&self, caller_id: &str, call_id: &str) -> Result<Option<ActionConfig>> {
    let row = self.routing_port.find_registered_number(caller_id).await?;

    match row {
        Some(row) => {
            // group_id が設定されている場合は Stage2 に移行
            if row.group_id.is_some() {
                info!("[RuleEvaluator] call_id={} Stage 1: group_id found, defer to Stage 2", call_id);
                return Ok(None);  // Stage2 へ移行
            }

            // group_id が NULL の場合のみ action_code を使用
            let action = ActionConfig {
                action_code: row.action_code,
                ivr_flow_id: row.ivr_flow_id,
                recording_enabled: row.recording_enabled,
                announce_enabled: row.announce_enabled,
            };
            info!("[RuleEvaluator] call_id={} Stage 1: action_code={} (no group)", call_id, action.action_code);
            Ok(Some(action))
        }
        None => Ok(None),
    }
}
```

---

**Option D: registered_numbers から action_code カラムを削除（スキーマ変更）**

- BD-004 の registered_numbers スキーマから action_code, recording_enabled, announce_enabled を削除
- 常に call_action_rules を参照する設計に変更
- マイグレーションが必要

**メリット**:
- 設計が最もシンプル
- 二重管理の問題が根本的に解決

**デメリット**:
- スキーマ変更が必要（マイグレーション、ロールバック計画）
- 既存データの移行が必要
- STEER-140 の設計を大幅に変更

**実装概要**:
```sql
-- migrations/20260216_xxx_remove_action_code_from_registered_numbers.sql
ALTER TABLE registered_numbers
  DROP COLUMN action_code,
  DROP COLUMN recording_enabled,
  DROP COLUMN announce_enabled,
  DROP COLUMN ivr_flow_id;
```

---

### 5.2 推奨案の選定（確定）

**段階的アプローチ**（短期 → 中長期）:

| Phase | Option | 目的 | スケジュール |
|-------|--------|------|-------------|
| Phase 1（短期） | **Option C** | 事故を止める（評価順序変更） | 本 STEER |
| Phase 2（中長期） | **Option D** | 設計をクリーンアップ（action_code カラム削除） | 別 STEER（運用安定後） |

---

**Phase 1: Option C（短期対応）**

**決定**:
- group_id がある場合は Stage2 (call_action_rules) を優先評価
- group_id が NULL の場合のみ registered_numbers.action_code を使用

**理由**:
- **最小変更、低リスク**（スキーマ変更不要、評価ロジックと同期処理のみ変更）
- **現在の Frontend 運用に適合**（番号グループ単位のみ）
- **事故を即座に止められる**（リリース後すぐに問題解決）

**運用方針**:
- **番号個別の action_code 設定は使わない**（仕様として明文化）
- Frontend では番号グループ単位で action を管理
- 例外が必要な場合は「グループを分ける」運用で吸収

**影響可視化**:
- リリース直後は `group_id != NULL` かつ Stage1 ヒット時に INFO ログ出力
- 影響対象件数を監視（必要なら feature flag で段階的にオン）

---

**Phase 2: Option D（中長期対応）**

**決定**:
- registered_numbers から action_code, recording_enabled, announce_enabled カラムを削除
- 常に call_action_rules を参照する設計に変更

**タイミング**:
- Phase 1 リリース後、運用で問題ないことを確認（1ヶ月程度）
- ログで「registered_numbers.action_code が参照されていない」ことを確認
- 別 STEER（例: STEER-xxx）で実施

**安全ルート**:
1. action_code を読み取り専用（deprecated）扱いにして更新を止める
2. 一定期間ログで「参照されていない」を確認
3. カラム削除のマイグレーション実行

**メリット**:
- 設計が最もシンプル（二重管理の問題が根本的に解決）
- Frontend とのデータモデル整合性が向上

---

### 5.3 番号削除時の処理（Phase 1 対応）

**問題**:
- 現在、グループ削除時は `group_id = NULL` にするだけで行は残る
- Frontend で番号を削除しても registered_numbers から消えない可能性

**決定**: **論理削除（deleted_at）を基本**

**理由**:
- 電話運用は「消した/消してない」でトラブルになりがち
- 復旧可能性がある方が強い（監査・復旧のため）

**実装**:
```rust
// converters.rs
async fn convert_phone_groups(...) {
    let all_phone_numbers: Vec<String> = groups.iter()
        .flat_map(|g| g.phone_numbers.clone())
        .collect();

    // Frontend にない番号は論理削除
    sqlx::query(
        "UPDATE registered_numbers
         SET deleted_at = NOW(), updated_at = NOW()
         WHERE phone_number != ALL($1)
           AND deleted_at IS NULL",
    )
    .bind(&all_phone_numbers)
    .execute(&mut **tx)
    .await?;
}
```

**部分ユニーク制約（既存）**:

**前提**: registered_numbers には既に部分ユニーク制約が存在（[20260206000005_create_registered_numbers.sql:24](../../migrations/20260206000005_create_registered_numbers.sql#L24)）

```sql
-- 既存の制約（migration 済み）
CREATE UNIQUE INDEX uq_registered_numbers_phone
    ON registered_numbers(phone_number) WHERE deleted_at IS NULL;
```

この制約により、論理削除後に同じ番号を再登録できる。新規 migration は不要。

**再登録時の処理**:

**Option A（復活）**: deleted_at IS NOT NULL の行を復活
```rust
sqlx::query(
    "UPDATE registered_numbers
     SET deleted_at = NULL, group_id = $1, group_name = $2, updated_at = NOW()
     WHERE phone_number = $3
       AND deleted_at IS NOT NULL",
)
```

**Option B（新規 INSERT）**: 既存行は無視して新規 INSERT
```rust
sqlx::query(
    "INSERT INTO registered_numbers (...)
     VALUES (...)
     ON CONFLICT ... DO UPDATE ...",
)
```

**推奨**: **Option A（復活）**（履歴が保持される、version が継続）

---

### 5.4 段階的実装計画（Phase 1 / Phase 2）

#### Phase 1: Option C 実装（本 STEER）

**目的**: 事故を止める（評価順序変更で問題を即座に解決）

**実装内容**:

1. **evaluator.rs の修正**
   - `match_registered_number()` で group_id がある場合は Stage2 に移行
   - ログ追加:
     ```rust
     info!("[RuleEvaluator] call_id={} Stage 1: group_id found, defer to Stage 2 (phone_number={}, group_id={})",
           call_id, phone_number, group_id);
     ```

2. **routing_port.rs の修正**
   - `RegisteredNumberRow` に `group_id: Option<Uuid>` フィールド追加
   - SQL に `group_id` を追加:
     ```sql
     SELECT action_code, ivr_flow_id, recording_enabled, announce_enabled, group_id
     FROM registered_numbers
     WHERE phone_number = $1 AND deleted_at IS NULL
     ```

3. **converters.rs の修正**
   - Frontend にない番号は論理削除（§5.3 参照）
   - 再登録時は復活処理（deleted_at = NULL）
   - 既存の部分ユニーク制約（uq_registered_numbers_phone）を前提に実装

**リリース後の監視**:
- `group_id != NULL` かつ Stage1 ヒット時のログ件数を監視
- 影響が大きい場合は feature flag で段階的にオン（必要なら）

**AC（受入条件）**:
- Frontend で番号グループの ActionCode 変更が次の着信から反映される
- ログに評価過程（Stage1 → Stage2 移行）が記録される

---

#### Phase 2: Option D 実装（別 STEER）

**目的**: 設計をクリーンアップ（action_code カラム削除で二重管理を解消）

**前提条件**:
- Phase 1 リリース後、運用で問題ないことを確認（目安: 1ヶ月）
- ログで「registered_numbers.action_code が参照されていない」ことを確認

**実装内容**:

1. **action_code を読み取り専用（deprecated）扱い**
   - converters.rs で action_code の更新を停止
   - evaluator.rs で action_code の参照を停止（常に Stage2 へ）
   - ログで「deprecated フィールドが参照された」を出力

2. **一定期間（例: 2週間）ログで確認**
   - 「action_code が参照された」ログが 0 件であることを確認

3. **カラム削除のマイグレーション実行**
   ```sql
   -- migrations/20260xxx_remove_action_code_from_registered_numbers.sql
   ALTER TABLE registered_numbers
     DROP COLUMN action_code,
     DROP COLUMN recording_enabled,
     DROP COLUMN announce_enabled,
     DROP COLUMN ivr_flow_id;
   ```

4. **evaluator.rs の修正**
   - Stage1 を廃止または簡略化（group_id のみ取得）
   - 常に Stage2 (call_action_rules) で action_code を決定

**ロールバック計画**:
- カラム削除前に DB バックアップ
- 問題発生時はマイグレーションをロールバック（カラム再追加）

**AC（受入条件）**:
- registered_numbers に action_code カラムが存在しない
- 全ての着信が Stage2 (call_action_rules) で処理される

---

## 6. 受入条件（Acceptance Criteria）

### Phase 1（本 STEER）

#### AC-1: Frontend 設定変更の反映

- [ ] Frontend で番号グループ「VIP」の ActionCode を VR → BZ に変更すると、次の着信から BZ で処理される
- [ ] Frontend で番号グループ「スパム」を作成し、ActionCode=BZ を設定すると、その番号からの着信が BZ で処理される
- [ ] 設定変更後の着信処理ログに、変更後の ActionCode が出力される

#### AC-2: 番号グループ削除時の動作

- [ ] Frontend で番号グループを削除すると、その番号からの着信はデフォルトアクション（Stage4）で処理される
- [ ] グループ削除後の着信処理ログに、Stage4（defaultAction）が適用されたことが記録される

#### AC-3: 番号削除時の動作（論理削除）

- [ ] Frontend で番号を削除すると、その番号は registered_numbers で論理削除される（deleted_at に日時設定）
- [ ] 削除された番号からの着信は、Stage3（カテゴリ評価）または Stage4（デフォルト）で処理される
- [ ] 同じ番号を再登録すると、deleted_at が NULL に戻り、履歴が保持される

#### AC-4: 部分ユニーク制約（既存）

- [ ] registered_numbers に部分ユニーク制約（uq_registered_numbers_phone）が存在することを確認
- [ ] 論理削除後に同じ番号を再登録できる（復活処理が動作）

#### AC-5: ログ出力

- [ ] 評価過程（Stage1〜4、Hit/Miss）がログ出力される
- [ ] Stage1 で group_id がある場合、Stage2 に移行することがログ出力される
  ```
  [RuleEvaluator] call_id=xxx Stage 1: group_id found, defer to Stage 2 (phone_number=+819012345678, group_id=xxx)
  ```
- [ ] 適用された ActionCode がログ出力される

#### AC-6: 影響可視化

- [ ] リリース直後、`group_id != NULL` かつ Stage1 ヒット時の件数を監視できる
- [ ] ログに INFO レベルで出力される

---

### Phase 2（別 STEER）

- [ ] registered_numbers に action_code カラムが存在しない
- [ ] 全ての着信が Stage2 (call_action_rules) で処理される
- [ ] action_code 参照ログが 0 件（一定期間確認後）

---

## 7. 未確定点・質問リスト（Open Questions）

### Q-01: 番号個別の action_code 設定は必要か？ ✅ 回答済み

**回答**: **現時点は "不要"（グループ単位に統一）**

**理由**:
- Frontend がグループ単位しか扱っていないため、番号個別は「隠し機能」になってバグ源（今回まさにそれ）
- 例外が将来出ても、**「例外はグループを分ける」**運用で大体吸収できる
- 運用も分かりやすい（グループ単位で統一）

**ただし**:
- 将来「VIP 番号だけ例外」みたいな要求が濃厚なら、番号個別は必要になる
- その場合でも **Option A（同期）より Option C（優先順位で両立）**の方が壊れにくい

**結論**:
- いまは **"番号個別は使わない"** を仕様として明文化
- データもそれに寄せる（group_id がある場合は Stage2 優先）

**選択肢への影響**:
- Phase 1: Option C（group_id がある場合は Stage2 優先）
- Phase 2: Option D（action_code カラム削除）

---

### Q-02: スキーマ変更のリスク許容度は？ ✅ 回答済み

**回答**: **段階的にやる（いきなり Option D はしない／でも最終的には Option D が強い）**

**段階的アプローチ**:
1. **まず Option C で本番挙動を直して事故を止める**（Phase 1）
2. **運用で問題なければ Option D（action_code カラム削除）へ**（Phase 2）

**安全ルート（Phase 2 実施時）**:
1. action_code を読み取り専用（deprecated）扱いにして更新を止める
2. 一定期間ログで「参照されていない」を確認
3. 落とす

**選択肢への影響**:
- Phase 1: Option C（スキーマ変更なし、評価ロジック＋同期処理変更）
- Phase 2: Option D（運用安定後、別 STEER で実施）

---

### Q-03: 番号削除は物理削除か論理削除か？ ✅ 回答済み

**回答**: **論理削除（deleted_at）を基本**（少なくとも MVP 以降の監査・復旧のため）

**理由**:
- 電話運用は「消した/消してない」でトラブルになりがちなので、復旧可能性がある方が強い

**実装指針（Postgres）**:
- registered_numbers のユニーク制約が phone_number 単体だと、論理削除後に同じ番号を再登録できない
- **部分ユニーク制約**が定石:
  ```sql
  CREATE UNIQUE INDEX ux_registered_numbers_phone_active
    ON registered_numbers(phone_number)
    WHERE deleted_at IS NULL;
  ```
- 再登録時は deleted_at IS NOT NULL の行を復活させるか、新規 INSERT するかを決める
  - **復活が楽**: `UPDATE ... SET deleted_at = NULL, ...`

**選択肢**:
- 論理削除: `UPDATE registered_numbers SET deleted_at = NOW() WHERE ...`
- 部分ユニーク制約: 上記 SQL

---

### Q-04: 評価順序変更（Option C）の影響は？ ✅ 回答済み

**回答**: 影響は **"番号個別 action_code を使っていたケース"** にだけ出る

**具体的影響**:
- 「registered_numbers にヒットし、かつ group_id がある番号」で、これまで Stage1 で確定していた action_code が、今後 Stage2 優先になる
- でも今回の不具合はまさにそれで、**"意図した仕様"** としては Stage2 を優先したいはず

**安全策**:
1. **影響対象をログで可視化**（`registered_numbers ヒット && group_id != NULL` の件数）
2. **リリース直後はそのイベントを INFO 以上で出す**
   ```rust
   info!("[RuleEvaluator] call_id={} Stage 1: group_id found, defer to Stage 2 (phone_number={}, group_id={})",
         call_id, phone_number, group_id);
   ```
3. **必要なら feature flag で段階的にオン**
   - 環境変数: `ENABLE_STAGE2_PRIORITY=true`
   - 初期リリースでは false、ログで影響を確認後に true に

**選択肢への影響**:
- ログ追加: evaluator.rs の match_registered_number() に INFO ログ
- 必要なら feature flag 追加（環境変数 or system_settings.extra）

---

## 8. リスク・制約

### 8.1 リスク

| リスク | 影響度 | 発生確率 | 対策 |
|--------|--------|---------|------|
| Option 選定ミス（後から別 Option に変更） | 高 | 中 | レビュー時に要件を明確化し、慎重に選定 |
| スキーマ変更失敗（Option D 選択時） | 高 | 低 | マイグレーション・ロールバック計画を作成 |
| 同期処理の複雑化（Option A 選択時） | 中 | 中 | call_action_rules → registered_numbers の逆引きロジックをテスト |
| 評価ロジックのバグ（Option C 選択時） | 中 | 低 | 単体テスト・統合テストで網羅的に検証 |

### 8.2 制約

| 制約 | 理由 | 代替案 |
|------|------|--------|
| Option 選定は Draft → Review で確定 | ユーザー要件（番号個別設定の必要性）が不明確 | - |
| スキーマ変更は慎重に検討 | 既存データ・マイグレーション影響が大きい | Option A, B, C を優先検討 |

---

## 9. 参照

| ドキュメント | セクション | 内容 |
|-------------|-----------|------|
| [STEER-140](STEER-140_rule-evaluation-engine.md) | §5.2 | ルール評価エンジンの設計 |
| [BD-004](../design/basic/BD-004_call-routing-db.md) | §4.2 | registered_numbers テーブル定義 |
| [STEER-096](STEER-096_serversync.md) | §5.2 | Frontend 同期の設計 |

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-16 | 初版作成（Draft）、4つの Option を提示、未確定点リスト追加 | Claude Code (claude-sonnet-4-5) |
| 2026-02-16 | §5.2 推奨案を確定（段階的アプローチ: Phase 1=Option C、Phase 2=Option D）、§5.3 番号削除を論理削除+部分ユニーク制約に確定、§5.4 段階的実装計画を追加、§6 受入条件を Phase 1/Phase 2 に分離、§7 Q-01〜Q-04 回答済み、§4.2 影響するコードに migrations 追加 | Claude Code (claude-sonnet-4-5) |
| 2026-02-16 | Codex レビュー#1 指摘対応（重大1件: AC-1 の RJ→BZ 修正、中2件: 部分ユニーク制約が既存・Option C 矛盾解消、軽2件: evaluator.rs パス修正・リンク相対パス化） | Claude Code (claude-sonnet-4-5) |
| 2026-02-16 | Codex レビュー#2 指摘対応（中1件: Phase 1 スコープ記述統一、軽3件: src/migrations リンクパス修正・ストーリー文論理削除に統一） | Claude Code (claude-sonnet-4-5) |
| 2026-02-16 | Codex レビュー#3 指摘対応（軽1件: DD-009 → STEER-096 参照に変更）、判定 OK（実装進行可） | Claude Code (claude-sonnet-4-5) |
| 2026-02-16 | 承認完了、ステータス → Approved：レビューサイクル完了（Codex 3回、全指摘対応完了）、実装フェーズへ引き継ぎ準備完了 | Claude Code (claude-sonnet-4-5) |
