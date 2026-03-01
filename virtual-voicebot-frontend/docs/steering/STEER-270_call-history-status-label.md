# STEER-270: 発着信履歴ステータス・通話時間の表示改善

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-270 |
| タイトル | 発着信履歴ステータス・通話時間の表示改善 |
| ステータス | Approved |
| 関連Issue | #270 |
| 優先度 | P1 |
| 作成日 | 2026-03-01 |

---

## 2. ストーリー（Why）

### 2.1 背景

- 発着信履歴のステータス列が「完了」「不在」の2値しかなく、通話がどのような理由で終了したか分からない
- 例：IVRで選択前に切断した場合も「完了」と表示されており、詳細画面を開くまで状況が把握できない
- 着信拒否と不在着信、通話終了、IVR離脱、留守電が区別できないため、運用上の問題を判別しにくい

### 2.2 目的

- ステータス列を終了済み通話で5種類（常時着信拒否/不在着信/IVR離脱/留守電/通話終了）、進行中通話で「通話中」を加えた計6種類に分類し、一覧で通話結果が判別できるようにする
- 通話時間（durationSec）の表示を、着信拒否・不在着信（通常着信）の場合は明示的に 0秒 と表示する

### 2.3 ユーザーストーリー

```
As a 管理者
I want to 発着信履歴の一覧でステータスを見ただけに通話結果の概要がわかるようにしたい
So that 詳細画面を開かずに着信拒否・不在・IVR離脱・留守電・通話終了を区別できる

受入条件:
- [ ] AC-1: callDisposition = "denied" の通話は「常時着信拒否」と表示される
- [ ] AC-2: endReason = "cancelled" かつ answeredAt = null の場合「不在着信」と表示される（callDisposition = "denied" は AC-1 で先に捕捉されるため実質着信許可時のみ到達）
- [ ] AC-3: actionCode = "IV" かつ transferStatus ∈ {"no_transfer", "none"} の場合「IVR離脱」と表示される（AC-2 の条件に該当しない場合。優先評価順は §5.2 参照）
- [ ] AC-4: actionCode = "IV" かつ transferStatus ∈ {"trying", "failed"} の場合「不在着信」と表示される（AC-2 の条件に該当しない場合。優先評価順は §5.2 参照）
- [ ] AC-5: actionCode = "IV" かつ transferStatus = "answered" の場合「通話終了」と表示される（AC-2 の条件に該当しない場合。優先評価順は §5.2 参照）
- [ ] AC-6: actionCode = "VM" の場合「留守電」と表示される
- [ ] AC-7: actionCode = "VB" の場合「通話終了」と表示される
- [ ] AC-8: actionCode = "VR" かつ answeredAt = null の場合「不在着信」と表示される
- [ ] AC-9: actionCode = "VR" かつ answeredAt != null の場合「通話終了」と表示される
- [ ] AC-10: callDisposition = "denied" の場合、通話時間は「00:00」と表示される
- [ ] AC-11: actionCode = "VR" かつ answeredAt = null の場合、通話時間は「00:00」と表示される
- [ ] AC-12: AC-10/AC-11 以外の場合、通話時間は従来通り（durationSec）を表示する
- [ ] AC-13: Drawer（通話詳細）のステータスバッジにも同じ displayStatus が反映される
- [ ] AC-14: CSV出力のステータス列も displayStatus のラベルを使用する
- [ ] AC-15: status ∈ {"ringing", "in_call"} の通話は「通話中」と表示される（優先度0、終了済み分類より先に評価）
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-03-01 |
| 起票理由 | 発着信履歴のステータス列が「完了」「不在」のみで詳細が分かりにくい |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Code (claude-sonnet-4-6) |
| 作成日 | 2026-03-01 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "Issue #270 の発着信履歴ステータス・通話時間表示改善のステアリングを作成" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| 1 | @MasanoriSuda | 2026-03-01 | OK | 指摘4ラウンド（中2件→軽4件→中2件→OK）を経て承認。残リスクは備考レベル |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | @MasanoriSuda |
| 承認日 | 2026-03-01 |
| 承認コメント | lgtm |

### 3.5 実装（該当する場合）

| 項目 | 値 |
|------|-----|
| 実装者 | Codex |
| 実装日 | - |
| 指示者 | @MasanoriSuda |
| 指示内容 | "STEER-270 に基づき displayStatus / displayDurationSec を実装" |
| コードレビュー | - |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ実行者 | - |
| マージ日 | - |
| マージ先 | RD-005（§3.3 発着信履歴）|

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| `virtual-voicebot-frontend/docs/requirements/RD-005_frontend.md` | 修正 | §3.3 ステータスカラム定義を displayStatus（終了済み5種類 + 進行中1種類）に更新 |

### 4.2 影響するコード

| モジュール | 変更種別 | 概要 |
|-----------|---------|------|
| `lib/mock-data.ts` | 修正 | `CallRecord` に `displayStatus: DisplayStatus` / `displayDurationSec: number` を追加 |
| `lib/call-display.ts` | 追加 | `resolveDisplayStatus()` / `resolveDisplayDuration()` / `displayStatusClass()` を新規定義（calls-table・call-detail-drawer で共有） |
| `components/call-history-content.tsx` | 修正 | `toRecord()` 内で上記関数を呼び出して `displayStatus` / `displayDurationSec` を設定 |
| `components/calls/calls-table.tsx` | 修正 | `statusLabel` → `call.displayStatus`、`formatDuration(call.durationSec)` → `formatDuration(call.displayDurationSec)` に置き換え |
| `components/calls/call-detail-drawer.tsx` | 修正 | `statusToLabel(call.status)` → `call.displayStatus` に置き換え |

---

## 5. 差分仕様（What / How）

### 5.1 DisplayStatus 型定義

```typescript
// lib/call-display.ts（新規）

