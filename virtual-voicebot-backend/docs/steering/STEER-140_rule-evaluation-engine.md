# STEER-140: Backend ルール評価エンジン実装（Phase 2）

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-140 |
| タイトル | Backend ルール評価エンジン実装（Phase 2: VR 動作検証） |
| ステータス | Approved |
| 関連Issue | #140 |
| 優先度 | P0 |
| 作成日 | 2026-02-08 |
| 親ステアリング | STEER-137 |

---

## 2. ストーリー（Why）

### 2.1 背景

Issue #139（STEER-139）で Frontend → Backend の設定同期が完了し、Frontend で設定した着信ルール・番号グループ・IVR フローが Backend DB に保存されるようになった。

**問題**:
- Backend DB に設定は保存されているが、実際の着信時にこれらの設定が使われていない
- 現在の着信処理は固定動作（VB または VR のみ）で、Frontend の設定が反映されない
- ユーザーが Frontend で設定したルールが実際の通話に反映されない

**影響**:
- Frontend PoC の価値が発揮されない（設定しても動かない）
- Phase 3（全 ActionCode 実装）、Phase 4（IVR 実行エンジン）に進めない
- エンドユーザーが期待する着信制御ができない

### 2.2 目的

Backend の着信処理に **ルール評価エンジン** を実装し、Frontend で設定したルールに基づいて着信動作を制御できるようにする。

**達成目標**:
- 着信時に 3段階評価（番号完全一致 → 番号グループ → カテゴリ → デフォルト）が動作する
- VR（voicebot 通常処理 + 録音フラグ対応）が正しく実行される
- Frontend で設定変更すると、実際の着信動作が変わる

### 2.3 ユーザーストーリー

