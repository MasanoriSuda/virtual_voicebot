# STEER-275: フロントエンド発着信履歴の発信時表示修正

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-275 |
| タイトル | フロントエンド発着信履歴の発信時表示修正 |
| ステータス | Approved |
| 関連Issue | #275 |
| 優先度 | P1 |
| 作成日 | 2026-03-02 |

---

## 2. ストーリー（Why）

### 2.1 背景

- Issue #272（STEER-272）にてバックエンドの発着信同時サポートが実装されたが、フロントエンドの発着信履歴画面は発信通話を想定した作り込みをしていなかった。
- 実際に発信が行われた通話ログを確認すると、以下の不正表示が確認されている：

| 列 | 現状（不正） | 期待 |
|----|-------------|------|
| 方向 | 着信 | 発信 |
| 発信者 | 未登録 / 非通知 | 発信元番号（登録 SIP ユーザー）|
| 着信先 | 未設定 | 発信先番号（顧客電話番号）|

- フロントエンドは方向を `actionCode === "AR"` の場合のみ「発信」と判定しているが、STEER-272 で実装されたの発信通話（B2BUA 転送）は AI パイプラインを使用しないため、`actionCode` が "AR" ではない可能性がある（ログ上では VB = ボイスボットが表示されている）。
- バックエンドの API レスポンスには `direction`（発信/着信）フィールドが含まれておらず、フロントエンドが方向を正確に判定できない。
- 着信先（発信先番号）の情報もAPIレスポンスに含まれていない。

### 2.2 目的

- 発信通話の履歴表示を正しくする：
  1. **方向列**：発信バッジ（「発信」+PhoneOutgoing アイコン）を正しく表示する
  2. **発信者列**：発信元番号（登録 SIP ユーザー番号）を表示する
  3. **着信先列**：発信先番号（顧客電話番号）を表示する
  4. **その他列**：発信時に意味を持たない列（着信応答・IVR詳細等）を「-」表示にする
  5. **ステータス**：発信通話の displayStatus を定義する
- バックエンドが `direction` および `calleeNumber` をAPIレスポンスに含めることで、フロントエンドが信頼性高く判定できるようにする。

### 2.3 ユーザーストーリー

