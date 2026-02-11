# STEER-144: Frontend UI の Backend 統合対応（Phase 6: E2E 動作確認）

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-144 |
| タイトル | Frontend UI の Backend 統合対応（Phase 6: E2E 動作確認） |
| ステータス | Review |
| 関連Issue | #144 |
| 親ステアリング | STEER-137（Backend 連携統合戦略） |
| 優先度 | P1 |
| 作成日 | 2026-02-12 |

---

## 2. ストーリー（Why）

### 2.1 背景

Phase 1〜5 で個別機能（同期基盤、ルール評価、ActionCode、IVR、録音）の実装が進んでいるが、Frontend-Backend の E2E 動作確認が未実施の状態にある。

| Phase | Issue | ステアリング | 状態 | 内容 |
|-------|-------|------------|------|------|
| Phase 1 | #139 | STEER-139 | Approved | Frontend → Backend 同期基盤（Serversync Pull） |
| Phase 2 | #140 | STEER-140 | Approved | ルール評価エンジン + VR（`recording_enabled` フラグ対応） |
| Phase 3 | #141 | STEER-141 | Approved | 全 ActionCode 実装（BZ/NR/AN/VM/IV 基盤） |
| Phase 4-A | #142 | STEER-142 | Approved | IVR DB スキーマ設計 |
| Phase 4-B | #156 | （未作成） | - | IVR 実行エンジン |
| Phase 5 | #143 | STEER-143 | Approved | 録音実装強化 |
| **Phase 6** | **#144** | **本ステアリング** | **Review** | **Frontend-Backend E2E 統合** |

**現状の問題**:

| 未実施項目 | 詳細 | 影響 |
|-----------|------|------|
| **Frontend-Backend 連携検証** | Frontend で設定した内容が Backend で正しく実行されるかの E2E テストが未実施 | システム全体の動作保証ができない |
| **ActionCode 動作確認** | 各 ActionCode（BZ/NR/AN/VM/IV/VR）が Frontend UI から確認できない | ユーザーが設定した通りに動作するか不明 |
| **Serversync 動作検証** | Frontend の設定変更が Serversync 経由で Backend に正しく同期されるかの確認が不足 | 設定変更が反映されない可能性 |
| **最小限の動作確認手順** | 開発者が手元で動作確認できる手順が未整備 | 開発効率が低下 |

### 2.2 目的

Phase 6 で以下を達成する：

1. **Frontend-Backend の E2E フローを検証する**（設定作成 → Serversync → Backend 実行）
2. **各 ActionCode が Frontend の設定通りに動作することを確認する**（BZ/NR/AN/VM/IV/VR）
3. **最小限の動作確認手順を整備する**（開発者が手元で確認できる手順書）
4. **不整合・バグを洗い出して修正する**（Phase 1〜5 で見落とした問題の解消）

### 2.3 ユーザーストーリー

