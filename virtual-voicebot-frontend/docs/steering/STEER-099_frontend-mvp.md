# STEER-099: Frontend MVP 全機能実装

> Frontend MVP 全機能の仕様書（Codex 実装）

| 項目 | 値 |
|------|-----|
| ステータス | Draft |
| 作成日 | 2026-02-02 |
| 関連Issue | #99 |
| 対応RD | RD-005 |
| 対応BD | BD-001 |
| 実装担当 | Codex |

---

## 1. 概要

Frontend 管理画面の MVP 全機能を Codex で実装する。

**技術スタック:**
- Next.js 14 (App Router)
- TypeScript
- Tailwind CSS
- shadcn/ui
- pnpm

**ベースプロジェクト:**
- [telephony-admin-prototype](https://github.com/MasanoriSuda/telephony-admin-prototype)

---

## 2. 実装対象（MVP スコープ）

| # | 機能 | 優先度 |
|---|------|--------|
| 1 | レイアウト（左ナビ + ヘッダー） | 必須 |
| 2 | Dashboard（KPI カード + グラフ） | 必須 |
| 3 | 発着信履歴（一覧・検索・フィルタ） | 必須 |
| 4 | 通話詳細（録音・文字起こし・要約） | 必須 |

---

## 3. 機能仕様

### 3.1 レイアウト

**構造:**
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

**ヘッダー:**
- ロゴ / アプリ名（左）
- ダークモード切替ボタン（右）
- ユーザーアバター（右、将来用プレースホルダー）

**サイドバー:**
- ロゴ
- ナビゲーション項目:
  - Dashboard（アイコン: LayoutDashboard）
  - 発着信履歴（アイコン: Phone）
- レスポンシブ: タブレット以上で固定、モバイルは折りたたみ

**受入条件:**
- [ ] AC-1: 左サイドバーに Dashboard と発着信履歴のナビゲーションが表示される
- [ ] AC-2: ナビゲーションクリックで対応ページに遷移する
- [ ] AC-3: ダークモード切替ができる
- [ ] AC-4: レスポンシブ対応（タブレット以上）

---

### 3.2 Dashboard

**パス:** `/`

**KPI カード（4枚）:**

| カード | 表示内容 | 形式 | アイコン |
|--------|---------|------|---------|
| 本日の総通話数 | 件数 + 前日比（%） | `142` `+12.5%` | Phone |
| 平均通話時間 | mm:ss 形式 | `02:34` | Clock |
| 応答率 | % 表示 | `87%` | CheckCircle |
| （予備） | 将来追加用 | `-` | TrendingUp |

**グラフ:**
- 時間帯別通話数（棒グラフ）
- X軸: 0時〜23時
- Y軸: 通話数
- 着信/発信で色分け

**データソース（MVP）:**
- モックデータを使用（静的 JSON / フィクスチャ）
- post-MVP で Backend API に接続

**受入条件:**
- [ ] AC-5: KPI カード（本日の通話数、平均通話時間、応答率）が表示される
- [ ] AC-6: 時間帯別通話数グラフが表示される
- [ ] AC-7: ダークモードでも視認性が確保される

---

### 3.3 発着信履歴

**パス:** `/calls`

**一覧テーブルカラム:**

| カラム | 説明 | ソート |
|--------|------|--------|
| 日時 | startedAt | ✓ |
| 方向 | 着信/発信/不在アイコン | - |
| 発信者 | from（電話番号 + 名前） | - |
| 着信先 | to | - |
| 通話時間 | durationSec → mm:ss 形式 | ✓ |
| ステータス | status バッジ（色分け） | - |
| 操作 | 詳細ボタン | - |

**ステータスバッジ:**

| status | 表示 | 色 |
|--------|------|-----|
| ringing | 呼出中 | yellow |
| in_call | 通話中 | blue |
| ended | 終了 | green |
| missed | 不在 | red |
| error | エラー | red |

**フィルタ条件:**
- 日付範囲: 今日 / 昨日 / 過去7日 / カスタム（DatePickerWithRange）
- 通話種別: すべて / 着信 / 発信 / 不在
- キーワード検索: 電話番号 / 名前

**ページネーション:**
- 表示件数切替: 10 / 25 / 50 件
- ページ番号ナビゲーション

**データソース:**
- Prisma 経由で Local DB（PostgreSQL）から取得

**受入条件:**
- [ ] AC-8: 通話履歴が一覧表示される
- [ ] AC-9: 日付範囲でフィルタできる
- [ ] AC-10: 通話種別（着信/発信/不在）でフィルタできる
- [ ] AC-11: キーワード検索ができる
- [ ] AC-12: ページネーションが動作する
- [ ] AC-13: 行クリックで詳細 Drawer が開く

---

### 3.4 通話詳細（3タブ）

**表示方法:** Drawer（右からスライドイン）

**ヘッダー情報:**
- 発信者番号 / 名前
- 着信先番号
- 通話日時
- 通話時間
- ステータスバッジ

**タブ構成:**

#### タブ1: 録音

- 波形付き音声プレイヤー（WaveSurfer.js 推奨）
- 再生/一時停止ボタン
- シークバー
- 現在時間 / 総時間表示
- 再生速度切替: 0.5x / 1x / 1.5x / 2x
- ダウンロードボタン
- `recordingUrl` が null の場合は「準備中」表示

#### タブ2: 文字起こし

- 話者ラベル付きトランスクリプト
  - 話者A / 話者B で色分け
- タイムスタンプ表示（クリックで音声シーク）
- コピーボタン（全文コピー）
- データがない場合は「文字起こしデータがありません」表示

#### タブ3: 要約

- AI 要約テキスト（summary フィールド）
- コピーボタン
- データがない場合は「要約がありません」表示

**受入条件:**
- [ ] AC-14: 録音タブで音声が再生できる
- [ ] AC-15: 再生速度を変更できる
- [ ] AC-16: 文字起こしタブでトランスクリプトが表示される
- [ ] AC-17: 要約タブで AI 要約が表示される
- [ ] AC-18: recordingUrl が null の場合「準備中」と表示される
- [ ] AC-19: ダウンロードボタンで録音ファイルをダウンロードできる

---

## 4. データベーススキーマ（Prisma）

```prisma
model Call {
  id          String   @id @default(cuid())
  callId      String   @unique  // Backend から受信
  from        String
  to          String
  startedAt   DateTime
  endedAt     DateTime?
  status      String   // ringing | in_call | ended | missed | error
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

---

## 5. API Routes

### 5.1 POST /api/ingest/call（Backend → Frontend）

Backend から通話データを受信して Local DB に保存。

**リクエスト:**
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

**レスポンス:**
```json
{
  "success": true,
  "callId": "c_123"
}
```

---

## 6. コンポーネント構成

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

---

## 7. 使用コンポーネント（shadcn/ui）

| コンポーネント | 用途 |
|--------------|------|
| Button | ボタン |
| Card | KPI カード |
| Table | 通話一覧 |
| Tabs | 通話詳細タブ |
| Sheet (Drawer) | 通話詳細パネル |
| DatePickerWithRange | 日付範囲フィルタ |
| Badge | ステータス表示 |
| Select | フィルタ選択 |
| Input | 検索入力 |
| Slider | 再生速度 |

---

## 8. モックデータ

### Dashboard KPI

```typescript
const mockKPI = {
  totalCalls: 142,
  totalCallsChange: 12.5, // %
  avgDurationSec: 154,
  avgDurationChange: -5.2,
  answerRate: 0.87,
  answerRateChange: 3.1,
};

const mockHourlyVolume = [
  { hour: 0, inbound: 2, outbound: 0 },
  { hour: 1, inbound: 1, outbound: 0 },
  // ... 23時まで
];
```

### 通話一覧

```typescript
const mockCalls = [
  {
    id: "1",
    callId: "c_001",
    from: "+81901234567",
    fromName: "田中太郎",
    to: "+81312345678",
    startedAt: "2026-02-02T10:30:00Z",
    endedAt: "2026-02-02T10:35:00Z",
    status: "ended",
    durationSec: 300,
    summary: "配送状況の確認。住所変更あり。",
    recordingUrl: "/mock/recording.wav",
  },
  // ...
];
```

---

## 9. 非機能要件

| 項目 | 要件 |
|------|------|
| 初回表示 | 3秒以内（LCP） |
| 一覧ページ遷移 | 1秒以内 |
| 音声再生開始 | 2秒以内 |
| ダークモード | 対応必須 |
| 日本語対応 | 必須 |
| レスポンシブ | タブレット以上 |

---

## 10. 受入条件チェックリスト

### レイアウト
- [ ] AC-1: 左サイドバーに Dashboard と発着信履歴のナビゲーションが表示される
- [ ] AC-2: ナビゲーションクリックで対応ページに遷移する
- [ ] AC-3: ダークモード切替ができる
- [ ] AC-4: レスポンシブ対応（タブレット以上）

### Dashboard
- [ ] AC-5: KPI カード（本日の通話数、平均通話時間、応答率）が表示される
- [ ] AC-6: 時間帯別通話数グラフが表示される
- [ ] AC-7: ダークモードでも視認性が確保される

### 発着信履歴
- [ ] AC-8: 通話履歴が一覧表示される
- [ ] AC-9: 日付範囲でフィルタできる
- [ ] AC-10: 通話種別（着信/発信/不在）でフィルタできる
- [ ] AC-11: キーワード検索ができる
- [ ] AC-12: ページネーションが動作する
- [ ] AC-13: 行クリックで詳細 Drawer が開く

### 通話詳細
- [ ] AC-14: 録音タブで音声が再生できる
- [ ] AC-15: 再生速度を変更できる
- [ ] AC-16: 文字起こしタブでトランスクリプトが表示される
- [ ] AC-17: 要約タブで AI 要約が表示される
- [ ] AC-18: recordingUrl が null の場合「準備中」と表示される
- [ ] AC-19: ダウンロードボタンで録音ファイルをダウンロードできる

---

## 変更履歴

| 日付 | バージョン | 変更内容 | 作成者 |
|------|-----------|---------|--------|
| 2026-02-02 | 1.0 | 初版作成（v0 向け MVP 全機能仕様） | Claude Code |
