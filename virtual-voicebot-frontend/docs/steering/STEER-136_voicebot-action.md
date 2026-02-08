# STEER-136: ボイスボットアクション追加

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-136 |
| タイトル | ボイスボットアクション追加（AllowVB + IVR 終端 VB + シナリオストア） |
| ステータス | Approved |
| 関連Issue | #136 |
| 優先度 | P1 |
| 作成日 | 2026-02-08 |

---

## 2. ストーリー（Why）

### 2.1 背景

- STEER-132 で着信アクション（Allow: VR/IV/VM、Deny: BZ/NR/AN）を定義済み
- STEER-134 で IVR フロー管理（終端: VR/VM/AN/IV、fallback: VR/VM/AN）を定義済み
- しかし「ボイスボット（AI 応対）」がアクション選択肢にない
- ボイスボットは複数シナリオを切り替える可能性が高く、`scenarioId` での参照が必要
- `ActionCode` の Canonical 型（types.ts）には既に `"VB"` が定義されているが、call-actions / IVR で未使用

### 2.2 目的

- 着信許可時のアクションに `VB`（ボイスボット）を追加し、シナリオを選択可能にする
- IVR 終端 / fallback にも VB を追加し、IVR 完了後にボイスボットへ遷移できるようにする
- ボイスボットシナリオを `scenarios.json` で管理（読み取り専用 API）し、ドロップダウンから選択可能にする

### 2.3 ユーザーストーリー

```
As a 管理者
I want to 着信許可時および IVR 終端でボイスボットを選択したい
So that AI ボットが自動応対し、用件に応じた対応ができる

受入条件:
- [ ] AC-1: call-actions のアクション選択に「ボイスボット(VB)」が追加される
- [ ] AC-2: VB 選択時にシナリオドロップダウンが表示され、シナリオを選択できる
- [ ] AC-3: VB 選択時に開始前アナウンス（welcomeAnnouncementId）を設定できる
- [ ] AC-4: VB 選択時に録音設定（recordingEnabled / includeAnnouncement）が表示される（PoC: recordingEnabled は true 固定）
- [ ] AC-5: IVR フロー定義の終端アクションに VB が追加され、同一パラメータ（scenarioId + welcomeAnnouncementId + recording）で設定できる
- [ ] AC-6: IVR フロー定義の fallback アクションに VB が追加される
- [ ] AC-7: scenarios.json にシード済みシナリオが格納される
- [ ] AC-8: GET /api/scenarios でシナリオ一覧を取得できる
- [ ] AC-9: 参照中のシナリオが削除（JSON 手編集で除去）された場合、参照元で「（削除済みシナリオ）」と表示される
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-08 |
| 起票理由 | 着信許可時・IVR 終了後にボイスボット応対を選択可能にする |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Code (claude-opus-4-6) |
| 作成日 | 2026-02-08 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "着信許可時、IVR突入時にボイスボットを追加する。scenarioId 必須、voicevoxStyleId 追加、管理UI は別 Issue" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| 1 | Codex | 2026-02-08 | 条件付きOK | 重大1件（正規化関数のVB対応漏れ）、中2件（scenario.id型矛盾、scenarioId未選択表現曖昧）、軽1件（inactiveシナリオ表示ポリシー不足）→全4件反映済み |
| 2 | Codex | 2026-02-08 | 条件付きOK | 重大1件（lib/db/ 層の正規化/厳密パース関数に VB ケース未記載 → 保存→復元でデータ欠落リスク）→反映済み |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | @MasanoriSuda |
| 承認日 | 2026-02-08 |
| 承認コメント | Codex レビュー 2 回済み、DB 層正規化含め全反映 OK |

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
| マージ先 | - |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| STEER-134_ivr-flow-ui.md | 修正 | IvrTerminalAction / IvrFallbackAction に VB 追加（差分参照） |

### 4.2 影響するコード

| モジュール | 変更種別 | 概要 |
|-----------|---------|------|
| lib/call-actions.ts | 修正 | AllowVB 型追加、AllowActionCode に "VB" 追加、createActionConfig 拡張、isAllowActionCode / getAnnouncementId / withAnnouncementId / cloneActionConfig の VB 対応 |
| lib/db/call-actions.ts | 修正 | `normalizeActionConfig` / `parseActionConfigStrict` に VB ケース追加（保存→復元時のデータ欠落防止） |
| lib/ivr-flows.ts | 修正 | IvrTerminalAction / IvrFallbackAction に VB 追加、terminalActionLabel / toIvrDestinationFromCallAction / validateIvrFlows（VB 終端の scenarioId 検証）の VB 対応 |
| lib/db/ivr-flows.ts | 修正 | `normalizeTerminalAction` / `normalizeFallbackAction` に VB ケース追加（保存→復元時のデータ欠落防止） |
| lib/scenarios.ts | 追加 | VoicebotScenario 型定義 + ユーティリティ関数 |
| lib/db/scenarios.ts | 追加 | scenarios.json 読み取り関数 |
| app/api/scenarios/route.ts | 追加 | GET /api/scenarios API |
| storage/db/scenarios.json | 追加 | シナリオ JSON ストア（シードデータ付き） |
| components/call-actions-content.tsx | 修正 | VB アクション UI（シナリオ選択 + 録音設定 + アナウンス選択） |
| components/ivr-content.tsx | 修正 | IVR 終端 / fallback に VB 選択肢追加 |

---

## 5. 差分仕様（What / How）

### 5.1 VoicebotScenario データモデル

```typescript
interface VoicebotScenario {
  id: string                    // 一意文字列（slug または UUID。例: "scenario-reception", UUID も可）
  name: string                  // 表示名（例: "受付ボット", "FAQ ボット"）
  description: string | null
  isActive: boolean
  voicevoxStyleId: number       // VoiceVox スタイル ID（必須 — ボットごとに声を変える）
  systemPrompt: string | null   // 将来用（PoC: UI なし、nullable で保持のみ）
  createdAt: string             // ISO 8601
  updatedAt: string
}

