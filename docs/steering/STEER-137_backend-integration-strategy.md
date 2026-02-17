# STEER-137: Backend 連携統合戦略

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-137 |
| タイトル | フロントエンド着信設定の Backend 連携統合戦略 |
| ステータス | Approved |
| 関連Issue | #137 |
| 優先度 | P0 |
| 作成日 | 2026-02-08 |

---

## 2. ストーリー（Why）

### 2.1 背景

STEER-132/134 で Frontend PoC として以下が完成している：

| 成果物 | 内容 | 永続化 |
|--------|------|--------|
| STEER-132 | 着信時アクション決定 UI（番号グループ × ルール評価） | `storage/db/call-actions.json` |
| STEER-134 | IVR フロー管理 UI（DTMF メニュー定義） | `storage/db/ivr-flows.json` |

一方、Backend 側（STEER-110, BD-004）では以下が定義されている：

| 成果物 | 内容 | データベース |
|--------|------|-------------|
| BD-004 | ルーティングテーブル設計（`registered_numbers`, `routing_rules`） | PostgreSQL |
| BD-004 | IVR テーブル設計（`ivr_flows`, `ivr_nodes`, `ivr_transitions`） | PostgreSQL |
| STEER-110 | 通話履歴・録音・同期基盤の DB 設計 | PostgreSQL |

**現状の問題**:

1. **データモデルの不一致**: Frontend PoC（番号グループ + ルール配列）と Backend（番号単位 + カテゴリ単位）でデータ構造が異なる
2. **IVR モデルの不一致**: Frontend（フローベース）と Backend（ノードベース）でモデルが根本的に異なる
3. **同期方式の未定義**: Frontend → Backend の設定同期方式が未定義
4. **Backend 実装の欠落**: 設定を受け取って実際に動作させる実装がない

### 2.2 目的

Frontend PoC と Backend を統合するための **設計戦略を確定** し、後続 Issue で詳細設計・実装を進める基盤を作る。

### 2.3 ユーザーストーリー