```
As a 管理者
I want to 発着信履歴で発信通話を一見して「発信」と判別できる表示にしたい
So that 着信と発信の通話実績を正確に区別して管理できる

受入条件:
- [ ] AC-1: direction = "outbound" の通話は 方向列に「発信」バッジ（PhoneOutgoing アイコン、emerald 色）が表示される
- [ ] AC-2: direction = "outbound" の通話は 発信者列に発信元番号（callerNumber、登録 SIP ユーザー番号）が表示される
- [ ] AC-3: direction = "outbound" の通話は 着信先列に発信先番号（calleeNumber）が表示される
- [ ] AC-4: direction = "outbound" の通話の 着信応答列 は "-" が表示される
- [ ] AC-5: direction = "outbound" の通話の IVR詳細列 は "-" が表示される
- [ ] AC-6: direction = "outbound" 通話の displayStatus は §5.10 で定義したルールで決定される
- [ ] AC-7: フィルタバーで「発信」を選択した場合、direction = "outbound" の通話のみ絞り込まれる
- [ ] AC-8: CSV エクスポートの「方向」列に「発信」と出力される
- [ ] AC-9: 通話詳細 Drawer（call-detail-drawer）でも発信通話の表示が正しく行われる
- [ ] AC-10: バックエンド API レスポンスに direction（"inbound" | "outbound"）が含まれる
- [ ] AC-11: バックエンド API レスポンスに calleeNumber（string | null）が含まれる
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-03-02 |
| 起票理由 | #272 発着信サポート実装後、発信通話の履歴表示が発信用に作り込まれていないことが判明 |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Code (claude-sonnet-4-6) |
| 作成日 | 2026-03-02 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "Issue #275 の発着信履歴発信時表示修正のステアリングを作成" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| 1 | @MasanoriSuda | 2026-03-02 | OK | lgtm |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | @MasanoriSuda |
| 承認日 | 2026-03-02 |
| 承認コメント | lgtm |

### 3.5 実装

| 項目 | 値 |
|------|-----|
| 実装者 | Codex |
| 実装日 | - |
| 指示者 | @MasanoriSuda |
| 指示内容 | "STEER-275 に基づき Backend API + Frontend 表示を実装" |
| コードレビュー | - |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ実行者 | - |
| マージ日 | - |
| マージ先 | virtual-voicebot-backend/docs/requirements/RD-001_product.md (F-13 補足), virtual-voicebot-frontend/docs/requirements/RD-005_frontend.md (§3.3 発着信履歴) |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| `virtual-voicebot-backend/docs/requirements/RD-001_product.md` | 修正 | F-13「UAC 発信」に履歴表示の要件（direction / calleeNumber の API 返却）を追記 |
| `virtual-voicebot-frontend/docs/requirements/RD-005_frontend.md` | 修正 | §3.3 発着信履歴の「方向」「発信者」「着信先」列の発信時仕様を追記 |

### 4.2 影響するコード

#### Backend

| モジュール | 変更種別 | 概要 |
|-----------|---------|------|
| `migrations/YYYYMMDD_add_direction_callee_number.sql` | 追加 | call_logs テーブルに `direction` / `callee_number` カラムを追加 |
| `src/shared/ports/call_log_port.rs` | 修正 | `EndedCallLog` 構造体に `direction: String` / `callee_number: Option<String>` を追加 |
| `src/interface/db/postgres.rs` | 修正 | sync payload に `direction` / `calleeNumber` を追加 |
| `src/protocol/session/coordinator.rs` | 修正 | 通話終了時に `direction`（"inbound" / "outbound"）と `callee_number` を `EndedCallLog` へ設定 |

#### Frontend

| モジュール | 変更種別 | 概要 |
|-----------|---------|------|
| `lib/db/sync.ts` | 修正 | `StoredCallLog` 型に `direction: string` / `calleeNumber: string \| null` を追加。`normalizeCallLog()` で Backend payload から `direction` / `calleeNumber` をパース |
| `lib/api.ts` | 修正 | `mapStoredCallToCall()` に `direction` / `calleeNumber` のマッピングを追加 |
| `lib/db/queries.ts` | 修正 | `deriveDirection()` を `callLog.direction` 優先に変更（`callLog.direction === "outbound"` を先に評価し、fallback として `actionCode === "AR"` を維持） |
| `lib/types.ts` | 修正 | `Call` インターフェースに `direction: "inbound" \| "outbound"` / `calleeNumber: string \| null` を追加 |
| `lib/mock-data.ts` | 修正 | `CallRecord` に対応するフィールドが不足している場合は追加（toRecord 経由で設定される想定） |
| `lib/call-display.ts` | 修正 | `resolveDisplayStatus()` に発信通話分岐を追加（§5.10 参照） |
| `components/call-history-content.tsx` | 修正 | `toDirection()` を `call.direction` ベースに変更。`toRecord()` に `calleeNumber` の設定を追加 |
| `components/calls/calls-table.tsx` | 修正 | 発信時の 着信応答列・IVR詳細列 を "-" 表示。着信先列を direction に応じて calleeNumber に切り替え |
| `components/calls/call-detail-drawer.tsx` | 修正 | 発信時の各フィールドを正しく表示（着信応答・IVR詳細の "-" 表示等） |

---

## 5. 差分仕様（What / How）

### 5.1 バックエンド: DB スキーマ追加

```sql
-- migrations/YYYYMMDD_add_direction_callee_number_to_call_logs.sql

ALTER TABLE call_logs
  ADD COLUMN direction TEXT NOT NULL DEFAULT 'inbound'
    CHECK (direction IN ('inbound', 'outbound')),
  ADD COLUMN callee_number TEXT NULL;
```

> **注**: 既存レコードは `DEFAULT 'inbound'` で補完する（#272 実装以前のレコードはすべて着信のため問題なし）。

### 5.2 バックエンド: EndedCallLog 構造体追加フィールド

```rust
// src/shared/ports/call_log_port.rs

