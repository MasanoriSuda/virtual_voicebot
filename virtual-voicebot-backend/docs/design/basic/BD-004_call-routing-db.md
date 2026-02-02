# BD-004_call-routing-db

> 着信ルーティング・IVRフロー管理のデータベース設計

| 項目 | 値 |
|------|-----|
| ID | BD-004 |
| ステータス | Draft |
| 作成日 | 2026-02-02 |
| 関連Issue | #92 |
| 対応RD | RD-004 |
| 対応IT | - |

---

## 1. 概要

### 1.1 目的

着信時の電話番号振り分け、IVRフロー制御、迷惑電話対策に必要なデータベース構造を定義する。

### 1.2 スコープ

- 発信者分類（4カテゴリ）のデータ構造
- アクションコード体系
- IVRフロー（木構造）のデータモデル
- Rust側の状態管理方針

---

## 2. 発信者分類

### 2.1 4カテゴリ分類

着信時に発信者番号（Caller ID）を以下の4カテゴリに分類し、それぞれ異なるアクションを設定可能とする。

| カテゴリ | 説明 | 判定条件 | デフォルトアクション |
|---------|------|----------|---------------------|
| spam | 迷惑電話DB登録番号 | `spam_numbers` テーブルに一致 | RJ（即時拒否） |
| registered | 登録済み電話番号 | `registered_numbers` テーブルに一致 | VR（ボイスボット録音あり） |
| unknown | 未登録番号 | 上記いずれにも該当しない | IV（IVR） |
| anonymous | 非通知 | Caller ID が空/非通知 | IV（IVR） |

> **Note**: デフォルトアクションは管理画面から変更可能。上記は初期値。

### 2.2 分類フロー

```
着信
  │
  ▼
┌─────────────────┐
│ Caller ID 取得   │
└────────┬────────┘
         │
         ▼
    ┌─────────┐
    │ 非通知？ │─── Yes ──▶ anonymous
    └────┬────┘
         │ No
         ▼
┌─────────────────┐
│ spam_numbers    │─── Hit ──▶ spam
│ テーブル検索     │
└────────┬────────┘
         │ Miss
         ▼
┌─────────────────────┐
│ registered_numbers  │─── Hit ──▶ registered
│ テーブル検索         │
└────────┬────────────┘
         │ Miss
         ▼
      unknown
```

---

## 3. アクションコード体系

### 3.1 設計方針

- 2文字の英字コードで処理を表現
- 録音有無は別コードでペア化（AN/AR、VB/VR）
- IVR関連は `I` プレフィックスで統一

### 3.2 基本アクションコード

| コード | 名称 | 説明 | 録音 |
|--------|------|------|------|
| VB | Voicebot | AI応答（ボイスボット）開始（録音なし） | ❌ |
| VR | Voicebot+Record | AI応答（ボイスボット）開始（録音あり） | ✅ |
| NR | No Response | 応答なし（コール音のみ） | - |
| RJ | Reject | 即時拒否（話中音） | - |
| BZ | Busy | 話中応答 | - |
| AN | Announce | アナウンス再生（録音なし） | ❌ |
| AR | Announce+Record | アナウンス再生（録音あり） | ✅ |
| VM | Voicemail | 留守番電話 | ✅ |
| IV | IVR | IVRフローへ移行 | - |

### 3.3 録音オプション対応表

| 機能 | 録音なし | 録音あり |
|------|---------|---------|
| ボイスボット（AI応答） | VB | VR |
| アナウンス再生 | AN | AR |
| 留守番電話 | - | VM（常に録音） |

### 3.4 IVR内アクションコード

| コード | 名称 | 説明 |
|--------|------|------|
| IA | IVR Announce | IVR内アナウンス再生 |
| IR | IVR Record | IVR内録音開始 |
| IK | IVR Keypad | DTMF入力待ち |
| IW | IVR Wait | 無音待機（タイムアウト付き） |
| IF | IVR Forward | 転送 |
| IT | IVR Transfer | 内線転送 |
| IB | IVR Voicebot | IVR内からボイスボットへ移行 |
| IE | IVR Exit | IVR終了（切断） |

### 3.5 コードチェイニング

アクションはチェイン可能。例：

```
AN → IV    # アナウンス再生後、IVRへ
IV → VR    # IVR完了後、ボイスボット（録音あり）
AR → IE    # 録音付きアナウンス後、切断
IV → VB    # IVR完了後、ボイスボット（録音なし）
```

