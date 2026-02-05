# BD-001_frontend

> Frontend 管理画面の基本設計（コンポーネント構成・データフロー）

| 項目 | 値 |
|------|-----|
| ID | BD-001 |
| ステータス | Draft |
| 作成日 | 2026-02-02 |
| 更新日 | 2026-02-05 |
| 関連Issue | #89 |
| 対応RD | [RD-005](../../requirements/RD-005_frontend.md) |
| 対応IT | - |

---

## 1. 概要

### 1.1 目的

Frontend 管理画面のコンポーネント構成、画面遷移、データフローを定義する。

### 1.2 スコープ

- 画面構成（レイアウト、ページ）
- コンポーネント分割
- データフロー（Backend → Frontend）
- 状態管理方針

---

## 2. システム構成

### 2.1 全体構成図

```
┌─────────────────────────────────────────────────────────────┐
│                        Frontend                              │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                   Next.js App                        │   │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────────────┐  │   │
│  │  │  Pages   │  │Components│  │   Server Sync    │  │   │
│  │  │ (routes) │  │   (UI)   │  │   (API Routes)   │  │   │
│  │  └────┬─────┘  └────┬─────┘  └────────┬─────────┘  │   │
│  │       │             │                  │            │   │
│  │  ┌────┴─────────────┴──────────────────┴───────┐   │   │
│  │  │              Local DB (PostgreSQL/Prisma)    │   │   │
│  │  └──────────────────────────────────────────────┘   │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                              ▲
                              │ POST /api/ingest/call
                              │ GET recordingUrl (Backend発行)
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                        Backend                               │
│          (SIP/RTP/AI/Recording - Rust)                      │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 コンポーネント一覧

| コンポーネント | 責務 | 技術選定 |
|--------------|------|---------|
| Next.js App | SSR/SSG/API Routes | Next.js 14 (App Router) |
| UI Components | 画面表示 | React + shadcn/ui |
| Server Sync | Backend からのデータ受信 | Next.js API Routes |
| Local DB | 通話データ永続化 | PostgreSQL + Prisma |
| State Management | クライアント状態管理 | React Context / Zustand |

---

## 3. 画面構成

### 3.1 レイアウト構造

```
┌─────────────────────────────────────────────────────────────┐
│  Header (64px)                                [🌙] [Avatar] │
├────────────┬────────────────────────────────────────────────┤
│            │                                                │
│  Sidebar   │              Main Content                      │
│  (240px)   │                                                │
│            │                                                │
│  ┌───────┐ │                                                │
│  │ Logo  │ │                                                │
│  ├───────┤ │                                                │
│  │ Nav   │ │                                                │
│  │ Items │ │                                                │
│  └───────┘ │                                                │
│            │                                                │
└────────────┴────────────────────────────────────────────────┘
```

### 3.2 ページ一覧（MVP）

| パス | ページ名 | コンポーネント | 説明 |
|------|---------|--------------|------|
| `/` | Dashboard | DashboardContent | KPI・グラフ表示 |
| `/calls` | 発着信履歴 | CallHistoryContent | 通話一覧・フィルタ |
| `/calls/[id]` | 通話詳細 | CallDetailDrawer | 録音・文字起こし・要約 |

### 3.3 将来ページ（MVP対象外）

| パス | ページ名 | 備考 |
|------|---------|------|
| `/groups` | 番号グループ | v0 プロトタイプにスケルトンあり |
| `/routing` | ルーティング | 同上 |
| `/ivr` | IVRフロー | 同上 |
| `/schedule` | スケジュール | 同上 |
| `/announcements` | アナウンス | 同上 |
| `/settings` | 設定 | 未作成 |
| `/audit-log` | 監査ログ | 未作成 |

---

## 4. コンポーネント構成

### 4.1 コンポーネントツリー

```
app/
├── layout.tsx              # RootLayout
│   └── AdminLayout
│       ├── AdminHeader
│       │   ├── Logo
│       │   ├── ThemeToggle
│       │   └── UserAvatar
│       └── AdminSidebar
│           └── NavItems
├── page.tsx                # Dashboard
│   └── DashboardContent
│       ├── KPICards
│       │   └── KPICard (x4)
│       └── Charts
│           └── HourlyCallChart
└── calls/
    ├── page.tsx            # Call History
    │   └── CallHistoryContent
    │       ├── FilterBar
    │       ├── CallsTable
    │       └── Pagination
    └── [id]/
        └── CallDetailDrawer
            ├── Tabs
            │   ├── RecordingTab
            │   │   └── AudioPlayer
            │   ├── TranscriptTab
            │   │   └── TranscriptView
            │   └── SummaryTab
            │       └── SummaryView
            └── CallMeta
