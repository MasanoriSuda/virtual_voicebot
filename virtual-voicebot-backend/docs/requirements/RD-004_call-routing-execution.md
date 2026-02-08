<!-- SOURCE_OF_TRUTH: 着信ルール評価・IVR実行の要件定義 -->
# RD-004: 着信ルール評価・IVR実行の要件定義

> Backend における着信時のルール評価エンジン、ActionCode 実行、IVR 実行エンジン、Frontend → Backend 設定同期の要件を定義する

| 項目 | 値 |
|------|-----|
| ID | RD-004 |
| ステータス | Draft |
| 作成日 | 2026-02-08 |
| 最終更新 | 2026-02-08 |
| 関連Issue | #138 |
| 対応STEER | STEER-137 |
| 対応BD | BD-004 |

---

## 1. 概要

### 1.1 目的

Frontend PoC（STEER-132/134）で実装された着信アクション設定・IVR フロー定義を Backend で実行するための要件を定義する。

### 1.2 スコープ

本 RD では以下を対象とする：

- **着信ルール評価エンジン**: 3段階評価（番号完全一致 → 番号グループ → カテゴリ → デフォルト）
- **ActionCode 実行**: VR, IV, VM, BZ, NR, AN の実行
- **IVR 実行エンジン**: DTMF 入力待ち、タイムアウト、リトライ、fallback
- **Frontend → Backend 設定同期**: Serversync による設定の Pull + 変換・保存

### 1.3 前提条件

- Frontend PoC（STEER-132/134）が完成している
- STEER-137（Backend 連携統合戦略）が承認されている
- BD-004（着信ルーティング DB 設計）が存在する
- STEER-096（Serversync）が存在する

---

## 2. 機能要件（FR）

### FR-1: 着信ルール評価エンジン

#### FR-1.1: 評価順序（3段階 + 非通知 + デフォルト）

着信時に発信者番号（Caller ID）に基づいて、以下の順序でルールを評価し、最初にマッチしたアクションを実行する。

```
着信
  ↓
【0】非通知判定
  - Caller ID が空 or "anonymous" → anonymousAction を適用
  ↓ (非通知でない場合)
【1】番号完全一致（registered_numbers テーブル）
  - registered_numbers に Caller ID が存在 → 該当レコードの action_code / action_config を適用
  ↓ (Miss)
【2】番号グループ評価（call_action_rules テーブル）
  - call_action_rules を priority 昇順で評価
  - 各ルールの caller_group_id に紐づく番号一覧（registered_numbers.group_id で絞り込み）に Caller ID が含まれるか判定
  - マッチした場合、該当ルールの action_type / action_config を適用
  ↓ (Miss)
【3】カテゴリ評価（routing_rules テーブル）
  - Caller ID を4カテゴリ（spam / registered / unknown / anonymous）に分類
  - routing_rules を priority 昇順で評価し、カテゴリが一致するルールを適用
  ↓ (Miss)
【4】デフォルトアクション
  - system_settings.extra の defaultAction を適用
```

**未設定時のフォールバック**:
- defaultAction が system_settings.extra に存在しない場合、以下のデフォルト値を使用：
  ```json
  {
    "actionType": "allow",
    "actionConfig": { "actionCode": "VR" }
  }
  ```

**評価結果のログ出力**（NFR-1 参照）:
- 各段階で評価したルール ID、マッチ結果、適用したアクション
- 最終的にどの段階でどのルールが適用されたか（トレーサビリティ）

#### FR-1.2: 番号正規化

発信者番号（Caller ID）と DB 内の電話番号は **E.164 形式**（`+819012345678`）で比較する。

- ハイフン `-`、空白、括弧 `()` を除去
- PoC では E.164 変換（+81 → 0）は行わない（将来的に対応）

#### FR-1.3: 非通知着信の処理

- Caller ID が空、"anonymous"、"withheld" の場合、**anonymousAction** を適用
- anonymousAction は system_settings.extra (JSONB) に保存
- 番号グループ・カテゴリ評価はスキップ

**未設定時のフォールバック**:
- anonymousAction が system_settings.extra に存在しない場合、以下のデフォルト値を使用：
  ```json
  {
    "actionType": "deny",
    "actionConfig": { "actionCode": "BZ" }
  }
  ```

#### FR-1.4: registered_numbers の group_id 参照

Frontend の CallerGroup（番号グループ）は以下のように Backend DB に展開される：

