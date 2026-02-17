# STEER-141: Backend 全 ActionCode 実装（Phase 3）

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-141 |
| タイトル | Backend 全 ActionCode 実装（Phase 3: IV/VM/BZ/NR/AN） |
| ステータス | Approved |
| 関連Issue | #141 |
| 優先度 | P0 |
| 作成日 | 2026-02-08 |
| 親ステアリング | STEER-137 |

---

## 2. ストーリー（Why）

### 2.1 背景

Issue #140（STEER-140）で Phase 2 が完了し、**VR（Voicebot+Record）** のみが実装された。

**Phase 2 の成果**:
- 3段階ルール評価エンジン（番号完全一致 → 番号グループ → カテゴリ → デフォルト）
- VR ActionCode 実行（voicebot 通常処理 + recording_enabled フラグ対応）
- Frontend で設定したルールが実際の着信に反映される

**問題**:
- **Phase 2 では VR のみ対応**しているため、他の ActionCode（BZ, NR, AN, VM, IV）を設定しても動作しない
- Frontend で設定できる ActionCode の大部分が使えない状態
- エンドユーザーが期待する着信制御（話中応答、応答なし、アナウンス再生、留守番電話）ができない

**影響**:
- Frontend PoC の価値が部分的にしか発揮されない
- Phase 4（IVR 実行エンジン）に進めない
- 実用的な着信制御が実現できない

### 2.2 目的

Backend の ActionCode 実行ロジックを拡張し、**VR 以外の全 ActionCode**（IV, VM, BZ, NR, AN）を実装する。

**達成目標**:
- BZ（話中応答）、NR（応答なし）、AN（アナウンス再生）、VM（留守番電話）が正しく実行される
- IV（IVR フローへ移行）が実装される（Phase 4 で IVR 実行エンジンを実装）
- Frontend で設定した全ての ActionCode が実際の着信で動作する

### 2.3 ユーザーストーリー