```

### 4.2 共通コンポーネント（ui/）

| コンポーネント | 出典 | 用途 |
|--------------|------|------|
| Button | shadcn/ui | ボタン |
| Card | shadcn/ui | カード |
| Table | shadcn/ui | テーブル |
| Tabs | shadcn/ui | タブ切替 |
| Dialog/Drawer | shadcn/ui | モーダル/ドロワー |
| DatePickerWithRange | shadcn/ui | 日付範囲選択 |
| Badge | shadcn/ui | ステータスバッジ |

---

## 5. データフロー

### 5.1 データ受信フロー

```
Backend                         Frontend
   │                               │
   │  POST /api/ingest/call        │
   │ ─────────────────────────────>│
   │  { callId, from, to, ...}     │
   │                               │
   │                         ┌─────┴─────┐
   │                         │ API Route │
   │                         │ (ingest)  │
   │                         └─────┬─────┘
   │                               │
   │                         ┌─────┴─────┐
   │                         │ Local DB  │
   │                         │ (Prisma)  │
   │                         └───────────┘
```

### 5.2 画面表示フロー

```
User                      Frontend                    Local DB
  │                          │                           │
  │  アクセス /calls          │                           │
  │ ───────────────────────> │                           │
  │                          │  SELECT * FROM calls      │
  │                          │ ─────────────────────────>│
  │                          │                           │
  │                          │  [Call data]              │
  │                          │ <─────────────────────────│
  │                          │                           │
  │  <CallHistoryContent>    │                           │
  │ <─────────────────────── │                           │
```

### 5.3 録音再生フロー

```
User                      Frontend                    Backend
  │                          │                           │
  │  再生ボタン クリック       │                           │
  │ ───────────────────────> │                           │
  │                          │  GET {recordingUrl}       │
  │                          │  Range: bytes=0-          │
  │                          │ ─────────────────────────>│
  │                          │                           │
  │                          │  206 Partial Content      │
  │                          │  audio/wav                │
  │                          │ <─────────────────────────│
  │                          │                           │
  │  <AudioPlayer>           │                           │
  │ <─────────────────────── │                           │
```

> **Note**: `recordingUrl` は Backend が `POST /api/ingest/call` で通知する完全URL。
> 将来的に署名付きURL（signed URL）となる可能性があるため、Frontend は固定パスを構築せず、
> 受信した URL をそのまま使用する。

---

## 6. データ設計

### 6.1 ローカル DB スキーマ（Prisma）

```prisma
model Call {
  id          String   @id @default(cuid())
  callId      String   @unique  // Backend から受信
  from        String
  to          String
  startedAt   DateTime
  endedAt     DateTime?
  status      String   // ringing | in_call | ended | error
  summary     String?
  durationSec Int?
  recordingUrl String?
  createdAt   DateTime @default(now())
  updatedAt   DateTime @updatedAt

  transcript  Transcript?
}