pub struct EndedCallLog {
    // ...既存フィールド...
    pub direction: String,           // "inbound" | "outbound" （追加）
    pub callee_number: Option<String>, // 発信先番号。着信時は None（追加）
}
```

### 5.3 バックエンド: direction / callee_number の設定ルール

| セッション種別 | direction | callee_number |
|--------------|-----------|---------------|
| 着信（UAS, `outbound_mode = false`） | `"inbound"` | `None` |
| 発信（B2BUA, `outbound_mode = true`）| `"outbound"` | `resolve_number(to_user)` 解決後の番号（実際に発呼した番号）|

`SessionCoordinator` の通話終了処理（BYE 受信 / タイムアウト等）で `outbound_mode` を参照して設定する。

> **OQ-5 解決済み**: `callee_number` はダイヤルプラン解決後（`resolve_number()` で得られた実際の発呼番号）を採用する。生の SIP To ユーザー部ではなく、実際に `sip:{number}@{outbound_domain}` として発呼した番号を記録する（[handlers/mod.rs:124](../../virtual-voicebot-backend/src/protocol/session/handlers/mod.rs), [config/mod.rs:560](../../virtual-voicebot-backend/src/shared/config/mod.rs)）。

### 5.4 バックエンド: API レスポンス追加フィールド

```rust
// src/interface/db/postgres.rs（sync payload）

json!({
    // ...既存フィールド...
    "direction": call_log.direction,           // 追加
    "calleeNumber": call_log.callee_number,    // 追加
})
```

### 5.5 フロントエンド: StoredCallLog 型 + normalizeCallLog() の変更

```typescript
// lib/db/sync.ts

// StoredCallLog 型への追加
export interface StoredCallLog {
  // ...既存フィールド...
  direction: string          // 追加（"inbound" | "outbound"）
  calleeNumber: string | null  // 追加
}

// normalizeCallLog() への追加（既存の他フィールドと同形式）
function normalizeCallLog(entityId: string, payload: unknown, nowIso: string): StoredCallLog {
  const input = isRecord(payload) ? payload : {}
  return {
    // ...既存フィールド...
    direction: asString(input, ["direction"], "inbound") ?? "inbound",  // 追加
    calleeNumber: asString(input, ["calleeNumber", "callee_number"], null), // 追加
  }
}
```

### 5.6 フロントエンド: mapStoredCallToCall() の変更

```typescript
// lib/api.ts

function mapStoredCallToCall(callLog: StoredCallLog): Call {
  return {
    // ...既存フィールド...
    direction: (callLog.direction as Call["direction"]) ?? "inbound",  // 追加
    calleeNumber: callLog.calleeNumber ?? null,                         // 追加
  }
}
```

### 5.7 フロントエンド: deriveDirection() の変更

```typescript
// lib/db/queries.ts

// Before
export function deriveDirection(callLog: StoredCallLog): CallDirection {
  if (callLog.status === "error" || callLog.endReason === "rejected") return "missed"
  if (callLog.actionCode === "AR") return "outbound"
  return "inbound"
}

// After: StoredCallLog.direction を優先参照し、actionCode === "AR" は後方互換 fallback として残す
export function deriveDirection(callLog: StoredCallLog): CallDirection {
  if (callLog.status === "error" || callLog.endReason === "rejected") return "missed"
  if (callLog.direction === "outbound" || callLog.actionCode === "AR") return "outbound"
  return "inbound"
}
```

### 5.8 フロントエンド: Call インターフェース拡張

```typescript
// lib/types.ts

export type CallDirection = "inbound" | "outbound"  // 追加

export interface Call {
  // ...既存フィールド...
  direction: CallDirection       // 追加（バックエンドから取得）
  calleeNumber: string | null    // 追加（バックエンドから取得）
}
```

### 5.9 フロントエンド: toDirection() の変更

```typescript
// components/call-history-content.tsx

// Before
function toDirection(call: Call): CallRecord["direction"] {
  if (call.status === "error" || call.endReason === "rejected") return "missed"
  if (call.actionCode === "AR") return "outbound"
  return "inbound"
}

