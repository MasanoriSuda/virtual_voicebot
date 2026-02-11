# STEER-143: Backend 録音実装強化（Phase 5）

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-143 |
| タイトル | Backend 録音実装強化（Phase 5: 録音フラグ連動・メタデータ管理・Frontend 同期） |
| ステータス | Approved |
| 関連Issue | #143 |
| 親ステアリング | STEER-137（Backend 連携統合戦略） |
| 優先度 | P2 |
| 作成日 | 2026-02-11 |

---

## 2. ストーリー（Why）

### 2.1 背景

STEER-137 で定義した Backend 連携統合戦略は 5 Phase 構成であり、Phase 1〜4 で以下が実装済み/進行中：

| Phase | Issue | ステアリング | 状態 | 内容 |
|-------|-------|------------|------|------|
| Phase 1 | #139 | STEER-139 | Approved | Frontend → Backend 同期基盤（Serversync Pull） |
| Phase 2 | #140 | STEER-140 | Approved | ルール評価エンジン + VR（`recording_enabled` フラグ対応） |
| Phase 3 | #141 | STEER-141 | Approved | 全 ActionCode 実装（BZ/NR/AN/VM/IV 基盤） |
| Phase 4-A | #142 | STEER-142 | Approved | IVR DB スキーマ設計 |
| Phase 4-B | #156 | （未作成） | - | IVR 実行エンジン |
| **Phase 5** | **#143** | **本ステアリング** | **Approved** | **録音実装強化** |

**#156 について、ステアリングファイルが未作成の理由は#142を設計フェーズ、#156を実装フェーズとしたため、作り忘れではない**

**現状の問題**:

Phase 2（STEER-140）で `recording_enabled` フラグの制御と `RecordingManager.set_enabled()` は実装されたが、以下が未実装のまま残っている：

| 未実装項目 | 詳細 | 影響 |
|-----------|------|------|
| **録音フラグの実動作** | `recording_enabled=true` でフラグはセットされるが、実際の録音開始・停止の**統合テスト**がされていない | 録音が確実に動作するか不明 |
| **`announce_enabled` の実動作** | Phase 2 で「Phase 3 で実装」としたが、Phase 3（STEER-141）では AN/VM 自体の実装に注力し、`announce_enabled` フラグとの連動が未完了 | VR 着信時の録音開始アナウンス（「この通話は録音されます」）が動作しない |
| **録音メタデータ管理** | `recordings` テーブルにメタデータ（duration, file_size 等）を正しく書き込む処理が未検証 | Frontend で録音の詳細情報が表示できない |
| **sync_outbox 連携** | STEER-123（Draft）で指摘された録音データの outbox エンキュー漏れ | Frontend に録音データが同期されない |
| **VM 録音の保存パス** | Phase 3 で VM の仕様は定義したが、voicemail 録音ファイルの保存先・命名規則が未確定 | 録音ファイルの管理が困難 |

### 2.2 目的

Phase 5 で以下を達成する：

1. **録音フラグ（`recording_enabled`, `announce_enabled`）に基づく録音の実動作を保証する**
2. **録音メタデータの正確な管理**（`recordings` テーブルへの `duration_sec`, `file_size_bytes`, `format`, `started_at`, `ended_at` の書き込み）
3. **録音データの Frontend 同期を完結させる**（STEER-123 の outbox 課題を本ステアリングで包含して解消）
4. **VR/VM 両方の録音データ整合性を検証する**（`recording_type` と同期 payload を含む）

### 2.3 ユーザーストーリー