- CallerGroup.id → registered_numbers.group_id（UUID、不変ID）
- CallerGroup.name → registered_numbers.group_name（VARCHAR、表示名）

> **前提条件**: BD-004（着信ルーティング DB 設計）に `registered_numbers` テーブルへの以下のカラム追加が必要：
> - `group_id UUID` - 番号グループの不変ID（CallerGroup.id に対応）
> - `group_name VARCHAR(255)` - 番号グループの表示名（CallerGroup.name に対応）
>
> これらのカラムは Issue #138 の DDL（マイグレーションファイル）で追加される。

**ルール評価時の挙動**:
- call_action_rules.caller_group_id と registered_numbers.group_id を照合
- グループ内の番号一覧を取得し、Caller ID が含まれるか判定

**CallerGroup のリネーム時の挙動**:
- Frontend で CallerGroup.name を変更した場合、registered_numbers.group_name も更新される
- group_id は不変のため、call_action_rules との照合に影響しない

**削除済みグループの挙動**:
- Frontend で CallerGroup が削除された場合、registered_numbers.group_id は NULL になる
- call_action_rules.caller_group_id が NULL 参照の場合、該当ルールはスキップされる（ログに警告出力）

---

### FR-2: ActionCode 実行

#### FR-2.1: 対象 ActionCode（MVP）

MVP では以下の ActionCode を実装する：

| ActionCode | 名称 | 説明 | 優先度 |
|-----------|------|------|--------|
| **VB** | Voicebot | AI応答（ボイスボット）開始（録音なし） | P0 |
| **VR** | Voicebot+Record | AI応答（ボイスボット）開始（録音あり） | P0 |
| **IV** | IVR | IVR フローへ移行 | P1 |
| **VM** | Voicemail | 留守番電話 | P1 |
| **BZ** | Busy | 話中応答 | P1 |
| **NR** | No Response | 応答なし（コール音のみ） | P1 |
| **AN** | Announce | アナウンス再生（録音なし） | P1 |

**将来対応**（MVP 外）:
- RJ（Reject 即時拒否）
- AR（Announce+Record）

> **注**: VB は BD-004 および contract.md で定義済みであり、現行実装でも使用されているため、MVP 対象とする。

#### FR-2.2: action_config のスキーマ

各 ActionCode の詳細設定は **action_config** (JSONB) に保存する。

**VB（Voicebot）**:
```json
{
  "actionCode": "VB"
}
```

**VR（Voicebot+Record）**:
```json
{
  "actionCode": "VR",
  "recordingEnabled": true,
  "announceEnabled": false,
  "announcementId": null
}
```

**IV（IVR）**:
```json
{
  "actionCode": "IV",
  "ivrFlowId": "uuid-v7",
  "includeAnnouncement": true
}
```

**VM（Voicemail）**:
```json
{
  "actionCode": "VM",
  "announcementId": "uuid-v7"
}
```

**BZ（Busy）/ NR（No Response）**:
```json
{
  "actionCode": "BZ"
}
```

**AN（Announce）**:
```json
{
  "actionCode": "AN",
  "announcementId": "uuid-v7"
}
```

#### FR-2.3: action_config のバージョニング方針

**設計方針**:
- action_config は JSONB で柔軟に拡張可能
- 既存フィールドの削除は行わない（後方互換性維持）
- 新規フィールド追加時は **デフォルト値を設定**し、既存データとの互換性を保つ

**バージョン管理**（将来対応）:
- action_config に `_version: "1.0"` のようなフィールドを追加することで、スキーマバージョンを明示
- MVP では version フィールドなし（全て v1.0 として扱う）

**後方互換性の例**:
- `recordingEnabled` フィールドを追加した場合、既存データは `recordingEnabled: false` として扱う
- `announcementId` が null の場合、アナウンス再生をスキップ

---

### FR-3: IVR 実行エンジン

#### FR-3.1: IVR フローの実行フロー

IVR フローは以下の手順で実行される：

```
1. IVR 開始
   - ivrFlowId で ivr_flows テーブルを検索
   - parent_id IS NULL のノードをルートノードとして取得

2. アナウンス再生
   - ルートノードの audio_file_url または tts_text を再生

3. DTMF 入力待ち
   - timeout_sec 秒間、DTMF 入力を待機
   - 有効キー入力 → ivr_transitions で遷移先ノードを検索、次のノードへ
   - 無効キー入力 → invalidInputAnnouncementId を再生（あれば）、リトライカウント++
   - タイムアウト → timeoutAnnouncementId を再生（あれば）、リトライカウント++

4. リトライ判定
   - リトライカウントが max_retries を超過 → fallbackAction を実行
   - 超過していない → ステップ2へ戻る（アナウンス再生からやり直し）

5. 終端ノード到達
   - IVR 終了、次のアクションへ移行
```