```
As a システム管理者
I want to Frontend で設定した全ての ActionCode が実際の通話に反映される
So that 話中応答・応答なし・アナウンス再生・留守番電話など、多様な着信制御ができる

受入条件:
- [ ] Frontend で ActionCode=BZ（話中応答）を設定すると、その番号からの着信が話中音で応答される
- [ ] Frontend で ActionCode=NR（応答なし）を設定すると、その番号からの着信が応答されない
- [ ] Frontend で ActionCode=AN（アナウンス再生）を設定すると、その番号からの着信でアナウンスが再生される
- [ ] Frontend で ActionCode=VM（留守番電話）を設定すると、その番号からの着信で留守番電話が動作する
- [ ] Frontend で ActionCode=IV（IVR）を設定すると、その番号からの着信が IVR フローに遷移する（Phase 4 で実行）
- [ ] ログ出力に call_id と ActionCode が含まれる
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-08 |
| 起票理由 | Issue #140 完了後、STEER-137 の Phase 3 実装を開始 |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Code (claude-sonnet-4-5) |
| 作成日 | 2026-02-08 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "Issue #141 のステアリングファイルを作成" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| 1 | Codex | 2026-02-08 | 要修正 | 重大4件（NR early return リスク、BZ enum 不一致、announcement_id 経路未定義、IvrState 不存在）、軽1件（mod.rs 未記載）→ 全て修正完了 |
| 2 | Codex | 2026-02-08 | 要修正 | 重大1件（NR RingDurationElapsed 未対応）、中2件（SessionOut 名不一致、影響範囲ずれ）、軽1件（BZ terminate 未定義）→ 全て修正完了 |
| 3 | Codex | 2026-02-08 | 要修正 | 中1件（BZ SipInvite 後続処理スキップ不足）→ 修正完了 |
| 4 | Codex | 2026-02-08 | 承認 | 指摘なし（実装可能） |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | @MasanoriSuda |
| 承認日 | 2026-02-08 |
| 承認コメント | Codex レビュー 4回実施、全指摘対応完了（重大5件、中4件、軽3件）。実装フェーズへ |

### 3.5 実装（該当する場合）

| 項目 | 値 |
|------|-----|
| 実装者 | Codex |
| 実装開始日 | - |
| 実装完了日 | - |
| PR番号 | - |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ日 | - |
| マージ先 | - |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| RD-004 | 参照 | FR-2（ActionCode 実行）を参照 |
| contract.md | 参照 | ActionCode の仕様を参照 |

### 4.2 影響するコード

| モジュール | 変更種別 | 概要 |
|-----------|---------|------|
| src/service/routing/executor.rs | 修正 | BZ/NR/AN/VM/IV の ActionCode 実行ロジックを追加 |
| src/service/routing/evaluator.rs | 修正 | ActionConfigDto/ActionConfig に announcement_id フィールドを追加 |
| src/protocol/session/coordinator.rs | 修正 | NR/AN/VM モード対応フィールド（no_response_mode, announce_mode, voicemail_mode, announcement_id）、アナウンス ID/IVR フロー ID 設定メソッド（set_no_response_mode, set_announce_mode, set_announcement_id, set_voicemail_mode, set_ivr_flow_id, send_sip_error）を追加 |
| src/protocol/session/handlers/mod.rs | 修正 | handle_control_event で NR モード時の SIP 応答抑止ロジック（SipInvite ブランチ + RingDurationElapsed ブランチ）を追加 |

---

## 5. 差分仕様（What / How）

### 5.1 ActionCode 実装方針

**Phase 2 で実装済み**:
- VR（Voicebot+Record）: voicebot 通常処理（`outbound_mode=false`）+ `recording_enabled` フラグ対応
- VB（Voicebot）: VR と同じ（`recording_enabled=false` で呼ばれる）

**Phase 3 で実装**:

| ActionCode | 名称 | 実装方針 |
|-----------|------|---------|
| **BZ** | 話中応答（Busy） | SIP 486 Busy Here 応答、通話を確立しない |
| **NR** | 応答なし（No Response） | SIP 応答を返さない（タイムアウト待ち） |
| **AN** | アナウンス再生（Announcement） | アナウンス音声を再生後、切断 |
| **VM** | 留守番電話（Voicemail） | アナウンス再生 + 音声録音 + 保存 |
| **IV** | IVR フローへ移行 | IVR 実行エンジンに制御を渡す（Phase 4 で実行エンジン実装） |

**Phase 4 で実装予定**:
- IVR 実行エンジン（DTMF 入力待ち、タイムアウト、リトライ、fallback）

---

### 5.1.1 ActionConfig 型定義の拡張

**既存（Phase 2）**:
```rust
#[derive(Debug, Clone)]
pub struct ActionConfig {
    pub action_code: String,
    pub ivr_flow_id: Option<Uuid>,
    pub recording_enabled: bool,
    pub announce_enabled: bool,
}
```

**Phase 3 拡張**:
```rust
#[derive(Debug, Clone)]
pub struct ActionConfig {
    pub action_code: String,
    pub ivr_flow_id: Option<Uuid>,
    pub recording_enabled: bool,
    pub announce_enabled: bool,
    pub announcement_id: Option<Uuid>,  // 新規: アナウンス ID（AN/VM で使用）
}
```

**ActionConfigDto（DB から取得）**:
```rust
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionConfigDto {
    pub action_code: String,
    #[serde(default)]
    pub ivr_flow_id: Option<Uuid>,
    #[serde(default)]
    pub recording_enabled: bool,
    #[serde(default)]
    pub announce_enabled: bool,
    #[serde(default)]
    pub announcement_id: Option<Uuid>,  // 新規: アナウンス ID
}
```

**evaluator.rs での取り込み経路**:
```rust
// evaluator.rs の RuleEvaluator::evaluate() 内で ActionConfigDto を取得
let action_config_dto = /* DB から取得（registered_numbers.action_config など） */;

