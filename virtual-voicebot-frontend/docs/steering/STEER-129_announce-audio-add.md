# STEER-129: アナウンスタブ音声追加機能

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-129 |
| タイトル | アナウンスタブ音声追加機能（WAV アップロード + VoiceVox TTS） |
| ステータス | Approved |
| 関連Issue | #129（親: #128） |
| 優先度 | P1 |
| 作成日 | 2026-02-08 |

---

## 2. ストーリー（Why）

### 2.1 背景

- アナウンスタブ（`/announcements`）は UI のハコ（モックデータ）のみ存在し、実際にアナウンス音声を追加・管理する機能が未実装
- ユーザーは独自の WAV ファイルをアップロードしたり、テキストから VoiceVox で音声合成したい
- 現状ではアナウンスの追加・削除・有効/無効切替のすべてが動作しない

### 2.2 目的

- アナウンスタブのモック UI を実動作化し、WAV アップロードと VoiceVox TTS 生成の 2 機能を使えるようにする
- 追加されたアナウンスに対して最低限の管理操作（削除・有効/無効切替・名称変更）を提供する

### 2.3 ユーザーストーリー

```
US-1: WAV ファイルアップロード
As a 管理者
I want to 自分で作成した WAV ファイルをアナウンスタブにアップロードしたい
So that IVR や保留音として使用できる

受入条件:
- [ ] AC-1: 「音声ファイルをアップロード」から WAV を選択し、アナウンスとして保存できる
- [ ] AC-2: アップロード後、ツリーに即時反映され再生プレビューできる
- [ ] AC-3: WAV 以外のファイルはバリデーションエラーになる
- [ ] AC-4: 10MB を超えるファイルはバリデーションエラーになる

US-2: VoiceVox TTS 生成
As a 管理者
I want to テキストを入力し VoiceVox でキャラクター音声を生成したい
So that 手軽にアナウンス音声を作成できる

受入条件:
- [ ] AC-5: 「テキスト読み上げ」からテキスト入力 + キャラ選択で音声を生成できる
- [ ] AC-6: VoiceVox /speakers API からキャラクター一覧を動的取得しドロップダウンで選択できる
- [ ] AC-7: 生成後、ツリーに即時反映され再生プレビューできる
- [ ] AC-8: VoiceVox 未起動時はエラーメッセージを表示する

US-3: アナウンス管理操作
As a 管理者
I want to 追加したアナウンスを削除・有効/無効切替・名称変更したい
So that アナウンスを整理できる

受入条件:
- [ ] AC-9: アナウンスを削除でき、音声ファイルも同時に削除される
- [ ] AC-10: 有効/無効切替が動作し、状態が永続化される
- [ ] AC-11: 名称変更が動作し、変更が永続化される
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-07 |
| 起票理由 | アナウンスタブの実動作化（モック → 実機能） |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Code (claude-opus-4-6) |
| 作成日 | 2026-02-08 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "Issue #129 の仕様を壁打ちして、ステアリングを起こす" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| 1 | Codex | 2026-02-08 | OK | PoC として問題なし |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | @MasanoriSuda |
| 承認日 | 2026-02-08 |
| 承認コメント | PoC として OK |

### 3.5 実装

| 項目 | 値 |
|------|-----|
| 実装者 | Codex |
| 実装日 | 2026-02-08 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "STEER-129 に基づきアナウンスタブ音声追加機能を実装" |
| コードレビュー | PoC（2026-02-08） |

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
| なし（PoC フェーズ） | - | 本体仕様書へのマージは #130 以降で検討 |

### 4.2 影響するコード

| モジュール | 変更種別 | 概要 |
|-----------|---------|------|
| `components/announcements-content.tsx` | 修正 | モックデータ → JSON ストア読み込み、追加/削除/切替/名称変更の実動作化 |
| `lib/db/announcements.ts` | 新規 | アナウンス JSON ストア（CRUD 操作） |
| `app/api/announcements/route.ts` | 新規 | GET（一覧）/ POST（作成）/ PATCH（更新）/ DELETE（削除） |
| `app/api/announcements/upload/route.ts` | 新規 | WAV ファイルアップロード API |
| `app/api/announcements/tts/route.ts` | 新規 | VoiceVox TTS 生成 API |
| `app/api/announcements/speakers/route.ts` | 新規 | VoiceVox キャラクター一覧取得 API |
| `public/audio/announcements/` | 新規 | 音声ファイル保存ディレクトリ（.gitignore 対象） |

---

## 5. 差分仕様（What / How）

### 5.1 データモデル

#### 5.1.1 JSON ストア構造

ファイルパス: `storage/db/announcements.json`

既存の `sync.json` と同じパターン（ファイル読み書き + 書き込みロック）を踏襲する。

```typescript
interface AnnouncementsDatabase {
  announcements: Record<string, StoredAnnouncement>
  folders: Record<string, StoredFolder>
  updatedAt: string  // ISO8601
}
```

#### 5.1.2 StoredAnnouncement

```typescript
interface StoredAnnouncement {
  id: string           // UUID v4
  name: string
  description: string | null
  announcementType: AnnouncementType  // 既存型を利用
  isActive: boolean
  folderId: string | null  // 所属フォルダ ID（null = ルート直下）
  audioFileUrl: string | null  // "/audio/announcements/{uuid}.wav"
  ttsText: string | null       // TTS 生成元テキスト（TTS の場合のみ）
  speakerId: number | null     // VoiceVox speaker_id（TTS の場合のみ）
  speakerName: string | null   // キャラクター名（表示用）
  durationSec: number | null   // 秒数（WAV ヘッダから自動計算）
  language: string             // デフォルト "ja"
  source: "upload" | "tts"     // 音声の出自
  createdAt: string  // ISO8601
  updatedAt: string  // ISO8601
}
```

**既存 `Announcement` 型（lib/types.ts）との差分:**
- `speakerId`, `speakerName`, `source` を追加（FE 独自、TTS 管理用）
- `version` は JSON ストアでは不要（楽観ロック不要）のため省略

#### 5.1.3 StoredFolder

モック UI に現在ハードコードされているフォルダ構造を永続化する。
今回フォルダ CRUD は対象外だが、初期データとして既存モックのフォルダを JSON に書き出す。

```typescript
interface StoredFolder {
  id: string
  name: string
  description: string | null
  parentId: string | null
  sortOrder: number
  createdAt: string
  updatedAt: string
}
```

### 5.2 API Routes

#### 5.2.1 GET /api/announcements

アナウンスとフォルダの一覧を返す。

**レスポンス:**
```typescript
{
  ok: true,
  announcements: StoredAnnouncement[],
  folders: StoredFolder[]
}
```

#### 5.2.2 POST /api/announcements/upload

WAV ファイルをアップロードしてアナウンスを作成する。

**リクエスト:** `multipart/form-data`

| フィールド | 型 | 必須 | 説明 |
|-----------|-----|------|------|
| file | File | Yes | WAV ファイル |
| name | string | Yes | アナウンス名 |
| announcementType | AnnouncementType | No | デフォルト `custom` |
| folderId | string | No | 所属フォルダ ID（省略時ルート直下） |

**バリデーション:**
- ファイル拡張子: `.wav` のみ
- ファイルサイズ: 最大 10MB
- Content-Type: `audio/wav` or `audio/x-wav`

**処理フロー:**
1. バリデーション
2. UUID v4 生成
3. `public/audio/announcements/{uuid}.wav` に保存
4. WAV ヘッダから duration を算出
5. `announcements.json` に StoredAnnouncement を追加
6. レスポンス返却

**レスポンス:**
```typescript
{ ok: true, announcement: StoredAnnouncement }
```

**エラー:**
```typescript
{ ok: false, error: string }  // 400 or 500
```

#### 5.2.3 POST /api/announcements/tts

VoiceVox でテキストから WAV を生成してアナウンスを作成する。

**リクエスト:** `application/json`

```typescript
{
  text: string         // 読み上げテキスト（必須、1〜1000文字）
  speakerId: number    // VoiceVox speaker_id（必須）
  name: string         // アナウンス名（必須）
  announcementType?: AnnouncementType  // デフォルト "custom"
  folderId?: string    // 所属フォルダ ID
}
```

**処理フロー:**
1. バリデーション
2. VoiceVox `/audio_query` に text + speaker_id を POST → 合成パラメータ取得
3. VoiceVox `/synthesis` に合成パラメータ + speaker_id を POST → WAV バイナリ取得
4. UUID v4 生成
5. `public/audio/announcements/{uuid}.wav` に保存
6. WAV ヘッダから duration を算出
7. `announcements.json` に StoredAnnouncement を追加（`source: "tts"`, `ttsText`, `speakerId`, `speakerName` を保存）
8. レスポンス返却

**レスポンス:**
```typescript
{ ok: true, announcement: StoredAnnouncement }
```

**エラー:**
```typescript
// VoiceVox 未起動時
{ ok: false, error: "VoiceVox に接続できません（localhost:50021）" }  // 502

