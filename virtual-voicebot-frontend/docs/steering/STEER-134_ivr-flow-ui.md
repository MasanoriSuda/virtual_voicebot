# STEER-134: IVR フロー管理 UI

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-134 |
| タイトル | IVR フロー管理 UI（DTMF メニュー定義 + JSON 永続化） |
| ステータス | Approved |
| 関連Issue | #134 |
| 優先度 | P1 |
| 作成日 | 2026-02-08 |

---

## 2. ストーリー（Why）

### 2.1 背景

- STEER-132 で着信アクションに `AllowIV`（IVR 誘導）を定義したが、IVR フローの管理 UI が存在しない
- 既存の `/ivr` ページはモックデータ（`LegacyIvrFolder/Flow/Node`）のみで、永続化・API がない
- call-actions ルールから `ivrFlowId` で IVR フローを参照するため、IVR フローの CRUD + 永続化が必要
- PoC として最小限の IVR メニュー定義（アナウンス再生 → DTMF 入力 → ルーティング）を実装する

### 2.2 目的

- `/ivr` ページを IVR フロー管理 UI として書き換え、JSON 永続化 + API を提供する
- IVR フロー定義を作成・編集・削除でき、call-actions から参照可能にする
- 例外系（無入力・無効入力）も破綻しない定義にする

### 2.3 ユーザーストーリー（該当する場合）

```
As a 管理者
I want to IVR フロー（DTMF メニュー）を定義・管理したい
So that 着信時に発信者を適切な部門・アクションへ振り分けられる

受入条件:
- [ ] AC-1: /ivr ページが IVR フロー管理 UI として表示される（既存モック置換）
- [ ] AC-2: IVR フローの CRUD（作成/編集/削除/複製）ができる
- [ ] AC-3: フロー定義: メニュー案内アナウンス選択（#129 のアナウンス一覧から）
- [ ] AC-4: フロー定義: DTMF キー（0-9, #, *）ごとにルート（遷移先）を設定できる
- [ ] AC-5: フロー定義: 各ルートの遷移先として VR（転送）/ VM（留守電）/ AN（アナウンス→切断）/ IV（サブIVR）を選択できる
- [ ] AC-6: フロー定義: timeout（DTMF 待ち秒数）と maxRetries（リトライ上限）を設定できる
- [ ] AC-7: フロー定義: 無効入力時 / タイムアウト時のアナウンスを個別に設定できる（未設定ならガイダンスなしで prompt 再生）
- [ ] AC-8: フロー定義: リトライ超過時の fallback 遷移先（VR / VM / AN のみ、IV 不可）を設定できる
- [ ] AC-9: IVR ネストは最大3層まで。循環参照はバリデーションで拒否
- [ ] AC-10: IVR フローが JSON ファイル（storage/db/ivr-flows.json）に永続化される
- [ ] AC-11: call-actions 側で IVR フロー一覧をドロップダウンから選択できる（AllowIV.ivrFlowId）
- [ ] AC-12: 参照中の IVR フローが削除された場合、参照元で「（削除済み IVR）」と表示
- [ ] AC-13: IVR フローの有効/無効切替ができる
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-08 |
| 起票理由 | STEER-132 の AllowIV から参照する IVR フロー管理が必要 |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Code (claude-opus-4-6) |
| 作成日 | 2026-02-08 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "IVR フロータブの仕様を壁打ちして、ステアリングを起こす" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| 1 | Codex | 2026-02-08 | OK | 実装可能。バリデーション責務・Legacy型削除タイミング・削除済みIVR保存ポリシーの3点を確認→反映済み。サーバ側構造検証なしは将来課題として認識 |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | @MasanoriSuda |
| 承認日 | 2026-02-08 |
| 承認コメント | PoC として OK。Codex レビュー済み |

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
| docs/requirements/RD-004_call-routing.md | 修正 | IVR フロー管理要件の追加 |

### 4.2 影響するコード

| モジュール | 変更種別 | 概要 |
|-----------|---------|------|
| components/ivr-content.tsx | 書換 | モック → IVR フロー管理 UI（2ペイン構成） |
| lib/ivr-flows.ts | 追加 | IVR フロー型定義 + ユーティリティ関数 |
| lib/db/ivr-flows.ts | 追加 | IVR フロー JSON ストア読み書き |
| app/api/ivr-flows/route.ts | 追加 | GET + PUT API |
| components/call-actions-content.tsx | 修正 | IVR フロー選択をドロップダウン化（手入力 → 一覧選択） |
| lib/types.ts | 修正 | `LegacyIvrFolder/Flow/Node` 型を廃止 |

---

## 5. 差分仕様（What / How）

### 5.1 IVR フロー処理フロー

```
着信 → call-actions ルール評価 → AllowIV マッチ
  ↓
