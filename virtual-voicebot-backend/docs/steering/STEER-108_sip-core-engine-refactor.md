# STEER-108: 3層アーキテクチャへのリファクタリング

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-108 |
| タイトル | 3層アーキテクチャへのリファクタリング |
| ステータス | Draft |
| 関連Issue | #108 |
| 優先度 | P1 |
| 作成日 | 2026-02-06 |

---

## 2. ストーリー（Why）

### 2.1 背景

現在の `src/` ディレクトリは、全モジュール（19個）がフラットに配置されており、以下の問題がある：

1. **責務境界の不明瞭さ**: プロトコル層・サービス層・インターフェース層の境界が不明確
2. **依存方向の管理困難**: フラット構造では、どのモジュールがどのモジュールに依存すべきかが不明確
3. **新機能追加の困難さ**: 新機能（health, monitoring, sync）をどこに配置すべきか判断しづらい
4. **保守性の低下**: モジュール間の依存関係が複雑化し、影響範囲の把握が困難

現在の `src/` 構造（フラット）:
```
src/
├── ai/
├── app/
├── codec/
├── config/
├── error/
├── http/
├── media/
├── recording/
├── rtp/
├── sdp/
├── session/
├── sip/
├── test_support/
├── transport/
├── lib.rs
└── main.rs
```

### 2.2 目的

全モジュールを **3層アーキテクチャ**（Protocol/Service/Interface）に再構成することで：

1. **責務境界の明確化**: 各層の役割を明確にし、依存方向を強制
2. **依存方向の管理**: Interface → Service → Protocol の一方向依存を実現
3. **新機能追加の容易化**: 新機能（health, monitoring, sync）を Interface 層に自然に配置
4. **保守性の向上**: 層単位での変更影響範囲の限定、テスト戦略の明確化

### 2.3 ユーザーストーリー（該当する場合）

