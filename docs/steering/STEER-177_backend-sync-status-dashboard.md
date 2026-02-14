# STEER-177: Backend同期状態可視化ダッシュボード

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-177 |
| タイトル | Backend同期状態可視化ダッシュボード |
| ステータス | Approved |
| 関連Issue | #177 |
| 優先度 | P1 |
| 作成日 | 2026-02-14 |

---

## 2. ストーリー（Why）

### 2.1 背景

現在、Frontend → Backend の設定同期は serversync の frontend_pull worker による Pull 型同期で実現している（contract.md §5.4）。Frontend の `call-actions.json` を定期的に Backend が Pull して `call_action_rules` テーブルに保存する。しかし、以下の課題がある:

1. **同期状態の可視性がない**: Frontend 側から「着信アクション設定が Backend に反映されたか」を確認する手段がない
2. **障害時の検知遅延**: frontend_pull worker が停止したり、ネットワーク障害で同期が滞っても、ユーザーが気づけない
3. **運用負荷**: 着信アクション（call-actions.json）を変更しても、Backend に反映されたか確認できない

特に、運用者が着信アクション設定を変更した際に「設定が反映されたか」を確認したいニーズが高い。

### 2.2 目的

Frontend のダッシュボードに Backend の着信アクション同期状態を可視化し、以下を実現する:

1. **即時可視性**: Backend の `call_action_rules` テーブルの最新状態をリアルタイムで確認
2. **障害検知**: 同期遅延や frontend_pull worker 停止を早期発見
3. **運用安心**: 着信アクション変更後、反映状態を確認してから運用開始できる

### 2.3 ユーザーストーリー

```
As a システム運用者
I want to ダッシュボードで Backend の着信アクション同期状態を確認する
So that 設定変更が正しく反映されたか、システムが正常に稼働しているかを把握できる

受入条件:
- [ ] ダッシュボードで着信アクション（call_action_rules）の同期状態を表示
- [ ] Backend テーブルの最終更新日時、エントリ数を表示
- [ ] 最終更新からの経過時間を表示
- [ ] 同期遅延時にアラート表示（例: 10分以上更新がない場合）
- [ ] 手動更新ボタンで表示をリフレッシュ
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-14 |
| 起票理由 | Backend の同期状態を Frontend で確認する必要性 |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Code (claude-sonnet-4-5) |
| 作成日 | 2026-02-14 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "Issue #177: フロントエンドのダッシュボードでバックエンドの同期状態を確認する仕様を作成" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| 1 | Codex | 2026-02-14 | 要修正 | スコープ、実装方式、エンティティ定義、受入条件の矛盾等を指摘 |
| 2 | Codex | 2026-02-14 | 要修正 | /api/sync/status 追加位置、Backend URL、PgPool、lastUpdatedAt、パフォーマンス注記、レビューチェックリスト等を指摘 |
| 3 | Codex | 2026-02-14 | 要修正 | ルール0件時の heartbeat 対応、エラーレスポンス形式を指摘 |
| 4 | Codex | 2026-02-14 | OK | 軽微な指摘のみ（main.rs の .await、requestId コメント、テスト観点追加）で承認 |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | @MasanoriSuda |
| 承認日 | 2026-02-14 |
| 承認コメント | Codex レビュー OK、実装へ |

### 3.5 実装（該当する場合）

| 項目 | 値 |
|------|-----|
| 実装者 | - |
| 実装日 | - |
| 指示者 | - |
| 指示内容 | - |
| コードレビュー | - |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ実行者 | - |
| マージ日 | - |
| マージ先 | contract.md, Backend DD, Frontend DD |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| docs/contract.md | 追加 | §5.2 に GET /api/sync/status エンドポイント追加 |
| virtual-voicebot-backend/docs/design/detail/DD-xxx.md | 追加 | sync status API の詳細設計 |
| virtual-voicebot-frontend/docs/design/detail/DD-xxx.md | 追加 | Dashboard 同期状態ウィジェットの詳細設計 |
| virtual-voicebot-frontend/docs/test/unit/UT-xxx.md | 追加 | 同期状態表示コンポーネントのテスト |

### 4.2 影響するコード

| モジュール | 変更種別 | 概要 |
|-----------|---------|------|
| virtual-voicebot-backend/src/interface/http/mod.rs | 修正 | `/api/sync/status` エンドポイント追加、DB Pool 渡し |
| virtual-voicebot-backend/src/main.rs | 修正 | spawn_recording_server に PgPool 渡す |
| virtual-voicebot-frontend/app/api/sync-status/route.ts | 追加 | Backend Proxy API（新規ファイル） |
| virtual-voicebot-frontend/components/CallActionsSyncWidget.tsx | 追加 | 同期状態表示ウィジェット（新規ファイル） |
| virtual-voicebot-frontend/components/dashboard-content.tsx | 修正 | CallActionsSyncWidget 配置 |
| virtual-voicebot-frontend/lib/api/sync-status.ts | 追加 | Proxy API クライアント（新規ファイル） |

---

## 5. 差分仕様（What / How）

### 5.1 Backend: Call Actions Sync Status API

#### 5.1.1 エンドポイント定義（contract.md へマージ）

**追加先**: `docs/contract.md` §5.2（Frontend → Backend API）

```markdown
| メソッド | パス | 説明 |
|---------|------|------|
| GET | /api/sync/status | Backend 着信アクション同期状態取得 |

