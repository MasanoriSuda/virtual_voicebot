# v0 プロンプト: Frontend 管理画面 MVP

> 関連Issue: #99 | ステアリング: STEER-099

以下をコピーして v0 に貼り付けてください（v0 復活時用）。

---

## プロンプト

```
電話システム管理画面（Telephony Admin Dashboard）を作成してください。

## 技術スタック
- Next.js 14 (App Router)
- TypeScript
- Tailwind CSS
- shadcn/ui
- Recharts（グラフ）

## 画面構成

### 1. レイアウト
左サイドバー（240px）+ 上部ヘッダー（64px）+ メインコンテンツの3カラムレイアウト。

**ヘッダー:**
- 左: ロゴ「Virtual Voicebot」
- 右: ダークモード切替ボタン（Moon/Sun アイコン）、ユーザーアバター

**サイドバー:**
- ロゴ
- ナビゲーション:
  - Dashboard（LayoutDashboard アイコン）→ /
  - 発着信履歴（Phone アイコン）→ /calls

### 2. Dashboard ページ（/）

**KPI カード 4枚を横並び:**

| カード | タイトル | 値例 | サブテキスト | アイコン |
|--------|---------|------|-------------|---------|
| 1 | 本日の総通話数 | 142 | +12.5% 前日比 | Phone |
| 2 | 平均通話時間 | 02:34 | -5.2% 前日比 | Clock |
| 3 | 応答率 | 87% | +3.1% 前日比 | CheckCircle |
| 4 | アクティブ通話 | 3 | 現在 | Activity |

**グラフ:**
- Recharts で時間帯別通話数の棒グラフ
- X軸: 0時〜23時
- Y軸: 通話数
- 着信（青）と発信（緑）の2系列
- ダークモード対応

### 3. 発着信履歴ページ（/calls）

**フィルタバー:**
- 日付範囲選択（DatePickerWithRange）: 今日/昨日/過去7日/カスタム
- 通話種別セレクト: すべて/着信/発信/不在
- キーワード検索（Input）

**通話一覧テーブル:**

| カラム | 内容 |
|--------|------|
| 日時 | 2026/02/02 10:30（ソート可） |
| 方向 | PhoneIncoming/PhoneOutgoing/PhoneMissed アイコン |
| 発信者 | +81-90-1234-5678（田中太郎） |
| 着信先 | +81-3-1234-5678 |
| 通話時間 | 05:00 |
| ステータス | Badge（ended=緑, missed=赤, in_call=青） |
| 操作 | 詳細ボタン |

**ページネーション:**
- 10/25/50 件切替
- ページ番号ナビゲーション

**行クリックで詳細 Drawer を開く**

### 4. 通話詳細 Drawer

右からスライドインする Sheet（幅 500px）。

**ヘッダー:**
- 発信者: +81-90-1234-5678（田中太郎）
- 着信先: +81-3-1234-5678
- 日時: 2026/02/02 10:30
- 通話時間: 05:00
- ステータス Badge

**3タブ構成（Tabs）:**

**タブ1: 録音**
- 波形付き音声プレイヤー
- 再生/一時停止ボタン
- シークバー（Slider）
- 現在時間 / 総時間（00:00 / 05:00）
- 再生速度切替ボタン: 0.5x / 1x / 1.5x / 2x
- ダウンロードボタン
- 録音がない場合: 「準備中」と表示

**タブ2: 文字起こし**
- 会話形式で表示:
  ```
  [00:00] 話者A: お電話ありがとうございます。
  [00:05] 話者B: 配送状況を確認したいのですが。
  [00:10] 話者A: かしこまりました。お名前をお願いします。
  ```
- 話者Aは左寄せ（青背景）、話者Bは右寄せ（グレー背景）
- コピーボタン

**タブ3: 要約**
- AI要約テキスト表示:
  「配送状況の確認。お客様から住所変更の依頼があり、新住所を登録。再配送を手配。」
- コピーボタン

## デザイン要件

- ダークモード完全対応
- 日本語UI
- レスポンシブ（タブレット以上）
- shadcn/ui のデフォルトスタイルを活用
- カラー: primary は青系

## モックデータ

通話一覧用のモックデータを10件程度含めてください:
- 着信/発信/不在が混在
- ステータス: ended, missed, in_call
- 日本の電話番号形式（+81-XX-XXXX-XXXX）
- 日本人の名前

## ファイル構成

```
app/
├── layout.tsx
├── page.tsx (Dashboard)
├── calls/
│   └── page.tsx (発着信履歴)
components/
├── admin-layout.tsx
├── admin-header.tsx
├── admin-sidebar.tsx
├── dashboard/
│   ├── kpi-cards.tsx
│   └── hourly-chart.tsx
├── calls/
│   ├── filter-bar.tsx
│   ├── calls-table.tsx
│   ├── call-detail-drawer.tsx
│   └── audio-player.tsx
lib/
└── mock-data.ts
```

全画面を実装してください。
```

---

## 補足（必要に応じて追加）

### データベース連携が必要な場合

```
Prisma を使用して PostgreSQL に接続します。

スキーマ:
- Call: id, callId, from, to, startedAt, endedAt, status, summary, durationSec, recordingUrl
- Transcript: id, callId, rawData, normalized, srt, vtt

API Route:
- POST /api/ingest/call: Backend から通話データを受信して DB に保存
```

### 音声プレイヤーライブラリ

```
音声プレイヤーには WaveSurfer.js を使用してください。
波形表示 + 再生/一時停止/シーク機能を実装。
```