```
As a Backend開発者
I want to 3層アーキテクチャによる明確な責務分離
So that モジュール間の依存方向が明確になり、新機能追加が容易になる

受入条件:
- [ ] 全モジュールが protocol/, service/, interface/, shared/ の4ディレクトリに配置されている
- [ ] 各モジュールは既存の4層構造（Entity/UseCase/Adapter/Infrastructure）を維持している
- [ ] 依存方向が Interface → Service → Protocol に統一されている
- [ ] main.rs および全モジュールのインポートパスが正しく更新されている
- [ ] 既存のテストが全て PASS する
- [ ] ドキュメント（DD-003 等）が新構造を反映している
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-06 |
| 起票理由 | 3層アーキテクチャによる責務明確化と新機能追加基盤の整備 |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Code (Sonnet 4.5) |
| 作成日 | 2026-02-06 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "案A: 3層ディレクトリ構造で進めてください" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| - | - | - | - | |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | @MasanoriSuda |
| 承認日 | - |
| 承認コメント | |

### 3.5 実装（該当する場合）

| 項目 | 値 |
|------|-----|
| 実装者 | Codex（実装AI） |
| 実装日 | - |
| 指示者 | @MasanoriSuda |
| 指示内容 | "[実装指示はCodexへ引き継ぎ]" |
| コードレビュー | CodeRabbit (自動) |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ実行者 | - |
| マージ日 | - |
| マージ先 | DD-003, DD-004, DD-005, DD-006, DD-007 |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| docs/design/detail/DD-003_clean-architecture.md | 修正 | ディレクトリ構造図の全面更新 |
| docs/design/detail/DD-004_rtp.md | 修正 | モジュール構造・インポートパス更新 |
| docs/design/detail/DD-005_session.md | 修正 | モジュール構造・インポートパス更新 |
| docs/design/detail/DD-006_transport.md | 修正 | モジュール構造・インポートパス更新 |
| docs/design/detail/DD-007_sip.md | 修正 | モジュール構造・インポートパス更新 |
| 各モジュール README.md | 修正 | パス参照の更新 |

### 4.2 影響するコード

| モジュール | 変更種別 | 概要 |
|-----------|---------|------|
| src/protocol/ | 追加 | 新ディレクトリ作成 |
| src/protocol/sip/ | 移動 | src/sip/ から移動 |
| src/protocol/rtp/ | 移動 | src/rtp/ から移動 |
| src/protocol/session/ | 移動 | src/session/ から移動 |
| src/protocol/transport/ | 移動 | src/transport/ から移動 |
| src/protocol/sdp/ | 移動 | src/sdp/ から移動 |
| src/protocol/mod.rs | 追加 | 再エクスポート定義 |
| src/service/ | 追加 | 新ディレクトリ作成 |
| src/service/ai/ | 移動 | src/ai/ から移動 |
| src/service/call_control/ | 移動+改名 | src/app/ から移動・改名 |
| src/service/recording/ | 移動 | src/recording/ から移動 |
| src/service/mod.rs | 追加 | 再エクスポート定義 |
| src/interface/ | 追加 | 新ディレクトリ作成 |
| src/interface/http/ | 移動 | src/http/ から移動 |
| src/interface/health/ | 追加 | 新機能（将来） |
| src/interface/monitoring/ | 追加 | 新機能（将来） |
| src/interface/sync/ | 追加 | 新機能（将来） |
| src/interface/mod.rs | 追加 | 再エクスポート定義 |
| src/shared/ | 追加 | 新ディレクトリ作成 |
| src/shared/config/ | 移動 | src/config/ から移動 |
| src/shared/error/ | 移動 | src/error/ から移動 |
| src/shared/codec/ | 移動 | src/codec/ から移動 |
| src/shared/media/ | 移動 | src/media/ から移動 |
| src/shared/test_support/ | 移動 | src/test_support/ から移動 |
| src/shared/mod.rs | 追加 | 再エクスポート定義 |
| src/lib.rs | 修正 | インポートパス更新 |
| src/main.rs | 修正 | インポートパス更新 |
| tests/integration/ | 修正 | インポートパス更新 |
| tests/unit/ | 修正 | インポートパス更新 |

---

## 5. 差分仕様（What / How）

### 5.1 新ディレクトリ構造

#### 移動後の `src/` 構造（3層アーキテクチャ）

```
src/
├── protocol/              # プロトコル層（最下層）
│   ├── mod.rs             # 再エクスポート
│   ├── sip/               # src/sip/ から移動
│   │   ├── entity/
│   │   ├── usecase/
│   │   ├── adapter/
│   │   ├── infrastructure/
│   │   └── mod.rs
│   ├── rtp/               # src/rtp/ から移動
│   │   ├── entity/
│   │   ├── usecase/
│   │   ├── adapter/
│   │   ├── infrastructure/
│   │   └── mod.rs
│   ├── session/           # src/session/ から移動
│   │   ├── entity/
│   │   ├── usecase/
│   │   ├── adapter/
│   │   ├── infrastructure/
│   │   └── mod.rs
│   ├── transport/         # src/transport/ から移動
│   │   ├── entity/
│   │   ├── usecase/
│   │   ├── adapter/
│   │   ├── infrastructure/
│   │   └── mod.rs
│   └── sdp/               # src/sdp/ から移動
│       ├── entity/
│       ├── usecase/
│       ├── adapter/
│       ├── infrastructure/
│       └── mod.rs
├── service/               # サービス層（中間層）
│   ├── mod.rs             # 再エクスポート
│   ├── ai/                # src/ai/ から移動
│   │   ├── entity/
│   │   ├── usecase/
│   │   ├── adapter/
│   │   ├── infrastructure/
│   │   └── mod.rs
│   ├── call_control/      # src/app/ から移動・改名
│   │   ├── entity/
│   │   ├── usecase/
│   │   ├── adapter/
│   │   ├── infrastructure/
│   │   └── mod.rs
│   └── recording/         # src/recording/ から移動
│       ├── entity/
│       ├── usecase/
│       ├── adapter/
│       ├── infrastructure/
│       └── mod.rs
├── interface/             # インターフェース層（最上層）
│   ├── mod.rs             # 再エクスポート
│   ├── http/              # src/http/ から移動
│   │   ├── entity/
│   │   ├── usecase/
│   │   ├── adapter/
│   │   ├── infrastructure/
│   │   └── mod.rs
│   ├── health/            # 将来追加予定
│   ├── monitoring/        # 将来追加予定
│   └── sync/              # 将来追加予定
├── shared/                # 共通モジュール
│   ├── mod.rs             # 再エクスポート
│   ├── config/            # src/config/ から移動
│   ├── error/             # src/error/ から移動
│   ├── codec/             # src/codec/ から移動
│   ├── media/             # src/media/ から移動
│   └── test_support/      # src/test_support/ から移動
├── lib.rs
└── main.rs
```

#### 設計方針

1. **3層の役割**
   - **Protocol 層**: SIP/RTP/Session/Transport/SDP（通信プロトコル実装）
   - **Service 層**: AI連携・通話制御・録音処理（ビジネスロジック）
   - **Interface 層**: HTTP API・監視・同期（外部インターフェース）
   - **Shared**: 全層から参照される共通モジュール（Config/Error/Codec/Media/TestSupport）

2. **依存方向の制約**
   - Interface → Service → Protocol → Shared（一方向依存）
   - 逆方向依存は禁止（DIP: Dependency Inversion Principle により抽象に依存）
   - Shared は全層から参照可能（ただし他モジュールへの依存は禁止）

3. **各モジュールの内部構造**
   - 各モジュール内部は既存の4層構造（Entity/UseCase/Adapter/Infrastructure）を維持
   - モジュール間の相対参照は可能な限り維持（変更最小化）

4. **再エクスポート戦略**
   - 各層（protocol, service, interface, shared）の `mod.rs` で主要な型を再エクスポート
   - 外部モジュールからは `crate::protocol::session::...` のように参照
   - 必要に応じて `src/lib.rs` でエイリアスを提供（後方互換性）

### 5.2 インポートパスの変更

#### 変更前（例: src/ai/mod.rs）

```rust
use crate::session::{SessionEvent, SessionHandle};
use crate::rtp::RtpStream;
use crate::config::Config;
```

#### 変更後（例: src/service/ai/mod.rs）

```rust
use crate::protocol::session::{SessionEvent, SessionHandle};
use crate::protocol::rtp::RtpStream;
use crate::shared::config::Config;
```

#### src/protocol/mod.rs の定義

```rust
// src/protocol/mod.rs

