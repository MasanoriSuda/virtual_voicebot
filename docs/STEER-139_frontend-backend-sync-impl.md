# STEER-139: Frontend → Backend 同期実装（Phase 1）

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-139 |
| タイトル | Frontend → Backend 同期実装（Phase 1: 同期基盤） |
| ステータス | Approved |
| 関連Issue | #139 |
| 優先度 | P0 |
| 作成日 | 2026-02-08 |
| 親ステアリング | STEER-137 |

---

## 2. ストーリー（Why）

### 2.1 背景

STEER-132/134 で Frontend PoC（着信アクション設定・IVR フロー管理）を JSON ファイルで実装したが、Backend はこれらの設定を実行できない状態にある。

**問題**:
- Frontend で設定した着信ルール・IVR フローが Backend で動作しない
- Backend DB に設定が同期されていない
- 着信時に Frontend の設定が反映されない

**影響**:
- Phase 2（ルール評価エンジン）、Phase 3（全 ActionCode 実装）に進めない
- エンドユーザーが設定した内容が実際の通話に反映されない

### 2.2 目的

Backend の Serversync が Frontend PoC の設定を Pull して Backend DB に保存する **同期基盤** を構築する。

**達成目標**:
- Frontend の JSON 設定が Backend DB に自動同期される
- 同期は Backend 契機の Pull 方式（30秒ごと）
- Frontend 停止時でも前回取得分で動作継続

### 2.3 ユーザーストーリー

```
As a システム管理者
I want to Frontend で設定した着信ルール・IVR フローが Backend DB に自動同期される
So that 設定変更が実際の通話処理に反映される（Phase 2 で実装）

受入条件:
- [ ] Frontend で番号グループを追加すると、30秒以内に Backend DB の registered_numbers に反映される
- [ ] Frontend で着信ルールを追加すると、30秒以内に Backend DB の call_action_rules に反映される
- [ ] Frontend で IVR フローを追加すると、30秒以内に Backend DB の ivr_nodes/ivr_transitions に反映される
- [ ] Frontend 停止中でも、Backend は前回取得分の設定で動作継続できる
- [ ] Serversync のログに同期状況（成功/失敗、取得件数）が出力される
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-08 |
| 起票理由 | Issue #138 完了後、STEER-137 の Phase 1 実装を開始 |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Code (claude-sonnet-4-5) |
| 作成日 | 2026-02-08 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "Issue #139 のステアリングファイルを作成" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| 1 | Codex | 2026-02-08 | 要修正 | 重大2件（announcements カラム名誤り、IVR変換非冪等）、中2件（エンドポイント名混在、DTO命名規則未定義）、軽1件（同期間隔不整合）→ 全て修正完了 |
| 2 | Codex | 2026-02-08 | 要修正 | 重大1件（announcementId null許容不整合）、中3件（announcementId解決未実装、CallerGroup削除反映不足、エンドポイント名残存）→ 全て修正完了 |
| 3 | Codex | 2026-02-08 | 要修正 | 中1件（RD-004 との変換値不整合）→ RD-004 を DB 制約に適合させる形で修正完了（v1.4） |
| 4 | Codex | 2026-02-08 | 要修正 | 中1件（マイグレーション実行コマンドのDB名/ユーザー不一致）→ sqlx migrate run に統一 |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | @MasanoriSuda |
| 承認日 | 2026-02-08 |
| 承認コメント | Codex レビュー 4回実施、全指摘対応完了（重大3件、中8件、軽2件）。実装フェーズへ |

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
| RD-004 | 修正（完了） | anonymousAction / defaultAction の未設定時フォールバック動作を追記（v1.3） |
| BD-004 | 参照 | call_action_rules、registered_numbers テーブル定義を参照 |
| contract.md | 修正（完了） | セクション 5.4「GET /api/ivr-flows」→「GET /api/ivr-flows/export」に変更（v2.2） |
| STEER-096 | 参照 | Serversync の仕組みを参照 |

### 4.2 影響するコード

| モジュール | 変更種別 | 概要 |
|-----------|---------|------|
| virtual-voicebot-frontend/app/api/number-groups/route.ts | 新規 | GET /api/number-groups 実装（JSON ファイルから読み取り） |
| virtual-voicebot-frontend/app/api/call-actions/route.ts | 修正 | GET メソッド追加（POST は既存、JSON ファイルから読み取り） |
| virtual-voicebot-frontend/app/api/ivr-flows/export/route.ts | 新規 | GET /api/ivr-flows/export 実装（JSON ファイルから読み取り） |
| virtual-voicebot-backend/src/bin/serversync.rs | 修正 | 設定 Pull ロジック追加（tokio タイマー、30秒間隔） |
| virtual-voicebot-backend/src/interface/sync/frontend_pull.rs | 新規 | Frontend 設定取得ロジック（3 API 呼び出し） |
| virtual-voicebot-backend/src/interface/sync/converters.rs | 新規 | データ変換処理（CallerGroup/IncomingRule/IvrFlow → Backend DB） |
| virtual-voicebot-backend/migrations/*.sql | 実行 | Issue #138 の DDL マイグレーション実行（2 ファイル） |

---

## 5. 差分仕様（What / How）

### 5.1 システム構成

```
┌─────────────────────────────────────────────────────────────┐
│                      Frontend (Next.js)                      │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌────────────────────┐        ┌─────────────────────────┐  │
│  │  JSON ファイル      │        │  GET API                │  │
│  │                    │        │                         │  │
│  │ - number-groups    │──────▶│ /api/number-groups      │  │
│  │ - call-actions     │──────▶│ /api/call-actions       │  │
│  │ - ivr-flows        │──────▶│ /api/ivr-flows/export   │  │
│  └────────────────────┘        └──────────┬──────────────┘  │
│                                           │                  │
└───────────────────────────────────────────┼──────────────────┘
                                            │
                                            │ HTTP GET (30秒ごと)
                                            │