---

## 4. データベース設計

### 4.1 ER図

```
┌──────────────────┐     ┌──────────────────┐
│  spam_numbers    │     │registered_numbers│
├──────────────────┤     ├──────────────────┤
│ id (PK)          │     │ id (PK)          │
│ phone_number     │     │ phone_number     │
│ reason           │     │ name             │
│ created_at       │     │ category         │
│ updated_at       │     │ action_code      │
└──────────────────┘     │ created_at       │
                         │ updated_at       │
                         └──────────────────┘

┌──────────────────┐     ┌──────────────────┐     ┌──────────────────┐
│    ivr_flows     │     │    ivr_nodes     │     │ ivr_transitions  │
├──────────────────┤     ├──────────────────┤     ├──────────────────┤
│ id (PK)          │◀────│ flow_id (FK)     │     │ id (PK)          │
│ name             │     │ id (PK)          │◀────│ from_node_id(FK) │
│ root_node_id(FK)─┼────▶│ parent_id (FK)   │     │ input_type       │
│ description      │     │ node_type        │     │ dtmf_key         │
│ is_active        │     │ action_code      │     │ to_node_id (FK)  │
│ created_at       │     │ audio_file_url   │     │ created_at       │
│ updated_at       │     │ tts_text         │     └──────────────────┘
└──────────────────┘     │ timeout_sec      │
                         │ max_retries      │
┌──────────────────┐     │ created_at       │
│ routing_rules    │     │ updated_at       │
├──────────────────┤     └──────────────────┘
│ id (PK)          │
│ caller_category  │
│ action_code      │
│ ivr_flow_id (FK) │
│ priority         │
│ is_active        │
│ created_at       │
│ updated_at       │
└──────────────────┘
```

### 4.2 テーブル定義

#### spam_numbers（迷惑電話DB）

```sql
CREATE TABLE spam_numbers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    phone_number VARCHAR(20) NOT NULL UNIQUE,
    reason VARCHAR(255),
    source VARCHAR(50),  -- 'manual' | 'import' | 'report'
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_spam_numbers_phone ON spam_numbers(phone_number);
```

#### registered_numbers（登録済み番号）

```sql
CREATE TABLE registered_numbers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    phone_number VARCHAR(20) NOT NULL UNIQUE,
    name VARCHAR(100),
    category VARCHAR(50),  -- 'vip' | 'customer' | 'partner' etc.
    action_code VARCHAR(2) DEFAULT 'VR',
    ivr_flow_id UUID REFERENCES ivr_flows(id),
    notes TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_registered_numbers_phone ON registered_numbers(phone_number);
```

#### routing_rules（ルーティングルール）

```sql
CREATE TABLE routing_rules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    caller_category VARCHAR(20) NOT NULL,  -- 'spam' | 'registered' | 'unknown' | 'anonymous'
    action_code VARCHAR(2) NOT NULL,
    ivr_flow_id UUID REFERENCES ivr_flows(id),
    priority INT DEFAULT 0,
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),

    CONSTRAINT valid_caller_category CHECK (
        caller_category IN ('spam', 'registered', 'unknown', 'anonymous')
    )
);

CREATE INDEX idx_routing_rules_category ON routing_rules(caller_category, is_active);
```

#### ivr_flows（IVRフロー定義）

```sql
CREATE TABLE ivr_flows (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(100) NOT NULL,
    description TEXT,
    root_node_id UUID,  -- 後でALTER ADDで参照追加
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);
```

#### ivr_nodes（IVRノード）

```sql
CREATE TABLE ivr_nodes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    flow_id UUID NOT NULL REFERENCES ivr_flows(id) ON DELETE CASCADE,
    parent_id UUID REFERENCES ivr_nodes(id),
    node_type VARCHAR(20) NOT NULL,  -- 'ANNOUNCE' | 'KEYPAD' | 'FORWARD' | 'EXIT'
    action_code VARCHAR(2),
    audio_file_url TEXT,
    tts_text TEXT,
    timeout_sec INT DEFAULT 10,
    max_retries INT DEFAULT 3,
    exit_action VARCHAR(2) DEFAULT 'IE',  -- リトライ超過時のアクション
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),

    CONSTRAINT valid_node_type CHECK (
        node_type IN ('ANNOUNCE', 'KEYPAD', 'FORWARD', 'TRANSFER', 'RECORD', 'EXIT')
    )
);

-- root_node_id の外部キー追加
ALTER TABLE ivr_flows
ADD CONSTRAINT fk_root_node
FOREIGN KEY (root_node_id) REFERENCES ivr_nodes(id);

CREATE INDEX idx_ivr_nodes_flow ON ivr_nodes(flow_id);
```