// After: direction フィールドを優先使用し、不在判定は現行ロジックを維持
function toDirection(call: Call): CallRecord["direction"] {
  if (call.status === "error" || call.endReason === "rejected") return "missed"
  return call.direction  // バックエンドが設定した "inbound" | "outbound" をそのまま使用
}
```

### 5.10 フロントエンド: resolveDisplayStatus() の発信通話分岐追加

発信通話（`direction === "outbound"`）は既存の actionCode ベース分岐の前に評価する。

> **背景**: Backend の `answered_at` は発信通話で現在 `None` 固定（[coordinator.rs:821](../../virtual-voicebot-backend/src/protocol/session/coordinator.rs)）のため、`answeredAt` を判定に使うと常に「応答なし」になる。発信の応答判定は B-leg 応答時刻 `transferAnsweredAt` で行う。

| 優先度 | 条件 | displayStatus |
|--------|------|---------------|
| 0 | `status ∈ {"ringing", "in_call"}` | 通話中 |
| **1 (追加)** | **`direction === "outbound"` AND `transferAnsweredAt !== null`** | **通話終了** |
| **2 (追加)** | **`direction === "outbound"` AND `transferAnsweredAt === null`** | **応答なし** |
| 3（旧1） | `callDisposition === "denied"` | 常時着信拒否 |
| 4（旧2） | `endReason === "cancelled"` AND `answeredAt === null` | 不在着信 |
| …（旧3〜10） | （STEER-270 §5.2 と同じ） | （変更なし） |

> **OQ-1 解決済み**: 発信時の未応答は新規 `"応答なし"` を使用する。着信側の「不在着信」（相手が出なかった）と発信側の「応答なし」（かけた先が出なかった）は意味が異なるため、混用しない。

> **OQ-3 解決済み**: 発信通話の `actionCode` は発信専用コードではなく、ルーティングルール評価結果（主に `"VR"` / `"IV"` 等）が設定される。Backend では `"AR"` を発信識別コードとしてセットしていないため、フロントエンドの `toDirection()` で `actionCode === "AR"` による発信判定は実質的に機能していない。発信の判定は必ずバックエンドが返す `direction` フィールドを使用すること（[executor.rs](../../virtual-voicebot-backend/src/service/routing/executor.rs), [coordinator.rs:755](../../virtual-voicebot-backend/src/protocol/session/coordinator.rs)）。

```typescript
// lib/call-display.ts（変更後）

export type DisplayStatus =
  | "通話中"
  | "常時着信拒否"
  | "不在着信"
  | "IVR離脱"
  | "留守電"
  | "通話終了"
  | "応答なし"    // 追加（発信時に相手が応答しなかった場合）

export function resolveDisplayStatus(call: Call): DisplayStatus {
  // 0. 通話中
  if (call.status === "ringing" || call.status === "in_call") return "通話中"

  // 1-2. 発信通話（direction フィールドで判定。actionCode には依存しない）
  // answeredAt は発信時 None 固定のため transferAnsweredAt（B-leg 応答時刻）を使用
  if (call.direction === "outbound") {
    return call.transferAnsweredAt !== null ? "通話終了" : "応答なし"
  }

  // 3-10. 着信通話（STEER-270 §5.2 と同じ、変更なし）
  if (call.callDisposition === "denied") return "常時着信拒否"
  // ...（省略、以降は STEER-270 の実装と同じ）
}
```

### 5.11 フロントエンド: displayStatusClass の追加エントリ

| displayStatus | スタイル |
|---------------|---------|
| 応答なし | `bg-orange-500/15 text-orange-600 dark:text-orange-300` |

### 5.12 フロントエンド: calls-table.tsx の発信時表示

| 列 | 着信時 | 発信時 |
|----|--------|--------|
| 方向 | 着信バッジ（着信, PhoneIncoming, primary 色） | 発信バッジ（発信, PhoneOutgoing, emerald 色）※既実装 |
| 発信者 | callerNumber / fromName（外部発信者） | callerNumber（登録 SIP ユーザー番号）/ fromName は `"-"` |
| 着信先 | `call.to`（システム番号） | `call.calleeNumber`（発信先番号）|
| 着信応答 | callDisposition ラベル | `"-"`（発信通話では callDisposition は常に "allowed" だが表示は意味を持たないため非表示）|
| IVR詳細 | actionCode に応じた詳細 | `"-"` |

> **OQ-2 解決済み**: 発信時の `fromName` は `"-"` を表示する。現行 `fromName` は `callerCategory` ラベル由来であり、着信専用（外部発信者の種別）のため、発信通話には適用できない（[call-history-content.tsx:198](../../virtual-voicebot-frontend/components/call-history-content.tsx)）。

> **OQ-4 解決済み**: 発信通話の `callDisposition` は通常ケースで `"allowed"` が設定される（BZ/RJ/AN=denied, NR=no_answer, それ以外=allowed）。発信では着信拒否・不在の概念が当てはまらないため、UI 表示は `"-"` とする（[coordinator.rs:860](../../virtual-voicebot-backend/src/protocol/session/coordinator.rs)）。

```typescript
// components/calls/calls-table.tsx（発信時の着信応答・IVR詳細）