┌───────────────────────────────────────────┼──────────────────┐
│                      Backend (Rust)       ▼                  │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌────────────────────────────────────────────────────┐     │
│  │              Serversync (独立バイナリ)              │     │
│  │                                                     │     │
│  │  ┌──────────────────┐     ┌───────────────────┐   │     │
│  │  │ tokio タイマー   │────▶│ Frontend Pull     │   │     │
│  │  │ (30秒間隔)       │     │ - GET 3 API       │   │     │
│  │  └──────────────────┘     └─────────┬─────────┘   │     │
│  │                                     │              │     │
│  │                            ┌────────▼──────────┐   │     │
│  │                            │ データ変換        │   │     │
│  │                            │ - CallerGroup →   │   │     │
│  │                            │   registered_nums │   │     │
│  │                            │ - IncomingRule →  │   │     │
│  │                            │   call_action_rls │   │     │
│  │                            │ - IvrFlow →       │   │     │
│  │                            │   ivr_nodes/trans │   │     │
│  │                            └─────────┬─────────┘   │     │
│  └──────────────────────────────────────┼─────────────┘     │
│                                          ▼                   │
│  ┌────────────────────────────────────────────────────┐     │
│  │                  Backend DB (PostgreSQL)            │     │
│  │                                                     │     │
│  │  - registered_numbers (group_id, group_name 追加)  │     │
│  │  - call_action_rules (新規テーブル)                │     │
│  │  - ivr_nodes, ivr_transitions (既存)               │     │
│  │  - system_settings.extra (anonymousAction 等)      │     │
│  └────────────────────────────────────────────────────┘     │
│                                                               │
└───────────────────────────────────────────────────────────────┘
```

### 5.2 Frontend 側実装

#### 5.2.1 GET /api/number-groups（新規）

**ファイル**: `virtual-voicebot-frontend/app/api/number-groups/route.ts`

**実装内容**:
```typescript
// number-groups.json から読み取って返す
export async function GET() {
  const data = await readJsonFile('number-groups.json');
  return NextResponse.json({
    ok: true,
    callerGroups: data.callerGroups || []
  });
}
```

**レスポンス仕様**: contract.md §5.4 参照

#### 5.2.2 GET /api/call-actions（追加）

**ファイル**: `virtual-voicebot-frontend/app/api/call-actions/route.ts`

**実装内容**:
```typescript
// 既存: POST メソッド（設定保存）
// 追加: GET メソッド（設定取得）
export async function GET() {
  const data = await readJsonFile('call-actions.json');
  return NextResponse.json({
    ok: true,
    rules: data.rules || [],
    anonymousAction: data.anonymousAction || { actionType: 'deny', actionConfig: { actionCode: 'BZ' } },
    defaultAction: data.defaultAction || { actionType: 'allow', actionConfig: { actionCode: 'VR' } }
  });
}
```

**レスポンス仕様**: contract.md §5.4 参照

#### 5.2.3 GET /api/ivr-flows/export（決定）

**決定**: 案A（エンドポイント分離）を採用

**エンドポイント**:
```
GET /api/ivr-flows        → Backend DB から取得（既存、将来用）
GET /api/ivr-flows/export → Frontend JSON から取得（Backend Pull 用）
```

**理由**:
- 既存の `GET /api/ivr-flows`（Backend DB 取得）と責務分離
- 将来の衝突回避
- 明確な API 設計

**ファイル**: `virtual-voicebot-frontend/app/api/ivr-flows/export/route.ts`（新規）

**実装内容**:
```typescript
// ivr-flows.json から読み取って返す
export async function GET() {
  const data = await readJsonFile('ivr-flows.json');
  return NextResponse.json({
    ok: true,
    flows: data.flows || []
  });
}
```

**レスポンス仕様**: contract.md §5.4 参照

### 5.3 Backend 側実装

#### 5.3.1 Serversync の拡張

**ファイル**: `virtual-voicebot-backend/src/bin/serversync.rs`

**実装内容**:

```rust
use tokio::time::{interval, Duration};

