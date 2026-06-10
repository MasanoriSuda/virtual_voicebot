# STEER-282: Flutter によるフロントエンド相当の制御実装

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-282 |
| タイトル | Flutter によるフロントエンド相当の制御実装 |
| ステータス | Draft |
| 関連Issue | #282 |
| 優先度 | P2（未確定：オーナー判断待ち） |
| 作成日 | 2026-04-18 |

---

## 2. ストーリー（Why）

### 2.1 背景

<!--
  Issue #282 本文が空のため、タイトルと現行リポジトリ状況から推定した内容を記載する。
  Open Questions（§8）の解消を経てストーリーを確定させる必要がある。
-->

- 現行のフロントエンドは [virtual-voicebot-frontend/](../../../virtual-voicebot-frontend/) に Next.js (TypeScript / React) で実装されている（参考: [STEER-099_frontend-mvp.md](STEER-099_frontend-mvp.md), [STEER-119_ui-backend-integration.md](STEER-119_ui-backend-integration.md)）。
- **重要な前提崩れ（レビュー指摘対応）**: 現行の制御系データは Backend ではなく **Frontend 配下のローカル JSON ストア**が SoT になっている。
  - [call-actions API](../../../virtual-voicebot-frontend/app/api/call-actions/route.ts) → [`storage/db/call-actions.json`](../../../virtual-voicebot-frontend/lib/db/call-actions.ts)
  - [ivr-flows API](../../../virtual-voicebot-frontend/app/api/ivr-flows/route.ts) → [`storage/db/ivr-flows.json`](../../../virtual-voicebot-frontend/lib/db/ivr-flows.ts)
  - [announcements API](../../../virtual-voicebot-frontend/app/api/announcements/route.ts) → [`storage/db/announcements.json` + `public/audio/`](../../../virtual-voicebot-frontend/lib/db/announcements.ts)
- また、上記 API には **認証・権限制御が一切実装されていない**（call-actions PUT / ivr-flows PUT / announcements POST / calls GET すべてノーガード）。したがって「Next.js 版と同等」を認証・権限の比較基準にはできない。
- Web ブラウザ UI のみでは、モバイル端末（iOS / Android）やデスクトップアプリからの運用シナリオに追従しにくいという想定課題がある。
- Flutter で同等の制御 UI を実装できれば、単一コードベースで複数プラットフォーム（Mobile/Desktop/Web）への配布が可能となり、現場運用者・管理者の利用端末の幅を広げられる。

> ⚠️ 上記のうち「Flutter 化の動機」は Issue 本文空のための推定。起票意図（目的・対象ユーザー・想定プラットフォーム）は §8 の Open Questions で確認する。

### 2.2 目的

本件を安全に進めるには、**Flutter 実装に先立って以下 2 点の前提決定（PRECOND）が必要**。これを確定しないと parity 実装は不能。

- **PRECOND-A（SoT 境界決定）**: 制御系データ（call-actions / ivr-flows / announcements / number-groups / scenarios）の SoT を、現行の Frontend ローカル JSON から移動するか／維持するか。選択肢：
  - A1: Backend を SoT に寄せる（Frontend API Route を Backend API へ置き換え、複数クライアントが同一 Backend を参照）
  - A2: Next.js API Route を外出し（独立サーバ化、Frontend/Flutter 共通利用）
  - A3: SoT 移動せず。Flutter も Next.js の API Route を叩く（Next.js サーバ常駐前提）
- **PRECOND-B（認証・権限設計）**: 現行 API は無認証。Flutter 導入に合わせて以下を先に RD/BD に落とす。
  - 認証方式（Cookie セッション / JWT / OAuth / 社内 SSO 等）
  - 権限境界（閲覧／編集／管理者 の分離）
  - 攻撃面（CORS、CSRF、トークン寿命、モバイル保管方式）
  - 既存 Next.js API への guard 追加（後方互換の扱いを含む）

この 2 点が確定した後に、Flutter ベースで Next.js フロントエンドと **機能的に等価（parity）** な制御 UI を提供する。parity の対象機能は §2.4 で定義する。

### 2.3 ユーザーストーリー

```
As a 運用管理者（モバイル端末を主に利用する現場担当者）
I want to Flutter アプリ（モバイル/デスクトップ）からフロントエンドと同等の制御操作を行えるようにしたい
So that ブラウザを開かずに、手持ちの端末から着信アクション・IVR・アナウンス等の設定が確認・変更できる
```

受入条件（PRECOND-A / PRECOND-B 確定後に最終化。以下は骨子案）:

