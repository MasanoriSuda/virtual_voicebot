<!-- SOURCE_OF_TRUTH: 開発ガイドライン -->
# 開発ガイドライン

> Virtual Voicebot Backend の開発ガイドラインを定義する

---

## 1. コーディング規約

### 1.1 Rust スタイル

| ルール | ツール | 実行タイミング |
|--------|--------|---------------|
| フォーマット | `cargo fmt` | コミット前必須 |
| リント | `cargo clippy` | コミット前必須 |
| ビルド確認 | `cargo build` | PR 前必須 |
| テスト | `cargo test` | PR 前必須 |

### 1.2 命名規約

| 対象 | スタイル | 例 |
|------|---------|-----|
| 関数/メソッド | snake_case | `handle_invite()` |
| 構造体/列挙型 | PascalCase | `SipMessage`, `SessionState` |
| 定数 | SCREAMING_SNAKE_CASE | `MAX_RETRY_COUNT` |
| モジュール | snake_case | `sip`, `session_manager` |
| ファイル | snake_case | `sip_parser.rs` |

### 1.3 エラーハンドリング

```rust
// 推奨: anyhow::Result を使用
fn process_invite(msg: &SipMessage) -> anyhow::Result<()> {
    let sdp = msg.body.as_ref()
        .context("INVITE must have SDP body")?;
    // ...
    Ok(())
}

// 禁止: unwrap() の安易な使用
fn bad_example() {
    let value = option.unwrap(); // NG: panicの原因
}
```

### 1.4 ログ出力

| レベル | 用途 | 例 |
|--------|------|-----|
| `error!` | 処理継続不可のエラー | AI 連続失敗、致命的パースエラー |
| `warn!` | 回復可能な問題 | 1回目の AI 失敗、不正パケット破棄 |
| `info!` | 重要なイベント | 通話開始/終了、状態遷移 |
| `debug!` | デバッグ用詳細 | パケット内容、中間状態 |
| `trace!` | 最詳細トレース | ループ内処理 |

```rust
// 必須: call_id を含める
info!(call_id = %call_id, "Call started");
warn!(call_id = %call_id, error = %e, "ASR failed, retrying");
```

---

## 2. 相関 ID 規約

### 2.1 ID 体系

| ID | スコープ | 生成タイミング | フォーマット |
|----|---------|---------------|-------------|
| `call_id` | 通話全体 | INVITE 受信時 | SIP Call-ID ヘッダ値 |
| `session_id` | セッション | INVITE 受信時 | MVP: `call_id` と同値 |
| `stream_id` | メディアストリーム | RTP 開始時 | UUID v4 |

### 2.2 使用ルール

- **全ログ**: `call_id` を必ず含める
- **全イベント**: `call_id` + `session_id` を含める
- **PCM 系イベント**: 上記 + `stream_id` を含める
- **外部 API 呼び出し**: `session_id` をトレース用に渡す

---

## 3. モジュール設計ルール

### 3.1 依存方向

```
上位 → 下位（許可）
─────────────────
app → session → sip → transport
       │         │
       └──► rtp ─┘
app → ai

下位 → 上位（禁止）
─────────────────
transport → sip → session → app（NG）
```

### 3.2 モジュール間通信

| パターン | 実装 | 用途 |
|---------|------|------|
| イベント送信 | `mpsc::Sender<Event>` | モジュール間非同期通信 |
| 1:1 応答 | `async fn` + `.await` | AI API 呼び出し |
| 状態共有 | 禁止 | - |

### 3.3 新規モジュール追加時

1. [design.md](design.md) のレイヤ図に位置を確認
2. 依存方向ルールを遵守
3. イベント定義を追加（該当する正本ファイルに）
4. ユニットテストを追加

---

## 4. テスト規約

### 4.1 テスト種別

| 種別 | 場所 | 実行方法 | 必須 |
|------|------|---------|:----:|
| ユニットテスト | `src/**/*.rs` 内 `#[cfg(test)]` | `cargo test` | ✓ |
| 統合テスト | `tests/*.rs` | `cargo test` | ✓ |
| E2E テスト | `test/e2e/*.rs` | `cargo test --test <name>` | - |
| SIPp テスト | `test/sipp/*.xml` | 手動実行 | - |

### 4.2 テスト命名

```rust
#[test]
fn test_parse_invite_valid() { ... }      // 正常系
#[test]
fn test_parse_invite_missing_via() { ... } // 異常系
#[test]
fn test_parse_invite_empty_body() { ... }  // 境界値
```

### 4.3 テストカバレッジ目標

| 対象 | 目標 |
|------|------|
| パーサ（sip, sdp） | 80%+ |
| 状態遷移（session） | 90%+ |
| AI ラッパー | 70%+ |
| 全体 | 60%+ |

---

## 5. Git ワークフロー

### 5.1 ブランチ戦略

| ブランチ | 用途 | マージ先 |
|---------|------|---------|
| `main` | リリース済み | - |
| `develop` | 開発統合 | `main` |
| `feature/<name>` | 機能開発 | `develop` |
| `fix/<name>` | バグ修正 | `develop` |
| `hotfix/<name>` | 緊急修正 | `main` + `develop` |

### 5.2 コミットメッセージ

```
<type>: <subject>

<body>

<footer>
```

| type | 用途 |
|------|------|
| `feat` | 新機能 |
| `fix` | バグ修正 |
| `refactor` | リファクタリング |
| `docs` | ドキュメント |
| `test` | テスト追加/修正 |
| `chore` | ビルド/設定変更 |

### 5.3 PR チェックリスト

