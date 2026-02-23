# STEER-227: 別マシン構成でのアナウンス音声 HTTP 取得対応

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-227 |
| タイトル | 別マシン構成でのアナウンス音声 HTTP 取得対応 |
| ステータス | Approved |
| 関連Issue | #227 |
| 優先度 | P1 |
| 作成日 | 2026-02-23 |

---

## 2. ストーリー（Why）

### 2.1 背景

STEER-226（#226）で Frontend / Backend を別マシンで動かせる環境変数対応を行ったが、
アナウンス再生に同一マシン前提の実装が残っていた。

**現状の問題:**

| 問題 | 詳細 |
|------|------|
| 音声ファイルのローカルパス変換 | `map_audio_file_url_to_local_path()`（`coordinator.rs` L679）は URL を常にローカルファイルパスへ変換する |
| Frontend ディレクトリへの依存 | 変換先が `CARGO_MANIFEST_DIR/../virtual-voicebot-frontend/public/audio/announcements/` 固定（`coordinator.rs` L692-703） |
| URL がホストなし相対パス | Frontend は `audio_file_url` を `/audio/announcements/{id}.wav`（ホストなし）として保存する（`lib/db/announcements.ts` L13, L257） |

結果として、Frontend が PC（ローカルサーバー）・Backend がラズパイの構成では、
Backend（ラズパイ）が PC 上の音声ファイルを参照できず `No such file or directory` になる。

### 2.2 目的

Backend が `FRONTEND_BASE_URL` を使って音声ファイルを HTTP GET し、
一時ファイルとして取得してから再生することで、別マシン構成でのすべての音声再生を実現する。

対象は `map_audio_file_url_to_local_path()` を呼ぶ全箇所：
- **アナウンス再生（AN mode）**: `coordinator.rs:513/522`
- **IVR メニュー音声**: `handlers/mod.rs:1000`
- **VB（Voicebot）イントロ音声**: `handlers/mod.rs:1283`

同一マシン構成（`FRONTEND_BASE_URL=http://localhost:3000` 等）でも動作し、
後方互換性を維持する。

### 2.3 ユーザーストーリー

