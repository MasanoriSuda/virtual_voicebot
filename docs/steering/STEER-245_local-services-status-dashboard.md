# STEER-245: ローカルサービス死活監視ダッシュボード

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-245 |
| タイトル | ローカルサービス死活監視ダッシュボード |
| ステータス | Approved |
| 関連Issue | #245 |
| 優先度 | P1 |
| 前提Issue | #246（Whisper Docker Compose 常駐化）← STEER-246 Approved |
| 作成日 | 2026-02-24 |

---

## 2. ストーリー（Why）

### 2.1 背景

ローカル AI サービス（ASR: Whisper / LLM: Ollama / TTS: VoiceVox）は Docker Compose または systemd で常駐しているが、現状 Frontend ダッシュボードから各サービスの死活状態を確認する手段がない。

- STEER-246 で Whisper サーバーが Docker Compose サービスとして常駐化された
- `GET /healthz` エンドポイントが Whisper に追加された（`virtual-voicebot-backend/script/whisper_server.py` L118）
- VoiceVox は `GET /speakers` で疎通確認できる（既実績: `virtual-voicebot-frontend/app/api/announcements/speakers/route.ts`）
- Ollama は `GET /api/tags` で疎通確認できる（互換性を優先して `/api/version` より `/api/tags` を選択）

### 2.2 目的

Frontend のダッシュボードにローカル AI サービスの死活状態を表示し、以下を実現する:

1. **運用可視性**: ASR/LLM/TTS の各サービスが正常稼働しているか一覧で確認できる
2. **障害早期検知**: サービス停止やネットワーク障害を即座に検知できる
3. **設定有効性確認**: `*_local_server_enabled=false` の場合は「無効」として明示する

### 2.3 ユーザーストーリー

```
As a システム運用者
I want to ダッシュボードでローカル AI サービス（ASR/LLM/TTS）の死活状態を確認する
So that 各サービスが正常に稼働しているか、設定が有効かを把握できる

受入条件:
- [ ] ダッシュボードに ASR / LLM / TTS の死活状態バッジを表示
- [ ] 正常（HTTP 200 + タイムアウト内）= 緑、異常（タイムアウト/非200）= 赤、無効（*_enabled=false）= 灰
- [ ] 30秒ごとに自動更新
- [ ] 手動更新ボタンで即座にリフレッシュ
- [ ] Backend の実ランタイム設定（URL / enabled フラグ）を使って確認する
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-24 |
| 起票理由 | STEER-246 完了後の前提基盤として死活監視ダッシュボード実装が必要 |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Sonnet 4.6 |
| 作成日 | 2026-02-24 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "Issue #245: フロントエンドのダッシュボードからローカルサーバーの状態を確認できるようにする" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| 1 | Codex | 2026-02-25 | 要修正 | ①docker-compose.yml 参照が `uas` のみの旧ファイルと混同・②`config::AiConfig::from_env()` が非公開・③§5.1.2 行番号参照ズレ・④Frontend timeout 説明が直列前提 |
| 2 | Codex | 2026-02-25 | 要修正 | root compose の TTS 不整合（`VOICEVOX_URL` vs `TTS_LOCAL_SERVER_BASE_URL`）未対応・`※注1` 孤立参照 |
| 3 | Codex | 2026-02-25 | OK | 前回指摘の解消を確認。残リスクは実装・動作確認時に §5.3 受入条件で対応。 |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | @MasanoriSuda |
| 承認日 | 2026-02-25 |
| 承認コメント | lgtm。Codex レビュー Round 3 OK を確認し承認。実装は Codex に引き継ぎ。 |

### 3.5 実装（該当する場合）

| 項目 | 値 |
|------|-----|
| 実装者 | Codex (GPT-5) |
| 実装日 | 2026-02-24 |
| 指示者 | @MasanoriSuda |
| 指示内容 | 「STEER-245 承認後の実装」 |
| コードレビュー | Codex 実装時セルフチェック完了（CodeRabbit 待ち） |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ実行者 | - |
| マージ日 | - |
| マージ先 | docs/contract.md, Backend DD, Frontend DD |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| docs/contract.md | 追加 | §5.2 に `GET /api/local-services/status` エンドポイント追加 |
| virtual-voicebot-backend/docs/design/detail/DD-xxx.md | 追加 | local-services status API の詳細設計 |
| virtual-voicebot-frontend/docs/design/detail/DD-xxx.md | 追加 | LocalServicesStatusWidget の詳細設計 |

### 4.2 影響するコード

| モジュール | 変更種別 | 概要 |
|-----------|---------|------|
| virtual-voicebot-backend/src/interface/http/mod.rs | 修正 | `GET /api/local-services/status` エンドポイント追加 |
| virtual-voicebot-backend/src/shared/config/mod.rs | 参照のみ | `asr_local_*`, `llm_local_*`, `tts_local_*` フィールドを参照 |
| （リポジトリルート）docker-compose.yml | 修正 | backend 環境変数に `LLM_LOCAL_SERVER_URL` と `TTS_LOCAL_SERVER_BASE_URL` を追加（`OLLAMA_URL`/`VOICEVOX_URL` との不整合解消） |
| virtual-voicebot-backend/docker-compose.dev.yml | 修正 | app 環境変数に `LLM_LOCAL_SERVER_URL=http://ollama:11434/api/chat` を追加（`TTS_LOCAL_SERVER_BASE_URL` は既設定のため変更不要） |
| virtual-voicebot-frontend/app/api/local-services-status/route.ts | 追加 | Backend Proxy API（新規ファイル） |
| virtual-voicebot-frontend/lib/api/local-services-status.ts | 追加 | Proxy API クライアント（新規ファイル） |
| virtual-voicebot-frontend/components/LocalServicesStatusWidget.tsx | 追加 | 死活状態表示ウィジェット（新規ファイル） |
| virtual-voicebot-frontend/components/dashboard-content.tsx | 修正 | LocalServicesStatusWidget を `<CallActionsSyncWidget />` の下に配置 |

