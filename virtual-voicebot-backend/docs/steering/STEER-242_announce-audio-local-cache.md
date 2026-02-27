# STEER-242: アナウンス音声のローカルキャッシュ対応（フロントエンド非依存再生）

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-242 |
| タイトル | アナウンス音声のローカルキャッシュ対応（フロントエンド非依存再生） |
| ステータス | Approved |
| 関連Issue | #242 |
| 優先度 | P1 |
| 作成日 | 2026-02-24 |

---

## 2. ストーリー（Why）

### 2.1 背景

STEER-227（#227）でアナウンス音声の HTTP 取得（再生時フェッチ）を実装したが、
この実装は **再生時点でフロントエンドが稼働している** ことを前提としている。

**現状の問題:**

| 問題 | 詳細 |
|------|------|
| 再生時 HTTP 取得 | `resolve_audio_file_url()`（`coordinator.rs` L545）は `FRONTEND_BASE_URL` が設定されている場合、着信ごとに `fetch_audio_to_temp()` を呼び出してフロントエンドから音声を取得する |
| フロントエンド停止で再生不可 | フロントエンド PC をスリープ/停止するとリクエストが失敗し、`failed to fetch audio` となりアナウンスが再生されない |
| ラズパイ単体運用を阻害 | フロントエンドが別 PC の場合、その PC が常時起動していなければアナウンス再生ができない |

**ログ事象（Issue #242）:**

```
[session xxx] failed to fetch audio from http://<frontend-pc>:3000 err=...
```

**根本原因:**

- STEER-164（#164）でアナウンス同期が実装されたが、音声ファイル本体のダウンロードは未実装のまま。
- STEER-227（#227）は「再生時に毎回 HTTP 取得」という方式で別マシン問題を解消したが、フロントエンド依存は残存した。

### 2.2 目的

音声ファイルを **同期時（serversync）に Backend ローカルに保存** し、
再生時はローカルキャッシュを直接参照することで、
フロントエンドの稼働状態に依存しないアナウンス再生を実現する。

対象は AN/VM アナウンス音声だけでなく、IVR メニュー音声・VB（Voicebot）イントロ音声も含め、
`resolve_audio_file_url()` を共通関数として全音声に統一する（`fetch_audio_to_temp()` 全廃）。

### 2.3 ユーザーストーリー