#### ivr_transitions（IVR遷移）

```sql
CREATE TABLE ivr_transitions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    from_node_id UUID NOT NULL REFERENCES ivr_nodes(id) ON DELETE CASCADE,
    input_type VARCHAR(20) NOT NULL,  -- 'DTMF' | 'TIMEOUT' | 'INVALID' | 'COMPLETE'
    dtmf_key VARCHAR(5),  -- '0'-'9', '*', '#', or NULL
    to_node_id UUID REFERENCES ivr_nodes(id),
    created_at TIMESTAMPTZ DEFAULT NOW(),

    CONSTRAINT valid_input_type CHECK (
        input_type IN ('DTMF', 'TIMEOUT', 'INVALID', 'COMPLETE')
    )
);

CREATE INDEX idx_ivr_transitions_from ON ivr_transitions(from_node_id);
```

---

## 5. Rust 側状態管理

### 5.1 IVR ステートマシン

Rust 側では IVR の現在状態を保持し、DTMF 入力やタイムアウトに応じて次の状態を決定する。

```
┌─────────────────────────────────────────────────────────┐
│                    IvrStateMachine                       │
├─────────────────────────────────────────────────────────┤
│ - flow_id: Uuid                                          │
│ - current_node_id: Uuid                                  │
│ - retry_count: u32                                       │
│ - state: IvrState                                        │
├─────────────────────────────────────────────────────────┤
│ + new(flow_id) -> Self                                   │
│ + process_dtmf(key: char) -> NextAction                  │
│ + process_timeout() -> NextAction                        │
│ + process_complete() -> NextAction                       │
└─────────────────────────────────────────────────────────┘

enum IvrState {
    Idle,
    PlayingAnnounce,
    WaitingInput,
    Recording,
    Forwarding,
    Completed,
}

enum NextAction {
    PlayAudio(url),
    WaitDtmf { timeout_sec: u32 },
    Forward { destination: String },
    StartRecording,
    Exit,
    Error(String),
}
```

### 5.2 状態遷移

```
                        ┌─────────────┐
                        │    Idle     │
                        └──────┬──────┘
                               │ start()
                               ▼
                        ┌─────────────────┐
              ┌────────│ PlayingAnnounce │
              │         └────────┬────────┘
              │                  │ complete
              │                  ▼
              │         ┌─────────────────┐
     timeout/ │         │  WaitingInput   │◀─────────┐
     invalid  │         └────────┬────────┘          │
     (retry)  │                  │                   │
              │         ┌────────┴────────┐          │
              │         │                  │          │
              │    valid DTMF         invalid/timeout │
              │         │                  │          │
              │         ▼                  └──────────┘
              │  ┌─────────────┐                (retry < max)
              │  │ Next Node   │
              │  └──────┬──────┘
              │         │
              │         ▼
              │  ┌─────────────┐
              └─▶│ Completed   │ (retry >= max → IE)
                 └─────────────┘
```

### 5.3 DTMF 入力パターン

| パターン | 説明 | 遷移先決定 |
|----------|------|-----------|
| valid | transitions に定義された DTMF キー | `to_node_id` へ遷移 |
| invalid | transitions に未定義のキー | `INVALID` 遷移 or リトライ |
| timeout | `timeout_sec` 経過 | `TIMEOUT` 遷移 or リトライ |

### 5.4 リトライ超過時の動作

`max_retries` を超えた場合、`exit_action`（デフォルト: `IE` = 切断）を実行する。

---

## 6. データフロー

### 6.1 着信時の処理フロー

```
SIP INVITE 受信
      │
      ▼
┌─────────────────┐
│ Caller ID 抽出   │
└────────┬────────┘
         │
         ▼
┌─────────────────────────────────┐
│ DB 検索（分類判定）              │
│ 1. spam_numbers                 │
│ 2. registered_numbers           │
│ 3. routing_rules (by category)  │
└────────────────┬────────────────┘
                 │
                 ▼
        ┌────────────────┐
        │ action_code    │
        │ 取得           │
        └────────┬───────┘
                 │
    ┌────────────┴────────────┐
    │                         │
    ▼                         ▼
┌────────┐              ┌──────────┐
│ 即時系  │              │ IVR系    │
│ NC/RJ/ │              │ IV       │
│ AN/AR  │              └────┬─────┘
└────────┘                   │
                             ▼
                    ┌────────────────┐
                    │ IVR フロー読込  │
                    │ (ivr_flows +   │
                    │  ivr_nodes +   │
                    │  transitions)  │
                    └────────┬───────┘
                             │
                             ▼
                    ┌────────────────┐
                    │ IvrStateMachine│
                    │ 生成・開始      │
                    └────────────────┘
```