```
As a 開発者
I want to Frontend で設定した内容が Backend で正しく実行されることを確認したい
So that システム全体の動作を保証し、ユーザーに信頼性の高いサービスを提供できる

受入条件:
- [ ] AC-1: Frontend UI で着信拒否（BZ）を設定した後、該当番号からの着信が Backend で拒否される
- [ ] AC-2: Frontend UI で番号非通知拒否（NR）を設定した後、非通知着信が Backend で拒否される
- [ ] AC-3: Frontend UI でアナウンス（AN）を設定した後、着信時に指定アナウンスが再生される
- [ ] AC-4: Frontend UI で留守番電話（VM）を設定した後、着信時にアナウンス再生→録音が動作する
- [ ] AC-5: Frontend UI で IVR（IV）を設定した後、着信時に IVR フローが実行される（前提: #156 の仕様/実装確定後）
- [ ] AC-6: Frontend UI で通話録音（VR）を設定した後、着信時に通話が録音される
- [ ] AC-7: Frontend で設定を変更した後、Serversync が変更を検知し、Backend に同期される
- [ ] AC-8: Backend のログから、Frontend の設定に基づいて ActionCode が実行されたことが確認できる
- [ ] AC-9: 最小限の動作確認手順書が整備され、開発者が手元で E2E テストを実施できる
- [ ] AC-10: 発見された不整合・バグが Issue 化され、修正計画が立てられている
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-12 |
| 起票理由 | Frontend-Backend の E2E 統合確認が未実施 |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Code (claude-sonnet-4-5) |
| 作成日 | 2026-02-12 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "Issue #144 の背景・目的・スコープ・受入条件を追加" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| 1 | Codex | 2026-02-12 | OK | Status整合、AC-5前提明記、AC-9判定対象の固定を確認 |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | @MasanoriSuda |
| 承認日 | 2026-02-12 |
| 承認コメント | lgtm |

### 3.5 実装（該当する場合）

| 項目 | 値 |
|------|-----|
| 実装者 | - |
| 実装日 | - |
| 指示者 | - |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ者 | - |
| マージ日 | - |
| コミットハッシュ | - |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| docs/guides/e2e-testing.md | 新規作成 | E2E 動作確認手順書（AC-9 判定対象） |
| virtual-voicebot-frontend/README.md | 修正 | Backend 連携の動作確認手順を追記 |
| virtual-voicebot-backend/README.md | 修正 | Frontend 連携の動作確認手順を追記 |

### 4.2 影響するコード

| モジュール | 変更種別 | 概要 |
|-----------|---------|------|
| Frontend: UI 設定画面 | 確認 | 各 ActionCode の設定が正しく保存されるか確認 |
| Backend: Serversync | 確認 | Frontend の設定変更を正しく Pull できるか確認 |
| Backend: Routing Executor | 確認 | 各 ActionCode が設定通りに実行されるか確認 |
| Backend: Recording Manager | 確認 | 録音設定が正しく動作するか確認 |

---

## 5. 差分仕様（What / How）

### 5.1 スコープ定義

本 Phase のスコープを以下の 3 領域に分類する：

| 領域 | 内容 | 関連 Phase |
|------|------|-----------|
| **A: Frontend 設定確認** | Frontend UI で各 ActionCode の設定が正しく作成・保存できるか確認 | Phase 1（STEER-139） |
| **B: Serversync 同期確認** | Frontend の設定が Serversync 経由で Backend に同期されるか確認 | Phase 1（STEER-139） |
| **C: Backend 実行確認** | Backend が同期された設定に基づいて ActionCode を正しく実行するか確認 | Phase 2〜5（STEER-140/141/142/143） |

### 5.2 Phase 1〜5 との関連図

```
Phase 1 (STEER-139)         Phase 2 (STEER-140)         Phase 3 (STEER-141)
Frontend→Backend同期         ルール評価+VR               全ActionCode(BZ/NR/AN/VM/IV)
 │                            │                            │
 │                            │                            │
 └────────────────────────────┴────────────────────────────┘
                              │
                   Phase 4-A (STEER-142)      Phase 5 (STEER-143)
                   IVR DB スキーマ設計        録音実装強化
                              │                            │
                              └────────────────────────────┘
                                          │
         ┌────────────────────────────────────────────────────────────────┐
         │  Phase 6 (STEER-144): Frontend-Backend E2E 統合                │
         │                                                                │
         │  A: Frontend 設定確認    ← Phase 1 同期基盤の検証              │
         │     各 ActionCode の設定作成・保存確認                         │
         │                                                                │
         │  B: Serversync 同期確認  ← Phase 1 同期基盤の検証              │
         │     設定変更の検知・同期確認                                   │
         │                                                                │
         │  C: Backend 実行確認     ← Phase 2〜5 個別機能の統合検証       │
         │     各 ActionCode が設定通りに動作することを確認               │
         └────────────────────────────────────────────────────────────────┘