```text
As a 開発者
I want to Frontend=ローカル PC、Backend=ラズパイ の構成でアナウンス再生したい
So that 別マシン構成でも voicebot の全機能が利用できる

受入条件:
- [ ] Frontend=PC、Backend=ラズパイの構成でアナウンス音声（AN mode）が着信時に再生される
- [ ] Frontend=PC、Backend=ラズパイの構成で IVR メニュー音声が再生される
- [ ] Frontend=PC、Backend=ラズパイの構成で VB（Voicebot）イントロ音声が再生される
- [ ] FRONTEND_BASE_URL を設定しない場合でも既存（同一マシン）動作が壊れない
- [ ] 一時ファイルはセッション終了時に削除される
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-23 |
| 起票理由 | Frontend=PC / Backend=ラズパイ の構成でアナウンス再生ができない（#226 の未対応箇所） |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Sonnet 4.6 |
| 作成日 | 2026-02-23 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "#227 のステアリング作成。Codex 調査済み: coordinator.rs の map_audio_file_url_to_local_path() が URL をローカルパス変換するため別マシン構成で失敗する。コード修正が必要" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| 1 | Codex | 2026-02-23 | NG | ①HTTP失敗時「音声なし」表現が誤り（実際は ANNOUNCEMENT_FALLBACK_WAV_PATH）②URL prefix バリデーション未明記 ③Cargo.toml が影響範囲に未記載 |
| 2 | Codex | 2026-02-23 | NG | ①HTTP 4xx/5xx が失敗扱いされない（error_for_status 未記載）②tempfile が dev-dependencies のみで dependencies 移動が必要 ③Q1確定後も §5.4/5.5 に「Codex が判断」が残存 |
| 3 | Codex | 2026-02-23 | NG | ①IVR メニュー音声（handlers/mod.rs:1000）・VR 転送イントロ音声（handlers/mod.rs:1283）が差分仕様の対象外 ②§3.6 マージ先に .env.example と Cargo.toml が未記載 |
| 4 | Codex | 2026-02-23 | NG | ①L1283 は "VR" ではなく "VB"（Voicebot）分岐 ②IVR/VB のフォールバック先が `ANNOUNCEMENT_FALLBACK_WAV_PATH` で統一されているが実際は IVR→`IVR_INTRO_WAV_PATH`・VB→`VOICEBOT_INTRO_WAV_PATH` |
| 5 | Codex | 2026-02-23 | NG | ①§5.4 コメントに `Option<NamedTempFile>` の旧表記が残存（確定済みは `Vec`）②§5.6.1 コメントでフォールバック先を `ANNOUNCEMENT_FALLBACK_WAV_PATH` 固定と誤記 ③§5.5 に `VR intro` の旧表記が残存 |
| 6 | Codex | 2026-02-23 | OK | 指摘なし。全指摘解消を確認 |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | @MasanoriSuda |
| 承認日 | 2026-02-23 |
| 承認コメント | 承認 |

### 3.5 実装

| 項目 | 値 |
|------|-----|
| 実装者 | Codex |
| 実装日 | 2026-02-23 |
| 指示者 | @MasanoriSuda |
| 指示内容 | #227 承認済みステアリングに従い、アナウンス/IVR/VB イントロ音声の remote fetch + temp file 管理を実装 |
| コードレビュー | ローカル検証実施（`cargo fmt --check` / `cargo test --lib` / `cargo clippy --lib -- -D warnings` / `cargo build --lib`） |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ実行者 | - |
| マージ日 | - |
| マージ先 | `coordinator.rs`、`handlers/mod.rs`、`config/mod.rs`、`Cargo.toml`、`.env.example`（ルート） |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| `.env.example`（ルート） | 修正 | `FRONTEND_BASE_URL` の説明にアナウンス音声取得用途を追記 |

### 4.2 影響するコード

| ファイル | 変更種別 | 概要 |
|---------|---------|------|
| `src/protocol/session/coordinator.rs` | 修正 | `resolve_announcement_playback_path()` を HTTP 取得対応に変更。`map_audio_file_url_to_local_path()` を共通ヘルパー呼び出しに置き換え |
| `src/protocol/session/coordinator.rs` | 追加 | `fetch_audio_to_temp()` 関数追加（HTTP GET → 一時ファイル保存） |
| `src/protocol/session/coordinator.rs` | 修正 | `SessionCoordinator` に `audio_tmp_files: Vec<tempfile::NamedTempFile>` フィールド追加（複数音声の一時ファイル管理） |
| `src/protocol/session/handlers/mod.rs` | 修正 | IVR メニュー音声（L1000）・VB（Voicebot）イントロ音声（L1283）の `map_audio_file_url_to_local_path()` 呼び出しを HTTP 取得対応に変更 |
| `src/shared/config/mod.rs` | 追加 | `AnnouncementConfig` または既存 config への `frontend_base_url` 追加 |
| `Cargo.toml`（virtual-voicebot-backend） | 修正 | `tempfile = "3"` が `[dev-dependencies]` にのみ存在するため、`[dependencies]` に移動（本番コードから参照するため） |

---

## 5. 差分仕様（What / How）

### 5.1 設計方針

`map_audio_file_url_to_local_path()` の呼び出しを非同期 HTTP 取得に置き換える。

| 変更点 | 変更前 | 変更後 |
|--------|--------|--------|
| 音声ファイル取得方法（全箇所） | URL → ローカルパス変換（同一マシン前提）| `FRONTEND_BASE_URL` + 相対パス で HTTP GET し一時ファイルに保存 |
| 対象呼び出し箇所 | `coordinator.rs:513/522`（AN）、`handlers/mod.rs:1000`（IVR）、`handlers/mod.rs:1283`（VB intro） | 同左（全箇所を置き換え） |
| 設定変数 | なし（固定パス） | `FRONTEND_BASE_URL`（main voicebot バイナリからも参照可能にする） |
| 後方互換 | - | `FRONTEND_BASE_URL` 未設定時は従来のローカルパス変換にフォールバック |
| 一時ファイル管理 | - | `SessionCoordinator.audio_tmp_files: Vec<NamedTempFile>` でセッション中保持 |

### 5.2 `FRONTEND_BASE_URL` の config 拡張

現在 `FRONTEND_BASE_URL` は `SyncConfig`（`serversync` バイナリ専用）にのみ存在し、
main voicebot バイナリの `coordinator.rs` からはアクセスできない。

**対応**: `shared/config/mod.rs` に独立した `announcement_config()` 関数（または既存グローバル config への追加）を設ける。

```rust
// src/shared/config/mod.rs に追加
#[derive(Clone, Debug)]
pub struct AnnouncementConfig {
    /// Frontend サーバーのベース URL（例: http://192.168.1.5:3000）
    /// 未設定の場合はローカルパス変換にフォールバック
    pub frontend_base_url: Option<String>,
}