```
As a プロジェクト管理者
I want to Frontend PoC と Backend の統合戦略を明確にしたい
So that 後続 Issue で迷いなく設計・実装を進められる

受入条件:
- [ ] AC-1: Frontend PoC と Backend のデータモデルギャップが明確化されている
- [ ] AC-2: データモデル統合戦略（番号グループ、ルール評価、IVR モデル）が決定されている
- [ ] AC-3: 同期方式（タイミング、API 設計、SoT）が決定されている
- [ ] AC-4: 実装優先順位（Phase 分割）が決定されている
- [ ] AC-5: 最低限必要な後続 Issue が列挙されている
- [ ] AC-6: リスク・制約事項が明記されている
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-08 |
| 起票理由 | Frontend PoC と Backend の連携検討が必要 |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Code (claude-sonnet-4-5) |
| 作成日 | 2026-02-08 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "Issue #137 の要件整理と統合戦略の壁打ち" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| - | - | - | - | - |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | @MasanoriSuda |
| 承認日 | 2026-02-08 |
| 承認コメント | Backend 契機の Pull 同期方針で承認。後続 Issue で段階的に実装。 |

### 3.5 実装（該当する場合）

本ステアリングは戦略定義のため、実装は後続 Issue で実施。

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
| docs/contract.md | 修正（後続 Issue） | Frontend → Backend 同期 API の追加 |
| virtual-voicebot-backend/docs/design/basic/BD-004_call-routing-db.md | 修正（後続 Issue） | `caller_groups` テーブル追加、ルール評価方式の追加 |
| virtual-voicebot-backend/docs/requirements/RD-新規.md | 追加（後続 Issue） | Backend 着信ルール評価・IVR 実行の要件定義 |

### 4.2 影響するコード

| モジュール | 変更種別 | 概要 |
|-----------|---------|------|
| virtual-voicebot-frontend/app/api/call-actions/route.ts | 修正（後続 Issue） | GET エンドポイント追加（JSON ファイルから読み取り） |
| virtual-voicebot-frontend/app/api/ivr-flows/route.ts | 修正（後続 Issue） | GET エンドポイント追加（JSON ファイルから読み取り） |
| virtual-voicebot-backend/src/bin/serversync.rs | 修正（後続 Issue） | 設定 Pull ロジック追加 |
| virtual-voicebot-backend/src/interface/sync/ | 修正（後続 Issue） | Frontend 設定取得・変換処理追加 |
| virtual-voicebot-backend/src/domain/ | 追加（後続 Issue） | ルール評価エンジン、IVR 実行エンジン実装 |

---

## 5. 差分仕様（What / How）

### 5.1 現状ギャップ分析

#### 5.1.1 データモデルの不一致

| 観点 | Frontend PoC (STEER-132) | Backend (BD-004) | 状態 |
|------|------------------------|-----------------|------|
| **番号管理** | `CallerGroup`（番号グループ）<br>複数番号をグループ化 | `registered_numbers`<br>番号単位で管理 | ❌ 不一致 |
| **ルール評価** | `IncomingRule[]`<br>優先順位付き配列、上から順に評価 | `registered_numbers`（番号完全一致）<br>→ `routing_rules`（カテゴリ + priority） | ❌ 方式が異なる |
| **IVR モデル** | `IvrFlowDefinition`<br>フローベース（単一メニュー + DTMF ルート配列） | `ivr_nodes` + `ivr_transitions`<br>ノードベース（グラフ構造） | ❌ 構造が根本的に異なる |
| **ActionCode** | VR, IV, VM, BZ, NR, AN | VB, VR, NR, RJ, BZ, AN, AR, VM, IV | △ 部分的に一致 |

#### 5.1.2 同期方式の未定義

| 項目 | 現状 | 必要な決定 |
|------|------|----------|
| Frontend → Backend 同期タイミング | 未定義 | 即時 / 定期 / 手動？ |
| Frontend → Backend API | CRUD API は定義済み（contract.md）<br>PoC の JSON → Backend への変換方式は未定義 | 変換方式 or 専用 API？ |
| SoT の運用 | 契約仕様：「Backend DB が SoT」 | Frontend JSON ファイルの位置づけは？ |

#### 5.1.3 Backend 実装の欠落

| 項目 | 現状 | 必要な実装 |
|------|------|----------|
| 着信時ルール評価エンジン | なし | Backend で着信時に設定を評価して ActionCode を決定 |
| IVR 実行エンジン | なし | DTMF 入力待ち、タイムアウト、リトライ、fallback |
| 録音の実装 | なし | STEER-132 では「フラグ保持のみ」 |
| ActionCode 実行 | 一部のみ | VR / IV / VM / BZ / NR / AN の実装 |

---

### 5.2 推奨アプローチ（設計判断）

#### 5.2.1 データモデル統合戦略

##### 判断 D-01: 番号グループの扱い

**決定**: Frontend の `CallerGroup` を Backend の `registered_numbers` に展開する（Option B）

**理由**:
- Backend の `registered_numbers` テーブルは既に定義済み（BD-004）
- Frontend PoC の `CallerGroup` は UI 上の便宜的なグルーピング
- グループ内の各番号を個別に `registered_numbers` に登録することで、Backend の既存設計を活かせる
- Frontend 側では引き続き `CallerGroup` の概念を保持（UI/UX のため）

**実装方針**:
- Frontend: `CallerGroup` は `storage/db/number-groups.json` で管理（変更なし）
- Backend: `registered_numbers` テーブルに展開（`group_name` カラムを追加して、どのグループに属するかを記録）
- 同期時: Frontend の `CallerGroup.phoneNumbers[]` → Backend の `registered_numbers` 複数レコード

##### 判断 D-02: ルール評価方式の統一

**決定**: 3段階評価（番号単位 > 番号グループ > カテゴリ）を採用（Option C）

**理由**:
- 最も柔軟性が高い
- Frontend PoC の IncomingRule（グループ単位ルール）と Backend の routing_rules（カテゴリ単位ルール）の両方を活かせる
- 番号完全一致 → グループ評価 → カテゴリ評価の順に fallback することで、きめ細かい制御が可能

**評価順序**:
```
1. registered_numbers（番号完全一致、最優先）
   ↓ Miss
