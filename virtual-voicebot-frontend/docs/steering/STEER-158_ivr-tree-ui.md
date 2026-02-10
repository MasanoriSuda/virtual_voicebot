# STEER-158: IVR DTMF ルート UI を N-木ツリービューに再設計

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-158 |
| タイトル | IVR DTMF ルート UI を N-木ツリービューに再設計 |
| ステータス | Draft |
| 関連Issue | #158 |
| 前提ステアリング | STEER-134（IVR フロー管理 UI） |
| 優先度 | P1 |
| 作成日 | 2026-02-09 |

---

## 2. ストーリー（Why）

### 2.1 背景

STEER-134 で実装した IVR フロー管理 UI に対し、以下のクレームが発生している：

| 問題 | 詳細 |
|------|------|
| **構造が見えない** | 左ペインはルートフローのフラットリストのみ。子フロー・孫フローが左ペインに現れず、全体の木構造を俯瞰する手段がない |
| **深い階層が単一スクロール領域に展開** | `renderRoutesEditor` が再帰インライン展開するため、Level 3 を編集するには Level 1 → 2 を全てスクロールで通過する必要がある。現在位置を見失う |
| **3分岐（有効入力/無効入力/タイムアウト）が視覚的に区別されない** | `routes[]`（有効入力）と `invalidInputAnnouncementId` / `timeoutAnnouncementId` / `fallbackAction` がバラバラのフォームフィールドとして配置され、N-木の分岐として認識できない |
| **パンくず/ツリーナビ不在** | 深い階層でのコンテキスト喪失 |

**要約**: データモデルは既にN-木構造（`IvrFlowDefinition` → `routes[]` → `IV` で子フロー参照）だが、UI がフラットであるため不整合が生じている。

### 2.2 目的

- IVR DTMF ルートの構築 UI を **N-木ツリービュー + 右ペイン詳細編集** に再設計し、構造の俯瞰と詳細編集を両立する
- 有効入力・無効入力・タイムアウトの 3 分岐をツリー上で視覚的に明示する
- インライン再帰編集を廃止し、右ペインへの編集集約 + パンくずナビゲーションで迷子を防止する

### 2.3 ユーザーストーリー