```

### 5.3 領域 A: Frontend 設定確認

#### 5.3.1 確認項目

Frontend UI で以下の設定が正しく作成・保存できることを確認：

| ActionCode | 設定項目 | 確認内容 |
|-----------|---------|---------|
| BZ（着信拒否） | 拒否番号リスト | 番号を追加・削除できるか |
| NR（番号非通知拒否） | 有効/無効フラグ | トグルで切り替えできるか |
| AN（アナウンス） | アナウンスファイル選択 | ファイルを選択・アップロードできるか |
| VM（留守番電話） | アナウンス + 録音有効化 | 設定を保存できるか |
| IV（IVR） | IVR フロー設定 | フロー図を作成・保存できるか |
| VR（通話録音） | recording_enabled, announce_enabled | 設定を保存できるか |

#### 5.3.2 確認手順

1. Frontend UI にログイン
2. 各 ActionCode の設定画面を開く
3. 設定を変更し、保存ボタンをクリック
4. 設定が正しく保存されたことを確認（画面再読み込みで設定が保持されているか）

### 5.4 領域 B: Serversync 同期確認

#### 5.4.1 確認項目

Serversync が Frontend の設定変更を検知し、Backend に同期できることを確認：

| 同期対象 | 確認内容 |
|---------|---------|
| call_action_rules | Frontend で設定したルールが Backend DB に同期される |
| announcements | Frontend でアップロードした音声ファイルが Backend に同期される |
| ivr_flows | Frontend で作成した IVR フローが Backend DB に同期される |

#### 5.4.2 確認手順

1. Frontend で設定を変更
2. Serversync のログで Pull リクエストが実行されたことを確認
3. Backend DB を直接確認し、設定が同期されていることを確認
   ```sql
   SELECT * FROM call_action_rules WHERE updated_at > NOW() - INTERVAL '1 minute';
   ```

### 5.5 領域 C: Backend 実行確認

#### 5.5.1 確認項目

Backend が同期された設定に基づいて各 ActionCode を正しく実行することを確認：

| ActionCode | 確認内容 | 期待動作 |
|-----------|---------|---------|
| BZ | 拒否番号リストに含まれる番号からの着信 | 即座に切断される |
| NR | 番号非通知の着信（NR 有効時） | 即座に切断される |
| AN | アナウンス再生設定がある着信 | 指定アナウンスが再生される |
| VM | 留守番電話設定がある着信 | アナウンス再生→録音開始 |
| IV | IVR 設定がある着信 | IVR フローが実行される |
| VR | 通話録音設定がある着信 | 通話が録音される |

#### 5.5.2 確認手順（例: BZ）

1. Frontend UI で着信拒否番号（例: `09012345678`）を設定
2. Serversync で同期が完了するのを待つ（数秒〜数十秒）
3. SIP クライアントから `09012345678` として Backend に発信
4. Backend のログで `BZ` ActionCode が実行されたことを確認
   ```
   [INFO] call_id=xxx action=BZ reason="Blocked number" caller=09012345678
   ```
5. 通話が即座に切断されることを確認

### 5.6 動作確認手順書の整備

開発者が手元で E2E テストを実施できるように、以下の手順書を作成：

**手順書に含めるべき内容:**
- 環境構築手順（Frontend + Backend + DB + Serversync のセットアップ）
- SIP クライアントの設定方法
- 各 ActionCode の動作確認手順
- ログの確認方法
- トラブルシューティング

**手順書の配置場所:**
- `docs/guides/e2e-testing.md`（横断ガイド）

### 5.7 不整合・バグの洗い出し

E2E テスト実施時に発見された不整合・バグを Issue 化し、優先度をつけて修正計画を立てる。

**Issue 化の基準:**
- **P0（即修正）**: 基本機能が動作しない致命的なバグ
- **P1（早急に修正）**: 一部機能が動作しない重要なバグ
- **P2（計画的に修正）**: 軽微な不整合・改善要望

### 5.8 スコープ外（将来）

| 項目 | 理由 |
|------|------|
| 自動 E2E テストの構築 | MVP 外。CI/CD パイプラインで自動化は将来対応 |
| Frontend UI の UX 改善 | 本 Phase は動作確認のみ。UI 改善は別 Issue で対応 |
| パフォーマンステスト | MVP 外。負荷試験は将来対応 |

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #144 | STEER-144 | 起票 |
| STEER-137 §5.2.6 Phase 6 | STEER-144 | 親戦略 → 具体化 |
| STEER-139（Phase 1） | STEER-144 §5.3/5.4 | 同期基盤 → E2E 検証 |
| STEER-140（Phase 2） | STEER-144 §5.5 | ルール評価 → E2E 検証 |
| STEER-141（Phase 3） | STEER-144 §5.5 | ActionCode 実装 → E2E 検証 |
| STEER-142（Phase 4-A） | STEER-144 §5.5 | IVR DB → E2E 検証 |
| STEER-143（Phase 5） | STEER-144 §5.5 | 録音実装 → E2E 検証 |

---

## 7. 未確定点・質問

### Q1: E2E テストの実施環境

Frontend + Backend + DB + Serversync を開発者のローカル環境で動かす手順が整備されているか？

**選択肢:**
- A: docker-compose で全コンポーネントを起動する（推奨）
- **B: 各コンポーネントを手動で起動する** ← **採用**
- C: 開発環境サーバーを用意してそこで実施する

**決定:** B（手動起動）を採用。将来的に docker-compose による自動化を目指す。

### Q2: SIP クライアントの選定

E2E テストで使用する SIP クライアントは何を使うか？

**選択肢:**
- **A: Zoiper（既存の動作確認で使用している場合）** ← **採用**
- B: Linphone（オープンソース）
- C: SIPp（自動化に向いている）

**決定:** A（Zoiper）を採用。

### Q3: Frontend の初期データセットアップ

Frontend で設定を作成する前に、初期データ（例: テスト用アナウンスファイル）を用意する必要があるか？

**選択肢:**
- A: 初期データセットアップスクリプトを用意する
- **B: 手動でアップロードする** ← **採用**
- C: サンプルデータを含めた docker-compose を用意する

**決定:** B（手動アップロード）を採用。将来的にセットアップスクリプトの整備を目指す。

---

## 8. 備考

- Phase 6 は Phase 1〜5 の統合確認フェーズであり、新規実装は最小限に抑える
- 発見されたバグは別 Issue として起票し、優先度に応じて修正する
- E2E テストの手順書は、将来の自動化の基礎となるため、詳細に記録する
- **MVP では手動テスト（手動起動 + Zoiper + 手動データセットアップ）を採用**
- **将来的には以下の自動化を目指す:**
  - docker-compose による環境統一
  - 初期データセットアップスクリプトの整備
  - SIPp による自動テストの構築

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-12 | 初版作成（Draft） | Claude Code (claude-sonnet-4-5) |
| 2026-02-12 | 未確定点 Q1/Q2/Q3 解消（手動テスト方針で合意） | Claude Code (claude-sonnet-4-5) |
| 2026-02-12 | レビュー指摘対応（Phase状態整合、AC-5前提追加、AC-9判定対象固定、手順書配置確定、レビュー記録追記） | Codex |