interface ScenariosDatabase {
  scenarios: VoicebotScenario[]
}
```

**設計判断:**
- `voicevoxStyleId` は必須。PoC でもボットごとに声を変えるユースケースが想定される
- `systemPrompt` は nullable で型に含めるが、PoC では UI を作らない（JSON 手編集で設定可能）
- シナリオ管理 UI（CRUD）は別 Issue で切り出す

### 5.2 AllowVB 型定義（call-actions.ts への差分）

```typescript
// --- 追加 ---
export type AllowVB = {
  actionCode: "VB"
  scenarioId: string                    // ボットシナリオ ID（未選択時は ""。保存時に "" はバリデーションエラー）
  welcomeAnnouncementId: string | null  // 開始前アナウンス（null = なし）
  recordingEnabled: boolean             // 録音（PoC: true 固定）
  includeAnnouncement: boolean          // 録音にアナウンスを含むか
}

// --- 変更 ---
export type AllowActionCode = "VR" | "IV" | "VM" | "VB"  // VB 追加

export type ActionConfig = AllowVR | AllowIV | AllowVM | AllowVB | DenyBZ | DenyNR | DenyAN  // AllowVB 追加
```

**設計判断:**
- `scenarioId` の型は `string`（null 不可）。未選択状態は `""` で表現し、保存時にバリデーションエラーとする。null は使用しない（AllowIV の `ivrFlowId: string | null` とは異なる判断 — VB は scenarioId が常に必須のため、null を許容する意味がない）
- `announceEnabled` フラグは不要 → `welcomeAnnouncementId` の有無で判断（null = アナウンスなし）
- `recordingEnabled` はフィールドとして持つが PoC は UI で true 固定表示
- `includeAnnouncement` は AllowIV の既存パターンを流用
- VR（転送）とは別物として扱い、VB 独自に録音・アナウンスを保持

#### 5.2.2 call-actions.ts 正規化関数の VB 対応

以下の既存関数に VB ケースを追加する:

| 関数 | VB 対応内容 |
|------|-----------|
| `isAllowActionCode` | `ALLOW_CODES` に `"VB"` を追加 → 自動的に対応 |
| `getAnnouncementId` | VB の場合は `config.welcomeAnnouncementId` を返す |
| `withAnnouncementId` | VB の場合は `{ ...config, welcomeAnnouncementId: announcementId }` を返す |
| `cloneActionConfig` | JSON.parse/stringify のため追加不要（自動対応） |
| `actionCodeLabel` | `"VB"` → `"ボイスボット"` を追加 |
| `createActionConfig` | §5.5.3 参照 |

#### 5.2.3 lib/db/call-actions.ts DB 層の VB 対応（重要）

> **リスク**: このファイルには独立した正規化/厳密パース関数があり、VB ケースがないと保存した VB 設定が読み込み時に VR へフォールバックしてデータ欠落する。

| 関数 | 現状 | VB 対応内容 |
|------|------|-----------|
| `normalizeActionConfig` (L59-110) | allow: VR/IV/VM のみ switch | `case "VB":` を追加。`scenarioId: asTrimmedString(raw.scenarioId) ?? ""`, `welcomeAnnouncementId: normalizeNullableString(raw.welcomeAnnouncementId)`, `recordingEnabled: typeof raw.recordingEnabled === "boolean" ? raw.recordingEnabled : true`, `includeAnnouncement: typeof raw.includeAnnouncement === "boolean" ? raw.includeAnnouncement : false` |
| `parseActionConfigStrict` (L250-305) | allow: VR/IV/VM のみ switch | `case "VB":` を追加。`scenarioId` は `requireString`（空文字不可）、他フィールドは `normalizeActionConfig` と同様のパース |

### 5.3 IvrTerminalAction / IvrFallbackAction への VB 追加（ivr-flows.ts への差分）

```typescript
// --- IvrTerminalAction: VB 追加 ---
export type IvrTerminalAction =
  | { actionCode: "VR" }
  | { actionCode: "VM"; announcementId: string | null }
  | { actionCode: "AN"; announcementId: string | null }
  | { actionCode: "IV"; ivrFlowId: string }
  | { actionCode: "VB"; scenarioId: string; welcomeAnnouncementId: string | null; recordingEnabled: boolean; includeAnnouncement: boolean }

