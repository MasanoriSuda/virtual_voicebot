# STEER-266: 着信ポップアップ通知

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-266 |
| タイトル | 着信ポップアップ通知 |
| ステータス | Approved |
| 関連Issue | #266 |
| 優先度 | P1 |
| 作成日 | 2026-02-28 |

---

## 2. ストーリー（Why）

### 2.1 背景

着信があっても Frontend の画面を見ていなければ気づけない。現状、通話履歴は serversync（通話終了後の非同期同期）で Frontend に届くが、通話**中**のリアルタイム通知手段がない。

- 通常着信（直接転送）: オペレーターが電話に出るタイミングを逃す
- IVR 経由の転送着信: オペレーターがどのボタン操作で来た呼なのか把握できない

### 2.2 目的

着信が発生した瞬間に Frontend 画面上でポップアップを表示し、以下を実現する:

1. **着信の即時認知**: 通話開始前にオペレーターが着信に気づける
2. **IVR 経路の可視化**: IVR 経由の転送時に、押下順（押し間違い含む）と IVR 滞留時間を確認できる
3. **発信者番号の表示**: どこからかかってきたか即座に把握できる

### 2.3 ユーザーストーリー

```
As a オペレーター（Frontend を開いている担当者）
I want to 着信が来たときに画面上のポップアップで通知を受け取る
So that 着信に即座に気づき、IVR 経路も含めた呼の状況を把握した上で対応できる

受入条件:
- [ ] 通常着信（直接転送）の際にポップアップが表示される
- [ ] IVR 転送着信の際にポップアップが表示される
- [ ] ポップアップに発信者番号が表示される
- [ ] IVR 転送着信時は押下番号の順番（押し間違い含む）が表示される
- [ ] IVR 転送着信時は IVR 滞留時間（秒）が表示される
- [ ] 複数着信が同時に発生した場合、スタック形式で複数表示される
- [ ] × ボタンでポップアップを手動で閉じられる
- [ ] 閉じたポップアップはブラウザリロード後に復活しない
- [ ] 着信検知の遅延は概ね 1〜2 秒以内（serversync 1秒ポーリング + ブラウザ 1秒ポーリングの位相ずれにより最悪ケースで 2 秒超の場合あり、P95 3秒を目安とする）
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-28 |
| 起票理由 | 着信時のオペレーター通知手段がなく、対応漏れリスクがある |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Sonnet 4.6 |
| 作成日 | 2026-02-28 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "Issue #266: 着信時に転送ポップアップが出るようにしてほしい。既存アーキ踏襲で1秒間隔のserversyncが通知を投げる。着信番号・IVR滞留時間・押下番号（手順）を表示" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| - | codex | 2026-02-28 | OK | OK|

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | @MasanoriSuda |
| 承認日 | 2026-02-28 |
| 承認コメント | lgtm |

### 3.5 実装（該当する場合）

| 項目 | 値 |
|------|-----|
| 実装者 | Codex (GPT-5) |
| 実装日 | 2026-02-28 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "承認したので実装お願いします Refs #266" |
| コードレビュー | 未実施（別途 PR で実施） |

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
| docs/contract.md | 追加 | `POST /api/ingest/incoming-call`、`GET /api/incoming-call-notifications`、`DELETE /api/incoming-call-notifications/{id}` を追加 |
| virtual-voicebot-backend/docs/design/detail/DD-xxx.md | 追加 | NotificationWorker の詳細設計 |
| virtual-voicebot-frontend/docs/design/detail/DD-xxx.md | 追加 | IncomingCallPopup コンポーネントの詳細設計 |

### 4.2 影響するコード

| モジュール | 変更種別 | 概要 |
|-----------|---------|------|
| `virtual-voicebot-backend/src/protocol/session/coordinator.rs` | 修正 | `is_ivr_call`・`ivr_started_at`・`dtmf_history`・`notification_sent` フィールド追加 |
| `virtual-voicebot-backend/src/protocol/session/handlers/mod.rs` | 修正 | ①180 Ringing 処理時に `is_ivr_call` で分岐して `direct` 通知を書き込み; ②`IvrAction::Transfer` 処理時（L873 付近）に IVR 滞留時間・DTMF 履歴を計算して `ivr_transfer` 通知を書き込み |
| `virtual-voicebot-backend/src/bin/serversync.rs` | 修正 | `NotificationWorker` を起動するよう追加 |
| `virtual-voicebot-backend/src/interface/sync/notification_worker.rs` | 追加 | 新規 Worker（1秒ポーリング、ファイル→Frontend POST） |
| `virtual-voicebot-frontend/app/api/ingest/incoming-call/route.ts` | 追加 | 着信通知受信エンドポイント（新規） |
| `virtual-voicebot-frontend/app/api/incoming-call-notifications/route.ts` | 追加 | GET（一覧取得）エンドポイント（新規） |
| `virtual-voicebot-frontend/app/api/incoming-call-notifications/[id]/route.ts` | 追加 | DELETE（dismiss）エンドポイント（新規） |
| `virtual-voicebot-frontend/lib/db/notifications.ts` | 追加 | 独立通知ストレージ（`notifications.json`）の読み書き（新規） |
| `virtual-voicebot-frontend/components/IncomingCallPopup.tsx` | 追加 | AlertDialog スタックコンポーネント（新規） |
| `virtual-voicebot-frontend/hooks/useIncomingCallNotifications.ts` | 追加 | 1秒ポーリング専用 hook（新規） |

---

## 5. 差分仕様（What / How）

### 5.1 アーキテクチャ概要

既存の `sync_outbox`（通話終了後の非同期同期）とは**完全独立**した fast-path 通知機構を追加する。既存の OutboxWorker・DB スキーマ・`sync.json` には一切変更を加えない。

```
[Backend プロセス]
着信（180 Ringing 処理時）
  → storage/notifications/pending.jsonl に1行 JSON 追記

