# STEER-132: 着信時アクション UI

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-132 |
| タイトル | 着信時アクション決定 UI（番号グループ × ルール評価） |
| ステータス | Approved |
| 関連Issue | #132（親: #127） |
| 優先度 | P1 |
| 作成日 | 2026-02-08 |

---

## 2. ストーリー（Why）

### 2.1 背景

- 現在、着信時のアクション（IVR / 転送 / 拒否 等）を発信者番号に基づいて制御する UI が存在しない
- バックエンドには IVR 時の録音（WAV）暫定実装があるが、フロントエンドからルールを設定する手段がない
- 既存の「番号グループ」「ルーティング」ページはモックデータのみで非稼働状態

### 2.2 目的

- 発信者番号を「番号グループ（Caller Group）」で分類し、グループ単位でアクション（Allow/Deny）を設定できる PoC UI を構築する
- ルールは優先順位付きで評価され、最初にマッチしたルールが適用される
- バックエンドとの結合は本 Issue では行わず、UI + JSON 永続化のみを対象とする

### 2.3 ユーザーストーリー

```
As a 管理者
I want to 発信者番号グループごとに着信時のアクションを設定したい
So that スパム拒否や VIP 優先対応など、発信者に応じた着信制御ができる

受入条件:
- [ ] AC-1: サイドバーに「着信アクション」が /calls と /groups の間に表示される
- [ ] AC-2: /call-actions ページが表示される
- [ ] AC-3: ルール作成時、番号グループを /groups タブで登録済みのグループから選択できる
- [ ] AC-4: ルールの CRUD（作成/編集/削除）ができる
- [ ] AC-5: ルールの優先順位を変更できる（上下ボタン）
- [ ] AC-6: デフォルトアクションを設定できる
- [ ] AC-7: Allow > VR（通常着信）: 録音あり/なし、事前アナウンスあり/なし + アナウンス選択
- [ ] AC-8: Allow > IV（IVR）: IVRフロー選択（IDのみ）、includeAnnouncement フラグ
- [ ] AC-9: Allow > VM（留守電）: 留守電アナウンス選択
- [ ] AC-10: Deny > BZ（BUSY）/ NR（RING_FOREVER）: 追加設定なしで選択可能
- [ ] AC-11: Deny > AN（ANNOUNCE_AND_HANGUP）: アナウンス選択
- [ ] AC-12: ルール・非通知・デフォルトの各アクションが JSON ファイル（storage/db/call-actions.json）に永続化される
- [ ] AC-13: アナウンス選択は #129 のアナウンス一覧（/api/announcements）から取得される
- [ ] AC-14: 非通知（anonymous）着信時のアクションを設定できる（デフォルトアクションとは独立）
- [ ] AC-15: 番号グループが 0 件の場合、ルール追加時に「先に番号グループタブでグループを作成してください」と案内される
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-08 |
| 起票理由 | 着信時アクション制御 UI の PoC 構築（#127 の子タスク） |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Code (claude-opus-4-6) |
| 作成日 | 2026-02-08 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "Issue #132 の着信時アクション UI 仕様を壁打ちして、ステアリングを起こす" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| 1 | Codex | 2026-02-08 | OK | 実装可能。API 簡略化・エラー仕様追記を反映済み |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | @MasanoriSuda |
| 承認日 | 2026-02-08 |
| 承認コメント | PoC として OK。バックエンド同期は後続で詰める |

### 3.5 実装（該当する場合）

| 項目 | 値 |
|------|-----|
| 実装者 | Codex |
| 実装日 | 2026-02-08 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "STEER-132 に基づき着信時アクション UI を実装" |
| コードレビュー | PoC（2026-02-08） |

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
| docs/requirements/RD-004_call-routing.md | 修正 | 着信アクション UI 要件の追加（FR-160〜） |

### 4.2 影響するコード

| モジュール | 変更種別 | 概要 |
|-----------|---------|------|
| app/(admin)/call-actions/page.tsx | 追加 | 着信アクションページ |
| components/call-actions-content.tsx | 追加 | メインコンポーネント（グループは読み取り専用） |
| components/admin-sidebar.tsx | 修正 | ナビゲーション項目追加 |
| lib/db/call-actions.ts | 追加 | call-actions JSON ストア読み書き |
| lib/db/number-groups.ts | 追加 | 番号グループ共有 JSON ストア読み書き |
| app/api/call-actions/route.ts | 追加 | GET（ルール取得）+ PUT（ルール保存） |
| app/api/number-groups/route.ts | 追加 | GET（グループ一覧取得）+ PUT（グループ保存、/groups ページ用） |
| components/number-groups-content.tsx | 修正 | モックデータ → JSON ストア永続化 |

---

## 5. 差分仕様（What / How）

### 5.1 ルール評価方式

- **非通知判定**が最優先: 発信者番号が非通知（anonymous / withheld）の場合、番号グループのマッチは行わず **非通知アクション（anonymousAction）** を適用する
- 非通知でない場合、ルールは **上から順に評価**し、最初にマッチしたものが適用される（index 0 が最高優先）
- どのルールにもマッチしない場合は **デフォルトアクション** が適用される
- 発信者番号のマッチは **完全一致**（正規化後）
  - 正規化: ハイフン `-`、空白、括弧 `()` を除去
  - PoC では E.164 変換（+81 → 0）は行わない

評価フロー:

```
着信 → 非通知? ──Yes──→ anonymousAction を適用
                  │
                  No
                  ↓
         ルール #1 マッチ? ──Yes──→ ルール #1 のアクションを適用
                  │
                  No
                  ↓
         ルール #2 マッチ? ──Yes──→ ...
                  │
                  ...
                  ↓
         defaultAction を適用