```
As a 管理者
I want to IVR の DTMF ルートを N-木（ツリー）構造で俯瞰・編集したい
So that 複雑な多階層メニューでも構造を把握しながら効率的に設定できる

受入条件:
- [ ] AC-1: 左ペインにコラプス可能なツリービューが表示される（ルートフロー → DTMFキー → 子フロー → … の階層構造）
- [ ] AC-2: 各ツリーノードに Valid/Invalid/Timeout の 3 分岐スロットが表示される
- [ ] AC-3: ツリーノードをクリックすると右ペインに該当フローの詳細編集フォームが表示される
- [ ] AC-4: 右ペイン上部にパンくずナビゲーション（例: メインメニュー > Key2:サポート > サポートメニュー）が表示される
- [ ] AC-5: 既存のインライン再帰編集（renderRoutesEditor の再帰展開）が廃止される
- [ ] AC-6: 各ツリーノードに要約チップ（ルート数、アクション種別等）が表示される
- [ ] AC-7: ドラッグ&ドロップは非対応（初期）
- [ ] AC-8: データモデル（IvrFlowDefinition）の変更なし（UI 変更のみ）
- [ ] AC-9: 既存バリデーション（STEER-134 §5.7）は維持
- [ ] AC-10: MAX_IVR_DEPTH=3 の制約は維持
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-09 |
| 起票理由 | IVR DTMF ルート構築 UI が使いづらいとクレーム。N-木構造の表現を要望 |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Code (claude-opus-4-6) |
| 作成日 | 2026-02-09 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "STEER-158 ドラフト作成。案A（ツリービュー）採用、3分岐明示、DnD非対応、パンくず必須" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| 1 | - | - | - | |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | - |
| 承認日 | - |
| 承認コメント | |

### 3.5 実装（該当する場合）

| 項目 | 値 |
|------|-----|
| 実装者 | Codex へ引き継ぎ |
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
| STEER-134 §5.6 | 差替 | UI 構成を「2ペイン + インライン再帰」→「ツリー + 右ペイン」に更新 |

### 4.2 影響するコード

| モジュール | 変更種別 | 概要 |
|-----------|---------|------|
| components/ivr-content.tsx | 大幅書換 | 左ペイン: フラットリスト→ツリービュー、右ペイン: インライン再帰→詳細編集、パンくず追加 |
| lib/ivr-flows.ts | 軽微修正 | ツリー構築ヘルパー関数の追加（型定義の変更なし） |

---

## 5. 差分仕様（What / How）

### 5.1 設計方針：決定事項サマリ

| 論点 | 決定 | 根拠 |
|------|------|------|
| UI 方式 | **A: ツリービュー + 右ペイン詳細編集** | 構造俯瞰と詳細編集の両立。実装コスト中。B（フローチャート）は実装コスト大 |
| 3分岐の表現 | **Valid/Invalid/Timeout をツリー上にスロットノードとして明示** | N-木の分岐構造を忠実に表現。クリックで右ペインの該当セクションにフォーカス |
| DnD | **初期は非対応** | STEER-134 の判断を踏襲（キー順表示で十分） |
| 編集モード | **ツリー＝常時ナビゲーション、編集＝右ペイン集約** | インライン再帰編集を廃止し、迷子リスクを排除 |
| 迷子対策 | **右ペイン上部パンくず必須 + 左ツリーに要約チップ** | 現在位置の明示とコンテキスト維持 |
| データモデル | **変更なし** | `IvrFlowDefinition` の構造は維持。UI レイヤーの変更に閉じる |

### 5.2 画面レイアウト（After）

```
┌──────────────────────────────────────────────────────────────┐
│  IVRフロー                                  [キャンセル] [保存] │
├────────────────────┬─────────────────────────────────────────┤
│ 左ペイン (360px)     │ 右ペイン (flex)                          │
│ ツリービュー          │                                         │
│                    │ パンくず: メインメニュー > Key2 > サポート    │
│ [+ 新規] 🔍検索     │ ─────────────────────────────────────── │
│                    │                                         │
│ ▼ メインメニュー     │ フロー名: [サポートメニュー        ]       │
│   ├─ ✅ Valid       │ 説明:     [サポート部門振り分け     ]       │
│   │  ├ 1: 営業→VR  │ 有効: [✓]                                │
│   │  ├ 2: サポ→IV  │                                         │
│   │  │  ▼ サポートM │ ── メニュー設定 ──                        │
│   │  │    ├ ✅ Valid │ 案内アナウンス: [サポートメニュー v]        │
│   │  │    │ ├1:技術 │ タイムアウト: [10] 秒                     │
│   │  │    │ └2:契約 │ リトライ上限: [2] 回                      │
│   │  │    ├ ❌ Inv. │                                         │
│   │  │    └ ⏱ T.O. │ ── DTMF ルート ──                        │
│   │  └ 9: 留守→VM  │ [1] 技術サポート → 転送(VR)    [×]        │
│   ├─ ❌ Invalid     │ [2] 契約サポート → 転送(VR)    [×]        │
│   │  → リトライ      │ [+ ルート追加]                            │
│   └─ ⏱ Timeout     │                                         │
│      → リトライ      │ ── 無効入力時 ──                          │
│                    │ アナウンス: [(なし) v]                      │
│                    │                                         │
│                    │ ── タイムアウト時 ──                       │
│                    │ アナウンス: [タイムアウト案内 v]              │
│                    │                                         │
│                    │ ── リトライ超過時（Fallback）──             │
│                    │ アクション: [アナウンス再生→切断 v]          │
│                    │                                         │
│                    │ [複製] [削除]                              │
└────────────────────┴─────────────────────────────────────────┘
```

### 5.3 ツリービュー仕様

#### 5.3.1 ツリーノード構成

各 `IvrFlowDefinition` は以下のノード構造で表示される：

```
▼ {フロー名} [{ルート数}件] [depth badge]
  ├─ ✅ Valid Input
  │  ├ {key}: {label} → {actionLabel}     ← IvrRoute ノード
  │  │  ▼ {子フロー名} [...]               ← actionCode=IV の場合、再帰展開
  │  │    ├─ ✅ Valid Input
  │  │    ├─ ❌ Invalid Input
  │  │    └─ ⏱ Timeout
  │  └ {key}: {label} → {actionLabel}
  ├─ ❌ Invalid Input
  │  → {invalidInputAnnouncementId ? アナウンス名 : "ガイダンスなし"} → リトライ
  └─ ⏱ Timeout
     → {timeoutAnnouncementId ? アナウンス名 : "ガイダンスなし"} → リトライ