#### FR-3.2: IVR のタイムアウト・リトライ上限

**論理的な管理単位**: IVR フロー（Frontend の `IvrFlowDefinition`）

以下のパラメータを Frontend で設定し、Backend に同期する：

| パラメータ | 型 | デフォルト値 | 説明 |
|-----------|-----|------------|------|
| timeoutSec | INT | 10 | DTMF 入力待ち時間（秒） |
| maxRetries | INT | 2 | リトライ上限（無効入力/タイムアウトの合計） |

**物理的な保存先**（BD-004 参照）:
- `ivr_nodes` テーブルの node_type="KEYPAD" のノードに保存
  - `ivr_nodes.timeout_sec` - DTMF 入力待ち時間
  - `ivr_nodes.max_retries` - リトライ上限

**MVP での制約**:
- timeout_sec: 10秒固定（将来的に IVR ごとに設定可能にする）
- max_retries: 2回固定（将来的に IVR ごとに設定可能にする）

#### FR-3.3: IVR ネスト制限

- IVR フローから別の IVR フローへの遷移（ネスト）は **最大3層** まで
- depth チェックは Serversync の設定 Pull 時に実施（循環参照検出も同時に実施）
- depth > 3 の場合、エラーログ出力 + fallbackAction を実行

#### FR-3.4: IVR 実行結果のログ出力

以下の情報をログ出力する（NFR-1 参照）：

- IVR フロー ID、ノード ID
- DTMF 入力値、タイムアウト/無効入力の発生
- リトライカウント
- fallback 実行理由
- 遷移先ノード ID

---

### FR-4: Frontend → Backend 設定同期

#### FR-4.1: 同期方式（Serversync による Pull）

STEER-096（Serversync）の方針に従い、Backend の Serversync が Frontend から設定を Pull する。

**同期の位置づけ**:
- 本同期方式は **移行期間の暫定モード** である
- Frontend PoC の JSON ファイルを Backend DB に同期することで、Frontend 設定を Backend で実行可能にする
- 将来的には Backend DB → Frontend DB の一方向同期（Transactional Outbox）に移行し、Frontend JSON ファイルは廃止される（STEER-137 §5.2.2 参照）

**同期フロー**:
```
1. Serversync が定期的（30秒ごと）に Frontend の GET API を呼び出し
   - GET /api/number-groups    （番号グループ取得）
   - GET /api/call-actions      （着信アクションルール取得）
   - GET /api/ivr-flows         （IVR フロー定義取得）

2. Frontend は JSON ファイルから読み取って返す
   - number-groups.json → GET /api/number-groups
   - call-actions.json → GET /api/call-actions
   - ivr-flows.json → GET /api/ivr-flows

3. Backend Serversync が取得したデータを変換・保存
   - CallerGroup[] → registered_numbers テーブルに展開
   - IncomingRule[] → call_action_rules テーブルに保存
   - IvrFlowDefinition[] → ivr_nodes + ivr_transitions に変換
```

#### FR-4.2: Frontend API 仕様

##### GET /api/number-groups（Frontend 側）

**Request**:
```
GET /api/number-groups
```

**Response**:
```json
{
  "ok": true,
  "callerGroups": [
    {
      "id": "uuid-v7",
      "name": "スパム",
      "description": "迷惑電話",
      "phoneNumbers": ["+819012345678", "+819087654321"],
      "createdAt": "2026-02-08T00:00:00Z",
      "updatedAt": "2026-02-08T00:00:00Z"
    }
  ]
}
```

##### GET /api/call-actions（Frontend 側）

**Request**:
```
GET /api/call-actions
```