- [ ] `cargo fmt` 実行済み
- [ ] `cargo clippy` 警告なし
- [ ] `cargo test` 全パス
- [ ] 関連ドキュメント更新済み
- [ ] 正本ファイルとの整合性確認

---

## 6. ドキュメント管理

### 6.1 正本と補助

| 種別 | 定義 | 更新ルール |
|------|------|-----------|
| 正本 | `<!-- SOURCE_OF_TRUTH: ... -->` 付き | 仕様変更時に必ず更新 |
| 補助 | 正本を参照する説明文書 | 正本参照リンクを維持 |

### 6.2 更新フロー

1. 仕様変更時は **正本を先に更新**
2. 補助ドキュメントは正本を参照（コピペ禁止）
3. [DOCS_INDEX.md](../../docs/DOCS_INDEX.md) で一覧管理
4. 矛盾時は正本を優先（[DOCS_POLICY.md](../../docs/DOCS_POLICY.md) 参照）

### 6.3 主要正本一覧

| 正本 | 内容 |
|------|------|
| [design.md](design.md) | アーキテクチャ設計 |
| [sip.md](sip.md) | SIP 詳細設計 |
| [rtp.md](rtp.md) | RTP 詳細設計 |
| [session.md](session.md) | Session 詳細設計 |
| [app.md](app.md) | App 層 I/F |
| [ai.md](ai.md) | AI 連携 I/F |
| [tests.md](tests.md) | テスト計画・AC |

---

## 7. セキュリティガイドライン

### 7.1 PII（個人情報）取り扱い

| データ | 分類 | 取り扱い |
|--------|------|---------|
| 音声データ | PII | 暗号化保存、アクセス制限 |
| 文字起こし | PII | ログ出力禁止 |
| LLM 入出力 | PII | ログ出力禁止 |
| call_id | 非 PII | ログ出力可 |

### 7.2 ログマスキング

```rust
// NG: 原文をログ出力
info!("User said: {}", transcript);

// OK: 長さのみ出力
info!(call_id = %call_id, len = transcript.len(), "ASR result received");

// OK: デバッグフラグ付きで限定出力
if cfg!(debug_assertions) {
    debug!("DEBUG ONLY: {}", transcript);
}
```

### 7.3 外部 API 呼び出し

- TLS 必須（本番環境）
- API キーは環境変数で管理
- タイムアウト必須（config 管理）
- エラー時のリトライ上限を設定

---

## 8. 運用・監視

### 8.1 ログ設定

| 環境 | ログレベル | 出力先 |
|------|-----------|--------|
| 開発 | `debug` | stdout |
| 本番 | `info` | stdout + ファイル |

### 8.2 メトリクス（将来）

| メトリクス | 単位 | 用途 |
|-----------|------|------|
| `sip_invite_total` | count | 着信数 |
| `sip_response_code` | count by code | 応答コード分布 |
| `ai_latency_ms` | histogram | AI 応答時間 |
| `rtp_packet_loss` | ratio | パケットロス率 |

### 8.3 アラート条件（将来）

| 条件 | 重要度 | アクション |
|------|--------|-----------|
| AI 連続失敗率 > 10% | Critical | 即時調査 |
| RTP ロス > 5% | Warning | ネットワーク確認 |
| セッション数 > 閾値 | Warning | スケーリング検討 |

---

## 9. 開発環境セットアップ

### 9.1 必要ツール

| ツール | バージョン | 用途 |
|--------|-----------|------|
| Rust | 1.75+ | コンパイラ |
| Cargo | 1.75+ | パッケージマネージャ |
| SIPp | 3.6+ | E2E テスト |
| Zoiper | 最新 | 手動テスト |

### 9.2 初期セットアップ

```bash
# 1. リポジトリクローン
git clone <repo>
cd virtual-voicebot/virtual-voicebot-backend

# 2. ビルド確認
cargo build

# 3. テスト実行
cargo test

# 4. 開発サーバ起動
cargo run
```

### 9.3 環境変数

```bash
# SIP 設定
export SIP_BIND_IP=0.0.0.0
export SIP_PORT=5060
export ADVERTISED_IP=<your-ip>
export ADVERTISED_RTP_PORT=<rtp-port>

# HTTP 設定
export RECORDING_HTTP_ADDR=0.0.0.0:18080
export RECORDING_BASE_URL=http://localhost:18080

# Frontend 連携
export INGEST_CALL_URL=http://localhost:3000/api/ingest/call

# ログ
export RUST_LOG=info
```

---

## 10. トラブルシューティング

### 10.1 よくある問題

| 症状 | 原因 | 対処 |
|------|------|------|
| INVITE に応答しない | ポートバインド失敗 | `SIP_PORT` 確認、他プロセス確認 |
| RTP 音声が来ない | NAT/ファイアウォール | `ADVERTISED_IP` 確認 |
| ASR が失敗する | API キー/URL 設定ミス | 環境変数確認 |
| 録音が再生できない | Range 未対応 | HTTP ヘッダ確認 |

### 10.2 デバッグ手順

1. `RUST_LOG=debug` でログ確認
2. Wireshark で SIP/RTP パケット確認
3. 関連するユニットテストを実行
4. SIPp で再現テスト

---

## 11. 参照ドキュメント

| ドキュメント | 内容 |
|-------------|------|
| [../AGENTS.md](../AGENTS.md) | AI/Codex 向け詳細指示 |
| [design.md](design.md) | アーキテクチャ設計 |
| [tests.md](tests.md) | テスト計画・AC |
| [DOCS_POLICY.md](../../docs/DOCS_POLICY.md) | ドキュメントポリシー |
| [STYLE.md](../../STYLE.md) | プロジェクト共通スタイル |