#[tokio::main]
async fn main() -> Result<()> {
    // 既存: Outbox Worker（Backend → Frontend 同期）
    tokio::spawn(async move {
        outbox_worker().await;
    });

    // 新規: Frontend Pull Worker（Frontend → Backend 同期）
    tokio::spawn(async move {
        frontend_pull_worker().await;
    });

    // メインループ
    tokio::signal::ctrl_c().await?;
    Ok(())
}

async fn frontend_pull_worker() {
    // 同期間隔: 環境変数 FRONTEND_SYNC_INTERVAL_SEC（デフォルト: 30秒）
    // 既存の SYNC_POLL_INTERVAL_SEC（Backend→Frontend、デフォルト300秒）とは別管理
    let interval_sec = std::env::var("FRONTEND_SYNC_INTERVAL_SEC")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(30);

    let mut interval = interval(Duration::from_secs(interval_sec));

    loop {
        interval.tick().await;

        match pull_frontend_settings().await {
            Ok(_) => info!("[Serversync] Frontend 設定同期完了"),
            Err(e) => warn!("[Serversync] Frontend 設定同期失敗: {}", e),
        }
    }
}
```

#### 5.3.2 Frontend 設定 Pull

**ファイル**: `virtual-voicebot-backend/src/interface/sync/frontend_pull.rs`（新規）

**DTO 定義**:

Frontend は TypeScript (camelCase) を使用するため、Rust 側 DTO に `#[serde(rename_all = "camelCase")]` を付与する。

```rust
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallerGroup {
    pub id: Uuid,
    pub name: String,
    pub phone_numbers: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IncomingRule {
    pub id: Uuid,
    pub name: String,
    pub caller_group_id: Option<Uuid>,
    pub action_type: String,
    pub action_config: serde_json::Value,
    pub is_active: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IvrFlowDefinition {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub announcement_id: Option<Uuid>,  // Frontend では null 許容（ivr-flows.ts line 41）
    pub timeout_sec: i32,
    pub max_retries: i32,
    pub routes: Vec<IvrRoute>,
    pub fallback_action: ActionDestination,
    pub is_active: bool,
}
```

**実装内容**:

```rust
pub async fn pull_frontend_settings() -> Result<()> {
    info!("[Serversync] Frontend から設定取得開始");

    // 1. GET /api/number-groups
    let groups = fetch_number_groups().await?;
    info!("[Serversync] GET /api/number-groups: 成功, グループ数={}", groups.len());

    // 2. GET /api/call-actions
    let actions = fetch_call_actions().await?;
    info!("[Serversync] GET /api/call-actions: 成功, ルール数={}", actions.rules.len());

    // 3. GET /api/ivr-flows/export
    let flows = fetch_ivr_flows_export().await?;
    info!("[Serversync] GET /api/ivr-flows/export: 成功, フロー数={}", flows.len());

    // 4. Backend DB に保存
    save_to_backend_db(groups, actions, flows).await?;
    info!("[Serversync] Backend DB に保存完了");

    Ok(())
}
```

#### 5.3.3 データ変換処理

**ファイル**: `virtual-voicebot-backend/src/interface/sync/converters.rs`（新規）

**実装内容**:

```rust
// CallerGroup → registered_numbers 変換（冪等性確保）
pub async fn convert_caller_groups(
    groups: Vec<CallerGroup>,
    pool: &PgPool
) -> Result<()> {
    // Frontend から削除された番号の group_id/group_name をクリア
    let all_phone_numbers: Vec<String> = groups
        .iter()
        .flat_map(|g| g.phone_numbers.clone())
        .collect();

    if !all_phone_numbers.is_empty() {
        // Frontend に存在しない番号の group_id/group_name を NULL 化
        sqlx::query!(
            r#"
            UPDATE registered_numbers
            SET group_id = NULL, group_name = NULL, updated_at = NOW()
            WHERE phone_number != ALL($1)
              AND group_id IS NOT NULL
              AND deleted_at IS NULL
            "#,
            &all_phone_numbers
        )
        .execute(pool)
        .await?;
    } else {
        // Frontend のグループが空の場合、全ての group_id/group_name を NULL 化
        sqlx::query!(
            r#"
            UPDATE registered_numbers
            SET group_id = NULL, group_name = NULL, updated_at = NOW()
            WHERE group_id IS NOT NULL AND deleted_at IS NULL
            "#
        )
        .execute(pool)
        .await?;
    }

    // グループごとに番号を upsert
    for group in groups {
        for phone_number in &group.phone_numbers {
            sqlx::query!(
                r#"
                INSERT INTO registered_numbers
                    (id, phone_number, group_id, group_name, category, action_code, recording_enabled, announce_enabled, created_at, updated_at)
                VALUES
                    (gen_random_uuid(), $1, $2, $3, 'general', 'VR', TRUE, TRUE, NOW(), NOW())
                ON CONFLICT (phone_number) WHERE deleted_at IS NULL
                DO UPDATE SET
                    group_id = $2,
                    group_name = $3,
                    updated_at = NOW()
                "#,
                phone_number,
                group.id,
                group.name
            )
            .execute(pool)
            .await?;
        }
    }
    Ok(())
}

// IncomingRule → call_action_rules 変換
pub async fn convert_incoming_rules(
    rules: Vec<IncomingRule>,
    pool: &PgPool
) -> Result<()> {
    // 既存ルールを削除（Frontend で削除されたルールを反映）
    sqlx::query!("DELETE FROM call_action_rules").execute(pool).await?;

    for (index, rule) in rules.iter().enumerate() {
        sqlx::query!(
            r#"
            INSERT INTO call_action_rules
                (id, name, caller_group_id, action_type, action_config, priority, is_active, created_at, updated_at)
            VALUES
                ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW())
            "#,
            rule.id,
            rule.name,
            rule.caller_group_id,
            rule.action_type,
            serde_json::to_value(&rule.action_config)?,
            index as i32,
            rule.is_active
        )
        .execute(pool)
        .await?;
    }
    Ok(())
}

// IvrFlowDefinition → ivr_nodes/ivr_transitions 変換（冪等性確保）
pub async fn convert_ivr_flows(
    flows: Vec<IvrFlowDefinition>,
    pool: &PgPool
) -> Result<()> {
    // Frontend から削除されたフローの対応（Frontend JSONに含まれないフローを削除）
    let frontend_flow_ids: Vec<Uuid> = flows.iter().map(|f| f.id).collect();

    if !frontend_flow_ids.is_empty() {
        // Frontend にないフローを削除
        sqlx::query!(
            "DELETE FROM ivr_flows WHERE id != ALL($1)",
            &frontend_flow_ids
        )
        .execute(pool)
        .await?;
    } else {
        // Frontend のフローが空の場合、全削除
        sqlx::query!("DELETE FROM ivr_flows").execute(pool).await?;
    }

    for flow in flows {
        // ivr_flows に挿入
        sqlx::query!(
            r#"
            INSERT INTO ivr_flows (id, name, description, is_active, created_at, updated_at)
            VALUES ($1, $2, $3, $4, NOW(), NOW())
            ON CONFLICT (id) DO UPDATE SET
                name = $2,
                description = $3,
                is_active = $4,
                updated_at = NOW()
            "#,
            flow.id,
            flow.name,
            flow.description,
            flow.is_active
        )
        .execute(pool)
        .await?;

        // 既存のノード/遷移を削除（冪等性確保、Frontend 削除反映）
        sqlx::query!(
            "DELETE FROM ivr_transitions WHERE from_node_id IN (SELECT id FROM ivr_nodes WHERE flow_id = $1)",
            flow.id
        )
        .execute(pool)
        .await?;

        sqlx::query!(
            "DELETE FROM ivr_nodes WHERE flow_id = $1",
            flow.id
        )
        .execute(pool)
        .await?;

        // ルートノード（ANNOUNCE）作成
        let root_node_id = Uuid::new_v4();

        // announcementId → audio_file_url 解決（D-02）
        let audio_file_url = match flow.announcement_id {
            Some(announcement_id) => resolve_announcement_url(announcement_id, pool).await?,
            None => None,
        };

        sqlx::query!(
            r#"
            INSERT INTO ivr_nodes
                (id, flow_id, parent_id, node_type, audio_file_url, depth, timeout_sec, max_retries, created_at, updated_at)
            VALUES
                ($1, $2, NULL, 'ANNOUNCE', $3, 0, $4, $5, NOW(), NOW())
            "#,
            root_node_id,
            flow.id,
            audio_file_url,
            flow.timeout_sec,
            flow.max_retries
        )
        .execute(pool)
        .await?;

        // KEYPAD ノード作成
        let keypad_node_id = Uuid::new_v4();
        sqlx::query!(
            r#"
            INSERT INTO ivr_nodes
                (id, flow_id, parent_id, node_type, depth, timeout_sec, max_retries, created_at, updated_at)
            VALUES
                ($1, $2, $3, 'KEYPAD', 1, $4, $5, NOW(), NOW())
            "#,
            keypad_node_id,
            flow.id,
            root_node_id,
            flow.timeout_sec,
            flow.max_retries
        )
        .execute(pool)
        .await?;

        // routes → ivr_transitions 作成
        for route in &flow.routes {
            let destination_node_id = create_destination_node(&route.destination, &flow.id, pool).await?;

            sqlx::query!(
                r#"
                INSERT INTO ivr_transitions
                    (id, from_node_id, input_type, dtmf_key, to_node_id, created_at)
                VALUES
                    ($1, $2, 'DTMF', $3, $4, NOW())
                "#,
                Uuid::new_v4(),
                keypad_node_id,
                route.dtmf_key,
                destination_node_id
            )
            .execute(pool)
            .await?;
        }

        // fallbackAction → ivr_transitions 作成（TIMEOUT / INVALID）
        let fallback_node_id = create_destination_node(&flow.fallback_action, &flow.id, pool).await?;

        for input_type in ["TIMEOUT", "INVALID"] {
            sqlx::query!(
                r#"
                INSERT INTO ivr_transitions
                    (id, from_node_id, input_type, dtmf_key, to_node_id, created_at)
                VALUES
                    ($1, $2, $3, NULL, $4, NOW())
                "#,
                Uuid::new_v4(),
                keypad_node_id,
                input_type,
                fallback_node_id
            )
            .execute(pool)
            .await?;
        }
    }
    Ok(())
}
```

