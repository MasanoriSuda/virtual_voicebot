# STEER-166: Frontend-Backend フィールド名不一致の修正（includeAnnouncement → announceEnabled）

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-166 |
| タイトル | Frontend-Backend フィールド名不一致の修正（includeAnnouncement → announceEnabled） |
| ステータス | Approved |
| 関連Issue | #166 |
| 親ステアリング | STEER-139（Frontend-Backend 同期基盤） |
| 優先度 | P0 |
| 作成日 | 2026-02-12 |

---

## 2. ストーリー（Why）

### 2.1 背景

Issue #144（E2E 統合テスト）実施中に、VB（Voicebot）ActionCode で録音告知アナウンス有効化（`includeAnnouncement: true`）を設定してもレガシーIVR が流れる不具合が発見された。

**発生条件**:
- Frontend UI で VB/VR ActionCode に録音告知アナウンス有効化を設定
- 非通知着信または通常着信

**期待動作**:
- 録音告知アナウンス再生後、B2BUA モードまたは Voicebot モードに移行

**実動作**:
- 録音告知アナウンスが再生されず、レガシーIVR intro が流れる

**根本原因**:

Frontend と Backend で ActionConfig のフィールド名が不一致：

| 観点 | Frontend | Backend | contract.md 仕様 |
|------|---------|---------|-----------------|
| フィールド名 | `includeAnnouncement` | `announceEnabled` | `announceEnabled` |
| 実装 | call-actions.json に保存 | evaluator.rs で読み取り | - |
| 結果 | ✅ 保存成功 | ❌ 読み取り失敗（デフォルト `false`） | - |

**技術的詳細（不具合発見時点 / 修正前）**:

1. **Frontend** が JSON に `includeAnnouncement: true` を保存
   ```json
   {
     "actionCode": "VB",
     "includeAnnouncement": true,  // ← これが問題
     "recordingEnabled": true
   }
   ```

2. **Backend** の evaluator.rs が `announceEnabled` (camelCase) を期待
   ```rust
   #[derive(Debug, Deserialize)]
   #[serde(rename_all = "camelCase")]
   struct ActionConfigDto {
       #[serde(default = "default_false")]
       announce_enabled: bool,  // ← "announceEnabled" にマッピング
       #[serde(default)]
       include_announcement: Option<bool>,  // ← "includeAnnouncement" にマッピング
   }
   ```

3. **From 実装**が `include_announcement` を無視
   ```rust
   impl From<ActionConfigDto> for ActionConfig {
       fn from(dto: ActionConfigDto) -> Self {
           Self {
               announce_enabled: dto.announce_enabled,  // ← これだけ使用
               // dto.include_announcement は無視される
           }
       }
   }
   ```

4. **executor.rs** が `action.announce_enabled` のみチェック
   ```rust
   if action.announce_enabled {  // ← false なので実行されない
       session.set_announce_mode(true);
       // ...
   }
   ```

**影響**:
- VB/VR ActionCode で録音告知アナウンス機能が動作しない
- MVP の基本機能（announce_enabled フラグ対応）が正常に動作しない
- Issue #165 の修正が効かない（フィールド名不一致により `announce_enabled=false` のまま）

### 2.2 目的

Frontend-Backend 間のフィールド名を contract.md 仕様に統一し、以下を達成する：

1. **Frontend が `announceEnabled` (camelCase) を使用する**（contract.md 準拠）
2. **既存データを `announceEnabled` に移行する**
3. **Backend は用途別フィールドを維持しつつ整合させる**（`welcomeAnnouncementId` 取り込み・`VB` 遷移修正）

### 2.3 ユーザーストーリー