2. call_actions ルール（番号グループ単位、優先順位付き配列）
   ↓ Miss
3. routing_rules（カテゴリ単位、priority 順）
   ↓ Miss
4. defaultAction（デフォルトアクション）
```

**実装方針**:
- Backend に `call_action_rules` テーブルを新設（Frontend の `IncomingRule` に対応）
- `call_action_rules` は `caller_group_id` を参照し、優先順位（`priority`）を持つ
- Backend の着信処理で上記の評価順序を実装

##### 判断 D-03: IVR モデルの統一

**決定**: Frontend で簡易定義、Backend で詳細定義に変換（Option C）

**理由**:
- Frontend PoC の `IvrFlowDefinition` は UI フレンドリー（単一メニュー + DTMF ルート配列）
- Backend の `ivr_nodes` + `ivr_transitions` は実行時の柔軟性が高い（グラフ構造）
- Frontend → Backend の変換ロジックを実装することで、両方の利点を活かせる

**変換方針**:
```
Frontend IvrFlowDefinition
  ├─ announcementId → ivr_nodes (node_type=ANNOUNCE)
  ├─ routes[] → ivr_nodes (node_type=KEYPAD) + ivr_transitions (DTMF)
  ├─ invalidInputAnnouncementId → ivr_nodes (ANNOUNCE) + ivr_transitions (INVALID)
  ├─ timeoutAnnouncementId → ivr_nodes (ANNOUNCE) + ivr_transitions (TIMEOUT)
  └─ fallbackAction → ivr_nodes (FORWARD/RECORD/EXIT)