model Transcript {
  id        String   @id @default(cuid())
  callId    String   @unique
  call      Call     @relation(fields: [callId], references: [callId])
  rawData   String   // ベンダー出力（JSON）
  normalized String  // UI用正規化 JSON
  srt       String?  // SRT形式
  vtt       String?  // VTT形式
  createdAt DateTime @default(now())
}
```

### 6.2 主要データ構造

| データ | 形式 | 格納先 | 説明 |
|--------|------|--------|------|
| Call | JSON → DB | Local DB | 通話メタ情報 |
| Transcript | JSON（3種） | Local DB | 文字起こし |
| Recording | WAV | Backend (`recordingUrl`) | 録音ファイル本体 |
| KPI | JSON | **MVP: モック** / post-MVP: Backend API | Dashboard 集計値 |

---

## 7. API 設計

### 7.1 IF-001: 通話データ受信（Backend → Frontend）

| 項目 | 内容 |
|------|------|
| エンドポイント | `POST /api/ingest/call` |
| 呼出元 | Backend |
| 呼出先 | Frontend API Route |
| プロトコル | HTTP |
| 同期/非同期 | 同期 |

#### リクエスト（contract.md 準拠）

```json
{
  "callId": "c_123",
  "from": "sip:zoiper@example",
  "to": "sip:bot@example",
  "startedAt": "2025-12-13T00:00:00.000Z",
  "endedAt": "2025-12-13T00:05:00.000Z",
  "status": "ended",
  "summary": "配送状況の確認。住所変更あり。",
  "durationSec": 300,
  "recording": {
    "recordingUrl": "https://backend.example/recordings/c_123/mixed.wav",
    "durationSec": 300,
    "sampleRate": 8000,
    "channels": 1
  }
}
```

#### レスポンス

```json
{
  "success": true,
  "callId": "c_123"
}
```

#### エラー

| コード | 意味 | 対処 |
|--------|------|------|
| 400 | リクエスト不正 | ペイロード検証 |
| 500 | 内部エラー | ログ確認・リトライ |

### 7.2 IF-002: KPI 取得（Frontend → Backend）【post-MVP】

> **MVP での扱い**: KPI は post-MVP で API 化予定。MVP ではモック表示で UI/導線のみ検証する。

| 項目 | 内容 |
|------|------|
| エンドポイント | `GET /api/kpi`（**post-MVP**） |
| 呼出元 | Frontend |
| 呼出先 | Backend |
| MVP 対応 | **モックデータ使用**（API 未実装） |

---

## 8. 状態管理

### 8.1 方針

| 種別 | 管理方法 | 例 |
|------|---------|-----|
| サーバー状態 | Prisma + React Query | Call 一覧 |
| モック状態（MVP） | 静的 JSON / フィクスチャ | KPI（post-MVP で API 化） |
| UI 状態 | React Context | サイドバー開閉、テーマ |
| フォーム状態 | React Hook Form | フィルタ条件 |

### 8.2 キャッシュ戦略

| データ | キャッシュ | 更新タイミング |
|--------|-----------|--------------|
| Call 一覧 | stale-while-revalidate | ページ遷移時 |
| KPI | **MVP: 静的** / post-MVP: 1分 TTL | MVP: 不要 / post-MVP: 定期 refetch |
| 録音ファイル | ブラウザキャッシュ | immutable |

---

## 9. 非機能設計方針

### 9.1 性能

| 項目 | 方針 |
|------|------|
| 初回表示 | SSR + Streaming |
| 一覧表示 | 仮想スクロール（大量データ時） |
| 音声再生 | Range リクエスト対応 |

### 9.2 可用性

| 項目 | 方針 |
|------|------|
| エラー境界 | React Error Boundary |
| フォールバック | ローカル DB からの表示 |

### 9.3 セキュリティ

| 項目 | 方針 |
|------|------|
| 認証（MVP） | なし（閉域想定） |
| 認証（将来） | NextAuth.js + JWT |
| XSS 対策 | React 標準エスケープ |

---

## 10. 前提条件・制約

### 10.1 前提条件

- Backend が `POST /api/ingest/call` で通話データを送信する
- 録音ファイルは `recordingUrl`（Backend 発行）から直接取得（Range 対応）
- KPI は **post-MVP** で Backend 集計 API 提供予定（MVP はモック表示）

### 10.2 制約

- MVP では認証なし
- v0 プロトタイプをベースに実装
- pnpm + Next.js 14 App Router

---

## 11. 未確定事項（Open Questions）

- [x] Q1: ~~文字起こしデータの形式~~ → RD-005 で解決（3点セット）
- [x] Q2: ~~KPI 集計方法~~ → RD-005 で解決（Backend 集計）
- [x] Q3: ~~KPI API のエンドポイント詳細~~ → **post-MVP へ延期**（MVP はモック表示）
- [ ] Q4: Transcript API（文字起こしデータ取得方法）

---

## 12. プロトタイプ参照

| 項目 | 値 |
|------|-----|
| GitHub | [MasanoriSuda/telephony-admin-prototype](https://github.com/MasanoriSuda/telephony-admin-prototype) |
| 実装済みコンポーネント | admin-layout, admin-sidebar, admin-header, dashboard-content, call-history-content, call-detail-drawer, audio-player 等 |

---

## 変更履歴

| 日付 | バージョン | 変更内容 | 作成者 |
|------|-----------|---------|--------|
| 2026-02-02 | 1.0 | 初版作成 | Claude Code |
| 2026-02-02 | 1.1 | Codex P2 対応: KPI API を post-MVP へ移動、recordingUrl 直接参照に変更 | Claude Code |
| 2026-02-02 | 1.2 | #62 対応: SQLite → PostgreSQL に変更 | Claude Code |
| 2026-02-05 | 2.0 | #107 対応: 新構造（virtual-voicebot-frontend/docs/）へ移行・書き直し | Claude Code |