IVR 転送（IvrAction::Transfer 処理時 ※handlers/mod.rs L873 付近のみ）
  → 同ファイルに1行 JSON 追記
    ※ IVR 開始時刻〜転送時刻の差分で dwellTimeSec を計算
    ※ DTMF 押下履歴（dtmfHistory）を SessionCoordinator が内部保持
    ※ IVR 転送コールには 180 Ringing 通知を発火しない（is_ivr_call フラグで判定、direct/ivr_transfer は相互排他）
    ※ 1コール1通知を保証するため SessionCoordinator に `is_ivr_call: bool` + `notification_sent: bool` フラグを持つ

[serversync プロセス - 新規 NotificationWorker]
1秒ポーリング
  → pending.jsonl を rename()（原子的）→ 読む → 削除
  → POST /api/ingest/incoming-call → Frontend サーバー

[Frontend Server (Next.js)]
POST /api/ingest/incoming-call
  → notifications.json に id（UUID）を付与して追記
  ※ dismissed フィールドは持たない（delete ベース: ファイル上に存在する = 未 dismiss）
  ※ sync.json とは完全独立

GET /api/incoming-call-notifications
  → 保存中の全エントリ（= 未 dismiss）を返す

DELETE /api/incoming-call-notifications/{id}
  → 該当エントリを削除（ブラウザリロード後に復活しない）

[ブラウザ]
1秒ポーリング: GET /api/incoming-call-notifications
  → 取得したエントリをすべて AlertDialog スタックで表示（取得できた = 未 dismiss）
  × ボタン → DELETE /api/incoming-call-notifications/{id}