impl AnnouncementConfig {
    pub fn from_env() -> Self {
        Self {
            frontend_base_url: env_non_empty("FRONTEND_BASE_URL"),
        }
    }
}

static ANNOUNCEMENT_CONFIG: OnceLock<AnnouncementConfig> = OnceLock::new();

pub fn announcement_config() -> &'static AnnouncementConfig {
    ANNOUNCEMENT_CONFIG.get_or_init(AnnouncementConfig::from_env)
}
```

> **Note:** `SyncConfig` の `FRONTEND_BASE_URL` は `required`（未設定でエラー）だが、
> `AnnouncementConfig` では `Optional`（未設定時はフォールバック）とする。
> 同じ環境変数を異なる config で読み分けることは問題ない。

### 5.3 `fetch_audio_to_temp()` 関数

**事前条件**: `relative_url_path` は `/audio/announcements/` で始まること。
それ以外のパスは呼び出し元でバリデーション済み（§5.4 参照）。

```rust
// coordinator.rs 内（または service/ai/mod.rs パターンに倣い独立モジュール化）
async fn fetch_audio_to_temp(relative_url_path: &str, frontend_base_url: &str) -> Result<tempfile::NamedTempFile> {
    // 呼び出し元で prefix チェック済みを前提とするが、念のためアサート
    debug_assert!(relative_url_path.starts_with("/audio/announcements/"));
    let full_url = format!("{}{}", frontend_base_url.trim_end_matches('/'), relative_url_path);
    let client = reqwest::Client::builder()
        .timeout(config::timeouts().ai_http)  // 既存の HTTP タイムアウトを流用
        .build()?;
    let resp = client.get(&full_url).send().await?.error_for_status()?;
    // error_for_status() により 4xx/5xx は Err に変換される
    let bytes = resp.bytes().await?;
    let mut tmp = tempfile::NamedTempFile::new()?;
    std::io::Write::write_all(&mut tmp, &bytes)?;
    Ok(tmp)
}
```

- `tempfile::NamedTempFile` を使用（`drop` 時に自動削除）
- `tempfile = "3"` は現在 `[dev-dependencies]` にのみ存在するため、`Cargo.toml` の `[dependencies]` に移動が必要

### 5.4 `resolve_announcement_playback_path()` の変更

```rust
// 変更前（L511-L541）
pub(crate) async fn resolve_announcement_playback_path(&self) -> Option<String> {
    if let Some(audio_file_url) = self.announcement_audio_file_url.clone() {
        return Some(map_audio_file_url_to_local_path(audio_file_url));  // ← ローカル変換
    }
    ...
    Ok(Some(audio_file_url)) => Some(map_audio_file_url_to_local_path(audio_file_url)),
    ...
}