// ActionConfigDto → ActionConfig 変換
let action_config = ActionConfig {
    action_code: action_config_dto.action_code,
    ivr_flow_id: action_config_dto.ivr_flow_id,
    recording_enabled: action_config_dto.recording_enabled,
    announce_enabled: action_config_dto.announce_enabled,
    announcement_id: action_config_dto.announcement_id,  // 新規
};
```

---

### 5.2 各 ActionCode の詳細仕様

#### 5.2.1 BZ（話中応答 / Busy）

**RD-004 定義**:
- BZ = 「話中応答（486 Busy Here）」

**実装**:
```rust
async fn execute_bz(&self, action: &ActionConfig, call_id: &str, session: &mut SessionCoordinator) -> Result<()> {
    info!("[ActionExecutor] call_id={} Executing BZ (Busy)", call_id);

    // 1. 通話を確立しないため、invite_rejected を設定（SipInvite 後続処理をスキップ）
    session.set_invite_rejected(true);

    // 2. SIP 486 Busy Here 応答（既存の SessionOut::SipSendError を使用）
    session.send_sip_error(486, "Busy Here").await?;

    Ok(())
}
```

**SessionCoordinator への追加メソッド**:
```rust
pub async fn send_sip_error(&mut self, code: u16, reason: &str) -> Result<()> {
    self.session_out_tx.try_send((
        self.call_id.clone(),
        SessionOut::SipSendError {
            code,
            reason: reason.to_string(),
        }
    ))?;
    Ok(())
}

pub fn set_invite_rejected(&mut self, rejected: bool) {
    self.invite_rejected = rejected;
}
```

**handle_control_event での処理**:
```rust
// handle_control_event の SessionControlIn::SipInvite ブランチ内で、
// invite_rejected=true の場合、outbound 起動・100/180 送信・pending_answer 設定をスキップ

match control_in {
    SessionControlIn::SipInvite { offer, session_timer } => {
        // ... 既存の処理（peer_sdp 設定、answer 生成など）...

        // BZ (Busy) の場合、ルール評価エンジンが invite_rejected=true を設定済み
        // この場合、outbound 起動や SIP 応答（100/180）をスキップし、終了を促す
        if self.invite_rejected {
            info!("[SessionCoordinator] call_id={} invite_rejected=true, skipping SIP responses", self.call_id);
            self.pending_answer = None;  // pending_answer をクリア
            return false;  // 状態遷移しない（セッション終了を促す）
        }

        // ... 既存の処理（outbound 起動判定、100 Trying 送信、180 Ringing 送信など）...
    }
    // ... 他のブランチ ...
}
```

**注**: 既存の `SessionOut::SipSendError` enum variant を使用するため、新規 enum variant の追加は不要。BZ は `invite_rejected=true` を設定することで、SipInvite ブランチの後続処理（outbound 起動、100/180 送信、pending_answer 設定）をスキップし、セッション終了を制御する。

**ログ出力例**:
```
[ActionExecutor] call_id=019503a0-1234-7000-8000-000000000001 action_code=BZ
[ActionExecutor] call_id=019503a0-1234-7000-8000-000000000001 Executing BZ (Busy)
[ActionExecutor] call_id=019503a0-1234-7000-8000-000000000001 Sent 486 Busy Here
```

---

#### 5.2.2 NR（応答なし / No Response）

**RD-004 定義**:
- NR = 「応答なし（タイムアウトまで無視）」

**実装**:
```rust
async fn execute_nr(&self, action: &ActionConfig, call_id: &str, session: &mut SessionCoordinator) -> Result<()> {
    info!("[ActionExecutor] call_id={} Executing NR (No Response)", call_id);

    // 1. セッションを "no_response" モードに設定
    // SessionCoordinator は INVITE に対して応答を返さず、タイムアウトまで待つ
    session.set_no_response_mode(true);

    // 2. outbound_mode = false（セッション自体は継続し、終了処理を妨げない）
    session.set_outbound_mode(false);

    Ok(())
}
```

**SessionCoordinator への追加フィールド・メソッド**:
```rust
pub struct SessionCoordinator {
    // ... 既存フィールド ...
    no_response_mode: bool,  // 新規: NR モード（SIP 応答を返さない）
}