```

### 5.2 通知ペイロード定義（contract.md へマージ）

**追加先**: `docs/contract.md` §（Backend → Frontend ingest）

#### POST /api/ingest/incoming-call

Backend serversync が Frontend に着信通知を送るエンドポイント。

**リクエスト Body（通常着信）**:

```json
{
  "callerNumber": "+81-3-xxxx-xxxx",
  "trigger": "direct",
  "receivedAt": "2026-02-28T10:00:00Z"
}
```

**リクエスト Body（IVR 転送着信）**:

```json
{
  "callerNumber": "+81-3-xxxx-xxxx",
  "trigger": "ivr_transfer",
  "receivedAt": "2026-02-28T10:00:05Z",
  "ivrData": {
    "dwellTimeSec": 42,
    "dtmfHistory": ["9", "9", "3"]
  }
}
```

**フィールド定義**:

| フィールド | 型 | 必須 | 説明 |
|-----------|-----|------|------|
| `callerNumber` | string | ✓ | 発信者番号（SIP URI の user 部分のみ抽出。例: `sip:0312345678@domain` → `"0312345678"`） |
| `trigger` | `"direct"` \| `"ivr_transfer"` | ✓ | 着信契機 |
| `receivedAt` | ISO8601 | ✓ | 着信検知時刻 |
| `ivrData` | object | trigger="ivr_transfer" 時のみ | IVR 経路情報 |
| `ivrData.dwellTimeSec` | number | ✓（ivr_transfer 時） | IVR 滞留時間（秒、切り捨て） |
| `ivrData.dtmfHistory` | string[] | ✓（ivr_transfer 時） | DTMF 押下順の配列（押し間違い含む） |

**レスポンス**: `{ "ok": true }` / エラー時 `{ "ok": false, "error": "..." }`

---

#### GET /api/incoming-call-notifications

ブラウザが未 dismiss の通知一覧を取得するエンドポイント。

**レスポンス**:

```json
{
  "notifications": [
    {
      "id": "UUID",
      "callerNumber": "+81-3-xxxx-xxxx",
      "trigger": "direct",
      "receivedAt": "2026-02-28T10:00:00Z",
      "ivrData": null
    },
    {
      "id": "UUID",
      "callerNumber": "+81-6-xxxx-xxxx",
      "trigger": "ivr_transfer",
      "receivedAt": "2026-02-28T10:00:05Z",
      "ivrData": {
        "dwellTimeSec": 42,
        "dtmfHistory": ["9", "9", "3"]
      }
    }
  ]
}
```

---

#### DELETE /api/incoming-call-notifications/{id}

ユーザーが × を押したときにブラウザが呼ぶエンドポイント。対象エントリを `notifications.json` から削除する。

**実装ファイル**: `app/api/incoming-call-notifications/[id]/route.ts`（動的ルート、新規）
既存実装パターン: `app/api/announcements/[id]/route.ts` と同一の Next.js 動的ルート方式を踏襲する。

**レスポンス**: `{ "ok": true }` / 該当なし時 `{ "ok": false, "error": "not_found" }`

---

### 5.3 Backend: ファイル排他制御

- **ファイルパス**: 環境変数 `NOTIFICATION_QUEUE_FILE`（デフォルト: `storage/notifications/pending.jsonl`）
- **Backend 書き込み**: `OpenOptions::append(true).create(true).open()` で O_APPEND 追記（各行は完結した JSON + `\n`）
- **serversync 読み取り（正常系）**:
  1. `pending.jsonl.processing` が存在すれば、先にそちらを処理する（再起動/前周期失敗からの復旧）
  2. `pending.jsonl` を `pending.jsonl.processing` に `rename()` で原子的に退避
  3. 各行を読んで Frontend に POST
  4. 全行送信成功後に `pending.jsonl.processing` を削除
- **serversync 読み取り（失敗系）**:
  - POST 失敗時は `pending.jsonl.processing` をそのまま残し、次周期の冒頭（ステップ1）で再送する
  - `rename()` 成功後にファイル読み込みが失敗した場合も同様に次周期で再試行する
  - これにより「Frontend 一時停止」「serversync 再起動」どちらのケースでも通知が失われない

### 5.4 Backend: IVR 滞留時間の計算と通知冪等制御

`SessionCoordinator` に以下を追加:

- `is_ivr_call: bool` — ルーティング評価時に IVR フロー行きと判定された場合 true（180 Ringing 送信より前に確定する）
- `ivr_started_at: Option<Instant>` — IVR 開始時刻（`IvrMenuWaiting` 等に初めて遷移したタイミング）
- `dtmf_history: Vec<char>` — DTMF 入力を受信するたびに push（`IvrAction::Invalid` 含む全入力）
- `notification_sent: bool` — 通知書き込み済みフラグ（1コール1通知の冪等保証）

**実装順序と `is_ivr_call` の必要性**:

```
ルーティング評価
  → is_ivr_call = true/false を確定  ← ここで判定
  ↓
180 Ringing 送信（handlers/mod.rs L169）
  → is_ivr_call が false かつ notification_sent == false → direct 通知を書き込み  ← ここで発火判定
  ↓
コール応答（200 OK）
  ↓