```
As a システム管理者
I want to Frontend で設定した着信ルールが実際の通話に反映される
So that 番号グループごとに異なる対応（録音あり/なし、拒否など）ができる

受入条件:
- [ ] Frontend で番号グループ「スパム」を作成し、ActionCode=BZ（話中応答）を設定すると、その番号からの着信が BZ で処理される
- [ ] Frontend で着信ルールを作成し、ActionCode=VR（録音あり）を設定すると、その番号からの着信が VR で処理される
- [ ] Frontend で非通知着信の設定（anonymousAction=BZ）を設定すると、非通知着信が BZ で処理される
- [ ] Frontend でデフォルトアクション（defaultAction=VR）を設定すると、ルールにマッチしない着信が VR で処理される
- [ ] ルール評価の過程（どの段階でマッチしたか）がログ出力される
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-08 |
| 起票理由 | Issue #139 完了後、STEER-137 の Phase 2 実装を開始 |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Code (claude-sonnet-4-5) |
| 作成日 | 2026-02-08 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "Issue #140 のステアリングファイルを作成" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| 1 | Codex | 2026-02-08 | 要修正 | 重大3件（モジュール構成不一致、テーブル名不一致、JSON解釈不一致）、中3件（VR定義揺れ、AC混在、call_id不足）、軽1件（E.164正規化明示不足）→ 全て修正完了 |
| 2 | Codex | 2026-02-08 | 要修正 | 重大2件（SessionCoordinator 統合ポイント不一致、Executor API 不存在）、中3件（電話番号正規化不足、VR定義揺れ、action_config JSON変換不足）→ 全て修正完了 |
| 3 | Codex | 2026-02-08 | 要修正 | 重大3件（非通知判定順序矛盾、private フィールド直接アクセス、RoutingContext 不十分）、中3件（VR定義揺れ、extract_caller_id シグネチャ不一致、ActionExecutor::new() 未定義）→ 全て修正完了 |
| 4 | Codex | 2026-02-08 | 要修正 | 重大2件（RoutingPort 注入経路不明、RecordingManager.set_enabled 未定義）、中2件（extract_user_from_to 不使用、VR定義揺れ）、軽1件（影響ファイル mod.rs 漏れ）→ 全て修正完了 |
| 5 | Codex | 2026-02-08 | 要修正 | 中1件（VR定義揺れ）、軽1件（影響ファイル handlers/mod.rs 漏れ）→ 全て修正完了 |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | @MasanoriSuda |
| 承認日 | 2026-02-08 |
| 承認コメント | Codex レビュー 5回実施、全指摘対応完了（重大10件、中12件、軽4件）。実装フェーズへ |

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
| RD-004 | 参照 | FR-1（着信ルール評価）、FR-2（ActionCode 実行）を参照 |
| BD-004 | 参照 | registered_numbers, call_action_rules, routing_rules, system_settings テーブル定義を参照 |
| contract.md | 参照 | ActionCode の仕様を参照 |

### 4.2 影響するコード

| モジュール | 変更種別 | 概要 |
|-----------|---------|------|
| src/service/mod.rs | 修正 | routing モジュールを追加 |
| src/service/routing/mod.rs | 新規 | ルール評価エンジン（3段階評価ロジック） |
| src/service/routing/evaluator.rs | 新規 | ルール評価ロジック実装 |
| src/service/routing/executor.rs | 新規 | ActionCode 実行ロジック（Phase 2 では VR のみ） |
| src/protocol/session/coordinator.rs | 修正 | routing_port フィールド追加、着信時にルール評価エンジンを呼び出し |
| src/protocol/session/handlers/mod.rs | 修正 | handle_control_event(SipInvite) にルール評価・Action実行を統合 |
| src/protocol/session/recording_manager.rs | 修正 | set_enabled() メソッド追加、enabled フラグ追加 |
| src/protocol/session/writing.rs | 修正 | spawn_session に routing_port パラメータ追加 |
| src/main.rs | 修正 | spawn_session 呼び出し時に routing_port を注入 |
| src/shared/ports/routing_port.rs | 新規 | RoutingPort trait 定義 |
| src/interface/db/routing_repo.rs | 新規 | RoutingPort 実装（registered_numbers, call_action_rules, routing_rules クエリ） |

---

## 5. 差分仕様（What / How）

### 5.1 システム構成

```
┌────────────────────────────────────────────────────────────┐
│                    SIP/RTP Engine                          │
├────────────────────────────────────────────────────────────┤
│                                                             │
│  INVITE 受信 (Caller ID あり)                               │
│         │                                                   │
│         ▼                                                   │
│  ┌─────────────────────────────────────┐                   │
│  │  Rule Evaluation Engine (新規)      │                   │
│  ├─────────────────────────────────────┤                   │
│  │                                     │                   │
│  │  【段階1】番号完全一致               │                   │
│  │  ├─ registered_numbers 検索         │                   │
│  │  │   WHERE phone_number = $1        │                   │
│  │  └─ Hit → action_code 取得          │                   │
│  │                                     │                   │
│  │  【段階2】番号グループ評価           │                   │
│  │  ├─ registered_numbers から group_id 取得 │             │
│  │  ├─ call_action_rules 検索          │                   │
│  │  │   WHERE caller_group_id = $1     │                   │
│  │  │   ORDER BY priority ASC          │                   │
│  │  └─ Hit → action_config 取得        │                   │
│  │                                     │                   │
│  │  【段階3】カテゴリ評価               │                   │
│  │  ├─ Caller ID を4カテゴリに分類     │                   │
│  │  │   (spam/registered/unknown)      │                   │
│  │  ├─ routing_rules 検索              │                   │
│  │  │   WHERE caller_category = $1     │                   │
│  │  │   ORDER BY priority ASC          │                   │
│  │  └─ Hit → action_code 取得          │                   │
│  │                                     │                   │
│  │  【段階4】デフォルトアクション       │                   │
│  │  └─ system_settings.extra の        │                   │
│  │     defaultAction を適用            │                   │
│  └─────────────────────────────────────┘                   │
│         │                                                   │
│         ▼                                                   │
│  ┌─────────────────────────────────────┐                   │
│  │  Action Executor (新規)             │                   │
│  ├─────────────────────────────────────┤                   │
│  │                                     │                   │
│  │  Phase 2 実装: VR のみ               │                   │
│  │  - outbound_mode=false 設定         │                   │
│  │  - recording_enabled フラグ確認     │                   │
│  │  - announce_enabled フラグ確認      │                   │
│  │  - 録音開始（recording_enabled=true）│                   │
│  │                                     │                   │
│  │  Phase 3 実装予定:                  │                   │
│  │  - IV（IVR）                        │                   │
│  │  - VM（留守番電話）                 │                   │
│  │  - BZ（話中応答）                   │                   │
│  │  - NR（応答なし）                   │                   │
│  │  - AN（アナウンス再生）             │                   │
│  └─────────────────────────────────────┘                   │
│                                                             │
└────────────────────────────────────────────────────────────┘
```

### 5.2 ルール評価エンジン実装

#### 5.2.1 評価フロー

**実装モジュール**: `src/service/routing/evaluator.rs`

**依存定義（Port）**:
```rust
// src/shared/ports/routing_port.rs
#[async_trait]
pub trait RoutingPort: Send + Sync {
    async fn find_registered_number(&self, phone_number: &str) -> Result<Option<RegisteredNumberRow>>;
    async fn find_caller_group(&self, phone_number: &str) -> Result<Option<Uuid>>;
    async fn find_call_action_rule(&self, group_id: Uuid) -> Result<Option<CallActionRuleRow>>;
    async fn is_spam(&self, phone_number: &str) -> Result<bool>;
    async fn is_registered(&self, phone_number: &str) -> Result<bool>;
    async fn find_routing_rule(&self, category: &str) -> Result<Option<RoutingRuleRow>>;
    async fn get_system_settings_extra(&self) -> Result<Option<serde_json::Value>>;
}

pub struct RegisteredNumberRow {
    pub action_code: String,
    pub ivr_flow_id: Option<Uuid>,
    pub recording_enabled: bool,
    pub announce_enabled: bool,
}

pub struct CallActionRuleRow {
    pub id: Uuid,
    pub action_config: serde_json::Value,
}

pub struct RoutingRuleRow {
    pub id: Uuid,
    pub action_code: String,
    pub ivr_flow_id: Option<Uuid>,
}
```

**公開インターフェース**:
```rust
pub struct RuleEvaluator {
    routing_port: Arc<dyn RoutingPort>,
}