ivrFlowId で IvrFlowDefinition 取得
  ↓
announcementId のアナウンス再生（メニュー案内）
  ↓
DTMF 待ち (timeoutSec)
  ├─ 有効キー → routes[key].destination を実行
  │    ├─ VR → 転送（PoC: 宛先固定）
  │    ├─ VM → 留守電
  │    ├─ AN → アナウンス再生→切断
  │    └─ IV → ivrFlowId で再帰（depth チェック、≤3）
  ├─ 無効キー → invalidInputAnnouncementId 再生（null なら省略）
  │    → リトライカウント++
  │    └─ maxRetries 超過 → fallbackAction 実行
  └─ タイムアウト → timeoutAnnouncementId 再生（null なら省略）
       → リトライカウント++
       └─ maxRetries 超過 → fallbackAction 実行

※ リトライ時はメニュー案内（announcementId）を再度再生
```

### 5.2 データモデル

#### 5.2.1 DtmfKey

```typescript
type DtmfKey = "0"|"1"|"2"|"3"|"4"|"5"|"6"|"7"|"8"|"9"|"#"|"*"
```

PoC 制約: `maxDigits = 1` 固定（1キー押下で即遷移）

#### 5.2.2 IvrTerminalAction（IVR 終端アクション）

着信時 `ActionConfig` のサブセット。IVR は通話応答後のため、BZ（BUSY）/ NR（RING_FOREVER）は除外。

```typescript
type IvrTerminalAction =
  | { actionCode: "VR" }                                    // 転送（PoC: 宛先固定）
  | { actionCode: "VM"; announcementId: string | null }     // 留守電
  | { actionCode: "AN"; announcementId: string | null }     // アナウンス再生→切断
  | { actionCode: "IV"; ivrFlowId: string }                 // サブIVRへ遷移（depth≤3）
  | { actionCode: "VB"; scenarioId: string; welcomeAnnouncementId: string | null; recordingEnabled: boolean; includeAnnouncement: boolean }  // ボイスボット（STEER-136 で追加）
```

> **設計根拠**: BZ / NR は「着信応答前」のアクション。IVR は通話応答後に動作するため、
> BUSY 応答やリング継続はセマンティクス的に無意味。IVR 終端は上記4種 + VB に限定する。
> VB は STEER-136 で追加。詳細は [STEER-136](STEER-136_voicebot-action.md) §5.3 を参照。

#### 5.2.3 IvrFallbackAction（リトライ超過時アクション）

`IvrTerminalAction` から IV を除外。リトライ超過 → 別 IVR → またリトライ超過 の連鎖を防止。

```typescript
type IvrFallbackAction =
  | { actionCode: "VR" }                                    // 転送
  | { actionCode: "VM"; announcementId: string | null }     // 留守電
  | { actionCode: "AN"; announcementId: string | null }     // アナウンス再生→切断
  | { actionCode: "VB"; scenarioId: string; welcomeAnnouncementId: string | null; recordingEnabled: boolean; includeAnnouncement: boolean }  // ボイスボット（STEER-136 で追加）
```

#### 5.2.4 IvrRoute（DTMF ルート）

```typescript
interface IvrRoute {
  dtmfKey: DtmfKey                     // DTMF キー
  label: string                        // 表示名（例: "営業"）
  destination: IvrTerminalAction       // 遷移先
}
```

#### 5.2.5 IvrFlowDefinition（IVR フロー定義）

```typescript
interface IvrFlowDefinition {
  id: string                                   // UUID
  name: string                                 // フロー名
  description: string | null
  isActive: boolean                            // 有効/無効