#### GET /api/sync/status

Backend の `call_action_rules` テーブルの同期状態を返す。Frontend が着信アクション設定の反映状況を確認するために使用する。

**リクエスト**: なし（クエリパラメータなし）

**レスポンス**:
```json
{
  "ok": true,
  "callActionsSync": {
    "lastUpdatedAt": "2026-02-14T10:00:00.000Z",
    "ruleCount": 5,
    "elapsedMinutes": 3
  }
}
```

**フィールド説明**:

| フィールド | 型 | 説明 |
|-----------|-----|------|
| callActionsSync.lastUpdatedAt | string (ISO8601) \| null | 着信アクション同期の最終更新日時。call_action_rules の MAX(updated_at) または system_settings の updated_at（ルール0件時の heartbeat）。frontend_pull が一度も成功していない場合のみ null |
| callActionsSync.ruleCount | number | call_action_rules テーブルのアクティブエントリ数（is_active = TRUE のみカウント） |
| callActionsSync.elapsedMinutes | number \| null | 最終更新からの経過時間（分）。lastUpdatedAt が null の場合は null |

**処理内容**:
1. `call_action_rules` テーブルから `MAX(updated_at)` を取得（全件対象、is_active 問わず）
2. ルールが0件の場合は `system_settings.updated_at` を使用（frontend_pull の heartbeat）
3. `COALESCE(MAX(call_action_rules.updated_at), (SELECT updated_at FROM system_settings WHERE id=1))` で取得
4. `call_action_rules` テーブルから `COUNT(*)` を取得（is_active = TRUE のみ）
5. 現在時刻と lastUpdatedAt の差分を計算して elapsedMinutes を算出
```

#### 5.1.2 実装方針

**変更ファイル**: `virtual-voicebot-backend/src/interface/http/mod.rs`（既存ファイル修正）

既存の独自 TCP HTTP サーバに `/api/sync/status` エンドポイントを追加する。

**実装概要**:

1. `handle_conn` 関数内で `/api/sync/status` パスを判定
2. DB から `call_action_rules` テーブルの情報を取得
3. JSON レスポンスを返却

**実装イメージ**:

```rust
// handle_conn 関数内（IMPORTANT: /recordings/ 判定より前に配置）