impl RuleEvaluator {
    pub async fn evaluate(&self, caller_id: &str, call_id: &str) -> Result<ActionConfig, RoutingError> {
        info!("[RuleEvaluator] call_id={} Evaluating caller_id={}", call_id, caller_id);

        // 0. 非通知判定（正規化の前に実施）
        // RD-004 FR-1.3: Caller ID が空、"anonymous"、"withheld" の場合、anonymousAction を適用
        if caller_id.is_empty() || caller_id.eq_ignore_ascii_case("anonymous") || caller_id.eq_ignore_ascii_case("withheld") {
            info!("[RuleEvaluator] call_id={} Anonymous caller detected (caller_id={})", call_id, caller_id);
            return self.get_anonymous_action(call_id).await;
        }

        // 1. 電話番号正規化（E.164形式確認）
        let normalized_caller_id = self.normalize_phone_number(caller_id)?;
        info!("[RuleEvaluator] call_id={} Normalized: {} -> {}", call_id, caller_id, normalized_caller_id);

        // 2. 段階1: 番号完全一致
        if let Some(action) = self.match_registered_number(&normalized_caller_id, call_id).await? {
            info!("[RuleEvaluator] call_id={} Hit: Stage 1 (registered_numbers) phone_number={}", call_id, normalized_caller_id);
            return Ok(action);
        }
        info!("[RuleEvaluator] call_id={} Miss: Stage 1 (registered_numbers)", call_id);

        // 3. 段階2: 番号グループ評価
        if let Some(action) = self.match_caller_group(&normalized_caller_id, call_id).await? {
            info!("[RuleEvaluator] call_id={} Hit: Stage 2 (call_action_rules)", call_id);
            return Ok(action);
        }
        info!("[RuleEvaluator] call_id={} Miss: Stage 2 (call_action_rules)", call_id);

        // 4. 段階3: カテゴリ評価
        let category = self.classify_caller(&normalized_caller_id, call_id).await?;
        if let Some(action) = self.match_routing_rule(&category, call_id).await? {
            info!("[RuleEvaluator] call_id={} Hit: Stage 3 (routing_rules) category={}", call_id, category);
            return Ok(action);
        }
        info!("[RuleEvaluator] call_id={} Miss: Stage 3 (routing_rules)", call_id);

        // 5. 段階4: デフォルトアクション
        info!("[RuleEvaluator] call_id={} Fallback: Stage 4 (defaultAction)", call_id);
        self.get_default_action(call_id).await
    }

    fn normalize_phone_number(&self, phone_number: &str) -> Result<String, RoutingError> {
        // RD-004 FR-1.2: E.164 形式（+819012345678）に正規化
        // 1. 空白除去
        let cleaned = phone_number.replace(" ", "").replace("-", "").replace("(", "").replace(")", "");

        // 2. E.164 形式チェック（+ で始まり、8文字以上）
        if cleaned.starts_with('+') && cleaned.len() >= 8 && cleaned.len() <= 16 {
            // 3. 数字のみかチェック（+ を除く）
            if cleaned[1..].chars().all(|c| c.is_ascii_digit()) {
                Ok(cleaned)
            } else {
                Err(RoutingError::InvalidPhoneNumber(format!("Non-digit characters in {}", phone_number)))
            }
        } else {
            Err(RoutingError::InvalidPhoneNumber(format!("Invalid E.164 format: {}", phone_number)))
        }
    }
}
```

#### 5.2.2 データ型定義

##### ActionConfig（内部型）

**定義**:
```rust
#[derive(Debug, Clone)]
pub struct ActionConfig {
    pub action_code: String,  // "VR", "VB", "BZ", etc.
    pub ivr_flow_id: Option<Uuid>,
    pub recording_enabled: bool,
    pub announce_enabled: bool,
}
```

##### ActionConfigDto（JSON 変換用）

**定義**:
```rust
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionConfigDto {
    pub action_code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ivr_flow_id: Option<Uuid>,
    #[serde(default = "default_true")]
    pub recording_enabled: bool,
    #[serde(default = "default_false")]
    pub announce_enabled: bool,
}

fn default_true() -> bool { true }
fn default_false() -> bool { false }
```

**変換**:
```rust
impl From<ActionConfigDto> for ActionConfig {
    fn from(dto: ActionConfigDto) -> Self {
        Self {
            action_code: dto.action_code,
            ivr_flow_id: dto.ivr_flow_id,
            recording_enabled: dto.recording_enabled,
            announce_enabled: dto.announce_enabled,
        }
    }
}
```

**JSON 例（call_action_rules.action_config）**:
```json
{
  "actionCode": "VR",
  "ivrFlowId": null,
  "recordingEnabled": true,
  "announceEnabled": false
}
```

#### 5.2.3 各段階の実装

##### 段階1: 番号完全一致

**実装**: `match_registered_number()`

```rust
async fn match_registered_number(&self, caller_id: &str, call_id: &str) -> Result<Option<ActionConfig>> {
    let row = self.routing_port.find_registered_number(caller_id).await?;

    match row {
        Some(row) => {
            let action = ActionConfig {
                action_code: row.action_code,
                ivr_flow_id: row.ivr_flow_id,
                recording_enabled: row.recording_enabled,
                announce_enabled: row.announce_enabled,
            };
            info!("[RuleEvaluator] call_id={} Stage 1: action_code={}", call_id, action.action_code);
            Ok(Some(action))
        }
        None => Ok(None),
    }
}
```

##### 段階2: 番号グループ評価

**実装**: `match_caller_group()`

```rust
async fn match_caller_group(&self, caller_id: &str, call_id: &str) -> Result<Option<ActionConfig>> {
    // 1. Caller ID の group_id を取得
    let group_id = self.routing_port.find_caller_group(caller_id).await?;

    let group_id = match group_id {
        Some(id) => id,
        None => return Ok(None),
    };

    // 2. call_action_rules から一致するルールを検索（priority 昇順）
    let row = self.routing_port.find_call_action_rule(group_id).await?;

    match row {
        Some(row) => {
            // action_config (JSONB) を ActionConfigDto 経由でパース
            let dto: ActionConfigDto = serde_json::from_value(row.action_config)?;
            let config: ActionConfig = dto.into();
            info!("[RuleEvaluator] call_id={} Stage 2: rule_id={} action_code={}", call_id, row.id, config.action_code);
            Ok(Some(config))
        }
        None => Ok(None),
    }
}
```

##### 段階3: カテゴリ評価

**実装**: `classify_caller()` + `match_routing_rule()`

```rust
async fn classify_caller(&self, caller_id: &str, call_id: &str) -> Result<CallerCategory> {
    // 1. spam_numbers チェック（spam）
    let is_spam = self.routing_port.is_spam(caller_id).await?;

    if is_spam {
        info!("[RuleEvaluator] call_id={} Classified as: spam", call_id);
        return Ok(CallerCategory::Spam);
    }

    // 2. registered_numbers チェック（registered）
    let is_registered = self.routing_port.is_registered(caller_id).await?;

    if is_registered {
        info!("[RuleEvaluator] call_id={} Classified as: registered", call_id);
        return Ok(CallerCategory::Registered);
    }

    // 3. その他は unknown
    info!("[RuleEvaluator] call_id={} Classified as: unknown", call_id);
    Ok(CallerCategory::Unknown)
}