- [ ] AC-1（着信アクション閲覧）: Flutter から [call-actions content](../../../virtual-voicebot-frontend/components/call-actions-content.tsx) と同等の画面で、ルール・匿名着信動作・デフォルト動作を閲覧できる
- [ ] AC-2（着信アクション編集）: ルールの追加・編集・削除・並び替え、VR/IV/VM/BZ/NR/AN 設定、ルール適用先番号グループ選択ができる
- [ ] AC-3（IVR 閲覧）: Flutter から [IVR content](../../../virtual-voicebot-frontend/components/ivr-content.tsx) と同等にルートメニュー・配下のサブメニュー（少なくとも 3 層ネスト）を閲覧できる
- [ ] AC-4（IVR 編集）: メニュー作成・削除・アナウンス紐付け・ DTMF キーへの分岐設定（アナウンス／サブメニュー／転送／留守電）の作成・変更ができる
- [ ] AC-5（アナウンス閲覧・再生）: アナウンス一覧の閲覧と音声再生ができる
- [ ] AC-6（アナウンス WAV アップロード）: WAV ファイルのアップロードができる（[announcements-content](../../../virtual-voicebot-frontend/components/announcements-content.tsx) と同等）
- [ ] AC-7（アナウンス TTS 生成）: VoiceVox（または同等 TTS）による音声生成ができる
- [ ] AC-8（アナウンス管理）: 削除・有効化切替・名称変更ができる
- [ ] AC-9（番号グループ管理）: 番号グループの作成・削除・番号の追加・削除ができる（call-actions のルール適用先として必要）
- [ ] AC-10（発着信履歴）: [STEER-270](STEER-270_call-history-status-label.md) の表示仕様（ステータス 6 分類・通話時間 0 秒表示）に準拠して一覧表示できる
- [ ] AC-11（SoT 整合性）: PRECOND-A の決定に従い、Next.js 版 UI（動作する場合）と Flutter 版 UI のいずれで編集しても同一の SoT に反映される。具体条件は §5.1 RD-282-FR-11 参照
- [ ] AC-12（認証）: PRECOND-B の RD/BD で定義した認証方式でログインでき、未認証状態では制御系 API にアクセスできない
- [ ] AC-13（権限）: PRECOND-B の RD/BD で定義した権限境界に従い、閲覧のみロール・編集ロール・管理者ロールの操作可否が守られる

### 2.4 スコープ（parity の対象／対象外）

`parity` の誤解を避けるため、対象と対象外を明示する。

**対象機能（Flutter 版で実装する）**:

| # | 機能 | 根拠 |
|---|------|------|
| S-1 | 着信アクション決定 UI | [STEER-132](STEER-132_call-action-ui.md) |
| S-2 | IVR フロー管理 UI（3 層ネスト対応） | [STEER-134](STEER-134_ivr-flow-ui.md), [STEER-158](STEER-158_ivr-tree-ui.md) |
| S-3 | アナウンス音声管理 UI（WAV アップロード + TTS 生成） | [STEER-129](STEER-129_announce-audio-add.md) |
| S-4 | 番号グループ管理 UI | [STEER-132](STEER-132_call-action-ui.md)（ルール適用先） |
| S-5 | 発着信履歴一覧 | [STEER-270](STEER-270_call-history-status-label.md) |
| S-6 | 認証・権限管理（PRECOND-B 準拠） | 本ステアリング §2.2 |

**対象外（今回は実装しない）**:

| # | 機能 | 理由 |
|---|------|------|
| O-1 | Backend 側（Rust）への SoT 移行そのもの（PRECOND-A が A1/A2 の場合） | 別ステアリング・別マイルストーンで扱う |
| O-2 | Next.js フロントエンドの廃止 | 本件は並行運用前提（Q1 の確認が必要） |
| O-3 | 発着信履歴 CSV 出力（Flutter 版） | 優先度低。必要になれば追加ステアリング |
| O-4 | 通話詳細 Drawer / 録音再生 | 同上（初期リリースには不要と仮置き） |
| O-5 | KPI ダッシュボード | 同上 |
| O-6 | 管理画面のユーザ・権限メンテ UI 自体 | PRECOND-B の RD/BD 側で扱う |