```
As a システム管理者
I want to フロントエンド PC が停止していてもアナウンス再生したい
So that ラズパイ単体でアナウンスが再生でき、フロントエンド PC の起動状態に依存しない安定した運用ができる

受入条件:
- [ ] AC-1: フロントエンド PC が停止中でも、既存アナウンスが再生される（AN/VM mode）
- [ ] AC-2: serversync 実行時（フロントエンド稼働中）に音声ファイルが Backend ローカルに保存される
- [ ] AC-3: アナウンス更新時（Frontend の updatedAt 変化）に音声ファイルが再ダウンロードされる
- [ ] AC-4: アナウンス削除時にローカルの音声ファイルも削除される
- [ ] AC-5: 音声ダウンロード失敗時は WARN ログが出るが、DB メタデータ同期および他アナウンスの処理は継続される
- [ ] AC-6: ローカルキャッシュが存在しないアナウンスを再生しようとした場合、WARN ログが出てフォールバック音声が再生される
- [ ] AC-7: フロントエンド PC が停止中でも、IVR メニュー音声・VB（Voicebot）イントロ音声が再生される
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-24 |
| 起票理由 | フロントエンド PC を停止するとアナウンス再生が失敗する（Issue #242） |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Sonnet 4.6 |
| 作成日 | 2026-02-24 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "#242 のステアリング作成。Codex との会話で方針確認済み：同期時に音声ファイルをラズパイに保存し、再生時はローカル参照に統一する" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| 1 | Codex | 2026-02-24 | NG | 重大①`updated_at` 比較が現状実装で成立しない（DTO/保存方法の変更必要）、重大②トランザクション内でファイルI/O、重大③部分ファイル残存リスク、中①DL失敗スコープ曖昧、中②相対パス基準未定義、軽①TTS説明誤り |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | @MasanoriSuda |
| 承認日 | 2026-02-24 |
| 承認コメント | lgtm |

### 3.5 実装

| 項目 | 値 |
|------|-----|
| 実装者 | Codex (GPT-5) |
| 実装日 | 2026-02-24 |
| 指示者 | @MasanoriSuda |
| 指示内容 | 「STEER-242 の実装をお願いします Refs #242」 |
| コードレビュー | 未実施（PR/CodeRabbit 待ち） |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ実行者 | - |
| マージ日 | - |
| マージ先 | `coordinator.rs`、`handlers/mod.rs`、`converters.rs`（または新規 `audio_downloader.rs`）、`frontend_pull.rs`、`config/mod.rs`、`.env.example` |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| `.env.example`（ルート） | 修正 | `ANNOUNCEMENT_AUDIO_DIR` 追加。同一マシン構成での設定例および working directory 注意事項を記載 |

### 4.2 影響するコード

| ファイル | 変更種別 | 概要 |
|---------|---------|------|
| `src/interface/sync/converters.rs` | 修正 | `FrontendAnnouncement` DTO に `updated_at` フィールド追加。UPSERT で `updated_at` に Frontend 値を保存（`NOW()` から変更） |
| `src/interface/sync/frontend_pull.rs` | 修正 | `save_snapshot()` 前にファイルダウンロードフェーズを追加（トランザクション外）。削除対象ファイルの削除もトランザクション外に移動 |
| `src/interface/sync/audio_downloader.rs` | 新規 | 音声ファイルダウンロード・保存（tmp→rename）・削除ロジック（または `frontend_pull.rs` に内包） |
| `src/protocol/session/coordinator.rs` | 修正 | `resolve_audio_file_url()` をローカルキャッシュ参照に変更。`fetch_audio_to_temp()` 関数・`audio_tmp_files` フィールド・`map_audio_file_url_to_local_path()` 関数を削除 |
| `src/protocol/session/handlers/mod.rs` | 修正 | IVR メニュー音声（L998 付近）・VB（Voicebot）イントロ音声（L1278 付近）を `resolve_audio_file_url()` に統一（STEER-227 未適用箇所が残る場合のみ） |
| `src/shared/config/mod.rs` | 修正 | `AnnouncementConfig` に `audio_dir: String` を追加（`ANNOUNCEMENT_AUDIO_DIR` 環境変数） |

---

## 5. 差分仕様（What / How）

### 5.1 設計方針

| フェーズ | 変更前（STEER-227 後） | 変更後 |
|----------|----------------------|--------|
| 同期時（serversync） | メタデータのみ DB 保存。`updated_at` は `NOW()` | メタデータ保存（`updated_at` = Frontend `updatedAt`）＋ トランザクション外で音声ファイルダウンロード |
| 再生時（voicebot） | `FRONTEND_BASE_URL` + URL で HTTP GET → 一時ファイル | `ANNOUNCEMENT_AUDIO_DIR/{filename}` を直接参照（全音声種別で統一） |
| フロントエンド依存 | 再生時に必要 | 同期時のみ（フロントエンドが停止中でも再生可能） |

**ファイル名規則:**

`audio_file_url` の最終セグメントをそのままファイル名として使用する。

例: `/audio/announcements/dca2a704-23c6-45fe-a16a-2c059bab0e45.wav`
→ ローカルパス: `{ANNOUNCEMENT_AUDIO_DIR}/dca2a704-23c6-45fe-a16a-2c059bab0e45.wav`

DB スキーマの列追加は行わない。`audio_file_url` カラムは URL 文字列のまま保持し、再生時にファイル名を取り出してローカルパスに変換する。

### 5.2 `ANNOUNCEMENT_AUDIO_DIR` の config 追加

```rust
// src/shared/config/mod.rs — AnnouncementConfig に追加
#[derive(Clone, Debug)]
pub struct AnnouncementConfig {
    pub frontend_base_url: Option<String>,
    /// アナウンス音声ファイルのローカルキャッシュディレクトリ
    /// デフォルト: "data/announcements"
    /// 相対パスはバックエンドプロセスの working directory 基準。
    /// Raspberry Pi の systemd 運用では絶対パス（例: /home/pi/voicebot/data/announcements）を推奨。
    pub audio_dir: String,
}

