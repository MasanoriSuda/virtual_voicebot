# STEER-257: 起動時 ASR/LLM/TTS 設定ログ出力

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-257 |
| タイトル | 起動時 ASR/LLM/TTS 設定ログ出力 |
| ステータス | Draft |
| 関連Issue | #257 |
| 優先度 | P2 |
| 作成日 | 2026-02-27 |

---

## 2. ストーリー（Why）

### 2.1 背景

ASR / LLM / TTS はローカルサーバー（Whisper, Ollama, VOICEVOX 等）をデフォルトとし、ストリーミングが有効なときに最適な応答遅延が得られる。しかし現在、起動時にこれらの設定がログに出力されないため、実際の稼働設定を systemd ログ（journalctl）で確認できない。

### 2.2 目的

起動時（systemd 起動直後）に ASR / LLM / TTS の設定サマリを `info!` レベルで出力し、以下を journalctl で即座に確認できるようにする。

- 接続先 URL（ローカル / Raspberry Pi / クラウド のいずれか）
- ストリーミング有効フラグ（ASR / LLM / TTS ごと）

### 2.3 ユーザーストーリー

```
As a 運用担当者
I want to journalctl -u <unit名> で起動直後の設定サマリを確認したい
So that ASR/LLM/TTS がローカルサーバー＋ストリーミング有効で最適化されていることを即座に検証できる
```

受入条件:
- [ ] `journalctl -u <unit名>` で `startup ai-config` を含む行が ASR / LLM / TTS の 3 行出力される（unit 名は環境依存。例: `virtual-voicebot-backend`）
- [ ] ストリーミングが無効の場合は `*_streaming=false`、未設定 URL は `*_url=none` として明示出力される
- [ ] PII（音声・テキスト本文）を含まない

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-27 |
| 起票理由 | ローカルサーバー + ストリーミング最適化の確認手段が不足 |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Sonnet 4.6 |
| 作成日 | 2026-02-27 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "起動時に ASR/LLM/TTS の設定をログで確認できるようにしたい（systemd 起動契機のみ）" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| 1 | | | | |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | |
| 承認日 | |
| 承認コメント | |

### 3.5 実装（該当する場合）

| 項目 | 値 |
|------|-----|
| 実装者 | Codex |
| 実装日 | |
| 指示者 | |
| 指示内容 | |
| コードレビュー | |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ実行者 | |
| マージ日 | |
| マージ先 | （本体仕様書への反映は不要。AGENTS.md §7 の観測可能性ルールは変更なし） |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| なし | - | 既存仕様と整合。新規要件の本体仕様書への反映は今後の判断 |

### 4.2 影響するコード

| モジュール | 変更種別 | 概要 |
|-----------|---------|------|
| `src/main.rs` | 修正 | 設定ロード後にログ出力処理を追加 |

---

## 5. 差分仕様（What / How）

### 5.1 要件（本体仕様書への反映候補）

**FR-257-01: 起動時 AI 設定サマリログ出力**

#### 概要

`main()` 内の設定ロード完了直後に、ASR / LLM / TTS の設定サマリを `info!` レベルで出力する。

#### トリガー

- プロセス起動時（systemd `ExecStart` 相当）に 1 回だけ実行。
- ホットリロードは対象外（今回の範囲外）。

#### 出力する情報