pub mod sip;
pub mod rtp;
pub mod session;
pub mod transport;
pub mod sdp;

// 主要な型を再エクスポート
pub use session::{Session, SessionEvent, SessionHandle, SessionManager};
pub use sip::{SipMessage, SipRequest, SipResponse, SipTransport};
pub use rtp::{RtpPacket, RtpSession, RtpStream};
pub use transport::{Transport, UdpTransport};
pub use sdp::{SdpSession, MediaDescription};
```

#### src/service/mod.rs の定義

```rust
// src/service/mod.rs

pub mod ai;
pub mod call_control;
pub mod recording;

// 主要な型を再エクスポート
pub use ai::{AiService, AiProvider};
pub use call_control::{CallController, CallEvent};
pub use recording::{RecordingService, RecordingFormat};
```

#### src/interface/mod.rs の定義

```rust
// src/interface/mod.rs

pub mod http;
// pub mod health;      // 将来追加
// pub mod monitoring;  // 将来追加
// pub mod sync;        // 将来追加

// 主要な型を再エクスポート
pub use http::{HttpServer, HttpConfig};
```

#### src/shared/mod.rs の定義

```rust
// src/shared/mod.rs

pub mod config;
pub mod error;
pub mod codec;
pub mod media;
pub mod test_support;

