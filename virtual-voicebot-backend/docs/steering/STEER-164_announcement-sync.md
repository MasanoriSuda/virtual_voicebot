# STEER-164: アナウンス同期機能の追加（STEER-139 設計誤り修正）

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-164 |
| タイトル | アナウンス同期機能の追加（STEER-139 設計誤り修正） |
| ステータス | Review |
| 関連Issue | #164 |
| 親ステアリング | STEER-139（Frontend-Backend 同期基盤） |
| 優先度 | P0 |
| 作成日 | 2026-02-12 |

---

## 2. ストーリー（Why）

### 2.1 背景

**STEER-139（Phase 1）の設計誤り:**

STEER-139 で Frontend → Backend の同期基盤が実装されたが、**アナウンス（announcements）の同期が含まれていなかった**。

| 同期対象 | STEER-139 での実装状況 | エンドポイント |
|---------|---------------------|--------------|
| NumberGroups | ✅ 実装済み | GET /api/number-groups |
| CallActions | ✅ 実装済み | GET /api/call-actions |
| IVR Flows | ✅ 実装済み | GET /api/ivr-flows/export |
| **Announcements** | **❌ 未実装** | **（未定義）** |

**Phase 3（STEER-141）との依存関係の破綻:**

STEER-141 で AN/VM/IV ActionCode が実装されたが、これらはアナウンスファイルを必要とする。

| ActionCode | 名称 | アナウンス依存 | 影響 |
|-----------|------|-------------|------|
| **AN** | アナウンス再生 | ✅ 必須（announcementId を参照） | アナウンスが Backend に存在しないため動作不可 |
| **VM** | 留守番電話 | ✅ 必須（アナウンス再生 + 録音） | アナウンスが Backend に存在しないため動作不可 |
| **IV** | IVR | ✅ 必須（IVR ノードが announcementId を参照） | アナウンスが Backend に存在しないため動作不可 |

### 2.2 現状の問題

**Issue #164 の発生:**

| 事象 | 詳細 | 根本原因 |
|------|------|---------|
| **AN ActionCode が動作しない** | Frontend で AN + announcementId を設定しても、Backend でアナウンスが再生されない | Backend に announcements テーブルのデータが存在しない |
| **フォールバックファイルが再生される** | Backend は `announcement not found` 警告を出し、デフォルトファイル（zundamon_sorry.wav）を再生 | - |
| **ユーザー体験の悪化** | 意図したアナウンスではなく、短いフォールバックファイルが再生される | アナウンス同期が未実装 |

**ログ証跡（Issue #164）:**

```
2026-02-11T16:50:48.224Z INFO [SessionCoordinator] evaluated action_code=AN
2026-02-11T16:50:48.224Z INFO [ActionExecutor] announcement_id=dca2a704-23c6-45fe-a16a-2c059bab0e45
2026-02-11T16:50:51.342Z WARN [session] announcement not found id=dca2a704-23c6-45fe-a16a-2c059bab0e45  ← Backend に存在しない
2026-02-11T16:50:51.342Z INFO [session] playing announcement path=.../zundamon_sorry.wav  ← フォールバック
2026-02-11T16:50:51.342Z INFO [playback_service] announcement finished, requesting hangup  ← 即座に終了
```

**Frontend の状態:**

- ✅ announcements.json にアナウンス定義が存在
  ```json
  "dca2a704-23c6-45fe-a16a-2c059bab0e45": {
    "name": "四国めたん非通知",
    "audioFileUrl": "/audio/announcements/dca2a704-23c6-45fe-a16a-2c059bab0e45.wav",
    "durationSec": 3.808
  }
  ```
- ✅ 音声ファイルが存在（179KB、3.8秒）
- ❌ Backend に同期されていない

### 2.3 目的

STEER-139 の設計誤りを修正し、**アナウンス同期機能を追加する**。

