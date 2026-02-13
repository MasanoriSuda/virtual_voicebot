# STEER-165: VR ActionCode の announce_enabled 時の IVR intro 誤再生バグ修正

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-165 |
| タイトル | VR ActionCode の announce_enabled 時の IVR intro 誤再生バグ修正 |
| ステータス | Draft |
| 関連Issue | #165 |
| 優先度 | P0 |
| 作成日 | 2026-02-12 |

---

## 2. ストーリー（Why）

### 2.1 背景

STEER-143（Backend 録音実装強化）で VR ActionCode の `announce_enabled` フラグ機能が実装されたが、**非通知着信時**の実動作が仕様と異なる不具合が発見された。

**発生条件**:
- Frontend UI で `anonymousAction`（非通知着信アクション）に VR（録音告知アナウンス + 録音あり）を設定
- 非通知番号から着信

**期待動作**:
1. 録音告知アナウンス再生（「この通話は録音されます」）
2. B2BUA モードに移行（発信者と受信者を直接接続 = レガシーIVRの3押下相当）
3. 通話録音

**実動作**:
1. 録音告知アナウンス再生（正常に再生される）
2. **レガシーIVR intro（zundamon_intro_ivr.wav）が再生される** ← バグ
3. 通話録音（録音自体は正常）

**影響**:
- ユーザーが非通知着信に対して「録音告知アナウンス + 録音あり」を設定しても、録音告知アナウンスの後にレガシーIVR intro が再生されてしまい、B2BUA モードに正常に移行しない
- MVP の基本機能（VR + announce_enabled）が正常に動作しない
- 発信者は人間のオペレーターと話すことができず、IVR メニューに誘導されてしまう
- 録音告知アナウンスと通話録音は正常に動作しているため、問題は**アナウンス再生後の遷移処理**にある

### 2.2 根本原因

#### 2.2.1 STEER-143 仕様と実装の乖離

**STEER-143 §5.3.2 の仕様**:

```rust
async fn execute_vr(&self, action: &ActionConfig, call_id: &str, session: &mut SessionCoordinator) -> Result<()> {
    session.set_outbound_mode(false);
    session.set_recording_enabled(action.recording_enabled);

    // announce_enabled=true の場合のみ告知アナウンスモードに遷移
    if action.announce_enabled {
        session.set_announce_mode(true);  // ← 仕様ではこれを呼ぶ

        if let Some(recording_announcement_id) = action.recording_announcement_id {
            session.set_announcement_id(recording_announcement_id);
        }
    }
}
```