impl SessionCoordinator {
    pub fn set_no_response_mode(&mut self, enabled: bool) {
        self.no_response_mode = enabled;
    }
}
```

**handle_control_event での処理**:
```rust
// handle_control_event の SessionControlIn::SipInvite ブランチ内で、
// NR モードの場合、100 Trying / 180 Ringing の送信部分をスキップ
// 200 OK は RingDurationElapsed で送信されるため、そちらでも抑止

match control_in {
    SessionControlIn::SipInvite { .. } => {
        // ... 既存の処理 ...

        // 100 Trying 送信
        if !self.no_response_mode {
            self.session_out_tx.try_send((self.call_id.clone(), SessionOut::SipSend100))?;
        } else {
            info!("[SessionCoordinator] call_id={} NR mode: skipping 100 Trying", self.call_id);
        }

        // ... 既存の処理 ...

        // 180 Ringing 送信
        if !self.no_response_mode {
            self.session_out_tx.try_send((self.call_id.clone(), SessionOut::SipSend180))?;
        } else {
            info!("[SessionCoordinator] call_id={} NR mode: skipping 180 Ringing", self.call_id);
        }

        // ... 既存の処理（pending_answer 設定など）は継続 ...
        // 注: 200 OK は RingDurationElapsed で送信される
    }

    // RingDurationElapsed での 200 OK 抑止
    SessionControlIn::RingDurationElapsed => {
        if self.outbound_mode || self.invite_rejected {
            return false;
        }

        // NR モードの場合、200 OK を送信しない
        if self.no_response_mode {
            info!("[SessionCoordinator] call_id={} NR mode: skipping 200 OK (RingDurationElapsed)", self.call_id);
            self.pending_answer = None;  // pending_answer をクリア
            return false;
        }

        // 通常モードの場合、200 OK を送信
        if let Some(answer) = self.pending_answer.take() {
            self.session_out_tx.try_send((self.call_id.clone(), SessionOut::SipSend200 { answer }))?;
        }
    }

    // ... 他のブランチ ...
}
```

**重要**: NR モードでも `return` で早期終了せず、セッション終了処理（BYE 送信、録音停止等）は正常に実行されるようにする。ただし、RingDurationElapsed では 200 OK 送信を抑止し、pending_answer をクリアする。

**ログ出力例**:
```
[ActionExecutor] call_id=019503a0-1234-7000-8000-000000000001 action_code=NR
[ActionExecutor] call_id=019503a0-1234-7000-8000-000000000001 Executing NR (No Response)
[SessionCoordinator] call_id=019503a0-1234-7000-8000-000000000001 NR mode: skipping SIP responses
```

---

#### 5.2.3 AN（アナウンス再生 / Announcement）

**RD-004 定義**:
- AN = 「アナウンス再生後、切断」

**実装**:
```rust
async fn execute_an(&self, action: &ActionConfig, call_id: &str, session: &mut SessionCoordinator) -> Result<()> {
    info!("[ActionExecutor] call_id={} Executing AN (Announcement)", call_id);

    // 1. outbound_mode = false（通話を確立）
    session.set_outbound_mode(false);

    // 2. アナウンス再生フラグを設定
    if action.announce_enabled {
        session.set_announce_mode(true);
        if let Some(announcement_id) = &action.announcement_id {
            session.set_announcement_id(announcement_id.clone());
            info!("[ActionExecutor] call_id={} Announcement: {}", call_id, announcement_id);
        } else {
            warn!("[ActionExecutor] call_id={} announce_enabled=true but announcement_id is None", call_id);
        }
    }

    // 3. 録音は無効
    session.set_recording_enabled(false);

    Ok(())
}
```

**注**: `ActionConfig` 型定義の拡張は §5.1.1 を参照。

**SessionCoordinator への追加フィールド・メソッド**:
```rust
pub struct SessionCoordinator {
    // ... 既存フィールド ...
    announce_mode: bool,  // 新規: AN モード（アナウンス再生後切断）
    announcement_id: Option<Uuid>,  // 新規: 再生するアナウンス ID
}