export type DisplayStatus =
  | "通話中"        // status = "ringing" / "in_call"（進行中）
  | "常時着信拒否"
  | "不在着信"
  | "IVR離脱"
  | "留守電"
  | "通話終了"
```

### 5.2 resolveDisplayStatus 導出ロジック

評価優先順位（上から順に評価し、最初にマッチした値を返す）:

| 優先度 | 条件 | displayStatus |
|--------|------|---------------|
| 0 | `status ∈ {"ringing", "in_call"}` | 通話中 |
| 1 | `callDisposition === "denied"` | 常時着信拒否 |
| 2 | `endReason === "cancelled"` AND `answeredAt === null` | 不在着信 |
| 3 | `actionCode === "IV"` AND `transferStatus === "answered"` | 通話終了 |
| 4 | `actionCode === "IV"` AND `transferStatus ∈ {"trying", "failed"}` | 不在着信 |
| 5 | `actionCode === "IV"` AND `transferStatus ∈ {"no_transfer", "none"}` | IVR離脱 |
| 6 | `actionCode === "VM"` | 留守電 |
| 7 | `actionCode === "VB"` | 通話終了 |
| 8 | `actionCode === "VR"` AND `answeredAt === null` | 不在着信 |
| 9 | `actionCode === "VR"` AND `answeredAt !== null` | 通話終了 |
| 10 | （上記以外 fallback） | 通話終了 |

**前提条件の解釈**:
- 優先度0で `in_call` / `ringing` を先に捕捉することで、終了済みでない通話が誤って分類されるのを防ぐ
- 優先度2「通話成立前キャンセル」は `actionCode = "IV"` の場合にも適用する（IVR 開始前に SIP セッションが確立していないため、IVR離脱より不在着信を優先）
- IVR離脱（優先度5）は SIP セッション確立後（`answeredAt != null`）に発火するケース。ただし本関数は `answeredAt` を追加条件としない（優先度2で先に弾かれるため）
- 優先度1以降の条件は `callDisposition = "denied"` でない（= "allowed" または "no_answer"）場合にのみ到達する

```typescript
// lib/call-display.ts

import type { Call } from "@/lib/types"