async fn match_routing_rule(&self, category: &CallerCategory, call_id: &str) -> Result<Option<ActionConfig>> {
    let category_str = category.as_str();
    let row = self.routing_port.find_routing_rule(category_str).await?;

    match row {
        Some(row) => {
            let action = ActionConfig {
                action_code: row.action_code,
                ivr_flow_id: row.ivr_flow_id,
                recording_enabled: true,  // routing_rules にはフラグがないのでデフォルト値
                announce_enabled: true,
            };
            info!("[RuleEvaluator] call_id={} Stage 3: rule_id={} action_code={}", call_id, row.id, action.action_code);
            Ok(Some(action))
        }
        None => Ok(None),
    }
}
```

##### 段階4: デフォルトアクション

**実装**: `get_default_action()` + `get_anonymous_action()`

```rust
async fn get_default_action(&self, call_id: &str) -> Result<ActionConfig> {
    let extra = self.routing_port.get_system_settings_extra().await?;

    match extra {
        Some(extra) => {
            // extra (JSONB) から defaultAction を取得
            if let Some(default_action) = extra.get("defaultAction") {
                // defaultAction は ActionDestination { actionType, actionConfig } 構造
                // actionConfig から ActionCode を取得
                if let Some(action_config) = default_action.get("actionConfig") {
                    let dto: ActionConfigDto = serde_json::from_value(action_config.clone())?;
                    let config: ActionConfig = dto.into();
                    info!("[RuleEvaluator] call_id={} Stage 4 (defaultAction): action_code={}", call_id, config.action_code);
                    Ok(config)
                } else {
                    warn!("[RuleEvaluator] call_id={} defaultAction.actionConfig not found, using fallback VR", call_id);
                    Ok(ActionConfig::default_vr())
                }
            } else {
                warn!("[RuleEvaluator] call_id={} defaultAction not found in system_settings.extra, using fallback VR", call_id);
                Ok(ActionConfig::default_vr())
            }
        }
        None => {
            warn!("[RuleEvaluator] call_id={} system_settings not found, using fallback VR", call_id);
            Ok(ActionConfig::default_vr())
        }
    }
}

async fn get_anonymous_action(&self, call_id: &str) -> Result<ActionConfig> {
    let extra = self.routing_port.get_system_settings_extra().await?;

    match extra {
        Some(extra) => {
            if let Some(anonymous_action) = extra.get("anonymousAction") {
                // anonymousAction は ActionDestination { actionType, actionConfig } 構造
                // actionConfig から ActionCode を取得
                if let Some(action_config) = anonymous_action.get("actionConfig") {
                    let dto: ActionConfigDto = serde_json::from_value(action_config.clone())?;
                    let config: ActionConfig = dto.into();
                    info!("[RuleEvaluator] call_id={} anonymousAction: action_code={}", call_id, config.action_code);
                    Ok(config)
                } else {
                    warn!("[RuleEvaluator] call_id={} anonymousAction.actionConfig not found, using fallback BZ", call_id);
                    Ok(ActionConfig::default_bz())
                }
            } else {
                warn!("[RuleEvaluator] call_id={} anonymousAction not found in system_settings.extra, using fallback BZ", call_id);
                Ok(ActionConfig::default_bz())
            }
        }
        None => {
            warn!("[RuleEvaluator] call_id={} system_settings not found, using fallback BZ", call_id);
            Ok(ActionConfig::default_bz())
        }
    }
}
```

**フォールバック定義**:
```rust
impl ActionConfig {
    pub fn default_vr() -> Self {
        Self {
            action_code: "VR".to_string(),
            ivr_flow_id: None,
            recording_enabled: true,
            announce_enabled: false,
        }
    }