### 5.4 DDL マイグレーション実行

**ファイル**: Issue #138 で作成した DDL

**実行方法**:

```bash
# 開発環境で実行（推奨）
cd virtual-voicebot-backend
sqlx migrate run
```

> **注**: 開発環境のDB設定は `docker-compose.dev.yml` を参照（DB名: voicebot、ユーザー名: voicebot）。
> `sqlx migrate run` は `DATABASE_URL` 環境変数から自動的に接続情報を取得する。

### 5.5 エラーハンドリング

#### 5.5.1 Frontend 停止時の挙動

**設計方針**: RD-004 FR-4.4 参照

- Frontend API が 404/500/timeout を返す場合、前回取得分で動作継続
- ログに WARNING レベルで出力
- 次回 Pull 時にリトライ

**実装**:
```rust
match fetch_number_groups().await {
    Ok(groups) => { /* 正常処理 */ },
    Err(e) => {
        warn!("[Serversync] GET /api/number-groups 失敗: {}", e);
        warn!("[Serversync] 前回取得分で動作継続");
        return Ok(()); // エラーでも終了しない
    }
}
```

#### 5.5.2 データ変換失敗時の挙動

**設計方針**:
- データ変換中にエラーが発生した場合、**ロールバック**
- 前回取得分で動作継続
- ログに ERROR レベルで出力