impl AnnouncementConfig {
    fn from_env() -> Self {
        Self {
            frontend_base_url: env_non_empty("FRONTEND_BASE_URL"),
            audio_dir: env_non_empty("ANNOUNCEMENT_AUDIO_DIR")
                .unwrap_or_else(|| "data/announcements".to_string()),
        }
    }
}
```

> **注:** `ANNOUNCEMENT_AUDIO_DIR` の相対パスはバックエンドプロセスの `cwd` 基準。
> systemd で `WorkingDirectory` を設定する場合は、その値を基準として解決される。
> ズレを防ぐため、本番環境では絶対パスを推奨する（`.env.example` に明記）。

### 5.3 `FrontendAnnouncement` DTO と `updated_at` 保存の変更

現行 `FrontendAnnouncement` 構造体（`converters.rs` L95）には `updated_at` フィールドがなく、
DB の `updated_at` は `NOW()` で保存されている。更新検知を正しく行うために両方を変更する。

#### 5.3.1 DTO への `updated_at` 追加

```rust
// src/interface/sync/converters.rs — FrontendAnnouncement に追加
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendAnnouncement {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    #[serde(default = "default_announcement_type")]
    pub announcement_type: String,
    #[serde(default = "default_true")]
    pub is_active: bool,
    pub audio_file_url: Option<String>,
    pub tts_text: Option<String>,
    pub duration_sec: Option<f64>,
    // 追加: Frontend の updatedAt を取得する
    // Option にすることで既存テストが壊れない（フィールドなしは None として扱う）
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}
```

#### 5.3.2 DB UPSERT での `updated_at` 変更

`convert_announcements()` の INSERT/UPSERT（`converters.rs` L365 付近）で、
`updated_at` を `NOW()` ではなく Frontend の値を使うよう変更する。

```sql
-- 変更前（NOW() 使用）:
INSERT INTO announcements (..., updated_at)
VALUES (..., NOW())
ON CONFLICT (id) DO UPDATE SET ..., updated_at = NOW();

-- 変更後（Frontend の updatedAt を使用）:
INSERT INTO announcements (..., updated_at)
VALUES (..., $updated_at)                            -- Frontend の updatedAt（None の場合は NOW()）
ON CONFLICT (id) DO UPDATE SET ..., updated_at = EXCLUDED.updated_at;
```

> `updated_at` が `None`（古い Frontend 実装等）の場合は `chrono::Utc::now()` を fallback として使用する。

### 5.4 同期時の音声ダウンロード（`frontend_pull.rs` 拡張）

#### 5.4.1 処理フロー（トランザクション外/内の分離）[重大②対応]

```
serversync 実行フロー（変更後）:

【Phase 1: DB トランザクション外（ファイル I/O フェーズ）】

1. Frontend から announcements メタデータ（Vec<FrontendAnnouncement>）を取得
2. 現在の DB から（id → updated_at, audio_file_url）をクエリ（SELECT のみ）
3. 削除対象アナウンス（Frontend に存在しない ID）のローカルファイルを削除
   - ファイルが存在しない場合は無視（エラーにしない）
4. 各アナウンスについてダウンロード要否を判定（以下のいずれかを満たす場合）:
   - ローカルファイルが存在しない
   - Frontend の `updated_at` が Some であり、かつ DB の `updated_at` と異なる
   - Frontend の `updated_at` が None の場合は `updated_at` 比較を行わず、ファイル不存在のみで判定する
     （古い Frontend 実装との互換性確保のため。毎回再 DL は行わない）
5. ダウンロード対象ファイルを tmp ファイルに保存 → 成功後に rename（原子的置換）
   - DL失敗: WARN ログ＋当該ファイルのみスキップ（後続アナウンス・Phase 2 は継続）
   - DL失敗時もメタデータ DB 同期は Phase 2 で実施する（音声キャッシュ更新のみスキップ）[中①確定]