| カテゴリ | 出力項目 | 取得元（`config` 関数） |
|---------|---------|----------------------|
| **ASR** | ストリーミング有効フラグ | `config::voicebot_asr_streaming_enabled()` |
| | ローカルサーバー有効フラグ | `config::ai_config().asr_local_server_enabled` |
| | ローカルサーバー URL（ストリーミング時） | `config::asr_streaming_server_url()` |
| | ローカルサーバー URL（非ストリーミング時） | `config::ai_config().asr_local_server_url` |
| | Raspberry Pi 有効フラグ | `config::ai_config().asr_raspi_enabled` |
| | Raspberry Pi URL | `config::ai_config().asr_raspi_url` |
| **LLM** | ストリーミング有効フラグ | `config::voicebot_streaming_enabled()` |
| | ローカルサーバー有効フラグ | `config::ai_config().llm_local_server_enabled` |
| | ローカルサーバー URL | `config::ai_config().llm_local_server_url` |
| | ローカルモデル名 | `config::ai_config().llm_local_model` |
| | Raspberry Pi 有効フラグ | `config::ai_config().llm_raspi_enabled` |
| | Raspberry Pi URL | `config::ai_config().llm_raspi_url` |
| **TTS** | ストリーミング有効フラグ | `config::voicebot_tts_streaming_enabled()` |
| | ローカルサーバー有効フラグ | `config::ai_config().tts_local_server_enabled` |
| | ローカルサーバーベース URL | `config::ai_config().tts_local_server_base_url` |
| | Raspberry Pi 有効フラグ | `config::ai_config().tts_raspi_enabled` |
| | Raspberry Pi URL | `config::ai_config().tts_raspi_base_url` |

#### 出力禁止

- API キー（`OPENAI_API_KEY`, `GEMINI_API_KEY` 等）は出力しない。
- 音声・テキスト本文（PII）は出力しない。
- URL に userinfo（`http://user:pass@host` 形式）が含まれる場合は除去してから出力する（資格情報のログ漏えい防止）。

#### ログ形式

1 カテゴリ 1 行を基本とし、可読性を優先する。形式例（実装者が読みやすい形にしてよい）：

```
[main] ASR  : streaming=true  local=true  url=ws://localhost:9001/ws  raspi=false
[main] LLM  : streaming=true  local=true  url=http://localhost:11434/api/chat  model=gemma3:4b  raspi=false
[main] TTS  : streaming=false local=true  url=http://localhost:50021  raspi=false
```

#### 実装箇所

`src/main.rs` の設定ロード完了直後（`session_cfg` の生成より後、サーバー起動処理より前）。

```rust
// 既存
let cfg = config::Config::from_env()?;
// ...
let session_cfg = Arc::new(config::SessionRuntimeConfig::from_env(&cfg));

// ↓ ここに追加
log_ai_config();  // 新規関数（引数なし。config::ai_config() で内部取得）
```

#### 不変条件

- ログ出力は設定値の読み取りのみ。状態を変更しない。
- `config::ai_config()` 等の `OnceLock` getter は、呼び出し時に初めて初期化される（lazy init）。`log_ai_config()` 内で呼んで初期化してよい。ただし `Config::from_env()` および `SessionRuntimeConfig::from_env()` の完了後に呼ぶことで、base config との整合が取れた状態を保つ。

### 5.2 詳細設計

#### 実装方針

`main.rs` 内に `fn log_ai_config()` を新設（private）し、`main()` から呼ぶ。
AI 設定は `config::ai_config()`（`OnceLock<AiConfig>` の getter）から取得する。

```rust
fn log_ai_config() {
    let ai = config::ai_config();

    // ASR
    let asr_url = if config::voicebot_asr_streaming_enabled() {
        config::asr_streaming_server_url().to_string()
    } else {
        ai.asr_local_server_url.clone()
    };
    let asr_raspi_url = ai.asr_raspi_url.as_deref().unwrap_or("none");
    log::info!(
        "[main] startup ai-config asr_streaming={} asr_local_enabled={} asr_local_url={} asr_raspi_enabled={} asr_raspi_url={}",
        config::voicebot_asr_streaming_enabled(),
        ai.asr_local_server_enabled,
        asr_url,  // userinfo を除去して出力
        ai.asr_raspi_enabled,
        asr_raspi_url,
    );

    // LLM
    let llm_raspi_url = ai.llm_raspi_url.as_deref().unwrap_or("none");
    log::info!(
        "[main] startup ai-config llm_streaming={} llm_local_enabled={} llm_local_url={} llm_model={} llm_raspi_enabled={} llm_raspi_url={}",
        config::voicebot_streaming_enabled(),
        ai.llm_local_server_enabled,
        ai.llm_local_server_url,  // userinfo を除去して出力
        ai.llm_local_model,
        ai.llm_raspi_enabled,
        llm_raspi_url,
    );

    // TTS
    let tts_raspi_url = ai.tts_raspi_base_url.as_deref().unwrap_or("none");
    log::info!(
        "[main] startup ai-config tts_streaming={} tts_local_enabled={} tts_local_url={} tts_raspi_enabled={} tts_raspi_url={}",
        config::voicebot_tts_streaming_enabled(),
        ai.tts_local_server_enabled,
        ai.tts_local_server_base_url,  // userinfo を除去して出力
        ai.tts_raspi_enabled,
        tts_raspi_url,
    );
}
```