---

## 5. 差分仕様（What / How）

### 5.1 Backend: Local Services Status API

#### 5.1.1 エンドポイント定義（contract.md へマージ）

**追加先**: `docs/contract.md` §5.2（Frontend → Backend API）

```markdown
| メソッド | パス | 説明 |
|---------|------|------|
| GET | /api/local-services/status | ローカル AI サービス死活状態取得 |

#### GET /api/local-services/status

Backend がランタイム設定を参照して ASR / LLM / TTS ローカルサービスへ死活確認プローブを実行し、結果を返す。

**リクエスト**: なし

**レスポンス（正常時）**:
```json
{
  "ok": true,
  "localServices": {
    "asr": {
      "status": "ok",
      "displayUrl": "http://whisper:9000"
    },
    "llm": {
      "status": "error",
      "displayUrl": "http://ollama:11434"
    },
    "tts": {
      "status": "disabled",
      "displayUrl": null
    }
  }
}
```

**status フィールドの値**:

| 値 | 条件 |
|----|------|
| `"ok"` | `*_local_server_enabled=true` かつ HTTP 200 がタイムアウト内に返却 |
| `"error"` | `*_local_server_enabled=true` かつ タイムアウト / 接続エラー / 非200レスポンス |
| `"disabled"` | `*_local_server_enabled=false` |

**displayUrl フィールド**:
- `"disabled"` の場合は `null`
- それ以外はサービスのベース URL（パス部分を除いたもの）
```

#### 5.1.2 プローブ方式

| サービス | 設定フィールド（`AiConfig` 構造体） | probe エンドポイント | タイムアウト | 判定 |
|---------|-------------------------------|-------------------|------------|------|
| ASR (Whisper) | `asr_local_server_url`, `asr_local_server_enabled` | `{ベースURL}/healthz` ※1 | 2000ms | HTTP 200 |
| LLM (Ollama) | `llm_local_server_url`, `llm_local_server_enabled` | `{ベースURL}/api/tags` ※2 | 2000ms | HTTP 200 |
| TTS (VoiceVox) | `tts_local_server_base_url`, `tts_local_server_enabled` | `{ベースURL}/speakers` | 2000ms | HTTP 200 |

**※1 ASR ベースURL導出**: `asr_local_server_url`（例: `http://whisper:9000/transcribe`）から scheme + host + port（`http://whisper:9000`）を抽出する。`/healthz` は `virtual-voicebot-backend/script/whisper_server.py` L118 で定義済み。

**※2 LLM ベースURL導出**: `llm_local_server_url`（例: `http://ollama:11434/api/chat`）から scheme + host + port（`http://ollama:11434`）を抽出して `/api/tags` を付与する。`/api/tags` は Ollama のモデル一覧を返す軽量エンドポイントで互換性が高い。

