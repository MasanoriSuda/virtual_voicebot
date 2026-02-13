# STEER-169: ActionCode "VR" の定義不整合修正（Frontend=転送、Backend=Voicebot → 統一して B2BUA 転送へ）

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-169 |
| タイトル | ActionCode "VR" の定義不整合修正（Frontend=転送、Backend=Voicebot → 統一して B2BUA 転送へ） |
| ステータス | Review |
| 関連Issue | #169（実装修正）、#170（SoT 整理） |
| 親ステアリング | - |
| 優先度 | P0 |
| 作成日 | 2026-02-12 |

---

## 2. ストーリー（Why）

### 2.1 背景

IVR フロー実行中、ユーザーが Frontend UI で「転送(VR)」を選択しても、実際には Voicebot モードに遷移してしまう不具合が発生している。

**発生条件**:
- Frontend UI の IVR フロー設定で destination として「転送(VR)」を選択
- IVR フローを実行し、該当キーを押下

**期待動作**:
- B2BUA モード（人間オペレーターへの転送）に遷移する

**実動作**:
- Voicebot モード（AI エージェント）に遷移する

**根本原因**: **Frontend-Backend 間で ActionCode "VR" の定義が不一致**

| 観点 | Frontend | Backend | 実動作 |
|------|---------|---------|--------|
| VR の定義 | **転送**（B2BUA モード） | **Voicebot モード**（AI エージェント） | Voicebot に遷移 |
| UI 表示 | [ivr-content.tsx:1780](virtual-voicebot-frontend/components/ivr-content.tsx#L1780) - 「転送(VR)」 | - | - |
| Backend 実装 | - | [executor.rs:51](virtual-voicebot-backend/src/service/routing/executor.rs#L51) - `"executing VR (voicebot mode, ...)"`<br>[handlers/mod.rs:1148](virtual-voicebot-backend/src/protocol/session/handlers/mod.rs#L1148) - `transition_to_voicebot_mode()` | Voicebot に遷移 |

**影響**:
- ユーザーが「転送」を選択したつもりが、AI エージェントにつながる
- 人間オペレーターへの転送機能が使えない
- MVP の基本機能（人間オペレーターへの転送）が正常に動作しない

**ステアリング・ドキュメントでも定義が混在**:
- STEER-134_ivr-flow-ui.md: VR = 転送
- STEER-140_rule-evaluation-engine.md: VR = Voicebot

### 2.2 目的

ActionCode "VR" の定義を **B2BUA 転送モード** に統一し、以下を達成する：

1. **Backend の VR 実装を B2BUA 転送モードに修正する**
2. **VB = Voicebot（AI エージェント）として明確化する**
3. **Frontend UI の表示と Backend の実装を一致させる**
4. **関連ステアリング・ドキュメントの定義を統一する**

**注記**: SoT（Source of Truth）の整理は Issue #170 で別途対応する。

### 2.3 ユーザーストーリー

```
As a システム管理者
I want to IVR フローで「転送(VR)」を選択したときに人間オペレーターへ転送される
So that ユーザーが適切な対応を受けられる

受入条件:
- [ ] AC-1: IVR フローで「転送(VR)」を選択したとき、B2BUA モードで人間オペレーターへ転送される
- [ ] AC-2: VR ActionCode が B2BUA モード（録音 + 人間オペレーター転送）として動作する
- [ ] AC-3: VB ActionCode が Voicebot モード（AI エージェント）として動作する
- [ ] AC-4: Frontend UI の「転送(VR)」表示と Backend の実装が一致する
- [ ] AC-5: Backend ログに "executing VR (B2BUA mode, ...)" と出力される
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-12 |
| 起票理由 | IVR フローで「転送」を選択しても Voicebot に遷移する不具合を発見 |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Code (claude-sonnet-4-5-20250929) |
| 作成日 | 2026-02-12 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "Issue #169 のステアリングファイルを作成、A案（VR = B2BUA 転送）で進める" |

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
| docs/contract.md | 参照 | ActionCode "VR" の定義確認（Issue #170 で整理） |
| virtual-voicebot-backend/docs/steering/STEER-134_ivr-flow-ui.md | 確認 | VR = 転送の記述確認 |
| virtual-voicebot-backend/docs/steering/STEER-140_rule-evaluation-engine.md | 確認 | VR = Voicebot の記述確認 |

### 4.2 影響するコード

| モジュール | 変更種別 | 概要 |
|-----------|---------|------|
| **Backend** | | |
| virtual-voicebot-backend/src/service/routing/executor.rs | 修正 | VR を B2BUA モードに変更（`execute_vr` の実装修正） |
| virtual-voicebot-backend/src/protocol/session/handlers/mod.rs | 修正 | VR の遷移先を `transition_to_voicebot_mode()` → B2BUA モード処理に変更 |
| **Frontend** | | |
| virtual-voicebot-frontend/components/ivr-content.tsx | 確認 | 「転送(VR)」表示の確認（変更不要） |

---

## 5. 差分仕様（What / How）

### 5.1 修正方針（A案: VR = B2BUA 転送）

**定義**:
- **VR (Voice Recording / Transfer)**: B2BUA モード + 録音（人間オペレーターへの転送）
- **VB (Voicebot)**: Voicebot モード（AI エージェント）

**変更内容**:
1. Backend の VR 実装を B2BUA モードに変更
2. Frontend は変更不要（既に「転送」と表示）
3. ステアリング・ドキュメントの定義を統一（将来的に Issue #170 で対応）

### 5.2 Backend 修正

#### 5.2.1 executor.rs の修正

**ファイル**: `virtual-voicebot-backend/src/service/routing/executor.rs`

**修正箇所**: `execute_vr` 関数

**修正前** ([executor.rs:44-76](virtual-voicebot-backend/src/service/routing/executor.rs#L44-L76)):
```rust
async fn execute_vr(
    &self,
    action: &ActionConfig,
    call_id: &str,
    session: &mut SessionCoordinator,
) -> Result<()> {
    info!(
        "[ActionExecutor] call_id={} executing VR (voicebot mode, recording_enabled={})",
        call_id, action.recording_enabled
    );
    session.set_outbound_mode(false);
    session.set_recording_enabled(action.recording_enabled);
    if action.announce_enabled {
        // ... 録音告知アナウンス処理
    }
    Ok(())
}
```

**修正後**:
```rust
async fn execute_vr(
    &self,
    action: &ActionConfig,
    call_id: &str,
    session: &mut SessionCoordinator,
) -> Result<()> {
    info!(
        "[ActionExecutor] call_id={} executing VR (B2BUA transfer mode, recording_enabled={})",
        call_id, action.recording_enabled
    );
    session.set_outbound_mode(false);  // B2BUA モードでは outbound_mode=false
    session.set_recording_enabled(action.recording_enabled);

    if action.announce_enabled {
        // 録音告知アナウンス処理
        let recording_announcement_id =
            action.recording_announcement_id.or(action.announcement_id);
        session.set_announce_mode(true);
        session.set_recording_notice_pending(true);
        if let Some(announcement_id) = recording_announcement_id {
            session.set_announcement_id(announcement_id);
            info!(
                "[ActionExecutor] call_id={} recording_announcement_id={}",
                call_id, announcement_id
            );
        } else {
            info!(
                "[ActionExecutor] call_id={} recording notice uses fallback audio",
                call_id
            );
        }
    }

    // VR は B2BUA モードなので、特別な voicebot フラグは設定しない
    Ok(())
}
```

#### 5.2.2 handlers/mod.rs の修正

**ファイル**: `virtual-voicebot-backend/src/protocol/session/handlers/mod.rs`

**修正箇所**: InviteAccepted ハンドラー内の VR 処理

**現在の問題箇所** ([handlers/mod.rs:1148](virtual-voicebot-backend/src/protocol/session/handlers/mod.rs#L1148)):
```rust
"VR" => {
    self.transition_to_voicebot_mode(Some(super::VOICEBOT_INTRO_WAV_PATH.to_string()))
        .await;
}
```

**修正後**:
```rust
"VR" => {
    // VR は B2BUA モード（人間オペレーターへの転送）
    // 録音告知アナウンス処理は executor.rs で設定済み
    // 既存の AppTransferRequest 機構を使用（handlers/mod.rs:552）
    // Issue #165 で録音告知アナウンス後の AppTransferRequest 発行が実装済み
}
```

**注記**:
- **既存の B2BUA 実装を活用**: handlers/mod.rs:552 の AppTransferRequest ハンドラーが既に存在
- **録音告知アナウンス後の転送**: Issue #165 の修正で、録音告知アナウンス再生後に AppTransferRequest を発行する機構が実装済み
- **VR の動作**: executor.rs で `announce_enabled=true` の場合、録音告知アナウンス再生 → AppTransferRequest 発行 → B2BUA 転送
- **通常の転送相当**: 新規実装は不要、既存の B2BUA 転送機構を使用する

### 5.3 実装手順

1. **Backend 修正**:
   1. **executor.rs 修正**:
      - `execute_vr` 関数のログメッセージを `"executing VR (voicebot mode, ...)"` → `"executing VR (B2BUA transfer mode, ...)"` に変更
      - 既存の録音告知アナウンス処理は維持（変更不要）

   2. **handlers/mod.rs 修正**:
      - handlers/mod.rs:1148 の VR 処理から `transition_to_voicebot_mode()` 呼び出しを削除
      - コメント追加: 「VR は B2BUA モード、既存の AppTransferRequest 機構を使用」
      - 既存の AppTransferRequest ハンドラー（:552）は変更不要

   3. **B2BUA 転送機構の確認**:
      - 既存実装（handlers/mod.rs:552, b2bua.rs）が正常動作することを確認
      - Issue #165 で録音告知アナウンス後の AppTransferRequest 発行が実装済み

2. **テスト**:
   1. VR ActionCode で録音告知アナウンスが再生されることを確認
   2. VR ActionCode で Voicebot に遷移せず、B2BUA 転送されることを確認
   3. VB ActionCode で Voicebot に遷移することを確認（既存動作）

3. **ドキュメント更新**:
   1. ステアリング・ドキュメントの VR 定義を確認（Issue #170 で SoT 整理対応）

### 5.4 未確定点（Open Questions）

| ID | 質問 | 決定 | 理由 | 決定日 | 決定者 |
|----|------|------|------|--------|--------|
| Q1 | B2BUA モードの詳細実装はどこまで含めるか？ | **既存のB2BUA実装を使用** | handlers/mod.rs:552 の AppTransferRequest 機構が既に存在し、Issue #165 修正で録音告知アナウンス後の AppTransferRequest 発行も実装済み。通常の転送相当で対応可能。 | 2026-02-13 | @MasanoriSuda |
| Q2 | handlers/mod.rs の VR 処理は削除するか？ | **既存の AppTransferRequest を使用** | Q1 と同じ。handlers/mod.rs:1148 の `transition_to_voicebot_mode()` を削除し、既存の AppTransferRequest ハンドラー（:552）を活用する。 | 2026-02-13 | @MasanoriSuda |
| Q3 | Frontend の表示は変更するか？ | **A: 変更不要**（「転送(VR)」のまま） | Frontend UI は既に「転送(VR)」と表示されており、意図と一致している。Backend を Frontend 仕様に合わせる。 | 2026-02-13 | @MasanoriSuda |

### 5.5 制限事項（本チケット外）

- IVR destination 単位の `announce_enabled` は現時点で未サポート（metadata に専用フィールドなし）。
- 本チケットでは IVR destination の `VR` は「即時 B2BUA 転送」を仕様とする。
- IVR destination での「告知アナウンス付き転送」は別チケットで設計・実装する。

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #169 | STEER-169 | 起票 |
| Issue #170 | STEER-169 | SoT 整理（関連） |
| executor.rs:44-76 | STEER-169 §5.2.1 | VR 実装の修正箇所 |
| handlers/mod.rs:1148 | STEER-169 §5.2.2 | VR 遷移処理の修正箇所 |
| ivr-content.tsx:1780 | STEER-169 §2.1 | Frontend UI 表示 |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [ ] VR の定義が B2BUA 転送モードとして明確化されているか
- [ ] VB の定義が Voicebot モードとして明確化されているか
- [ ] Backend 修正の影響範囲が明確か
- [ ] 未確定点 Q1/Q2/Q3 が解消されているか
- [ ] トレーサビリティが維持されているか

### 7.2 マージ前チェック（Approved → Merged）

- [ ] Backend 修正が完了している
- [ ] VR ActionCode で Voicebot に遷移しないことを確認している
- [ ] VB ActionCode で Voicebot に遷移することを確認している
- [ ] コードレビューを受けている

---

## 8. 備考

- 本ステアリングは Issue #169（VR 定義不整合）の修正である
- SoT（contract.md）の整理は Issue #170 で別途対応する
- **既存の B2BUA 実装を活用**:
  - handlers/mod.rs:552 の AppTransferRequest ハンドラーが既に存在
  - Issue #165 で録音告知アナウンス後の AppTransferRequest 発行機構が実装済み
  - 新規実装は不要、通常の転送相当で対応可能
- Frontend UI は変更不要（既に「転送(VR)」と表示されており、意図と一致している）
- **修正範囲**:
  - executor.rs: ログメッセージの修正のみ（"voicebot mode" → "B2BUA transfer mode"）
  - handlers/mod.rs: VR 処理から `transition_to_voicebot_mode()` 削除のみ

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-12 | 初版作成（Draft） | Claude Code (claude-sonnet-4-5-20250929) |
| 2026-02-13 | 未確定点 Q1/Q2/Q3 解消（既存B2BUA実装を活用する方針に決定） | Claude Code (claude-sonnet-4-5-20250929) |