export function resolveDisplayStatus(call: Call): DisplayStatus {
  // 0. 通話中（進行中レコードは分類しない）
  if (call.status === "ringing" || call.status === "in_call") return "通話中"

  // 1. 着信拒否
  if (call.callDisposition === "denied") return "常時着信拒否"

  // 2. 通話成立前（相手がCANCEL）
  if (call.endReason === "cancelled" && call.answeredAt === null) return "不在着信"

  // 3-5. IVR
  if (call.actionCode === "IV") {
    if (call.transferStatus === "answered") return "通話終了"
    if (call.transferStatus === "trying" || call.transferStatus === "failed") return "不在着信"
    // no_transfer / none: IVR 選択前に切断
    return "IVR離脱"
  }

  // 6. 留守電
  if (call.actionCode === "VM") return "留守電"

  // 7. ボイスボット
  if (call.actionCode === "VB") return "通話終了"

  // 8-9. 通常着信（VR）
  if (call.actionCode === "VR") {
    return call.answeredAt === null ? "不在着信" : "通話終了"
  }

  // 10. fallback
  return "通話終了"
}
```

### 5.3 resolveDisplayDuration 導出ロジック

| 条件 | displayDurationSec |
|------|-------------------|
| `callDisposition === "denied"` | 0 |
| `actionCode === "VR"` AND `answeredAt === null` | 0 |
| それ以外 | `durationSec ?? 0`（従来通り） |

> **注**: IVR・VM・VB ケースで displayStatus が「不在着信」「IVR離脱」となった場合でも、durationSec は従来通りとする（本 Issue スコープ外、別 Issue で検討予定）。

```typescript
// lib/call-display.ts

export function resolveDisplayDuration(call: Call): number {
  if (call.callDisposition === "denied") return 0
  if (call.actionCode === "VR" && call.answeredAt === null) return 0
  return call.durationSec ?? 0
}
```

### 5.4 CallRecord 型の拡張

```typescript
// lib/mock-data.ts への追加

export type CallRecord = {
  // ...既存フィールド...
  displayStatus: DisplayStatus    // 追加
  displayDurationSec: number      // 追加
}
```

### 5.5 toRecord() 内での計算

```typescript
// components/call-history-content.tsx

import { resolveDisplayStatus, resolveDisplayDuration } from "@/lib/call-display"