// テキスト空
{ ok: false, error: "text is required" }  // 400
```

#### 5.2.4 GET /api/announcements/speakers

VoiceVox のキャラクター一覧を取得する。

**処理フロー:**
1. VoiceVox `GET /speakers` を呼び出し
2. レスポンスを整形して返却

**レスポンス:**
```typescript
{
  ok: true,
  speakers: Array<{
    name: string           // キャラクター名（例: "ずんだもん"）
    styles: Array<{
      id: number           // speaker_id
      name: string         // スタイル名（例: "ノーマル", "あまあま"）
    }>
  }>
}
```

**エラー（VoiceVox 未起動時）:**
```typescript
{ ok: false, error: "VoiceVox に接続できません" }  // 502
```

#### 5.2.5 PATCH /api/announcements/[id]

アナウンスの更新（名称変更・有効/無効切替）。

**リクエスト:** `application/json`

```typescript
{
  name?: string       // 名称変更
  isActive?: boolean  // 有効/無効切替
}
```

**レスポンス:**
```typescript
{ ok: true, announcement: StoredAnnouncement }
```

#### 5.2.6 DELETE /api/announcements/[id]

アナウンスの削除。音声ファイルも同時に削除する。

**処理フロー:**
1. `announcements.json` から該当レコード取得
2. `audioFileUrl` に対応するファイルを `public/` 配下から削除
3. JSON からレコード削除
4. レスポンス返却

**レスポンス:**
```typescript
{ ok: true }
```

### 5.3 UI 変更

#### 5.3.1 データソース切り替え

- `announcements-content.tsx` のモックデータ（`mockAnnouncementTree`）を削除
- `GET /api/announcements` から取得したデータでツリーを構築
- フォルダは JSON ストアの初期データ（現モックのフォルダ構造をシードとして保存）を使用

#### 5.3.2 「音声ファイルをアップロード」ダイアログ

既存の「追加」ドロップダウンメニューの「音声ファイルをアップロード」をクリックした時に表示。

| フィールド | 型 | 必須 | 説明 |
|-----------|-----|------|------|
| ファイル | File input (.wav) | Yes | ドラッグ＆ドロップまたはファイル選択 |
| アナウンス名 | text input | Yes | デフォルト: ファイル名（拡張子なし） |
| タイプ | select | No | AnnouncementType、デフォルト `custom` |

- 所属フォルダ: 現在選択中のフォルダ（未選択ならルート直下）
- 保存ボタン押下 → `POST /api/announcements/upload`
- 成功後: ツリーを再取得して反映

#### 5.3.3 「テキスト読み上げ」ダイアログ

既存の「追加」ドロップダウンメニューの「テキスト読み上げ」をクリックした時に表示。

| フィールド | 型 | 必須 | 説明 |
|-----------|-----|------|------|
| テキスト | textarea | Yes | 読み上げテキスト（最大 1000 文字） |
| キャラクター | select | Yes | `/api/announcements/speakers` から動的取得 |
| アナウンス名 | text input | Yes | デフォルト: テキスト先頭 20 文字 |
| タイプ | select | No | AnnouncementType、デフォルト `custom` |

- キャラクター選択: `speakers` のネスト構造を「キャラ名 - スタイル名」の形でフラットに表示
- 生成ボタン押下 → `POST /api/announcements/tts`
- 生成中はローディング表示（VoiceVox の合成に数秒かかる）
- 成功後: ツリーを再取得して反映

#### 5.3.4 アナウンス管理操作

| 操作 | トリガー | API |
|------|---------|-----|
| 削除 | ドロップダウン/コンテキストメニューの「削除」 | `DELETE /api/announcements/[id]` |
| 有効/無効切替 | Switch コンポーネント | `PATCH /api/announcements/[id]` |
| 名称変更 | ドロップダウン/コンテキストメニューの「編集」→ インライン編集 or ダイアログ | `PATCH /api/announcements/[id]` |

- 削除時は確認ダイアログを表示（「この操作は取り消せません」）

#### 5.3.5 AudioPreview 実動作化

現在のモック波形表示を実際の音声再生に切り替える。

- `audioFileUrl` が存在する場合、`<audio>` 要素で再生
- Play/Pause ボタンで再生制御
- duration 表示は `StoredAnnouncement.durationSec` から取得

### 5.4 ファイルストレージ

#### 5.4.1 保存パス

```
virtual-voicebot-frontend/
  public/
    audio/
      announcements/
        {uuid}.wav       ← アップロード or TTS 生成ファイル