  // メニュー設定
  announcementId: string | null                // メニュー案内音声（アナウンスタブから選択）
  timeoutSec: number                           // DTMF 待ち時間（デフォルト: 10）
  maxRetries: number                           // リトライ上限（デフォルト: 2）

  // リトライ時音声（null = ガイダンスなしで prompt 再生）
  invalidInputAnnouncementId: string | null    // 無効入力時アナウンス
  timeoutAnnouncementId: string | null         // タイムアウト時アナウンス

  // DTMF ルーティング
  routes: IvrRoute[]                           // 最大12件（DTMF キー数）

  // リトライ超過時の遷移先（IV 不可）
  fallbackAction: IvrFallbackAction

  createdAt: string                            // ISO 8601
  updatedAt: string
}
```

#### 5.2.6 JSON ストア構造

**IVR フローストア**: `storage/db/ivr-flows.json`

```typescript
interface IvrFlowsDatabase {
  flows: IvrFlowDefinition[]
}
```

初期状態:

```json
{
  "flows": []
}
```

**JSON 例（2層ネスト）**:

```json
{
  "flows": [
    {
      "id": "ivr-main",
      "name": "メインメニュー",
      "description": "受付振り分け",
      "isActive": true,
      "announcementId": "ann-welcome",
      "timeoutSec": 10,
      "maxRetries": 2,
      "invalidInputAnnouncementId": "ann-invalid",
      "timeoutAnnouncementId": null,
      "routes": [
        { "dtmfKey": "1", "label": "営業", "destination": { "actionCode": "VR" } },
        { "dtmfKey": "2", "label": "サポート", "destination": { "actionCode": "IV", "ivrFlowId": "ivr-support" } },
        { "dtmfKey": "9", "label": "留守電", "destination": { "actionCode": "VM", "announcementId": null } }
      ],
      "fallbackAction": { "actionCode": "VR" },
      "createdAt": "2026-02-08T00:00:00Z",
      "updatedAt": "2026-02-08T00:00:00Z"
    },
    {
      "id": "ivr-support",
      "name": "サポートメニュー",
      "description": "サポート部門振り分け",
      "isActive": true,
      "announcementId": "ann-support-menu",
      "timeoutSec": 10,
      "maxRetries": 2,
      "invalidInputAnnouncementId": null,
      "timeoutAnnouncementId": "ann-timeout",
      "routes": [
        { "dtmfKey": "1", "label": "技術サポート", "destination": { "actionCode": "VR" } },
        { "dtmfKey": "2", "label": "契約サポート", "destination": { "actionCode": "VR" } }
      ],
      "fallbackAction": { "actionCode": "AN", "announcementId": "ann-goodbye" },
      "createdAt": "2026-02-08T00:00:00Z",
      "updatedAt": "2026-02-08T00:00:00Z"
    }
  ]
}
```

### 5.3 IVR ネスト方式

- **ID 参照方式**: ルートの destination が `{ actionCode: "IV", ivrFlowId: "xxx" }` で別の IvrFlowDefinition を参照
- 各 IvrFlowDefinition は単一メニュー（アナウンス → DTMF 待ち → ルート）
- depth はバリデーション時に再帰カウント（**depth > 3 → エラー**）
- **循環検出**: 保存時に参照グラフを走査し、循環参照を検出 → エラー

```
Level 1: メインメニュー (1→営業VR, 2→サポートIV, 9→留守電VM)
           ↓ key=2
Level 2: サポートメニュー (1→技術VR, 2→契約VR)
```

### 5.4 IVR 終端アクション ActionCode マッピング

| ActionCode | 説明 | IVR 終端 | fallback |
|---|---|---|---|
| `VR` | 転送（PoC: 宛先固定） | 使用可 | 使用可 |
| `VM` | 留守電 | 使用可 | 使用可 |
| `AN` | アナウンス再生→切断 | 使用可 | 使用可 |
| `IV` | サブIVR へ遷移 | 使用可（depth≤3） | **不可** |
| `BZ` | BUSY | **不可**（応答済み） | **不可** |
| `NR` | RING_FOREVER | **不可**（応答済み） | **不可** |

### 5.5 API Routes

#### GET /api/ivr-flows

IVR フロー一覧を返す。

```typescript
// Response
{
  ok: boolean
  flows: IvrFlowDefinition[]
}
```

#### PUT /api/ivr-flows

IVR フローを一括保存。

```typescript
// Request body
{
  flows: IvrFlowDefinition[]
}