// 変更後（疑似コード）
// Q1=A案確定: SessionCoordinator に Vec<NamedTempFile> フィールドを追加し、
// セッション終了時に一括 drop → 自動削除する。
// シグネチャ変更: &mut self（フィールド書き込みのため）
pub(crate) async fn resolve_announcement_playback_path(
    &mut self,
) -> Option<String> {
    let audio_file_url = self.announcement_audio_file_url.clone()
        .or_else(|| /* DB lookup */ None)?;

    let cfg = config::announcement_config();
    if let Some(base_url) = &cfg.frontend_base_url {
        // HTTP 取得パス
        let url_path = extract_url_path(&audio_file_url);  // ホスト部除去

        // /audio/announcements/ 以外のパスは取得しない（セキュリティ・スコープ制限）
        if !url_path.starts_with("/audio/announcements/") {
            log::warn!(
                "[session {}] unexpected audio url path (not /audio/announcements/): {}",
                self.call_id, url_path
            );
            return None;
            // → 呼び出し側（handlers/mod.rs:279 等）で ANNOUNCEMENT_FALLBACK_WAV_PATH にフォールバック
        }

        match fetch_audio_to_temp(&url_path, base_url).await {
            Ok(tmp_file) => {
                let path = tmp_file.path().to_string_lossy().to_string();
                // Q1=A案（Vec対応）: audio_tmp_files に push し、セッション終了時に一括 drop
                self.audio_tmp_files.push(tmp_file);
                return Some(path);
            }
            Err(err) => {
                log::warn!(
                    "[session {}] failed to fetch audio from {} err={:?}",
                    self.call_id, base_url, err
                );
                return None;
                // → 呼び出し側（handlers/mod.rs:279 等）で ANNOUNCEMENT_FALLBACK_WAV_PATH にフォールバック
            }
        }
    }

    // FRONTEND_BASE_URL 未設定 → 従来のローカルパス変換
    Some(map_audio_file_url_to_local_path(audio_file_url))
}
```

### 5.5 一時ファイルのライフタイム管理（Q1=A案確定・複数音声対応）

`NamedTempFile` を `drop` すると即座にファイルが削除される。
スコープが AN / IVR / VB（Voicebot）イントロ の 3 種類に拡大されたため、
**A案** を `Vec<NamedTempFile>` で採用する：

```rust
// SessionCoordinator 構造体に追加
audio_tmp_files: Vec<tempfile::NamedTempFile>,  // セッション終了時に一括 drop
```

**実装上の変更点:**
- `SessionCoordinator` 構造体に `audio_tmp_files: Vec<NamedTempFile>` フィールド追加（`Option<>` から `Vec` に変更）
- HTTP 取得成功時: `self.audio_tmp_files.push(tmp_file)` し、パス文字列を返す
- セッション終了時（`drop`）: `Vec` 内の全 `NamedTempFile` が自動削除
- `resolve_announcement_playback_path()` を `&mut self` に変更（§5.4）
- `handlers/mod.rs` の IVR / VB 呼び出し箇所も同様に `&mut self` 経由で取得

### 5.6 IVR メニュー音声・VB（Voicebot）イントロ音声の変更（handlers/mod.rs）

`handlers/mod.rs` の 2 箇所は `map_audio_file_url_to_local_path()` を直接呼び出しているため、
§5.4 と同じ HTTP 取得ロジックを適用する。

#### 5.6.1 共通ヘルパーメソッドの追加（coordinator.rs）

URL → 再生パス解決ロジックを `resolve_audio_file_url()` として SessionCoordinator のメソッドに切り出す。
`resolve_announcement_playback_path()` もこのメソッドに委譲する（§5.4 疑似コードを更新）。

```rust
// coordinator.rs に追加（private ヘルパー）
// 呼び出し元から URL 文字列を受け取り、再生可能なパスを返す
// FRONTEND_BASE_URL 設定時: HTTP 取得 → 一時ファイルパス
// FRONTEND_BASE_URL 未設定時: ローカルパス変換（後方互換）
async fn resolve_audio_file_url(&mut self, audio_file_url: String) -> Option<String> {
    let cfg = config::announcement_config();
    if let Some(base_url) = &cfg.frontend_base_url {
        let url_path = extract_url_path(&audio_file_url);
        if !url_path.starts_with("/audio/announcements/") {
            log::warn!(
                "[session {}] unexpected audio url path (not /audio/announcements/): {}",
                self.call_id, url_path
            );
            return None;
            // → 呼び出し側の既存フォールバックに委ねる（AN/VM・IVR・VB で異なる。§5.6.2/5.6.3 参照）
        }
        match fetch_audio_to_temp(&url_path, base_url).await {
            Ok(tmp_file) => {
                let path = tmp_file.path().to_string_lossy().to_string();
                self.audio_tmp_files.push(tmp_file);
                return Some(path);
            }
            Err(err) => {
                log::warn!(
                    "[session {}] failed to fetch audio from {} err={:?}",
                    self.call_id, base_url, err
                );
                return None;
                // → 呼び出し側の既存フォールバックに委ねる（AN/VM・IVR・VB で異なる。§5.6.2/5.6.3 参照）
            }
        }
    }
    // FRONTEND_BASE_URL 未設定 → 従来のローカルパス変換
    Some(map_audio_file_url_to_local_path(audio_file_url))
}
```

`resolve_announcement_playback_path()` は DB lookup 後に `self.resolve_audio_file_url(url).await` へ委譲するよう変更する。

#### 5.6.2 IVR メニュー音声の変更（handlers/mod.rs L998-1007 付近）

```rust
// 変更前（L998-1002）:
self.ivr_menu_audio_file_url = Some(
    menu.audio_file_url
        .map(super::map_audio_file_url_to_local_path)
        .unwrap_or_else(|| super::IVR_INTRO_WAV_PATH.to_string()),
);