**Response**:
```json
{
  "ok": true,
  "rules": [
    {
      "id": "uuid-v7",
      "name": "スパム拒否",
      "callerGroupId": "uuid-v7",
      "actionType": "deny",
      "actionConfig": {
        "actionCode": "BZ"
      },
      "isActive": true,
      "createdAt": "2026-02-08T00:00:00Z",
      "updatedAt": "2026-02-08T00:00:00Z"
    }
  ],
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

##### GET /api/ivr-flows（Frontend 側）

**Request**:
```
GET /api/ivr-flows
```

**Response**:
```json
{
  "ok": true,
  "flows": [
    {
      "id": "uuid-v7",
      "name": "メインメニュー",
      "description": "受付振り分け",
      "isActive": true,
      "announcementId": "uuid-v7",
      "timeoutSec": 10,
      "maxRetries": 2,
      "invalidInputAnnouncementId": null,
      "timeoutAnnouncementId": null,
      "routes": [
        {
          "dtmfKey": "1",
          "label": "営業",
          "destination": {
            "actionCode": "VR"
          }
        }
      ],
      "fallbackAction": {
        "actionCode": "VR"
      },
      "createdAt": "2026-02-08T00:00:00Z",
      "updatedAt": "2026-02-08T00:00:00Z"
    }
  ]
}
```

**認証**（MVP）:
- MVP では認証なし（Backend と Frontend はローカルネットワーク前提）
- 将来的に `X-Api-Key` ヘッダーでの認証を追加可能（拡張余地を残す）

#### FR-4.3: Backend Serversync の変換処理

##### CallerGroup → registered_numbers 変換

Frontend の `CallerGroup` を Backend の `registered_numbers` テーブルに展開する。

**変換ロジック**:
```
1. callerGroups[] の各 CallerGroup について：
   a. phoneNumbers[] の各番号を registered_numbers に INSERT または UPDATE
      - phone_number: E.164 正規化後の番号
      - group_id: CallerGroup.id（UUID、不変ID）
      - group_name: CallerGroup.name（VARCHAR、表示名）
      - category: "general"（DB デフォルト値、CHECK 制約で 'registered' は許可されていない）
      - action_code: "VR"（DB デフォルト値、NOT NULL 制約あり）
      - recording_enabled: true（DB デフォルト値）
      - announce_enabled: true（DB デフォルト値）
   b. 既に存在する番号の場合、group_id / group_name を UPDATE
   c. Frontend から削除された番号の場合、group_id / group_name を NULL に UPDATE

2. 削除済み CallerGroup の処理：
   - Frontend の callerGroups[] に存在しない group_id を持つ registered_numbers のレコードは、
     group_id / group_name を NULL に UPDATE
```

> **注**: DB スキーマ（20260206000005_create_registered_numbers.sql）の制約に従う。
> category の CHECK 制約は ('vip', 'customer', 'partner', 'general') のみ許可。
> action_code は NOT NULL 制約があり、NULL は許可されていない。

##### IncomingRule → call_action_rules 変換

Frontend の `IncomingRule` を Backend の `call_action_rules` テーブルに保存する。

**変換ロジック**:
```
1. rules[] の各 IncomingRule について：
   a. call_action_rules に INSERT または UPDATE
      - id: IncomingRule.id
      - name: IncomingRule.name
      - caller_group_id: IncomingRule.callerGroupId（FK なし、NULL 許容）
      - action_type: IncomingRule.actionType
      - action_config: IncomingRule.actionConfig（JSONB）
      - priority: 配列のインデックス（0, 1, 2, ...）
      - is_active: IncomingRule.isActive

2. Frontend から削除されたルールの処理：
   - Frontend の rules[] に存在しない id を持つ call_action_rules のレコードは DELETE
```

##### IvrFlowDefinition → ivr_nodes/ivr_transitions 変換

Frontend の `IvrFlowDefinition`（フローベース）を Backend の `ivr_nodes` + `ivr_transitions`（ノードベース）に変換する。

**変換ロジック**（概要）:
```
1. IvrFlowDefinition について：
   a. ivr_flows に INSERT または UPDATE
      - id: IvrFlowDefinition.id
      - name: IvrFlowDefinition.name
      - description: IvrFlowDefinition.description
      - is_active: IvrFlowDefinition.isActive

   b. ルートノード（ANNOUNCE）を作成
      - node_type: "ANNOUNCE"
      - audio_file_url: announcementId に対応する音声ファイル URL
      - parent_id: NULL（ルート）

   c. DTMF 入力待ちノード（KEYPAD）を作成
      - node_type: "KEYPAD"
      - timeout_sec: IvrFlowDefinition.timeoutSec
      - max_retries: IvrFlowDefinition.maxRetries

   d. routes[] の各ルートについて遷移（ivr_transitions）を作成
      - from_node_id: KEYPAD ノード
      - input_type: "DTMF"
      - dtmf_key: route.dtmfKey
      - to_node_id: destination に応じた終端ノード（FORWARD / RECORD / EXIT）

   e. fallbackAction を終端ノードとして作成