```

### 5.2 データモデル

#### 5.2.1 CallerGroup（番号グループ）— 外部参照

> **設計変更**: 番号グループの CRUD は `/groups` タブ（番号グループページ）が担当する。
> `/call-actions` はグループを **読み取り専用で参照** し、ルール作成時の選択肢として使用する。

グループデータは共有ストア `storage/db/number-groups.json` に `CallerGroup[]` として永続化される。
`/groups` ページが書き込み、`/call-actions` ページが読み取りを行う。

```typescript
// 共有型（lib/call-actions.ts に定義）
interface CallerGroup {
  id: string             // UUID
  name: string           // グループ名（例: "スパム", "VIP"）
  description: string | null
  phoneNumbers: string[] // 正規化済み電話番号の配列
  createdAt: string      // ISO 8601
  updatedAt: string
}
```

**API**: `GET /api/number-groups` でグループ一覧を取得する。
グループの作成・編集・削除は `/groups` ページ側の API（`PUT /api/number-groups`）で行う。

> **前提条件**: `/groups` ページに JSON 永続化 + API を追加する必要がある（別 Issue で対応）。

#### 5.2.2 ActionConfig（アクション設定）

```typescript
// Allow 系
type AllowVR = {
  actionCode: "VR"          // B2BUA 転送（通常着信）
  recordingEnabled: boolean  // 録音あり/なし（PoC ではフラグ保持のみ）
  announceEnabled: boolean   // 事前アナウンスあり/なし
  announcementId: string | null  // アナウンス ID（#129 から選択）
}

type AllowIV = {
  actionCode: "IV"                // IVR
  ivrFlowId: string | null        // IVRフロー ID（別ページで管理）
  includeAnnouncement: boolean    // 録音にアナウンスを含める（フラグ保持のみ）
}

type AllowVM = {
  actionCode: "VM"               // 留守電
  announcementId: string | null  // 留守電アナウンス ID
}

// Deny 系
type DenyBZ = {
  actionCode: "BZ"  // 即ビジー応答
}