**タイムアウト共通値**: 2000ms（監視専用固定値）。推論・音声処理で使う `*_local_timeout`（ASR: 3000ms / LLM: 8000ms / TTS: 5000ms）とは目的が異なるため、ダッシュボード UX を優先して短い固定値を採用する。

**並列実行**: 3サービスへの HTTP リクエストは `tokio::join!` で並列実行する。直列実行だと最大 2000ms × 3 = 6000ms の待ち時間が発生する。

#### 5.1.3 docker-compose 修正（LLM URL 不整合解消）

**背景**: リポジトリルート `docker-compose.yml` は `OLLAMA_URL` と `VOICEVOX_URL` を設定しているが、Backend は `LLM_LOCAL_SERVER_URL`（デフォルト: `http://localhost:11434/api/chat`）と `TTS_LOCAL_SERVER_BASE_URL`（デフォルト: `http://localhost:50021`）を読む。両者が未設定のままだと Ollama・VoiceVox コンテナへの probe が false negative になる。

> **注意**: `virtual-voicebot-backend/docker-compose.yml`（`uas` サービスのみの旧ファイル）は本 Issue の対象外。対象は以下2ファイルのみ。

**修正内容 ①**（リポジトリルート `docker-compose.yml`、`backend` サービス環境変数）:

```yaml
# 追加（OLLAMA_URL の下に追記）
LLM_LOCAL_SERVER_URL: http://ollama:11434/api/chat
# 追加（VOICEVOX_URL の下に追記）
TTS_LOCAL_SERVER_BASE_URL: http://voicevox:50021
```

**修正内容 ②**（`virtual-voicebot-backend/docker-compose.dev.yml`、`app` サービス環境変数）:

```yaml
# 追加（LLM のみ。TTS_LOCAL_SERVER_BASE_URL: http://voicevox:50021 は既に設定済み）
LLM_LOCAL_SERVER_URL: http://ollama:11434/api/chat
```

> **systemd 運用スコープ外**: STEER-235 で導入した systemd EnvironmentFile テンプレートへの `LLM_LOCAL_SERVER_URL`・`TTS_LOCAL_SERVER_BASE_URL` 追加は本 Issue の対象外とする。別 Issue で対応すること（§8.1 参照）。

#### 5.1.4 実装方針

**変更ファイル**: `virtual-voicebot-backend/src/interface/http/mod.rs`（既存ファイル修正）

既存の独自 TCP HTTP サーバに `/api/local-services/status` エンドポイントを追加する。追加位置は `/api/sync/status` と同様、`/recordings/` 判定より前。

**実装イメージ**:

```rust
// handle_conn 内（/api/sync/status 判定の後、/recordings/ より前）

if method == "GET" && path == "/api/local-services/status" {
    // config::ai_config() は OnceLock キャッシュ済みの公開アクセサ（from_env() は非公開）
    let ai_cfg = config::ai_config();
    let json = probe_local_services(ai_cfg).await;
    return write_json_response(socket, 200, "OK", json.as_bytes()).await;
}
```

**新規関数 `probe_local_services`**:

```rust
const PROBE_TIMEOUT_MS: u64 = 2_000;

#[derive(serde::Serialize)]
struct LocalServiceEntry {
    status: &'static str,          // "ok" | "error" | "disabled"
    #[serde(rename = "displayUrl")]
    display_url: Option<String>,
}

#[derive(serde::Serialize)]
struct LocalServicesResponse {
    ok: bool,
    #[serde(rename = "localServices")]
    local_services: LocalServicesMap,
}

#[derive(serde::Serialize)]
struct LocalServicesMap {
    asr: LocalServiceEntry,
    llm: LocalServiceEntry,
    tts: LocalServiceEntry,
}

/// URL から scheme://host:port 部分を抽出する
/// 例: "http://whisper:9000/transcribe" → "http://whisper:9000"
fn extract_base_url(url: &str) -> String {
    let url = url.trim_end_matches('/');
    let scheme = if url.starts_with("https://") { "https://" } else { "http://" };
    if let Some(after_scheme) = url.strip_prefix(scheme) {
        let host_part = after_scheme.split('/').next().unwrap_or(after_scheme);
        return format!("{}{}", scheme, host_part);
    }
    url.to_string()
}

async fn probe_once(url: &str) -> bool {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(PROBE_TIMEOUT_MS))
        .build()
        .unwrap_or_default();
    matches!(client.get(url).send().await, Ok(r) if r.status().is_success())
}

async fn probe_local_services(ai_cfg: &'static config::AiConfig) -> String {
    // 3サービスを並列 probe
    let (asr_result, llm_result, tts_result) = tokio::join!(
        async {
            if !ai_cfg.asr_local_server_enabled {
                return LocalServiceEntry { status: "disabled", display_url: None };
            }
            let base = extract_base_url(&ai_cfg.asr_local_server_url);
            let ok = probe_once(&format!("{}/healthz", base)).await;
            LocalServiceEntry { status: if ok { "ok" } else { "error" }, display_url: Some(base) }
        },
        async {
            if !ai_cfg.llm_local_server_enabled {
                return LocalServiceEntry { status: "disabled", display_url: None };
            }
            let base = extract_base_url(&ai_cfg.llm_local_server_url);
            let ok = probe_once(&format!("{}/api/tags", base)).await;
            LocalServiceEntry { status: if ok { "ok" } else { "error" }, display_url: Some(base) }
        },
        async {
            if !ai_cfg.tts_local_server_enabled {
                return LocalServiceEntry { status: "disabled", display_url: None };
            }
            let base = ai_cfg.tts_local_server_base_url.trim_end_matches('/').to_string();
            let ok = probe_once(&format!("{}/speakers", base)).await;
            LocalServiceEntry { status: if ok { "ok" } else { "error" }, display_url: Some(base) }
        },
    );

    let resp = LocalServicesResponse {
        ok: true,
        local_services: LocalServicesMap {
            asr: asr_result,
            llm: llm_result,
            tts: tts_result,
        },
    };
    serde_json::to_string(&resp).unwrap_or_else(|_| {
        r#"{"ok":false,"error":{"code":"INTERNAL_ERROR","message":"serialization failed"}}"#.to_string()
    })
}
```

**依存クレート**: `reqwest`。Cargo.toml に含まれているか確認すること（なければ追加）。

---

### 5.2 Frontend: Local Services Status Widget

#### 5.2.1 Backend Proxy API

**ファイル**: `virtual-voicebot-frontend/app/api/local-services-status/route.ts`（新規作成）

STEER-177 実装の `app/api/sync-status/route.ts`（L1〜L31）と同一パターンを使用する。Frontend タイムアウトは Backend probe timeout (2000ms) + 処理・通信余裕 = 10_000ms を設定する（並列実行のため Backend 側の実待ち時間は ~2000ms）。

```typescript
import { NextResponse } from "next/server"
import type { LocalServicesStatusResponse } from "@/lib/api/local-services-status"

export const runtime = "nodejs"

// Backend probe timeout(2000ms) + 処理・通信余裕（並列実行のため実待ち ~2000ms）
const BACKEND_TIMEOUT_MS = 10_000

async function fetchWithTimeout(
  input: string,
  init: RequestInit,
  timeoutMs: number,
): Promise<Response> {
  const controller = new AbortController()
  const timer = setTimeout(() => controller.abort(), timeoutMs)
  try {
    return await fetch(input, { ...init, signal: controller.signal })
  } finally {
    clearTimeout(timer)
  }
}

export async function GET() {
  const backendUrl = process.env.BACKEND_URL || "http://localhost:18080"
  try {
    const response = await fetchWithTimeout(
      `${backendUrl}/api/local-services/status`,
      { method: "GET", cache: "no-store" },
      BACKEND_TIMEOUT_MS,
    )
    if (!response.ok) {
      return NextResponse.json(
        { ok: false, error: "failed to fetch local services status from backend" },
        { status: 502 },
      )
    }
    const payload = (await response.json()) as LocalServicesStatusResponse
    return NextResponse.json(payload)
  } catch (error) {
    console.error("[api/local-services-status] failed", error)
    return NextResponse.json(
      { ok: false, error: "backend connection failed" },
      { status: 503 },
    )
  }
}
```

#### 5.2.2 API クライアント

**ファイル**: `virtual-voicebot-frontend/lib/api/local-services-status.ts`（新規作成）