```
As a システム管理者
I want to 通話録音が設定通りに動作し、Frontend で再生・確認できる
So that 通話内容の証拠保全と業務改善に活用できる

受入条件:
- [ ] AC-1: ActionCode=VR (recording_enabled=true) の着信で通話が録音される
- [ ] AC-2: ActionCode=VR (recording_enabled=false) の着信では録音されない
- [ ] AC-3: announce_enabled=true の場合、通話開始時に録音告知アナウンス（「この通話は録音されます」）が再生される
- [ ] AC-4: announce_enabled=false の場合、録音告知アナウンスなしで録音が開始される
- [ ] AC-5: ActionCode=VM の着信でアナウンス再生後に留守番電話メッセージが録音される
- [ ] AC-6: 通話終了時に recordings テーブルに録音メタデータ（duration_sec, file_size_bytes, format, started_at, ended_at）が正しく書き込まれる
- [ ] AC-7: 通話終了時に sync_outbox へトランザクショナルに INSERT される（録音あり: call_log + recording + recording_file の3件 / 録音なし: call_log のみ）
- [ ] AC-8: Serversync 起動後、録音データが Frontend に同期され、Frontend UI で録音が再生できる
- [ ] AC-9: 通話録音（VR）と留守番電話（VM）が `recording_type`（`full_call`/`voicemail`）で区別される
- [ ] AC-10: 録音に関するログ（開始・停止・保存・エラー）が call_id 付きで出力される
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-11 |
| 起票理由 | STEER-137 Phase 5 の具体化。Phase 2/3 で後回しにした録音実動作の完結 |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Code (claude-opus-4-6) |
| 作成日 | 2026-02-11 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "Issue #143 の背景・目的・スコープ・Phase 1-4 関連・受入条件を追加" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| 1 | Codex | 2026-02-11 | OK | |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | Masanori Suda |
| 承認日 |  2026-02-11 |
| 承認コメント | lgtm |

### 3.5 実装（該当する場合）

| 項目 | 値 |
|------|-----|
| 実装者 | Codex へ引き継ぎ |
| 実装日 | - |
| 指示者 | - |
| 指示内容 | - |
| コードレビュー | - |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ実行者 | - |
| マージ日 | - |
| マージ先 | - |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| virtual-voicebot-backend/docs/requirements/RD-004_call-routing-execution.md | 修正 | 録音要件（FR-3.x）の受入条件を具体化 |
| STEER-123（録音 outbox エンキュー） | 参照/包含実装 | STEER-123 の outbox 課題を本ステアリングの実装範囲に含める |

### 4.2 影響するコード

| モジュール | 変更種別 | 概要 |
|-----------|---------|------|
| src/protocol/session/recording_manager.rs | 修正 | メタデータ収集（duration, file_size）、保存パス管理 |
| src/service/routing/executor.rs | 修正 | `announce_enabled` フラグに基づく録音告知アナウンス再生 |
| src/interface/db/postgres.rs | 修正 | トランザクショナルライト（call_log + recording + recording_file → sync_outbox） |
| src/protocol/session/coordinator.rs | 修正 | 通話終了時の録音メタデータ書き込み統合 |

---

## 5. 差分仕様（What / How）

### 5.1 スコープ定義

本 Phase のスコープを以下の 3 領域に分類する：

| 領域 | 内容 | 関連 Phase |
|------|------|-----------|
| **A: 録音フラグ実動作** | `recording_enabled` / `announce_enabled` に基づく録音開始・停止・告知アナウンスの統合 | Phase 2（STEER-140）の残課題 |
| **B: 録音メタデータ管理** | `recordings` テーブルへの正確なメタデータ書き込み + `recording_type` による VR/VM 区分 | Phase 3（STEER-141）の残課題 |
| **C: Frontend 同期完結** | sync_outbox へのトランザクショナルエンキュー（STEER-123 課題の包含実装） | Phase 1（STEER-139）の残課題 |

### 5.2 Phase 1〜4 との関連図

```
Phase 1 (STEER-139)         Phase 2 (STEER-140)         Phase 3 (STEER-141)
Frontend→Backend同期         ルール評価+VR               全ActionCode(BZ/NR/AN/VM/IV)
 │                            │                            │
 │ sync_outbox 未完結          │ recording_enabled          │ VM録音仕様定義
 │ (recording未エンキュー)       │ フラグ制御のみ              │ announce_enabled 未連動
 │                            │ announce_enabled 後回し      │
 └────────────┐               └────────────┐               └────────────┐
              ▼                            ▼                            ▼
         ┌─────────────────────────────────────────────────────────────────┐
         │  Phase 5 (STEER-143): 録音実装強化                               │
         │                                                                 │
         │  A: 録音フラグ実動作      ← Phase 2 残課題                        │
         │     recording_enabled 統合テスト                                  │
         │     announce_enabled 実装完結                                     │
         │                                                                 │
         │  B: 録音メタデータ管理    ← Phase 3 残課題                        │
         │     recordings テーブル書き込み                                    │
         │     recording_type による VR/VM 区分                              │
         │                                                                 │
         │  C: Frontend 同期完結    ← Phase 1 残課題 + STEER-123 包含実装    │
         │     sync_outbox トランザクショナルエンキュー                        │
         │     Frontend 録音再生の検証                                        │
         └─────────────────────────────────────────────────────────────────┘