// 主要な型を再エクスポート
pub use config::Config;
pub use error::{Error, Result};
pub use codec::{Codec, CodecType};
pub use media::{MediaFrame, MediaStream};
```

### 5.3 段階的実装アプローチ

#### Phase 1: ディレクトリ移動とモジュール定義

1. `src/protocol/`, `src/service/`, `src/interface/`, `src/shared/` ディレクトリ作成
2. Protocol 層: `src/{sip,rtp,session,transport,sdp}/` → `src/protocol/` 配下に移動
3. Service 層: `src/{ai,app,recording}/` → `src/service/` 配下に移動（app は call_control に改名）
4. Interface 層: `src/http/` → `src/interface/` 配下に移動
5. Shared: `src/{config,error,codec,media,test_support}/` → `src/shared/` 配下に移動
6. 各層の `mod.rs` 作成（再エクスポート定義）

#### Phase 2: インポートパス更新

1. `src/lib.rs` のインポートパス更新
2. `src/main.rs` のインポートパス更新
3. Protocol 層内部の相対参照を必要に応じて修正
4. Service 層のインポートパス更新（protocol, shared への依存）
5. Interface 層のインポートパス更新（service, protocol, shared への依存）

#### Phase 3: テスト修正

1. 単体テストのインポートパス更新
2. 統合テストのインポートパス更新
3. 全テスト実行・検証

#### Phase 4: ドキュメント更新

1. DD-003 (Clean Architecture) のディレクトリ構造図更新
2. DD-004, DD-005, DD-006, DD-007 のパス参照更新
3. 各モジュール README.md の更新

### 5.4 詳細設計追加（DD-003 へマージ）

```markdown
## DD-003-STRUCT-02: 3層アーキテクチャ構造

### 概要

全モジュールを Protocol/Service/Interface の3層構造に再構成。
各層は明確な責務を持ち、依存方向は Interface → Service → Protocol の一方向とする。

### ディレクトリ構造

src/
├── protocol/            # プロトコル層（最下層）
│   ├── mod.rs           # 再エクスポート
│   ├── sip/             # SIP プロトコル実装
│   │   ├── entity/
│   │   ├── usecase/
│   │   ├── adapter/
│   │   ├── infrastructure/
│   │   └── mod.rs
│   ├── rtp/             # RTP/RTCP 実装
│   ├── session/         # Session 管理
│   ├── transport/       # Transport 抽象化
│   └── sdp/             # SDP プロトコル実装
├── service/             # サービス層（中間層）
│   ├── mod.rs           # 再エクスポート
│   ├── ai/              # AI連携サービス
│   ├── call_control/    # 通話制御サービス
│   └── recording/       # 録音サービス
├── interface/           # インターフェース層（最上層）
│   ├── mod.rs           # 再エクスポート
│   ├── http/            # HTTP API
│   ├── health/          # ヘルスチェック（将来）
│   ├── monitoring/      # 監視（将来）
│   └── sync/            # Frontend/Backend 同期（将来）
└── shared/              # 共通モジュール
    ├── mod.rs           # 再エクスポート
    ├── config/          # 設定管理
    ├── error/           # エラー型定義
    ├── codec/           # コーデック
    ├── media/           # メディア処理
    └── test_support/    # テストヘルパー

### 依存方向

1. 層間依存:
   - Interface → Service, Protocol, Shared
   - Service → Protocol, Shared
   - Protocol → Shared
   - Shared → （外部依存なし）

2. Protocol 層内部:
   - sip → session, transport, sdp
   - rtp → session, transport
   - session → transport
   - transport → （外部依存なし）
   - sdp → （外部依存なし）

3. Service 層内部:
   - ai → protocol::session
   - call_control → protocol::session, protocol::sip
   - recording → protocol::rtp, protocol::session

4. Interface 層内部:
   - http → service::*, protocol::session
   - health → service::* （将来）
   - monitoring → service::* （将来）
   - sync → service::* （将来）

### 再エクスポート

各層の mod.rs で主要な型を再エクスポートし、外部モジュールからのアクセスを簡潔にする。

### トレース
- ← BD: BD-003-MOD-01
- → UT: UT-003-TC-01（ディレクトリ構造検証）
```

### 5.5 テストケース追加（UT-003 へマージ）

```markdown
## UT-003-TC-01: 3層アーキテクチャ構造検証

### 対象
DD-003-STRUCT-02

### 目的
3層構造（protocol, service, interface, shared）が正しく配置され、依存方向が適切であることを検証

### 入力
```rust
// Protocol 層
use crate::protocol::session::Session;
use crate::protocol::sip::SipMessage;
use crate::protocol::rtp::RtpPacket;

// Service 層
use crate::service::ai::AiService;
use crate::service::call_control::CallController;

// Interface 層
use crate::interface::http::HttpServer;

// Shared
use crate::shared::config::Config;
use crate::shared::error::Error;
```

