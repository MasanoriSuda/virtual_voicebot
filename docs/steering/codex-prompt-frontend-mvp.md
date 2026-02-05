# Codex プロンプト: Frontend 管理画面 MVP 実装

> 関連Issue: #99 | ステアリング: STEER-099

以下を Codex に貼り付けてください。

---

## プロンプト

```
# タスク: Frontend 管理画面 MVP 実装（#99）

## 概要
電話システム管理画面（Telephony Admin Dashboard）を実装してください。

## 対象リポジトリ
- ベース: https://github.com/MasanoriSuda/telephony-admin-prototype
- または新規作成

## 技術スタック（必須）
- Next.js 14 (App Router)
- TypeScript
- Tailwind CSS
- shadcn/ui
- Recharts（グラフ）
- pnpm

## 実装する画面

### 1. レイアウト（app/layout.tsx）

構造:
- 左サイドバー（240px）
- 上部ヘッダー（64px）
- メインコンテンツ

ヘッダー:
- 左: ロゴ「Virtual Voicebot」
- 右: ダークモード切替（next-themes）、ユーザーアバター（プレースホルダー）

サイドバー:
- ナビゲーション:
  - Dashboard（LayoutDashboard アイコン）→ /
  - 発着信履歴（Phone アイコン）→ /calls

### 2. Dashboard（app/page.tsx）

KPI カード 4枚（横並び grid）:
1. 本日の総通話数: 142, +12.5% 前日比, Phone アイコン
2. 平均通話時間: 02:34, -5.2% 前日比, Clock アイコン
3. 応答率: 87%, +3.1% 前日比, CheckCircle アイコン
4. アクティブ通話: 3, 現在, Activity アイコン

グラフ（Recharts BarChart）:
- 時間帯別通話数（0時〜23時）
- 着信（青）と発信（緑）の2系列
- ダークモード対応

### 3. 発着信履歴（app/calls/page.tsx）

フィルタバー:
- DatePickerWithRange（shadcn/ui）: 今日/昨日/過去7日/カスタム
- Select: すべて/着信/発信/不在
- Input: キーワード検索

テーブル（shadcn/ui Table）:
| カラム | 内容 |
|--------|------|
| 日時 | startedAt（ソート可） |
| 方向 | PhoneIncoming/PhoneOutgoing/PhoneMissed アイコン |
| 発信者 | from + fromName |
| 着信先 | to |
| 通話時間 | durationSec → mm:ss |
| ステータス | Badge（ended=緑, missed=赤, in_call=青） |
| 操作 | 詳細ボタン |

ページネーション:
- 10/25/50 件切替
- ページ番号

行クリック → 詳細 Drawer 開く

### 4. 通話詳細 Drawer（components/calls/call-detail-drawer.tsx）

Sheet（shadcn/ui）、右からスライド、幅 500px

ヘッダー:
- 発信者/着信先/日時/通話時間/ステータス

Tabs（3タブ）:

**タブ1: 録音**
- 音声プレイヤー（HTML5 audio または wavesurfer.js）
- 再生/一時停止
- シークバー
- 現在時間/総時間
- 再生速度: 0.5x/1x/1.5x/2x
- ダウンロードボタン
- recordingUrl が null → 「準備中」表示

**タブ2: 文字起こし**
- 会話形式表示（話者A/B で色分け）
- タイムスタンプ付き
- コピーボタン

**タブ3: 要約**
- summary テキスト表示
- コピーボタン

## ファイル構成

```
app/
├── layout.tsx
├── page.tsx
├── calls/
│   └── page.tsx
├── api/
│   └── ingest/
│       └── call/
│           └── route.ts
components/
├── admin-layout.tsx
├── admin-header.tsx
├── admin-sidebar.tsx
├── theme-toggle.tsx
├── dashboard/
│   ├── kpi-cards.tsx
│   └── hourly-chart.tsx
├── calls/
│   ├── filter-bar.tsx
│   ├── calls-table.tsx
│   ├── call-detail-drawer.tsx
│   └── audio-player.tsx
lib/
├── mock-data.ts
└── utils.ts
```

## モックデータ（lib/mock-data.ts）

```typescript
export const mockCalls = [
  {
    id: "1",
    callId: "c_001",
    from: "+81-90-1234-5678",
    fromName: "田中太郎",
    to: "+81-3-1234-5678",
    startedAt: "2026-02-02T10:30:00Z",
    endedAt: "2026-02-02T10:35:00Z",
    status: "ended",
    durationSec: 300,
    summary: "配送状況の確認。住所変更あり。",
    recordingUrl: "/mock/recording.wav",
  },
  // 10件程度作成（着信/発信/不在混在）
];

export const mockKPI = {
  totalCalls: 142,
  totalCallsChange: 12.5,
  avgDurationSec: 154,
  avgDurationChange: -5.2,
  answerRate: 0.87,
  answerRateChange: 3.1,
  activeCalls: 3,
};

export const mockHourlyVolume = [
  { hour: 0, inbound: 2, outbound: 0 },
  { hour: 1, inbound: 1, outbound: 0 },
  // ... 23時まで
];

export const mockTranscript = [
  { time: "00:00", speaker: "A", text: "お電話ありがとうございます。" },
  { time: "00:05", speaker: "B", text: "配送状況を確認したいのですが。" },
  { time: "00:10", speaker: "A", text: "かしこまりました。お名前をお願いします。" },
];
```

## 要件

1. ダークモード完全対応（next-themes）
2. 日本語 UI
3. レスポンシブ（タブレット以上）
4. shadcn/ui コンポーネント活用
5. TypeScript 型安全

## API Route（将来用）

POST /api/ingest/call:
- Backend から通話データを受信
- MVP ではモックデータ使用、API は空実装で可

## 実装順序

1. shadcn/ui セットアップ（npx shadcn@latest init）
2. 必要なコンポーネント追加（Button, Card, Table, Tabs, Sheet, Badge, DatePicker, Select, Input）
3. レイアウト実装
4. Dashboard 実装
5. 発着信履歴 実装
6. 通話詳細 Drawer 実装
7. ダークモード対応

全画面を実装してください。
```