```

### 5.3 領域 A: 録音フラグ実動作

#### 5.3.1 recording_enabled の動作保証

Phase 2（STEER-140）で実装済みの以下を統合テストで検証し、不足があれば修正する：

```rust
// STEER-140 で実装済み（executor.rs）
async fn execute_voicebot(&self, action: &ActionConfig, call_id: &str, session: &mut SessionCoordinator) -> Result<()> {
    session.set_outbound_mode(false);
    session.set_recording_enabled(action.recording_enabled);
    // ...
}

// STEER-140 で実装済み（recording_manager.rs）
pub fn start_main(&mut self) -> Result<(), RecordingError> {
    if !self.enabled {
        return Ok(());  // 録音無効なら何もしない
    }
    // ...
}
```

**検証ポイント**:
- `recording_enabled=true` → `RecordingManager.start_main()` が呼ばれ、録音ファイルが生成される
- `recording_enabled=false` → `start_main()` が早期リターンし、録音ファイルが生成されない
- 通話終了時に `RecordingManager.stop()` が呼ばれ、ファイルが正常にクローズされる

#### 5.3.2 announce_enabled の実装完結

Phase 2 の決定事項 D-03 で「Phase 3 で AN と一緒に実装」としたが、Phase 3 では AN ActionCode 自体の実装に注力し、VR の `announce_enabled` フラグとの連動が未完了。

**実装方針**:

```rust
// executor.rs - execute_vr() で announce_enabled を反映
async fn execute_vr(&self, action: &ActionConfig, call_id: &str, session: &mut SessionCoordinator) -> Result<()> {
    session.set_outbound_mode(false);
    session.set_recording_enabled(action.recording_enabled);

    // announce_enabled=true の場合のみ告知アナウンスモードに遷移
    if action.announce_enabled {
        session.set_announce_mode(true);

        // JSON 優先: call_action_rules.action_config.recordingAnnouncementId
        // 設定あり: Frontend announcement master を参照
        // 設定なし/不正: 固定音声 ANNOUNCEMENT_FALLBACK_WAV_PATH にフォールバック
        if let Some(recording_announcement_id) = action.recording_announcement_id {
            session.set_announcement_id(recording_announcement_id);
        }
    }
}
```

**録音告知アナウンスの音声**:
- MVP はシステム固定音声を既定値として使用する
- `call_action_rules.action_config.recordingAnnouncementId`（nullable）を導入し、設定時のみ Frontend announcement master を参照する
- 無音/設定ミス/参照失敗時は固定音声（`ANNOUNCEMENT_FALLBACK_WAV_PATH`）へフォールバックし、録音処理は継続する

### 5.4 領域 B: 録音メタデータ管理

#### 5.4.1 recordings テーブル書き込み

通話終了時に以下のメタデータを `recordings` テーブルに書き込む：

| カラム | 型 | 説明 | 取得元 |
|--------|-----|------|--------|
| id | UUID | 録音 ID | 自動生成 |
| call_log_id | UUID | 通話ログ ID | call_log 永続化結果 |
| duration_sec | INTEGER | 録音時間（秒） | RecordingManager（開始〜停止の差分） |
| file_size_bytes | BIGINT | ファイルサイズ | ファイルシステムから取得 |
| format | VARCHAR | 録音フォーマット | "wav"（固定） |
| file_path | TEXT | 録音ファイルパス | RecordingManager.output_path |
| recording_type | VARCHAR | 録音種別 | "full_call"（VR）/ "voicemail"（VM） |
| started_at | TIMESTAMPTZ | 録音開始時刻 | SessionCoordinator.started_wall |
| ended_at | TIMESTAMPTZ | 録音終了時刻 | 通話終了時刻 |

注記:
- `recording_type` は既存値 `full_call` を維持し、`call` へ改名しない
- `recordings` テーブルは既存カラムを使用するため、本件で新規カラム追加マイグレーションは不要

#### 5.4.2 VR / VM 録音パスの区分

| 録音種別 | recording_type | 保存先パス | ファイル名 |
|---------|---------------|-----------|-----------|
| 通話録音（VR） | "full_call" | `storage/recordings/{call_id}/` | `mixed.wav` |
| 留守番電話（VM） | "voicemail" | `storage/recordings/{call_id}/` | `mixed.wav` |

区分はディレクトリではなく `recording_type` と outbox payload で行う（既存配信・同期実装と整合させる）。

### 5.5 領域 C: Frontend 同期完結

STEER-123（Draft）の outbox 課題を本ステアリングの実装範囲に包含する。

#### 5.5.1 トランザクショナルライト

通話終了時に以下の 3 エントリを**単一トランザクション**で sync_outbox に INSERT：

```sql
BEGIN;
  INSERT INTO sync_outbox (entity_type, entity_id, payload)
  VALUES ('call_log', $call_id, $call_log_json);

  INSERT INTO sync_outbox (entity_type, entity_id, payload)
  VALUES ('recording', $recording_id, $recording_json);

  INSERT INTO sync_outbox (entity_type, entity_id, payload)
  VALUES ('recording_file', $recording_id, $recording_file_json);