**達成目標:**
1. Frontend のアナウンス定義（announcements.json）を Backend DB（announcements テーブル）に同期
2. Frontend の音声ファイル（public/audio/announcements/*.wav）を Backend のストレージに転送
3. AN/VM/IV ActionCode が正しくアナウンスを再生できるようにする
4. IVR フローの announcementId 解決が正しく動作するようにする

### 2.4 ユーザーストーリー

```
As a システム管理者
I want to Frontend で設定したアナウンスが Backend で正しく再生される
So that AN/VM/IV ActionCode が期待通りに動作し、エンドユーザーに正しいアナウンスを届けられる

受入条件:
- [ ] AC-1: Frontend のアナウンス定義が Backend DB（announcements テーブル）に同期される
- [ ] AC-2: Frontend の音声ファイルが Backend のストレージに転送される
- [ ] AC-3: AN ActionCode が Frontend で設定したアナウンスを正しく再生する
- [ ] AC-4: VM ActionCode がアナウンス再生→録音を正しく実行する
- [ ] AC-5: IVR フローの announcementId 解決が正しく動作する（announcement not found 警告が出ない）
- [ ] AC-6: Serversync のログでアナウンス同期が確認できる
- [ ] AC-7: アナウンス更新時に Backend が変更を検知し、再同期される
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-12 |
| 起票理由 | Issue #164 発生（AN ActionCode が動作しない） |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Code (claude-sonnet-4-5) |
| 作成日 | 2026-02-12 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "Issue #164 の根本原因を分析し、STEER-164 を作成" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| - | - | - | - | - |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | - |
| 承認日 | - |
| 承認コメント | - |

### 3.5 実装（該当する場合）

| 項目 | 値 |
|------|-----|
| 実装者 | Codex (GPT-5) |
| 実装日 | 2026-02-11 |
| 指示者 | @MasanoriSuda |

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
| contract.md | 修正 | GET /api/announcements エンドポイント仕様を追加 |
| STEER-139 | 参照 | アナウンス同期が追加されたことを記録（変更履歴） |

### 4.2 影響するコード

#### Frontend

| ファイル | 変更種別 | 概要 |
|---------|---------|------|
| app/api/announcements/route.ts | 新規作成 | GET /api/announcements エンドポイント（announcements.json を返す） |

#### Backend

| ファイル | 変更種別 | 概要 |
|---------|---------|------|
| src/interface/sync/frontend_pull.rs | 修正 | アナウンス取得・保存ロジックを追加 |
| src/interface/sync/converters.rs | 修正 | AnnouncementDto → announcements テーブル変換ロジックを追加 |
| src/interface/sync/audio_transfer.rs | 新規作成 | 音声ファイル転送ロジック（Base64 or HTTP ダウンロード） |

---

## 5. 差分仕様（What / How）

### 5.1 スコープ定義

本ステアリングで実装する範囲：

| 領域 | 内容 | 優先度 |
|------|------|--------|
| **A: Frontend API 追加** | GET /api/announcements エンドポイントを追加 | P0 |
| **B: Backend Serversync 拡張** | アナウンス取得・保存ロジックを追加 | P0 |
| **C: 音声ファイル転送** | Frontend の音声ファイルを Backend に転送 | P0 |
| **D: 動作確認** | AN/VM/IV ActionCode が正しく動作することを確認 | P0 |

### 5.2 Frontend API 追加

#### 5.2.1 GET /api/announcements

**ファイル**: `virtual-voicebot-frontend/app/api/announcements/route.ts`

**実装内容**:
```typescript
import { NextResponse } from 'next/server';
import { readJsonFile } from '@/lib/storage';

export async function GET() {
  const data = await readJsonFile('announcements.json');
  return NextResponse.json({
    ok: true,
    announcements: Object.values(data.announcements || {})
  });
}
```

**レスポンス仕様（contract.md に追加）:**

```json
{
  "ok": true,
  "announcements": [
    {
      "id": "dca2a704-23c6-45fe-a16a-2c059bab0e45",
      "name": "四国めたん非通知",
      "description": null,
      "announcementType": "custom",
      "isActive": true,
      "folderId": "folder-greetings",
      "audioFileUrl": "/audio/announcements/dca2a704-23c6-45fe-a16a-2c059bab0e45.wav",
      "ttsText": "おそれいりますが、ばんごうをつうちのうえおかけなおしください。",
      "speakerId": 2,
      "speakerName": "四国めたん - ノーマル",
      "durationSec": 3.808,
      "language": "ja",
      "source": "tts",
      "createdAt": "2026-02-11T16:48:23.274Z",
      "updatedAt": "2026-02-11T16:48:23.274Z"
    }
  ]
}
```

### 5.3 Backend Serversync 拡張

#### 5.3.1 アナウンス取得ロジック

**ファイル**: `src/interface/sync/frontend_pull.rs`

**実装内容**:

```rust
// 既存の同期処理に追加
pub async fn sync_from_frontend(pool: &PgPool, frontend_url: &str) -> Result<()> {
    // ... 既存の同期処理（number-groups, call-actions, ivr-flows）...

    // アナウンス同期を追加
    info!("[serversync] fetching announcements from frontend");
    let announcements = fetch_announcements(frontend_url).await?;
    info!("[serversync] GET /api/announcements: success count={}", announcements.len());

    // DB に保存
    save_announcements(pool, &announcements).await?;

    // 音声ファイル転送
    for announcement in &announcements {
        if let Some(audio_url) = &announcement.audio_file_url {
            transfer_audio_file(frontend_url, audio_url, &announcement.id).await?;
        }
    }

    info!("[serversync] announcement sync completed");
    Ok(())
}

async fn fetch_announcements(frontend_url: &str) -> Result<Vec<AnnouncementDto>> {
    let url = format!("{}/api/announcements", frontend_url);
    let response = reqwest::get(&url).await?;
    let data: AnnouncementsResponse = response.json().await?;
    Ok(data.announcements)
}

#[derive(Debug, Deserialize)]
struct AnnouncementsResponse {
    ok: bool,
    announcements: Vec<AnnouncementDto>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AnnouncementDto {
    id: Uuid,
    name: String,
    description: Option<String>,
    announcement_type: String,
    is_active: bool,
    folder_id: String,
    audio_file_url: Option<String>,
    tts_text: Option<String>,
    speaker_id: Option<i32>,
    speaker_name: Option<String>,
    duration_sec: Option<f64>,
    language: String,
    source: String,
    created_at: String,
    updated_at: String,
}
```

#### 5.3.2 DB 保存ロジック

**ファイル**: `src/interface/sync/converters.rs`

**実装内容**:

```rust
async fn save_announcements(pool: &PgPool, announcements: &[AnnouncementDto]) -> Result<()> {
    // 既存のアナウンスを削除（冪等性確保）
    sqlx::query!("DELETE FROM announcements")
        .execute(pool)
        .await?;

    // 新しいアナウンスを挿入
    for announcement in announcements {
        sqlx::query!(
            r#"
            INSERT INTO announcements (
                id, name, description, audio_file_url, duration_sec,
                language, source, is_active, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
            announcement.id,
            announcement.name,
            announcement.description,
            announcement.audio_file_url,
            announcement.duration_sec,
            announcement.language,
            announcement.source,
            announcement.is_active,
            parse_timestamp(&announcement.created_at)?,
            parse_timestamp(&announcement.updated_at)?,
        )
        .execute(pool)
        .await?;
    }

    info!("[serversync] saved {} announcements to DB", announcements.len());
    Ok(())
}
```

### 5.4 音声ファイル転送

#### 5.4.1 転送方式の検討

**選択肢:**

| 方式 | メリット | デメリット | 採用 |
|------|---------|-----------|------|
| **A: HTTP ダウンロード** | シンプル、標準的 | Frontend が HTTP サーバーとして動作する必要がある | ✅ **推奨** |
| **B: Base64 エンコード** | API レスポンスに含められる | ファイルサイズが増加（約 1.3倍）、メモリ消費 | △ 将来検討 |
| **C: 共有ストレージ** | 転送不要 | インフラ依存、MVP では複雑 | ❌ MVP 外 |

**決定: 方式A（HTTP ダウンロード）を採用**

- Frontend の `public/audio/announcements/*.wav` を HTTP で配信（Next.js の静的ファイル配信機能を利用）
- Backend が HTTP GET でファイルをダウンロードし、ローカルストレージに保存

#### 5.4.2 実装

**ファイル**: `src/interface/sync/audio_transfer.rs`（新規作成）

**実装内容**:

```rust
use anyhow::Result;
use reqwest;
use std::fs;
use std::path::Path;
use tracing::{info, warn};
use uuid::Uuid;

/// 音声ファイルを Frontend からダウンロードして Backend のストレージに保存
pub async fn transfer_audio_file(
    frontend_url: &str,
    audio_file_url: &str,  // 例: "/audio/announcements/xxx.wav"
    announcement_id: &Uuid,
) -> Result<()> {
    // Frontend の URL を構築（例: "http://localhost:3000/audio/announcements/xxx.wav"）
    let download_url = format!("{}{}", frontend_url, audio_file_url);

    // ファイルをダウンロード
    info!("[audio_transfer] downloading {} from {}", announcement_id, download_url);
    let response = reqwest::get(&download_url).await?;

    if !response.status().is_success() {
        warn!("[audio_transfer] failed to download {} (status: {})", announcement_id, response.status());
        return Ok(());  // 警告のみ、同期は継続
    }

    let audio_data = response.bytes().await?;

    // Backend のストレージに保存（例: "data/announcements/xxx.wav"）
    let storage_dir = Path::new("data/announcements");
    fs::create_dir_all(storage_dir)?;

    let file_path = storage_dir.join(format!("{}.wav", announcement_id));
    fs::write(&file_path, &audio_data)?;

    info!("[audio_transfer] saved {} bytes to {:?}", audio_data.len(), file_path);
    Ok(())
}
```

**Backend のストレージパス:**
- `data/announcements/{announcement_id}.wav`

**playback_service での参照:**

既存の playback_service を修正し、`audio_file_url` から実際のファイルパスを解決する。

```rust
// 既存: announcements テーブルから audio_file_url を取得
let audio_file_url = /* DB から取得 */;