// --- IvrFallbackAction: VB 追加（無限ループリスクなし）---
export type IvrFallbackAction =
  | { actionCode: "VR" }
  | { actionCode: "VM"; announcementId: string | null }
  | { actionCode: "AN"; announcementId: string | null }
  | { actionCode: "VB"; scenarioId: string; welcomeAnnouncementId: string | null; recordingEnabled: boolean; includeAnnouncement: boolean }
```

**設計判断:**
- IVR 側の VB は AllowVB と同一パラメータセット。IVR から起動するボットも scenarioId で切り替え可能
- VB は IV と異なり無限ループリスクがないため、fallback でも使用可

#### 5.3.2 ivr-flows.ts 正規化/検証関数の VB 対応

| 関数 | VB 対応内容 |
|------|-----------|
| `terminalActionLabel` | §5.6.3 参照 |
| `toIvrDestinationFromCallAction` | §5.6.4 参照 |
| `validateIvrFlows` | VB 終端/fallback の `scenarioId` が空文字の場合エラー追加。削除済みシナリオは警告 |
| `cloneIvrFlow` | JSON.parse/stringify のため追加不要（自動対応） |
| `referencedIvrIds` | VB は IVR 参照しないため変更不要 |
| `detectCycles` / `getMaxDepth` | VB は IVR ネストに関与しないため変更不要 |

#### 5.3.3 lib/db/ivr-flows.ts DB 層の VB 対応（重要）

> **リスク**: このファイルには独立した正規化関数があり、VB ケースがないと IVR フロー保存時に VB 終端/fallback が VR へフォールバックしてデータ欠落する。

| 関数 | 現状 | VB 対応内容 |
|------|------|-----------|
| `normalizeTerminalAction` (L91-122) | VM/AN/IV/VR のみ switch、default → VR | `case "VB":` を追加。`scenarioId: asTrimmedString(raw.scenarioId) ?? ""`, `welcomeAnnouncementId: asNullableString(raw.welcomeAnnouncementId)`, `recordingEnabled: typeof raw.recordingEnabled === "boolean" ? raw.recordingEnabled : true`, `includeAnnouncement: typeof raw.includeAnnouncement === "boolean" ? raw.includeAnnouncement : false`。`scenarioId` が空の場合は VR にフォールバック（IV と同じパターン） |
| `normalizeFallbackAction` (L124-145) | VM/AN/VR のみ switch、default → VR | `case "VB":` を追加。正規化ロジックは `normalizeTerminalAction` と同一 |

### 5.4 scenarios.json ストア + API

#### 5.4.1 JSON ストア

**パス**: `storage/db/scenarios.json`

```json
{
  "scenarios": [
    {
      "id": "scenario-reception",
      "name": "受付ボット",
      "description": "一般的な受付対応を行うボイスボット",
      "isActive": true,
      "voicevoxStyleId": 3,
      "systemPrompt": null,
      "createdAt": "2026-02-08T00:00:00Z",
      "updatedAt": "2026-02-08T00:00:00Z"
    },
    {
      "id": "scenario-faq",
      "name": "FAQ ボット",
      "description": "よくある質問に回答するボイスボット",
      "isActive": true,
      "voicevoxStyleId": 1,
      "systemPrompt": null,
      "createdAt": "2026-02-08T00:00:00Z",
      "updatedAt": "2026-02-08T00:00:00Z"
    }
  ]
}
```

#### 5.4.2 API

**GET /api/scenarios**

| 項目 | 値 |
|------|-----|
| メソッド | GET |
| レスポンス | `{ ok: true, scenarios: VoicebotScenario[] }` |
| エラー | `{ ok: false, error: string }` |

> PUT は本 STEER のスコープ外（管理 UI なし）。シナリオの追加・編集は JSON 手編集で行う。

#### 5.4.3 lib/scenarios.ts

```typescript
export interface VoicebotScenario {
  id: string
  name: string
  description: string | null
  isActive: boolean
  voicevoxStyleId: number
  systemPrompt: string | null
  createdAt: string
  updatedAt: string
}