COMMIT;
```

#### 5.5.2 Frontend 同期検証

Serversync が sync_outbox からエントリを取得し、Frontend に送信する流れを E2E で検証：

1. 通話終了 → sync_outbox に 3 エントリが INSERT される
2. Serversync が pending エントリを検出 → Frontend API に POST
3. Frontend が録音データを保存 → UI で録音再生可能

### 5.6 依存関係

```
STEER-139 (Phase 1) ───── 完了必須 ──────┐
STEER-140 (Phase 2) ───── 完了必須 ──────┤
STEER-141 (Phase 3) ───── 完了必須 ──────┼── STEER-143 (Phase 5)
STEER-123 (録音outbox) ── 包含実装 ──────┘
STEER-142 (Phase 4-A) ── 並行可能（依存なし）
```

### 5.7 スコープ外（将来）

| 項目 | 理由 |
|------|------|
| 録音ファイルの暗号化 | MVP 外。セキュリティ強化フェーズで対応 |
| 録音ファイルの自動削除（保持期間） | 運用要件が未確定 |
| ステレオ録音（話者分離） | 現在はモノラル録音のみ |
| 録音の一時停止/再開 | PoC では対応しない |
| AR（Announce+Record）ActionCode | RD-004 で MVP 外と定義済み |

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #143 | STEER-143 | 起票 |
| STEER-137 §5.2.3 Phase 5 | STEER-143 | 親戦略 → 具体化 |
| STEER-140 D-03 | STEER-143 §5.3 | announce_enabled 後回し → 完結 |
| STEER-141 §5.2.4 | STEER-143 §5.4 | VM 録音仕様 → メタデータ管理 |
| STEER-123 | STEER-143 §5.5 | sync_outbox バグフィックス → 包含実装 |
| call_action_rules.action_config | STEER-143 §5.3.2 | `recordingAnnouncementId`（nullable）導入 |
| RD-004 FR-3.1〜3.3 | STEER-143 | 録音要件 |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [ ] 要件の記述が明確か
- [ ] Phase 1〜4 との責務分界が明確か（何が既存実装で何が新規か）
- [ ] STEER-123 対応（包含実装）に漏れがないか
- [ ] 録音メタデータのカラム定義が既存 DB スキーマと整合しているか
- [ ] `recording_type` の既存値（`full_call` / `voicemail`）と整合しているか
- [ ] sync_outbox のカラム名（`entity_type`, `entity_id`, `payload`）と整合しているか
- [ ] テストケース（AC-1〜10）が網羅的か
- [ ] トレーサビリティが維持されているか

### 7.2 マージ前チェック（Approved → Merged）

- [ ] 実装が完了している
- [ ] コードレビューを受けている
- [ ] 関連テストが PASS
- [ ] Frontend で録音が再生できることが確認されている

---

## 8. 備考

- STEER-123（録音 outbox エンキュー）は Draft のまま独立で保持し、outbox 課題の実装は本ステアリングに包含して進める
- Phase 4（IVR）とは並行実施可能。IVR 内での録音制御は Phase 5 完了後に IVR 実行エンジン側で `recording_enabled` を参照する形で自然に統合される
- 録音告知アナウンスは MVP では固定音声を既定とし、`recordingAnnouncementId` 設定時のみ Frontend の announcement master を参照する
- `recording_type` は既存カラムを使用し、値は `full_call`（通話録音）/`voicemail`（留守電録音）を利用する。追加マイグレーションは不要

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-11 | 初版作成（Draft） | Claude Code (claude-opus-4-6) |
| 2026-02-11 | レビュー指摘反映（Review）: §2/§4/§5/§8 の整合修正（recording_type/sync_outbox/STEER-123 方針/AC 更新） | Codex |
| 2026-02-11 | オーナー承認（Status: Review → Approved） | Masanori Suda |