```

#### 5.4.2 .gitignore

`virtual-voicebot-frontend/.gitignore` に以下を追加:

```
public/audio/announcements/
```

#### 5.4.3 将来の移行パス

現在は `public/` 配下で静的配信するが、将来（#130 以降）は以下に移行可能:
- `public/` 外に保存
- Next.js API Route 経由で配信（認証付き）
- S3 等のオブジェクトストレージ

`audioFileUrl` を相対パスで保存しているため、配信方式変更時は URL 生成ロジックのみ変更すればよい。

### 5.5 WAV ヘッダ解析（duration 算出）

WAV ファイルのヘッダから duration を算出する。

```
duration_sec = data_chunk_size / (sample_rate * channels * bits_per_sample / 8)
```

- data チャンクサイズ、サンプルレート、チャンネル数、ビット深度を WAV ヘッダ（先頭 44 バイト）から読み取る
- ヘッダ解析に失敗した場合は `durationSec: null` とする（エラーにはしない）

### 5.6 初期データ（シード）

JSON ストアが存在しない場合、初回アクセス時に以下のフォルダ構造を自動生成する。

```typescript
const SEED_FOLDERS: StoredFolder[] = [
  { id: "folder-greetings",  name: "挨拶メッセージ",  description: "着信時の挨拶音声",     parentId: null, sortOrder: 1 },
  { id: "folder-hold",       name: "保留音",          description: "保留中の音声",         parentId: null, sortOrder: 2 },
  { id: "folder-ivr",        name: "IVRメニュー",     description: "自動音声応答メニュー",  parentId: null, sortOrder: 3 },
  { id: "folder-closed",     name: "時間外案内",      description: "営業時間外のアナウンス", parentId: null, sortOrder: 4 },
]
```

- アナウンス（モックの `a1`〜`a9`）はシードに含めない（空フォルダから開始）
- フォルダ構造のみ初期化し、ユーザーが自由にアナウンスを追加する運用

---

## 6. スコープ外（今回対応しない）

| 項目 | 理由 | 対応時期 |
|------|------|---------|
| フォルダ CRUD（新規/削除/移動） | PoC スコープ外 | 後続イシュー |
| 「録音する」（ブラウザ録音） | PoC スコープ外 | 後続イシュー |
| 既存モックデータの複製機能 | PoC スコープ外 | 後続イシュー |
| Backend（Rust）改修 | FE 単独で完結 | #130 以降 |
| Backend 同期（Serversync） | FE 単独で完結 | #130 以降 |
| S3 アップロード | 当面ローカル保存 | 後続イシュー |
| MP3 サポート | WAV のみで十分 | 後続イシュー |
| 生成前プレビュー（試聴） | 生成後再生で十分 | 後続イシュー |

---

## 7. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #128 | Issue #129 | 親子 |
| Issue #129 | STEER-129 | 起票 |
| STEER-129 AC-1〜AC-4 | §5.2.2 upload API | WAV アップロード |
| STEER-129 AC-5〜AC-8 | §5.2.3 tts API, §5.2.4 speakers API | TTS 生成 |
| STEER-129 AC-9〜AC-11 | §5.2.5 PATCH, §5.2.6 DELETE | 管理操作 |

---

## 8. レビューチェックリスト

### 8.1 仕様レビュー（Review → Approved）

- [ ] WAV アップロードの入出力仕様が明確か
- [ ] VoiceVox TTS 生成の入出力仕様が明確か
- [ ] JSON ストアのデータ構造が既存パターン（sync.json）と整合するか
- [ ] 既存 UI（ツリー構造）との統合方針が明確か
- [ ] スコープ外の項目が明確か

### 8.2 マージ前チェック（Approved → Merged）

- [ ] AC-1〜AC-11 がすべて PASS
- [ ] VoiceVox 未起動時のエラーハンドリングが動作する
- [ ] .gitignore が設定されている
- [ ] コードレビュー済み

---

## 9. 備考

### 9.1 VoiceVox の前提条件

- VoiceVox Engine が `localhost:50021` で起動していること
- Docker での起動例: `docker run --rm -p 50021:50021 voicevox/voicevox_engine`
- 未起動時は TTS 生成・キャラ一覧取得が 502 エラーになるが、WAV アップロード機能には影響しない

### 9.2 既存型との関係

- `lib/types.ts` に `Announcement` 型（Canonical）と `LegacyAnnouncement` 型が存在
- 今回は `StoredAnnouncement`（`lib/db/announcements.ts`）を新設し、JSON ストア専用とする
- UI 層では `LegacyAnnouncement` / `LegacyAnnouncementFolder` をそのまま使い、API レスポンスから変換する
- 将来 BE 同期時に Canonical `Announcement` 型に統一する

### 9.3 実装は Codex 担当

本仕様に基づくコード実装は Codex へ引き継いでください。

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-08 | 初版作成 | Claude Code (claude-opus-4-6) |
| 2026-02-08 | Approved — Codex レビュー OK、実装完了 | Claude Code (claude-opus-4-6) |