```

#### 5.3.2 ノード種別

| ノード種別 | アイコン | クリック時の右ペイン動作 |
|-----------|---------|----------------------|
| **FlowNode** | フォルダ（▼/▶） | フロー詳細編集フォーム全体を表示 |
| **ValidSlot** | ✅ | DTMFルートセクションにフォーカス |
| **RouteNode** | DTMFキー番号 | 該当ルートをハイライト（編集可能状態） |
| **InvalidSlot** | ❌ | 無効入力時セクションにフォーカス |
| **TimeoutSlot** | ⏱ | タイムアウト時セクションにフォーカス |

#### 5.3.3 要約チップ

各 FlowNode に以下の要約情報をチップ（Badge）で表示：

| チップ | 表示条件 | 例 |
|--------|---------|-----|
| ルート数 | 常時 | `3件` |
| Depth | depth > 1 | `Lv.2` |
| 無効 | isActive === false | `無効` |
| 未設定 | announcementId === null | `⚠ アナウンス未設定` |

#### 5.3.4 展開/折りたたみ

- FlowNode: デフォルト展開（depth 1）、デフォルト折りたたみ（depth 2+）
- ValidSlot / InvalidSlot / TimeoutSlot: FlowNode 展開時に表示
- ルートノード: ValidSlot 展開時に表示
- 手動でコラプス/エクスパンド可能（▼/▶ トグル）

#### 5.3.5 ツリー検索

- 既存の検索フィルタを維持（フロー名で部分一致）
- マッチしたフローは祖先ノードも自動展開して表示

### 5.4 パンくずナビゲーション仕様

右ペイン上部に固定表示。選択中ノードまでのパスを表示する。

```
メインメニュー > Key 2: サポート > サポートメニュー
```

| 要素 | 表示内容 | クリック動作 |
|------|---------|------------|
| ルートフロー | フロー名 | ルートフローの詳細を右ペインに表示 |
| ルート経由 | `Key {n}: {label}` | 親フローの詳細で該当ルートをハイライト |
| 現在フロー | フロー名（太字） | なし（現在地） |

- セパレータ: `>` (chevron)
- 最大表示幅を超える場合: 先頭を `…` で省略

### 5.5 右ペイン詳細編集仕様

#### 5.5.1 レイアウト構成

```
┌─────────────────────────────────────────┐
│ パンくず: A > Key2 > B                    │ ← 固定ヘッダー
├─────────────────────────────────────────┤
│ ① 基本情報セクション                       │
│   フロー名 / 説明 / 有効・無効              │
├─────────────────────────────────────────┤
│ ② メニュー設定セクション                    │
│   案内アナウンス / タイムアウト / リトライ上限 │
├─────────────────────────────────────────┤
│ ③ DTMF ルートセクション                    │ ← ValidSlot クリック時フォーカス
│   ルート一覧（キー/ラベル/遷移先/削除）       │
│   [+ ルート追加]                           │
├─────────────────────────────────────────┤
│ ④ 無効入力時セクション                      │ ← InvalidSlot クリック時フォーカス
│   アナウンス選択                            │
├─────────────────────────────────────────┤
│ ⑤ タイムアウト時セクション                   │ ← TimeoutSlot クリック時フォーカス
│   アナウンス選択                            │
├─────────────────────────────────────────┤
│ ⑥ フォールバックセクション                   │
│   リトライ超過時アクション                    │
├─────────────────────────────────────────┤
│ [複製] [削除]                              │
└─────────────────────────────────────────┘
```

#### 5.5.2 フォーカス連動

左ツリーでスロットノード（Valid/Invalid/Timeout）をクリックした際、右ペインの対応セクションにスムーズスクロールし、ハイライト（一時的な背景色変化）を付与する。

| ツリーノード | スクロール先 |
|------------|------------|
| FlowNode | セクション①（ページ先頭） |
| ValidSlot | セクション③ |
| RouteNode | セクション③ + 該当ルート行ハイライト |
| InvalidSlot | セクション④ |
| TimeoutSlot | セクション⑤ |

### 5.6 インライン再帰編集の廃止

STEER-134 で実装した `renderRoutesEditor` の再帰展開（depth > 1 でインライン展開）を廃止する。

| Before（STEER-134） | After（STEER-158） |
|---------------------|---------------------|
| `renderRoutesEditor` が depth+1 で自身を再帰呼出し | 子フローは左ツリーのノードとして表示。クリックで右ペインに詳細切替 |
| Level 1〜3 が同一スクロール領域に展開 | 常に 1 フローの詳細のみ右ペインに表示 |
| ルート追加ドラフト（routeDrafts）が各 flowId に散在 | 選択中フローのドラフトのみ右ペインに表示 |

**移行対応**: `renderRoutesEditor` は削除し、新たに `IvrTreeView`（左ペイン）+ `IvrFlowEditor`（右ペイン）コンポーネントに分割する。

### 5.7 ツリー構築ヘルパー（lib/ivr-flows.ts 追加）

型定義の変更は **なし**。以下のヘルパー関数を追加する：

```typescript
/** ツリー表示用のノード型（UI レイヤー専用） */
interface IvrTreeNode {
  type: "flow" | "valid-slot" | "invalid-slot" | "timeout-slot" | "route"
  flowId: string
  routeIndex?: number            // type="route" のとき
  label: string                  // ツリー表示ラベル
  children: IvrTreeNode[]
  depth: number
  meta: {
    routeCount?: number          // type="flow" のとき
    actionLabel?: string         // type="route" のとき
    hasWarning?: boolean         // アナウンス未設定等
  }
}