    pub fn default_bz() -> Self {
        Self {
            action_code: "BZ".to_string(),
            ivr_flow_id: None,
            recording_enabled: false,
            announce_enabled: false,
        }
    }
}
```

### 5.3 ActionCode 実行ロジック（Phase 2: VR のみ）

#### 5.3.1 Action Executor

**実装モジュール**: `src/service/routing/executor.rs`

**VR の定義（Phase 2）**:
- **RD-004 定義**: VR = Voicebot+Record = 「AI応答（ボイスボット）開始（録音あり）」
- **Phase 2 実装**: VR = voicebot 通常処理（`outbound_mode=false`）+ `recording_enabled` フラグ対応
  - `outbound_mode=false` → 既存の voicebot 処理が動作（IVR → AI 対話）
  - `recording_enabled=true` → 録音フラグをセット（recording_manager が録音開始）
  - `recording_enabled=false` → 録音なし（VB 相当の動作）

**SessionCoordinator への公開メソッド追加**:
```rust
// src/protocol/session/coordinator.rs に追加
impl SessionCoordinator {
    pub fn set_outbound_mode(&mut self, enabled: bool) {
        self.outbound_mode = enabled;
    }

    pub fn set_recording_enabled(&mut self, enabled: bool) {
        self.recording.set_enabled(enabled);
    }
}
```

**RecordingManager への録音制御フラグ追加**:
```rust
// src/protocol/session/recording_manager.rs に追加
pub struct RecordingManager {
    call_id: String,
    recorder: Recorder,
    b_leg_recorder: Option<Recorder>,
    error_sink: RecordingErrorSink,
    enabled: bool,  // 新規: 録音有効/無効フラグ
}

