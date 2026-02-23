# STEER-226: Frontend/Backend 別マシン構成の環境変数対応

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-226 |
| タイトル | Frontend/Backend 別マシン構成の環境変数対応 |
| ステータス | Approved |
| 関連Issue | #226 |
| 優先度 | P1 |
| 作成日 | 2026-02-23 |

---

## 2. ストーリー（Why）

### 2.1 背景

現状は Frontend と Backend を同一マシン（Raspberry Pi）で動かすことを前提とした設定になっている。
具体的には：

| 問題 | 詳細 |
|------|------|
| `.env.example` の注記が誤り | 「VOICEVOX/AI エンドポイントは localhost 固定」と記載されているが、`BACKEND_URL` / `VOICEVOX_BASE_URL` は既にコードで環境変数化済み |
| `BACKEND_URL` が `.env.example` に未記載 | `docker-compose.yml` にのみ存在し、ローカル開発者が設定方法を知ることができない |
| `VOICEVOX_BASE_URL` が「将来予定」扱い | `.env.example` L70 に `# VOICEVOX_URL=...` としてコメントアウトされているが、フロントエンドは既に `VOICEVOX_BASE_URL` を参照している |
| 別マシン構成の設定例がない | Frontend をローカル PC で動かし Backend をラズパイで動かすユースケースのドキュメントが存在しない |

結果として、開発者が Frontend を別マシンで起動しようとしても必要な環境変数が分からず、
意図せずデフォルト（`localhost`）のままになってしまう。

### 2.2 目的

`.env.example` を実態に合わせて更新し、Frontend（ローカルサーバー等）と
Backend（Raspberry Pi 等）を別マシンで動かすための設定が自明に分かるようにする。

**コード変更は不要。** 接続先の環境変数化は既に完了している：
- Frontend → Backend: `BACKEND_URL`（`app/api/sync-status/route.ts` L9）
- Frontend → VoiceVox: `VOICEVOX_BASE_URL`（`app/api/announcements/tts/route.ts` L35-36）
- Backend CORS: `Access-Control-Allow-Origin: *` 実装済み（`src/interface/http/mod.rs` L221, L522）

### 2.3 ユーザーストーリー

```text
As a 開発者
I want to Frontend を自分の PC で起動し Backend はラズパイを使いたい
So that Frontend の UI 開発・デバッグを高速に行える

受入条件:
- [ ] .env.example に BACKEND_URL の設定例が記載されている
- [ ] .env.example に VOICEVOX_BASE_URL の設定例が記載されている（「将来予定」から「利用可能」に変更）
- [ ] 別マシン構成時に設定すべき変数が .env.example のコメントで分かる
- [ ] 誤った注記（「localhost 固定」）が削除または修正されている
- [ ] BACKEND_URL に Backend の IP を設定することで、Frontend から Backend へ疎通できる
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-23 |
| 起票理由 | Frontend をローカルサーバーで動かし Backend をラズパイで動かす構成を実現したい |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Sonnet 4.6 |
| 作成日 | 2026-02-23 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "フロントエンド：ローカルサーバーなど、バックエンド：ラズパイ、と環境変数でできるようにしたい。調査済み。ステアリング作成をお願いします" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| 1 | Codex | 2026-02-23 | NG | ①`.env.local` 設定先の未明記 ②docker-compose VOICEVOX 整合性 ③VOICEVOX_BASE_URL 呼び出し元の表現 |
| 2 | Codex | 2026-02-23 | NG | §5.2「docker-compose 内通信は設定不要」コメントが実装（VOICEVOX_BASE_URL 未設定）と矛盾 |
| 3 | Codex | 2026-02-23 | OK | 指摘なし。全指摘解消を確認 |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | @MasanoriSuda |
| 承認日 | 2026-02-23 |
| 承認コメント | lgtm |

### 3.5 実装

| 項目 | 値 |
|------|-----|
| 実装者 | Codex |
| 実装日 | 2026-02-23 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "STEER-226（承認済み）に基づき、root `.env.example` の誤注記修正、`BACKEND_URL` / `VOICEVOX_BASE_URL` の Frontend 接続先セクション追加、将来予定セクションの誤記整理を実施" |
| コードレビュー | - |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ実行者 | - |
| マージ日 | - |
| マージ先 | `.env.example`（ルート） |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| `.env.example`（ルート） | 修正 | 誤注記の修正・`BACKEND_URL` / `VOICEVOX_BASE_URL` の追加・別マシン構成例のコメント追加 |

### 4.2 影響するコード

**なし。** コード変更は不要。

> **参考（既に実装済みの箇所）:**
> - `virtual-voicebot-frontend/app/api/sync-status/route.ts` L9: `BACKEND_URL`
> - `virtual-voicebot-frontend/app/api/announcements/tts/route.ts` L35-36: `VOICEVOX_BASE_URL`
> - `virtual-voicebot-frontend/app/api/announcements/speakers/route.ts` L13-14: `VOICEVOX_BASE_URL`
> - `virtual-voicebot-backend/src/interface/http/mod.rs` L221, L522: `Access-Control-Allow-Origin: *`

---

## 5. 差分仕様（What / How）

### 5.1 `.env.example` の変更方針

以下の 3 点を変更する。コードは変更しない。

| 変更点 | 変更前 | 変更後 |
|--------|--------|--------|
| 誤注記の削除 | L7-8: 「AI エンドポイントは現在コード内でlocalhost 固定です。URL 変数はコードでは参照されません。」 | 削除または正確な内容に差し替え |
| `BACKEND_URL` の追記 | 記載なし | Frontend セクションとして追加 |
| `VOICEVOX_BASE_URL` の記載変更 | L70: `# VOICEVOX_URL=http://voicevox:50021`（将来予定・変数名も誤り） | 正しい変数名 `VOICEVOX_BASE_URL` で実装済みとして記載 |