---

## 7. 非機能設計方針

### 7.1 性能

| 項目 | 方針 |
|------|------|
| 番号検索 | インデックス使用、O(log N) |
| IVR フロー読込 | 開始時に全ノード/遷移をメモリ展開 |
| キャッシュ | routing_rules はアプリ起動時にキャッシュ |

### 7.2 可用性

| 項目 | 方針 |
|------|------|
| DB 障害時 | デフォルトルール（unknown → IV）適用 |
| IVR フロー不整合 | エラーログ + 切断 |

### 7.3 セキュリティ

| 項目 | 方針 |
|------|------|
| SQL インジェクション | パラメータバインド必須（SQLx） |
| 電話番号 | E.164 正規化して保存（`+819012345678` 形式） |

---

## 8. 前提条件・制約

### 8.1 前提条件

- PostgreSQL を使用（#62 決定事項）
- Backend は Rust + SQLx でDB接続
- SIP INVITE から Caller ID を取得可能

### 8.2 制約

- IVR フローは木構造（循環禁止）
- 1つの IVR フロー内のノード数は 100 以下を推奨
- DTMF キーは 0-9, *, # のみ

---

## 9. 未確定事項（Open Questions）

### 解決済み

- [x] Q1: 本設計のスコープ → 仕様決定のみ
- [x] Q2: RD-004 との関係 → BD レベルで対応
- [x] Q3: DB 技術選定 → PostgreSQL
- [x] Q4: アクションコード体系 → 2文字コード（v3）
- [x] Q5: 発信者分類 → 4カテゴリ
- [x] Q6: 非通知デフォルト → IVR
- [x] Q7: IVR リトライ超過時 → 切断（IE）

### 解決済み（Codex 指摘）

- [x] Q8: **DBスキーマ移行方針** → **SQLx migrate** を使用
  - Rust + SQLx 環境に整合、`migrations/` ディレクトリで管理

- [x] Q9: **ActionCode 適用優先度** → **registered_numbers 優先**（個別設定 > デフォルト）
  - 判定順序: spam_numbers → registered_numbers（個別 action_code）→ routing_rules（カテゴリデフォルト）

- [x] Q10: **ivr_nodes.node_type vs action_code** → **責務分離**
  - `node_type`: ノードの構造種別（ANNOUNCE / KEYPAD / FORWARD / EXIT 等）
  - `action_code`: そのノードで実行する処理コード（IA / IK / IF / IE 等）
  - node_type は木構造の意味、action_code は実行時の処理を規定

- [x] Q11: **IVR 木構造保証** → **両方**（DB + アプリ）
  - DB: `parent_id` による親子関係 + アプリ側で挿入/更新時に循環チェック
  - 理由: DBトリガーは複雑になるため、アプリ側で検証しつつ DB で基本構造を保証

- [x] Q12: **Caller ID 正規化ルール** → **E.164 正規化**
  - 国番号欠落時: 日本 `+81` を補完（デフォルト設定）
  - 先頭 `0` 除去: `090...` → `+8190...`
  - ハイフン/スペース除去: `090-1234-5678` → `+819012345678`
  - 保存形式: `+{国番号}{番号}` （例: `+819012345678`）

---

## 変更履歴

| 日付 | バージョン | 変更内容 | 作成者 |
|------|-----------|---------|--------|
| 2026-02-02 | 1.0 | 初版作成（#92 壁打ち結果） | Claude Code |
| 2026-02-02 | 1.1 | VB/VR（ボイスボット録音なし/あり）追加、IB追加、録音オプション対応表追加 | Claude Code |
| 2026-02-02 | 1.2 | Codex 指摘による Open Questions 追加（Q8〜Q12） | Claude Code |
| 2026-02-02 | 1.3 | Q8〜Q12 解決: SQLx migrate、優先度、責務分離、循環チェック、E.164正規化 | Claude Code |