impl SessionCoordinator {
    pub fn set_announce_mode(&mut self, enabled: bool) {
        self.announce_mode = enabled;
    }

    pub fn set_announcement_id(&mut self, id: Uuid) {
        self.announcement_id = Some(id);
    }
}
```

**アナウンス再生ロジック（playback との統合）**:
```rust
// SessionCoordinator::tick() 内で、announce_mode=true の場合
if self.announce_mode && self.announcement_id.is_some() {
    // 1. アナウンス音声ファイルを読み込み
    let announcement_path = self.resolve_announcement_path(&self.announcement_id.unwrap())?;

    // 2. playback_service で再生
    self.start_playback(&announcement_path).await?;

    // 3. 再生完了後、切断
    // （playback_service が完了を通知するまで待機、完了後に SessionOut::SipBye を送信）
}
```

**ログ出力例**:
```
[ActionExecutor] call_id=019503a0-1234-7000-8000-000000000001 action_code=AN
[ActionExecutor] call_id=019503a0-1234-7000-8000-000000000001 Executing AN (Announcement)
[ActionExecutor] call_id=019503a0-1234-7000-8000-000000000001 Announcement: a1b2c3d4-e5f6-7890-abcd-ef1234567890
[SessionCoordinator] call_id=019503a0-1234-7000-8000-000000000001 Playing announcement: /var/announcements/a1b2c3d4.wav
[SessionCoordinator] call_id=019503a0-1234-7000-8000-000000000001 Announcement completed, sending BYE
```

---

#### 5.2.4 VM（留守番電話 / Voicemail）

**RD-004 定義**:
- VM = 「アナウンス再生 + 音声録音 + 保存」

**実装**:
```rust
async fn execute_vm(&self, action: &ActionConfig, call_id: &str, session: &mut SessionCoordinator) -> Result<()> {
    info!("[ActionExecutor] call_id={} Executing VM (Voicemail)", call_id);

    // 1. outbound_mode = false（通話を確立）
    session.set_outbound_mode(false);

    // 2. アナウンス再生フラグを設定（留守番電話案内）
    if action.announce_enabled {
        session.set_announce_mode(true);
        if let Some(announcement_id) = &action.announcement_id {
            session.set_announcement_id(announcement_id.clone());
            info!("[ActionExecutor] call_id={} Voicemail announcement: {}", call_id, announcement_id);
        }
    }

    // 3. 録音有効（留守番電話メッセージを録音）
    session.set_recording_enabled(true);

    // 4. 留守番電話モードを設定
    session.set_voicemail_mode(true);

    Ok(())
}
```

**SessionCoordinator への追加フィールド・メソッド**:
```rust
pub struct SessionCoordinator {
    // ... 既存フィールド ...
    voicemail_mode: bool,  // 新規: VM モード（アナウンス後に録音開始）
}