**実装**:
```rust
let mut tx = pool.begin().await?;

match convert_all_data(&mut tx, groups, actions, flows).await {
    Ok(_) => tx.commit().await?,
    Err(e) => {
        error!("[Serversync] データ変換失敗: {}", e);
        tx.rollback().await?;
        warn!("[Serversync] 前回取得分で動作継続");
    }
}
```

### 5.6 ログ出力

**ログレベル**:
- INFO: 正常な同期処理（取得開始、取得成功、保存完了）
- WARN: 一時的な失敗（Frontend 停止、API エラー）
- ERROR: データ変換失敗、致命的エラー

**ログ形式**: RD-004 NFR-1.3 参照

```
[Serversync] Frontend から設定取得開始
[Serversync] GET /api/number-groups: 成功, グループ数=3
[Serversync] GET /api/call-actions: 成功, ルール数=5
[Serversync] GET /api/ivr-flows/export: 成功, フロー数=2
[Serversync] Backend DB に保存完了
```

---

## 6. 受入条件（Acceptance Criteria）

### AC-1: Frontend GET API 実装
- [ ] GET /api/number-groups が number-groups.json から CallerGroup 一覧を返す
- [ ] GET /api/call-actions が call-actions.json から IncomingRule + anonymousAction + defaultAction を返す
- [ ] GET /api/ivr-flows/export が ivr-flows.json から IvrFlowDefinition 一覧を返す
- [ ] レスポンス形式が contract.md §5.4 に準拠する

### AC-2: Backend Serversync 実装
- [ ] Serversync が 30秒ごとに Frontend の 3 つの GET API を呼び出す（/api/number-groups, /api/call-actions, /api/ivr-flows/export）
- [ ] CallerGroup → registered_numbers 変換が正しく動作する（group_id, group_name の保存）
- [ ] IncomingRule → call_action_rules 変換が正しく動作する（priority の設定）
- [ ] IvrFlowDefinition → ivr_nodes/ivr_transitions 変換が正しく動作する（ルートノード、KEYPAD ノード、遷移の作成）
- [ ] announcementId → audio_file_url の解決が正しく動作する（announcements テーブル参照）
- [ ] anonymousAction / defaultAction が system_settings.extra (JSONB) に保存される
- [ ] anonymousAction / defaultAction が未設定の場合、RD-004 のフォールバック値が使用される

### AC-3: エラーハンドリング
- [ ] Frontend 停止時でも、Serversync はエラーで終了せず、前回取得分で動作継続する
- [ ] データ変換失敗時、トランザクションがロールバックされ、前回取得分で動作継続する
- [ ] Frontend API が 404/500/timeout を返した場合、WARN ログが出力される

### AC-4: ログ出力
- [ ] 同期開始時に INFO ログ「Frontend から設定取得開始」が出力される
- [ ] 各 API 呼び出し成功時に INFO ログ「GET /api/xxx: 成功, 件数=N」が出力される
- [ ] 保存完了時に INFO ログ「Backend DB に保存完了」が出力される
- [ ] 取得失敗時に WARN ログが出力される

