<!-- SOURCE_OF_TRUTH: 実装計画 -->
# Implementation Plan (PLAN.md)

- docs/** は変更しない（Stepで明示されている場合のみ例外）
- 依存追加は禁止（必要なら別途Spec/Plan）

| 項目 | 値 |
|------|-----|
| **Status** | Active |
| **Owner** | TBD |
| **Last Updated** | 2026-01-20 |
| **SoT (Source of Truth)** | Yes - 実装計画 |
| **上流ドキュメント** | [gap-analysis.md](../gap-analysis.md), [Issue #8](https://github.com/MasanoriSuda/virtual_voicebot/issues/8), [Issue #9](https://github.com/MasanoriSuda/virtual_voicebot/issues/9), [Issue #13](https://github.com/MasanoriSuda/virtual_voicebot/issues/13), [Issue #18](https://github.com/MasanoriSuda/virtual_voicebot/issues/18), [Issue #19](https://github.com/MasanoriSuda/virtual_voicebot/issues/19), [Issue #20](https://github.com/MasanoriSuda/virtual_voicebot/issues/20), [Issue #21](https://github.com/MasanoriSuda/virtual_voicebot/issues/21), [Issue #22](https://github.com/MasanoriSuda/virtual_voicebot/issues/22), [Issue #23](https://github.com/MasanoriSuda/virtual_voicebot/issues/23), [Issue #24](https://github.com/MasanoriSuda/virtual_voicebot/issues/24), [Issue #25](https://github.com/MasanoriSuda/virtual_voicebot/issues/25), [Issue #26](https://github.com/MasanoriSuda/virtual_voicebot/issues/26) |

---

## 概要

本ドキュメントは gap-analysis.md で特定されたギャップを、仕様駆動で段階的に実装するための計画です。

**原則**:
- 1 Step = 1 PR
- 各 Step は <=200行 / <=5ファイル
- Spec変更が必要なものは Deferred Steps へ分離
- 各 Step に DoD（Definition of Done）を明記

---

## Step 一覧（UAS 優先順）

**凡例**: 依存欄の `→ Step-XX` は、そのStepの完了後に着手可能を意味する。

### P0: 最優先（NTT Docomo 接続 - Issue #13）

| Step | 概要 | 依存 | 状態 |
|------|------|------|------|
| [Step-14](#step-14-tls-トランスポート) | TLS トランスポート | - | 完了 |
| [Step-15](#step-15-uac-register-送信) | UAC REGISTER 送信 | → Step-14 | 完了 |
| [Step-16](#step-16-digest-認証401407) | Digest 認証 (401/407) | → Step-15 | 完了 |
| [Step-17](#step-17-register-リフレッシュ) | REGISTER リフレッシュ | → Step-16 | 完了 |

### P0: 必須（ボイスボット運用）

| Step | 概要 | 依存 | 状態 |
|------|------|------|------|
| [Step-18](#step-18-asr-低レイテンシ化-issue-19) | ASR 低レイテンシ化 (Issue #19) | - | 着手中 |
| [Step-19](#step-19-session-expires-対応-issue-20) | Session-Expires 対応 (Issue #20) | - | 完了 |
| [Step-20](#step-20-llm-会話履歴ロール分離-issue-21) | LLM 会話履歴ロール分離 (Issue #21) | - | 未着手 |
| [Step-21](#step-21-時間帯別イントロ-issue-22) | 時間帯別イントロ (Issue #22) | - | 未着手 |
| [Step-22](#step-22-ハルシネーション時謝罪音声-issue-23) | ハルシネーション時謝罪音声 (Issue #23) | → Step-20 | 未着手 |
| [Step-23](#step-23-ivr-メニュー機能-issue-25) | IVR メニュー機能 (Issue #25) | → Step-02 | 完了 |
| [Step-24](#step-24-bye-即時応答音声再生キャンセル-issue-26) | BYE 即時応答・音声再生キャンセル (Issue #26) | - | 未着手 |
| [Step-01](#step-01-cancel-受信処理) | CANCEL 受信処理 | - | 未着手 |
| [Step-02](#step-02-dtmf-トーン検出-goertzel) | DTMF トーン検出 (Goertzel) | - | 完了 |
| [Step-03](#step-03-sipp-cancel-シナリオ) | SIPp CANCEL シナリオ | → Step-01 | 未着手 |
| [Step-04](#step-04-dtmf-トーン検出-e2e-検証) | DTMF トーン検出 E2E 検証 | → Step-02 | 未着手 |

### P1: 重要（RFC 準拠・相互接続性）

| Step | 概要 | 依存 | 状態 |
|------|------|------|------|
| [Step-05](#step-05-rseq-ランダム化) | RSeq ランダム化 | - | 完了 |
| [Step-06](#step-06-options-応答) | OPTIONS 応答 | - | 完了 |
| [Step-07](#step-07-artpmap-パース) | a=rtpmap パース | - | 未着手 |
| [Step-08](#step-08-rtcp-sdes-cname) | RTCP SDES (CNAME) | - | 未着手 |
| [Step-09](#step-09-486-busy-here) | 486 Busy Here (Issue #18) | - | 完了 |
| [Step-12](#step-12-timer-ghij-実装) | Timer G/H/I/J 実装 | - | 未着手 |
| - | 183 Session Progress | - | 実装済み |
| - | 複数 Reliable Provisional | - | 未着手 |
| - | Contact URI 完全パース | - | 未着手 |
| - | IPv6 対応 (c=IN IP6) | - | 未着手 |

### P2: 拡張（汎用 SIP）

| Step | 概要 | 依存 | 状態 |
|------|------|------|------|
| [Step-10](#step-10-afmtp-パース) | a=fmtp パース | → Step-07 | 未着手 |
| [Step-11](#step-11-rfc-2833-dtmf-受信) | RFC 2833 DTMF 受信 | → Step-02 | 未着手 |
| [Step-13](#step-13-rtp-extensioncsrc-サポート) | RTP extension/CSRC サポート | - | 未着手 |
| - | a=ptime パース | - | 未着手 |
| - | RTCP BYE | - | 未着手 |
| - | RTCP 動的送信間隔 | - | 未着手 |
| - | RFC 3389 Comfort Noise | - | 未着手 |
| - | 5xx サーバーエラー応答 | - | 未着手 |

---

## Deferred Steps（後工程）

以下は UAS 完了後に着手する項目です。実装前に仕様/設計の決定が必要です。
Spec 策定後に Active へ昇格させます。

### UAC 機能（発信）

| ID | 項目 | RFC | 必要な決定事項 |
|----|------|-----|---------------|
| DEF-01 | UAC INVITE 送信 | 3261 §17.1.1 | UAC トランザクション状態機械の設計 |
| DEF-02 | UAC ACK/BYE 送信 | 3261 | ダイアログ管理の設計 |
| DEF-03 | DNS SRV 解決 | 3263 | resolver クレート選定、キャッシュ戦略 |
| DEF-04 | DNS NAPTR 解決 | 3263 | トランスポート自動選択ロジック |

### 認証

| ID | 項目 | RFC | 必要な決定事項 |
|----|------|-----|---------------|
| DEF-05 | Digest 認証 (UAS) | 3261 §22 | credentials ストア設計、nonce 管理 |
| DEF-06 | 401/407 チャレンジ | 3261 | realm/qop 設定方針 |
| DEF-07 | 403 Forbidden | 3261 | 認証失敗時のポリシー |

### セキュリティ

| ID | 項目 | RFC | 必要な決定事項 |
|----|------|-----|---------------|
| ~~DEF-08~~ | ~~TLS トランスポート~~ | - | **→ Step-14 に昇格（Issue #13）** |
| DEF-09 | SRTP | 3711 | キー交換方式 (SDES vs DTLS-SRTP) |

### セッション管理

| ID | 項目 | RFC | 必要な決定事項 |
|----|------|-----|---------------|
| ~~DEF-10~~ | ~~re-INVITE 送信~~ | 4028 | **→ Step-19 に統合（Issue #20）** |
| ~~DEF-11~~ | ~~UPDATE 送信~~ | 3311 | **→ Step-19 に統合（Issue #20）** |
| DEF-12 | Hold/Resume | 3264 | a=sendonly/recvonly 切り替え設計 |
| DEF-13 | 複数コーデック交渉 | 3264 | コーデック優先度、動的 PT 管理 |

### Proxy 機能

| ID | 項目 | RFC | 必要な決定事項 |
|----|------|-----|---------------|
| DEF-14 | Proxy 機能 | 3261 | Stateful/Stateless、フォーキング戦略 |
| DEF-15 | Record-Route/Route | 3261 | ルーティングテーブル設計 |
| DEF-16 | REGISTER バインディング (UAS/Registrar) | 3261 | バインディング DB、Expires 管理 |

### Registration (UAC)

> **Note**: Issue #13 により Step-15〜17 に昇格。以下は参照用。

| ID | 項目 | RFC | 状態 |
|----|------|-----|------|
| ~~DEF-19~~ | ~~UAC REGISTER 送信~~ | 3261 §10 | **→ Step-15** |
| ~~DEF-20~~ | ~~401/407 応答処理~~ | 3261 §22 | **→ Step-16** |
| ~~DEF-21~~ | ~~REGISTER リフレッシュ~~ | 3261 §10.2.4 | **→ Step-17** |
| DEF-22 | Registration 状態管理 | - | 未着手（Step-17 後に検討） |

### 転送

| ID | 項目 | RFC | 必要な決定事項 |
|----|------|-----|---------------|
| DEF-17 | REFER | 3515 | Refer-To 処理、NOTIFY 送信 |
| DEF-18 | Replaces | 3891 | ダイアログ置換ロジック |

---

## Architecture Improvements（アーキテクチャ改善）

**関連**: [Issue #8](https://github.com/MasanoriSuda/virtual_voicebot/issues/8)

> ※設計/仕様変更を伴うため、各 ARCH は Spec 策定後に Step 化（Deferred Steps 扱い）する。

**現状評価**: 総合 62/100

| 観点 | スコア | 主な課題 |
|------|--------|---------|
| Clean Architecture | 55/100 | 依存方向逆転、境界曖昧 |
| オブジェクト指向 | 58/100 | 手続き的処理、カプセル化不足 |
| トレイト活用 | 40/100 | 具象依存、ポート未定義 |
| デザインパターン | 70/100 | 一部適用済み、全体設計限定的 |

### ARCH-01: 外部I/Oのポート化（エピック）

**目的**: ASR/LLM/TTS、HTTP、ファイルI/O を trait で抽象化し、core から切り離す

**効果**: Clean Architecture スコア向上、テスト容易性向上

**状態**: 完了

> 範囲が広いため、以下のサブステップに分割する。

#### ARCH-01a: AI ポート化（ASR/LLM/TTS）

**目的**: AI 連携（ASR/LLM/TTS）を trait で抽象化

**対象ファイル**: `src/ports/ai.rs`, `src/ai/mod.rs`, `src/app/mod.rs`

**変更内容**:
- `src/ports/ai.rs` (line 15): `AiPort` trait 定義
- `src/ai/mod.rs` (line 221): trait 実装
- `src/app/mod.rs` (line 10): trait 依存に変更

**状態**: 完了

#### ARCH-01b: Ingest HTTP のポート化

**目的**: HTTP 通信（ingest 等）を trait で抽象化

**対象ファイル**: `src/http/mod.rs`, `src/http/ingest.rs`, `src/session/session.rs`

**変更内容**:
- `src/http/ingest.rs` に trait 定義（`IngestPort`）（line 1）
- `src/session/session.rs` から reqwest 直接依存を排除

**状態**: 完了

#### ARCH-01c: ファイルI/O（Recorder/再生）ポート化

**目的**: ファイル I/O（録音・再生）を trait で抽象化

**対象ファイル**: `src/recording/mod.rs`, `src/recording/storage.rs`, `src/session/session.rs`

**変更内容**:
- `src/recording/storage.rs` に trait 定義（`StoragePort`）（line 1）
- `src/recording/mod.rs` と `src/session/session.rs` を抽象化

**状態**: 完了

### ARCH-02: 純粋な状態遷移の抽出

**目的**: SIP/Session の状態遷移を純粋関数化し、I/O は外側で実行

**対象ファイル**: `src/session/types.rs` (line 135), `src/sip/mod.rs` (line 328)

**効果**: テスト容易性向上、設計の明確化

**状態**: 一部完了（Session 側のみ、SIP 側は別ステップ化が必要）

### ARCH-03: Session の責務分割

**目的**: Session が抱える責務（録音・RTP・タイマ・app連携）をサブコンポーネントに分割

**実装済み分割**:
- `SessionTimers`: セッションタイマー管理（`src/session/timers.rs` line 7）
- `AudioCapture`: 音声キャプチャ/バッファ（`src/session/capture.rs` line 3）

**未対応（TODO）**:
- `Media`: RTP 送受信の分離
- `Notifier`: app 層への通知の分離

**対象ファイル**: `src/session/session.rs` (line 60), `src/session/timers.rs`, `src/session/capture.rs`

**効果**: 単一責任の原則、可読性・保守性向上

**状態**: 一部完了（SessionTimers/AudioCapture 実装済み、Media/Notifier 未対応）

### ARCH-04: コンポジションルートの強化

**目的**: Session 内での依存生成をやめ、main.rs で依存を組み立てる

**対象ファイル**: `src/main.rs` (line 101), `src/session/session.rs` (line 68), `src/session/writing.rs` (line 15)

**効果**: 依存関係の明確化、DI パターンの適用

**状態**: 完了

### ARCH-05: 設定/環境依存の集中

**目的**: env 参照や config を各モジュールに散らさず、config で集約して注入

**対象ファイル**: `src/config.rs`, `src/sip/mod.rs`, `src/main.rs`, `src/ai/*.rs`

**効果**: テスト容易性向上、移植性向上

**状態**: 未着手

---

## Code Quality Improvements（コード品質改善）

**関連**: [Issue #8](https://github.com/MasanoriSuda/virtual_voicebot/issues/8)

> ※Codex による品質評価（2025-12-30）に基づく改善項目。

**品質評価スコア**: 総合 65/100

| 観点 | スコア | 配点 |
|------|--------|------|
| A. アーキテクチャ整合 | 21/30 | 責務境界/依存方向/循環依存 |
| B. テスト容易性 | 12/25 | 境界差し替え/ユニット可能性 |
| C. 可読性/保守性 | 13/20 | 巨大関数/状態管理/命名 |
| D. Rustらしさ | 11/15 | ownership/Result/trait適切さ |
| E. デザインパターン妥当性 | 8/10 | 必要十分/目的化回避 |

### CQ-01: Session::run のイベント別ハンドラ分割

**目的**: Session::run をイベント別ハンドラ + 共通クリーンアップに分割して責務集中を緩和

**根拠**: `src/session/session.rs` (line 117, 195, 270, 499)

**対象ファイル**: `src/session/session.rs`

**効果**: High / **コスト**: Medium / **リスク**: Low

**状態**: 未着手

### CQ-02: unbounded_channel の bounded 化

**目的**: unbounded_channel を用途別に bounded 化し、音声系は drop/間引き方針を明示

**根拠**: `src/main.rs` (line 18, 43-48), `src/session/session.rs` (line 82), `src/app/mod.rs` (line 8, 27)

**対象ファイル**: `src/main.rs`, `src/session/session.rs`, `src/app/mod.rs`

**効果**: High / **コスト**: Medium / **リスク**: Medium

**状態**: 未着手

### CQ-03: handle_conn の責務分割

**目的**: handle_conn をリクエスト解析/パス解決/レスポンス生成に分割し最小テスト追加

**根拠**: `src/http/mod.rs` (line 54, 74, 126, 176)

**対象ファイル**: `src/http/mod.rs`

**効果**: Medium / **コスト**: Low / **リスク**: Low

**状態**: 未着手

### CQ-04: AI URL の設定移動

**目的**: Whisper/Ollama URL を設定に移動し環境依存を解消

**根拠**: `src/ai/mod.rs` (line 101, 111, 190)

**対象ファイル**: `src/config.rs`, `src/ai/mod.rs`

**効果**: Medium / **コスト**: Low / **リスク**: Low

**状態**: 未着手

### CQ-05: app 履歴の上限設定

**目的**: app の履歴を上限N件/最大文字数で切り詰めてプロンプト肥大を抑制

**根拠**: `src/app/mod.rs` (line 38, 141, 157)

**対象ファイル**: `src/app/mod.rs`

**効果**: Medium / **コスト**: Low / **リスク**: Low

**状態**: 未着手

---

## Step-01: CANCEL 受信処理

**目的**: INVITE 中の CANCEL を受信し、適切に 487 を返す

**RFC参照**: RFC 3261 §9.2

### DoD (Definition of Done)

- [ ] CANCEL 受信時に INVITE トランザクションへ 487 Request Terminated を送信
- [ ] CANCEL 自体に 200 OK を応答
- [ ] Unit test 追加 (`sip/transaction.rs`)
- [ ] 既存テスト (cargo test) がパス

### 対象パス

| ファイル | 変更内容 |
|---------|---------|
| `src/sip/mod.rs` | CANCEL ハンドラ追加 |
| `src/sip/transaction.rs` | CANCEL 処理メソッド追加 |
| `src/sip/builder.rs` | 487 レスポンスビルダー追加 |

### 変更上限

- **行数**: <=150行
- **ファイル数**: <=3

### 検証方法

```bash
cargo test sip::
# E2E: Step-03 で SIPp シナリオ追加後に検証
```

---

## Step-02: DTMF トーン検出 (Goertzel)

**目的**: 音声ストリーム内の DTMF トーン（インバンド）を Goertzel アルゴリズムで検出

**関連**: [Issue #24](https://github.com/MasanoriSuda/virtual_voicebot/issues/24)

**背景**: RFC 2833 非対応の相手にも対応するため、音声信号から直接 DTMF を検出

### 技術詳細

DTMF は 2 周波数の組み合わせ:
- **Low group**: 697, 770, 852, 941 Hz
- **High group**: 1209, 1336, 1477, 1633 Hz

Goertzel アルゴリズムは特定周波数のエネルギーを効率的に計算（FFT より軽量）

### DoD (Definition of Done)

- [ ] Goertzel アルゴリズム実装（8kHz サンプリング対応）
- [ ] DTMF 8周波数のエネルギー検出
- [ ] 閾値判定で 0-9, *, # を識別
- [ ] `SessionIn::Dtmf` イベント発火
- [ ] Unit test 追加（テスト用トーン生成含む）

### 対象パス

| ファイル | 変更内容 |
|---------|---------|
| `src/rtp/dtmf.rs` | Goertzel + DTMF 検出（新規） |
| `src/rtp/mod.rs` | dtmf モジュール追加 |
| `src/rtp/rx.rs` | DTMF 検出呼び出し追加 |
| `src/session/types.rs` | SessionIn::Dtmf 追加 |

### 変更上限

- **行数**: <=200行
- **ファイル数**: <=4

### 検証方法

```bash
cargo test rtp::dtmf
# E2E: Step-04 で検証
```

### 参考

- Goertzel algorithm: [Wikipedia](https://en.wikipedia.org/wiki/Goertzel_algorithm)
- DTMF frequencies: ITU-T Q.23

---

## Step-03: SIPp CANCEL シナリオ

**目的**: Step-01 の E2E 検証用 SIPp シナリオ作成

**依存**: Step-01 完了後

**関連**: AC-4 (gap-analysis.md)

### DoD (Definition of Done)

- [ ] `test/sipp/sip/scenarios/cancel_uac.xml` 作成
- [ ] INVITE → CANCEL → 200 OK (CANCEL) + 487 (INVITE) フロー
- [ ] CI で実行可能

### 対象パス

| ファイル | 変更内容 |
|---------|---------|
| `test/sipp/sip/scenarios/cancel_uac.xml` | 新規作成 |

### 変更上限

- **行数**: <=100行
- **ファイル数**: <=1

### 検証方法

```bash
cd test/sipp/sip && sipp -sf scenarios/cancel_uac.xml -m 1 <target>
```

---

## Step-04: DTMF トーン検出 E2E 検証

**目的**: Step-02 の E2E 検証用スクリプト作成

**依存**: Step-02 完了後

**関連**: AC-6 (gap-analysis.md)

### DoD (Definition of Done)

- [ ] DTMF トーン生成・送信スクリプト作成 (Python)
- [ ] 8kHz PCMU で DTMF トーン波形を生成
- [ ] RTP パケットとして送信
- [ ] サーバーログで SessionIn::Dtmf 確認可能
- [ ] 使用手順をドキュメント化

### 対象パス

| ファイル | 変更内容 |
|---------|---------|
| `test/scripts/send_dtmf_tone.py` | 新規作成 |

### 変更上限

- **行数**: <=150行
- **ファイル数**: <=1

### 検証方法

```bash
# DTMF "5" のトーンを生成して送信
python test/scripts/send_dtmf_tone.py --target <ip:port> --digit 5 --duration 200
# サーバーログで SessionIn::Dtmf(5) を確認
```

---

## Step-05: RSeq ランダム化

**目的**: RFC 3262 準拠のため、RSeq 初期値をランダム化

**RFC参照**: RFC 3262 §3 (1〜2^31-1 の範囲でランダム推奨)

### DoD (Definition of Done)

- [x] RSeq 初期値を乱数生成（1〜2^31-1）
- [x] 既存の 100rel テストがパス
- [x] Unit test で乱数範囲を確認

### 対象パス

| ファイル | 変更内容 |
|---------|---------|
| `src/sip/mod.rs` | RSeq 生成ロジック変更 |

### 変更上限

- **行数**: <=30行
- **ファイル数**: <=1

### 検証方法

```bash
cargo test sip::
# E2E: test/sipp/sip/scenarios/basic_uas_100rel.xml
```

---

## Step-06: OPTIONS 応答

**目的**: OPTIONS リクエストに対してケイパビリティを返す

**RFC参照**: RFC 3261 §11

### DoD (Definition of Done)

- [ ] OPTIONS 受信時に 200 OK を返す
- [ ] Allow ヘッダに対応メソッド一覧を含める
- [ ] Supported ヘッダに 100rel, timer を含める
- [ ] Unit test 追加

### 対象パス

| ファイル | 変更内容 |
|---------|---------|
| `src/sip/mod.rs` | OPTIONS ハンドラ追加 |
| `src/sip/builder.rs` | OPTIONS 応答ビルダー追加 |

### 変更上限

- **行数**: <=80行
- **ファイル数**: <=2

### 検証方法

```bash
cargo test sip::
# 手動: sipp -sn uac で OPTIONS 送信確認
```

---

## Step-07: a=rtpmap パース

**目的**: SDP の a=rtpmap 行をパースしてコーデック情報を取得

**RFC参照**: RFC 8866, RFC 3264

### DoD (Definition of Done)

- [ ] `a=rtpmap:0 PCMU/8000` 形式をパース
- [ ] Sdp 構造体に rtpmap フィールド追加
- [ ] Unit test 追加
- [ ] 既存 SDP パースとの互換性維持

### 対象パス

| ファイル | 変更内容 |
|---------|---------|
| `src/session/types.rs` | Sdp 構造体拡張 |
| `src/sip/mod.rs` または `src/sip/parse.rs` | rtpmap パーサ追加 |

### 変更上限

- **行数**: <=100行
- **ファイル数**: <=2

### 検証方法

```bash
cargo test session::
cargo test sip::
```

---

## Step-08: RTCP SDES (CNAME)

**目的**: RTCP Compound パケットに SDES (CNAME) を含める

**RFC参照**: RFC 3550 §6.5.1 (MUST)

### DoD (Definition of Done)

- [ ] SDES パケット構造体追加
- [ ] CNAME 生成ロジック追加
- [ ] SR/RR 送信時に SDES を付与
- [ ] Unit test 追加

### 対象パス

| ファイル | 変更内容 |
|---------|---------|
| `src/rtp/rtcp.rs` | SDES 構造体・ビルダー追加 |
| `src/rtp/tx.rs` | SDES 送信ロジック追加 |

### 変更上限

- **行数**: <=150行
- **ファイル数**: <=2

### 検証方法

```bash
cargo test rtp::rtcp
# Wireshark で RTCP パケット確認
```

---

## Step-09: 486 Busy Here

**目的**: 通話中に新規 INVITE を受信した場合、486 Busy Here を返す

**RFC参照**: RFC 3261 §21.4.7

**関連**: [Issue #18](https://github.com/MasanoriSuda/virtual_voicebot/issues/18)

### 背景

ボイスボットは同時に 1 通話のみ対応する設計。通話中に別の INVITE が来た場合は 486 Busy Here で拒否し、発信者に「話し中」を伝える。

### DoD (Definition of Done)

- [ ] 486 レスポンスビルダー追加
- [ ] INVITE 受信時にアクティブセッション有無をチェック
- [ ] アクティブセッション存在時に 486 を返す
- [ ] max_sessions 設定追加（デフォルト: 1）
- [ ] Unit test 追加
- [ ] SIPp シナリオで E2E 検証

### 対象パス

| ファイル | 変更内容 |
|---------|---------|
| `src/sip/builder.rs` | 486 ビルダー追加 |
| `src/sip/mod.rs` | INVITE 受信時のセッション数チェック追加 |
| `src/config.rs` | max_sessions 設定追加 |

### 変更上限

- **行数**: <=100行
- **ファイル数**: <=3

### 検証方法

```bash
cargo test sip::
# E2E: 通話中に別の INVITE を送信し 486 応答を確認
```

### シーケンス

```
Call A (active)          Voicebot           Call B (new)
    |                       |                    |
    |<--- RTP (通話中) ---->|                    |
    |                       |<--- INVITE --------|
    |                       |---- 486 Busy ----->|
    |                       |                    |
```

---

## Step-10: a=fmtp パース

**目的**: SDP の a=fmtp 行をパースしてコーデックパラメータを取得

**RFC参照**: RFC 8866

### DoD (Definition of Done)

- [ ] `a=fmtp:101 0-16` 形式をパース
- [ ] Sdp 構造体に fmtp フィールド追加
- [ ] Unit test 追加

### 対象パス

| ファイル | 変更内容 |
|---------|---------|
| `src/session/types.rs` | Sdp 構造体拡張 |
| `src/sip/mod.rs` または `src/sip/parse.rs` | fmtp パーサ追加 |

### 変更上限

- **行数**: <=80行
- **ファイル数**: <=2

### 検証方法

```bash
cargo test session::
cargo test sip::
```

---

## Step-11: RFC 2833 DTMF 受信

**目的**: RTP ペイロードタイプ 101 で受信した DTMF イベントを検出

**RFC参照**: RFC 2833 / RFC 4733

**依存**: Step-02 完了後

**備考**: インバンド DTMF トーン検出（Step-02）より優先度低。RFC 2833 対応端末との相互接続用。

### DoD (Definition of Done)

- [ ] PT=101 (telephone-event) パケット検出
- [ ] DTMF イベント (0-9, *, #) をパース
- [ ] `SessionIn::Dtmf` イベント発火（Step-02 と共通）
- [ ] Unit test 追加

### 対象パス

| ファイル | 変更内容 |
|---------|---------|
| `src/rtp/dtmf.rs` | RFC 2833 パーサ追加 |
| `src/rtp/rx.rs` | RFC 2833 検出ロジック追加 |

### 変更上限

- **行数**: <=100行
- **ファイル数**: <=2

### 検証方法

```bash
cargo test rtp::dtmf
# RFC 2833 送信ツールで検証
```

---

## Step-12: Timer G/H/I/J 実装

**目的**: RFC 3261 §17.2.1/§17.2.2 準拠のトランザクションタイマーを実装

**RFC参照**: RFC 3261 §17.2.1 (INVITE サーバートランザクション), §17.2.2 (非 INVITE サーバートランザクション)

**関連**: [Issue #9](https://github.com/MasanoriSuda/virtual_voicebot/issues/9) PR-RFC-3

### 背景

- **Timer G**: INVITE final response 再送間隔（UDP の場合）
- **Timer H**: ACK 待機タイムアウト（64*T1）
- **Timer I**: 確認済み状態の保持期間（T4）
- **Timer J**: 非 INVITE 最終応答後の待機期間（64*T1）

### DoD (Definition of Done)

- [ ] Timer G 実装（T1 から 2*T1, 4*T1... と倍増、最大 T2）
- [ ] Timer H 実装（64*T1 後にトランザクション終了）
- [ ] Timer I 実装（T4 後に Confirmed → Terminated）
- [ ] Timer J 実装（64*T1 後に Completed → Terminated）
- [ ] トランザクション状態遷移図との整合性確認
- [ ] Unit test 追加

### 対象パス

| ファイル | 変更内容 |
|---------|---------|
| `src/sip/transaction.rs` | Timer G/H/I/J ロジック追加 |
| `src/sip/mod.rs` | タイマー発火ハンドリング |

### 変更上限

- **行数**: <=200行
- **ファイル数**: <=3

### 検証方法

```bash
cargo test sip::transaction
# E2E: INVITE → no ACK シナリオで Timer H 動作確認
```

---

## Step-13: RTP extension/CSRC サポート

**目的**: RTP ヘッダの拡張フィールド (X bit) と CSRC リストをパースする

**RFC参照**: RFC 3550 §5.1

**関連**: [Issue #9](https://github.com/MasanoriSuda/virtual_voicebot/issues/9) PR-RFC-4

### 背景

- **CC (CSRC Count)**: ミキサー環境で複数ソースを識別
- **X bit**: 拡張ヘッダの存在を示す
- 現状は CC=0, X=0 前提でパースしているため、拡張ヘッダ付きパケットを正しく処理できない

### DoD (Definition of Done)

- [ ] RTP ヘッダの CC フィールドをパース
- [ ] CSRC リスト (CC*4 バイト) を読み飛ばし
- [ ] X bit = 1 の場合、拡張ヘッダをパース/スキップ
- [ ] Unit test 追加（拡張ヘッダ付きパケット）

### 対象パス

| ファイル | 変更内容 |
|---------|---------|
| `src/rtp/packet.rs` | RTP ヘッダパーサ拡張 |
| `src/rtp/rx.rs` | 拡張ヘッダ対応 |

### 変更上限

- **行数**: <=100行
- **ファイル数**: <=2

### 検証方法

```bash
cargo test rtp::packet
# 拡張ヘッダ付き RTP パケットでテスト
```

---

## Step-14: TLS トランスポート

**目的**: SIP over TLS (SIPS) をサポートし、NTT Docomo 等のキャリア接続を可能にする

**RFC参照**: RFC 3261 §26, RFC 5246 (TLS 1.2)

**関連**: [Issue #13](https://github.com/MasanoriSuda/virtual_voicebot/issues/13)

### DoD (Definition of Done)

- [x] TLS 1.2/1.3 対応のトランスポート層追加
- [x] rustls または native-tls クレート導入
- [x] 証明書検証（CA 検証 or 自己署名許可オプション）
- [ ] SIPS URI スキーム対応
- [x] Unit test 追加

### 対象パス

| ファイル | 変更内容 |
|---------|---------|
| `src/transport/mod.rs` | TLS トランスポート追加 |
| `src/transport/tls.rs` | TLS 接続ロジック（新規） |
| `src/config.rs` | TLS 設定（証明書パス等）追加 |
| `Cargo.toml` | rustls/native-tls 依存追加 |

### 変更上限

- **行数**: <=300行
- **ファイル数**: <=5

### 検証方法

```bash
cargo test transport::tls
# E2E: TLS 対応 SIP サーバーへの接続確認
```

### 設計決定事項

| 項目 | 決定 | 理由 |
|------|------|------|
| TLS ライブラリ | rustls（推奨） | pure Rust、OpenSSL 依存なし |
| 証明書検証 | CA 検証デフォルト | セキュリティ優先 |
| 認証情報管理 | 環境変数 | コンテナ親和性 |

---

## Step-15: UAC REGISTER 送信

**目的**: SIP Registrar に REGISTER リクエストを送信し、着信可能な状態にする

**RFC参照**: RFC 3261 §10

**関連**: [Issue #13](https://github.com/MasanoriSuda/virtual_voicebot/issues/13)

**依存**: Step-14 (TLS)

### DoD (Definition of Done)

- [x] REGISTER リクエスト生成・送信
- [x] 200 OK 受信処理
- [x] Contact URI の適切な設定
- [x] Expires ヘッダの設定（デフォルト 3600秒）
- [x] Unit test 追加

### 対象パス

| ファイル | 変更内容 |
|---------|---------|
| `src/sip/register.rs` | REGISTER 送信ロジック（新規） |
| `src/sip/builder.rs` | REGISTER リクエストビルダー追加 |
| `src/sip/mod.rs` | register モジュール追加 |
| `src/config.rs` | Registrar 設定追加 |

### 変更上限

- **行数**: <=200行
- **ファイル数**: <=4

### 検証方法

```bash
cargo test sip::register
# E2E: SIP Registrar への登録確認
```

---

## Step-16: Digest 認証 (401/407)

**目的**: 401 Unauthorized / 407 Proxy Authentication Required に対して Digest 認証で再送信

**RFC参照**: RFC 3261 §22, RFC 2617

**関連**: [Issue #13](https://github.com/MasanoriSuda/virtual_voicebot/issues/13)

**依存**: Step-15 (REGISTER)

### DoD (Definition of Done)

- [x] 401/407 レスポンスのパース
- [x] WWW-Authenticate / Proxy-Authenticate ヘッダ解析
- [x] MD5 ハッシュ計算（response 生成）
- [x] Authorization / Proxy-Authorization ヘッダ付きリクエスト再送
- [x] nonce/cnonce/nc カウント管理
- [x] Unit test 追加

### 対象パス

| ファイル | 変更内容 |
|---------|---------|
| `src/sip/auth.rs` | Digest 認証ロジック（新規） |
| `src/sip/register.rs` | 認証応答処理追加 |
| `src/config.rs` | 認証情報（username/password）追加 |

### 変更上限

- **行数**: <=250行
- **ファイル数**: <=4

### 検証方法

```bash
cargo test sip::auth
# E2E: 認証付き REGISTER 確認
```

---

## Step-17: REGISTER リフレッシュ

**目的**: Expires 前に自動的に再登録し、登録状態を維持する

**RFC参照**: RFC 3261 §10.2.4

**関連**: [Issue #13](https://github.com/MasanoriSuda/virtual_voicebot/issues/13)

**依存**: Step-16 (Digest 認証)

### DoD (Definition of Done)

- [x] Expires 値に基づく再登録タイマー（Expires * 0.8 推奨）
- [x] 再登録失敗時のリトライロジック
- [x] 登録状態の通知（成功/失敗/期限切れ）
- [x] graceful shutdown 時の登録解除（Expires: 0）
- [x] Unit test 追加

### 対象パス

| ファイル | 変更内容 |
|---------|---------|
| `src/sip/register.rs` | リフレッシュタイマー追加 |
| `src/sip/mod.rs` | 登録状態管理追加 |

### 変更上限

- **行数**: <=150行
- **ファイル数**: <=3

### 検証方法

```bash
cargo test sip::register
# E2E: 長時間運用での再登録確認
```

---

## Step-18: ASR 低レイテンシ化 (Issue #19)

**目的**: 発話終了を自然に検出し、体感 2 秒以内で ASR 結果を得る

**関連**: [Issue #19](https://github.com/MasanoriSuda/virtual_voicebot/issues/19)

### 背景

現在の実装は 10 秒固定待ちのため、ユーザーに「壊れた？」と思われる体験になっている。
VAD（Voice Activity Detection）+ 無音検出により、発話終了を自然に判定し低レイテンシ化する。

### 技術アプローチ

```
[RTP音声] → [VAD: 発話開始検出] → [音声バッファリング] → [無音検出: 発話終了] → [Whisper ASR] → [結果]
                                                                ↓
                                                        無音閾値: 800ms
```

### DoD (Definition of Done)

- [ ] VAD による発話開始/終了検出
- [ ] 開始待ち時間（デフォルト 800ms）- 通話開始後の無音を許容
- [ ] 終了無音検出（デフォルト 800ms）- 発話後の無音で終了判定
- [ ] 発話区間のみを Whisper に送信
- [ ] 10 秒固定待ちの廃止
- [ ] 体感レイテンシ 2 秒以内の達成
- [ ] Unit test 追加
- [ ] E2E 検証（実際の通話で確認）

### 対象パス

| ファイル | 変更内容 |
|---------|---------|
| `src/session/capture.rs` | VAD + 無音検出ロジック追加 |
| `src/session/session.rs` | 発話区間判定の統合 |
| `src/ai/mod.rs` | ASR 呼び出しタイミング変更 |
| `src/config.rs` | VAD 閾値設定追加 |

### 変更上限

- **行数**: <=300行
- **ファイル数**: <=5

### 検証方法

```bash
cargo test session::capture
# E2E: 実際の通話で発話→応答のレイテンシを計測
```

### 設計決定事項

| 項目 | 決定 | 理由 |
|------|------|------|
| VAD 方式 | エネルギーベース（RMS 閾値） | シンプル、低負荷 |
| 開始待ち時間 | 800ms（設定可能） | 通話開始時の無音を待つ |
| 終了無音閾値 | 800ms（設定可能） | 自然な発話終了判定 |
| 最小発話長 | 300ms | ノイズ誤検出防止 |
| 最大発話長 | 30s | メモリ保護 |

### 環境変数（追加）

| 変数名 | 説明 | デフォルト |
|--------|------|-----------|
| `VAD_START_SILENCE_MS` | 開始待ち時間（ms）- 通話開始後、この時間内は無音でも待機 | `800` |
| `VAD_END_SILENCE_MS` | 終了無音閾値（ms）- 発話後、この時間無音で発話終了と判定 | `800` |
| `VAD_MIN_SPEECH_MS` | 最小発話長（ms） | `300` |
| `VAD_MAX_SPEECH_MS` | 最大発話長（ms） | `30000` |
| `VAD_ENERGY_THRESHOLD` | エネルギー閾値（RMS） | `500` |

### シーケンス

```
User                    Voicebot                    Whisper
  |                         |                          |
  |                         | [0.8秒待機（開始無音許容）]|
  |--- 発話開始 ----------->|                          |
  |                         | [VAD: 発話検出]          |
  |                         | [バッファリング開始]     |
  |--- 発話中 ------------->|                          |
  |                         | [音声蓄積]               |
  |--- 発話終了（無音）---->|                          |
  |                         | [800ms 無音検出]         |
  |                         |--- WAV 送信 ------------>|
  |                         |<-- テキスト結果 ---------|
  |<-- 応答（< 2秒）--------|                          |
```

---

## Step-19: Session-Expires 対応 (Issue #20)

**目的**: RFC 4028 準拠のセッションタイマーを実装し、長時間通話に対応する

**関連**: [Issue #20](https://github.com/MasanoriSuda/virtual_voicebot/issues/20)

**RFC参照**: RFC 4028 (Session Timers in SIP)

### 背景

現在の実装では、セッションタイムアウトが 120 秒（2 分）に固定されている:

```rust
const SESSION_TIMEOUT: Duration = Duration::from_secs(120);
```

NTT Docomo からの INVITE には Session-Expires ヘッダが含まれており、この値を無視すると相手側がタイムアウトで BYE を送信する可能性がある。

### 現状の問題

1. **固定タイムアウト**: 120 秒で強制的にセッション終了
2. **Session-Expires 無視**: INVITE の Session-Expires ヘッダを解析していない
3. **re-INVITE/UPDATE 未対応**: RFC 4028 準拠のセッションリフレッシュ機能がない

### 技術アプローチ

RFC 4028 準拠の実装:
1. INVITE の Session-Expires ヘッダを解析
2. Session-Expires がある場合はその値を使用
3. Session-Expires がない場合は環境変数のデフォルト値を使用
4. refresher に応じて re-INVITE を送信しセッションをリフレッシュ

```
INVITE 例:
Session-Expires: 1800;refresher=uac
Min-SE: 90
```

### DoD (Definition of Done)

- [ ] INVITE の Session-Expires ヘッダ解析
- [ ] Min-SE ヘッダ解析（最小セッション時間）
- [ ] refresher パラメータ解析（uac/uas）
- [ ] 200 OK に Session-Expires ヘッダを含める
- [ ] Session-Expires 値に基づくセッションタイマー設定
- [ ] **refresher=uas** の場合、期限前（80%）に re-INVITE または UPDATE 送信
- [ ] **refresher=uac** の場合、相手からの re-INVITE/UPDATE を受信・処理
- [ ] re-INVITE 受信時に 200 OK を返しセッションタイマーをリセット
- [ ] UPDATE 受信時に 200 OK を返しセッションタイマーをリセット
- [ ] Session-Expires がない場合のデフォルト値対応
- [ ] `SESSION_TIMEOUT_SEC` 環境変数追加（デフォルト用）
- [ ] Unit test 追加
- [ ] README に環境変数追加

### 対象パス

| ファイル | 変更内容 |
|---------|---------|
| `src/sip/mod.rs` | Session-Expires/Min-SE ヘッダ解析、UPDATE 受信処理 |
| `src/sip/builder.rs` | 200 OK に Session-Expires 追加 |
| `src/sip/reinvite.rs` | re-INVITE 送信ロジック（新規） |
| `src/sip/update.rs` | UPDATE 送受信ロジック（新規） |
| `src/session/session.rs` | セッションタイマー統合 |
| `src/session/timers.rs` | Session-Expires タイマー追加 |
| `src/config.rs` | `SessionConfig` 追加 |
| `README.md` | 環境変数ドキュメント追加 |

### 変更上限

- **行数**: <=500行
- **ファイル数**: <=8

### 検証方法

```bash
cargo test sip::
cargo test session::
# E2E: NTT Docomo からの通話で Session-Expires が正しく処理されることを確認
# E2E: 長時間通話（> Session-Expires）で re-INVITE によるリフレッシュを確認
```

### 環境変数（追加）

| 変数名 | 説明 | デフォルト |
|--------|------|-----------|
| `SESSION_TIMEOUT_SEC` | デフォルトセッションタイムアウト（秒）。INVITE に Session-Expires がない場合に使用。`0` で無制限。 | `1800` |
| `SESSION_MIN_SE` | 最小セッション時間（秒）。相手の Min-SE より小さい場合は相手の値を使用。 | `90` |

### 統合 Deferred Steps

> **Note**: 本 Step は DEF-10/DEF-11 を統合し、RFC 4028/RFC 3311 準拠の完全実装を行う。

- ~~**DEF-10**~~: re-INVITE 送信 → **本 Step に統合**
- ~~**DEF-11**~~: UPDATE 送信 → **本 Step に統合**

### re-INVITE vs UPDATE

| 方式 | RFC | 特徴 | 用途 |
|------|-----|------|------|
| re-INVITE | 3261 | ACK が必要、SDP 再ネゴシエーション可能 | セッションリフレッシュ + メディア変更 |
| UPDATE | 3311 | ACK 不要、軽量 | セッションリフレッシュのみ |

> **実装方針**: 送信時は UPDATE を優先（軽量）。受信時は re-INVITE/UPDATE 両方に対応。

### シーケンス

#### ケース 1: refresher=uas（Voicebot がリフレッシュ責任 - UPDATE 使用）

```
NTT Docomo                         Voicebot
     |                                 |
     |--- INVITE ----------------------|
     |    Session-Expires: 1800        |
     |    Min-SE: 90                   |
     |                                 |
     |<-- 200 OK ----------------------|
     |    Session-Expires: 1800;       |
     |    refresher=uas                |
     |                                 |
     |--- ACK -------------------------|
     |                                 |
     |<== RTP 通話 ====================>|
     |                                 |
     |... 1440秒経過 (80%) ............|
     |                                 |
     |<-- UPDATE ---------------------|  ← Voicebot が UPDATE を送信（軽量）
     |    Session-Expires: 1800        |
     |                                 |
     |--- 200 OK ----------------------|  ← ACK 不要
     |                                 |
     |<== 通話継続 ===================>|  ← セッションタイマーリセット
     |                                 |
```

#### ケース 2: refresher=uac（NTT Docomo がリフレッシュ責任 - re-INVITE/UPDATE 受信）

```
NTT Docomo                         Voicebot
     |                                 |
     |--- INVITE ----------------------|
     |    Session-Expires: 1800;       |
     |    refresher=uac                |
     |    Min-SE: 90                   |
     |                                 |
     |<-- 200 OK ----------------------|
     |    Session-Expires: 1800;       |
     |    refresher=uac                |  ← 相手の refresher を維持
     |                                 |
     |--- ACK -------------------------|
     |                                 |
     |<== RTP 通話 ====================>|
     |                                 |
     |... 1440秒経過 (80%) ............|
     |                                 |
     |--- re-INVITE or UPDATE ---------|  ← NTT Docomo が送信
     |    Session-Expires: 1800        |
     |                                 |
     |<-- 200 OK ----------------------|  ← Voicebot は受信して応答
     |    Session-Expires: 1800;       |
     |    refresher=uac                |
     |                                 |
     |--- ACK (re-INVITE の場合のみ) --|
     |                                 |
     |<== 通話継続 ===================>|  ← セッションタイマーリセット
     |                                 |
```

### フォールバック動作

```
Session-Expires なしの INVITE:

UAC                              Voicebot
  |                                 |
  |--- INVITE ----------------------|
  |    (Session-Expires なし)       |
  |                                 |
  |<-- 200 OK ----------------------|
  |    Session-Expires: 1800        |  ← デフォルト値を使用
  |    refresher=uas                |
  |                                 |
```

---

## Step-20: LLM 会話履歴ロール分離 (Issue #21)

**目的**: LLM API に対して適切なロール（system/user/assistant）で会話履歴を渡し、文脈を正しく理解させる

**関連**: [Issue #21](https://github.com/MasanoriSuda/virtual_voicebot/issues/21)

### 背景

現在の実装では、会話履歴が以下のように処理されている:

1. `app/mod.rs` の `build_prompt()` が `User: ... Bot: ...` 形式の文字列を生成
2. `ai/mod.rs` の `build_llm_prompt()` が「以下の質問に120文字以内にまとめてください。質問: {履歴込み文字列}」でラップ
3. すべてが単一の `user` ロールとして Ollama に送信される

この結果、LLM は会話の文脈を正しく理解できず、過去の応答内容を「質問」として解釈してしまう。

### 技術アプローチ

OpenAI 互換の `messages[]` 形式を使用:

```json
{
  "model": "gemma3:4b",
  "messages": [
    {"role": "system", "content": "あなたはボイスボットです。120文字以内で回答してください。"},
    {"role": "user", "content": "最初の質問"},
    {"role": "assistant", "content": "最初の回答"},
    {"role": "user", "content": "2番目の質問"}
  ],
  "stream": false
}
```

### DoD (Definition of Done)

#### LLM ロール分離
- [ ] `ChatMessage` 構造体と `Role` enum (User/Assistant) を追加
- [ ] `AiPort::generate_answer` の引数を `Vec<ChatMessage>` に変更
- [ ] `call_ollama` を `messages[]` 形式に対応
- [ ] `call_gemini` を `contents[]` 形式に対応
- [ ] システムプロンプト追加（120文字制限等）
- [ ] `app/mod.rs` の `build_prompt()` を削除
- [ ] `ai/mod.rs` の `build_llm_prompt()` を削除
- [ ] 履歴を `Vec<ChatMessage>` として管理するよう変更

#### Whisper ハルシネーション対策
- [ ] ASR 結果フィルタ追加（無音時の誤認識パターンを除外）
- [ ] フィルタ対象パターン: 「ご視聴ありがとうございました」「チャンネル登録」「いいね」等
- [ ] フィルタにマッチした場合は空文字列を返す（LLM に渡さない）

#### 検証
- [ ] Unit test 追加
- [ ] E2E 検証（連続質問で文脈が正しく維持されることを確認）
- [ ] E2E 検証（無音時にハルシネーションが LLM に渡らないことを確認）

### 対象パス

| ファイル | 変更内容 |
|---------|---------|
| `src/ports/ai.rs` | `ChatMessage`, `Role` 型追加、`generate_answer` 引数変更 |
| `src/ai/mod.rs` | `call_ollama`/`call_gemini` を messages 形式に変更、`build_llm_prompt` 削除 |
| `src/ai/asr.rs` | Whisper ハルシネーションフィルタ追加 |
| `src/app/mod.rs` | `build_prompt` 削除、履歴を `Vec<ChatMessage>` で管理 |

### 変更上限

- **行数**: <=200行
- **ファイル数**: <=4

### 検証方法

```bash
cargo test ai::
cargo test app::
# E2E: 連続質問で文脈が正しく維持されることを確認
# 例: "東京の天気は？" → "大阪は？" で大阪の天気について回答すること
```

### 型定義（案）

```rust
// src/ports/ai.rs
#[derive(Debug, Clone)]
pub enum Role {
    User,
    Assistant,
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
}
```

### Whisper ハルシネーションフィルタ（案）

```rust
// src/ai/asr.rs
const HALLUCINATION_PATTERNS: &[&str] = &[
    "ご視聴ありがとうございました",
    "チャンネル登録",
    "高評価",
    "いいね",
    "お願いします", // 単独で出現する場合
    "ありがとうございました", // 単独で出現する場合
];

/// ASR 結果がハルシネーションかどうかを判定
fn is_hallucination(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return true;
    }
    HALLUCINATION_PATTERNS.iter().any(|p| trimmed.contains(p))
}

/// ASR 結果をフィルタリング
pub fn filter_asr_result(text: &str) -> Option<String> {
    if is_hallucination(text) {
        log::debug!("ASR hallucination filtered: {}", text);
        None
    } else {
        Some(text.to_string())
    }
}
```

> **Note**: フィルタパターンは運用中に追加・調整が必要。環境変数での設定も検討。

### Ollama API 形式

```rust
// messages 形式への変換
let messages: Vec<OllamaMessage> = vec![
    OllamaMessage {
        role: "system".to_string(),
        content: "あなたはボイスボットです。120文字以内で回答してください。".to_string(),
    },
];
// 履歴を追加
for msg in &history {
    messages.push(OllamaMessage {
        role: match msg.role {
            Role::User => "user".to_string(),
            Role::Assistant => "assistant".to_string(),
        },
        content: msg.content.clone(),
    });
}
```

### シーケンス

```
User                    AppWorker                   AI (Ollama)
  |                         |                           |
  |--- 音声("東京の天気") --->|                           |
  |                         |                           |
  |                         | history = []              |
  |                         | messages = [              |
  |                         |   {system: "..."},        |
  |                         |   {user: "東京の天気"}    |
  |                         | ]                         |
  |                         |--- POST /api/chat ------->|
  |                         |                           |
  |                         |<-- "東京は晴れです" ------|
  |                         |                           |
  |                         | history.push(user, asst)  |
  |                         |                           |
  |<-- 音声("東京は晴れ") ---|                           |
  |                         |                           |
  |--- 音声("大阪は？") ---->|                           |
  |                         |                           |
  |                         | messages = [              |
  |                         |   {system: "..."},        |
  |                         |   {user: "東京の天気"},   |
  |                         |   {asst: "東京は晴れ"},   |
  |                         |   {user: "大阪は？"}      |  ← 文脈を維持
  |                         | ]                         |
  |                         |--- POST /api/chat ------->|
  |                         |                           |
  |                         |<-- "大阪は曇りです" ------|  ← 正しく大阪の天気を回答
  |                         |                           |
  |<-- 音声("大阪は曇り") ---|                           |
```

---

## Step-21: 時間帯別イントロ (Issue #22)

**目的**: 時間帯によってイントロ音声を切り替え、「おはよう」「こんにちは」「こんばんは」の挨拶を自然にする

**関連**: [Issue #22](https://github.com/MasanoriSuda/virtual_voicebot/issues/22)

### 背景

現在の実装では、イントロ音声は固定パス（`zundamon_intro.wav`）を使用している:

```rust
// src/session/session.rs:27-30
const INTRO_WAV_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/test/simpletest/audio/zundamon_intro.wav"
);
```

時間帯に応じた挨拶に変更することで、ユーザー体験を向上させる。

### 時間帯定義

| 時間帯 | 開始 | 終了 | 挨拶 | ファイル |
|--------|------|------|------|----------|
| 朝 | 05:00 | 11:59 | おはよう | `data/zundamon_intro_morning.wav` |
| 昼 | 12:00 | 16:59 | こんにちは | `data/zundamon_intro_afternoon.wav` |
| 夜 | 17:00 | 04:59 | こんばんは | `data/zundamon_intro_evening.wav` |

### DoD (Definition of Done)

- [ ] `get_intro_wav_path()` 関数追加（現在時刻から適切なパスを返す）
- [ ] 時間帯判定ロジック実装（ローカルタイム使用）
- [ ] `INTRO_WAV_PATH` 定数を関数呼び出しに置換
- [ ] 3つのイントロ WAV ファイルが `data/` に存在することを確認
- [ ] Unit test 追加（各時間帯でのパス判定）
- [ ] E2E 検証（実際の通話で正しい挨拶が再生されることを確認）

### 対象パス

| ファイル | 変更内容 |
|---------|---------|
| `src/session/session.rs` | `INTRO_WAV_PATH` → `get_intro_wav_path()` 関数化 |
| `data/zundamon_intro_morning.wav` | 朝用イントロ（既存） |
| `data/zundamon_intro_afternoon.wav` | 昼用イントロ（既存） |
| `data/zundamon_intro_evening.wav` | 夜用イントロ（既存） |

### 変更上限

- **行数**: <=50行
- **ファイル数**: <=1（コード変更）

### 検証方法

```bash
cargo test session::
# E2E: 各時間帯に通話して正しい挨拶が再生されることを確認
```

### 実装案

```rust
// src/session/session.rs
use chrono::Local;

fn get_intro_wav_path() -> &'static str {
    let hour = Local::now().hour();
    match hour {
        5..=11 => concat!(env!("CARGO_MANIFEST_DIR"), "/data/zundamon_intro_morning.wav"),
        12..=16 => concat!(env!("CARGO_MANIFEST_DIR"), "/data/zundamon_intro_afternoon.wav"),
        _ => concat!(env!("CARGO_MANIFEST_DIR"), "/data/zundamon_intro_evening.wav"),
    }
}
```

> **Note**: `chrono` クレートが必要（既に依存に含まれている場合は追加不要）。

---

## Step-22: ハルシネーション時謝罪音声 (Issue #23)

**目的**: Whisper がハルシネーションを起こした際、謝罪音声を再生してユーザーに再度発話を促す

**関連**: [Issue #23](https://github.com/MasanoriSuda/virtual_voicebot/issues/23)

**依存**: Step-20（ハルシネーションフィルタ実装後）

### 背景

Step-20 で Whisper ハルシネーション（「ご視聴ありがとうございました」等）をフィルタし、LLM に渡さないようにする。
本 Step では、フィルタでハルシネーションを検出した場合に、謝罪音声を再生して会話を継続させる。

### 動作フロー

```
User（無音）→ Whisper「ご視聴ありがとうございました」→ フィルタ検出 → LLMスキップ
                                                        ↓
                                            謝罪音声再生（zundamon_sorry.wav）
                                                        ↓
                                            キャプチャ再開（ユーザーの再発話待ち）
```

### DoD (Definition of Done)

- [ ] ハルシネーション検出時に `SessionOut::AppSendBotAudioFile` で謝罪音声を送信
- [ ] 謝罪音声ファイル: `data/zundamon_sorry.wav`
- [ ] 謝罪音声再生後、キャプチャを再開して次の発話を待つ
- [ ] Unit test 追加
- [ ] E2E 検証（無音→謝罪音声→再発話で正常動作を確認）

### 対象パス

| ファイル | 変更内容 |
|---------|---------|
| `src/app/mod.rs` | ハルシネーション検出時に謝罪音声送信を追加 |
| `data/zundamon_sorry.wav` | 謝罪音声（既存） |

### 変更上限

- **行数**: <=30行
- **ファイル数**: <=1（コード変更）

### 検証方法

```bash
cargo test app::
# E2E: 無音→ハルシネーション検出→謝罪音声再生→再発話で正常応答を確認
```

### 実装案

```rust
// src/app/mod.rs (handle_audio_buffer 内)
let trimmed = user_text.trim();
if trimmed.is_empty() {
    log::debug!("[app {call_id}] empty ASR text after filtering, playing sorry audio");
    // 謝罪音声を再生
    let sorry_path = concat!(env!("CARGO_MANIFEST_DIR"), "/data/zundamon_sorry.wav");
    let _ = self.session_out_tx.send((
        self.call_id.clone(),
        SessionOut::AppSendBotAudioFile { path: sorry_path.to_string() },
    ));
    return Ok(());
}
```

### シーケンス

```
User                    AppWorker                   Session
  |                         |                          |
  |--- 無音 -------------->|                          |
  |                         |                          |
  |                         | ASR: "ご視聴ありがとう"  |
  |                         | filter: ハルシネーション |
  |                         | → 空文字列               |
  |                         |                          |
  |                         |--- sorry.wav ----------->|
  |                         |                          |
  |<-- "すみません..." ----|<-------------------------|
  |                         |                          |
  |                         | キャプチャ再開           |
  |                         |                          |
  |--- 再発話 ------------>|                          |
  |                         | ASR: 正常テキスト        |
  |                         | → LLM 呼び出し           |
```

---

## Step-23: IVR メニュー機能 (Issue #25)

**目的**: 通話開始時に DTMF メニューを提供し、番号選択に応じて異なる動作を実行する

**関連**: [Issue #25](https://github.com/MasanoriSuda/virtual_voicebot/issues/25)

**依存**: Step-02（DTMF トーン検出 Goertzel）

### 背景

通話開始時にイントロ音声でメニューを案内し、DTMF 入力に応じて処理を分岐させる IVR（Interactive Voice Response）機能を実装する。

### 動作フロー

```
通話開始
    ↓
zundamon_intro_ivr.wav 再生（「1, 2, 9 のいずれかを押してください」）
    ↓
DTMF 待機
    ↓
┌─────────────────────────────────────────────────────┐
│ 1 検出 → ずんだもんボイスボット開始（既存機能）      │
│          → DTMF 検出停止                            │
├─────────────────────────────────────────────────────┤
│ 2 検出 → zundamon_sendai.wav 再生                   │
│          → 再生後、DTMF 待機に戻る（継続待機）      │
├─────────────────────────────────────────────────────┤
│ 9 検出 → zundamon_intro_ivr_again.wav 再生          │
│          → DTMF 待機に戻る（ループ）                │
├─────────────────────────────────────────────────────┤
│ その他 → zundamon_invalid.wav 再生                  │
│          → zundamon_intro_ivr_again.wav 再生        │
│          → DTMF 待機に戻る（ループ）                │
├─────────────────────────────────────────────────────┤
│ タイムアウト → zundamon_intro_ivr_again.wav 再生    │
│                → DTMF 待機に戻る（ループ）          │
└─────────────────────────────────────────────────────┘
```

### 状態遷移

```
                    ┌──────────────────────┐
                    │  IvrMenuWaiting      │ ← 初期状態
                    │  (DTMF待機中)        │
                    └──────────┬───────────┘
                               │
            ┌──────────────────┼──────────────────┬──────────────┐
            │                  │                  │              │
        DTMF=1             DTMF=2              DTMF=9/other  タイムアウト
            │                  │                  │              │
            ▼                  ▼                  │              │
┌─────────────────┐  ┌─────────────────┐         │              │
│  VoicebotMode   │  │  InfoPlayback   │         │              │
│  (既存会話機能) │  │  (仙台案内再生) │         │              │
│  DTMF検出停止   │  └────────┬────────┘         │              │
└─────────────────┘           │                  │              │
                              │                  │              │
                              ▼                  ▼              ▼
                    ┌──────────────────────────────────────────────┐
                    │              IvrMenuWaiting                  │
                    │            (DTMF待機に戻る)                  │
                    └──────────────────────────────────────────────┘
```

### DoD (Definition of Done)

#### セッション状態管理
- [ ] `IvrState` enum 追加（`IvrMenuWaiting`, `VoicebotMode`）
- [ ] Session に `ivr_state` フィールド追加
- [ ] DTMF 検出の有効/無効切り替え機能
- [ ] IVR タイムアウトタイマー追加（設定可能、デフォルト 10秒）

#### DTMF ハンドリング
- [ ] `session.rs` に `SessionIn::Dtmf` ハンドラ追加
- [ ] 検出された数字に応じた分岐処理
- [ ] DTMF 検出時にログ出力

#### 音声ファイル再生
- [ ] イントロ: `data/zundamon_intro_ivr.wav`（メニュー案内）
- [ ] 仙台案内: `data/zundamon_sendai.wav`
- [ ] 再案内: `data/zundamon_intro_ivr_again.wav`
- [ ] 無効入力: `data/zundamon_invalid.wav`

#### 検証
- [ ] Unit test 追加（状態遷移テスト）
- [ ] E2E 検証（各 DTMF 入力で正しい動作を確認）

### 対象パス

| ファイル | 変更内容 |
|---------|---------|
| `src/session/session.rs` | IVR 状態管理、DTMF ハンドラ追加、タイムアウト処理 |
| `src/session/types.rs` | `IvrState` enum 追加 |
| `src/rtp/rx.rs` | DTMF 検出ログ追加（任意） |
| `data/zundamon_intro_ivr.wav` | IVR イントロ（既存） |
| `data/zundamon_intro_ivr_again.wav` | 再案内（既存） |
| `data/zundamon_sendai.wav` | 仙台案内（既存） |
| `data/zundamon_invalid.wav` | 無効入力（既存） |

### 変更上限

- **行数**: <=300行
- **ファイル数**: <=4（コード変更）

### 環境変数（追加）

| 変数名 | 説明 | デフォルト |
|--------|------|-----------|
| `IVR_TIMEOUT_SEC` | IVR メニューのタイムアウト秒数。タイムアウト時は再案内を再生。 | `10` |

### 型定義（案）

```rust
// src/session/types.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IvrState {
    #[default]
    IvrMenuWaiting,  // IVR メニュー待機中（DTMF 検出有効）
    VoicebotMode,    // ボイスボットモード（DTMF 検出無効）
}
```

### 実装案

```rust
// src/session/session.rs

const IVR_INTRO_WAV_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/data/zundamon_intro_ivr.wav");
const IVR_INTRO_AGAIN_WAV_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/data/zundamon_intro_ivr_again.wav");
const SENDAI_WAV_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/data/zundamon_sendai.wav");
const INVALID_WAV_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/data/zundamon_invalid.wav");

// match ブロック内
(SessState::Established, SessionIn::Dtmf { digit }) => {
    info!("[session {}] DTMF received: '{}'", self.call_id, digit);

    if self.ivr_state != IvrState::IvrMenuWaiting {
        // IVR メニュー待機中以外は無視
        debug!("[session {}] ignoring DTMF in {:?}", self.call_id, self.ivr_state);
        continue;
    }

    // IVR タイムアウトタイマーをリセット
    self.reset_ivr_timeout();

    match digit {
        '1' => {
            // ボイスボットモードに移行
            info!("[session {}] switching to voicebot mode", self.call_id);
            self.ivr_state = IvrState::VoicebotMode;
            self.stop_ivr_timeout();
            self.capture.start();  // 音声キャプチャ開始
        }
        '2' => {
            // 仙台案内を再生後、メニュー待機に戻る
            info!("[session {}] playing sendai info", self.call_id);
            self.play_audio(SENDAI_WAV_PATH).await;
            self.play_audio(IVR_INTRO_AGAIN_WAV_PATH).await;
            self.reset_ivr_timeout();
            // ivr_state は IvrMenuWaiting のまま
        }
        '9' => {
            // 再案内を再生してループ
            info!("[session {}] replaying menu", self.call_id);
            self.play_audio(IVR_INTRO_AGAIN_WAV_PATH).await;
            self.reset_ivr_timeout();
        }
        _ => {
            // 無効入力
            info!("[session {}] invalid DTMF: '{}'", self.call_id, digit);
            self.play_audio(INVALID_WAV_PATH).await;
            self.play_audio(IVR_INTRO_AGAIN_WAV_PATH).await;
            self.reset_ivr_timeout();
        }
    }
}

(_, SessionIn::IvrTimeout) => {
    if self.ivr_state == IvrState::IvrMenuWaiting {
        info!("[session {}] IVR timeout, replaying menu", self.call_id);
        self.play_audio(IVR_INTRO_AGAIN_WAV_PATH).await;
        self.reset_ivr_timeout();
    }
}
```

### シーケンス

#### ケース 1: DTMF "1" でボイスボットモードへ

```
User                    Session                      App
  |                         |                          |
  |<-- intro_ivr.wav -------|                          |
  |    "1,2,9を押して..."   |                          |
  |                         |                          |
  |--- DTMF "1" ----------->|                          |
  |                         | ivr_state = VoicebotMode |
  |                         | capture.start()          |
  |                         |                          |
  |--- 音声 --------------->|                          |
  |                         |--- AudioBuffered ------->|
  |                         |                          |
  |                         |<-- LLM応答 --------------|
  |<-- 応答音声 ------------|                          |
```

#### ケース 2: DTMF "2" で仙台案内後、継続待機

```
User                    Session
  |                         |
  |<-- intro_ivr.wav -------|
  |                         |
  |--- DTMF "2" ----------->|
  |                         |
  |<-- sendai.wav ----------|
  |<-- intro_ivr_again.wav -|
  |                         | ivr_state = IvrMenuWaiting (維持)
  |                         |
  |--- DTMF "1" ----------->|
  |                         | → VoicebotMode へ
```

#### ケース 3: 無効入力後、再案内

```
User                    Session
  |                         |
  |<-- intro_ivr.wav -------|
  |                         |
  |--- DTMF "5" (invalid) ->|
  |                         |
  |<-- invalid.wav ---------|
  |<-- intro_ivr_again.wav -|
  |                         | ivr_state = IvrMenuWaiting (維持)
  |                         |
  |--- DTMF "1" ----------->|
  |                         | → VoicebotMode へ
```

#### ケース 4: タイムアウト時、再案内

```
User                    Session
  |                         |
  |<-- intro_ivr.wav -------|
  |                         |
  |    ... 10秒経過 ...     |
  |                         | IvrTimeout 発火
  |                         |
  |<-- intro_ivr_again.wav -|
  |                         | タイマーリセット
  |                         |
  |--- DTMF "1" ----------->|
  |                         | → VoicebotMode へ
```

### 検証方法

```bash
cargo test session::
# E2E: 各 DTMF 入力で正しい動作を確認
# - DTMF "1": ボイスボット会話開始
# - DTMF "2": 仙台案内 → 再案内 → DTMF 待機
# - DTMF "9": 再案内 → DTMF 待機
# - DTMF "5": 無効 → 再案内 → DTMF 待機
# - タイムアウト: 再案内 → DTMF 待機
```

---

## Step-24: BYE 即時応答・音声再生キャンセル (Issue #26)

**目的**: 音声再生中に BYE を受信した場合、即時に 200 OK を返し、再生をキャンセルしてセッションを正常終了する

**関連**: [Issue #26](https://github.com/MasanoriSuda/virtual_voicebot/issues/26)

### 背景

現在の実装では、音声再生（`play_audio` / `send_wav_as_rtp_pcmu`）がセッションループをブロックするため、再生中に BYE を受信しても処理されない。結果として:

1. 200 OK が返らない
2. UAC が BYE を再送し続ける
3. TransactionTimeout が発生する

```
[問題のシーケンス]

Session::run ──► play_audio().await ──► send_wav_as_rtp_pcmu ──► sleep(20ms) × N
     │                                       ↑
     │                                  ブロッキング（数秒〜数十秒）
     │
     └──► SessionIn::SipBye がキューに溜まる
          → 再生完了まで処理されない
          → 200 OK が遅延
          → BYE 再送・タイムアウト
```

### 技術アプローチ

#### 問題1: BYE 受信時の即時応答

音声再生をキャンセル可能にし、BYE 受信時に即座に中断して 200 OK を返す。
→ **実装済み**: `cancel_playback()` + `PlaybackState` によるステートベース再生

#### 問題2: 再生ティックが潰れる（プツプツ音声）

`tokio::select!` で毎ループ `sleep(20ms)` を作り直すと、RTP 受信が連続した場合に sleep が満了せず再生フレームが送出されない。

```
[問題]
tokio::select! {
    ev = rx.recv() => { ... }                           // RTP が来るたびにこちらが発火
    _ = sleep(20ms), if playback.is_some() => { ... }   // ← 毎回リセットされて満了しない
}
→ 再生フレームが間引かれて「プツプツ」
```

**修正**: `tokio::time::interval` をループ外で作成し、`biased;` で再生ティック優先。

```rust
[改善後]
let mut playback_tick = tokio::time::interval(Duration::from_millis(20));
playback_tick.set_missed_tick_behavior(MissedTickBehavior::Skip);

loop {
    tokio::select! {
        biased;  // 再生ティック優先
        _ = playback_tick.tick(), if self.playback.is_some() => {
            self.step_playback();
        }
        maybe_ev = rx.recv() => { ... }
    }
}
```

### DoD (Definition of Done)

#### 音声再生のキャンセル機構（実装済み）
- [x] `PlaybackState` による再生状態管理
- [x] `cancel_playback()` による即時キャンセル
- [x] BYE/AppHangup/SessionTimerFired/Abort 時にキャンセル呼び出し

#### 再生ティック安定化（未対応）
- [ ] `tokio::time::interval` をループ外で作成
- [ ] `biased;` で再生ティック優先
- [ ] `MissedTickBehavior::Skip` で遅延蓄積を防止

#### 検証
- [ ] E2E 検証（再生中に BYE を送信し、即座に 200 OK が返ることを確認）
- [ ] E2E 検証（音声が途切れずに再生されることを確認）
- [ ] TransactionTimeout が発生しないことを確認

### 対象パス

| ファイル | 変更内容 |
|---------|---------|
| `src/session/session.rs` | `interval` ベースの再生ティック、`biased` 優先 |

### 変更上限

- **行数**: <=50行
- **ファイル数**: <=1

### 実装案（interval ベース - 推奨）

```rust
// src/session/session.rs

use tokio::time::{interval, Duration, MissedTickBehavior};

async fn run(&mut self, mut rx: UnboundedReceiver<SessionIn>) {
    // ループ外で interval を作成（再生中のみ tick が有効）
    let mut playback_tick = interval(Duration::from_millis(20));
    playback_tick.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            biased;  // 再生ティック優先（RTP 受信に潰されない）

            // 再生中のみ tick を処理
            _ = playback_tick.tick(), if self.playback.is_some() => {
                self.step_playback();
            }

            // イベント処理
            maybe_ev = rx.recv() => {
                let Some(ev) = maybe_ev else { break; };
                // ... 既存のイベント処理 ...
            }
        }
    }
}
```

### ポイント

| 項目 | 説明 |
|------|------|
| `biased;` | select 内で上から順に評価し、再生ティックを優先 |
| `MissedTickBehavior::Skip` | 遅延した tick をスキップし、蓄積を防止 |
| `if self.playback.is_some()` | 再生中のみ tick を有効化 |

### シーケンス

#### 改善後: BYE 即時応答

```
UAC                     SIP Layer                  Session
  |                         |                          |
  |                         |                          | [再生中]
  |--- BYE ---------------->|                          |
  |                         |--- SessionIn::SipBye --->|
  |                         |                          | cancel_playback()
  |                         |                          | [再生中断]
  |                         |                          |
  |                         |<-- SipSendBye200 --------|
  |<-- 200 OK --------------|                          |
  |                         |                          | [セッション終了処理]
  |                         |                          | recorder.stop()
  |                         |                          | send_ingest()
  |                         |                          | CallEnded
```

### 検証方法

```bash
cargo test session::
# E2E: 再生中に BYE を送信
# 1. 通話開始 → IVR イントロ再生中に BYE 送信
# 2. 即座に 200 OK が返ることを確認
# 3. TransactionTimeout が発生しないことを確認
# 4. ログで "playback cancelled" が出力されることを確認
```

### リスク/ロールバック観点

| リスク | 対策 |
|--------|------|
| `biased;` による RTP 処理遅延 | 再生フレームは 20ms 毎に 1 フレームのみ、RTP 処理への影響は軽微 |
| interval の遅延蓄積 | `MissedTickBehavior::Skip` で古い tick を破棄 |
| 再生中断時の RTP タイムスタンプずれ | `align_rtp_clock()` で吸収（既存実装と同様） |

---

## Step-25: B2BUA コール転送 (Issue #27)

**目的**: DTMF "3" をトリガーとして、通話を Zoiper に転送し、Bot が B2BUA としてセッションを維持する

**関連**: [Issue #27](https://github.com/MasanoriSuda/virtual_voicebot/issues/27)

**依存**: Step-02（DTMF トーン検出 Goertzel）, Step-23（IVR メニュー機能）

### 背景

IVR メニューで DTMF "3" を押すと、通話を外部 SIP エンドポイント（Zoiper）に転送する。Bot は B2BUA（Back-to-Back User Agent）として機能し、両レグのメディアを中継する。

```
[現在: 1 レグ]
iPhone ←──── RTP ────→ Voicebot

[転送後: 2 レグ（B2BUA）]
iPhone ←──── RTP ────→ Voicebot ←──── RTP ────→ Zoiper
                          │
                          └── 両方向のメディアを中継
```

### 転送方式の選定

| 方式 | RFC | 特徴 | 採用 |
|------|-----|------|------|
| SIP REFER | 3515 | 盲目転送、一部 SIP サーバーで非対応 | ❌ |
| B2BUA | 3261 | Bot がセッション維持、メディアリレー | ✅ |

**採用理由**:
- SIP REFER は Odin など一部 SIP サーバーで未対応の可能性
- B2BUA は確実に動作し、将来の拡張（会話監視、録音、AI 介入）に対応可能

### 動作フロー

```
IVR メニュー待機中
    ↓
DTMF "3" 検出
    ↓
┌──────────────────────────────────────────────────────────────┐
│ 1. 転送案内音声再生（zundamon_transfer.wav）                 │
│ 2. Zoiper に UAC INVITE 送信                                 │
│ 3. 200 OK 受信 → ACK 送信                                    │
│ 4. B2BUA モードに移行                                        │
│    - iPhone → Voicebot → Zoiper (メディアリレー)            │
│    - Zoiper → Voicebot → iPhone (メディアリレー)            │
│ 5. いずれかが BYE → 両レグ終了                               │
└──────────────────────────────────────────────────────────────┘
```

### 状態遷移

```
                    ┌──────────────────────┐
                    │  IvrMenuWaiting      │
                    │  (DTMF待機中)        │
                    └──────────┬───────────┘
                               │
            ┌──────────────────┼──────────────────┬───────────────┐
            │                  │                  │               │
        DTMF=1             DTMF=2              DTMF=3          DTMF=9/other
            │                  │                  │               │
            ▼                  ▼                  ▼               │
┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐    │
│  VoicebotMode   │  │  InfoPlayback   │  │  Transferring   │    │
│  (既存会話機能) │  │  (仙台案内再生) │  │  (UAC INVITE中) │    │
└─────────────────┘  └────────┬────────┘  └────────┬────────┘    │
                              │                    │              │
                              ▼                    ▼              ▼
                    ┌─────────────────┐  ┌─────────────────┐  ┌───────────┐
                    │ IvrMenuWaiting  │  │  B2buaMode      │  │  Loop     │
                    │   (継続待機)    │  │  (メディア中継) │  │           │
                    └─────────────────┘  └─────────────────┘  └───────────┘
```

### DoD (Definition of Done)

#### 状態管理
- [ ] `IvrState` enum に `Transferring`, `B2buaMode` 追加
- [ ] B レグ用のセッション情報管理（Call-ID, tags, SDP）
- [ ] 転送失敗時のフォールバック処理

#### UAC INVITE 送信（B レグ）
- [ ] 環境変数 `TRANSFER_TARGET_SIP_URI` から転送先を取得
- [ ] INVITE リクエスト生成（SDP 含む）
- [ ] 100 Trying / 180 Ringing 処理
- [ ] 200 OK 受信 → ACK 送信
- [ ] エラー（4xx/5xx）時のハンドリング

#### メディアリレー
- [ ] A レグ (iPhone) からの RTP を B レグ (Zoiper) に転送
- [ ] B レグ (Zoiper) からの RTP を A レグ (iPhone) に転送
- [ ] RTP ヘッダの SSRC/タイムスタンプ変換（必要に応じて）
- [ ] B レグ用 RTP 送受信ソケット管理

#### 終了処理
- [ ] A レグ BYE 受信 → B レグに BYE 送信 → 両レグ終了
- [ ] B レグ BYE 受信 → A レグに BYE 送信 → 両レグ終了
- [ ] 転送中に A レグ BYE → 転送キャンセル

#### 音声ファイル
- [ ] `data/zundamon_transfer.wav`（転送案内）作成

#### 検証
- [ ] Unit test 追加（状態遷移テスト）
- [ ] E2E 検証（DTMF "3" で転送、双方向通話確認）

### 対象パス

| ファイル | 変更内容 |
|---------|---------|
| `src/session/session.rs` | B2BUA 状態管理、メディアリレー、終了処理 |
| `src/session/types.rs` | `IvrState` enum 拡張（`Transferring`, `B2buaMode`） |
| `src/session/b2bua.rs` | B レグ管理、UAC INVITE ロジック（新規） |
| `src/sip/builder.rs` | UAC INVITE ビルダー追加（必要に応じて） |
| `src/config.rs` | `TRANSFER_TARGET_SIP_URI` 設定追加 |
| `data/zundamon_transfer.wav` | 転送案内音声（新規） |

### 変更上限

- **行数**: <=500行
- **ファイル数**: <=6（コード変更）

### 環境変数（追加）

| 変数名 | 説明 | デフォルト |
|--------|------|-----------|
| `TRANSFER_TARGET_SIP_URI` | 転送先 SIP URI | `sip:zoiper@192.168.1.4:8000` |
| `TRANSFER_TIMEOUT_SEC` | UAC INVITE のタイムアウト秒数 | `30` |

### 型定義（案）

```rust
// src/session/types.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IvrState {
    #[default]
    IvrMenuWaiting,   // IVR メニュー待機中
    VoicebotMode,     // ボイスボットモード
    Transferring,     // 転送中（UAC INVITE 送信中）
    B2buaMode,        // B2BUA モード（メディア中継中）
}

// src/session/b2bua.rs
#[derive(Debug)]
pub struct BLeg {
    pub call_id: String,
    pub local_tag: String,
    pub remote_tag: Option<String>,
    pub remote_rtp_addr: SocketAddr,
    pub local_rtp_socket: UdpSocket,
    pub rtp_ssrc: u32,
    pub rtp_seq: u16,
    pub rtp_ts: u32,
}
```

### シーケンス

#### ケース 1: 正常転送

```
iPhone                  Voicebot                   Zoiper
  |                         |                          |
  |<-- intro_ivr.wav -------|                          |
  |                         |                          |
  |--- DTMF "3" ----------->|                          |
  |                         |                          |
  |<-- transfer.wav --------|                          |  転送案内
  |                         |                          |
  |                         |--- INVITE -------------->|  UAC INVITE (B レグ)
  |                         |                          |
  |                         |<-- 100 Trying -----------|
  |                         |<-- 180 Ringing ----------|
  |                         |<-- 200 OK ---------------|
  |                         |--- ACK ----------------->|
  |                         |                          |
  |                         | ivr_state = B2buaMode    |
  |                         |                          |
  |=== RTP (A→B) =========>|=== RTP (relay) =========>|  メディア中継
  |<== RTP (B→A) ==========|<== RTP (relay) ==========|
  |                         |                          |
  |--- BYE ---------------->|                          |  A レグ終了
  |<-- 200 OK --------------|                          |
  |                         |--- BYE ----------------->|  B レグも終了
  |                         |<-- 200 OK ---------------|
```

#### ケース 2: 転送失敗（タイムアウト）

```
iPhone                  Voicebot                   Zoiper
  |                         |                          |
  |--- DTMF "3" ----------->|                          |
  |                         |                          |
  |<-- transfer.wav --------|                          |
  |                         |                          |
  |                         |--- INVITE -------------->|
  |                         |                          |
  |                         |    ... タイムアウト ...  |
  |                         |                          |
  |<-- transfer_fail.wav ---|                          |  失敗案内
  |                         |                          |
  |                         | ivr_state = IvrMenuWaiting|  メニューに戻る
  |<-- intro_ivr_again.wav -|                          |
```

### 実装案

```rust
// src/session/session.rs

const TRANSFER_WAV_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/data/zundamon_transfer.wav");
const TRANSFER_FAIL_WAV_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/data/zundamon_transfer_fail.wav");

// DTMF "3" ハンドリング
(SessState::Established, SessionIn::Dtmf { digit: '3' }) => {
    if self.ivr_state != IvrState::IvrMenuWaiting {
        continue;
    }

    info!("[session {}] initiating transfer to Zoiper", self.call_id);
    self.ivr_state = IvrState::Transferring;

    // 転送案内音声を再生
    self.play_audio(TRANSFER_WAV_PATH).await;

    // B レグ作成（UAC INVITE）
    match self.initiate_b_leg().await {
        Ok(b_leg) => {
            info!("[session {}] B-leg established", self.call_id);
            self.b_leg = Some(b_leg);
            self.ivr_state = IvrState::B2buaMode;
        }
        Err(e) => {
            warn!("[session {}] transfer failed: {}", self.call_id, e);
            self.play_audio(TRANSFER_FAIL_WAV_PATH).await;
            self.play_audio(IVR_INTRO_AGAIN_WAV_PATH).await;
            self.ivr_state = IvrState::IvrMenuWaiting;
            self.reset_ivr_timeout();
        }
    }
}

// B2BUA モードでの RTP 中継
(SessState::Established, SessionIn::Rtp { packet }) => {
    if self.ivr_state == IvrState::B2buaMode {
        // A レグから受信した RTP を B レグに転送
        if let Some(ref b_leg) = self.b_leg {
            b_leg.forward_rtp(&packet).await;
        }
    }
    // ... 既存の RTP 処理 ...
}

// B レグからの RTP 受信（別タスクで監視）
(_, SessionIn::BLegRtp { packet }) => {
    // B レグから受信した RTP を A レグに転送
    self.send_rtp_to_a_leg(&packet).await;
}

// B レグ終了処理
(_, SessionIn::BLegBye) => {
    info!("[session {}] B-leg BYE received, ending both legs", self.call_id);
    // A レグに BYE 送信
    self.request_hangup();
}
```

### 検証方法

```bash
cargo test session::
# E2E: DTMF "3" で転送テスト
# 1. iPhone から通話開始 → IVR メニュー
# 2. DTMF "3" 入力
# 3. Zoiper で着信を確認
# 4. 双方向で音声通話ができることを確認
# 5. いずれかが BYE → 両方終了を確認
```

### Open Questions

| # | 質問 | 回答待ち |
|---|------|---------|
| Q1 | B レグ SDP のコーデック交渉は A レグと同じで良いか？（PCMU 固定） | Yes（PCMU 固定） |
| Q2 | RTP タイムスタンプ/SSRC は変換が必要か？ | 要検証（最小限の変換で開始） |
| Q3 | 転送中の保留音声は必要か？ | 暫定 No（将来検討） |

### リスク/ロールバック観点

| リスク | 対策 |
|--------|------|
| B レグ INVITE が Zoiper に到達しない | ネットワーク設定確認、タイムアウト後にフォールバック |
| RTP 中継による遅延増加 | 最小限の処理でスルーパット優先 |
| 片方終了時の残存セッション | 両レグ連動終了を徹底 |

---

## 凡例

| 状態 | 意味 |
|------|------|
| 未着手 | 作業開始前 |
| 進行中 | PR 作成中 |
| レビュー中 | PR レビュー待ち |
| 完了 | マージ済み |
| 実装済み | 既に実装されている |

---

## 変更履歴

| 日付 | バージョン | 変更内容 |
|------|-----------|---------|
| 2026-01-21 | 2.8 | Issue #27 統合: Step-25（B2BUA コール転送）追加、DTMF "3" でZoiper転送、メディアリレー方式 |
| 2026-01-20 | 2.7 | Step-24 更新: interval ベースの再生ティック安定化、プツプツ音声問題の修正方針追加 |
| 2026-01-20 | 2.6 | Issue #26 統合: Step-24（BYE 即時応答・音声再生キャンセル）追加、Step-23 状態を完了に更新 |
| 2026-01-19 | 2.5 | Issue #25 統合: Step-23（IVR メニュー機能）追加、Step-02 状態を完了に更新 |
| 2026-01-19 | 2.4 | Issue #24 統合: Step-02（DTMF トーン検出 Goertzel）に Issue リンク追加 |
| 2026-01-18 | 2.3 | Issue #23 統合: Step-22（ハルシネーション時謝罪音声）追加 |
| 2026-01-18 | 2.2 | Issue #22 統合: Step-21（時間帯別イントロ）追加 |
| 2026-01-18 | 2.1 | Issue #21 統合: Step-20（LLM 会話履歴ロール分離）追加 |
| 2026-01-18 | 2.0 | Issue #20 統合: Step-19（Session-Expires 対応）追加、DEF-10/DEF-11 を Step-19 に統合 |
| 2026-01-18 | 1.9 | Issue #19 統合: Step-18（ASR 低レイテンシ化）追加、VAD + 無音検出方式 |
| 2026-01-18 | 1.8 | Issue #18 統合: Step-09（486 Busy Here）詳細化、シーケンス図追加 |
| 2026-01-14 | 1.7 | Issue #13 統合: Step-14〜17（TLS/REGISTER/認証）追加、P0 最優先に昇格 |
| 2025-12-30 | 1.6 | Issue #8 統合: Code Quality Improvements セクション追加（CQ-01〜05）、ARCH-01 サブステップ化 |
| 2025-12-30 | 1.5 | Issue #8 統合: Architecture Improvements セクション追加（ARCH-01〜05） |
| 2025-12-29 | 1.4 | TODO.md 統合: P1/P2 追加項目、Deferred 詳細化（TODO.md 廃止） |
| 2025-12-28 | 1.3 | Issue #9 統合: Step-12 (Timer G/H/I/J), Step-13 (RTP extension/CSRC) 追加 |
| 2025-12-27 | 1.2 | UAS 優先に再構成、Deferred Steps 追加、Step 番号を依存順に並び替え |
| 2025-12-25 | 1.1 | RFC 2833 を P2 に変更、DTMF トーン検出 (Goertzel) を P0 で追加 |
| 2025-12-25 | 1.0 | 初版作成 |