// 変更後（疑似コード）:
// HTTP 取得失敗（None）時は IVR_INTRO_WAV_PATH にフォールバック（既存挙動と同等）
let resolved_path = match menu.audio_file_url {
    Some(url) => self.resolve_audio_file_url(url).await
        .unwrap_or_else(|| super::IVR_INTRO_WAV_PATH.to_string()),
    None => super::IVR_INTRO_WAV_PATH.to_string(),
};
self.ivr_menu_audio_file_url = Some(resolved_path);
```

- HTTP 取得失敗（`None`）時のフォールバックは `IVR_INTRO_WAV_PATH`（既存コード L1001/1007 と同等）
- `ANNOUNCEMENT_FALLBACK_WAV_PATH` は AN/VM mode のフォールバックであり、IVR メニューには適用されない

#### 5.6.3 VB（Voicebot）イントロ音声の変更（handlers/mod.rs L1278-1287 付近）

```rust
// 変更前（L1278-1287）:
let intro_path = if action.include_announcement.unwrap_or(false) {
    action
        .announcement_audio_file_url
        .as_ref()
        .cloned()
        .map(super::map_audio_file_url_to_local_path)
        .or_else(|| Some(super::VOICEBOT_INTRO_WAV_PATH.to_string()))
} else {
    None
};