// Response
{ ok: boolean }
// or
{ ok: false; error: string }
```

サーバ側バリデーション:
- PoC ではサーバ側バリデーションは **最小限**（JSON パースエラーのみ拒否）
- 構造バリデーション（フロー名空チェック、dtmfKey 重複、ネスト depth、循環検出）は **フロントエンド側が SoT**（§5.7 参照）
- 将来的にサーバ側でも同等のバリデーションを追加する（Backend 連携時）

### 5.6 UI 構成

#### 5.6.1 画面レイアウト

```
┌──────────────────────────────────────────────────────┐
│  IVRフロー                            [+ 新規作成]     │
├──────────────┬───────────────────────────────────────┤
│ [フロー一覧]   │  [フロー詳細 / 編集]                     │
│              │                                       │
│ 🔍 検索...    │  フロー名: [メインメニュー        ]       │
│              │  説明:     [受付振り分け           ]       │
│ ▶ メインメニュー│  有効: [✓]                              │
│   サポートメニュー│                                       │
│   時間外メニュー │  ── メニュー設定 ──                     │
│              │  案内アナウンス: [ウェルカムメッセージ v]    │
│              │  タイムアウト: [10] 秒                    │
│              │  リトライ上限: [2] 回                     │
│              │                                       │
│              │  無効入力時アナウンス: [入力エラー    v]    │
│              │  タイムアウト時アナウンス: [(なし)    v]    │
│              │                                       │
│              │  ── DTMF ルート ──                      │
│              │  [1] 営業      → 転送(VR)    [×]       │
│              │  [2] サポート   → IVR(サポートメニュー) [×] │
│              │  [9] 留守電    → 留守電(VM)   [×]       │
│              │  [+ ルート追加]                          │
│              │                                       │
│              │  ── リトライ超過時 ──                     │
│              │  fallback: [転送(VR)           v]       │
│              │                                       │
│              │  [保存]  [キャンセル]                     │
└──────────────┴───────────────────────────────────────┘
```

**左ペイン: フロー一覧**

- IVR フロー一覧（名前 + 有効/無効バッジ）
- 検索フィルター（フロー名で部分一致）
- フロー選択 → 右ペインに詳細表示
- 右クリックまたは「…」メニュー: 編集 / 複製 / 削除
- フォルダツリーは **PoC では廃止**（フラットリスト + 検索で十分）

**右ペイン: フロー詳細/編集**

- フロー基本情報: 名前、説明、有効/無効
- メニュー設定: 案内アナウンス、タイムアウト秒数、リトライ上限
- リトライ音声: 無効入力時アナウンス、タイムアウト時アナウンス
- DTMF ルート一覧: キー + ラベル + 遷移先、追加/削除
- fallback 設定: リトライ超過時の遷移先（VR / VM / AN のみ）
- 保存 / キャンセル ボタン

#### 5.6.2 DTMF ルート追加 UI

```
┌─────────────────────────────────────────┐
│ ルート追加                                │
│ キー: [2 v]  ← 未使用キーのみ選択可        │
│ ラベル: [サポート          ]               │
│ 遷移先: [IVR v]                           │
│   IVRフロー: [サポートメニュー v]            │
│ [追加]  [キャンセル]                        │
└─────────────────────────────────────────┘
```

- キー選択: 既に routes に設定済みのキーは選択不可
- 遷移先選択: VR / VM / AN / IVR の4択
  - VM 選択時: アナウンス選択ドロップダウン表示
  - AN 選択時: アナウンス選択ドロップダウン表示
  - IVR 選択時: IVR フロー選択ドロップダウン表示（自分自身は除外）

#### 5.6.3 アナウンス選択

STEER-129 / STEER-132 と同じパターン:
- `/api/announcements` から取得した一覧をドロップダウンで表示
- 0件: 「（アナウンス未登録）」disabled
- 「なし」選択肢を先頭に配置（`announcementId = null`）
- 削除済みアナウンス: 「（削除済み）」表示

### 5.7 エラー仕様・バリデーション

| ケース | 挙動 |
|--------|------|
| フロー名が空 | エラー「フロー名を入力してください」 |
| routes が 0 件 | エラー「少なくとも1つのルートを追加してください」 |
| 同じ dtmfKey が重複 | エラー「キー X が重複しています」 |
| ネスト depth > 3 | エラー「IVR のネストは3層までです」 |
| 循環参照（A→B→A） | エラー「循環参照が検出されました: A → B → A」 |
| 参照先 IVR が存在しない | **警告表示、保存は許可**。ルート表示に「（削除済み IVR）」。保存ボタン押下時に確認ダイアログ「参照先が見つからないルートがあります。保存しますか？」 |
| ルートのラベルが空 | エラー「ラベルを入力してください」 |

> **注**: バリデーションは **フロントエンド側が SoT**。サーバ側は PoC では JSON パースエラーのみ拒否。

### 5.8 IVR フロー操作

| 操作 | 挙動 |
|------|------|
| 新規作成 | デフォルト値で作成: timeoutSec=10, maxRetries=2, routes=[], fallbackAction=VR |
| 編集 | 右ペインでインライン編集 → 保存で PUT |
| 削除 | 確認ダイアログ後に削除。参照元（call-actions ルール / 他 IVR）で「（削除済み IVR）」表示 |
| 複製 | 全フィールドコピー + 新 UUID + 名前に「(コピー)」付与 |
| 有効/無効切替 | isActive トグル。無効 IVR を call-actions から参照している場合は警告表示 |

### 5.9 call-actions 連携（STEER-132 への影響）

STEER-132 の `AllowIV` は既に以下の定義:

```typescript
type AllowIV = {
  actionCode: "IV"
  ivrFlowId: string | null
  includeAnnouncement: boolean    // ルール側に保持（壁打ちで確定）
}
```

**変更点**:
- `ivrFlowId` の UI を「テキスト入力」→「ドロップダウン選択」に変更
- `/api/ivr-flows` から取得したフロー一覧を選択肢に表示
- 0件: 「（IVR フロー未登録）」disabled
- 削除済みフロー: 「（削除済み IVR）」表示、**保存は許可**（`ivrFlowId` は保持、警告のみ）
- `includeAnnouncement` はルール側（AllowIV）に残す（IVR 定義はメニュー構造に専念）

> **削除済み IVR ポリシー（統一）**: IVR 側・call-actions 側ともに「警告表示 + 保存許可」で統一。
> STEER-132 の番号グループ削除時ポリシー（「（削除済み）」表示 + 保存時警告）と同じパターン。

### 5.10 PoC 制約

| 項目 | PoC での扱い |
|------|-------------|
| maxDigits | 1 固定（multi-digit は将来） |
| VR 転送先 | 固定（IVR 定義では宛先を指定しない） |
| VM 転送先 | 固定 |
| バックエンド連携 | 本 Issue では行わない |
| ドラッグ&ドロップ並替え | ルート一覧の並替えは不要（キー順表示） |
| フォルダツリー | PoC では廃止（フラットリスト + 検索） |
| 既存 Legacy 型 | `LegacyIvrFolder/Flow/Node` を廃止し `IvrFlowDefinition` に置換。**ivr-content.tsx 置換と同時に削除**すること（段階実装で型参照が壊れるのを防ぐ） |
| リトライ音声の実動作 | フラグ保持のみ（Backend 実装は後続） |

---

## 5.11 詳細設計追加

### DD-134-FN-01: IvrContent コンポーネント

```typescript
export function IvrContent(): JSX.Element
```

#### 状態管理

| State | 型 | 説明 |
|-------|-----|------|
| flows | IvrFlowDefinition[] | IVR フロー一覧 |
| selectedFlowId | string \| null | 選択中のフロー ID |
| announcements | StoredAnnouncement[] | アナウンス一覧（#129 から取得） |
| loading | boolean | 読み込み中フラグ |
| busy | boolean | 操作中フラグ |
| searchQuery | string | フロー検索クエリ |

#### レンダリング

- 2ペイン構成（左: フロー一覧、右: フロー詳細/編集）
- フロー未選択時: 「IVR フローを選択してください」プレースホルダ
- フロー選択時: 編集フォーム表示

### DD-134-FN-02: ivrFlowsStore（lib/db/ivr-flows.ts）

```typescript
function readIvrFlows(): Promise<IvrFlowsDatabase>
function writeIvrFlows(data: IvrFlowsDatabase): Promise<void>
```

- `storage/db/ivr-flows.json` の読み書き
- ファイル未存在時は `{ flows: [] }` を返す
- 書き込みは一時ファイル経由のアトミック書き込み（sync.ts と同パターン）

### DD-134-FN-03: validateIvrFlows（lib/ivr-flows.ts）

```typescript
function validateIvrFlows(flows: IvrFlowDefinition[]): ValidationResult
function detectCycles(flows: IvrFlowDefinition[]): string[][] | null
function getMaxDepth(flowId: string, flows: IvrFlowDefinition[]): number
```

- `validateIvrFlows`: 全フローのバリデーション（名前空チェック、ルート重複、ネスト深度、循環検出）
- `detectCycles`: 参照グラフを DFS で走査し、循環パスを返す（null = 循環なし）
- `getMaxDepth`: 指定フローからの最大ネスト深度を計算

### DD-134-FN-04: IvrFlowDefinition 型定義（lib/ivr-flows.ts）

```typescript
// 型定義（§5.2 参照）
export type DtmfKey = "0"|"1"|"2"|"3"|"4"|"5"|"6"|"7"|"8"|"9"|"#"|"*"
export type IvrTerminalAction = ...
export type IvrFallbackAction = ...
export interface IvrRoute { ... }
export interface IvrFlowDefinition { ... }
export interface IvrFlowsDatabase { ... }