### AC-5: 統合テスト
- [ ] Frontend で番号グループを追加（JSON 保存）→ 30秒以内に Backend DB の registered_numbers に反映される
- [ ] Frontend で着信ルールを追加（JSON 保存）→ 30秒以内に Backend DB の call_action_rules に反映される
- [ ] Frontend で IVR フローを追加（JSON 保存）→ 30秒以内に Backend DB の ivr_nodes/ivr_transitions に反映される
- [ ] Frontend で設定を削除（JSON 更新）→ 30秒以内に Backend DB から削除される

---

## 7. 設計決定事項（Design Decisions）

### D-01: GET /api/ivr-flows のエンドポイント名

**決定**: `GET /api/ivr-flows/export` を採用

**理由**:
- 既存の `GET /api/ivr-flows`（Backend DB 取得）と責務分離
- 将来の衝突回避
- 明確な API 設計

### D-02: IVR 変換の announcementId → audio_file_url 解決

**決定**: Backend が announcements テーブルを参照して URL を解決

**理由**:
- Frontend に URL 責務を持たせず、実行エンジンの一貫性を優先
- Backend が announcements テーブルの SoT を管理（contract.md §1.3）

**null 許容**:
- Frontend の `announcementId` は `string | null`（ivr-flows.ts line 41）
- Rust DTO は `Option<Uuid>` で対応
- `None` の場合、`ivr_nodes.audio_file_url` も `NULL` として保存

**実装方針**:
```rust
// Backend Serversync の変換処理
async fn resolve_announcement_url(announcement_id: Uuid, pool: &PgPool) -> Result<Option<String>> {
    let result = sqlx::query!(
        "SELECT audio_file_url FROM announcements WHERE id = $1",
        announcement_id
    )
    .fetch_optional(pool)
    .await?;

    match result {
        Some(row) => Ok(row.audio_file_url),  // audio_file_url は nullable (Option<String>)
        None => {
            warn!("[Serversync] announcementId {} not found", announcement_id);
            Ok(None)
        }
    }
}
```

### D-03: anonymousAction / defaultAction の保存先

**決定**: `system_settings.extra` (JSONB) に保存

**理由**:
- 拡張性優先（RD-004 の方針）
- システム全体のデフォルト動作として管理

**未設定時のフォールバック**（RD-004 に追記必要）:
```json
{
  "anonymousAction": {
    "actionType": "deny",
    "actionConfig": { "actionCode": "BZ" }
  },
  "defaultAction": {
    "actionType": "allow",
    "actionConfig": { "actionCode": "VR" }
  }
}
```

**保存形式**:
```rust
// system_settings.extra (JSONB)
{
  "anonymousAction": { /* ... */ },
  "defaultAction": { /* ... */ }
}
```

### D-04: IVR変換の冪等性確保

**決定**: 削除＋再作成方式（delete-and-recreate）を採用

**理由**:
- Frontend で削除された設定を Backend DB に反映する必要がある（AC-5）
- 冪等性を保証し、同期を繰り返しても同じ結果になる

**実装方針**:
```rust
// 1. Frontend から削除されたフロー全体を削除
DELETE FROM ivr_flows WHERE id != ALL($frontend_flow_ids)

// 2. 各フローごとに既存ノード/遷移を削除
DELETE FROM ivr_transitions WHERE from_node_id IN (SELECT id FROM ivr_nodes WHERE flow_id = $1)
DELETE FROM ivr_nodes WHERE flow_id = $1

// 3. ノード/遷移を再作成（新規IDで作成）
INSERT INTO ivr_nodes (id, ...) VALUES (gen_random_uuid(), ...)
```

**トレードオフ**:
- メリット: シンプル、確実に冪等性が保証される
- デメリット: created_at がリセットされる（監査ログで対応）
- 代替案: 決定論的IDを使った upsert 方式（より複雑）

---

## 8. リスク・制約

### 8.1 リスク