// 変更後（疑似コード）:
let intro_path = if action.include_announcement.unwrap_or(false) {
    match action.announcement_audio_file_url.clone() {
        Some(url) => {
            // HTTP 取得失敗（None）時は VOICEBOT_INTRO_WAV_PATH にフォールバック（既存挙動と同等）
            Some(self.resolve_audio_file_url(url).await
                .unwrap_or_else(|| super::VOICEBOT_INTRO_WAV_PATH.to_string()))
        }
        None => Some(super::VOICEBOT_INTRO_WAV_PATH.to_string()),
    }
} else {
    None  // include_announcement=false の場合はイントロ音声なし（既存挙動と同等）
};
```

- `include_announcement=false` の場合は従来通り音声なし（`intro_path = None`）
- `include_announcement=true` かつ URL あり: HTTP 取得を試みる。失敗時（`None`）は `VOICEBOT_INTRO_WAV_PATH` にフォールバック（既存コード L1284 と同等）
- `include_announcement=true` かつ URL なし: `VOICEBOT_INTRO_WAV_PATH` を使用（既存挙動と同等）
- `ANNOUNCEMENT_FALLBACK_WAV_PATH` は AN/VM mode のフォールバックであり、VB イントロには適用されない

> **Note:** `resolve_audio_file_url()` は `async` かつ `&mut self` を要求するため、
> 呼び出し元のハンドラ関数シグネチャが `&mut self` であることを確認すること。
> 現在 `super::map_audio_file_url_to_local_path()` は同期・`&self` で呼ばれているため、
> 非同期化による影響範囲を Codex が確認する。

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #227 | STEER-227 | 起票 |
| STEER-226 | STEER-227 | 前提（別マシン構成対応の未対応箇所） |
| STEER-227 | `coordinator.rs` | AN mode・共通ヘルパー修正 |
| STEER-227 | `handlers/mod.rs` | IVR メニュー音声・VB（Voicebot）イントロ音声修正 |
| STEER-227 | `config/mod.rs` | `AnnouncementConfig` 追加 |
| STEER-227 | `Cargo.toml` | `tempfile` 依存関係移動 |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [ ] `AnnouncementConfig.frontend_base_url` が `Optional` で後方互換性が保たれているか
- [ ] `SyncConfig.frontend_base_url`（required）との重複読み込みに問題がないか
- [ ] `fetch_audio_to_temp()` のタイムアウト設定が適切か（`config::timeouts().ai_http` を流用）
- [ ] 一時ファイルのライフタイム管理方針（A案: `SessionCoordinator` フィールド追加、セッション終了時 drop）に合意しているか
- [ ] HTTP 取得失敗（4xx/5xx 含む）時のフォールバック方針（呼び出し側の既存フォールバックに委ねる：AN/VM→`ANNOUNCEMENT_FALLBACK_WAV_PATH`、IVR→`IVR_INTRO_WAV_PATH`、VB→`VOICEBOT_INTRO_WAV_PATH`）に合意しているか
- [ ] `tempfile` を `[dev-dependencies]` から `[dependencies]` へ移動することに問題はないか

### 7.2 マージ前チェック（Approved → Merged）

- [ ] Frontend=PC、Backend=ラズパイの構成でアナウンス音声（AN mode）が着信時に再生される
- [ ] Frontend=PC、Backend=ラズパイの構成で IVR メニュー音声が再生される
- [ ] Frontend=PC、Backend=ラズパイの構成で VB（Voicebot）イントロ音声が再生される（`include_announcement=true` 設定時）
- [ ] `FRONTEND_BASE_URL` 未設定（同一マシン構成）でも既存動作が壊れない
- [ ] セッション終了後に一時ファイルが残存しないことを確認している
- [ ] HTTP 取得失敗時（Frontend サーバ停止等）に WARNING ログが出て各呼び出し側の既存フォールバックが再生されることを確認している（AN/VM→`ANNOUNCEMENT_FALLBACK_WAV_PATH`、IVR→`IVR_INTRO_WAV_PATH`、VB→`VOICEBOT_INTRO_WAV_PATH`）
- [ ] `/audio/announcements/` 以外のパスを渡した場合に WARNING ログが出て各呼び出し側のフォールバックが適用されることを確認している

---

## 8. 未確定点・質問

| # | 質問 | 選択肢 | 推奨 | オーナー回答 |
|---|------|--------|------|-------------|
| Q1 | 一時ファイルのライフタイム管理をどうするか | A: セッション構造体に保持 / B: persist + 明示削除 / C: 固定パス + 手動削除 | **A（SessionCoordinator フィールド追加）が最も安全だが構造体変更を伴う。Codex に委ねる** | **推奨案（A）。Codex に委ねる** |
| Q2 | HTTP 取得失敗時に同一マシンフォールバック（ローカルパス変換）を試みるか | ローカルパス変換フォールバックあり / 呼び出し側の既存フォールバックに委ねる | **呼び出し側の既存フォールバックに委ねる（別マシン構成でローカルパス変換を試みても無意味なため）。AN/VM→`ANNOUNCEMENT_FALLBACK_WAV_PATH`、IVR→`IVR_INTRO_WAV_PATH`、VB→`VOICEBOT_INTRO_WAV_PATH`** | **推奨案（なし・呼び出し側に委ねる）** |
| Q3 | `tempfile` crate が未依存の場合、`Cargo.toml` への追加で問題ないか | 追加 / 代替実装（tempdir 手動管理） | **追加（`tempfile = "3"`）。既存のエコシステムに沿う** | **推奨案（追加）** |

---

## 9. リスク・ロールバック観点

| リスク | 影響 | 緩和策 |
|--------|------|--------|
| `FRONTEND_BASE_URL` 未設定の既存環境 | フォールバックでローカルパス変換が動き、影響なし | `Optional` 設計により後方互換 |
| Frontend サーバ停止中の HTTP 取得失敗 | 呼び出し側の既存フォールバックが再生される（AN/VM→`ANNOUNCEMENT_FALLBACK_WAV_PATH`、IVR→`IVR_INTRO_WAV_PATH`、VB→`VOICEBOT_INTRO_WAV_PATH`） | WARN ログ出力、通話は継続 |
| 一時ファイルの残存 | ディスク使用量増加 | `NamedTempFile` の drop 管理または定期クリーンアップ |
| `tempfile` crate の `dev-dependencies` → `dependencies` 移動 | 本番ビルドに含まれるようになるが、`tempfile = "3"` は既存の crate | Cargo.lock 差分なし（既に解決済み） |
| handlers/mod.rs のハンドラを非同期化 | `map(super::map_audio_file_url_to_local_path)` が同期だったため、async への変更でハンドラ関数シグネチャに影響する可能性 | Codex が `&mut self` 伝播範囲を確認 |

**ロールバック手順:** PR を revert。`.env.example` への `FRONTEND_BASE_URL` 説明追記も revert。
変更ファイルは `coordinator.rs` / `handlers/mod.rs` / `config/mod.rs` / `Cargo.toml` / `.env.example` だが、
いずれもロールバックで元に戻せる。

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-23 | 初版作成（Codex 調査結果を元に差分仕様を記述） | Claude Sonnet 4.6 |
| 2026-02-23 | Codex レビュー Round 1 NG 対応（§4.2 Cargo.toml 追加、§5.3 prefix バリデーション追加、§5.4 フォールバック表現を ANNOUNCEMENT_FALLBACK_WAV_PATH に修正、§7.1/7.2/8/9 同期修正） | Claude Sonnet 4.6 |
| 2026-02-23 | Q1/Q2/Q3 オーナー回答記入（全て推奨案で確定） | @MasanoriSuda |
| 2026-02-23 | Codex レビュー Round 2 NG 対応（§5.3 error_for_status 追加、§4.2/9 tempfile dev→dependencies 移動に修正、§5.4/5.5 Q1=A案確定で更新、§7.1 チェック項目更新） | Claude Sonnet 4.6 |
| 2026-02-23 | スコープ拡大（ユーザー指示）＆ Codex レビュー Round 3 NG 対応（§2.2/2.3 全 wav 再生を対象に変更、§3.3 Round 3 NG 追加、§3.6 .env.example/Cargo.toml 追加、§4.2 handlers/mod.rs 追加、§5.1 対象箇所・一時ファイル管理行追加、§5.4/5.5 Vec 化 + handlers 連携明記、§5.6 IVR/VR 変更疑似コード追加、§7.1 非同期化リスク追記、§7.2 IVR/VR 確認項目追加、§9 ロールバック対象ファイル更新） | Claude Sonnet 4.6 |
| 2026-02-23 | Codex レビュー Round 4 NG 対応（全箇所 VR→VB（Voicebot）に修正、§5.6.2 IVR フォールバックを IVR_INTRO_WAV_PATH に修正・疑似コード更新、§5.6.3 include_announcement 制御と VOICEBOT_INTRO_WAV_PATH フォールバックを反映・疑似コード全面更新、§7.1/7.2/Q2/§9 フォールバック記述を AN/VM・IVR・VB 別に明記） | Claude Sonnet 4.6 |
| 2026-02-23 | Codex レビュー Round 5 NG 対応（§5.4 コメント Option→Vec 修正、§5.5 VR intro→VB（Voicebot）イントロ 修正、§5.6.1 共通ヘルパーコメントを「呼び出し側の既存フォールバックに委ねる」に修正） | Claude Sonnet 4.6 |
| 2026-02-23 | Codex レビュー Round 6 OK（§3.3 更新） | Claude Sonnet 4.6 |
| 2026-02-23 | 承認（§1 ステータス Approved・§3.4 承認情報記入） | @MasanoriSuda |