// /api/sync/status エンドポイント（/recordings/ より前に判定）
if method == "GET" && path == "/api/sync/status" {
    match &pool {
        Some(p) => {
            match get_sync_status_json(p).await {
                Ok(json_response) => {
                    return write_json_response(socket, 200, "OK", json_response.as_bytes()).await;
                }
                Err(_) => {
                    // MVP では requestId を省略（将来的に追加可能）
                    let error_json = r#"{"error":{"code":"INTERNAL_ERROR","message":"Database error"}}"#;
                    return write_json_response(socket, 500, "Internal Server Error", error_json.as_bytes()).await;
                }
            }
        }
        None => {
            // MVP では requestId を省略（将来的に追加可能）
            let error_json = r#"{"error":{"code":"SERVICE_UNAVAILABLE","message":"Database not available"}}"#;
            return write_json_response(socket, 503, "Service Unavailable", error_json.as_bytes()).await;
        }
    }
}

let is_get = method == "GET";
let is_head = method == "HEAD";
if (!is_get && !is_head) || !path.starts_with("/recordings/") {
    // 既存の 404 処理...
}

// 既存の /recordings/ 処理...
```

**新規関数**:

```rust
use serde::Serialize;
use chrono::{DateTime, Utc};

#[derive(Serialize)]
struct SyncStatusResponse {
    ok: bool,
    #[serde(rename = "callActionsSync")]
    call_actions_sync: CallActionsSync,
}

#[derive(Serialize)]
struct CallActionsSync {
    #[serde(rename = "lastUpdatedAt")]
    last_updated_at: Option<DateTime<Utc>>,
    #[serde(rename = "ruleCount")]
    rule_count: i64,
    #[serde(rename = "elapsedMinutes")]
    elapsed_minutes: Option<i64>,
}

async fn get_sync_status_json(pool: &PgPool) -> Result<String, std::io::Error> {
    // 1. call_action_rules の最終更新日時を取得（全件対象）
    //    ルールが0件の場合は system_settings.updated_at を使用（frontend_pull の heartbeat）
    let last_updated_at: Option<DateTime<Utc>> = sqlx::query_scalar(
        "SELECT COALESCE(
            (SELECT MAX(updated_at) FROM call_action_rules),
            (SELECT updated_at FROM system_settings WHERE id = 1)
        )"
    )
    .fetch_one(pool)
    .await
    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    // 2. call_action_rules のアクティブエントリ数を取得
    let rule_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM call_action_rules WHERE is_active = TRUE"
    )
    .fetch_one(pool)
    .await
    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    // 3. 経過時間を計算
    let elapsed_minutes = last_updated_at.map(|ts| {
        let now = Utc::now();
        (now - ts).num_minutes()
    });

    let response = SyncStatusResponse {
        ok: true,
        call_actions_sync: CallActionsSync {
            last_updated_at,
            rule_count,
            elapsed_minutes,
        },
    };

    serde_json::to_string(&response)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
}

async fn write_json_response(
    socket: &mut tokio::net::TcpStream,
    status: u16,
    reason: &str,
    body: &[u8],
) -> std::io::Result<()> {
    let headers = [
        ("Content-Type", "application/json".to_string()),
        ("Access-Control-Allow-Origin", "*".to_string()),
    ];
    write_response_with_headers(
        socket,
        status,
        reason,
        &headers,
        body,
        body.len() as u64,
        true,
    )
    .await
}
```

**DB Pool の渡し方**:

`spawn_recording_server` のシグネチャを変更して `Option<PgPool>` を受け取る（現行の「DBなしでも起動可能」設計を維持）:

```rust
pub async fn spawn_recording_server(bind: &str, base_dir: PathBuf, pool: Option<PgPool>) {
    // ...
    // handle_conn 内で pool.as_ref() を使用
}
```

**main.rs での呼び出し**:

```rust
// postgres_adapter が None の場合もある
let pool = postgres_adapter.as_ref().map(|adapter| adapter.pool().clone());
spawn_recording_server(&bind, base_dir, pool).await;
```

---

### 5.2 Frontend: Call Actions Sync Status Widget

#### 5.2.1 Backend Proxy API（中継エンドポイント）

**ファイル**: `virtual-voicebot-frontend/app/api/sync-status/route.ts`（新規作成）

```typescript
import { NextResponse } from "next/server"