```
As a システム管理者
I want to Frontend で設定した「録音告知アナウンス有効化」が Backend で正しく認識される
So that VB/VR ActionCode で録音告知アナウンス機能が正常に動作する

受入条件:
- [ ] AC-1: Frontend UI で VB/VR ActionCode に「録音告知アナウンス有効化」を設定した時、JSON に `announceEnabled: true` が保存される
- [ ] AC-2: Backend が Frontend の `announceEnabled: true` を正しく読み取り、`action.announce_enabled=true` になる
- [ ] AC-3: VB ActionCode で `announceEnabled=true` の時、録音告知アナウンスが再生され、レガシーIVR が流れない
- [ ] AC-4: VR ActionCode で `announceEnabled=true` の時、録音告知アナウンスが再生され、B2BUA モードに移行する
- [ ] AC-5: 既存データ（`includeAnnouncement: true`）が `announceEnabled: true` に移行される
- [ ] AC-6: contract.md の ActionConfig 仕様と Frontend-Backend 実装が一致する
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-12 |
| 起票理由 | Issue #144 E2E テスト中に VB ActionCode で録音告知アナウンス機能が動作しない不具合を発見 |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Code (claude-sonnet-4-5-20250929) |
| 作成日 | 2026-02-12 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "Issue #166 のステアリングファイルを作成、A案（Frontend修正）で進める" |

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
| 実装日 | 2026-02-12 |
| 指示者 | @MasanoriSuda |
| 指示内容 | VB選択時に開始前アナウンス後も Voicebot 会話へ遷移するよう修正（Refs #166） |
| コードレビュー | Owner LGTM（Refs #166） |

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
| docs/contract.md | 確認 | ActionConfig 仕様が `announceEnabled` であることを再確認 |
| virtual-voicebot-frontend/README.md | 修正 | データマイグレーション手順を追記 |

### 4.2 影響するコード

| モジュール | 変更種別 | 概要 |
|-----------|---------|------|
| **Frontend** | | |
| virtual-voicebot-frontend/storage/db/call-actions.json | 修正 | `includeAnnouncement` → `announceEnabled` |
| virtual-voicebot-frontend/app/api/call-actions/route.ts | 修正 | フィールド名変更対応 |
| virtual-voicebot-frontend/lib/call-actions.ts | 修正 | ActionConfig 定義・正規化処理の更新 |
| virtual-voicebot-frontend/lib/db/call-actions.ts | 修正 | 既存データ読み替え（`includeAnnouncement` -> `announceEnabled`） |
| virtual-voicebot-frontend/components/call-actions-content.tsx | 修正 | UI フィールド更新 |
| **Backend** | | |
| virtual-voicebot-backend/src/service/routing/evaluator.rs | 修正 | `welcomeAnnouncementId` を `announcement_id` にマップ |
| virtual-voicebot-backend/src/service/routing/executor.rs | 修正 | `VB` 専用分岐追加、`voicebot_direct_mode` 設定 |
| virtual-voicebot-backend/src/protocol/session/coordinator.rs | 修正 | `voicebot_direct_mode` フラグ追加（初期化・reset） |
| virtual-voicebot-backend/src/protocol/session/handlers/mod.rs | 修正 | 開始前アナウンス後に Voicebot へ遷移 |

---

## 5. 差分仕様（What / How）

### 5.1 実装反映（2026-02-12）

- Frontend は `announceEnabled` を正として扱い、既存 `includeAnnouncement` は読み替え互換を維持。
- Backend evaluator は `welcomeAnnouncementId` を `announcement_id` にマップ。
- Backend の `include_announcement` は IVR destination metadata 用として維持（削除しない）。
- Backend executor は `VB` を `VR` から分離し、`voicebot_direct_mode` を導入。
- 初期着信時、`VB + announceEnabled=true` は「開始前アナウンス再生 -> VoicebotMode遷移」とし、legacy IVR に落ちない。

### 5.2 実装ファイル（今回の差分）

| ファイル | 変更概要 |
|---------|---------|
| `virtual-voicebot-backend/src/service/routing/executor.rs` | `VB` 専用分岐を追加し、`voicebot_direct_mode=true` を設定 |
| `virtual-voicebot-backend/src/protocol/session/coordinator.rs` | `voicebot_direct_mode` フラグ追加（初期化・reset対象） |
| `virtual-voicebot-backend/src/protocol/session/handlers/mod.rs` | 初期着信分岐に `voicebot_direct_mode` 経路追加、開始前アナウンス後に Voicebot 遷移 |
| `virtual-voicebot-backend/src/service/routing/evaluator.rs` | `welcomeAnnouncementId` -> `announcement_id` マッピング追加 |

### 5.3 検証結果

- `cargo fmt` 実行済み
- `cargo test -p virtual-voicebot-backend action_config_dto_ -- --nocapture` pass
- `cargo test -p virtual-voicebot-backend reset_action_modes_clears_voicebot_direct_mode -- --nocapture` pass

### 5.4 未確定点（更新）

| ID | 質問 | 決定 | 理由 |
|----|------|------|------|
| Q1 | `include_announcement` は削除するか？ | **B: 残す** | IVR destination metadata で使用中のため |
| Q2 | `VB + announceEnabled=true` の遷移先は？ | **Voicebot** | 開始前アナウンス後に `transition_to_voicebot_mode()` を実行 |
| Q3 | `welcomeAnnouncementId` の取り込みは必要か？ | **Yes** | Frontend 設定値を開始前アナウンス再生に反映するため |

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #166 | STEER-166 | 起票 |
| Issue #144 | Issue #166 | E2E テスト中に発見 |
| Issue #165 | Issue #166 | 根本原因の発見 |
| STEER-139 | STEER-166 | 同期基盤の設計誤り修正 |
| contract.md §ActionConfig | STEER-166 §5.2 | 仕様準拠 |
| Issue #166 | `virtual-voicebot-backend/src/service/routing/executor.rs` | `VB` 専用分岐の実装 |
| Issue #166 | `virtual-voicebot-backend/src/protocol/session/coordinator.rs` | `voicebot_direct_mode` 導入 |
| Issue #166 | `virtual-voicebot-backend/src/protocol/session/handlers/mod.rs` | 開始前アナウンス後の Voicebot 遷移 |
| Issue #166 | `virtual-voicebot-backend/src/service/routing/evaluator.rs` | `welcomeAnnouncementId` マッピング |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [x] `VB + announceEnabled=true` で legacy IVR に落ちない仕様になっている
- [x] `welcomeAnnouncementId` が Backend で受理される
- [x] `include_announcement` の扱い（削除しない）が実装と一致している
- [x] トレーサビリティが実装ファイルに接続されている

### 7.2 マージ前チェック（Approved → Merged）

- [x] Backend 修正が完了している（§5.2）
- [x] 単体テストが通過している（§5.3）
- [ ] E2E で「開始前アナウンス -> Voicebot会話」を最終確認する
- [x] コードレビュー（Owner LGTM）を受けている

---

## 8. 備考

- 本ステアリングは Issue #165（VR ActionCode バグ）の根本原因修正である
- Frontend-Backend の仕様不一致は STEER-139（同期基盤）の設計誤りに起因する
- MVP では手動データ修正で対応し、将来的には自動マイグレーション機能を検討する
- Backend の `include_announcement` は IVR destination metadata 用として維持し、`VB` の開始前アナウンス経路は `voicebot_direct_mode` で制御する

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-12 | 初版作成（Draft） | Claude Code (claude-sonnet-4-5-20250929) |
| 2026-02-12 | 未確定点 Q1/Q2/Q3 解消（全てA案採用） | Claude Code (claude-sonnet-4-5-20250929) |