/** IvrFlowDefinition[] からツリーノードを構築 */
function buildIvrTree(
  flows: IvrFlowDefinition[],
  rootFlowId: string,
  allFlows: Map<string, IvrFlowDefinition>,
  depth?: number,
  ancestry?: string[],
): IvrTreeNode

/** パンくずパス構築 */
function buildBreadcrumb(
  targetFlowId: string,
  flows: IvrFlowDefinition[],
): BreadcrumbItem[]

interface BreadcrumbItem {
  flowId: string
  flowName: string
  viaRoute?: { dtmfKey: DtmfKey; label: string }
}
```

### 5.8 バリデーション

STEER-134 §5.7 のバリデーションルールを **すべて維持**。変更なし。

- フロー名空チェック
- routes 0 件チェック
- dtmfKey 重複チェック
- ネスト depth > 3 チェック
- 循環参照検出
- ラベル空チェック

### 5.9 状態管理の変更

| State | Before | After | 変更理由 |
|-------|--------|-------|---------|
| `selectedFlowId` | 左リストの選択 | ツリーノード選択（FlowNode の flowId） | ツリーナビゲーション |
| `selectedSection` | なし | `"basic" \| "routes" \| "invalid" \| "timeout" \| "fallback" \| null` | フォーカス連動 |
| `treeExpandedNodes` | なし | `Set<string>` | ツリーの展開/折りたたみ状態 |
| `flows` | 変更なし | 変更なし | - |
| `routeDrafts` | 変更なし | 変更なし | - |
| `searchQuery` | 変更なし | 変更なし | - |

### 5.10 PoC 制約（STEER-134 からの差分）

| 項目 | STEER-134 の判断 | STEER-158 の判断 |
|------|-----------------|-----------------|
| フォルダツリー | PoC 廃止（フラットリスト） | **復活: N-木ツリービュー採用** |
| DnD 並替え | 不要 | 不要（据え置き） |
| データモデル変更 | - | なし（UI のみ） |
| インライン再帰編集 | 採用 | **廃止 → 右ペイン詳細に集約** |

---

## 5.11 詳細設計追加

### DD-158-FN-01: IvrTreeView コンポーネント

```typescript
interface IvrTreeViewProps {
  flows: IvrFlowDefinition[]
  selectedFlowId: string | null
  selectedSection: string | null
  searchQuery: string
  announcements: StoredAnnouncement[]
  onSelectNode: (flowId: string, section?: string, routeIndex?: number) => void
  onExpandToggle: (nodeKey: string) => void
  expandedNodes: Set<string>
}