export interface ScenariosDatabase {
  scenarios: VoicebotScenario[]
}
```

#### 5.4.4 lib/db/scenarios.ts

- `readScenariosDatabase(): Promise<ScenariosDatabase>` — scenarios.json を読み取り
- ファイルが存在しない場合は `{ scenarios: [] }` を返す

### 5.5 call-actions UI 変更

#### 5.5.1 ALLOW_ACTION_CODES 定数

```typescript
// 変更前
const ALLOW_ACTION_CODES: CallActionCode[] = ["VR", "IV", "VM"]

// 変更後
const ALLOW_ACTION_CODES: CallActionCode[] = ["VR", "IV", "VM", "VB"]
```

#### 5.5.2 actionCodeLabel 拡張

```typescript
case "VB":
  return "ボイスボット"
```

#### 5.5.3 createActionConfig 拡張

```typescript
case "VB":
  return {
    actionCode: "VB",
    scenarioId: "",           // 未選択状態
    welcomeAnnouncementId: null,
    recordingEnabled: true,   // PoC: true 固定
    includeAnnouncement: false,
  }
```

#### 5.5.4 VB 設定パネル（renderActionEditor 内に追加）

VB 選択時に以下の設定パネルを表示:

| 設定項目 | UI 要素 | 備考 |
|---------|--------|------|
| シナリオ | ドロップダウン（scenarios 一覧） | 必須。未選択 = "" で保存時バリデーション |
| 開始前アナウンス | アナウンスドロップダウン（既存 renderAnnouncementSelect 再利用） | null = なし |
| 録音あり | Switch（PoC: true 固定、disabled） | フィールドは保持 |
| includeAnnouncement | Switch | 録音にアナウンスを含むか |

#### 5.5.5 シナリオドロップダウン（renderScenarioSelect）

新規ヘルパー関数。パターンは既存の `renderIvrFlowSelect` と同様:

- isActive なシナリオを一覧表示
- isActive = false のシナリオも、**現在選択中であれば** 表示して `[無効]` バッジを付与（既存の IVR / アナウンスと同一パターン）
- 選択中のシナリオが存在しない場合「（削除済みシナリオ）」表示
- シナリオ未登録時「（シナリオ未登録）」表示

#### 5.5.6 buildActionSummary 拡張

```typescript
if (actionConfig.actionCode === "VB") {
  return `${head}${actionConfig.scenarioId ? ` (scenario: ${actionConfig.scenarioId})` : ""}`
}
```

#### 5.5.7 データフェッチ拡張

`useEffect` 内の `Promise.all` に `fetch("/api/scenarios")` を追加。パターンは既存の announcements / ivr-flows と同様。

#### 5.5.8 保存時バリデーション

VB アクション使用時、`scenarioId` が空文字の場合はエラー:
```
「シナリオを選択してください」
```

参照中のシナリオが存在しない場合は警告表示 + 保存は許可（STEER-132/134 の削除済みリソース統一方針に準拠）。

### 5.6 IVR UI 変更（ivr-content.tsx）

#### 5.6.1 終端アクション選択肢追加

DTMF ルートの遷移先選択に VB を追加:

| 選択肢 | 追加パラメータ |
|--------|-------------|
| 転送(VR) | なし |
| 留守電(VM) | announcementId |
| アナウンス→切断(AN) | announcementId |
| IVR(IV) | ivrFlowId |
| **ボイスボット(VB)** | **scenarioId + welcomeAnnouncementId + recordingEnabled + includeAnnouncement** |

VB 選択時は call-actions と同様の設定パネルを表示。

#### 5.6.2 fallback アクション選択肢追加

fallback にも VB を追加（IV は引き続き除外）:

| 選択肢 | 追加パラメータ |
|--------|-------------|
| 転送(VR) | なし |
| 留守電(VM) | announcementId |
| アナウンス→切断(AN) | announcementId |
| **ボイスボット(VB)** | **scenarioId + welcomeAnnouncementId + recordingEnabled + includeAnnouncement** |

#### 5.6.3 terminalActionLabel 拡張

```typescript
case "VB": {
  // scenarioId → scenarios 一覧から名前を引く
  return `ボイスボット(${scenarioLabel})`
}
```

#### 5.6.4 toIvrDestinationFromCallAction 拡張

```typescript
case "VB":
  return config.scenarioId
    ? {
        actionCode: "VB",
        scenarioId: config.scenarioId,
        welcomeAnnouncementId: config.welcomeAnnouncementId ?? null,
        recordingEnabled: config.recordingEnabled,
        includeAnnouncement: config.includeAnnouncement,
      }
    : null