IVR フロー開始（ivr_started_at を記録）  ← 180 より後なので、ここは抑止条件に使えない
```

`ivr_started_at` は 180 Ringing 送信の**後**に設定される（L316 付近）。このため 180 時点での `direct` 通知の抑止条件として `ivr_started_at` を使うと競合する。代わりにルーティング評価時点で確定する `is_ivr_call` を使う。

**`is_ivr_call` の設定タイミング（実装ヒント）**:
- DB IVR フローを使う場合: `ivr_flow_id.is_some()` が判断基準になる
- レガシー IVR メニューを使う場合: `voicebot_direct_mode == false && announce_mode == false && outbound_mode == false` の条件で IVR に入るため、同条件を参照する
- 詳細は Codex 実装時に `initial_action_code` 等の既存フィールドと照合して確定すること

**通知の発火ポイント（重要）**:

`IvrState::Transferring` への遷移箇所は `handlers/mod.rs` 内に複数存在するが、着信ポップアップ通知を発火するのは **`IvrAction::Transfer` 処理箇所のみ**（DTMF "3" が押されて IVR メニューから転送を選んだ場合、L873 付近）。

| 箇所 | 条件 | 通知発火 |
|------|------|---------|
| L138 (`outbound_mode = true`) | 初期ルーティングが B2BUA 直接転送（IVR なし） | **direct** で発火（180 Ringing 時） |
| L275 (`announce_mode`) | アナウンス再生後の転送（IVR なし） | **direct** で発火（180 Ringing 時） |
| L873 (`IvrAction::Transfer`) | IVR メニューで転送ボタン（DTMF "3"）が押された | **ivr_transfer** で発火（ここのみ） |
| L957 (`start_b2bua_transfer`) | VoiceBot/App からの転送指示 | 通知しない（VB/App ルートはスコープ外） |

**IVR 転送コールの通知は `direct` を発火しない**:
- 180 Ringing 時に `is_ivr_call == true` であれば `direct` 通知を発火しない（`notification_sent` は更新しない）
- `IvrAction::Transfer` 処理時に `notification_sent == false` であれば `ivr_transfer` 通知を発火する
- これにより `direct` / `ivr_transfer` は相互排他となり、1コール1通知が保証される

**滞留時間の計算**（`ivr_transfer` 発火時）:
```
dwellTimeSec = (Instant::now() - ivr_started_at).as_secs()  // 切り捨て
dtmfHistory  = dtmf_history の char を string 配列に変換
```

### 5.5 Backend: NotificationWorker（serversync）

**ファイル**: `src/interface/sync/notification_worker.rs`（新規）

```rust
pub struct NotificationWorker {
    queue_file: PathBuf,          // NOTIFICATION_QUEUE_FILE
    frontend_base_url: String,    // FRONTEND_BASE_URL
    client: reqwest::Client,
}

impl NotificationWorker {
    pub async fn run(&self) {
        let mut ticker = interval(Duration::from_secs(1));
        loop {
            ticker.tick().await;
            if let Err(e) = self.process().await {
                log::warn!("[notification_worker] error: {}", e);
            }
        }
    }

    async fn process(&self) -> Result<(), ...> {
        let processing = self.queue_file.with_extension("jsonl.processing");

        // ステップ1: 前周期/再起動で残った .processing を優先処理（復旧）
        if processing.exists() {
            self.flush_processing_file(&processing).await?;
        }

        // ステップ2: pending.jsonl が無ければスキップ
        if !self.queue_file.exists() { return Ok(()); }

        // ステップ3: 原子的に退避（以降の Backend 書き込みは新 pending.jsonl へ）
        std::fs::rename(&self.queue_file, &processing)?;

        // ステップ4: 送信（失敗時は .processing を残して return → 次周期ステップ1で再送）
        self.flush_processing_file(&processing).await
    }