impl RecordingManager {
    pub fn new(call_id: impl Into<String>) -> Self {
        let call_id = call_id.into();
        Self {
            recorder: Recorder::new(call_id.clone()),
            b_leg_recorder: None,
            call_id,
            error_sink: RecordingErrorSink::default(),
            enabled: true,  // デフォルトは録音有効
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn start_main(&mut self) -> Result<(), RecordingError> {
        if !self.enabled {
            // 録音無効の場合は何もしない
            return Ok(());
        }
        self.recorder
            .start()
            .map_err(|e| RecordingError::Start(e.to_string()))
    }
}
```

**公開インターフェース**:
```rust
pub struct ActionExecutor;

impl ActionExecutor {
    pub fn new() -> Self {
        Self
    }

    pub async fn execute(&self, action: &ActionConfig, call_id: &str, session: &mut SessionCoordinator) -> Result<()> {
        info!("[ActionExecutor] call_id={} action_code={}", call_id, action.action_code);

        match action.action_code.as_str() {
            "VR" | "VB" => self.execute_voicebot(action, call_id, session).await,
            // Phase 3 で追加予定
            // "BZ" => self.execute_bz(action, call_id, session).await,
            // "NR" => self.execute_nr(action, call_id, session).await,
            // "AN" => self.execute_an(action, call_id, session).await,
            // "VM" => self.execute_vm(action, call_id, session).await,
            // "IV" => self.execute_iv(action, call_id, session).await,
            _ => {
                warn!("[ActionExecutor] call_id={} Unknown ActionCode: {}, fallback to VR", call_id, action.action_code);
                self.execute_voicebot(action, call_id, session).await
            }
        }
    }

    async fn execute_voicebot(&self, action: &ActionConfig, call_id: &str, session: &mut SessionCoordinator) -> Result<()> {
        info!("[ActionExecutor] call_id={} Executing Voicebot (action_code={}, recording_enabled={})",
              call_id, action.action_code, action.recording_enabled);

        // 1. outbound_mode = false（通常着信処理 = voicebot 処理）
        session.set_outbound_mode(false);

        // 2. 録音フラグ設定
        session.set_recording_enabled(action.recording_enabled);
        if action.recording_enabled {
            info!("[ActionExecutor] call_id={} Recording enabled", call_id);
        } else {
            info!("[ActionExecutor] call_id={} Recording disabled", call_id);
        }

        // 3. アナウンス再生（announce_enabled=true の場合）
        if action.announce_enabled {
            // TODO: アナウンス再生ロジック（Phase 3 で実装）
            // session.playback_service.play_announcement(action.announcement_id).await?;
            warn!("[ActionExecutor] call_id={} announce_enabled=true だがアナウンス再生は Phase 3 で実装予定", call_id);
        }

        // 4. IVR フローへの遷移（ivr_flow_id が指定されている場合）
        if let Some(ivr_flow_id) = action.ivr_flow_id {
            // TODO: IVR フロー実行ロジック（Phase 4 で実装）
            warn!("[ActionExecutor] call_id={} ivr_flow_id={} specified but IVR execution is Phase 4", call_id, ivr_flow_id);
        }

        Ok(())
    }
}
```

**VR の動作（Phase 2）**:
- VR = voicebot 通常処理（`outbound_mode=false`）+ 録音フラグ対応（`recording_enabled`）
- `outbound_mode=false` → voicebot 通常処理が動作（既存の IVR → AI 対話フロー）
- `recording_enabled=true/false` → 録音の有無を制御

### 5.4 既存コード統合

#### 5.4.1 Session への統合

**修正ファイル**: `src/protocol/session/handlers/mod.rs`

**統合ポイント**: `handle_control_event` の `SessionControlIn::SipInvite` 分岐

**RoutingPort 注入経路**:
1. `main.rs` で `RoutingPort` 実装（`RoutingRepoImpl`）を生成
2. `spawn_session()` に `routing_port: Arc<dyn RoutingPort>` を渡す
3. `SessionCoordinator::spawn()` で `routing_port` フィールドに保存
4. `handle_control_event()` で `self.routing_port` を使用

**SessionCoordinator 構造体への追加**:
```rust
// src/protocol/session/coordinator.rs に追加
pub struct SessionCoordinator {
    // ... 既存フィールド ...
    routing_port: Arc<dyn RoutingPort>,  // 新規: ルーティングDB アクセス用 Port
}
```

**spawn_session() シグネチャ変更**:
```rust
// src/protocol/session/writing.rs 修正
pub async fn spawn_session(
    call_id: CallId,
    from_uri: String,
    to_uri: String,
    registry: SessionRegistry,
    media_cfg: MediaConfig,
    session_out_tx: tokio::sync::mpsc::Sender<(CallId, SessionOut)>,
    app_tx: AppEventTx,
    rtp_tx: RtpTxHandle,
    ingest_url: Option<String>,
    recording_base_url: Option<String>,
    ingest_port: Arc<dyn IngestPort>,
    storage_port: Arc<dyn StoragePort>,
    call_log_port: Arc<dyn CallLogPort>,
    routing_port: Arc<dyn RoutingPort>,  // 新規パラメータ
    runtime_cfg: Arc<SessionRuntimeConfig>,
) -> SessionHandle {
    // ... SessionCoordinator::spawn に routing_port を渡す ...
}
```

**SessionCoordinator::spawn() シグネチャ変更**:
```rust
// src/protocol/session/coordinator.rs 修正
impl SessionCoordinator {
    pub fn spawn(
        call_id: CallId,
        from_uri: String,
        to_uri: String,
        session_out_tx: mpsc::Sender<(CallId, SessionOut)>,
        app_tx: AppEventTx,
        media_cfg: MediaConfig,
        rtp_tx: RtpTxHandle,
        ingest_url: Option<String>,
        recording_base_url: Option<String>,
        ingest_port: Arc<dyn IngestPort>,
        storage_port: Arc<dyn StoragePort>,
        call_log_port: Arc<dyn CallLogPort>,
        routing_port: Arc<dyn RoutingPort>,  // 新規パラメータ
        runtime_cfg: Arc<SessionRuntimeConfig>,
    ) -> mpsc::Receiver<SessionControlIn> {
        // ...
        let coordinator = Self {
            // ... 既存フィールド ...
            routing_port,  // 新規フィールド
        };
        // ...
    }
}
```

**main.rs での注入**:
```rust
// src/main.rs 修正
let routing_port: Arc<dyn RoutingPort> = Arc::new(RoutingRepoImpl::new(pg_pool.clone()));

// ... SipEvent::InboundInvite 処理内 ...
let sess_handle = spawn_session(
    call_id.clone(),
    from.clone(),
    to.clone(),
    session_registry.clone(),
    MediaConfig::pcmu(advertised_ip.clone(), rtp_port),
    session_out_tx.clone(),
    app_tx,
    rtp_handle.clone(),
    ingest_url,
    recording_base_url,
    ingest_port.clone(),
    storage_port.clone(),
    call_log_port.clone(),
    routing_port.clone(),  // 新規: RoutingPort 注入
    session_cfg.clone(),
)
.await;
```

**変更内容（handle_control_event 内）**:
```rust
use crate::service::routing::{RuleEvaluator, ActionExecutor};
use crate::protocol::session::handlers::sip_handler::extract_user_from_to;

impl SessionCoordinator {
    pub(crate) async fn handle_control_event(
        &mut self,
        current_state: SessState,
        ev: SessionControlIn,
    ) -> bool {
        let mut advance_state = true;
        match (current_state, ev) {
            (
                SessState::Idle,
                SessionControlIn::SipInvite {
                    offer,
                    session_timer,
                    ..
                },
            ) => {
                self.peer_sdp = Some(offer);
                if let Some(timer) = session_timer {
                    self.update_session_expires(timer);
                }
                let answer = self.build_answer_pcmu8k();
                self.local_sdp = Some(answer.clone());

                // ルール評価エンジン呼び出し（新規）
                // from_uri から Caller ID を抽出（既存ヘルパー関数を使用）
                let caller_id = extract_user_from_to(&self.from_uri).unwrap_or_default();
                let call_id_str = self.call_id.to_string();

                // RoutingPort 経由でルール評価（port/adapter パターン）
                let evaluator = RuleEvaluator::new(self.routing_port.clone());

                match evaluator.evaluate(&caller_id, &call_id_str).await {
                    Ok(action) => {
                        info!("[SessionCoordinator] call_id={} Evaluated action_code={}", self.call_id, action.action_code);

                        // ActionCode 実行（新規）
                        let executor = ActionExecutor::new();
                        if let Err(e) = executor.execute(&action, &call_id_str, self).await {
                            error!("[SessionCoordinator] call_id={} ActionCode execution failed: {}", self.call_id, e);
                            // エラー時は voicebot 通常処理にフォールバック
                            self.outbound_mode = false;
                        }
                    }
                    Err(e) => {
                        error!("[SessionCoordinator] call_id={} Rule evaluation failed: {}", self.call_id, e);
                        // エラー時は voicebot 通常処理にフォールバック
                        self.outbound_mode = false;
                    }
                }

                // 既存の処理（outbound_mode 判定など）を継続
                self.outbound_answered = false;
                self.outbound_sent_180 = false;
                self.outbound_sent_183 = false;
                self.invite_rejected = false;
                self.stop_ring_delay();

                // outbound 判定（既存）
                if self.runtime_cfg.outbound.enabled {
                    // （既存のコード）
                }

                // 100 Trying 送信（既存）
                if advance_state {
                    if let Err(err) = self
                        .session_out_tx
                        .try_send((self.call_id.clone(), SessionOut::SipSend100))
                    {
                        warn!("[session {}] dropped SipSend100 (channel full): {:?}", self.call_id, err);
                    }
                    // （既存の処理続く）
                }
            }
            // （他の分岐は既存のまま）
        }
        advance_state
    }
}
```

**注**: `extract_user_from_to()` は既存ヘルパー関数（`src/protocol/session/handlers/sip_handler.rs`）で、`Option<String>` を返す。

### 5.5 ログ出力

**ログ形式**: RD-004 NFR-1、AGENTS.md §4.2 参照

```
[RuleEvaluator] call_id=019503a0-1234-7000-8000-000000000001 Evaluating caller_id=+819012345678
[RuleEvaluator] call_id=019503a0-1234-7000-8000-000000000001 Normalized: +819012345678 -> +819012345678
[RuleEvaluator] call_id=019503a0-1234-7000-8000-000000000001 Miss: Stage 1 (registered_numbers)
[RuleEvaluator] call_id=019503a0-1234-7000-8000-000000000001 Hit: Stage 2 (call_action_rules)
[RuleEvaluator] call_id=019503a0-1234-7000-8000-000000000001 Stage 2: rule_id=019503a0-5678-7000-8000-000000000002 action_code=VR
[ActionExecutor] call_id=019503a0-1234-7000-8000-000000000001 action_code=VR
[ActionExecutor] call_id=019503a0-1234-7000-8000-000000000001 Executing Voicebot (action_code=VR, recording_enabled=true)
[ActionExecutor] call_id=019503a0-1234-7000-8000-000000000001 Recording enabled
```

**出力項目**:
- **call_id**（必須、全ログに含める）
- 評価段階（Stage 1〜4）
- マッチ結果（Hit / Miss）
- マッチしたルール ID（call_action_rules.id, routing_rules.id）
- 適用した ActionCode
- 録音開始/未開始

---

## 6. 受入条件（Acceptance Criteria）

### AC-1: ルール評価エンジン実装
- [ ] 段階1（registered_numbers）で番号完全一致が動作する
- [ ] 段階2（call_action_rules）で番号グループ評価が動作する
- [ ] 段階3（routing_rules）でカテゴリ評価が動作する
- [ ] 段階4（defaultAction）でデフォルトアクションが動作する
- [ ] 非通知着信が anonymousAction で処理される
- [ ] 評価結果がログ出力される（段階、マッチルール、ActionCode）

### AC-2: VR 実装
- [ ] ActionCode=VR が正しく実行される（voicebot 通常処理が動作）
- [ ] recording_enabled=true の場合、録音が開始される
- [ ] recording_enabled=false の場合、録音が開始されない
- [ ] outbound_mode=false が設定される（通常着信処理）

### AC-3: 統合テスト（Phase 2 スコープ）
- [ ] Frontend で着信ルール「VIP録音」を作成し、ActionCode=VR（recording_enabled=true）を設定すると、その番号からの着信が VR で処理され、録音が開始される
- [ ] Frontend で着信ルール「VIP録音なし」を作成し、ActionCode=VR（recording_enabled=false）を設定すると、その番号からの着信が VR で処理され、録音が開始されない
- [ ] Frontend でデフォルトアクション（defaultAction=VR）を設定すると、ルールにマッチしない着信が VR で処理される
- [ ] 未知の ActionCode（例: "XX"）を設定した場合、VR にフォールバックして処理される

### AC-3-後続: 統合テスト（Phase 3 で実施）
- [ ] Frontend で番号グループ「スパム」を作成し、ActionCode=BZ を設定すると、その番号からの着信が BZ で処理される（Phase 3 で BZ 実装後）
- [ ] Frontend で非通知着信の設定（anonymousAction=BZ）を設定すると、非通知着信が BZ で処理される（Phase 3 で BZ 実装後）

### AC-4: ログ出力
- [ ] 全てのログに call_id が含まれる
- [ ] 電話番号正規化のログが出力される（Normalized: xxx -> yyy）
- [ ] ルール評価の過程（段階1〜4、Hit/Miss）がログ出力される
- [ ] 適用された ActionCode がログ出力される
- [ ] 録音開始/未開始がログ出力される

---

## 7. 設計決定事項（Design Decisions）

### D-01: Phase 2 では VR のみ実装

**決定**: Phase 2 では VR（Voicebot+Record）のみ実装し、他の ActionCode は Phase 3 で実装

**理由**:
- VR は既存実装（voicebot 通常処理 + 録音フラグ制御）で実装できる
- ルール評価エンジンの動作検証には VR だけで十分
- 最もシンプルな ActionCode で動作検証してから、他の ActionCode を追加する方が安全

**Phase 3 で実装予定の ActionCode**:
- BZ（話中応答）
- NR（応答なし）
- AN（アナウンス再生）
- VM（留守番電話）
- IV（IVR フローへ移行）※ Phase 4 で IVR 実行エンジンも実装

### D-02: 未知 ActionCode のフォールバック

**決定**: 未知の ActionCode を受け取った場合、VR にフォールバック

**理由**:
- 設定ミスや DB 不整合時にエラーで止まるよりも、安全側（VR）で動作継続する方が良い
- ログに WARNING を出力して、後から問題を検出できるようにする

### D-03: recording_enabled / announce_enabled の扱い

**決定**: Phase 2 では recording_enabled のみ実装、announce_enabled は Phase 3 で実装

**理由**:
- recording_enabled は RecordingManager に enabled フィールドと set_enabled() メソッドを追加して対応可能
- announce_enabled はアナウンス再生ロジックが必要で、Phase 3 で AN（アナウンス再生）と一緒に実装する方が効率的

---

## 8. リスク・制約

### 8.1 リスク

| リスク | 影響度 | 発生確率 | 対策 |
|--------|--------|---------|------|
| ルール評価エンジンのパフォーマンス問題（DB クエリが遅い） | 中 | 低 | 初期実装では N+1 問題を避け、必要に応じてキャッシュ導入 |
| ActionCode の実装漏れ（VR 以外が動かない） | 高 | 低 | Phase 2 では VR のみ実装、他は Phase 3 で実装することを明示 |
| Frontend の設定と Backend の実装が一致しない | 高 | 中 | AC-3 で統合テスト実施、設定変更が実際の動作に反映されることを確認 |

### 8.2 制約

| 制約 | 理由 | 代替案 |
|------|------|--------|
| Phase 2 では VR のみ実装 | 最小限の ActionCode で動作検証 | - |
| announce_enabled は Phase 3 で実装 | アナウンス再生ロジックが必要 | - |
| IVR 実行エンジンは Phase 4 で実装 | ルール評価とは別の複雑なロジック | - |

---

## 9. 参照

| ドキュメント | セクション | 内容 |
|-------------|-----------|------|
| [STEER-137](STEER-137_backend-integration-strategy.md) | §5.2.4 | Issue #140 の定義 |
| [RD-004](virtual-voicebot-backend/docs/requirements/RD-004_call-routing-execution.md) | FR-1, FR-2 | ルール評価エンジン、ActionCode 実行の要件 |
| [BD-004](virtual-voicebot-backend/docs/design/basic/BD-004_call-routing-db.md) | §4 | registered_numbers, call_action_rules, routing_rules テーブル定義 |
| [contract.md](contract.md) | §3 | ActionCode の仕様 |
| [STEER-139](STEER-139_frontend-backend-sync-impl.md) | - | Frontend → Backend 同期実装（前提条件） |

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-08 | 初版作成（Draft） | Claude Code (claude-sonnet-4-5) |
| 2026-02-08 | Codex レビュー#1 指摘対応（重大3件、中3件、軽1件）：モジュール名修正（src/service/routing/...）、テーブル名修正（blocklist→spam_numbers、system_settings WHERE id=1）、JSON解釈修正（extra.defaultAction.actionConfig）、call_id ログ追加、VR定義明確化、AC分離（Phase3依存項目を後続へ）、電話番号正規化明示 | Claude Code (claude-sonnet-4-5) |
| 2026-02-08 | Codex レビュー#2 指摘対応（重大2件、中3件）：SessionCoordinator 統合ポイント修正（handle_invite→handle_control_event、port/adapter パターン対応）、Executor API 修正（SessionOut イベント駆動、outbound_mode/recording_manager 使用）、電話番号正規化強化（空白除去追加）、VR定義統一（outbound_mode=false + recording_enabled）、ActionConfigDto 追加（camelCase DTO 変換） | Claude Code (claude-sonnet-4-5) |
| 2026-02-08 | Codex レビュー#3 指摘対応（重大3件、中3件）：非通知判定順序修正（正規化前に実施）、RoutingPort 定義追加（call_log_port → 専用 Port）、SessionCoordinator 公開メソッド追加（set_outbound_mode/set_recording_enabled）、VR定義統一（voicebot 通常処理）、extract_caller_id 修正（Ok 型対応）、ActionExecutor::new() 追加 | Claude Code (claude-sonnet-4-5) |
| 2026-02-08 | Codex レビュー#4 指摘対応（重大2件、中2件、軽1件）：RoutingPort 注入経路明記（main.rs → spawn_session → SessionCoordinator::spawn → coordinator.routing_port）、RecordingManager.set_enabled() 仕様追加（enabled フィールド + start_main 条件分岐）、extract_user_from_to 使用（既存ヘルパー関数）、VR定義統一（B2BUA コンテキスト削除、voicebot 通常処理 + recording_enabled）、影響ファイルに mod.rs 追加 | Claude Code (claude-sonnet-4-5) |
| 2026-02-08 | Codex レビュー#5 指摘対応（中1件、軽1件）：VR定義統一（§2.2 達成目標の B2BUA転送 削除）、影響ファイルに handlers/mod.rs 追加 | Claude Code (claude-sonnet-4-5) |
| 2026-02-08 | 承認完了、ステータス → Approved：レビューサイクル完了（Codex 5回、全指摘対応完了）、実装フェーズへ引き継ぎ準備完了 | Claude Code (claude-sonnet-4-5) |