```

### 5.7 バリデーション

#### 5.7.1 call-actions 側

| チェック | 条件 | アクション |
|---------|------|----------|
| scenarioId 必須 | `actionCode === "VB" && scenarioId === ""` | エラー: 「シナリオを選択してください」 |
| 削除済みシナリオ参照 | scenarioId が scenarios 一覧に存在しない | 警告表示 + 保存許可 |

#### 5.7.2 IVR 側

| チェック | 条件 | アクション |
|---------|------|----------|
| scenarioId 必須 | VB 終端/fallback で `scenarioId === ""` | エラー: 「シナリオを選択してください」 |
| 削除済みシナリオ参照 | scenarioId が scenarios 一覧に存在しない | 警告表示 + 保存許可 |

### 5.8 PoC 制約

| 項目 | 制約 | 理由 |
|------|------|------|
| recordingEnabled | UI で true 固定（disabled Switch） | PoC では常時録音 |
| systemPrompt | 型に含むが UI なし | 管理 UI は別 Issue |
| シナリオ管理 UI | 本 STEER では未実装 | 別 Issue で 2ペイン UI を追加予定 |
| scenarios.json | PUT API なし（読み取り専用） | JSON 手編集 / シードデータで運用 |
| VoiceVox styleId | scenarios.json に含むが UI では表示のみ | 変更は JSON 手編集 |

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #136 | STEER-136 | 起票 |
| STEER-132 | STEER-136 | 依存（AllowActionCode 拡張） |
| STEER-134 | STEER-136 | 依存（IvrTerminalAction / IvrFallbackAction 拡張） |
| STEER-136 §5.2 | lib/call-actions.ts | AllowVB 型追加 |
| STEER-136 §5.3 | lib/ivr-flows.ts | IvrTerminalAction / IvrFallbackAction に VB 追加 |
| STEER-136 §5.4 | lib/scenarios.ts, lib/db/scenarios.ts, app/api/scenarios/route.ts | シナリオストア + API |
| STEER-136 §5.5 | components/call-actions-content.tsx | VB アクション UI |
| STEER-136 §5.6 | components/ivr-content.tsx | IVR 終端 / fallback VB UI |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [ ] AllowVB のプロパティが過不足ないか
- [ ] IVR 側 VB と call-actions 側 VB の型が一致しているか
- [ ] シナリオ JSON ストアのスキーマが妥当か
- [ ] 既存 STEER-132 / STEER-134 との整合性があるか
- [ ] 削除済みシナリオの表示ポリシーが統一されているか
- [ ] PoC 制約が明確で、将来の拡張パスが見えるか

### 7.2 マージ前チェック（Approved → Merged）

- [ ] 実装が完了している（該当する場合）
- [ ] コードレビューを受けている（該当する場合）
- [ ] 関連テストが PASS（該当する場合）
- [ ] 本体仕様書への反映準備ができている

---

## 8. 備考

- 本 STEER はフロントエンド UI + JSON 読み取りのみ。バックエンドとの VoiceBot 連携は後続 Issue
- `ActionCode` の Canonical 型（types.ts:12）に既に `"VB"` が存在するため、types.ts の変更は不要
- シナリオ管理 UI（CRUD + 2ペイン構成）は次 Issue で /ivr と同様に追加予定
- AllowVB の `recordingEnabled` は PoC true 固定だが、UI 上は Switch を disabled 表示して存在を見せる
- `voicevoxStyleId` はシナリオ JSON に含まれるが、call-actions / IVR 側からは参照不要（バックエンドがシナリオ取得時に読む想定）
- `welcomeAnnouncementId` は既存の `renderAnnouncementSelect` を再利用して実装

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-08 | 初版作成 | Claude Code (claude-opus-4-6) |
| 2026-02-08 | Codex レビュー1回目反映: 正規化関数VB対応明記（重大）、scenario.id型統一（中）、scenarioId未選択表現明確化（中）、inactiveシナリオ表示ポリシー追加（軽） | Claude Code (claude-opus-4-6) |
| 2026-02-08 | Codex レビュー2回目反映: lib/db/call-actions.ts・lib/db/ivr-flows.ts の正規化/厳密パース関数に VB ケース明記（重大） | Claude Code (claude-opus-4-6) |
| 2026-02-08 | 承認（Approved） | Claude Code (claude-opus-4-6) |
