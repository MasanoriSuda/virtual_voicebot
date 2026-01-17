<!-- SOURCE_OF_TRUTH: 実装計画 -->
# Implementation Plan (PLAN.md)

- docs/** は変更しない（Stepで明示されている場合のみ例外）
- 依存追加は禁止（必要なら別途Spec/Plan）

| 項目 | 値 |
|------|-----|
| **Status** | Active |
| **Owner** | TBD |
| **Last Updated** | 2026-01-14 |
| **SoT (Source of Truth)** | Yes - 実装計画 |
| **上流ドキュメント** | [gap-analysis.md](../gap-analysis.md), [Issue #8](https://github.com/MasanoriSuda/virtual_voicebot/issues/8), [Issue #9](https://github.com/MasanoriSuda/virtual_voicebot/issues/9), [Issue #13](https://github.com/MasanoriSuda/virtual_voicebot/issues/13) |

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
| [Step-01](#step-01-cancel-受信処理) | CANCEL 受信処理 | - | 未着手 |
| [Step-02](#step-02-dtmf-トーン検出-goertzel) | DTMF トーン検出 (Goertzel) | - | 未着手 |
| [Step-03](#step-03-sipp-cancel-シナリオ) | SIPp CANCEL シナリオ | → Step-01 | 未着手 |
| [Step-04](#step-04-dtmf-トーン検出-e2e-検証) | DTMF トーン検出 E2E 検証 | → Step-02 | 未着手 |

### P1: 重要（RFC 準拠・相互接続性）

| Step | 概要 | 依存 | 状態 |
|------|------|------|------|
| [Step-05](#step-05-rseq-ランダム化) | RSeq ランダム化 | - | 完了 |
| [Step-06](#step-06-options-応答) | OPTIONS 応答 | - | 完了 |
| [Step-07](#step-07-artpmap-パース) | a=rtpmap パース | - | 未着手 |
| [Step-08](#step-08-rtcp-sdes-cname) | RTCP SDES (CNAME) | - | 未着手 |
| [Step-09](#step-09-486-busy-here) | 486 Busy Here | - | 未着手 |
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
| DEF-10 | re-INVITE 送信 | 4028 | refresher=uas 時のタイマー設計 |
| DEF-11 | UPDATE 送信 | 3311 | セッション更新トリガー設計 |
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

**目的**: 同時通話数制限時に 486 Busy Here を返す

**RFC参照**: RFC 3261 §21.4.7

### DoD (Definition of Done)

- [ ] 486 レスポンスビルダー追加
- [ ] SipCore に同時セッション制限オプション追加
- [ ] 制限超過時に 486 を返す
- [ ] Unit test 追加

### 対象パス

| ファイル | 変更内容 |
|---------|---------|
| `src/sip/builder.rs` | 486 ビルダー追加 |
| `src/sip/mod.rs` | 制限チェック追加 |
| `src/config.rs` | max_sessions 設定追加 |

### 変更上限

- **行数**: <=100行
- **ファイル数**: <=3

### 検証方法

```bash
cargo test sip::
# E2E: 複数 INVITE 同時送信で確認
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
| 2026-01-14 | 1.7 | Issue #13 統合: Step-14〜17（TLS/REGISTER/認証）追加、P0 最優先に昇格 |
| 2025-12-30 | 1.6 | Issue #8 統合: Code Quality Improvements セクション追加（CQ-01〜05）、ARCH-01 サブステップ化 |
| 2025-12-30 | 1.5 | Issue #8 統合: Architecture Improvements セクション追加（ARCH-01〜05） |
| 2025-12-29 | 1.4 | TODO.md 統合: P1/P2 追加項目、Deferred 詳細化（TODO.md 廃止） |
| 2025-12-28 | 1.3 | Issue #9 統合: Step-12 (Timer G/H/I/J), Step-13 (RTP extension/CSRC) 追加 |
| 2025-12-27 | 1.2 | UAS 優先に再構成、Deferred Steps 追加、Step 番号を依存順に並び替え |
| 2025-12-25 | 1.1 | RFC 2833 を P2 に変更、DTMF トーン検出 (Goertzel) を P0 で追加 |
| 2025-12-25 | 1.0 | 初版作成 |