```typescript
export type ServiceStatus = "ok" | "error" | "disabled"

export interface LocalServiceEntry {
  status: ServiceStatus
  displayUrl: string | null
}

export interface LocalServicesStatusResponse {
  ok: boolean
  localServices: {
    asr: LocalServiceEntry
    llm: LocalServiceEntry
    tts: LocalServiceEntry
  }
}

export async function fetchLocalServicesStatus(): Promise<LocalServicesStatusResponse> {
  const response = await fetch("/api/local-services-status", {
    method: "GET",
    cache: "no-store",
  })
  if (!response.ok) {
    throw new Error(`Failed to fetch local services status: ${response.statusText}`)
  }
  return response.json() as Promise<LocalServicesStatusResponse>
}
```

#### 5.2.3 ウィジェットコンポーネント

**ファイル**: `virtual-voicebot-frontend/components/LocalServicesStatusWidget.tsx`（新規作成）

`CallActionsSyncWidget.tsx`（L10: `POLL_INTERVAL_MS = 30_000`、L31: `useCallback`、L43: `useEffect`、L90: `Button variant="outline"`）の実装パターンを踏襲する。

```tsx
"use client"

import { useCallback, useEffect, useState } from "react"
import { RefreshCw } from "lucide-react"
import {
  fetchLocalServicesStatus,
  type LocalServicesStatusResponse,
  type ServiceStatus,
} from "@/lib/api/local-services-status"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"

const POLL_INTERVAL_MS = 30_000

const SERVICE_LABELS: Record<string, string> = {
  asr: "ASR (Whisper)",
  llm: "LLM (Ollama)",
  tts: "TTS (VoiceVox)",
}

const STATUS_STYLE: Record<ServiceStatus, { label: string; className: string }> = {
  ok:       { label: "正常", className: "bg-green-100 text-green-700 border-green-300" },
  error:    { label: "異常", className: "bg-red-100 text-red-700 border-red-300" },
  disabled: { label: "無効", className: "bg-gray-100 text-gray-500 border-gray-300" },
}

export function LocalServicesStatusWidget() {
  const [data, setData] = useState<LocalServicesStatusResponse | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(async () => {
    setError(null)
    try {
      setData(await fetchLocalServicesStatus())
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load local services status")
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    void load()
    const timer = setInterval(() => void load(), POLL_INTERVAL_MS)
    return () => clearInterval(timer)
  }, [load])

  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between gap-2 space-y-0">
        <div>
          <CardTitle className="text-base">ローカルサービス状態</CardTitle>
          <CardDescription>ASR / LLM / TTS</CardDescription>
        </div>
        <Button variant="outline" size="sm" onClick={() => void load()}>
          <RefreshCw className="h-4 w-4" />
          更新
        </Button>
      </CardHeader>
      <CardContent className="space-y-3">
        {loading && <p className="text-sm text-muted-foreground">読み込み中...</p>}
        {error && <p className="text-sm text-destructive">{error}</p>}
        {data?.localServices &&
          (["asr", "llm", "tts"] as const).map((key) => {
            const entry = data.localServices[key]
            const style = STATUS_STYLE[entry.status]
            return (
              <div key={key} className="flex items-center justify-between text-sm">
                <div>
                  <span className="font-medium">{SERVICE_LABELS[key]}</span>
                  {entry.displayUrl && (
                    <span className="ml-2 text-xs text-muted-foreground">{entry.displayUrl}</span>
                  )}
                </div>
                <span
                  className={`inline-flex items-center rounded-full border px-2 py-0.5 text-xs font-medium ${style.className}`}
                >
                  {style.label}
                </span>
              </div>
            )
          })}
      </CardContent>
    </Card>
  )
}
```

#### 5.2.4 Dashboard ページへの配置

**ファイル**: `virtual-voicebot-frontend/components/dashboard-content.tsx`（既存ファイル修正）

`<CallActionsSyncWidget />` の直後（L56 付近）に `<LocalServicesStatusWidget />` を追加する。

```tsx
// 追加 import
import { LocalServicesStatusWidget } from "./LocalServicesStatusWidget"

// DashboardContent 内（L55〜L57 付近）
{/* Call Actions Sync Status */}
<CallActionsSyncWidget />

{/* Local Services Status */}
<LocalServicesStatusWidget />
```

---

### 5.3 受入条件