    async fn flush_processing_file(&self, processing: &PathBuf) -> Result<(), ...> {
        let content = std::fs::read_to_string(processing)?;
        for line in content.lines().filter(|l| !l.is_empty()) {
            // POST 失敗時はここで Err を返し、.processing を残す
            self.send_notification(line).await?;
        }
        // 全行送信成功後にのみ削除
        std::fs::remove_file(processing)?;
        Ok(())
    }
}
```

### 5.6 Frontend: 独立ストレージ

**ファイル**: `virtual-voicebot-frontend/lib/db/notifications.ts`（新規）

- ストレージ先: `storage/notifications/notifications.json`（`sync.json` とは独立）
- エントリ構造: `{ id: string, callerNumber: string, trigger: string, receivedAt: string, ivrData: IvrData | null }`
  （`dismissed` フィールドは持たない。ファイル上に存在するエントリ = すべて未 dismiss）
- `addNotification(entry)`: POST 受信時に id（UUID v4）を付与して追記
- `getNotifications()`: 全エントリを返す
- `deleteNotification(id)`: 該当エントリを削除（これが dismiss 操作）

### 5.7 Frontend: UI コンポーネント

**ファイル**: `virtual-voicebot-frontend/components/IncomingCallPopup.tsx`（新規）

- `useIncomingCallNotifications` hook（1秒ポーリング）と組み合わせ
- 通知ごとに `AlertDialog`（または `Toast`）を独立表示（スタック）
- 表示内容:

| フィールド | 表示条件 | 例 |
|-----------|---------|-----|
| 発信者番号 | 常時 | `+81-3-xxxx-xxxx` |
| 着信種別 | 常時 | `直接着信` / `IVR 転送` |
| IVR 滞留時間 | trigger="ivr_transfer" | `42 秒` |
| 押下番号（手順） | trigger="ivr_transfer" | `9 → 9 → 3` |

- × ボタン押下: `DELETE /api/incoming-call-notifications/{id}` を呼んでポップアップを閉じる

---

### 5.8 受入条件

- [ ] 通常着信（直接転送）発生時に Frontend ポップアップに発信者番号が表示される
- [ ] IVR 転送着信時に発信者番号・IVR 滞留時間・押下順（押し間違い含む）が表示される
- [ ] 着信から概ね 1〜2 秒以内（P95 3秒以内）にポップアップが表示される（serversync + ブラウザ双方 1秒ポーリング、位相ずれあり）
- [ ] 複数着信が同時に来た場合、スタック形式で複数表示される
- [ ] × ボタンでポップアップを個別に閉じられる
- [ ] 閉じたポップアップはブラウザリロード後に復活しない
- [ ] 既存 `sync_outbox` / `OutboxWorker` / `sync.json` に変更がない
- [ ] `pending.jsonl` の rename による原子的読み取りが実装されている
- [ ] Backend と serversync が `NOTIFICATION_QUEUE_FILE` 環境変数で同じファイルパスを参照している
- [ ] `notifications.json` が `sync.json` と独立したファイルに保存される
- [ ] Backend 停止中・ファイル未生成時でも NotificationWorker がパニックしない
- [ ] Frontend 5xx 時に `pending.jsonl.processing` が残り、次周期で再送される（通知取りこぼしなし）
- [ ] serversync 再起動後に `pending.jsonl.processing` が存在する場合、起動初回の処理で再送される
- [ ] 1コールにつき通知は1件（`direct` / `ivr_transfer` いずれか一方のみ、重複なし）

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #266 | STEER-266 | 起票 |
| STEER-266 | contract.md §ingest | API エンドポイント追加（POST ingest, GET/DELETE notifications） |
| STEER-266 | Backend DD-xxx | NotificationWorker / coordinator 修正 詳細設計 |
| STEER-266 | Frontend DD-xxx | IncomingCallPopup / notifications.ts 詳細設計 |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [ ] 着信トリガーのタイミング（180 Ringing）が SessionCoordinator の実装と一致しているか
- [ ] IVR 転送通知の発火箇所が `IvrAction::Transfer` 処理（L873 付近）のみに限定されているか（L138/L275/L957 では発火しないこと）
- [ ] IVR 転送コールで `direct` 通知が発火しない（ルーティング評価時に確定する `is_ivr_call` フラグで判定、180 Ringing より前に確定すること）
- [ ] `notification_sent` フラグによる1コール1通知の冪等保証が実装されているか
- [ ] `dtmf_history` が `IvrAction::Invalid`（押し間違い）を含む全入力を記録しているか
- [ ] `pending.jsonl.processing` 残存時の優先再処理（起動時・各周期冒頭）が実装されているか
- [ ] POST 失敗時に `pending.jsonl.processing` が削除されず次周期で再送されることが保証されているか
- [ ] `pending.jsonl` の rename 原子性がサーバー OS（Linux）で保証されているか（同一ファイルシステム上であること）
- [ ] `DELETE /api/incoming-call-notifications/{id}` が `[id]/route.ts` の動的ルートで実装されているか
- [ ] `NOTIFICATION_QUEUE_FILE` の共有パスが Backend / serversync の両方から到達可能か
- [ ] `notifications.json` が `sync.json` と独立していることが明示されているか
- [ ] ブラウザリロード後に通知が復活しない（dismiss = delete）設計になっているか
- [x] スタック表示の上限: 上限なし（2ch 構成のため同時着信数が少ない、OQ-1 確定）
- [ ] 既存 OutboxWorker への影響がないことが確認されているか

### 7.2 マージ前チェック（Approved → Merged）

- [ ] 実装が完了している
- [ ] コードレビューを受けている
- [ ] 受入条件が全て Pass している
- [ ] contract.md への反映準備ができている

---

## 8. 備考

### 8.1 設計判断

**なぜ sync_outbox を使わず独立ファイルにしたか**

- `sync_outbox` の OutboxWorker は通話終了後エンティティ（call_log, recording 等）を扱う。着信通知はこれらと性質が異なる（通話中・一時的・消費型）
- OutboxWorker の `SYNC_POLL_INTERVAL_SEC` を 1 秒にすると全エンティティのポーリングが 1 秒になり、通話履歴・録音ファイルのアップロードにも影響する
- 独立ファイル + 専用 Worker とすることで、既存機能に影響を与えずスコープを着信通知のみに限定できる

**なぜ DB テーブルではなくファイルにしたか**

- 着信通知は消費型の一時データであり、永続的な DB レコードとして扱う必要がない
- ファイルの append + rename による軽量な排他制御で十分
- Backend と serversync が同一ホスト上で動作するため、ファイル共有に問題がない（Issue #266 の前提）

**`callerNumber` の値について**

- `SessionCoordinator.from_uri` は SIP URI 形式（例: `sip:0312345678@domain`）
- Backend が `pending.jsonl` に書き込む時点で user 部分（`@` より前の文字列）を抽出する
- `sip:` プレフィックスがある場合は除去し、数字・`+` のみの番号文字列として格納する
- ペイロード例: `"callerNumber": "0312345678"`

### 8.2 Open Questions

| # | 質問 | 採択 | 状態 |
|---|------|------|------|
| OQ-1 | ポップアップのスタック上限を設けるか | 上限なし — 2ch 構成のため同時着信数が少なく、上限制御は不要 | **確定** |
| OQ-2 | `callerNumber` を SIP URI のまま渡すか user 部分のみ抽出するか | (B) user 部分のみ抽出（`@` より前、`sip:` 除去後）— 表示用途なので番号のみが適切 | **確定** |

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-28 | 初版作成 | Claude Sonnet 4.6 |
| 2026-02-28 | レビュー NG Round 1 対応: ①NotificationWorker `.processing` 復旧フロー追加（重大1）; ②DELETE route を `[id]/route.ts` に分離（重大2）; ③`callerNumber` を user 部分抽出で一本化・OQ-2 確定（重大3）; ④SLO を「P95 3秒」表現に修正（中1）; ⑤IVR 通知発火箇所を `IvrAction::Transfer` のみに限定・1コール1通知冪等保証を追加（中2）; ⑥受入条件に再送・復旧・重複なし AC 追加（軽1）; §7.1 チェックリスト更新 | Claude Sonnet 4.6 |
| 2026-02-28 | レビュー NG Round 2 対応: ①`direct` 抑止条件を `ivr_started_at`（180 より後）→ `is_ivr_call`（ルーティング評価時確定）に変更、実装順序の矛盾を解消（重大1）; ②§4.2 IVR 通知ロジックの対象ファイルを `ivr_service.rs` → `handlers/mod.rs` に修正（中2）; ③`dismissed` フィールドを削除・delete ベースに統一（§5.1 / §5.6）（軽3） | Claude Sonnet 4.6 |
| 2026-02-28 | レビュー NG Round 3 対応: `dismissed: false` の残存表現（§5.1 GET 説明・ブラウザ説明）を「保存中の全エントリ（= 未 dismiss）」に統一（軽） | Claude Sonnet 4.6 |
