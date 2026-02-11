# STEER-142: Backend IVR DB スキーマ設計（Phase 4-A）

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-142 |
| タイトル | Backend IVR DB スキーマ設計（Phase 4-A: 既存スキーマとの整合性確保） |
| ステータス | Approved |
| 関連Issue | #142 |
| 優先度 | P0 |
| 作成日 | 2026-02-08 |
| 親ステアリング | STEER-137 |

---

## 2. ストーリー（Why）

### 2.1 背景

Issue #141（STEER-141）で Phase 3 が完了し、**IV（IVR フローへ移行）** の基盤が実装された。

**Phase 3 の成果**:
- IV ActionCode 実行（ivr_state 設定、ivr_flow_id 保存）
- IVR フローへの遷移準備（Phase 4 で実行エンジンを実装）
- Frontend で定義した IVR フローが Backend DB に同期される

**問題**:
- **既存 DB スキーマ（ivr_nodes + ivr_transitions）と IVR 実行エンジンの設計に乖離が発生**
- Frontend PoC (#132, #134) が既存スキーマに依存しているため、スキーマ変更の影響範囲が不明
- シンプルな IVR（1アナウンス + DTMFルート）の仕様が未確定

**影響**:
- Phase 4-B（IVR エンジン実装）が開始できない（DB スキーマが未確定）
- Frontend との整合性が取れない
- 実装フェーズで手戻りが発生するリスク

### 2.2 目的

Backend IVR の **DB スキーマ設計を確定**し、Phase 4-B（IVR エンジン実装）のブロッカーを解消する。

**達成目標**:
- 既存 DB スキーマ（ivr_nodes + ivr_transitions）の分析
- シンプルな IVR（1アナウンス + DTMFルート）の仕様確定
- contract.md の IVR 仕様更新
- Frontend PoC との整合性確認
- 必要に応じた DB マイグレーション設計

### 2.3 ユーザーストーリー

```
As a バックエンド開発者
I want to IVR の DB スキーマが確定している
So that Phase 4-B で IVR 実行エンジンを実装できる

受入条件:
- [ ] 既存 DB スキーマ（ivr_nodes + ivr_transitions）の使用方法が明確
- [ ] シンプルな IVR（1アナウンス + DTMFルート）の仕様が確定
- [ ] contract.md の IVR 仕様が更新されている
- [ ] Frontend PoC (#132, #134) との整合性が確認されている
- [ ] DB マイグレーションの必要性が判定されている
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-08 |
| 起票理由 | Issue #141 完了後、Phase 4 を Phase 4-A（DB設計）+ Phase 4-B（実装）に分割 |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Code (claude-sonnet-4-5) |
| 作成日 | 2026-02-08 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "Issue #142 を Phase 4-A（DB設計）に限定し、改訂版を作成" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| 1 | Claude Code (self-review) | 2026-02-08 | 要修正 | 重大2件（DB スキーマ乖離、IVR 構造不整合）、中2件（アナウンス再生 TODO、fallback Action 未定義）、軽1件（ネスト IVR 曖昧）→ Phase 4 分割で対応 |
| 2 | Codex | 2026-02-08 | 要修正 | 重大2件（toNodeId=NULL 曖昧、fallback 定義不一致）、中2件（Frontend 連携説明ずれ、max_retries 既定値不一致）、軽1件（相対リンク切れ）→ 全て修正完了 |
| 3 | Codex | 2026-02-08 | 要修正 | 軽1件（exit_action 記述不整合）→ 修正完了 |
| 4 | Codex | 2026-02-08 | 要修正 | 重大1件（EXIT ノード tts_text メタデータ未仕様化）、中1件（contract.md toNodeId=null 記述残存）→ 全て修正完了 |
| 5 | Codex | 2026-02-08 | 承認 | 中2件（VB welcomeAnnouncementId を tts_text 非保存、IV recordingEnabled を任意化）→ 全て修正完了、実装可能 |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | Codex (final review #5) |
| 承認日 | 2026-02-08 |
| 承認コメント | 実装可能判定: Yes（実装着手可）。重大なし、中なし、軽なし。全 5 回のレビューサイクルで指摘された全ての問題が解決済み。Phase 4-B 実装に進んで問題なし。 |

### 3.5 実装（該当する場合）

| 項目 | 値 |
|------|-----|
| 実装者 | - |
| 実装開始日 | - |
| 実装完了日 | - |
| PR番号 | - |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ日 | - |
| マージ先 | - |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| contract.md | 修正 | §3.6-3.8（IvrFlow, IvrNode, IvrTransition）の仕様明確化 |
| RD-004 | 参照 | FR-3（IVR 実行）を参照 |

### 4.2 影響するコード

| モジュール | 変更種別 | 概要 |
|-----------|---------|------|
| virtual-voicebot-backend/migrations/*.sql | 参照/修正 | 既存 DB スキーマの分析、必要に応じてマイグレーション追加 |

---

## 5. 差分仕様（What / How）

### 5.1 既存 DB スキーマの分析

**既存テーブル構成**:

#### ivr_flows テーブル

```sql
CREATE TABLE ivr_flows (
    id UUID NOT NULL PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    description TEXT,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

**問題点**:
- `initial_announcement_id`（初期アナウンス ID）が存在しない
- `retry_announcement_id`（リトライアナウンス ID）が存在しない
- `timeout_seconds`（タイムアウト秒数）が存在しない
- `max_retries`（最大リトライ回数）が存在しない
- `fallback_action_config`（fallback Action 設定）が存在しない

#### ivr_nodes テーブル

```sql
CREATE TABLE ivr_nodes (
    id UUID NOT NULL PRIMARY KEY,
    flow_id UUID NOT NULL REFERENCES ivr_flows(id) ON DELETE CASCADE,
    parent_id UUID REFERENCES ivr_nodes(id) ON DELETE CASCADE,
    node_type VARCHAR(20) NOT NULL,  -- 'ANNOUNCE', 'KEYPAD', 'FORWARD', 'TRANSFER', 'RECORD', 'EXIT'
    action_code VARCHAR(2),
    audio_file_url TEXT,
    tts_text TEXT,
    timeout_sec INT NOT NULL DEFAULT 10,
    max_retries INT NOT NULL DEFAULT 3,
    depth SMALLINT NOT NULL DEFAULT 0,
    exit_action VARCHAR(2) NOT NULL DEFAULT 'IE',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT chk_node_type
        CHECK (node_type IN ('ANNOUNCE', 'KEYPAD', 'FORWARD', 'TRANSFER', 'RECORD', 'EXIT')),
    CONSTRAINT chk_node_depth
        CHECK (depth >= 0 AND depth <= 3)
);
```

**特徴**:
- ノード単位で `timeout_sec`, `max_retries` を保持
- ツリー構造（parent_id でノードを階層化）
- `exit_action` で fallback を定義

#### ivr_transitions テーブル

```sql
CREATE TABLE ivr_transitions (
    id UUID NOT NULL PRIMARY KEY,
    from_node_id UUID NOT NULL REFERENCES ivr_nodes(id) ON DELETE CASCADE,
    input_type VARCHAR(20) NOT NULL,  -- 'DTMF', 'TIMEOUT', 'INVALID', 'COMPLETE'
    dtmf_key VARCHAR(5),
    to_node_id UUID REFERENCES ivr_nodes(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT chk_transition_input_type
        CHECK (input_type IN ('DTMF', 'TIMEOUT', 'INVALID', 'COMPLETE')),
    CONSTRAINT chk_dtmf_key_required
        CHECK (input_type != 'DTMF' OR dtmf_key IS NOT NULL)
);
```

**特徴**:
- DTMF 入力 → 次ノードへの遷移を定義
- TIMEOUT, INVALID 時の遷移も定義可能

---

### 5.2 シンプルな IVR の仕様（Phase 4-A スコープ）

**Phase 4-A で実装する IVR の範囲**:
- **1つの IVR フロー = ルートノード1つ（ANNOUNCE） + KEYPADノード1つ + 遷移のみ**
- ネスト IVR は「次のフロー全体に遷移」のみ（ノード単位のネストは Phase 5 以降）
- 複雑なノードツリー（複数アナウンスの階層的再生）は Phase 5 以降

**データモデル（既存スキーマ使用）**:

```
ivr_flows (id=A)
  └─ ivr_nodes (id=ROOT, flow_id=A, parent_id=NULL, node_type='ANNOUNCE', depth=0)
       └─ audio_file_url: 初期アナウンスの音声ファイル URL

  └─ ivr_nodes (id=KEYPAD, flow_id=A, parent_id=ROOT, node_type='KEYPAD', depth=1)
       └─ timeout_sec: 10
       └─ max_retries: 2  ※ アプリ既定値（DB default=3 だが converters.rs で 2 を使用）
       └─ exit_action: 'IE'  ※ Phase 5+ で使用予定、Phase 4-A では未使用

  └─ ivr_nodes (id=EXIT_VR, flow_id=A, parent_id=KEYPAD, node_type='EXIT', depth=2)
       └─ action_code: 'VR'  ※ DTMF='1' の遷移先（ボイスボット起動）
       └─ tts_text: NULL  ※ VR はメタデータ不要

  └─ ivr_nodes (id=EXIT_VM, flow_id=A, parent_id=KEYPAD, node_type='EXIT', depth=2)
       └─ action_code: 'VM'  ※ DTMF='2' の遷移先（留守番電話）
       └─ audio_file_url: 'https://...留守番電話案内音声'  ※ announcement_id から解決
       └─ tts_text: NULL  ※ VM は audio_file_url 使用

  └─ ivr_nodes (id=EXIT_IV, flow_id=A, parent_id=KEYPAD, node_type='EXIT', depth=2)
       └─ action_code: 'IV'  ※ DTMF='3' の遷移先（サブ IVR へ）
       └─ tts_text: '{"ivrFlowId":"<uuid>"}'  ※ JSON メタデータ

  └─ ivr_nodes (id=FALLBACK_VR, flow_id=A, parent_id=KEYPAD, node_type='EXIT', depth=2)
       └─ action_code: 'VR'  ※ TIMEOUT/INVALID 時の fallback 先
       └─ tts_text: NULL  ※ VR はメタデータ不要

  └─ ivr_transitions (from_node_id=KEYPAD, input_type='DTMF', dtmf_key='1', to_node_id=EXIT_VR)
  └─ ivr_transitions (from_node_id=KEYPAD, input_type='DTMF', dtmf_key='2', to_node_id=EXIT_VM)
  └─ ivr_transitions (from_node_id=KEYPAD, input_type='DTMF', dtmf_key='3', to_node_id=EXIT_IV)
  └─ ivr_transitions (from_node_id=KEYPAD, input_type='TIMEOUT', dtmf_key=NULL, to_node_id=FALLBACK_VR)
  └─ ivr_transitions (from_node_id=KEYPAD, input_type='INVALID', dtmf_key=NULL, to_node_id=FALLBACK_VR)
```

**使用パターン**:

1. **初期アナウンス再生**: ルートノード（parent_id IS NULL）の `audio_file_url` を再生
2. **DTMF 入力待ち**: KEYPAD ノードの `timeout_sec` まで待機
3. **ルート評価**: `ivr_transitions` で DTMF に対応する遷移を検索
4. **アクション実行**:
   - `to_node_id` で指定された EXIT ノードへ遷移
   - EXIT ノードの `action_code` を実行（VR, VM, AN など）
5. **リトライ処理**: TIMEOUT / INVALID 時に `max_retries` まで再試行
6. **fallback Action**: リトライ上限到達時に TIMEOUT/INVALID 遷移先の EXIT ノードの `action_code` を実行

**Phase 4-A の重要な設計決定**:
- **toNodeId は常に EXIT ノードを指す**: NULL は使用しない（converters.rs 実装に準拠）
- **fallback は TIMEOUT/INVALID 遷移で実装**: exit_action フィールドは Phase 5+ で使用予定
- **max_retries 既定値は 2**: Frontend (ivr-flows.ts) と Backend (converters.rs) で統一
- **EXIT ノードの tts_text にメタデータを保存**:
  - actionCode='IV' の場合: `{"ivrFlowId":"<uuid>"}` 形式の JSON（recordingEnabled は任意、未指定時はデフォルト適用）
  - actionCode='VB' の場合: `{"scenarioId":"<uuid>","recordingEnabled":true,"includeAnnouncement":true}` 形式の JSON（welcomeAnnouncementId は audio_file_url に変換済み）
  - その他の actionCode: NULL（メタデータ不要）

---

### 5.3 contract.md の更新内容

**修正箇所**: contract.md §3.6-3.8

**変更内容**:

#### IvrFlow（変更なし）

```typescript
IvrFlow {
  id: UUID
  name: string
  description: string | null
  isActive: boolean
  folderId: UUID | null
  nodes: IvrNode[]  // 展開時のみ
  createdAt: ISO8601
  updatedAt: ISO8601
}
```

#### IvrNode（仕様明確化）

**Phase 4-A で使用するフィールド**:

```typescript
IvrNode {
  id: UUID
  flowId: UUID
  parentId: UUID | null  // NULL = ルートノード（初期アナウンス）
  nodeType: "ANNOUNCE" | "KEYPAD" | "EXIT"  // Phase 4-A では ANNOUNCE/KEYPAD/EXIT を使用
  audioFileUrl: string | null  // ANNOUNCE の場合、初期アナウンス音声。EXIT の場合、announcement_id から解決した音声 URL
  ttsText: string | null  // EXIT ノードの場合、JSON メタデータ（ivrFlowId, scenarioId など）。詳細は下記参照
  timeoutSec: number  // KEYPAD の場合、DTMF 入力タイムアウト
  maxRetries: number  // KEYPAD の場合、リトライ上限（既定値=2）
  exitAction: string  // Phase 5+ で使用予定（Phase 4-A では未使用）
  actionCode: string | null  // EXIT ノードの場合、実行する ActionCode (VR, VM, AN, IV, VB など)
  transitions: IvrTransition[]  // 展開時のみ
}
```

**EXIT ノードの ttsText 使用方法**:
- `actionCode='IV'` の場合: `{"ivrFlowId":"<uuid>"}` （recordingEnabled は任意、未指定時はデフォルト適用）
- `actionCode='VB'` の場合: `{"scenarioId":"<uuid>","recordingEnabled":true,"includeAnnouncement":true}` （welcomeAnnouncementId は audio_file_url に変換済み、tts_text には保存しない）
- その他の actionCode: `null`（メタデータ不要）

**Phase 5 以降で使用するフィールド**:
- `ttsText`: TTS テキスト（Phase 5）
- `depth`: ノード階層深度（Phase 5）
- `exitAction`: exit_action フィールド（Phase 5）

#### IvrTransition（仕様明確化）

**Phase 4-A で使用するパターン**:

```typescript
IvrTransition {
  id: UUID
  fromNodeId: UUID  // KEYPAD ノードの ID
  inputType: "DTMF" | "TIMEOUT" | "INVALID"
  dtmfKey: string | null  // inputType='DTMF' の場合のみ
  toNodeId: UUID  // 遷移先 EXIT ノードの ID（NULL は使用しない）
}
```

**使用例（Phase 4-A）**:

```json
{
  "id": "transition-001",
  "fromNodeId": "keypad-node-001",
  "inputType": "DTMF",
  "dtmfKey": "1",
  "toNodeId": "exit-node-vr-001"  // ← EXIT ノードへ遷移（action_code='VR' を実行）
}
```

---

### 5.4 DB マイグレーションの必要性判定

**結論**: **Phase 4-A では DB マイグレーション不要**

**理由**:
1. 既存スキーマ（ivr_nodes + ivr_transitions）でシンプルな IVR を実現可能
2. ノード単位の `timeout_sec`, `max_retries` を使用（fallback は TIMEOUT/INVALID 遷移 + EXIT ノードの action_code で実装、exit_action は Phase 5+ で使用）
3. `ivr_flows` テーブルにカラム追加は不要（ノードレベルで管理）

**Phase 5 以降での検討事項**:
- `ivr_flows` に `default_timeout_sec`, `default_max_retries` を追加する可能性
- パフォーマンス最適化のため、フロー全体の設定を `ivr_flows` に持たせる選択肢

---

### 5.5 Frontend PoC との整合性確認

**Frontend PoC (#132, #134) の依存関係**:

1. **Issue #132**: Frontend 着信アクション設定画面
   - `ivr_flows` テーブルの `id`, `name` を参照
   - ActionCode=IV 設定時に `ivr_flow_id` を保存

2. **Issue #134**: Frontend IVR フロー設定画面
   - Frontend は `IvrFlowDefinition` JSON を管理（ivr-flows.ts）
   - Backend の `converters.rs` が JSON を `ivr_nodes`, `ivr_transitions` へ変換・保存
   - UI ではルート設定（routes）と fallback アクションを編集

**整合性確認**:
- ✅ Phase 4-A の仕様（シンプルな IVR）は既存スキーマを使用
- ✅ Frontend が作成した IvrFlowDefinition を Backend converters.rs が ivr_nodes/ivr_transitions へ変換
- ✅ 変換後の DB 構造を Phase 4-A で実行可能
- ✅ DB マイグレーション不要のため、Frontend PoC への影響なし

---

## 6. 受入条件（Acceptance Criteria）

### AC-1: 既存 DB スキーマの分析完了
- [x] ivr_flows, ivr_nodes, ivr_transitions の構造が明確（§5.1 で分析完了）
- [x] Phase 4-A で使用するフィールドが特定されている（§5.2 で定義）
- [x] Phase 5 以降で使用するフィールドが区別されている（§5.3 で明確化）

### AC-2: シンプルな IVR の仕様確定
- [x] 1アナウンス + DTMFルートの仕様が明確（§5.2 で定義）
- [x] ルートノード（ANNOUNCE） + KEYPADノードの使用方法が定義されている（§5.2）
- [x] ivr_transitions の使用パターンが明確（§5.2, §5.3）

### AC-3: contract.md の更新完了
- [x] IvrFlow の Phase 4-A 使用フィールドが明記されている
- [x] IvrNode の Phase 4-A シンプル IVR パターンが明記されている
- [x] IvrTransition の使用パターンが明記されている
- [x] Phase 5 以降のフィールドが区別されている

### AC-4: Frontend PoC との整合性確認
- [x] Frontend PoC (#132, #134) への影響がないことを確認（§4 で整合性確認）
- [x] 既存 IVR フローが Phase 4-A で実行可能（§5.2 で確認）

### AC-5: DB マイグレーション判定完了
- [x] Phase 4-A での DB マイグレーション不要を確認（§5.4 で結論）
- [x] Phase 5 以降での検討事項を文書化（§5.4 で記載）

---

## 7. 設計決定事項（Design Decisions）

### D-01: 既存 DB スキーマを使用（マイグレーション不要）

**決定**: Phase 4-A では既存 DB スキーマ（ivr_nodes + ivr_transitions）をそのまま使用し、DB マイグレーションは実施しない

**理由**:
- 既存スキーマでシンプルな IVR（1アナウンス + DTMFルート）を実現可能
- ノード単位の `timeout_sec`, `max_retries` を活用（fallback は TIMEOUT/INVALID 遷移 + EXIT ノードの action_code で実装、exit_action は Phase 5+ で使用）
- Frontend PoC (#132, #134) への影響を最小化

**代替案**:
- `ivr_flows` にカラム追加（`initial_announcement_id`, `timeout_seconds` など）
- 却下理由: 既存スキーマで実現可能であり、Phase 4-A のスコープを拡大するメリットが少ない

### D-02: Phase 4-A はシンプルな IVR のみをサポート

**決定**: Phase 4-A では「ルートノード1つ（ANNOUNCE） + KEYPADノード1つ + 遷移」のみをサポート

**理由**:
- Phase 4-A は DB スキーマ設計の確定が目的
- IVR エンジン実装（Phase 4-B）の範囲を明確化
- 複雑なノードツリー（複数アナウンスの階層的再生）は Phase 5 以降で段階的に実装

**Phase 5 以降で実装予定**:
- 複数アナウンスノードの階層的再生
- ノード単位のネスト IVR
- 親ノードへの戻り処理

### D-03: contract.md で Phase 4-A / Phase 5 の使用フィールドを区別

**決定**: contract.md §3.7（IvrNode）で Phase 4-A と Phase 5 以降の使用フィールドを明記

**理由**:
- Phase 4-B（IVR エンジン実装）で参照するフィールドを明確化
- Frontend PoC との整合性を確保
- 段階的な実装を可能にする

---

## 8. リスク・制約

### 8.1 リスク

| リスク | 影響度 | 発生確率 | 対策 |
|--------|--------|---------|------|
| Frontend PoC が想定外の IVR フローを作成 | 中 | 中 | Phase 4-B 実装時に検証、必要に応じて Frontend に制約追加 |
| Phase 5 で DB マイグレーションが必要になる | 低 | 中 | Phase 5 計画時に再評価、マイグレーション戦略を検討 |

### 8.2 制約

| 制約 | 理由 | 代替案 |
|------|------|--------|
| Phase 4-A ではシンプルな IVR のみ | スコープ限定、段階的実装 | Phase 5 で複雑な IVR を実装 |
| DB マイグレーション不実施 | 既存スキーマで実現可能 | Phase 5 で必要に応じて実施 |

---

## 9. 参照

| ドキュメント | セクション | 内容 |
|-------------|-----------|------|
| [STEER-137](STEER-137_backend-integration-strategy.md) | §5.2.4.4 | Issue #142（Phase 4）の定義 |
| [STEER-141](STEER-141_actioncode-phase3.md) | - | Phase 3 実装（IV 基盤） |
| [contract.md](contract.md) | §3.6-3.8 | IVR フローの仕様 |
| [migrations/20260206000001_create_ivr_flows.sql](../virtual-voicebot-backend/migrations/20260206000001_create_ivr_flows.sql) | - | ivr_flows テーブル定義 |
| [migrations/20260206000002_create_ivr_nodes.sql](../virtual-voicebot-backend/migrations/20260206000002_create_ivr_nodes.sql) | - | ivr_nodes テーブル定義 |
| [migrations/20260206000003_create_ivr_transitions.sql](../virtual-voicebot-backend/migrations/20260206000003_create_ivr_transitions.sql) | - | ivr_transitions テーブル定義 |

---

## 10. 後続 Issue

**Issue #156: Backend IVR 実行エンジン実装（Phase 4-B）**

**URL**: https://github.com/MasanoriSuda/virtual_voicebot/issues/156

**スコープ**:
- 確定した DB スキーマに基づく IVR エンジン実装
- DTMF 入力処理（既存 `SessionMediaIn::Dtmf` 使用）
- アナウンス再生統合（最低限の playback_service 連携）
- タイムアウト / リトライ処理
- fallback Action 実行

**依存**: Issue #142（Phase 4-A）完了後

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-08 | 初版作成（Draft）：Phase 4 を Phase 4-A（DB設計）+ Phase 4-B（実装）に分割 | Claude Code (claude-sonnet-4-5) |
| 2026-02-08 | contract.md §3.6-3.8 更新完了：Phase 4-A vs Phase 5+ の範囲明確化 | Claude Code (claude-sonnet-4-5) |
| 2026-02-08 | Codex Review #1 対応（重大2件、中2件、軽1件修正）：toNodeId=NULL 削除し EXIT ノード方式に統一、fallback を TIMEOUT/INVALID 遷移方式に修正、Frontend 連携説明を IvrFlowDefinition JSON 管理方式に修正、max_retries 既定値を 2 に統一、相対リンク修正 | Claude Code (claude-sonnet-4-5) |
| 2026-02-08 | Codex Review #2 対応（軽1件修正）：exit_action の記述を Phase 5+ 使用予定に統一 | Claude Code (claude-sonnet-4-5) |
| 2026-02-08 | Codex Review #3 対応（重大1件、中1件修正）：EXIT ノード tts_text メタデータ仕様化（ivrFlowId, scenarioId など JSON 形式）、contract.md toNodeId Phase 4-A 注記修正（常に非NULL） | Claude Code (claude-sonnet-4-5) |
| 2026-02-08 | Codex Review #4 承認（中2件修正）：VB welcomeAnnouncementId を audio_file_url 変換のみ（tts_text 非保存）、IV/VB recordingEnabled を任意化、実装可能判定 | Claude Code (claude-sonnet-4-5) |