export interface SyncStatusResponse {
  ok: boolean
  callActionsSync: CallActionsSync
}

export interface CallActionsSync {
  lastUpdatedAt: string | null
  ruleCount: number
  elapsedMinutes: number | null
}

export async function GET() {
  try {
    const backendUrl = process.env.BACKEND_URL || "http://localhost:18080"
    const response = await fetch(`${backendUrl}/api/sync/status`, {
      method: "GET",
      headers: {
        "Content-Type": "application/json",
      },
      cache: "no-store",
    })

    if (!response.ok) {
      return NextResponse.json(
        { error: "Failed to fetch sync status from backend" },
        { status: 502 }
      )
    }

    const data: SyncStatusResponse = await response.json()
    return NextResponse.json(data)
  } catch (error) {
    console.error("[sync-status] Error fetching from backend:", error)
    return NextResponse.json(
      { error: "Backend connection failed" },
      { status: 503 }
    )
  }
}
```

#### 5.2.2 API クライアント実装

**ファイル**: `virtual-voicebot-frontend/lib/api/sync-status.ts`（新規作成）

```typescript
export interface SyncStatusResponse {
  ok: boolean
  callActionsSync: CallActionsSync
}

export interface CallActionsSync {
  lastUpdatedAt: string | null
  ruleCount: number
  elapsedMinutes: number | null
}

export async function fetchSyncStatus(): Promise<SyncStatusResponse> {
  const response = await fetch("/api/sync-status", {
    method: "GET",
    headers: {
      "Content-Type": "application/json",
    },
  })

  if (!response.ok) {
    throw new Error(`Failed to fetch sync status: ${response.statusText}`)
  }

  return response.json()
}
```

#### 5.2.3 Sync Status Widget コンポーネント

**ファイル**: `virtual-voicebot-frontend/components/CallActionsSyncWidget.tsx`（新規作成）

```tsx
"use client"

import { useEffect, useState } from "react"
import { fetchSyncStatus, type CallActionsSync } from "@/lib/api/sync-status"

export function CallActionsSyncWidget() {
  const [status, setStatus] = useState<CallActionsSync | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const loadStatus = async () => {
    setLoading(true)
    setError(null)
    try {
      const response = await fetchSyncStatus()
      setStatus(response.callActionsSync)
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load sync status")
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    loadStatus()
    // 30秒ごとに自動更新
    const interval = setInterval(loadStatus, 30000)
    return () => clearInterval(interval)
  }, [])

  if (loading) {
    return (
      <div className="rounded-lg border bg-card p-4">
        <h3 className="text-sm font-semibold mb-2">着信アクション同期状態</h3>
        <p className="text-xs text-muted-foreground">読み込み中...</p>
      </div>
    )
  }

  if (error) {
    return (
      <div className="rounded-lg border bg-card p-4">
        <h3 className="text-sm font-semibold mb-2">着信アクション同期状態</h3>
        <p className="text-xs text-destructive">{error}</p>
        <button
          onClick={loadStatus}
          className="mt-2 text-xs text-primary hover:underline"
        >
          再読み込み
        </button>
      </div>
    )
  }

  if (!status) return null

  const isDelayed = status.elapsedMinutes !== null && status.elapsedMinutes > 10

  return (
    <div className="rounded-lg border bg-card p-4">
      <div className="flex items-center justify-between mb-3">
        <h3 className="text-sm font-semibold">着信アクション同期状態</h3>
        <button
          onClick={loadStatus}
          className="text-xs text-muted-foreground hover:text-foreground"
        >
          更新
        </button>
      </div>

      <div className="space-y-2">
        {/* Rule Count */}
        <div className="flex items-center justify-between text-xs">
          <span className="text-muted-foreground">アクティブルール数:</span>
          <span className="font-medium">{status.ruleCount} 件</span>
        </div>

        {/* Last Updated */}
        {status.lastUpdatedAt ? (
          <div className="flex items-center justify-between text-xs">
            <span className="text-muted-foreground">最終更新:</span>
            <span className="font-medium">
              {new Date(status.lastUpdatedAt).toLocaleString("ja-JP")}
            </span>
          </div>
        ) : (
          <div className="text-xs text-muted-foreground">
            最終更新: データなし
          </div>
        )}

        {/* Elapsed Time */}
        {status.elapsedMinutes !== null && (
          <div className="flex items-center justify-between text-xs">
            <span className="text-muted-foreground">経過時間:</span>
            <span className={isDelayed ? "font-medium text-yellow-600" : "font-medium"}>
              {status.elapsedMinutes} 分前
            </span>
          </div>
        )}

        {/* Alert */}
        {isDelayed && (
          <div className="mt-2 p-2 bg-yellow-50 border border-yellow-200 rounded-md">
            <p className="text-xs text-yellow-800">
              ⚠️ 10分以上更新がありません
            </p>
          </div>
        )}
      </div>
    </div>
  )
}
```

#### 5.2.4 Dashboard ページへの配置

**ファイル**: `virtual-voicebot-frontend/components/dashboard-content.tsx`（既存ファイル修正）

```tsx
import { CallActionsSyncWidget } from "./CallActionsSyncWidget"