const IvrTreeView: React.FC<IvrTreeViewProps>
```

#### Props

| パラメータ | 型 | 説明 |
|-----------|-----|------|
| flows | IvrFlowDefinition[] | 全 IVR フロー |
| selectedFlowId | string \| null | 選択中フロー ID |
| selectedSection | string \| null | 選択中セクション |
| searchQuery | string | 検索フィルタ |
| announcements | StoredAnnouncement[] | アナウンス一覧（要約チップ用） |
| onSelectNode | function | ノード選択コールバック |
| onExpandToggle | function | 展開/折りたたみコールバック |
| expandedNodes | Set\<string\> | 展開中ノードキーの集合 |

#### レンダリング

- `buildIvrTree()` でツリーノードを構築し、再帰的にレンダリング
- 検索クエリに応じてフィルタ + 祖先自動展開
- 選択中ノードをハイライト（`bg-accent`）

#### トレース

- ← STEER-158 AC-1, AC-2, AC-6
- → UT-158-TC-01, UT-158-TC-02

### DD-158-FN-02: IvrFlowEditor コンポーネント

```typescript
interface IvrFlowEditorProps {
  flow: IvrFlowDefinition
  allFlows: IvrFlowDefinition[]
  announcements: StoredAnnouncement[]
  breadcrumb: BreadcrumbItem[]
  focusSection: string | null
  focusRouteIndex: number | null
  onUpdateFlow: (updated: IvrFlowDefinition) => void
  onDeleteFlow: (flowId: string) => void
  onDuplicateFlow: (flowId: string) => void
  onBreadcrumbNavigate: (flowId: string) => void
  busy: boolean
}