**実際の実装** ([executor.rs:55-71](virtual-voicebot-backend/src/service/routing/executor.rs#L55-L71)):

```rust
if action.announce_enabled {
    session.set_recording_notice_pending(true);  // ← 実装はこれのみ
    if let Some(announcement_id) = recording_announcement_id {
        session.set_announcement_id(announcement_id);
    }
}
```

#### 2.2.2 バグの発生フロー

1. VR + announce_enabled=true → executor が `recording_notice_pending=true` のみ設定（`announce_mode` は false のまま）
2. [handlers/mod.rs:265-304](virtual-voicebot-backend/src/protocol/session/handlers/mod.rs#L265-L304) のイベントハンドラで：
   ```rust
   if !self.outbound_mode {
       if self.announce_mode {  // false なので通過しない
           // アナウンス再生パス
       } else {
           if let Some(ivr_flow_id) = self.ivr_flow_id {
               // DB IVR 起動
           } else {
               self.start_legacy_ivr_menu().await;  // ← ここが呼ばれる
           }
       }
   }
   ```
3. `start_legacy_ivr_menu()` ([handlers/mod.rs:837-876](virtual-voicebot-backend/src/protocol/session/handlers/mod.rs#L837-L876)) が呼ばれる：
   ```rust
   let mut playback_paths: Vec<String> = Vec::with_capacity(2);
   if self.recording_notice_pending {
       // 録音告知アナウンスを playback_paths に追加
       playback_paths.push(recording_notice_path);
   }
   playback_paths.push(super::IVR_INTRO_WAV_PATH.to_string());  // ← 常に追加
   ```
4. 結果：録音告知アナウンス + IVR intro の両方が再生される

### 2.3 目的

VR ActionCode の `announce_enabled=true` 時の動作を STEER-143 の仕様通りに修正し、以下を達成する：

1. **録音告知アナウンス再生後、B2BUA モードに正常に移行する**（レガシーIVR intro を再生しない）
2. **STEER-143 の仕様と実装を整合させる**
3. **VR と IVR の処理パスを明確に分離する**（VR の録音告知で IVR が起動しないようにする）

**用語定義**:
- **VR (Voice Recording)**: B2BUA モード + 録音。発信者と受信者（人間）を直接接続し、通話を録音する
- **VB (Voicebot)**: AI音声エージェント（別の ActionCode、本ステアリングのスコープ外）
- **B2BUA モード**: Back-to-Back User Agent。発信者と受信者を SIP プロトコルレベルで中継接続するモード

### 2.4 ユーザーストーリー

```
As a システム管理者
I want to VR ActionCode で「録音告知アナウンス + 録音あり」を設定した時、録音告知アナウンス再生後に B2BUA モードに移行する
So that 通話相手に録音を告知した上で、発信者と受信者（人間）を直接接続して通常の通話を行える

受入条件:
- [ ] AC-1: VR + announce_enabled=true + recording_enabled=true の着信で、録音告知アナウンスが再生される
- [ ] AC-2: 録音告知アナウンス再生後、レガシーIVR intro が再生されない
- [ ] AC-3: 録音告知アナウンス再生後、B2BUA モードに移行する（発信者と受信者を直接接続 = レガシーIVRの3押下相当）
- [ ] AC-4: 通話が録音される（mixed.wav が生成される）
- [ ] AC-5: VR + announce_enabled=false + recording_enabled=true の着信では、録音告知アナウンスなしで即座に B2BUA モードに移行する
- [ ] AC-6: IVR ActionCode（IV）の動作に影響しない（既存の IVR 機能が正常に動作する）
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-12 |
| 起票理由 | VR + announce_enabled=true 時にレガシーIVR intro が誤再生される不具合の報告 |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Code (claude-sonnet-4-5-20250929) |
| 作成日 | 2026-02-12 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "Issue #165 の根本原因調査と修正仕様の作成" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| - | - | - | - | |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | - |
| 承認日 | - |
| 承認コメント | |

### 3.5 実装（該当する場合）

| 項目 | 値 |
|------|-----|
| 実装者 | Codex へ引き継ぎ |
| 実装日 | - |
| 指示者 | - |
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
| virtual-voicebot-backend/docs/steering/STEER-143_recording-enhancement.md | 参照 | 本バグは STEER-143 の実装不具合。仕様は正しいため修正不要 |
| virtual-voicebot-backend/docs/requirements/RD-004_call-routing-execution.md | 参照 | FR-3.x（録音要件）の動作確認 |

### 4.2 影響するコード

| モジュール | 変更種別 | 概要 |
|-----------|---------|------|
| src/service/routing/executor.rs | 修正 | execute_vr() で announce_enabled=true 時に announce_mode を設定する |
| src/protocol/session/handlers/mod.rs | 修正 | announce_mode 時のアナウンス再生後の処理を修正（IVR ではなく Voicebot に移行） |
| src/protocol/session/handlers/mod.rs | 確認 | start_legacy_ivr_menu() の recording_notice_pending 処理が不要になる可能性（削除検討） |

---

## 5. 差分仕様（What / How）

### 5.1 修正方針

#### 5.1.1 修正の基本方針

**選択肢**:

| 案 | 内容 | メリット | デメリット |
|---|------|---------|----------|
| **A案（採用）** | executor.rs で `set_announce_mode(true)` を設定する（STEER-143 仕様に合わせる） | STEER-143 仕様との整合性、最小修正 | announce_mode の挙動を VR と AN で分岐する必要がある可能性 |
| B案 | VR 専用の録音告知モード（`recording_notice_mode`）を新設する | VR と IVR/AN が明確に分離される | モードの追加によるコード複雑化 |
| C案 | start_legacy_ivr_menu() を録音告知処理から分離する | 既存の announce_mode に影響しない | 根本的な仕様乖離が残る |

**採用**：**A案** - STEER-143 仕様に実装を合わせる

**理由**：
- STEER-143 で既に `set_announce_mode(true)` の使用が仕様として定義されている
- 最小限の修正で仕様との整合性を取れる
- announce_mode の後続処理（アナウンス再生後の遷移先）を VR 用に調整すればよい

#### 5.1.2 修正ポイント

| ファイル | 行 | 現状 | 修正後 |
|---------|-----|------|--------|
| executor.rs | 55-71 | `set_recording_notice_pending(true)` のみ | `set_announce_mode(true)` + `set_recording_notice_pending(true)` |
| handlers/mod.rs | 266-291 | announce_mode 時のアナウンス再生後の処理が未定義 | アナウンス再生後、voicemail_mode/recording_notice_pending の状態に応じて遷移先を決定 |
| handlers/mod.rs | 837-876 | start_legacy_ivr_menu() で recording_notice_pending を処理 | recording_notice_pending 処理を削除（announce_mode に統合） |

### 5.2 詳細設計修正（DD-004 へマージ）

#### 5.2.1 executor.rs - execute_vr() の修正

**現状**:

```rust
// src/service/routing/executor.rs:55-71
if action.announce_enabled {
    let recording_announcement_id =
        action.recording_announcement_id.or(action.announcement_id);
    session.set_recording_notice_pending(true);
    if let Some(announcement_id) = recording_announcement_id {
        session.set_announcement_id(announcement_id);
        // ...
    }
}
```

**修正後**:

```rust
// src/service/routing/executor.rs:55-71
if action.announce_enabled {
    let recording_announcement_id =
        action.recording_announcement_id.or(action.announcement_id);

    // STEER-143 仕様通り announce_mode を設定
    session.set_announce_mode(true);

    // VR の録音告知フラグも設定（アナウンス再生後の遷移判定に使用）
    session.set_recording_notice_pending(true);

    if let Some(announcement_id) = recording_announcement_id {
        session.set_announcement_id(announcement_id);
        // ...
    }
}
```

**変更理由**:
- STEER-143 §5.3.2 の仕様に実装を合わせる
- `announce_mode=true` にすることで、handlers/mod.rs:266 の分岐でアナウンス再生パスに入る
- `recording_notice_pending` は VR の録音告知であることを示すフラグとして残す（アナウンス再生後の遷移判定に使用）

#### 5.2.2 handlers/mod.rs - アナウンス再生後の処理修正

**現状**:

```rust
// handlers/mod.rs:266-291
if self.announce_mode {
    self.ivr_state = IvrState::Transferring;
    let announcement_path = self
        .resolve_announcement_playback_path()
        .await
        .unwrap_or_else(|| super::ANNOUNCEMENT_FALLBACK_WAV_PATH.to_string());
    if self.voicemail_mode {
        // VM の処理
    } else {
        // AN の処理（アナウンス再生のみ）
    }
    if let Err(e) = self.start_playback(&[announcement_path.as_str()]).await {
        // エラー処理
    }
}
```

**修正後**:

```rust
// handlers/mod.rs:266-291
if self.announce_mode {
    self.ivr_state = IvrState::Transferring;
    let announcement_path = self
        .resolve_announcement_playback_path()
        .await
        .unwrap_or_else(|| super::ANNOUNCEMENT_FALLBACK_WAV_PATH.to_string());

    if self.voicemail_mode {
        info!("[session {}] playing voicemail announcement path={}", self.call_id, announcement_path);
    } else if self.recording_notice_pending {
        // VR の録音告知アナウンス
        info!("[session {}] playing recording notice announcement path={}", self.call_id, announcement_path);
    } else {
        // AN の通常アナウンス
        info!("[session {}] playing announcement path={}", self.call_id, announcement_path);
    }

    if let Err(e) = self.start_playback(&[announcement_path.as_str()]).await {
        warn!("[session {}] failed to play announcement: {:?}", self.call_id, e);
        if !self.voicemail_mode && !self.recording_notice_pending {
            // AN の場合のみ切断（VR/VM は継続）
            let _ = self.control_tx.try_send(SessionControlIn::AppHangup);
        }
    }
}
```

**アナウンス再生完了後の処理**:

アナウンス再生完了イベント（PlaybackComplete）のハンドラで、遷移先を決定：

```rust
// handlers/mod.rs - PlaybackComplete ハンドラ（疑似コード）
(_, SessionControlIn::PlaybackComplete) => {
    if self.announce_mode {
        self.announce_mode = false;

        if self.voicemail_mode {
            // VM: 録音開始
            self.start_voicemail_recording();
        } else if self.recording_notice_pending {
            // VR: 録音告知完了 → B2BUA モードに移行
            self.recording_notice_pending = false;
            // B2BUA モード開始（レガシーIVR の3押下相当）
            // 既存の B2BUA確立処理に進む（IVR intro は再生しない）
        } else {
            // AN: アナウンス完了 → 切断
            let _ = self.control_tx.try_send(SessionControlIn::AppHangup);
        }
    }
}
```

**変更理由**:
- VR の録音告知アナウンス再生後、B2BUA モードに移行する（IVR intro を再生しない）
- `recording_notice_pending` フラグで VR の録音告知と AN/VM を区別する
- AN はアナウンス再生後に切断、VM はアナウンス再生後に録音開始、VR はアナウンス再生後に Voicebot モード開始という遷移を明確にする

#### 5.2.3 start_legacy_ivr_menu() の修正

**現状**:

```rust
// handlers/mod.rs:837-876
async fn start_legacy_ivr_menu(&mut self) {
    // ...
    let mut playback_paths: Vec<String> = Vec::with_capacity(2);
    if self.recording_notice_pending {
        // 録音告知アナウンスを prepend
        playback_paths.push(recording_notice_path);
        self.recording_notice_pending = false;
    }
    playback_paths.push(super::IVR_INTRO_WAV_PATH.to_string());
    // ...
}
```

**修正後**:

```rust
// handlers/mod.rs:837-876
async fn start_legacy_ivr_menu(&mut self) {
    // ...
    // recording_notice_pending 処理を削除
    // （announce_mode で既に処理されるため）

    let playback_path = super::IVR_INTRO_WAV_PATH.to_string();
    if let Err(err) = self.start_playback(&[playback_path.as_str()]).await {
        // エラー処理
    }
}
```

**変更理由**:
- VR の録音告知は announce_mode で処理されるため、start_legacy_ivr_menu() での recording_notice_pending 処理は不要
- start_legacy_ivr_menu() は純粋に「レガシーIVR を開始する」機能に特化する

### 5.3 動作フロー

#### 5.3.1 VR + announce_enabled=true の場合

```
1. 着信
   ↓
2. ルール評価 → VR (announce_enabled=true, recording_enabled=true)
   ↓
3. executor.execute_vr()
   - set_announce_mode(true)
   - set_recording_notice_pending(true)
   - set_recording_enabled(true)
   - set_announcement_id(...)
   ↓
4. handlers/mod.rs - InviteAccepted
   - announce_mode=true → アナウンス再生パスに入る
   - recording_notice_pending=true → VR の録音告知と認識
   - 録音告知アナウンス再生
   ↓
5. handlers/mod.rs - PlaybackComplete
   - announce_mode=false
   - recording_notice_pending=false
   - B2BUA モード開始（IVR intro は再生しない）
   ↓
6. 通話開始（録音あり、発信者と受信者を直接接続）
```

#### 5.3.2 VR + announce_enabled=false の場合

```
1. 着信
   ↓
2. ルール評価 → VR (announce_enabled=false, recording_enabled=true)
   ↓
3. executor.execute_vr()
   - set_recording_enabled(true)
   （announce_mode=false のまま）
   ↓
4. handlers/mod.rs - InviteAccepted
   - announce_mode=false → アナウンスなし
   - 即座に B2BUA モード開始
   ↓
5. 通話開始（録音あり、発信者と受信者を直接接続）
```

#### 5.3.3 IV（IVR ActionCode）の場合（既存動作を維持）

```
1. 着信
   ↓
2. ルール評価 → IV (ivr_flow_id=...)
   ↓
3. executor.execute_iv()
   - set_ivr_flow_id(...)
   ↓
4. handlers/mod.rs - InviteAccepted
   - announce_mode=false
   - ivr_flow_id != None → enter_db_ivr_flow() または start_legacy_ivr_menu()
   ↓
5. IVR intro 再生 → IVR メニュー待機
```

### 5.4 未確定点（Open Questions）

| ID | 質問 | 選択肢 | 推奨 | 理由 |
|----|------|--------|------|------|
| Q1 | PlaybackComplete ハンドラは既存の処理に追加するか、新規作成するか？ | A: 既存に追加<br>B: 新規作成 | A | 既存のアナウンス完了処理に VR 分岐を追加する方が整合性が高い |
| Q2 | start_legacy_ivr_menu() の recording_notice_pending 処理は削除するか、コメントアウトするか？ | A: 削除<br>B: コメントアウト | A | announce_mode で処理されるため完全に不要 |
| Q3 | VR 録音告知後の B2BUA モード開始処理は、既存の B2BUA確立処理を流用するか？ | A: 流用<br>B: 新規作成 | A | 「レガシーIVRの3押下相当」は既存の B2BUA確立処理と同じ |

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #165 | STEER-165 | 起票 |
| STEER-143 §5.3.2 | STEER-165 | 仕様定義 → 実装バグ修正 |
| STEER-165 | executor.rs | 修正実装 |
| STEER-165 | handlers/mod.rs | 修正実装 |
| RD-004 FR-3.1〜3.3 | STEER-165 | 録音要件 → バグ修正 |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [ ] 根本原因の分析が正しいか
- [ ] 修正方針（A案）が妥当か
- [ ] STEER-143 の仕様との整合性が取れているか
- [ ] VR/AN/VM/IV の各 ActionCode の動作に影響しないか
- [ ] 受入条件（AC-1〜6）が網羅的か
- [ ] 未確定点（Q1〜Q3）の推奨案が妥当か

### 7.2 マージ前チェック（Approved → Merged）

- [ ] 実装が完了している
- [ ] コードレビューを受けている
- [ ] VR + announce_enabled=true のテストが PASS
- [ ] VR + announce_enabled=false のテストが PASS
- [ ] IVR ActionCode（IV）のテストが PASS（既存動作が維持されている）
- [ ] AN/VM ActionCode のテストが PASS（既存動作が維持されている）

---

## 8. 備考

### 8.1 関連する既知の問題

なし（本バグが唯一の既知の問題）

### 8.2 テスト時の注意事項

- VR + announce_enabled=true のテスト時、録音告知アナウンス再生後に IVR intro が再生されないことを確認する
- 既存の IVR ActionCode（IV）の動作に影響しないことを確認する（IVR intro が正常に再生されること）
- AN/VM ActionCode の動作に影響しないことを確認する

### 8.3 リスクとロールバック観点

**リスク**:
- announce_mode の変更により、AN/VM ActionCode の動作に影響する可能性
- PlaybackComplete ハンドラの修正により、既存のアナウンス完了後の処理に影響する可能性

**リスク軽減策**:
- AN/VM/IV の各 ActionCode の統合テストを実施する
- announce_mode の分岐処理を慎重に実装し、VR/AN/VM を明確に区別する

**ロールバック手順**:
1. executor.rs の `set_announce_mode(true)` 呼び出しを削除
2. handlers/mod.rs の announce_mode 分岐処理を元に戻す
3. start_legacy_ivr_menu() の recording_notice_pending 処理を復元

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-12 | 初版作成（Draft） | Claude Code (claude-sonnet-4-5-20250929) |
| 2026-02-12 | 用語修正: VR の定義を明確化（Voicebot モード → B2BUA モード）、VB（Voicebot）との区別を追記 | Claude Code (claude-sonnet-4-5-20250929) |
| 2026-02-12 | 背景修正: 「非通知着信時の挙動」を明確化、anonymousAction 設定の前提を追記、アナウンスと録音は正常動作していることを明記 | Claude Code (claude-sonnet-4-5-20250929) |