- [ ] `GET /api/local-services/status` が Backend から JSON レスポンスを返す
- [ ] ASR / LLM / TTS それぞれの `enabled=false` 時は `"disabled"` を返す
- [ ] ASR probe (`GET {base}/healthz`、2000ms タイムアウト) が正常時に `"ok"` を返す
- [ ] LLM probe (`GET {base}/api/tags`、2000ms タイムアウト) が正常時に `"ok"` を返す
- [ ] TTS probe (`GET {base}/speakers`、2000ms タイムアウト) が正常時に `"ok"` を返す
- [ ] タイムアウト・接続エラー時は `"error"` を返す
- [ ] 3サービスの probe が並列実行される
- [ ] リポジトリルート `docker-compose.yml` の backend 環境変数に `LLM_LOCAL_SERVER_URL=http://ollama:11434/api/chat` が追加される
- [ ] リポジトリルート `docker-compose.yml` の backend 環境変数に `TTS_LOCAL_SERVER_BASE_URL=http://voicevox:50021` が追加される
- [ ] `virtual-voicebot-backend/docker-compose.dev.yml` の app 環境変数に `LLM_LOCAL_SERVER_URL=http://ollama:11434/api/chat` が追加される（`TTS_LOCAL_SERVER_BASE_URL` は既設定のため変更不要）
- [ ] `virtual-voicebot-backend/docker-compose.yml`（`uas` サービスのみの旧ファイル）には変更を加えない
- [ ] Frontend Proxy API `/api/local-services-status` が Backend から取得し返却する
- [ ] ダッシュボードに `LocalServicesStatusWidget` が `<CallActionsSyncWidget />` の直後に配置される
- [ ] 正常 = 緑「正常」、異常 = 赤「異常」、無効 = 灰「無効」のバッジが表示される
- [ ] 30秒ごとに自動更新される
- [ ] 手動更新ボタンで即座にリフレッシュできる
- [ ] Backend 停止中でも widget がクラッシュせず、エラー表示される

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #245 | STEER-245 | 起票 |
| STEER-246 | STEER-245 | 前提（Whisper /healthz 実装済み） |
| STEER-245 | contract.md §5.2 | API エンドポイント追加 |
| STEER-245 | Backend DD-xxx | local-services status API 詳細設計 |
| STEER-245 | Frontend DD-xxx | LocalServicesStatusWidget 詳細設計 |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [ ] 3サービスの probe が `tokio::join!` で並列実行される設計になっているか
- [ ] `PROBE_TIMEOUT_MS = 2_000` が各プローブに適用されているか
- [ ] `extract_base_url` が `http://host:port/path` 形式を正しく処理するか
- [ ] LLM probe URL が `{base}/api/tags` であることが明示されているか
- [ ] `enabled=false` 時の挙動（`"disabled"`, `displayUrl: null`）が一貫しているか
- [ ] `config::ai_config()`（公開アクセサ）を使っており `AiConfig::from_env()` を直接呼んでいないか
- [ ] リポジトリルート `docker-compose.yml` に `LLM_LOCAL_SERVER_URL` と `TTS_LOCAL_SERVER_BASE_URL` の両方が追加されているか
- [ ] `virtual-voicebot-backend/docker-compose.dev.yml` に `LLM_LOCAL_SERVER_URL` が追加され、`TTS_LOCAL_SERVER_BASE_URL` は変更不要であることが確認されているか
- [ ] `virtual-voicebot-backend/docker-compose.yml`（`uas` のみ）は対象外として明示されているか
- [ ] Frontend widget が `CallActionsSyncWidget` の既存パターンと整合しているか
- [ ] contract.md との整合性があるか

### 7.2 マージ前チェック（Approved → Merged）

- [ ] 実装が完了している
- [ ] コードレビューを受けている
- [ ] `reqwest` クレートが Cargo.toml に含まれているか確認
- [ ] contract.md への反映準備ができている

---

## 8. 備考

### 8.1 設計判断

**なぜ Backend 集約 API を選んだか**

- Backend の実ランタイム設定（`*_local_server_url`, `*_local_server_enabled`）をそのまま使える
- Frontend（ブラウザ）からローカルネットワーク内サービスへ直接アクセスすると CORS / 到達性の問題が発生する
- Next.js API route を挟むだけでは不十分（Next.js サーバーから見ても `http://whisper:9000` は Backend Docker ネットワーク内にあり到達できない場合がある）
- Backend が probe を実行することで、実際に通話に使われる経路と同一ネットワークでの確認になる

**Frontend 既存の `speakers/route.ts` との関係**

- `app/api/announcements/speakers/route.ts` は Frontend（Next.js サーバー）が直接 VoiceVox に接続してスピーカー一覧を取得するルート
- STEER-245 の TTS probe は Backend が実施する → 異なる経路・目的（スピーカー一覧取得 vs 死活確認）
- 両者は独立して存在する