// 着信応答列
{call.direction === "outbound" ? "-" : dispositionLabel(call.callDisposition)}

// IVR詳細列
{call.direction === "outbound" ? "-" : ivrDetailLabel(call.actionCode)}

// 着信先列
{call.direction === "outbound" ? call.calleeNumber ?? "-" : call.to}
```

### 5.13 フロントエンド: call-detail-drawer.tsx の発信時表示

call-detail-drawer でも同様に、発信通話は以下の列/項目を `-` 表示にする：
- 着信応答（callDisposition）
- IVR詳細（actionCode 詳細パネル）

着信先は `call.calleeNumber` を使用する。

### 5.14 フロントエンド: CSV エクスポート

```typescript
// components/call-history-content.tsx（handleExportCSV）

// 方向列
call.direction === "outbound" ? "発信" : call.direction === "missed" ? "不在" : "着信"
```

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #275 | STEER-275 | 起票 |
| Issue #272 / STEER-272 | STEER-275 | 前提（発着信バックエンド実装） |
| STEER-275 | RD-001 F-13 | 要件補足（direction / calleeNumber の API 返却） |
| STEER-275 | RD-005 §3.3 | 要件修正（発信時表示列定義追加） |
| STEER-275 | `migrations/...` | DB スキーマ追加 |
| STEER-275 | `src/shared/ports/call_log_port.rs` | EndedCallLog 拡張 |
| STEER-275 | `src/interface/db/postgres.rs` | API レスポンス拡張 |
| STEER-275 | `lib/db/sync.ts` (StoredCallLog + normalizeCallLog) | 取り込み層: direction / calleeNumber パース追加 |
| STEER-275 | `lib/api.ts` (mapStoredCallToCall) | マッピング層: direction / calleeNumber 通過 |
| STEER-275 | `lib/db/queries.ts` (deriveDirection) | フィルタ層: direction フィールド優先判定 |
| STEER-275 | `lib/types.ts` | Call インターフェース拡張 |
| STEER-275 | `lib/call-display.ts` | resolveDisplayStatus 発信分岐追加（transferAnsweredAt 判定）|
| STEER-275 | `components/call-history-content.tsx` | toDirection / toRecord 修正 |
| STEER-275 | `components/calls/calls-table.tsx` | 発信時表示修正 |
| STEER-275 | `components/calls/call-detail-drawer.tsx` | 発信時表示修正 |
| STEER-270 | STEER-275 | displayStatus 定義の拡張（発信分岐追加） |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [ ] `direction` カラムの DEFAULT 'inbound' による既存レコードへの影響が問題ないか
- [ ] 発信通話で `callee_number` をどのタイミング・どのコードパスで設定するか確認済みか（§5.3 参照）
- [x] OQ-1（「応答なし」vs「不在着信」）→ **解決済み**（新規「応答なし」を追加）
- [x] OQ-2（発信時の fromName 表示）→ **解決済み**（`"-"` を表示）
- [x] OQ-3（発信時の actionCode: 何が設定されるか）→ **解決済み**（ルーティング評価結果が入る。direction フィールドで判定する）
- [x] OQ-4（発信通話の callDisposition）→ **解決済み**（通常ケース "allowed"、UI 表示は "-"）
- [x] OQ-5（callee_number の定義）→ **解決済み**（resolve_number() 解決後の番号）
- [ ] `toDirection()` 変更後、既存の 着信通話の方向表示がデグレしないか（status=error/rejected の missed 判定は引き続き有効か）
- [ ] 発信フィルタ（filter-bar）が `direction === "outbound"` に正しく反応するか確認済みか
- [ ] `DisplayStatus "応答なし"` の displayStatusClass（orange）が STEER-270 のカラーパレットと整合しているか

### 7.2 マージ前チェック（Approved → Merged）

- [ ] DB マイグレーションが適用済み
- [ ] バックエンドの実装完了（direction / callee_number の設定 + API 返却）
- [ ] フロントエンドの実装完了（全 AC が満たされていること）
- [ ] `resolveDisplayStatus` の単体テスト（発信分岐）が PASS（テスト観点: 下記参照）
- [ ] 既存の着信通話表示がデグレしていない（回帰テスト）
- [ ] コードレビュー（CodeRabbit）が完了している
- [ ] RD-001 F-13 および RD-005 §3.3 への反映準備ができている

#### テスト観点（最低限カバーすること）

| # | テスト対象 | 入力 | 期待結果 |
|---|-----------|------|---------|
| T-01 | `resolveDisplayStatus` | direction=outbound, transferAnsweredAt=非null | 通話終了 |
| T-02 | `resolveDisplayStatus` | direction=outbound, transferAnsweredAt=null | 応答なし |
| T-03 | `resolveDisplayStatus` | direction=inbound, actionCode=IV, transferStatus=no_transfer | IVR離脱（デグレ確認）|
| T-04 | `deriveDirection` | StoredCallLog.direction="outbound" | outbound |
| T-05 | `deriveDirection` | StoredCallLog.direction="inbound", status=error | missed |
| T-06 | `normalizeCallLog` | payload に direction="outbound", calleeNumber="09012345678" | StoredCallLog に正しく格納 |
| T-07 | `normalizeCallLog` | payload に direction/calleeNumber なし | direction="inbound", calleeNumber=null（fallback）|
| T-08 | filter-bar 発信フィルタ | direction=outbound のレコードのみ存在 | 1件ヒット（「発信」選択時）|
| T-09 | calls-table 着信先列 | direction=outbound, calleeNumber="09099998888" | "09099998888" 表示 |
| T-10 | calls-table 着信先列 | direction=outbound, calleeNumber=null | "-" 表示 |
| T-11 | calls-table 着信応答列 | direction=outbound | "-" 表示 |
| T-12 | CSV エクスポート | direction=outbound | 方向列に "発信" |

---

## 8. Open Questions

| # | 質問 | 解決方針 | 合意状況 |
|---|------|---------|---------|
| OQ-1 | 発信時に相手が応答しなかった場合の displayStatus は新規「応答なし」か、既存「不在着信」を転用するか？ | 新規「応答なし」を追加する。着信の「不在着信」（相手に出てもらえなかった）と発信の「応答なし」（かけた先が出なかった）は意味が異なるため混用しない。 | **解決済み** |
| OQ-2 | 発信時の 発信者列（fromName）は何を表示するか？ | `fromName` は `"-"` を表示する。現行 `fromName` は着信専用の `callerCategory` ラベル由来のため発信には不適。主表示は `callerNumber`（登録 SIP ユーザー番号）のみ。 | **解決済み** |
| OQ-3 | 発信通話（STEER-272 の B2BUA 転送）では `actionCode` に何が設定されるか？ | 発信専用コードは設定されない。ルーティングルール評価結果（主に VR / IV 等）が入る。`"AR"` は Backend が実際にセットしていないため、フロントエンドの発信判定は必ず `direction` フィールドを使用すること。 | **解決済み** |
| OQ-4 | 発信通話の `callDisposition` には何が設定されるか？ | 通常ケースでは `"allowed"`（BZ/RJ/AN=denied, NR=no_answer, それ以外=allowed）。UI 表示は発信では意味を持たないため `"-"` を表示する。 | **解決済み** |
| OQ-5 | `callee_number` は SIP To URI のユーザー部（生の電話番号）か、ダイヤルプランで解決後の番号か？ | `resolve_number()` 解決後の番号（実際に `sip:{number}@{outbound_domain}` として発呼した番号）を採用する。 | **解決済み** |

---

## 9. 備考

- **STEER-270 との関係**: STEER-270 の §9 備考に「`AR`（発信コール）の displayStatus は本 Issue では未定義（Fallback → 通話終了）。発信通話の履歴改善は別 Issue で検討予定」とあり、本 STEER-275 がその別 Issue に該当する。
- **STEER-272 との関係**: STEER-272 でバックエンドの発着信同時サポートが実装されたが、`direction` / `callee_number` の API 露出は本ステアリングのスコープ。RD-001 F-13 の補足として追記する。
- **actionCode の扱い（OQ-3 より）**: バックエンドは発信通話に `"AR"` をセットしていない。`actionCode` には着信と同じルーティング評価結果（VR / IV 等）が入るため、`resolveDisplayStatus` の発信通話分岐は `direction === "outbound"` チェックを actionCode ベース分岐より前に評価することで、誤分類（例: outbound + IV が「IVR離脱」になる）を防ぐ。
- **callerNumber の意味（発信時）**: 着信時は外部発信者番号だが、発信時は登録 SIP ユーザー番号（REGISTER_USER）が入る。表示上は同じフィールドを流用するが、列ラベル「発信者」はどちらの場合も文脈的に成立する（着信時: 誰が発信してきたか、発信時: 誰が発信したか）。
- **fromName = "-" の根拠（OQ-2 より）**: 現行 `fromName` は `callerCategory`（"未登録" / "登録済み" 等）ラベルであり、発信通話では `callerCategory` を発信者種別として使用しないため `"-"` が適切。
- **503 拒否の known limitation**: `is_outbound_intent = true` だが config 不備（domain 未設定 / dial plan 未解決）で 503 を返した通話は、`outbound_mode = false` のまま ingest されるため、履歴上は `direction = "inbound"` として記録される（[handlers/mod.rs:124-139](../../virtual-voicebot-backend/src/protocol/session/handlers/mod.rs)）。この通話は実態と異なる「着信」として表示されるが、config 不備による異常系であり、本スコープでは許容する。
- **answeredAt の現状（重大指摘2 対応）**: Backend の `answered_at` は発信通話で `None` 固定（[coordinator.rs:821](../../virtual-voicebot-backend/src/protocol/session/coordinator.rs)）。発信通話の「通話終了 / 応答なし」判定には B-leg 応答時刻 `transferAnsweredAt` を使用する（§5.10 参照）。将来 `answered_at` の保存が追加された場合は判定軸の見直しが必要。

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-03-02 | 初版作成 | Claude Code (claude-sonnet-4-6) |
| 2026-03-02 | OQ-1〜5 全解決済みに更新（応答なし追加確定・fromName="-"確定・actionCode非依存設計確定・callee_number=resolve後番号確定） | Claude Code (claude-sonnet-4-6) |
| 2026-03-02 | レビュー指摘対応（重大3件・中3件）: sync.ts/api.ts/queries.ts を影響範囲追加、answeredAt→transferAnsweredAt 変更、503 known limitation 追記、AC-6 参照節修正、相対パス修正、テスト観点12件追加 | Claude Code (claude-sonnet-4-6) |
| 2026-03-02 | レビュー指摘対応（中1件・軽1件）: §5.7→§5.10 参照修正（AC-6・§4.2）、章番号欠番（5.10 空き）を解消（5.11〜5.15 → 5.10〜5.14 繰り上げ） | Claude Code (claude-sonnet-4-6) |
| 2026-03-02 | レビュー OK（新規指摘なし）。残リスク: 503 異常系 known limitation・answered_at 将来見直し（§9 明示済み） | @MasanoriSuda (via Claude Code) |