impl SessionCoordinator {
    pub fn set_voicemail_mode(&mut self, enabled: bool) {
        self.voicemail_mode = enabled;
    }
}
```

**留守番電話フロー**:
1. 通話確立（200 OK 応答）
2. アナウンス再生（「ただいま電話に出ることができません。ピーという音の後にメッセージをお残しください。」）
3. ビープ音再生
4. 録音開始（caller の音声を recording_manager で録音）
5. タイムアウト or 通話終了で録音停止
6. 録音ファイルを保存（voicemail ディレクトリ）

**ログ出力例**:
```
[ActionExecutor] call_id=019503a0-1234-7000-8000-000000000001 action_code=VM
[ActionExecutor] call_id=019503a0-1234-7000-8000-000000000001 Executing VM (Voicemail)
[ActionExecutor] call_id=019503a0-1234-7000-8000-000000000001 Voicemail announcement: b2c3d4e5-f6g7-h8i9-jklm-nopqrstuvwxy
[SessionCoordinator] call_id=019503a0-1234-7000-8000-000000000001 Playing voicemail announcement
[SessionCoordinator] call_id=019503a0-1234-7000-8000-000000000001 Voicemail recording started
[SessionCoordinator] call_id=019503a0-1234-7000-8000-000000000001 Voicemail recording saved: /var/voicemail/019503a0-1234-7000-8000-000000000001.wav
```

---

#### 5.2.5 IV（IVR フローへ移行）

**RD-004 定義**:
- IV = 「IVR フローへ移行（DTMF メニュー）」

**Phase 3 実装スコープ**:
- IVR フローへの遷移準備（ivr_state 設定、ivr_flow_id 保存）
- **IVR 実行エンジン（DTMF 入力待ち、タイムアウト、リトライ）は Phase 4 で実装**

**Phase 3 実装**:
```rust
async fn execute_iv(&self, action: &ActionConfig, call_id: &str, session: &mut SessionCoordinator) -> Result<()> {
    info!("[ActionExecutor] call_id={} Executing IV (IVR Flow)", call_id);

    // 1. outbound_mode = false（通話を確立）
    session.set_outbound_mode(false);

    // 2. IVR フロー ID を設定
    if let Some(ivr_flow_id) = action.ivr_flow_id {
        session.set_ivr_flow_id(ivr_flow_id);
        info!("[ActionExecutor] call_id={} IVR flow: {}", call_id, ivr_flow_id);
    } else {
        warn!("[ActionExecutor] call_id={} IV ActionCode but ivr_flow_id is None, fallback to VR", call_id);
        self.execute_voicebot(action, call_id, session).await?;
        return Ok(());
    }

    // 3. IVR 状態を設定（既存の IvrMenuWaiting を使用）
    session.set_ivr_state(IvrState::IvrMenuWaiting);

    // 4. 録音フラグ設定
    session.set_recording_enabled(action.recording_enabled);

    // 注: Phase 4 で IVR 実行エンジンが ivr_state を見て IVR フローを実行する

    Ok(())
}
```

**SessionCoordinator への追加メソッド**:
```rust
impl SessionCoordinator {
    pub fn set_ivr_flow_id(&mut self, ivr_flow_id: Uuid) {
        self.ivr_flow_id = Some(ivr_flow_id);
    }
}
```

**Phase 4 への引き継ぎ**:
- `ivr_state = IvrState::IvrMenuWaiting` が設定されている（既存の enum 値を使用）
- `ivr_flow_id` が設定されている
- Phase 4 で IVR 実行エンジンが `ivr_state` を監視し、IVR フローを実行

**ログ出力例**:
```
[ActionExecutor] call_id=019503a0-1234-7000-8000-000000000001 action_code=IV
[ActionExecutor] call_id=019503a0-1234-7000-8000-000000000001 Executing IV (IVR Flow)
[ActionExecutor] call_id=019503a0-1234-7000-8000-000000000001 IVR flow: c3d4e5f6-g7h8-i9j0-klmn-opqrstuvwxyz
[SessionCoordinator] call_id=019503a0-1234-7000-8000-000000000001 IVR state: IvrMenuWaiting
```

---

### 5.3 ActionExecutor の拡張

**既存（Phase 2）**:
```rust
impl ActionExecutor {
    pub async fn execute(&self, action: &ActionConfig, call_id: &str, session: &mut SessionCoordinator) -> Result<()> {
        match action.action_code.as_str() {
            "VR" | "VB" => self.execute_voicebot(action, call_id, session).await,
            _ => {
                warn!("[ActionExecutor] call_id={} Unknown ActionCode: {}, fallback to VR", call_id, action.action_code);
                self.execute_voicebot(action, call_id, session).await
            }
        }
    }
}
```

**Phase 3 拡張**:
```rust
impl ActionExecutor {
    pub async fn execute(&self, action: &ActionConfig, call_id: &str, session: &mut SessionCoordinator) -> Result<()> {
        info!("[ActionExecutor] call_id={} action_code={}", call_id, action.action_code);

        match action.action_code.as_str() {
            "VR" | "VB" => self.execute_voicebot(action, call_id, session).await,
            "BZ" => self.execute_bz(action, call_id, session).await,
            "NR" => self.execute_nr(action, call_id, session).await,
            "AN" => self.execute_an(action, call_id, session).await,
            "VM" => self.execute_vm(action, call_id, session).await,
            "IV" => self.execute_iv(action, call_id, session).await,
            _ => {
                warn!("[ActionExecutor] call_id={} Unknown ActionCode: {}, fallback to VR", call_id, action.action_code);
                self.execute_voicebot(action, call_id, session).await
            }
        }
    }
}
```

---

### 5.4 ログ出力

**ログ形式**: RD-004 NFR-1、AGENTS.md §4.2 参照

**出力項目**:
- **call_id**（必須、全ログに含める）
- ActionCode
- 実行内容（BZ: 486 送信、NR: 無応答、AN: アナウンス ID、VM: 録音開始、IV: IVR フロー ID）

---

## 6. 受入条件（Acceptance Criteria）

### AC-1: BZ（話中応答）実装
- [ ] ActionCode=BZ が正しく実行される（SIP 486 Busy Here 応答）
- [ ] 通話が確立されない
- [ ] ログに call_id と "Sent 486 Busy Here" が出力される

### AC-2: NR（応答なし）実装
- [ ] ActionCode=NR が正しく実行される（SIP 応答を返さない）
- [ ] タイムアウトまで応答しない
- [ ] ログに call_id と "NR mode: skipping SIP responses" が出力される

### AC-3: AN（アナウンス再生）実装
- [ ] ActionCode=AN が正しく実行される（アナウンス再生）
- [ ] アナウンス音声が再生される
- [ ] 再生完了後、切断される
- [ ] ログに call_id と announcement_id が出力される

### AC-4: VM（留守番電話）実装
- [ ] ActionCode=VM が正しく実行される（アナウンス + 録音）
- [ ] 留守番電話アナウンスが再生される
- [ ] caller の音声が録音される
- [ ] 録音ファイルが保存される
- [ ] ログに call_id と "Voicemail recording started" が出力される

### AC-5: IV（IVR フローへ移行）実装
- [ ] ActionCode=IV が正しく実行される（IVR フロー遷移準備）
- [ ] ivr_flow_id が設定される
- [ ] ivr_state が IvrMenuWaiting になる
- [ ] ログに call_id と ivr_flow_id が出力される

### AC-6: 統合テスト
- [ ] Frontend で ActionCode=BZ を設定すると、実際の着信が BZ で処理される
- [ ] Frontend で ActionCode=NR を設定すると、実際の着信が NR で処理される
- [ ] Frontend で ActionCode=AN を設定すると、実際の着信が AN で処理される
- [ ] Frontend で ActionCode=VM を設定すると、実際の着信が VM で処理される
- [ ] Frontend で ActionCode=IV を設定すると、実際の着信が IV で処理される

### AC-7: ログ出力
- [ ] 全てのログに call_id が含まれる
- [ ] 各 ActionCode のログが出力される（BZ, NR, AN, VM, IV）

---

## 7. 設計決定事項（Design Decisions）

### D-01: Phase 3 では IVR 実行エンジンは未実装

**決定**: Phase 3 では IV ActionCode は「IVR フローへの遷移準備」のみ実装し、実際の IVR 実行エンジン（DTMF 入力待ち、タイムアウト、リトライ）は Phase 4 で実装

**理由**:
- IVR 実行エンジンは最も複雑な機能であり、Phase 3 で全 ActionCode を実装するには時間がかかりすぎる
- IV 以外の ActionCode（BZ, NR, AN, VM）は比較的シンプルで、Phase 3 で完結できる
- Phase 3 で IV の基盤（ivr_state 設定、ivr_flow_id 保存）を整えておけば、Phase 4 で IVR 実行エンジンを実装しやすい

### D-02: AN/VM のアナウンス再生は playback_service を活用

**決定**: AN/VM のアナウンス再生は、既存の playback_service（AI 応答の音声再生に使用）を流用する

**理由**:
- 既存実装を活用できる
- アナウンス音声ファイル（WAV）も AI 応答音声と同じ形式で再生可能
- 新たに音声再生ロジックを実装する必要がない

### D-03: VM の録音は recording_manager を活用

**決定**: VM の録音は、既存の recording_manager（通話録音に使用）を流用する

**理由**:
- 既存実装を活用できる
- 留守番電話メッセージも通話録音と同じ形式で録音可能
- voicemail ディレクトリに保存する（通話録音とは別ディレクトリ）

---

## 8. リスク・制約

### 8.1 リスク

| リスク | 影響度 | 発生確率 | 対策 |
|--------|--------|---------|------|
| AN/VM のアナウンス再生が playback_service と競合 | 中 | 低 | playback_service の排他制御を確認、必要に応じて状態管理を追加 |
| VM の録音ファイルが通話録音と混在 | 低 | 低 | voicemail 専用ディレクトリを作成（/var/voicemail/） |
| IV の基盤実装が Phase 4 で不足 | 中 | 中 | Phase 3 で ivr_state, ivr_flow_id の設定を確実に実装 |

### 8.2 制約

| 制約 | 理由 | 代替案 |
|------|------|--------|
| Phase 3 では IVR 実行エンジンは未実装 | 最も複雑な機能なので後回し | IV は遷移準備のみ、Phase 4 で実行エンジン実装 |
| AN/VM のアナウンス音声ファイルは事前アップロード必須 | Phase 3 ではアナウンス管理 UI は未実装 | Phase 3 ではテスト用音声ファイルを手動配置 |

---

## 9. 参照

| ドキュメント | セクション | 内容 |
|-------------|-----------|------|
| [STEER-137](STEER-137_backend-integration-strategy.md) | §5.2.4.3 | Issue #141（Phase 3）の定義 |
| [STEER-140](STEER-140_rule-evaluation-engine.md) | - | Phase 2 実装（VR のみ） |
| [RD-004](virtual-voicebot-backend/docs/requirements/RD-004_call-routing-execution.md) | FR-2 | ActionCode 実行の要件 |
| [contract.md](contract.md) | §3 | ActionCode の仕様 |

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-08 | 初版作成（Draft） | Claude Code (claude-sonnet-4-5) |
| 2026-02-08 | Codex Review #1 対応（重大4件、軽1件修正）：NR の早期 return 修正、BZ の SessionOut 統一、announcement_id 取り込み経路追加、IvrState を既存値に変更、影響範囲に mod.rs 追加 | Claude Code (claude-sonnet-4-5) |
| 2026-02-08 | Codex Review #2 対応（重大1件、中2件、軽1件修正）：NR の RingDurationElapsed 対応、SessionOut 名統一（SipSend100/180/200）、影響範囲明確化、BZ の終了制御（invite_rejected 使用） | Claude Code (claude-sonnet-4-5) |
| 2026-02-08 | Codex Review #3 対応（中1件修正）：BZ の SipInvite 後続処理スキップロジック追加（invite_rejected チェックで outbound 起動・100/180 送信・pending_answer 設定を抑止） | Claude Code (claude-sonnet-4-5) |
| 2026-02-08 | 承認完了、ステータス → Approved：レビューサイクル完了（Codex 4回、全指摘対応完了）、実装フェーズへ引き継ぎ準備完了 | Claude Code (claude-sonnet-4-5) |