**LLM probe エンドポイントに `/api/tags` を選んだ理由**

- `/api/version` より `/api/tags` の方が Ollama の互換性上長期的に安定している
- 監視目的では応答サイズが大きくても 2000ms タイムアウトで十分

**systemd 運用（STEER-235）への `LLM_LOCAL_SERVER_URL` 追加について**

- STEER-235 で導入した systemd EnvironmentFile テンプレートへの `LLM_LOCAL_SERVER_URL` 追加は本 Issue のスコープ外とする
- systemd で運用する場合は手動で EnvironmentFile に `LLM_LOCAL_SERVER_URL=http://localhost:11434/api/chat` を追加するか、別 Issue で EnvironmentFile テンプレートを更新すること

### 8.2 技術的注意点

- **並列 probe**: `tokio::join!` で3サービスを同時確認し、直列実行の最大 6000ms 待ちを回避する
- **`reqwest` クレート**: Cargo.toml に `reqwest` が含まれているか確認（なければ追加）
- **テスト観点**:
  - 全サービス正常時
  - 一部サービス停止時（2サービスが `"error"`、1サービスが `"ok"`）
  - 全サービス `enabled=false` 時（全て `"disabled"`）
  - Backend HTTP サーバーが probe 結果を正しくシリアライズするか

### 8.3 将来拡張

- **OpenAI / Gemini クラウドプロバイダー**: API キー有効性確認（別Issue）
- **Raspi プロバイダー**: `*_raspi_enabled=true` 時の死活確認追加
- **ダッシュボード通知**: 異常検知時のアラートバッジ

---

## 9. Open Questions

| # | 質問 | 採択 | 状態 |
|---|------|------|------|
| OQ-1 | LLM (Ollama) の probe エンドポイントをどこにするか | (A) `{base}/api/tags` ― 互換性優先 | **確定** |
| OQ-2 | ASR ベースURL抽出方式をどうするか | (A) URL の scheme + host + port を抽出して `/healthz` を付与 ― 既存設定を増やさない | **確定** |
| OQ-3 | probe のタイムアウトをどう設定するか | (B) 監視専用固定値 2000ms ― ダッシュボード UX 優先、推論タイムアウトと目的が異なる | **確定** |
| OQ-4 | docker-compose での LLM URL 不整合（`OLLAMA_URL` vs `LLM_LOCAL_SERVER_URL`） | (A) この Issue で `LLM_LOCAL_SERVER_URL=http://ollama:11434/api/chat` を追加 ― 監視の正確性に直結 | **確定** |
| OQ-5 | probe の実行場所（Backend 集約 vs Frontend 直接） | (A) Backend 集約 API ― 実ランタイム設定と同一経路での確認、CORS 回避 | **確定**（初期設計時に確定済み） |

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-24 | 初版作成 | Claude Sonnet 4.6 |
| 2026-02-25 | OQ-1〜5 全確定（LLM probe=/api/tags, ASR base抽出, timeout=2000ms固定, LLM URL不整合解消, Backend集約）; §5.1.2 probe表更新; §5.1.3 §5.1.4 分割、docker-compose修正追加; §4.2 影響範囲更新; §5.3 受入条件追記; §7.1 レビューチェックリスト更新; §9 OQ全確定 | Claude Sonnet 4.6 |
| 2026-02-25 | Codex NG Round 1 対応: ①docker-compose参照をルート明示・`virtual-voicebot-backend/docker-compose.yml`（uas）は対象外注記; ②`config::ai_config()`（公開アクセサ）に修正; ③§5.1.2 行番号参照を削除しフィールド名のみに; ④§5.2.1 Frontend timeout説明を並列前提に修正; ⑤systemdスコープ外を§5.1.3・§8.1に明記 | Claude Sonnet 4.6 |
| 2026-02-25 | Codex NG Round 2 対応: ①root compose に TTS 不整合（`VOICEVOX_URL` vs `TTS_LOCAL_SERVER_BASE_URL`）解消を追加（§4.2・§5.1.3・§5.3・§7.1 更新）; ②`※注1` 孤立参照を除去 | Claude Sonnet 4.6 |
| 2026-02-25 | ステータス Draft → Approved（@MasanoriSuda 承認） | @MasanoriSuda |