> **注意**: 上記は仕様意図を示す参考実装案。Codex は同等の情報が出力されれば実装スタイルを調整してよい。
> ただし以下の推奨に従うことを期待する（変更理由がある場合はコメントに記載）。
>
> **推奨フォーマット** (`key=value` スペース区切り、プレフィックス付き):
> ```
> [main] startup ai-config asr_streaming=true asr_local_url=ws://localhost:9001/ws asr_raspi_url=none
> [main] startup ai-config llm_streaming=true llm_local_url=http://localhost:11434/api/chat llm_model=gemma3:4b llm_raspi_url=none
> [main] startup ai-config tts_streaming=false tts_local_url=http://localhost:50021 tts_raspi_url=none
> ```
>
> - **`none` 明示**: 未設定 URL は `"none"` と出力する（省略すると「未設定」と「ログ欠落」が区別できない）
> - **`key=value` 形式**: `grep key=` で検索しやすく、将来の項目追加にも強い

### 5.3 テストケース

本変更は副作用のないログ出力のみのため、ユニットテストは任意。
CI（`cargo clippy`, `cargo test`）が通ることを確認すれば十分。

確認方法（手動）:

```
VOICEBOT_ASR_STREAMING_ENABLED=true \
VOICEBOT_STREAMING_ENABLED=true \
cargo run 2>&1 | grep "startup ai-config"
```

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #257 | STEER-257 | 起票 |
| STEER-257 | `src/main.rs` | 実装対象 |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [ ] 出力項目が「ローカルサーバー URL」「ストリーミング有効フラグ」を網羅しているか
- [ ] API キー等の機密情報が出力対象に含まれていないか
- [ ] 実装箇所（`main.rs` の挿入位置）が適切か
- [ ] 既存ログ規約（AGENTS.md §7）と矛盾しないか

### 7.2 マージ前チェック（Approved → Merged）

- [ ] 実装が完了している
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` が通る
- [ ] `cargo test --all --all-features` が通る
- [ ] journalctl または標準出力で `startup ai-config` を含む行が 3 行確認できる（`grep "startup ai-config"` でマッチ）
- [ ] ストリーミング無効時は `*_streaming=false`、未設定 URL は `*_url=none` が出力される

---

## 8. 備考

- Raspberry Pi が未設定（`None`）の場合、URL は **`none` と明示出力する**（省略より `grep` での「未設定確認」がしやすく、ログ欠落との区別もできる）。
- ログ形式は **`key=value` スペース区切り＋プレフィックス `startup ai-config`** を推奨（§5.2 参考実装例を参照）。いずれも実装者（Codex）判断で変更可。変更理由をコメントに記載すること。
- 将来的にホットリロードやダッシュボードが必要になった場合は別イシューで対応する。

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-27 | 初版作成 | Claude Sonnet 4.6 |
| 2026-02-27 | レビュー指摘対応（cfg.ai→ai_config(), URL userinfo禁止, grep修正, OnceLock説明修正） | Claude Sonnet 4.6 |
| 2026-02-27 | レビュー指摘対応（unit名を環境依存注記に修正, "disabled"→false/none に統一） | Claude Sonnet 4.6 |