> ⚠️ 対象外の項目は Q3（MVP 範囲）の最終確定次第で見直し。

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-04-18 |
| 起票理由 | Flutter で現行フロントエンド相当の制御 UI を提供可能にするため |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Code (claude-opus-4-7) |
| 作成日 | 2026-04-18 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "Issue #282 の Flutter フロントエンド相当制御に関するステアリングを作成" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| 1 | @MasanoriSuda | - | 未実施 | Issue #282 本文が空のため、§8 Open Questions の確認から着手 |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | - |
| 承認日 | - |
| 承認コメント | - |

### 3.5 実装（該当する場合）

| 項目 | 値 |
|------|-----|
| 実装者 | Codex（Approved 後） |
| 実装日 | - |
| 指示者 | - |
| 指示内容 | - |
| コードレビュー | CodeRabbit（自動） |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ実行者 | - |
| マージ日 | - |
| マージ先 | RD-282（新規）, BD-282（新規）, DD-282（新規）, UT-282（新規）※ID は承認後確定 |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| virtual-voicebot-frontend/docs/requirements/RD-282-flutter-parity.md（仮） | 追加 | Flutter 版フロントエンドの機能要件 |
| virtual-voicebot-frontend/docs/design/basic/BD-282-flutter-architecture.md（仮） | 追加 | Flutter アプリのアーキテクチャ・状態管理・通信設計・API 契約境界 |
| virtual-voicebot-frontend/docs/design/basic/BD-282-auth-boundary.md（仮） | 追加 | PRECOND-B: 認証方式・権限境界・CORS/CSRF の設計 |
| virtual-voicebot-frontend/docs/design/detail/DD-282-flutter-screens.md（仮） | 追加 | 画面構成・Widget 設計・ルーティング |
| virtual-voicebot-frontend/docs/test/unit/UT-282-flutter.md（仮） | 追加 | Flutter UT（`flutter test`、widget test）方針 |
| virtual-voicebot-frontend/docs/test/integration/IT-282-flutter.md（仮） | 追加 | Flutter IT（`flutter test integration_test/`）方針 — 画面遷移・API Route/Backend API 疎通・SoT 整合性 |
| virtual-voicebot-frontend/docs/test/system/ST-282-flutter.md（仮） | 追加 | Flutter ST 方針（実機／エミュレータ、配信導線） |
| docs/contract.md | 修正 | クライアント種別として Flutter を追記（API 契約は PRECOND-A / PRECOND-B 決定後に再定義） |
| virtual-voicebot-frontend/docs/process/v-model.md | 修正 | 対象スタック（Next.js / Flutter）を併記、Flutter の成果物対応を追記 |
| virtual-voicebot-frontend/docs/process/quality-gate.md | 修正 | QG-4（DD→UT）に `flutter test`、QG-5（UT→IT）に widget test、QG-6（IT→ST）に integration test、QG-7 に Flutter ST を追加 |

> ⚠️ 配置先（frontend 配下に置くか、新規 `virtual-voicebot-flutter/` を作るか）は §8 Open Questions で確定する。

### 4.2 影響するコード