### 5.2 変更後の `.env.example`（差分）

```diff
-# 注意: AI エンドポイント（Whisper/Ollama/VOICEVOX）は現在コード内で
-#       localhost 固定です。URL 変数はコードでは参照されません。
+# 注意: Backend の AI エンドポイント（Whisper/Ollama）はバックエンドで設定します。
+#       Frontend 接続先（BACKEND_URL / VOICEVOX_BASE_URL）は下記で設定できます。
```

```diff
+# =============================================================================
+# === Frontend — 接続先 ===
+# Frontend を Backend と別マシンで動かす場合に設定してください。
+# （同一マシンで pnpm dev 起動の場合は設定不要）
+# ※ docker-compose 起動時は別途 docker-compose.yml の environment で設定してください。
+# =============================================================================
+
+# Backend API の URL（Next.js API route から呼び出し）
+# 例: ラズパイ上の Backend に接続する場合
+# BACKEND_URL=http://192.168.1.5:18080
+BACKEND_URL=http://localhost:18080
+
+# VoiceVox TTS の URL（Next.js API Route（Frontend サーバ）から呼び出し）
+# 例: ラズパイ上の VoiceVox に接続する場合
+# VOICEVOX_BASE_URL=http://192.168.1.5:50021
+VOICEVOX_BASE_URL=http://localhost:50021
```

```diff
-# TODO(Issue起票): AI エンドポイントを環境変数化する際に有効化
-# LLM_PROVIDER=ollama
-# TTS_PROVIDER=voicevox
-# ASR_PROVIDER=whisper
-# OLLAMA_URL=http://ollama:11434
-# VOICEVOX_URL=http://voicevox:50021
+# VOICEVOX_BASE_URL と BACKEND_URL は上記 Frontend セクションを参照してください
```

### 5.3 別マシン構成時の設定手順（コメントに追記）

Frontend（ローカル PC）と Backend（ラズパイ）を分離して動かす場合：

```bash
# Backend マシン（ラズパイ）側: 既存設定で問題なし
# ADVERTISED_IP=<ラズパイのIPアドレス>   ← SIP 用に要設定（既存設定）
# RECORDING_HTTP_ADDR=0.0.0.0:18080     ← 全 NIC でリッスン（既存設定）

# Frontend マシン（ローカル PC）側
# → virtual-voicebot-frontend/.env.local（または起動シェルの env）へ設定する
#   （`pnpm dev` は virtual-voicebot-frontend ディレクトリで実行するため、
#    ルートの .env.example ではなく Frontend サブディレクトリの .env.local を参照する）
BACKEND_URL=http://<ラズパイのIPアドレス>:18080
VOICEVOX_BASE_URL=http://<VoiceVoxが動いているIPアドレス>:50021
```