```

**実装方針**:
- Frontend: `IvrFlowDefinition` は `storage/db/ivr-flows.json` で管理（変更なし）
- Backend: `ivr_flows`, `ivr_nodes`, `ivr_transitions` テーブルで管理（BD-004 そのまま）
- 同期 API（`POST /api/ivr-flows/sync`）で Frontend の `IvrFlowDefinition` を受け取り、Backend のノードグラフに変換

##### 判断 D-04: ActionCode の統一

**決定**: Backend の ActionCode 定義を優先、Frontend で未対応のコードは将来追加

**理由**:
- Backend（BD-004）の ActionCode 体系が包括的（基本9種 + IVR内8種）
- Frontend PoC で使用していない VB（Voicebot 録音なし）、RJ（Reject）、AR（Announce+Record）は将来的に UI に追加可能
- 現時点では Frontend PoC で使用中の VR, IV, VM, BZ, NR, AN のみ実装

**ActionCode マッピング（現時点）**:

| Frontend PoC | Backend (BD-004) | 説明 |
|-------------|-----------------|------|
| VR | VR | Voicebot+Record（通常着信・録音あり） |
| IV | IV | IVR フローへ移行 |
| VM | VM | 留守番電話 |
| BZ | BZ | 話中応答 |
| NR | NR | 応答なし（コール音のみ） |
| AN | AN | アナウンス再生（録音なし） |
| - | VB | 未対応（将来追加） |
| - | RJ | 未対応（将来追加） |
| - | AR | 未対応（将来追加） |

#### 5.2.2 同期方式の設計

##### 判断 D-05: Frontend → Backend の同期タイミング

**決定**: Backend 契機の Pull 同期（Serversync の tokio タイマー）

**理由**:
- STEER-096 の方針：「原則として全データ同期は Serversync（Outbox Worker）を経由する」
- Backend の Serversync が定期的に Frontend から設定を Pull することで、同期機構を統一
- Frontend は設定を公開する GET API を提供するのみ（Push 不要）
- 同期失敗時のリトライ、ログ、監視を Serversync で一元管理

**実装方針**:
- Backend の Serversync が定期的（例: 30秒ごと）に Frontend の設定 API を呼び出し
  - `GET /api/call-actions` → Backend DB の `call_action_rules` に反映
  - `GET /api/ivr-flows` → Backend DB の `ivr_flows` / `ivr_nodes` / `ivr_transitions` に反映
- Frontend は JSON ファイル保存のみ（Backend への通知不要）
- Serversync 停止中は設定同期が行われない（通話データと同じ挙動）

##### 判断 D-06: Frontend → Backend の API 設計

**決定**: Frontend が設定を公開する GET API を新設（Backend が Pull）

**理由**:
- STEER-096 の方針に従い、Backend の Serversync が Frontend から設定を Pull
- Frontend は設定を JSON ファイルに保存し、GET API で公開するのみ
- Backend 側で Pull + 変換処理を実装することで、同期ロジックを Backend に集約

**新設 API（Frontend 側）**:

| エンドポイント | メソッド | レスポンス | 説明 |
|--------------|---------|----------|------|
| `GET /api/call-actions` | GET | `{ ok: boolean, callerGroups: CallerGroup[], rules: IncomingRule[], anonymousAction, defaultAction }` | 着信アクション設定を返す |
| `GET /api/ivr-flows` | GET | `{ ok: boolean, flows: IvrFlowDefinition[] }` | IVR フロー定義を返す |

**処理内容（Backend Serversync）**:
- 定期的（例: 30秒ごと）に Frontend の GET API を呼び出し
- `GET /api/call-actions`:
  1. `callerGroups[]` → Backend DB の `registered_numbers` テーブルに展開（`group_name` カラムに記録）
  2. `rules[]` → Backend DB の `call_action_rules` テーブルに保存
  3. `anonymousAction`, `defaultAction` → Backend DB の `system_settings.extra` (JSONB) に保存
- `GET /api/ivr-flows`:
  1. `flows[]` の各 `IvrFlowDefinition` を `ivr_nodes` + `ivr_transitions` に変換
  2. 循環参照検出、depth チェック（≤3）を実施
  3. Backend DB に保存

##### 判断 D-07: SoT の明確化

**決定**: Backend DB が SoT、Frontend JSON ファイルは PoC 用の一時ストレージ

**理由**:
- 契約仕様（contract.md v2）の SoT 原則：「Backend DB が全エンティティの SoT」
- Frontend の JSON ファイルは PoC 段階の便宜的な永続化
- 将来的には Backend DB から設定を取得する方式に移行（Backend → Frontend 同期）

**移行方針**:
- **Phase 1（現在）**: Frontend JSON → Backend DB（一方向同期）
- **Phase 2（将来）**: Backend DB → Frontend DB（Transactional Outbox 経由）
- **Phase 3（将来）**: Frontend は Backend から設定を取得して表示（JSON ファイル廃止）

#### 5.2.3 実装優先順位（Phase 分割）

##### Phase 1: 同期基盤（最優先）
- Frontend 側: 設定公開 API 実装（`GET /api/call-actions`, `GET /api/ivr-flows`）
- Backend Serversync: Frontend から設定を Pull + DB に保存（変換処理含む）
- **実装は行うが、着信時の動作はまだしない**（設定が正しく保存されるかのみ検証）

##### Phase 2: ルール評価エンジン + VR 実装
- Backend で着信時にルール評価エンジンを実装（3段階評価）
- VR（Voicebot+Record / B2BUA転送）のみ動作させる
- **最もシンプルな ActionCode で動作検証**

##### Phase 3: 全 ActionCode 実装
- IV, VM, BZ, NR, AN の実装
- IVR 以外の全 ActionCode が動作する状態

##### Phase 4: IVR 実行エンジン実装
- Backend で IVR 実行エンジンを実装
- DTMF 入力待ち、タイムアウト、リトライ、fallback の実装
- **最も複雑な機能なので後回し**

##### Phase 5: 録音実装
- STEER-132 では「フラグ保持のみ」だった録音を実際に動作させる
- 録音ファイルの保存、メタデータ管理、Frontend への同期

---

### 5.3 最低限必要な後続 Issue

以下の Issue を順次起票・実施する：

#### Issue #138（仮）: Backend 要件定義 + データベーススキーマ設計

**スコープ**:
- Backend 要件定義書（RD）作成
  - 着信ルール評価エンジンの要件
  - IVR 実行エンジンの要件
- BD-004 修正
  - `call_action_rules` テーブル追加
  - `registered_numbers.group_name` カラム追加
  - `system_settings.extra` (JSONB) に `anonymousAction`, `defaultAction` 追加
- contract.md 修正
  - `POST /api/call-actions/sync` API 仕様追加
  - `POST /api/ivr-flows/sync` API 仕様追加
- DDL（マイグレーションファイル）作成

**優先度**: P0（ブロッカー）

**依存**: STEER-137 承認後

---

#### Issue #139（仮）: Frontend → Backend 同期実装（Phase 1）

**スコープ**:
- Frontend 側実装
  - `GET /api/call-actions` エンドポイント実装（JSON ファイルから読み取って返す）
  - `GET /api/ivr-flows` エンドポイント実装（JSON ファイルから読み取って返す）
- Backend Serversync 実装
  - Serversync に設定 Pull ロジック追加（tokio タイマーで定期実行）
  - Frontend から `GET /api/call-actions` を呼び出し
  - Frontend から `GET /api/ivr-flows` を呼び出し
  - CallerGroup → registered_numbers 変換処理
  - IvrFlowDefinition → ivr_nodes/ivr_transitions 変換処理
  - Backend DB に保存
- テスト
  - Frontend で設定変更（JSON 保存）→ Serversync が Pull → Backend DB に正しく保存されるか検証

**優先度**: P0（ブロッカー）

**依存**: Issue #138 完了後

---

#### Issue #140（仮）: Backend ルール評価エンジン実装（Phase 2）

**スコープ**:
- Backend 側実装
  - 着信時のルール評価エンジン（3段階評価）
    1. `registered_numbers`（番号完全一致）
    2. `call_action_rules`（番号グループ単位）
    3. `routing_rules`（カテゴリ単位）
    4. `defaultAction`
  - VR（Voicebot+Record / B2BUA転送）の実装
- テスト
  - Frontend で設定したルールが Backend で正しく評価されるか検証
  - VR が正しく動作するか検証

**優先度**: P0（ブロッカー）

**依存**: Issue #139 完了後

---

#### Issue #141（仮）: Backend 全 ActionCode 実装（Phase 3）

**スコープ**:
- Backend 側実装
  - IV（IVR フローへ移行）※ IVR 実行エンジンは Phase 4
  - VM（留守番電話）
  - BZ（話中応答）
  - NR（応答なし）
  - AN（アナウンス再生）
- テスト
  - 各 ActionCode が正しく動作するか検証

**優先度**: P1

**依存**: Issue #140 完了後

---

#### Issue #142（仮）: Backend IVR 実行エンジン実装（Phase 4）

**スコープ**:
- Backend 側実装
  - IVR 実行エンジン
    - アナウンス再生 → DTMF 入力待ち → ルート評価
    - タイムアウト / 無効入力時のアナウンス再生
    - リトライカウント + fallback 処理
    - ネスト IVR（depth ≤ 3）
- テスト
  - Frontend で定義した IVR フローが Backend で正しく実行されるか検証

**優先度**: P1

**依存**: Issue #141 完了後

---

#### Issue #143（仮）: Backend 録音実装強化（Phase 5）

**スコープ**:
- Backend 側実装
  - 録音フラグ（`recording_enabled`, `announceEnabled`）に基づく実際の録音
  - 録音ファイルの保存（既存実装の強化）
  - 録音メタデータの管理（`recordings` テーブル）
- テスト
  - 録音が正しく動作するか検証
  - Frontend で録音ファイルが再生できるか検証

**優先度**: P2

**依存**: Issue #141 完了後（Phase 4 と並行可能）

---

### 5.4 追加で必要になる可能性のある Issue（将来的に）

以下は最低限には含まれないが、実装過程で必要になる可能性がある：

- **Issue #144（仮）**: Frontend UI の Backend 統合対応
  - Backend から設定を取得して表示（Backend → Frontend 同期）
  - JSON ファイルの段階的廃止
- **Issue #145（仮）**: エラーハンドリング強化
  - Backend API のエラーレスポンスを Frontend で適切に表示
  - 楽観ロック（version）対応
- **Issue #146（仮）**: パフォーマンス最適化
  - ルール評価エンジンのキャッシュ
  - DB クエリの最適化
- **Issue #147（仮）**: E2E テスト実装
  - Frontend → Backend → 着信動作 の一連のフローをテスト
- **Issue #148（仮）**: 監視・ログ強化
  - ルール評価結果のログ出力
  - IVR 実行フローのトレース

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #137 | STEER-137 | 起票 |
| STEER-132 | STEER-137 | Frontend PoC（着信アクション） |
| STEER-134 | STEER-137 | Frontend PoC（IVR フロー） |
| STEER-110 | STEER-137 | Backend DB 設計 |
| BD-004 | STEER-137 | Backend ルーティングテーブル設計 |
| contract.md | STEER-137 | Frontend ↔ Backend API 契約 |
| STEER-137 | Issue #138 | 要件定義 + DB スキーマ設計 |
| STEER-137 | Issue #139 | 同期 API 実装（Phase 1） |
| STEER-137 | Issue #140 | ルール評価エンジン + VR（Phase 2） |
| STEER-137 | Issue #141 | 全 ActionCode 実装（Phase 3） |
| STEER-137 | Issue #142 | IVR 実行エンジン（Phase 4） |
| STEER-137 | Issue #143 | 録音実装強化（Phase 5） |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [ ] ギャップ分析が網羅的か
- [ ] 推奨アプローチの設計判断に合理性があるか
- [ ] 後続 Issue の切り分けが適切か
- [ ] リスク・制約事項が明記されているか
- [ ] 既存仕様（STEER-132/134, BD-004, contract.md）との整合性があるか

### 7.2 マージ前チェック（Approved → Merged）

- [ ] 本体仕様書（contract.md, BD-004 等）への反映は後続 Issue で実施されることが確認されている
- [ ] 後続 Issue が起票されている

---

## 8. 備考

### 8.1 実装の進め方

- **詳細設計・実装は後続 Issue で実施**
- 本ステアリングは「方針確定」のみ
- Issue #138 以降で段階的に実装を進める

### 8.2 リスク・制約事項

| リスク | 影響 | 対策 |
|--------|------|------|
| Frontend PoC と Backend のデータモデル不一致 | 大規模な書き換えが必要 | 変換ロジックを Backend 側で実装することで Frontend の変更を最小化 |
| IVR モデルの変換複雑度 | 変換ロジックのバグリスク | Phase 4 で IVR に集中、テストを徹底 |
| Backend 実装の工数 | 実装が長期化 | Phase 分割して段階的にリリース |
| SoT の曖昧さ | データ競合・矛盾 | Backend DB が SoT という原則を厳守、Frontend JSON ファイルは将来廃止 |

### 8.3 将来的な方向性

- **Phase 1〜5 完了後**: Frontend は Backend DB から設定を取得して表示（Backend → Frontend 同期）
- **Frontend JSON ファイル廃止**: `call-actions.json`, `ivr-flows.json` は PoC 用の一時ストレージとして、最終的には廃止
- **契約仕様の完全準拠**: contract.md の SoT 原則に従い、全エンティティの SoT を Backend DB に統一

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-08 | 初版作成（Draft） | Claude Code (claude-sonnet-4-5) |
| 2026-02-08 | 判断 D-05/D-06 修正：Backend 契機の Pull 同期に変更（STEER-096 方針に従う） | Claude Code (claude-sonnet-4-5) |
| 2026-02-08 | 承認（Approved） | Claude Code (claude-sonnet-4-5) |