| モジュール | 変更種別 | 概要 |
|-----------|---------|------|
| virtual-voicebot-flutter/（新規 or virtual-voicebot-frontend/flutter/） | 追加 | Flutter プロジェクトを新設 |
| virtual-voicebot-frontend/app/api/**/route.ts | 修正 | PRECOND-B の認証 guard 追加（PRECOND-A が A3 の場合は必須） |
| virtual-voicebot-frontend/lib/db/*.ts | 修正（可能性） | PRECOND-A が A2 の場合、JSON ストアアクセスを外出し |
| virtual-voicebot-backend（API 側） | 修正（可能性） | PRECOND-A が A1 の場合、制御系エンドポイントを新規実装。CORS・認証もここで実装 |
| CI（GitHub Actions） | 追加 | Flutter `analyze` / `test` / `integration_test` / ビルド（iOS/Android/Web/Desktop のうち対象分）ジョブ追加 |

---

## 5. 差分仕様（What / How）

> ⚠️ Issue 本文が空のため、本セクションは Open Questions 解消後に詳細化する。以下は骨子案。

### 5.1 要件追加（RD-282 へマージ）

```markdown
## RD-282-FR-01: Flutter クライアントからの着信アクション設定管理

### 概要
Flutter アプリから、Next.js 版と同等の操作で着信アクション（VR/IV/VM/BZ/NR/AN）の設定を閲覧・変更できる。

### 入力
- 番号グループID、アクション種別、パラメータ（[STEER-132](STEER-132_call-action-ui.md) 参照）

### 出力
- 設定結果（成功/失敗、更新後の設定一覧）

### 受入条件
- [ ] 既存の Backend API（番号グループ・ルール関連）を利用して設定変更できる
- [ ] 変更結果が Next.js 版にも即座に反映される（共通 DB を参照）

### トレース
- → BD: BD-282-xx
- → ST: ST-282-TC-xx
```

```markdown
## RD-282-FR-02: Flutter クライアントからの IVR フロー管理

### 概要
Flutter アプリから IVR フロー（DTMF メニュー、アナウンス、ルーティング、3層ネスト）を閲覧・編集できる。

### 受入条件
- [ ] [STEER-134](STEER-134_ivr-flow-ui.md) の受入条件と同等の操作が Flutter 側でも可能
```

```markdown
## RD-282-FR-11: SoT 境界定義（PRECOND-A）

### 概要
制御系データ（call-actions / ivr-flows / announcements / number-groups / scenarios）の SoT を定義し、Next.js と Flutter の両クライアントが同一 SoT を参照することを保証する。

### 受入条件
- [ ] Q1〜Q4 の確定後、採用する選択肢（A1/A2/A3）が RD に明記されている
- [ ] Next.js 版（動作する場合）と Flutter 版で同一 SoT を参照し、いずれで編集しても他方の表示で整合する
- [ ] SoT の永続化先（JSON ファイル／Backend DB／外部サービス）・書込み競合の扱いが定義されている

### トレース
- → BD: BD-282-flutter-architecture（API 契約境界）
- → IT: IT-282-TC-SoT-*
```

```markdown
## RD-282-FR-12: 認証・権限（PRECOND-B）

### 概要
現状無認証の制御系 API に対し、認証方式と権限境界を定義する。Flutter / Next.js の両クライアントに等しく適用される。

### 受入条件
- [ ] 認証方式（Q6）が RD に決定されている（Cookie セッション / JWT / OAuth / 社内 SSO 等）
- [ ] 権限ロールが定義されている（例: viewer / editor / admin）
- [ ] 制御系 API（call-actions / ivr-flows / announcements / number-groups / calls）に認証 guard が実装されている
- [ ] 未認証リクエストは 401、権限不足リクエストは 403 を返す
- [ ] Flutter 側でトークン保管方式（secure storage 等）が定義されている
- [ ] CORS・CSRF 対策が定義されている（Flutter Web ビルド含む）

### トレース
- → BD: BD-282-auth-boundary
- → IT: IT-282-TC-Auth-*
- → ST: ST-282-TC-Auth-*
```

（FR-03 アナウンス管理 / FR-04 発着信履歴 / FR-05 番号グループ管理 / FR-06 マルチプラットフォームビルド 等は Open Questions 確定後に追記）

---

### 5.2 詳細設計追加（DD-282 へマージ）

```markdown
## DD-282-FN-01: Flutter アプリ アーキテクチャ（骨子）

### 構成案
- 状態管理: Riverpod / Bloc（選定は Open Questions）
- HTTP クライアント: dio / http
- ルーティング: go_router
- 認証: Next.js と同方式（セッション or JWT、Open Questions）
- ビルドターゲット: iOS / Android / Web / Desktop（優先順位は Open Questions）

### トレース
- ← BD: BD-282-xx
- → UT: UT-282-TC-xx
```

---

### 5.3 テストケース追加（UT-282 / IT-282 / ST-282 へマージ）

```markdown
## UT-282-TC-01: 着信アクション一覧 Widget

### 対象
Flutter の着信アクション一覧 Widget（状態管理含む）

### 目的
モック API レスポンスから Widget が正しく描画されることを確認する。

### 入力
モック API レスポンス（番号グループ×アクション）

### 期待結果
一覧 Widget に全件が描画される

### トレース
← DD: DD-282-FN-xx
```

```markdown
## IT-282-TC-SoT-01: Next.js 編集 → Flutter 反映（PRECOND-A 準拠）

### 対象
採用した SoT 経路全体（Flutter ↔ API ↔ SoT ↔ Next.js）

### 目的
PRECOND-A で定義した SoT 経由で、Next.js 側の編集が Flutter 側で整合することを結合観点で確認する。

### 入力
1. Next.js UI でルール追加
2. Flutter アプリで再読込

### 期待結果
Flutter 側に追加ルールが表示される

### トレース
← BD: BD-282-flutter-architecture
← RD: RD-282-FR-11
```

```markdown
## IT-282-TC-Auth-01: 未認証アクセス拒否（PRECOND-B 準拠）

### 対象
制御系 API の認証 guard

### 目的
未認証の Flutter リクエストが 401 で拒否されることを確認する。

### 入力
トークンなしで `GET /api/call-actions`

### 期待結果
HTTP 401 / Flutter アプリはログイン画面へ誘導

### トレース
← BD: BD-282-auth-boundary
← RD: RD-282-FR-12
```

```markdown
## IT-282-TC-API-01: API Route 疎通（`flutter test integration_test/`）

### 対象
Flutter 実機（または emulator）から API Route（または Backend API）への疎通

### 目的
画面遷移〜API 呼び出し〜描画の一連を integration_test で検証する。

### 入力
アプリ起動 → ログイン → 着信アクション画面 → 1 件編集 → 保存

### 期待結果
保存完了表示、再読込後も変更が保持されている

### トレース
← BD: BD-282-flutter-architecture
```

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #282 | STEER-282 | 起票 |
| STEER-282 | RD-282-FR-01〜FR-12 | 要件追加 |
| RD-282-FR-xx | BD-282-flutter-architecture / BD-282-auth-boundary | 基本設計 |
| BD-282-xx | DD-282-FN-xx | 詳細設計 |
| BD-282-xx | IT-282-TC-xx | 結合テスト |
| DD-282-FN-xx | UT-282-TC-xx | 単体テスト |
| RD-282-FR-xx | ST-282-TC-xx | システムテスト |
| RD-282-FR-11 | IT-282-TC-SoT-xx | SoT 整合性 IT |
| RD-282-FR-12 | IT-282-TC-Auth-xx / ST-282-TC-Auth-xx | 認証境界 IT/ST |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [ ] §8 Open Questions がすべて解消されている
- [ ] PRECOND-A（SoT 境界）が A1/A2/A3 のどれかに確定し、RD に記載されている
- [ ] PRECOND-B（認証・権限）の方式・権限境界・CORS/CSRF が BD に記載されている
- [ ] 現行 Next.js 版との機能 parity 範囲（§2.4 の対象／対象外）が合意されている
- [ ] Flutter プロジェクトの配置先（frontend 配下 or 新規サブプロジェクト）が合意されている
- [ ] ビルドターゲット優先順位（iOS/Android/Web/Desktop）が決まっている
- [ ] 既存 API（call-actions / ivr-flows / announcements / number-groups / calls）で guard 追加が必要な箇所が洗い出されている
- [ ] Flutter UT / IT / ST の具体的なテスト手段（`flutter test`, widget test, `integration_test/`, 実機／emulator）が明記されている
- [ ] CI の追加ジョブ（analyze / test / integration_test / build）が定義されている
- [ ] 品質ゲート（QG-4〜QG-7）に Flutter の基準が追加されている
- [ ] 既存仕様との整合性がある
- [ ] トレーサビリティ（RD→BD→DD→UT / BD→IT / RD→ST）が維持されている

### 7.2 マージ前チェック（Approved → Merged）

- [ ] 実装が完了している
- [ ] コードレビュー（CodeRabbit）を受けている
- [ ] 関連 UT / IT / ST が PASS
- [ ] RD-282 / BD-282 / DD-282 / UT-282 / IT-282 / ST-282 への反映準備ができている

---

## 8. 備考（Open Questions）

Issue #282 は本文が空のため、以下を起票者（@MasanoriSuda）に確認する必要がある。

| # | 質問 | 選択肢（例） | 確定後の影響 |
|---|------|-----------|-------------|
| Q1 | 目的は「Next.js 置き換え」か「並行運用（モバイル/デスクトップ追加）」か？ | A) 置き換え / B) 並行運用 / C) 未定 | §2.2 目的、§4 影響範囲 |
| Q2 | 対象プラットフォームと優先度は？ | iOS / Android / Web / macOS / Windows / Linux | §5.2 ビルド設定 |
| Q3 | Flutter 版で実装する機能スコープは？（§2.4 の対象／対象外の確定） | A) §2.4 案のまま / B) S-4 番号グループ外し等 / C) S-3 を参照のみ | §2.3 AC、§2.4、§5.1 FR |
| Q4 | Flutter プロジェクトの配置先は？ | A) `virtual-voicebot-frontend/flutter/` / B) 新規 `virtual-voicebot-flutter/` / C) 別リポジトリ | §4, ドキュメント体系 |
| **Q5** | **PRECOND-A: 制御系データ（call-actions / ivr-flows / announcements / number-groups / scenarios）の SoT をどこに置くか？** | **A1) Backend (Rust) に移行 / A2) Next.js API Route を独立サーバ化 / A3) 現状維持（Next.js API Route を Flutter も叩く）** | §2.2, §4, §5.1 RD-282-FR-11 |
| **Q6** | **PRECOND-B: 認証方式は？ 現行 API は無認証のため新規策定が必要** | **Cookie セッション / JWT / OAuth / 社内 SSO / 他** | §5.1 RD-282-FR-12, §5.2 BD-282-auth-boundary |
| Q6-a | 権限ロール設計は？ | viewer / editor / admin の 3 段 / 2 段 / 単一 | RD-282-FR-12 |
| Q6-b | Next.js 既存 API に guard を追加するタイミングは？ | Flutter 導入と同時 / 先行して別 PR | Backend・Frontend 工程 |
| Q7 | 状態管理ライブラリの指定はあるか？ | Riverpod / Bloc / Provider / 任意 | DD-282 |
| Q8 | 配信方法は？ | 社内配布（Firebase App Distribution 等）/ App Store / Play Store / 未定 | 運用・CI |
| Q9 | デザインシステムの扱いは？ | Next.js と UI 共通化 / Flutter 独自 / Material3 準拠 | DD-282 |
| Q10 | 優先度（P0/P1/P2）は？ | 現状 P2 仮置き | §1 メタ情報 |
| Q11 | 既存 Frontend 向け storage/db/*.json の移行方針は？ | 現状維持（Q5=A3）/ Backend DB へ移行（Q5=A1）/ 独立サーバに外出し（Q5=A2）に従い migration 手段を定義 | Q5 の下位決定 |
| Q12 | アナウンス音声ファイル（`public/audio/`）の配信方法は？ | Next.js の `/audio/*` を Flutter から直接取得 / Backend 経由 / 専用 CDN | §5.1 FR-03 |

### その他の注意事項

- 本ステアリングは Draft のため、§5 差分仕様は骨子のみ。Open Questions 解消後に詳細化する。
- 本体仕様書（RD/BD/DD/UT）の新規 ID は Approved 後に採番する。
- Flutter 導入に伴い、Frontend のプロセス定義（V字モデル）・品質ゲート（ESLint 等は Next.js 向け）を Flutter 向けに拡張または分離する必要があるか要検討（関連: [virtual-voicebot-frontend/docs/process/v-model.md](../process/v-model.md), [virtual-voicebot-frontend/docs/process/quality-gate.md](../process/quality-gate.md)）。

---

## リスク / ロールバック観点

- **リスク1（SoT 不定のまま着手）**: PRECOND-A が未確定のまま Flutter を先行実装すると、SoT 切替時に Flutter 側の API 呼び出し経路を全面的に作り直す可能性がある。Q5 の確定を Approved の前提条件とする。
- **リスク2（認証の後付け破綻）**: PRECOND-B が未確定のまま API の guard を後付けすると、Flutter / Next.js / CI / 既存運用に同時に破壊的変更が走る。Q6 / Q6-a / Q6-b を BD に落とすまで実装着手しない。
- **リスク3（スコープ肥大化）**: Flutter 版で Next.js 版と完全な parity を目指すと実装コストが大きい。§2.4 の対象／対象外を Q3 で確定する。
- **リスク4（API の二重保守）**: Flutter 追加で API の変更コストが上がる。Q5 = A1/A2 を採用する場合は、Next.js 側の API Route を薄く保つか削除する方針も同時に決める。
- **リスク5（ドキュメント体系の分散）**: Frontend ドキュメント体系が Next.js 前提のため、Flutter 用ドキュメントをどこに置くか（Q4）で整合性が崩れる恐れ。
- **ロールバック**: Flutter プロジェクトは独立ディレクトリに置く前提のため、本件の撤退は該当ディレクトリの削除と CI ジョブの無効化で可能。ただし PRECOND-B で Next.js API に guard を追加した場合は、その guard の扱い（維持／revert）を別途判断する必要がある。

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-04-18 | 初版作成（Draft、Issue #282 本文空のため Open Questions 主体） | Claude Code (claude-opus-4-7) |
| 2026-04-18 | レビュー指摘対応: SoT 前提崩れ（重大）／認証ゼロ前提（重大）／IT 成果物欠落（中）／parity スコープ不明（中）に対応。PRECOND-A / PRECOND-B を §2.2 に追加、AC を 6 → 13 件に再構成、§2.4 スコープ（対象／対象外）新設、§4/§5/§6 に IT 追記、§8 Open Questions に Q5（SoT）/ Q6（認証）/ Q11 / Q12 追加 | Claude Code (claude-opus-4-7) |