type DenyNR = {
  actionCode: "NR"  // 鳴らし続ける（居留守）
}

type DenyAN = {
  actionCode: "AN"               // アナウンス再生後切断
  announcementId: string | null  // アナウンス ID
}

type ActionConfig = AllowVR | AllowIV | AllowVM | DenyBZ | DenyNR | DenyAN
```

#### 5.2.3 IncomingRule（着信ルール）

```typescript
interface IncomingRule {
  id: string                    // UUID
  name: string                  // ルール名
  callerGroupId: string         // CallerGroup.id への参照（/groups タブで管理されるグループ）
  actionType: "allow" | "deny"  // 大分類
  actionConfig: ActionConfig    // アクション詳細
  isActive: boolean             // 有効/無効
  createdAt: string
  updatedAt: string
}
```

#### 5.2.4 JSON ストア構造

**call-actions ストア**: `storage/db/call-actions.json`

> **変更**: `callerGroups` を本ストアから除去。グループは共有ストアで管理する。

```typescript
interface CallActionsDatabase {
  rules: IncomingRule[]         // 配列順 = 優先順位（index 0 が最高）
  anonymousAction: {            // 非通知着信時のアクション
    actionType: "allow" | "deny"
    actionConfig: ActionConfig
  }
  defaultAction: {
    actionType: "allow" | "deny"
    actionConfig: ActionConfig
  }
}
```

初期状態:

```json
{
  "rules": [],
  "anonymousAction": {
    "actionType": "deny",
    "actionConfig": {
      "actionCode": "BZ"
    }
  },
  "defaultAction": {
    "actionType": "allow",
    "actionConfig": {
      "actionCode": "VR",
      "recordingEnabled": false,
      "announceEnabled": false,
      "announcementId": null
    }
  }
}
```

**番号グループ共有ストア**: `storage/db/number-groups.json`

```typescript
// /groups ページと /call-actions ページで共有
interface NumberGroupsDatabase {
  callerGroups: CallerGroup[]
}
```

初期状態:

```json
{
  "callerGroups": []
}
```

### 5.3 ActionCode マッピング

| PoC アクション | ActionCode | 大分類 | 説明 |
|---|---|---|---|
| 通常着信（B2BUA転送） | `VR` | Allow | B2BUA で宛先へ転送。PoC では宛先固定 |
| IVR | `IV` | Allow | IVR フローへ誘導。アナウンス・録音は固定 |
| 留守電 | `VM` | Allow | 留守電応答。宛先は PoC で固定 |
| BUSY | `BZ` | Deny | 即ビジー応答 |
| RING_FOREVER | `NR` | Deny | 応答せず鳴らし続ける（居留守） |
| ANNOUNCE_AND_HANGUP | `AN` | Deny | アナウンス再生後切断 |

### 5.4 API Routes

#### GET /api/call-actions

ルール・非通知アクション・デフォルトアクションを返す。

```typescript
// Response
{
  ok: boolean
  rules: IncomingRule[]         // 配列順 = 優先順位
  anonymousAction: { actionType: "allow" | "deny"; actionConfig: ActionConfig }
  defaultAction: { actionType: "allow" | "deny"; actionConfig: ActionConfig }
}
```

#### PUT /api/call-actions

ルール・非通知アクション・デフォルトアクションを一括保存。

```typescript
// Request body
{
  rules: IncomingRule[]
  anonymousAction: { actionType: "allow" | "deny"; actionConfig: ActionConfig }
  defaultAction: { actionType: "allow" | "deny"; actionConfig: ActionConfig }
}