【Phase 2: DB トランザクション内（メタデータ同期フェーズ）】

6. apply_frontend_snapshot() を実行（既存処理と同様）
   - convert_announcements(): updated_at = Frontend の updatedAt を保存（§5.3 参照）
   - number-groups, call-actions, ivr-flows: 現状と同じ
```

> **不変条件:** Phase 2（DB トランザクション内）でファイル I/O は行わない。
> ファイル状態と DB 状態の完全な一致は保証しない（ベストエフォート）。
> ファイルのみ更新後に DB が失敗した場合、次回 serversync で `updated_at` 比較により再 DL が行われる。

#### 5.4.2 `audio_file_url = None` のスキップ条件[軽①対応]

ダウンロードフェーズで `audio_file_url` が `None` のアナウンスはスキップする。
これは音声未設定アナウンス（TTS・Upload を問わず、`audioFileUrl` が返らない状態）を表す。
TTS 作成時でも Frontend は `audioFileUrl` を生成・保存するため、
TTS アナウンスが `audio_file_url = None` になることは通常ない。

#### 5.4.3 ダウンロード関数（原子的書き込み）[重大③対応]

```rust
// src/interface/sync/audio_downloader.rs（新規）または frontend_pull.rs 内
async fn download_audio_file(
    http_client: &reqwest::Client,
    frontend_base_url: &str,
    url_path: &str,        // 例: "/audio/announcements/xxx.wav"（バリデーション済み）
    local_path: &std::path::Path,
) -> anyhow::Result<()> {
    const MAX_BYTES: u64 = 8 * 1024 * 1024;  // STEER-227 と統一

    let full_url = format!("{}{}", frontend_base_url.trim_end_matches('/'), url_path);
    let mut resp = http_client.get(&full_url).send().await?.error_for_status()?;

    // 親ディレクトリ作成
    let parent = local_path.parent().expect("local_path has parent");
    std::fs::create_dir_all(parent)?;

    // 原子的書き込み: {filename}.tmp に書いてから rename[重大③]
    let tmp_path = local_path.with_extension("tmp");
    let result: anyhow::Result<()> = async {
        let mut file = std::fs::File::create(&tmp_path)?;
        let mut total = 0u64;
        while let Some(chunk) = resp.chunk().await? {
            total = total.checked_add(chunk.len() as u64)
                .ok_or_else(|| anyhow::anyhow!("size overflow"))?;
            if total > MAX_BYTES {
                return Err(anyhow::anyhow!("audio too large: {} bytes", total));
            }
            std::io::Write::write_all(&mut file, &chunk)?;
        }
        Ok(())
    }.await;

    // エラー（MAX_BYTES 超過・通信切断・timeout 等）はすべて tmp を削除[重大③]
    if let Err(err) = result {
        std::fs::remove_file(&tmp_path).ok();  // 削除失敗は無視
        return Err(err);
    }

    // 成功時のみ rename（原子的置換）
    std::fs::rename(&tmp_path, local_path)?;
    Ok(())
}
```

- `FRONTEND_BASE_URL` が未設定の場合は `download_audio_file()` を呼び出さない（Phase 1 全体をスキップ）。
- タイムアウトは `http_client` の設定で管理（serversync の既存タイムアウト設定を踏襲）。

### 5.5 再生時の `resolve_audio_file_url()` 変更（AN/VM 音声）

```rust
// src/protocol/session/coordinator.rs
// シグネチャは既存の &mut self のまま（STEER-227 適用済み前提）
async fn resolve_audio_file_url(&mut self, audio_file_url: String) -> Option<String> {
    let cfg = config::announcement_config();
    let url_path = extract_url_path(&audio_file_url);

    if !is_safe_announcement_url_path(&url_path) {
        log::warn!(
            "[session {}] rejected unsafe audio url path: {}",
            self.call_id, url_path
        );
        return None;
    }

    // url_path の最終セグメントからローカルパスを構築
    let filename = url_path.rsplit('/').next()?;
    let local_path = std::path::Path::new(&cfg.audio_dir).join(filename);

    if local_path.exists() {
        return Some(local_path.to_string_lossy().to_string());
    }

    // ローカルキャッシュが存在しない場合（sync 未実施または失敗）
    log::warn!(
        "[session {}] local audio cache not found: {:?} (has serversync run?)",
        self.call_id, local_path
    );
    None
    // → 呼び出し側の既存フォールバックに委ねる（AN/VM → ANNOUNCEMENT_FALLBACK_WAV_PATH 等）
}
```

**削除対象コード（coordinator.rs）:**
- `fetch_audio_to_temp()` 関数
- `SessionCoordinator.audio_tmp_files: Vec<NamedTempFile>` フィールド
- `map_audio_file_url_to_local_path()` 関数（後方互換フォールバック不要のため[Q3確定]）

**変更点の概要:**

| 項目 | 変更前（STEER-227） | 変更後 |
|------|-------------------|--------|
| `FRONTEND_BASE_URL` あり | HTTP GET → 一時ファイル | ローカルキャッシュ参照 |
| `FRONTEND_BASE_URL` なし | `map_audio_file_url_to_local_path()`（従来パス） | ローカルキャッシュ参照（後方互換フォールバックなし） |
| 失敗時 | WARN + None | WARN + None（同等） |
| 一時ファイル管理 | `audio_tmp_files: Vec<NamedTempFile>` | 不要（フィールド削除） |

### 5.6 IVR メニュー音声・VB（Voicebot）イントロ音声の変更（handlers/mod.rs）[Q4確定]

`resolve_audio_file_url()` の内部実装変更のみで AN/VM 以外の音声も統一される。

**STEER-227 適用済みの場合:** `handlers/mod.rs` のコード変更は不要。
呼び出し先の `resolve_audio_file_url()` がローカルキャッシュを参照するよう変わるため、
IVR/VB のフォールバック先（`IVR_INTRO_WAV_PATH` / `VOICEBOT_INTRO_WAV_PATH`）も引き続き機能する。

**STEER-227 未適用箇所が残る場合:** `map_audio_file_url_to_local_path()` の直接呼び出しを
`self.resolve_audio_file_url(url).await` に差し替えること（STEER-227 §5.5/5.6 参照）。

### 5.7 `FRONTEND_BASE_URL` 未設定時の挙動

| 状況 | 変更後 |
|------|--------|
| serversync 実行・FRONTEND_BASE_URL 未設定 | Phase 1（ファイル DL）をスキップ。メタデータのみ DB 同期 |
| 再生時・ローカルファイルあり | 直接再生 |
| 再生時・ローカルファイルなし | WARN ログ + フォールバック音声 |

**同一マシン構成での対応方法（`.env.example` に記載）:**

`map_audio_file_url_to_local_path()` は削除されるため、既存ファイルを参照したい場合は
`ANNOUNCEMENT_AUDIO_DIR` を明示的に設定する。

```dotenv
# 同一マシン構成（音声ファイルが frontend の public/ にある場合）の例
# 注意: 相対パスはバックエンドプロセスの working directory 基準
# 本番環境（systemd 等）では絶対パスを推奨
ANNOUNCEMENT_AUDIO_DIR=/home/pi/voicebot/virtual-voicebot-frontend/public/audio/announcements
```

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #242 | STEER-242 | 起票 |
| STEER-164 §5.4 | STEER-242 | STEER-164 で未実装だった音声ファイル転送を本ステアリングで実現 |
| STEER-227 | STEER-242 | STEER-227 の再生時 HTTP フェッチをローカルキャッシュに置き換え |
| STEER-242 §5.3 | `src/interface/sync/converters.rs` | FrontendAnnouncement DTO・UPSERT 変更 |
| STEER-242 §5.4 | `src/interface/sync/frontend_pull.rs` | トランザクション外ファイル DL フェーズ追加 |
| STEER-242 §5.5 | `src/protocol/session/coordinator.rs` | 再生時ローカル参照に変更・不要コード削除 |
| STEER-242 §5.6 | `src/protocol/session/handlers/mod.rs` | IVR/VB イントロ音声を `resolve_audio_file_url()` に統一（未適用箇所のみ） |
| STEER-242 §5.2 | `src/shared/config/mod.rs` | `ANNOUNCEMENT_AUDIO_DIR` 追加 |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [ ] `ANNOUNCEMENT_AUDIO_DIR` のデフォルト値（`data/announcements`）・相対パス基準の説明に合意しているか
- [ ] DB スキーマ変更なし・`announcements.updated_at` に Frontend `updatedAt` を保存する方針に合意しているか
- [ ] ファイル DL フェーズ（Phase 1）を DB トランザクション外に置く構造に合意しているか
- [ ] tmp→rename の原子的書き込みパターンで部分ファイルリスクが解消されているか確認できているか
- [ ] DL 失敗時「メタデータ同期継続・キャッシュ更新のみスキップ」に合意しているか
- [ ] `map_audio_file_url_to_local_path()` 削除による既存環境への影響が `.env.example` で説明されているか
- [ ] `fetch_audio_to_temp()` / `audio_tmp_files` の削除スコープが本 Issue に含まれることに合意しているか

### 7.2 マージ前チェック（Approved → Merged）

- [ ] serversync 実行後、`ANNOUNCEMENT_AUDIO_DIR` に音声ファイルが保存されている
- [ ] フロントエンド PC 停止中でも AN アナウンスが再生される（AC-1）
- [ ] フロントエンド PC 停止中でも IVR メニュー音声・VB イントロ音声が再生される（AC-7）
- [ ] アナウンス更新後の次回 serversync で音声ファイルが再ダウンロードされる（AC-3・updated_at 変化）
- [ ] ローカルファイル削除後の次回 serversync で音声ファイルが補完ダウンロードされる（AC-3・ファイル不存在）
- [ ] アナウンス削除後の次回 serversync でローカルファイルが削除される（AC-4）
- [ ] ダウンロード途中でプロセスが停止した場合に `*.tmp` ファイルが残存するのみで、`.wav` が壊れない
- [ ] DL 失敗時に WARN ログが出て DB メタデータ同期が継続されることを確認している（AC-5）
- [ ] ローカルキャッシュ未存在時に WARN ログが出てフォールバック再生される（AC-6）
- [ ] `audio_tmp_files` フィールド削除後にコンパイルエラーが出ないことを確認している

---

## 8. 未確定点・質問（全確定）

| # | 質問 | 推奨 | オーナー回答 |
|---|------|------|-------------|
| Q1 | 音声ダウンロード失敗時の同期継続方針は？ | A: WARN ログのみで継続 | **A（はい）** |
| Q2 | ダウンロード要否判定を「ファイル不存在 OR updated_at 変化」の両方にするか？ | A: 両方 | **A（両方）** |
| Q3 | `map_audio_file_url_to_local_path()` へのフォールバックを残さないか？ | A: 残さない | **A（不要）** |
| Q4 | IVR/VB イントロ音声も `resolve_audio_file_url()` に統一するか？ | B: 全廃で一貫性を保つ | **B（横展開する）** |
| Q5（レビュー追加）| `announcements.updated_at` を Frontend `updatedAt` に合わせるか？ | Frontend 値を保存 | **はい（NOW() 運用を変更）** |
| Q6（レビュー追加）| DL 失敗時は「DB メタデータ同期・キャッシュのみスキップ」で確定か？ | DB は同期 | **はい（確定）** |
| Q7（レビュー追加）| serversync のファイル I/O は DB トランザクション外に出すか？ | トランザクション外 | **はい（トランザクション外でDL→トランザクション内でDB反映）** |

---

## 9. リスク・ロールバック観点

| リスク | 影響 | 緩和策 |
|--------|------|--------|
| serversync が一度も成功していない初期状態でアナウンス再生 | ローカルファイル未存在 → WARN + フォールバック音声 | `.env.example` に serversync 先行実行の注意を記載 |
| `ANNOUNCEMENT_AUDIO_DIR` のディスク容量不足 | ダウンロード失敗 → WARN ログ。tmp ファイルは削除される。既存ファイルは保持 | 運用ドキュメントでディスク容量の目安を案内 |
| `map_audio_file_url_to_local_path()` 削除による同一マシン構成の変更 | `ANNOUNCEMENT_AUDIO_DIR` を設定しない場合、音声が見つからない | `.env.example` に移行案（絶対パス設定例）を明記 |
| Phase 1 後・Phase 2 前にプロセスが停止（更新ケース） | ファイルのみ更新、DB は古い `updated_at` のまま残る。次回 serversync で `updated_at` 比較により再 DL されるため最終的に整合 | 許容（ベストエフォート） |
| Phase 1 でファイル削除後・Phase 2 が失敗（削除ケース） | DB に行が残るがローカルファイルは消えた状態。再生時は「ローカルキャッシュなし → WARN + フォールバック音声」。次回 serversync の Phase 2 成功時に DB 削除が追随して整合 | 許容（ベストエフォート）。Phase 2 失敗は DB 例外としてログ出力される |
| `*.tmp` ファイルの残存（プロセス強制終了等） | 次回 DL 時に上書きされる（`File::create` で再作成）。再生には使われない（`.wav` 拡張子でない） | 許容。必要なら起動時に `*.tmp` の清掃を行う（別 Issue） |
| `fetch_audio_to_temp()` / `audio_tmp_files` 削除（STEER-227 実装の全面変更） | 変更範囲が広い | STEER-227 の変更ファイルと重複するため差分を慎重に確認 |

**ロールバック手順:** PR を revert。`ANNOUNCEMENT_AUDIO_DIR` のローカル音声ファイルは手動削除（または放置可。再 DL 時に上書き）。
変更ファイルは `converters.rs` / `frontend_pull.rs`（または `audio_downloader.rs`）/ `coordinator.rs` / `handlers/mod.rs` / `config/mod.rs` / `.env.example`。

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-24 | 承認（§1 ステータス Approved・§3.4 承認情報記入） | @MasanoriSuda |
| 2026-02-24 | 初版作成（Draft） | Claude Sonnet 4.6 |
| 2026-02-24 | Q1〜Q4 オーナー回答記入。§2.2/2.3 IVR/VB スコープ明記（Q4）、§3.6 handlers/mod.rs 追加、§4.2 削除コード明記・handlers/mod.rs 追加、§5.4 map_audio_file_url_to_local_path 削除明記（Q3）、§5.5 IVR/VB 変更仕様追加（Q4）、§5.6 FRONTEND_BASE_URL 未設定時の挙動と .env.example 例を更新（Q3）、§7.1/7.2 チェック項目更新、§9 リスク更新 | Claude Sonnet 4.6 |
| 2026-02-24 | Codex レビュー Round 2 OK 対応（軽①: §5.4.1 updated_at=None 時は比較スキップ・ファイル不存在のみ判定と明記、軽②: §9 に削除先行→Phase 2 失敗の一時不整合リスク追記） | Claude Sonnet 4.6 |
| 2026-02-24 | Codex レビュー Round 1 NG 対応（Q5/Q6/Q7 オーナー確認後反映）: §3.3 レビュー記入、§3.6 frontend_pull.rs 追加、§4.2 frontend_pull.rs・DTO 変更行追加、§5.3 FrontendAnnouncement DTO・updated_at 保存変更追加（重大①）、§5.4 Phase 1/2 分離（トランザクション外ファイルI/O、重大②）・audio_file_url=None 説明修正（軽①）・tmp→rename 原子的書き込み追加（重大③）・DL失敗時スコープ明確化（中①）、§5.2 相対パス基準明記（中②）、§5.7 .env.example 絶対パス例に更新、§7.1/7.2 チェック項目追加、§8 Q5/Q6/Q7 追加、§9 Phase 間停止リスク・tmp 残存リスク追加 | Claude Sonnet 4.6 |