2. 循環参照検出・depth チェック（詳細は BD-004 参照）
```

#### FR-4.4: Serversync の取得失敗時の挙動

**設計方針**:
- Serversync が Frontend からの設定取得に失敗した場合、**前回取得分で動作継続**
- 起動時に取得必須ではない（Backend DB に既存設定があれば、それを使用）

**挙動の詳細**:
| ケース | 挙動 |
|--------|------|
| Serversync 起動時、Frontend が停止中 | 警告ログ出力、前回取得分（Backend DB）で動作継続 |
| Serversync 定期 Pull 時、Frontend が停止中 | 警告ログ出力、前回取得分で動作継続 |
| Backend DB に設定が存在しない（初回起動） | エラーログ出力、デフォルトアクション（system_settings.defaultAction）のみで動作 |
| Frontend API が 404 / 500 を返す | エラーログ出力、前回取得分で動作継続、リトライ（次回 Pull 時） |

**ログ出力**（NFR-1 参照）:
- 取得成功: INFO レベル、取得したルール数・IVR フロー数
- 取得失敗: WARN レベル、エラー理由、前回取得分で動作継続中
- 初回起動時に取得失敗: ERROR レベル、デフォルトアクションのみで動作中

---

## 3. 非機能要件（NFR）

### NFR-1: ログ・監視

#### NFR-1.1: ルール評価結果のログ出力

着信ごとに以下の情報をログ出力する（INFO レベル）：

```
[着信] Caller ID: +819012345678
[評価] 段階0（非通知判定）: スキップ
[評価] 段階1（番号完全一致）: Miss
[評価] 段階2（番号グループ）: ルール ID=xxx, 優先度=0, マッチ, アクション=BZ
[決定] アクション: BZ, ルール: xxx, 理由: 番号グループ評価
```

**出力項目**:
- Caller ID（E.164 正規化後）
- 評価段階（0〜4）
- 各段階での評価結果（Hit / Miss / Skip）
- マッチしたルール ID、優先順位
- 適用したアクション（ActionCode + action_config）
- fallback 理由（該当する場合）

#### NFR-1.2: IVR 実行フローのトレース

IVR 実行中に以下の情報をログ出力する（INFO レベル）：

```
[IVR開始] フロー ID: xxx, ノード ID: xxx
[アナウンス] 音声ファイル: xxx.wav, 再生完了
[DTMF入力待ち] タイムアウト: 10秒
[DTMF入力] キー: 1, 遷移先ノード ID: xxx
[IVR終了] フロー ID: xxx, 終端アクション: VR
```

**出力項目**:
- IVR フロー ID、ノード ID
- アナウンス再生（音声ファイル名、TTS テキスト）
- DTMF 入力値、タイムアウト/無効入力の発生
- リトライカウント
- fallback 実行理由
- 遷移先ノード ID、終端アクション

#### NFR-1.3: Serversync のログ出力

Serversync の設定 Pull 時に以下の情報をログ出力する：

```
[Serversync] Frontend から設定取得開始
[Serversync] GET /api/number-groups: 成功, グループ数=3
[Serversync] GET /api/call-actions: 成功, ルール数=5
[Serversync] GET /api/ivr-flows: 成功, フロー数=2
[Serversync] Backend DB に保存完了
```

**出力項目**:
- 取得開始・完了のタイムスタンプ
- 取得したルール数、グループ数、IVR フロー数
- 取得失敗時のエラー理由
- 前回取得分で動作継続中の警告

### NFR-2: パフォーマンス

#### NFR-2.1: ルール評価の応答時間

- ルール評価（段階0〜4）は **100ms 以内** に完了すること
- DB クエリの最適化（インデックス、クエリプラン）を実施

#### NFR-2.2: Serversync の同期頻度

- Serversync の設定 Pull は **30秒ごと** に実行（デフォルト）
- 設定変更後、最大30秒で Backend に反映される

### NFR-3: 可用性

#### NFR-3.1: Serversync 停止時の挙動

- Serversync 停止中でも、Backend は前回取得分の設定で動作継続
- 着信処理への影響なし

#### NFR-3.2: Frontend 停止時の挙動

- Frontend 停止中でも、Backend は前回取得分の設定で動作継続
- Serversync は警告ログ出力、次回 Pull 時にリトライ

---

## 4. 制約事項

### 4.1 MVP 制約

| 項目 | MVP での扱い |
|------|-------------|
| ActionCode | VB, VR, IV, VM, BZ, NR, AN を実装（RJ, AR は将来） |
| IVR タイムアウト | 10秒固定（将来的に IVR ごとに設定可能） |
| IVR リトライ上限 | 2回固定（将来的に IVR ごとに設定可能） |
| IVR ネスト深度 | 最大3層（将来的に拡張可能） |
| Frontend API 認証 | なし（将来的に X-Api-Key 対応） |
| E.164 変換 | なし（+81 → 0 の変換は将来対応） |

### 4.2 既存機能との互換性

- BD-004 で定義された発信者分類（4カテゴリ）は維持
- STEER-096 で定義された Serversync の仕組みを活用
- contract.md v2 の SoT 原則（Backend DB が SoT）を厳守

---

## 5. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #138 | RD-004 | 起票 |
| STEER-137 | RD-004 | 要件の根拠 |
| STEER-132 | RD-004 | Frontend PoC（着信アクション） |
| STEER-134 | RD-004 | Frontend PoC（IVR フロー） |
| BD-004 | RD-004 | 設計の根拠（発信者分類、ActionCode、DB テーブル） |
| STEER-096 | RD-004 | 同期方式の根拠 |
| RD-004 | DD-004-XX（新規） | 詳細設計（ルール評価エンジン、IVR 実行エンジン、Serversync） |

---

## 6. 受入条件（Acceptance Criteria）

### AC-1: 着信ルール評価エンジン
- [ ] 3段階評価（番号完全一致 → 番号グループ → カテゴリ → デフォルト）が正しく動作する
- [ ] 非通知着信が anonymousAction で処理される
- [ ] 評価結果がログ出力される（段階、マッチルール、優先順位、fallback理由）

### AC-2: ActionCode 実行
- [ ] VB, VR, IV, VM, BZ, NR, AN が正しく実行される
- [ ] action_config の JSONB が正しく解釈される
- [ ] 将来の ActionCode 追加に対応できる（バージョニング方針）

### AC-3: IVR 実行エンジン
- [ ] DTMF 入力待ち、タイムアウト、無効入力が正しく処理される
- [ ] リトライカウント + fallback が正しく動作する
- [ ] IVR ネスト（depth ≤ 3）が正しく動作する
- [ ] IVR 実行フローがログ出力される

### AC-4: Frontend → Backend 設定同期
- [ ] Serversync が Frontend から設定を Pull できる
- [ ] CallerGroup → registered_numbers 変換が正しく動作する
- [ ] IncomingRule → call_action_rules 変換が正しく動作する
- [ ] IvrFlowDefinition → ivr_nodes/ivr_transitions 変換が正しく動作する
- [ ] Frontend 停止時でも前回取得分で動作継続する

### AC-5: ログ・監視
- [ ] ルール評価結果がログ出力される
- [ ] IVR 実行フローがログ出力される
- [ ] Serversync のログ出力される

---

## 変更履歴

| 日付 | バージョン | 変更内容 | 作成者 |
|------|-----------|---------|--------|
| 2026-02-08 | 1.0 | 初版作成（Draft） | Claude Code (claude-sonnet-4-5) |
| 2026-02-08 | 1.1 | Codex レビュー反映：VB を MVP 対象に追加、GET /api/number-groups 追加、IVR timeout/maxRetries 保存先明記、group_id/group_name 前提条件明記、SoT の移行期間モード明記 | Claude Code (claude-sonnet-4-5) |
| 2026-02-08 | 1.2 | Codex 再レビュー反映：Serversync ログ例を API 分離に修正（GET /api/number-groups 行を追加） | Claude Code (claude-sonnet-4-5) |
| 2026-02-08 | 1.3 | Issue #139 決定反映：anonymousAction / defaultAction の未設定時フォールバック動作を明記 | Claude Code (claude-sonnet-4-5) |
| 2026-02-08 | 1.4 | STEER-139 整合性修正：CallerGroup 変換の category/action_code/recording_enabled/announce_enabled を DB 制約に適合させる（category='general', action_code='VR', recording_enabled=true, announce_enabled=true） | Claude Code (claude-sonnet-4-5) |