const IvrFlowEditor: React.FC<IvrFlowEditorProps>
```

#### Props

| パラメータ | 型 | 説明 |
|-----------|-----|------|
| flow | IvrFlowDefinition | 編集対象フロー |
| allFlows | IvrFlowDefinition[] | 全フロー（ルート遷移先選択用） |
| announcements | StoredAnnouncement[] | アナウンス一覧 |
| breadcrumb | BreadcrumbItem[] | パンくずパス |
| focusSection | string \| null | フォーカスセクション |
| focusRouteIndex | number \| null | フォーカスルートインデックス |
| onUpdateFlow | function | フロー更新コールバック |
| onDeleteFlow | function | フロー削除コールバック |
| onDuplicateFlow | function | フロー複製コールバック |
| onBreadcrumbNavigate | function | パンくずクリック時のナビゲーション |
| busy | boolean | 操作中フラグ |

#### レンダリング

- セクション ①〜⑥（§5.5.1 参照）を上から順に配置
- `focusSection` 変更時に対応セクションへ `scrollIntoView({ behavior: "smooth" })` + 一時ハイライト
- ルート追加 UI は STEER-134 §5.6.2 のドラフトモードを踏襲（右ペイン内でインライン）

#### トレース

- ← STEER-158 AC-3, AC-4, AC-5
- → UT-158-TC-03, UT-158-TC-04

### DD-158-FN-03: buildIvrTree / buildBreadcrumb（lib/ivr-flows.ts）

§5.7 のシグネチャ参照。

#### buildIvrTree

- ルートフローから再帰的に `IvrTreeNode` を構築
- `ancestry` で循環検出（STEER-134 の既存ロジックを再利用）
- `depth >= MAX_IVR_DEPTH` で再帰停止
- 各フローに Valid/Invalid/Timeout のスロットノードを自動付与

#### buildBreadcrumb

- `targetFlowId` からルートフローまでの経路を逆引き
- 全フローの `routes` を走査し、`destination.actionCode === "IV" && destination.ivrFlowId === targetFlowId` となるルートを探索
- 経路が見つからない場合（ルートフロー自身）は単一要素の配列を返す

#### トレース

- ← DD-158-FN-01, DD-158-FN-02
- → UT-158-TC-05, UT-158-TC-06

---

## 5.12 テストケース追加

### UT-158-TC-01: ツリービューのレンダリング

**対象**: DD-158-FN-01 (IvrTreeView)

| # | テスト名 | 入力 | 期待結果 |
|---|---------|------|---------|
| 1 | 単一フロー | 1フロー, 2ルート | FlowNode + ValidSlot(2ルート) + InvalidSlot + TimeoutSlot |
| 2 | ネスト2層 | 親→子(IV) | 親FlowNode > ValidSlot > RouteNode(IV) > 子FlowNode > ... |
| 3 | ネスト3層 | 親→子→孫 | 3層まで展開。depth badge が Lv.2, Lv.3 |
| 4 | 循環参照 | A→B→A | 循環ノードで展開停止、警告アイコン表示 |
| 5 | 検索フィルタ | query="サポート" | マッチフローが表示、祖先も自動展開 |

### UT-158-TC-02: ツリーノード選択

**対象**: DD-158-FN-01 (IvrTreeView)

| # | テスト名 | 操作 | 期待結果 |
|---|---------|------|---------|
| 1 | FlowNode選択 | FlowNodeクリック | onSelectNode(flowId, "basic") |
| 2 | ValidSlot選択 | ValidSlotクリック | onSelectNode(flowId, "routes") |
| 3 | RouteNode選択 | RouteNodeクリック | onSelectNode(flowId, "routes", routeIndex) |
| 4 | InvalidSlot選択 | InvalidSlotクリック | onSelectNode(flowId, "invalid") |
| 5 | TimeoutSlot選択 | TimeoutSlotクリック | onSelectNode(flowId, "timeout") |

### UT-158-TC-03: 右ペイン詳細編集

**対象**: DD-158-FN-02 (IvrFlowEditor)

| # | テスト名 | 入力 | 期待結果 |
|---|---------|------|---------|
| 1 | フロー表示 | flow + breadcrumb | 全セクション（①〜⑥）が表示、パンくず正常 |
| 2 | セクションフォーカス | focusSection="invalid" | ④無効入力時セクションにスクロール+ハイライト |
| 3 | ルートフォーカス | focusSection="routes", focusRouteIndex=1 | ③の2番目ルート行がハイライト |
| 4 | パンくずナビ | breadcrumb[0]クリック | onBreadcrumbNavigate(rootFlowId) |

### UT-158-TC-04: ルート編集（右ペイン内）

**対象**: DD-158-FN-02 (IvrFlowEditor)

| # | テスト名 | 操作 | 期待結果 |
|---|---------|------|---------|
| 1 | ルート追加 | [+ルート追加]クリック | ドラフトモード表示、キー/ラベル/遷移先入力 |
| 2 | ルート削除 | [×]クリック | 確認なしで即削除、ツリー反映 |
| 3 | 遷移先IV選択 | 遷移先=IV, フロー選択 | 子フローがツリーに追加表示 |
| 4 | ルート遷移先変更 | VR→VM | destination 更新、ツリー要約チップ更新 |

### UT-158-TC-05: buildIvrTree

**対象**: DD-158-FN-03

| # | テスト名 | 入力 | 期待結果 |
|---|---------|------|---------|
| 1 | 単一フロー | 1フロー(2ルート) | FlowNode > [ValidSlot(2子), InvalidSlot, TimeoutSlot] |
| 2 | 2層ネスト | 親(key2→IV子) | 親 > ValidSlot > Route(key2) > 子FlowNode > [Valid, Invalid, Timeout] |
| 3 | 循環検出 | A→B→A | A > ... > B で停止（B の子に A を再帰展開しない） |
| 4 | depth上限 | 3層ネスト | depth=3 で IV ルートの子フロー展開なし |

### UT-158-TC-06: buildBreadcrumb

**対象**: DD-158-FN-03

| # | テスト名 | 入力 | 期待結果 |
|---|---------|------|---------|
| 1 | ルートフロー | rootFlowId | [{flowId: root, flowName: "メイン"}] |
| 2 | 子フロー | childFlowId | [{flowId: root, flowName: "メイン", viaRoute: {key:"2", label:"サポート"}}, {flowId: child, flowName: "サポートM"}] |
| 3 | 孫フロー | grandchildFlowId | 3要素の配列（root > child > grandchild） |
| 4 | 孤立フロー | orphanFlowId（参照元なし） | [{flowId: orphan, flowName: "..."}]（単一要素） |

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #158 | STEER-158 | 起票 |
| STEER-134 | STEER-158 | UI 再設計（差替） |
| STEER-158 AC-1,2,6 | DD-158-FN-01 | ツリービュー |
| STEER-158 AC-3,4,5 | DD-158-FN-02 | 右ペイン詳細編集 |
| DD-158-FN-01 | UT-158-TC-01, TC-02 | 単体テスト |
| DD-158-FN-02 | UT-158-TC-03, TC-04 | 単体テスト |
| DD-158-FN-03 | UT-158-TC-05, TC-06 | 単体テスト |
| RD-004 FR-121,122 | STEER-158 | 要件トレース（DTMF メニュー、多階層メニュー） |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [ ] 要件の記述が明確か
- [ ] 詳細設計で実装者（Codex）が迷わないか
- [ ] テストケースが網羅的か
- [ ] STEER-134 との整合性があるか（データモデル不変、バリデーション維持）
- [ ] トレーサビリティが維持されているか
- [ ] ツリービューの操作性が UX 的に妥当か

### 7.2 マージ前チェック（Approved → Merged）

- [ ] 実装が完了している
- [ ] コードレビューを受けている
- [ ] 関連テストがPASS
- [ ] STEER-134 §5.6 の「UI 構成」セクションが本仕様で差替されている

---

## 8. 備考

- 本 Issue は **UI レイヤーの変更のみ**。データモデル（`IvrFlowDefinition`）、API（`/api/ivr-flows`）、バリデーションロジックは STEER-134 のまま維持
- `renderRoutesEditor` の再帰展開ロジックは **全面廃止** し、`IvrTreeView` + `IvrFlowEditor` のコンポーネント分割で置換する
- ツリーの展開/折りたたみ状態は React state（`treeExpandedNodes`）で管理。永続化は不要（ページリロードでリセット）
- 将来拡張: DnD によるルート並替え、ツリーノードの右クリックコンテキストメニュー、ミニマップ表示は後続 Issue で検討

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-09 | 初版作成 | Claude Code (claude-opus-4-6) |