// Response
{ ok: boolean }
// or
{ ok: false; error: string }
```

サーバ側バリデーション:
- `rules` 配列の順序がそのまま優先順位として保存される

#### GET /api/number-groups（読み取り専用・依存 API）

> **注**: このエンドポイントは `/groups` ページの永続化と合わせて実装される（別 Issue）。
> `/call-actions` はこのエンドポイントからグループ一覧を取得し、ルール作成時の選択肢として使用する。

```typescript
// Response
{
  ok: boolean
  callerGroups: CallerGroup[]
}
```

### 5.5 UI 構成

#### 5.5.1 サイドバー変更

`admin-sidebar.tsx` の `primaryNavItems` に追加:

```typescript
// /calls と /groups の間に挿入
{ href: "/call-actions", icon: PhoneIncoming, label: "Call Actions", labelJa: "着信アクション" }
```

#### 5.5.2 画面レイアウト

```
┌──────────────────────────────────────────────────────┐
│  着信アクション                                        │
├─────────────────┬────────────────────────────────────┤
│ [番号グループ]    │  [ルール一覧]                        │
│  (読み取り専用)   │                                    │
│                 │  #1 スパム → Deny > BUSY           │
│ ▶ スパム (3)     │  #2 VIP → Allow > 通常着信 [↑][↓]  │
│ ▶ VIP (5)       │  #3 取引先 → Allow > IVR [↑][↓]   │
│ ▶ 取引先 (12)    │  ─────────────────────             │
│                 │  非通知: Deny > BUSY               │
│ ※ グループの     │  デフォルト: Allow > 通常着信         │
│   編集は番号グ   │                                    │
│   ループタブへ    │  [+ ルール追加]                      │
├─────────────────┴────────────────────────────────────┤
│ [ルール詳細 / 編集パネル]                               │
│ グループ: [VIP          v]  ← /groups から選択         │
│ アクション: [Allow v] > [通常着信 v]                    │
│   ☑ 録音あり  ☑ 事前アナウンスあり                       │
│   アナウンス: [挨拶メッセージ          v]                │
└──────────────────────────────────────────────────────┘
```

**左ペイン: 番号グループ一覧（読み取り専用）**

> **設計変更**: グループの CRUD は `/groups` タブで行う。`/call-actions` では参照のみ。

- `/api/number-groups` から取得したグループ一覧を表示（名称 + 登録番号数）
- グループ選択 → 右ペイン下部にグループ内の番号一覧を **読み取り専用** で表示
- グループ追加・編集・削除ボタンは **表示しない**
- フッターに「グループの編集は[番号グループ]タブへ」のリンクを表示
- グループが 0 件の場合:「番号グループタブでグループを作成してください」と案内

**右ペイン上部: ルール一覧**

- 優先順位順に表示（#1 が最高優先）
- 各ルール: 優先度番号、グループ名、アクションサマリー、有効/無効トグル
- 優先順位変更: 上下ボタン（↑↓）
- ルールクリック → 下部に詳細編集パネル表示
- **非通知アクション**: ルール一覧の末尾（デフォルトの直前）に固定表示（削除不可、編集可能）
- **デフォルトアクション**: 常に最末尾に表示（削除不可、編集可能）
- 非通知/デフォルト をクリック → 詳細編集パネルにアクション設定のみ表示（グループ選択は不要）
- ルール追加ボタン

**右ペイン下部: 詳細編集パネル**

ルール選択時:

- グループ選択（ドロップダウン）
- アクション大分類: Allow / Deny（ラジオボタン or セレクト）
- アクション小分類:
  - Allow 選択時: 通常着信(VR) / IVR(IV) / 留守電(VM)
  - Deny 選択時: BUSY(BZ) / RING_FOREVER(NR) / ANNOUNCE_AND_HANGUP(AN)
- アクション別オプション:
  - VR: ☑録音あり/なし、☑事前アナウンスあり/なし、アナウンス選択
  - IV: IVRフロー選択（ID入力）、☑includeAnnouncement
  - VM: アナウンス選択
  - BZ / NR: 追加設定なし
  - AN: アナウンス選択
- 保存 / キャンセル ボタン

グループ選択時（左ペインで選択した場合）:

- グループ名（読み取り専用）
- 説明テキスト（読み取り専用）
- 登録番号一覧（読み取り専用）
- 「番号グループタブで編集」リンク

#### 5.5.3 アナウンス選択コンポーネント

`/api/announcements` から取得したアナウンス一覧をドロップダウンで表示。
各アイテムはアナウンス名 + タイプバッジで識別。
#129 で作成したアナウンスがそのまま選択肢になる。

### 5.6 電話番号正規化

> **設計変更**: 電話番号の入力・正規化は `/groups` ページ側で行う。
> `/call-actions` ではグループを読み取り専用で参照するため、正規化処理は不要。

正規化ロジック（`/groups` ページ側で実装）:

```typescript
function normalizePhoneNumber(raw: string): string {
  // ハイフン、空白、括弧を除去
  return raw.replace(/[-\s()（）]/g, "")
}
```

- PoC では E.164 変換（+81 → 0）は行わない

### 5.7 エラー仕様・バリデーション

> **設計変更**: グループ管理系のバリデーション（番号重複、グループ名必須等）は `/groups` ページ側で実施。
> `/call-actions` ではルール固有のバリデーションのみ行う。

| ケース | 挙動 |
|---|---|
| ルール名が空 | エラー「ルール名を入力してください」 |
| ルールにグループ未選択 | エラー「番号グループを選択してください」 |
| 番号グループが 0 件でルール追加 | エラー「先に番号グループタブでグループを作成してください」 |
| 選択済みグループが /groups 側で削除された場合 | ルール一覧でグループ名を「（削除済み）」と表示。保存時に警告 |

> **注**: これらのバリデーションはフロントエンド側で実施。

### 5.8 アナウンス選択の UI 挙動

| 状態 | 挙動 |
|---|---|
| アナウンスが 0 件 | ドロップダウンに「（アナウンス未登録）」を表示、選択不可（disabled） |
| アナウンス選択で「なし」を選ぶ | 「なし」選択肢を常に先頭に配置（`announcementId = null`） |
| 選択済みアナウンスが削除された場合 | `announcementId` は保持されるが、ドロップダウンに「（削除済み）」と表示 |

### 5.9 PoC 制約事項

| 項目 | PoC での扱い |
|------|-------------|
| VR 宛先 | 固定（UI で宛先は触らない） |
| VM 宛先 | 固定 |
| 録音の実動作 | フラグ保持のみ（実録音は将来） |
| includeAnnouncement | フラグ保持のみ |
| IVR フロー選択 | ID 手入力 or 既存フロー一覧から選択（一覧がなければ手入力） |
| ドラッグ&ドロップ並替え | 上下ボタンで代替 |
| バックエンド連携 | 本 Issue では行わない |
| 番号グループの管理 | `/groups` タブで行う。`/call-actions` は読み取り専用 |
| /groups の永続化 | 本 Issue の前提条件として別 Issue で対応が必要 |
| 既存 /routing | そのまま共存（将来統合予定） |

---

## 5.10 詳細設計追加

### DD-132-FN-01: CallActionsContent コンポーネント

```typescript
export function CallActionsContent(): JSX.Element
```

#### 状態管理

| State | 型 | 説明 |
|-------|-----|------|
| callerGroups | CallerGroup[] | 番号グループ一覧（`/api/number-groups` から読み取り専用で取得） |
| rules | IncomingRule[] | ルール一覧（優先順位順） |
| anonymousAction | AnonymousAction | 非通知着信時のアクション |
| defaultAction | DefaultAction | デフォルトアクション |
| selectedGroupId | string \| null | 選択中のグループ ID（左ペインで選択、読み取り専用表示用） |
| selectedRuleId | string \| null | 選択中のルール ID |
| announcements | StoredAnnouncement[] | アナウンス一覧（#129 から取得） |
| loading | boolean | 読み込み中フラグ |
| busy | boolean | 操作中フラグ |

#### レンダリング

- 2ペイン構成（左: グループ一覧 **読み取り専用**、右: ルール一覧 + 詳細パネル）
- ルール詳細は選択中ルールに応じて表示
- グループ詳細は選択中グループに応じて左ペイン下部に **読み取り専用** で表示

### DD-132-FN-02: callActionsStore（lib/db/call-actions.ts）

```typescript
function readCallActions(): Promise<CallActionsDatabase>
function writeCallActions(data: CallActionsDatabase): Promise<void>
```

- `storage/db/call-actions.json` の読み書き（ルール・非通知・デフォルトのみ）
- ファイル未存在時は初期状態を返す
- 書き込みは一時ファイル経由のアトミック書き込み（sync.ts と同パターン）

### DD-132-FN-03: numberGroupsStore（lib/db/number-groups.ts）

```typescript
function readNumberGroups(): Promise<NumberGroupsDatabase>
function writeNumberGroups(data: NumberGroupsDatabase): Promise<void>
```

- `storage/db/number-groups.json` の読み書き（番号グループ共有ストア）
- `/groups` ページが書き込み、`/call-actions` ページが読み取りで利用
- ファイル未存在時は `{ callerGroups: [] }` を返す

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #132 | STEER-132 | 起票 |
| Issue #127 | Issue #132 | 親子 |
| STEER-132 | RD-004 (FR-160〜) | 要件追加 |
| RD-004 | DD-132-FN-01, DD-132-FN-02 | 設計 |
| STEER-129 | STEER-132 | アナウンス選択の依存 |
| /groups ページ永続化（別 Issue） | STEER-132 | 前提条件（番号グループ共有ストア） |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [ ] 要件の記述が明確か
- [ ] 詳細設計で実装者が迷わないか
- [ ] ActionCode マッピングが既存定義と整合しているか
- [ ] アナウンス選択が #129 と正しく連携するか
- [ ] PoC 制約事項が明記されているか

### 7.2 マージ前チェック（Approved → Merged）

- [ ] 実装が完了している（該当する場合）
- [ ] コードレビューを受けている（該当する場合）
- [ ] 関連テストがPASS（該当する場合）
- [ ] 本体仕様書への反映準備ができている

---

## 8. 備考

- 本 Issue はフロントエンド UI + JSON 永続化のみ。バックエンドへのルール同期は #130 以降で対応
- **番号グループの管理は `/groups` タブが担当**。`/call-actions` はグループを読み取り専用で参照する
- **前提条件**: `/groups` ページのモックデータを JSON 永続化 + API に切り替える必要がある（別 Issue で対応）
- 番号グループの共有ストアは `storage/db/number-groups.json` に配置する
- `ActionCode` 型は `lib/types.ts` に既に `VR` を含むため、型定義の追加は不要
- IVR フロー一覧の取得は既存 `/ivr` ページの実装状況に依存する。PoC では手入力でもよい
- 既存の `/routing`（ルーティング）ページはそのまま残す（将来統合予定）

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-08 | 初版作成 | Claude Code (claude-opus-4-6) |
| 2026-02-08 | Codex レビュー反映: API 簡略化（GET+PUT）、エラー仕様追加、アナウンス未登録時 UI 追加 → Approved | Claude Code (claude-opus-4-6) |
| 2026-02-08 | 非通知（anonymous）着信アクション対応を追加: anonymousAction フィールド、評価フロー、AC-16、UI 配置 | Claude Code (claude-opus-4-6) |
| 2026-02-08 | 番号グループを /groups タブ参照に変更: CallerGroup を共有ストア化、call-actions から CRUD 除去、左ペイン読み取り専用化、number-groups API 追加 | Claude Code (claude-opus-4-6) |
| 2026-02-08 | 実装完了（Codex）、承認記録 | Claude Code (claude-opus-4-6) |