function toRecord(call: Call): CallRecord {
  return {
    // ...既存フィールド...
    displayStatus: resolveDisplayStatus(call),       // 追加
    displayDurationSec: resolveDisplayDuration(call), // 追加
  }
}
```

### 5.6 calls-table.tsx の変更点

| 変更箇所 | Before | After |
|---------|--------|-------|
| ステータスバッジラベル | `statusLabel(call.status)` | `call.displayStatus` |
| ステータスバッジ色 | `statusClass(call.status)` | `displayStatusClass(call.displayStatus)` |
| 通話時間 | `formatDuration(call.durationSec)` | `formatDuration(call.displayDurationSec)` |

**displayStatusClass マッピング**:

| displayStatus | スタイル |
|---------------|---------|
| 通話中 | `bg-sky-500/15 text-sky-600 dark:text-sky-300`（既存 in_call と同色） |
| 常時着信拒否 | `bg-neutral-500/15 text-neutral-600 dark:text-neutral-300` |
| 不在着信 | `bg-rose-500/15 text-rose-600 dark:text-rose-300` |
| IVR離脱 | `bg-amber-500/15 text-amber-600 dark:text-amber-300` |
| 留守電 | `bg-blue-500/15 text-blue-600 dark:text-blue-300` |
| 通話終了 | `bg-emerald-500/15 text-emerald-600 dark:text-emerald-300` |

### 5.7 call-detail-drawer.tsx の変更点

| 変更箇所 | Before | After |
|---------|--------|-------|
| バッジラベル | `statusToLabel(call.status)` | `call.displayStatus` |
| バッジ色 | `statusToClass(call.status)` | `displayStatusClass(call.displayStatus)` |
| 通話時間 | `formatDuration(call.durationSec)` | `formatDuration(call.displayDurationSec)` |

### 5.8 CSV出力の変更点

`call-history-content.tsx` 内の `handleExportCSV` でステータス列を `call.displayStatus` に変更する。

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #270 | STEER-270 | 起票 |
| STEER-270 | RD-005 §3.3 | 要件修正（ステータスカラム定義更新） |
| STEER-270 | `lib/call-display.ts` | 新規ユーティリティ |
| STEER-270 | `lib/mock-data.ts` (CallRecord) | 型追加 |
| STEER-270 | `call-history-content.tsx` | toRecord 修正 |
| STEER-270 | `calls/calls-table.tsx` | ステータス表示修正 |
| STEER-270 | `calls/call-detail-drawer.tsx` | バッジ表示修正 |
| STEER-270 | `call-history-content.tsx`（CSV出力） | ステータス列を displayStatus に変更 |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [ ] DisplayStatus 全6種類のラベル名が要件と一致しているか（通話中/常時着信拒否/不在着信/IVR離脱/留守電/通話終了）
- [ ] 優先度2「通話成立前キャンセル」と優先度5「IVR離脱」の評価順序が意図通りか
- [ ] durationSec の 0秒上書き対象が要件と一致しているか（denied + VR/answeredAt=null のみ）
- [ ] callDisposition = "denied" となる actionCode はバックエンド側と整合しているか（OQ-2: denied であれば actionCode 問わず「常時着信拒否」のため実装上は問題なし）

### 7.2 マージ前チェック（Approved → Merged）

- [ ] 実装が完了している
- [ ] `resolveDisplayStatus` / `resolveDisplayDuration` の単体テスト（各分岐）がPASS
- [ ] コードレビューを受けている
- [ ] RD-005 §3.3 への反映準備ができている

---

## 8. Open Questions

| # | 質問 | 暫定方針 | 合意状況 |
|---|------|---------|---------|
| OQ-1 | IVR で `transferStatus = "failed"` の場合は「不在着信」か「IVR離脱」か？ | 不在着信（転送が試行された＝選択後の失敗） | **解決済み** |
| OQ-2 | `callDisposition = "denied"` となる actionCode は RJ/BZ/NR/AN のみか？他にあるか？ | denied の場合は actionCode によらず「常時着信拒否」を使用する | **解決済み** |
| OQ-3 | IVR・VM・VB で displayStatus が「不在着信」「IVR離脱」となった場合の通話時間も 0秒にすべきか？ | 本 Issue スコープ外、別 Issue で検討予定 | **解決済み（スコープ外）** |
| OQ-4 | `endReason = "timeout"` かつ `answeredAt = null`（例: 鳴らし続けてタイムアウト）の場合の displayStatus は何か？ | VR の場合は answeredAt = null → 不在着信。その他は fallback → 通話終了。NR（RING_FOREVER）は denied 扱いのため常時着信拒否 | **解決済み** |
| OQ-5 | displayStatusClass のカラーリング（特に IVR離脱・留守電）は現行デザイントークンに合っているか？ | amber（IVR離脱）/ blue（留守電）を採用（§5.6 参照） | **解決済み** |

---

## 9. 備考

- `lib/call-display.ts` を純粋関数として独立させることで、単体テストの作成と他コンポーネントからの再利用が容易になる
- 現行の `statusLabel` / `statusClass` / `statusToLabel` / `statusToClass` 関数は、`in_call` 用表示（通話中バッジ）を除き、本変更後は不要となる。ただしリアルタイム通話中ステータスの表示は本スコープ外のため、削除は別 Issue で行うこと
- `AR`（発信コール）の displayStatus は本 Issue では未定義（Fallback → 通話終了）。発信通話の履歴改善は別 Issue で検討予定

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-03-01 | 初版作成 | Claude Code (claude-sonnet-4-6) |
| 2026-03-01 | OQ-1〜OQ-5 を暫定方針にて全解決済みに更新 | Claude Code (claude-sonnet-4-6) |
| 2026-03-01 | レビュー指摘対応: 自己参照import削除、DisplayStatus に「通話中」追加（優先度0）、AC-2表記をロジックと統一 | Claude Code (claude-sonnet-4-6) |
| 2026-03-01 | 再レビュー指摘対応: §2.2 を6種類に修正、AC-3〜AC-5 に優先評価順注記追加、§4.2 に displayStatusClass 定義場所明記、§6 に CSV出力行追加 | Claude Code (claude-sonnet-4-6) |
| 2026-03-01 | 三度目レビュー指摘対応: AC-15（通話中）追加、分類数の表現を「終了済み5種類 + 進行中1種類」に統一（§4.1・§7.1） | Claude Code (claude-sonnet-4-6) |