// ユーティリティ
export const DTMF_KEYS: DtmfKey[] = ["1","2","3","4","5","6","7","8","9","0","#","*"]
export const DEFAULT_TIMEOUT_SEC = 10
export const DEFAULT_MAX_RETRIES = 2
export const MAX_IVR_DEPTH = 3

export function createDefaultIvrFlow(): IvrFlowDefinition
export function cloneIvrFlow(flow: IvrFlowDefinition): IvrFlowDefinition
export function terminalActionLabel(action: IvrTerminalAction, flows: IvrFlowDefinition[]): string
```

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #134 | STEER-134 | 起票 |
| STEER-132 | STEER-134 | AllowIV.ivrFlowId の参照先 |
| STEER-134 | RD-004 | 要件追加（IVR フロー管理） |
| RD-004 | DD-134-FN-01〜04 | 設計 |
| STEER-129 | STEER-134 | アナウンス選択の依存 |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [ ] 要件の記述が明確か
- [ ] 詳細設計で実装者が迷わないか
- [ ] テストケースが網羅的か
- [ ] 既存仕様との整合性があるか
- [ ] トレーサビリティが維持されているか

### 7.2 マージ前チェック（Approved → Merged）

- [ ] 実装が完了している（該当する場合）
- [ ] コードレビューを受けている（該当する場合）
- [ ] 関連テストがPASS（該当する場合）
- [ ] 本体仕様書への反映準備ができている

---

## 8. 備考

- 本 Issue はフロントエンド UI + JSON 永続化のみ。バックエンドへの IVR フロー同期は後続 Issue で対応
- IVR 終端アクション（VR/VM/AN/IV）は `ActionConfig` のサブセット。BZ/NR は IVR 終端では無効
- `includeAnnouncement` は call-actions ルール側（AllowIV）に保持。IVR 定義はメニュー構造に専念
- fallback では IV（サブIVR）を選択不可とし、無限ループリスクを排除
- 既存の `LegacyIvrFolder/Flow/Node` 型は本 Issue で廃止し、`IvrFlowDefinition` に置換
- `/api/ivr-flows` は call-actions ページからも参照される（IVR フロー選択ドロップダウン）

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-08 | 初版作成 | Claude Code (claude-opus-4-6) |
| 2026-02-08 | Codex レビュー反映: バリデーション責務明確化（フロント側 SoT）、LegacyIvr* 同時削除明記、削除済み IVR 保存ポリシー統一（警告+許可） | Claude Code (claude-opus-4-6) |
| 2026-02-08 | 承認（Approved） | Claude Code (claude-opus-4-6) |
| 2026-02-08 | STEER-136 差分: IvrTerminalAction / IvrFallbackAction に VB 追加 | Claude Code (claude-opus-4-6) |