| リスク | 影響度 | 発生確率 | 対策 |
|--------|--------|---------|------|
| Frontend API が返す JSON 形式が想定と異なる | 高 | 中 | Frontend のスキーマバリデーションを実装 |
| IVR 変換ロジックが複雑で実装に時間がかかる | 中 | 高 | MVP では最小限の IVR 構造のみサポート（depth=1） |
| Serversync の定期実行が Backend の他の処理に影響する | 中 | 低 | tokio の非同期タスクで分離、CPU 負荷を監視 |
| Frontend 停止時の前回取得分が古い場合、設定が反映されない | 低 | 低 | 起動時に必ず Pull 実行 |

### 8.2 制約

| 制約 | 理由 | 代替案 |
|------|------|--------|
| MVP では IVR の depth=1 のみサポート | Phase 1 はデータ同期のみ、実行は Phase 2 | - |
| Frontend API 認証なし | MVP はローカル / 閉域想定（contract.md §7） | 将来 X-Api-Key 導入 |
| 同期頻度はデフォルト 30秒（環境変数で変更可能） | Frontend→Backend 同期は即時性が重要（設定変更の反映速度）。既存の Backend→Frontend 同期（SYNC_POLL_INTERVAL_SEC=300秒）とは別管理 | 環境変数 FRONTEND_SYNC_INTERVAL_SEC で調整可能 |

---

## 9. 参照

| ドキュメント | セクション | 内容 |
|-------------|-----------|------|
| [STEER-137](STEER-137_backend-integration-strategy.md) | §5.2.4 | Issue #139 の定義 |
| [RD-004](virtual-voicebot-backend/docs/requirements/RD-004_call-routing-execution.md) | FR-4 | Frontend → Backend 設定同期の要件 |
| [BD-004](virtual-voicebot-backend/docs/design/basic/BD-004_call-routing-db.md) | §4.3.1 | call_action_rules, registered_numbers テーブル定義 |
| [contract.md](contract.md) | §5.4 | Frontend 設定公開 API 仕様 |
| [STEER-096](virtual-voicebot-backend/docs/steering/STEER-096_serversync.md) | - | Serversync の仕組み |

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-08 | 初版作成（Draft） | Claude Code (claude-sonnet-4-5) |
| 2026-02-08 | 設計決定反映（Q1〜Q3）：GET /api/ivr-flows/export 採用、announcementId 解決方式、anonymousAction/defaultAction フォールバック定義、ステータス → Review | Claude Code (claude-sonnet-4-5) |
| 2026-02-08 | Codexレビュー指摘対応（重大2件、中2件、軽1件）：announcements テーブルカラム名修正（file_path→audio_file_url）、IVR変換冪等性確保（削除＋再作成方式）、エンドポイント名統一（/api/ivr-flows/export）、DTO命名規則明示（#[serde(rename_all = "camelCase")]）、同期間隔明確化（FRONTEND_SYNC_INTERVAL_SEC=30秒） | Claude Code (claude-sonnet-4-5) |
| 2026-02-08 | Codex再レビュー指摘対応（重大1件、中3件）：announcementId を Option<Uuid> に修正、resolve_announcement_url 実装（Option<String> 返却）、CallerGroup 削除反映ロジック追加（group_id/group_name NULL 化）、システム構成図エンドポイント名修正 | Claude Code (claude-sonnet-4-5) |
| 2026-02-08 | Codex再確認指摘対応（中1件）：RD-004 の CallerGroup 変換値を DB 制約（20260206000005_create_registered_numbers.sql）に適合させる形で修正（category='general', action_code='VR', recording_enabled=true, announce_enabled=true）、RD-004 v1.4 に反映 | Claude Code (claude-sonnet-4-5) |
| 2026-02-08 | Codex再々確認指摘対応（中1件）：マイグレーション実行コマンドを `sqlx migrate run` に統一（開発環境 DB 設定との整合性確保、docker-compose.dev.yml 参照） | Claude Code (claude-sonnet-4-5) |
| 2026-02-08 | 承認完了、ステータス → Approved：レビューサイクル完了（Codex 4回、全指摘対応完了）、実装フェーズへ引き継ぎ準備完了 | Claude Code (claude-sonnet-4-5) |