// 新規: audio_file_url をローカルパスに変換
let file_path = if let Some(url) = audio_file_url {
    // Frontend URL（例: "/audio/announcements/xxx.wav"）→ Backend パス（例: "data/announcements/xxx.wav"）
    let announcement_id = extract_announcement_id_from_url(&url)?;
    format!("data/announcements/{}.wav", announcement_id)
} else {
    // フォールバック
    "data/zundamon_sorry.wav".to_string()
};
```

### 5.5 DB スキーマ確認

**announcements テーブル（既存）:**

```sql
CREATE TABLE announcements (
    id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    audio_file_url VARCHAR(512),  -- Frontend の URL（参照用）
    duration_sec REAL,
    language VARCHAR(10) NOT NULL DEFAULT 'ja',
    source VARCHAR(50) NOT NULL DEFAULT 'upload',
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

**変更不要** - 既存スキーマで対応可能

### 5.6 スコープ外（将来）

| 項目 | 理由 |
|------|------|
| Base64 エンコード方式 | MVP では HTTP ダウンロードで十分 |
| 差分同期（変更検知） | MVP では全件同期で対応 |
| 音声ファイルの圧縮 | MVP では未実装（将来的に検討） |
| アナウンスの削除検知 | MVP では全削除→全挿入で対応 |

### 5.7 実装済み内容（2026-02-11 追記）

以下は Refs #164 に対して実装済み：

- ✅ `src/interface/sync/frontend_pull.rs`
  - `GET /api/announcements` 取得を追加
  - snapshot 保存処理に `announcements` を追加
- ✅ `src/interface/sync/converters.rs`
  - `FrontendAnnouncement` DTO を追加
  - `convert_announcements()` を追加し、`announcements` テーブルへ同期
  - 同期方針: Frontend に存在しないIDを削除 + 存在IDを upsert
- ✅ `src/protocol/session/services/playback_service.rs`
  - `cancel_playback()` 修正（再生未開始時に `finish_playback()` を呼ばない）
  - AN モードで再生開始直後に `AppHangup` される不具合を修正

今回未実装（別途対応が必要）：

- ⚠️ `src/interface/sync/audio_transfer.rs` の新規作成
- ⚠️ Frontend 音声ファイルを Backend `data/announcements/` へ転送する処理

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #164 | STEER-164 | 起票 |
| STEER-164 §5.3.1 | `src/interface/sync/frontend_pull.rs` | 実装済（`/api/announcements` pull追加） |
| STEER-164 §5.3.2 | `src/interface/sync/converters.rs` | 実装済（announcements同期追加） |
| Issue #164 ログ事象 | `src/protocol/session/services/playback_service.rs` | 実装済（AN即時終了バグ修正） |
| STEER-139 §5.2 | STEER-164 §5.2/5.3 | 同期基盤 → アナウンス同期追加 |
| STEER-141 §5.2.3 AN | STEER-164 §5.4 | AN 実装 → アナウンス転送 |
| STEER-141 §5.2.4 VM | STEER-164 §5.4 | VM 実装 → アナウンス転送 |
| STEER-141 §5.2.5 IV | STEER-164 §5.4 | IV 実装 → アナウンス転送 |

---

## 7. 未確定点・質問

### Q1: 音声ファイル転送方式

**質問**: HTTP ダウンロード方式で問題ないか？

**選択肢:**
- **A: HTTP ダウンロード（推奨）** - Frontend の静的ファイル配信を利用 ← **採用**
- B: Base64 エンコード - API レスポンスに含める
- C: 共有ストレージ - インフラ依存

**決定**: A（HTTP ダウンロード）を採用方針。

**実装状況（2026-02-11）**:
- metadata同期（announcements テーブル反映）までは実装済み
- 音声ファイル転送（HTTP ダウンロード実装）は未着手

### Q2: アナウンス更新方式

**質問**: アナウンスの更新検知をどうするか？

**選択肢:**
- **A: 全削除→全挿入（推奨）** - シンプル、冪等性確保 ← **採用**
- B: 差分検知（updated_at 比較） - 効率的だが実装が複雑

**決定**: A をベースに実装。

**実装状況（2026-02-11）**:
- Frontend に存在しない ID を削除
- 存在する ID は upsert（更新反映）

### Q3: 音声ファイルの配置場所

**質問**: Backend の音声ファイル配置場所は `data/announcements/` でよいか？

**選択肢:**
- **A: data/announcements/{id}.wav（推奨）** - 既存の data/ ディレクトリに統一 ← **採用**
- B: storage/announcements/{id}.wav - 別ディレクトリ
- C: /tmp/announcements/{id}.wav - 一時ディレクトリ

**決定**: 方針Aを維持（実装は保留）。

**実装状況（2026-02-11）**:
- `data/announcements/{id}.wav` への転送実装は未着手
- 既存の `audio_file_url` → ローカルパス解決を継続利用

---

## 8. 備考

- STEER-139 の設計誤りを修正するため、優先度を P0 に設定
- Issue #164 の解決が本ステアリングの目標
- Frontend の Next.js 静的ファイル配信機能（public/）を活用
- Backend の音声ファイル管理は playback_service で既に実装されているため、パス解決ロジックの追加のみ

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-12 | 初版作成（Draft） | Claude Code (claude-sonnet-4-5) |
| 2026-02-12 | 未確定点 Q1/Q2/Q3 解消（HTTP ダウンロード、全削除→全挿入、data/announcements/ 採用） | Claude Code (claude-sonnet-4-5) |