### 期待結果
- 全てのインポートがコンパイルエラーなく解決される
- 依存方向の逆転が発生していない（コンパイラによる検証）
- 既存の統合テストが全て PASS する

### トレース
← DD: DD-003-STRUCT-02
```

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #108 | STEER-108 | 起票 |
| STEER-108 | DD-003-STRUCT-02 | 構造定義追加 |
| DD-003-STRUCT-02 | UT-003-TC-01 | 単体テスト |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [ ] 3層構造（Protocol/Service/Interface）の責務が明確か
- [ ] 依存方向（Interface → Service → Protocol）が適切か
- [ ] 既存のClean Architecture 4層構造が各モジュール内部で維持されているか
- [ ] インポートパスの変更が網羅的か
- [ ] 段階的実装アプローチが現実的か
- [ ] 既存仕様（DD-003, DD-004 等）との整合性があるか
- [ ] app → call_control への改名が適切か

### 7.2 マージ前チェック（Approved → Merged）

- [ ] 実装が完了している
- [ ] Codex によるコードレビューを受けている
- [ ] 全てのテストが PASS している
- [ ] ドキュメント（DD-003, DD-004, DD-005, DD-006, DD-007）が更新されている
- [ ] 各モジュール README.md が更新されている

---

## 8. 備考

### 8.1 app → call_control への改名

**決定事項**: `src/app/` を `src/service/call_control/` へ移動・改名する

理由:
- `app` という名前が汎用的すぎて責務が不明確
- `call_control` の方が「通話制御」という役割が明確
- Service 層の他モジュール（ai, recording）との命名一貫性

### 8.2 Shared モジュールの扱い

Shared モジュールは全層から参照可能だが、以下の制約を設ける：

- **Shared は他モジュールに依存しない**（循環依存を防ぐ）
- **Shared は純粋な Utility/Helper のみ**（ビジネスロジックを含まない）
- **Shared 内のモジュール同士の依存は最小限に**

### 8.3 新機能の配置

将来追加予定の新機能は以下のように配置：

- **health/**: `src/interface/health/` — ヘルスチェックエンドポイント
- **monitoring/**: `src/interface/monitoring/` — メトリクス収集・監視
- **sync/**: `src/interface/sync/` — Frontend/Backend データ同期

これらは全て Interface 層に配置し、Service 層を経由して Protocol 層にアクセスする。

### 8.4 後方互換性

外部クレートや統合テストでインポートパスを直接参照している場合、`src/lib.rs` で以下のようなエイリアスを提供できる：

```rust
// src/lib.rs

// Protocol 層のエイリアス
pub use protocol::session as session;
pub use protocol::sip as sip;
pub use protocol::rtp as rtp;
pub use protocol::transport as transport;
pub use protocol::sdp as sdp;

// Service 層のエイリアス
pub use service::ai as ai;
pub use service::call_control as app;  // 旧名 app への互換性
pub use service::recording as recording;

// Interface 層のエイリアス
pub use interface::http as http;

// Shared のエイリアス
pub use shared::config as config;
pub use shared::error as error;
```

ただし、この方法は過渡的な措置とし、最終的には 3層構造のパスを明示的に使用することを推奨。

### 8.5 リスク

| リスク | 影響度 | 緩和策 |
|--------|--------|--------|
| インポートパス更新漏れ | 高 | コンパイラによる網羅的検出 + grep による事前確認 |
| テスト失敗 | 中 | Phase 3 で全テスト実行・検証 |
| ドキュメント更新漏れ | 低 | Phase 4 でチェックリスト確認 |
| 依存方向の逆転 | 高 | UT-003-TC-01 で依存方向を検証 |

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-06 | 初版作成（sip-core-engine 方式） | Claude Code (Sonnet 4.5) |
| 2026-02-06 | sdp/ モジュールを追加（Q5:A 決定） | Claude Code (Sonnet 4.5) |
| 2026-02-06 | 3層アーキテクチャ方式へ全面変更（案A 採用） | Claude Code (Sonnet 4.5) |