**データフロー（分離後）:**
```
ブラウザ（PC）
  ↓ fetch /api/* (同一ホスト)
Next.js API Route（PC:3000）
  ↓ BACKEND_URL
Backend（ラズパイ:18080） ← CORS 対応済み
```

ブラウザから Backend へ直接 fetch はなく Next.js 経由のため、CORS 問題は発生しない。

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #226 | STEER-226 | 起票 |
| STEER-226 | `.env.example` | ドキュメント修正 |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [ ] `.env.example` の変更差分が正確か（変数名が実装と一致しているか）
- [ ] `BACKEND_URL` のデフォルト値（`http://localhost:18080`）が既存の docker-compose 設定と整合しているか
- [ ] `VOICEVOX_BASE_URL` のデフォルト値（`http://localhost:50021`）が既存の設定と整合しているか
- [ ] コード変更が不要であることに合意しているか
- [ ] 将来予定セクションの整理（`VOICEVOX_URL` → `VOICEVOX_BASE_URL` への修正）が正しいか

### 7.2 マージ前チェック（Approved → Merged）

- [ ] `.env.example` が更新されている
- [ ] ローカル PC 起動時（`pnpm dev`）に `virtual-voicebot-frontend/.env.local` へ `BACKEND_URL` を別ホストに設定して Frontend から Backend へ疎通できることを確認している
- [ ] ローカル PC 起動時（`pnpm dev`）に `virtual-voicebot-frontend/.env.local` のデフォルト値（`localhost`）のままで同一マシン構成が動作することを確認している

> **Note:** docker-compose 内の Frontend コンテナへの `VOICEVOX_BASE_URL` 設定（`http://voicevox:50021`）は現在 `docker-compose.yml` に未設定であり、コンテナ内 localhost では VoiceVox に到達できない。この問題は本 Issue のスコープ外とし、別 Issue で対処する。

---

## 8. 未確定点・質問

| # | 質問 | 選択肢 | 推奨 | オーナー回答 |
|---|------|--------|------|-------------|
| Q1 | `BACKEND_URL` のデフォルト値を `http://localhost:18080` にするか、空文字（未設定）にするか | localhost デフォルト / 空 | **localhost デフォルト（既存の `route.ts` L9 の `\|\| "http://localhost:18080"` と一致）** | - |
| Q2 | `VOICEVOX_BASE_URL` の変数名を `.env.example` の `VOICEVOX_URL` から変更することで既存環境に影響はないか | 影響なし / 影響あり | **影響なし（コードは `VOICEVOX_BASE_URL` を参照。`VOICEVOX_URL` はコードで未参照の誤記）** | - |

---

## 9. リスク・ロールバック観点

| リスク | 影響 | 緩和策 |
|--------|------|--------|
| `.env.example` 変更による既存環境への影響 | `.env.example` はサンプルファイルのため `.env.local` に影響しない | - |
| デフォルト値の変更 | 今回はデフォルト値を変更しない（`localhost` を維持） | コードのデフォルトと一致させることで互換維持 |

**ロールバック手順:** `.env.example` の変更を `git revert`。コード変更がないためロールバックリスクは低い。

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-23 | 初版作成（調査結果を元に差分仕様を記述） | Claude Sonnet 4.6 |
| 2026-02-23 | Codex レビュー Round 1 NG 対応（§3.3 更新、§5.2 VOICEVOX コメント修正、§5.3 `.env.local` 設定先明記、§7.2 docker-compose 問題を Note 化） | Claude Sonnet 4.6 |
| 2026-02-23 | Codex レビュー Round 2 NG 対応（§5.2「docker-compose 内通信は設定不要」コメントを pnpm dev 限定に修正、docker-compose は別途 environment 設定が必要と追記） | Claude Sonnet 4.6 |
| 2026-02-23 | Codex レビュー Round 3 OK（§3.3 更新） | Claude Sonnet 4.6 |