export function DashboardContent() {
  return (
    <div className="container mx-auto p-6">
      <h1 className="text-3xl font-bold mb-6">ダッシュボード</h1>

      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        {/* 既存のウィジェット */}
        {/* ... */}

        {/* 🆕 Call Actions Sync Widget */}
        <CallActionsSyncWidget />
      </div>
    </div>
  )
}
```

---

### 5.3 受入条件

- [ ] Backend API `GET /api/sync/status` が正しく実装され、call_action_rules テーブルのデータを返す
- [ ] Backend HTTP サーバが JSON レスポンスを正しく返却できる
- [ ] Frontend Proxy API `/api/sync-status` が Backend からデータを取得できる
- [ ] Frontend Dashboard に CallActionsSyncWidget が配置され、同期状態が表示される
- [ ] 最終更新日時が日本時間で表示される
- [ ] アクティブルール数が正しく表示される
- [ ] 経過時間（分）が正しく計算・表示される
- [ ] 10分以上更新がない場合、アラート表示される
- [ ] 30秒ごとに自動更新される
- [ ] 手動更新ボタンで即座にリフレッシュできる
- [ ] データなし（lastUpdatedAt = null）の場合でもエラーにならず適切に表示される

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #177 | STEER-177 | 起票 |
| STEER-177 | contract.md §5.2 | API エンドポイント追加 |
| STEER-177 | Backend DD-xxx | sync_status API 詳細設計 |
| STEER-177 | Frontend DD-xxx | SyncStatusWidget 詳細設計 |
| Backend DD-xxx | Backend UT-xxx | 単体テスト |
| Frontend DD-xxx | Frontend UT-xxx | コンポーネントテスト |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [ ] 要件の記述が明確か
- [ ] 詳細設計で実装者が迷わないか
- [ ] テストケースが網羅的か
- [ ] 既存仕様（contract.md）との整合性があるか
- [ ] トレーサビリティが維持されているか
- [ ] call_action_rules テーブル構造と整合しているか
- [ ] frontend_pull の同期フローと整合性があるか
- [ ] Backend HTTP サーバ（mod.rs）の既存実装と整合しているか

### 7.2 マージ前チェック（Approved → Merged）

- [ ] 実装が完了している
- [ ] コードレビューを受けている
- [ ] 関連テストがPASS
- [ ] contract.md への反映準備ができている

---

## 8. 備考

### 8.1 設計判断

**なぜ着信アクション（call_action_rules）のみに絞ったか？**

- Issue #177 の本来の目的は「Frontend の設定変更が Backend に反映されたかの確認」
- 現状、Backend → Frontend の outbox 同期（call_log/recording/ivr_session_event）は自動的に行われるため、ユーザーが意識する必要性が低い
- 一方、Frontend → Backend の Pull 型同期（着信アクション）は、ユーザーが設定変更後に「反映されたか」を確認したいニーズが高い
- したがって、Phase 1 では着信アクションのみに絞り、シンプルな実装を目指す

**なぜ独自 HTTP サーバ拡張を選んだか？**

- Backend は axum 依存がなく、独自実装の TCP HTTP サーバを使用している
- 既存の `/recordings/:callId/:recordingId` エンドポイントと同じサーバに追加すれば、影響範囲が最小
- axum を新規追加するとスコープが大きくなり、依存管理が複雑化する

**なぜ Frontend Proxy API を追加したか？**

- クライアント（ブラウザ）から直接 Backend へアクセスすると、CORS/到達性の問題が発生しやすい
- Next.js の API Routes をプロキシとして挟むことで、サーバ側で Backend にアクセスできる
- 将来的に認証を追加する場合も、Proxy API 側で制御しやすい

### 8.2 将来拡張

- **他のエンティティ追加**: number-groups（registered_numbers/spam_numbers）、ivr-flows も同様に監視
- **frontend_pull worker 監視**: 最終 Pull 実行日時を DB に記録して、worker 稼働状態を判定
- **エラーログ追跡**: frontend_pull でエラーが発生した場合、詳細ログを表示
- **手動同期トリガー**: Frontend から Backend の frontend_pull worker を手動起動
- **通知機能**: 同期遅延が閾値を超えた場合、メール/Slack 通知
- **グラフ表示**: 同期遅延の時系列グラフ、ルール数の推移

### 8.3 技術的注意点

- **パフォーマンス**:
  - `MAX(updated_at)` クエリは全件スキャンになるが、call_action_rules はルール数が少ない（数十〜数百件程度）ため影響は小さい
  - `COUNT(*) WHERE is_active = TRUE` は既存の `idx_call_action_rules_priority` インデックスを活用できる
  - 将来的にパフォーマンス問題が発生した場合は、`updated_at` にインデックスを追加することを検討
- **セキュリティ**: MVP では認証なしだが、将来的には Bearer トークンによる認証を追加
- **DB Pool の渡し方**: spawn_recording_server に `Option<PgPool>` を渡す必要があるため、main.rs の起動シーケンスを確認（DB未接続時は 503 返却）
- **テスト観点**: `/api/sync/status` は以下のパターンを単体テストで網羅することを推奨
  - DB あり + rules あり
  - DB あり + rules なし（system_settings の heartbeat を使用）
  - DB なし（503 返却）
  - system_settings なし（稀だが null 対応）

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-14 | 初版作成 | Claude Code (claude-sonnet-4-5) |
| 2026-02-14 | Codex レビュー指摘対応（1回目 NG）：スコープを着信アクション（call_action_rules）のみに変更、Backend API を独自 HTTP サーバ拡張に変更、Frontend Proxy API 追加、dashboard-content.tsx への配置変更 | Claude Code (claude-sonnet-4-5) |
| 2026-02-14 | Codex レビュー指摘対応（2回目 NG）：/api/sync/status の追加位置を /recordings/ より前に修正、Backend URL を :18080 に修正、PgPool を Option<PgPool> に変更、lastUpdatedAt を全件対象に修正、パフォーマンス注記を修正、レビューチェックリストを修正 | Claude Code (claude-sonnet-4-5) |
| 2026-02-14 | Codex レビュー指摘対応（3回目 NG）：lastUpdatedAt を COALESCE(MAX(call_action_rules.updated_at), system_settings.updated_at) に変更（ルール0件時の heartbeat 対応）、エラーレスポンス形式を contract 準拠に修正 | Claude Code (claude-sonnet-4-5) |
| 2026-02-14 | Codex レビュー指摘対応（4回目 OK）：main.rs 呼び出し例に .await 追加、エラーレスポンスに「MVP では requestId 省略」コメント追加、テスト観点を §8.3 に追加 | Claude Code (claude-sonnet-4-5) |
