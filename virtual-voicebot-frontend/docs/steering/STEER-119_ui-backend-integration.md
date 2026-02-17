# STEER-119: Frontend UI と Backend の連携実装

---

## 1. メタ情報

| 項目 | 値 |
|------|-----|
| ID | STEER-119 |
| タイトル | Frontend UI と Backend の連携実装 |
| ステータス | Approved |
| 関連Issue | #119 |
| 優先度 | P0 |
| 作成日 | 2026-02-07 |

---

## 2. ストーリー（Why）

### 2.1 背景

- Frontend UI コンポーネント（Dashboard, 通話履歴, 通話詳細）は実装済み（~80%）
- 現状はすべてモックデータ（`lib/mock-data.ts`, `lib/api.ts`）を使用
- Backend Serversync (#96) と Frontend Ingest API (#116) が実装され、Frontend DB にデータが蓄積される
- UI から Frontend DB のデータを取得し、実データでの動作確認が必要

### 2.2 目的

- モックデータから実データへの切り替え
- RD-005 の受入条件（AC-1 〜 AC-14）を実データで検証
- Backend との E2E データフローを確立

### 2.3 ユーザーストーリー

```
As a システム管理者
I want Frontend UI で実際の通話履歴・録音・要約を閲覧したい
So that Backend で処理された通話データを確認し、システムが正常に動作しているか検証できる

受入条件:
- [ ] Dashboard に Frontend DB から集計した KPI（通話数、平均通話時間、応答率）が表示される
- [ ] 通話履歴一覧に Frontend DB の call_logs テーブルのデータが表示される
- [ ] フィルタ（日付範囲、通話種別、キーワード検索）が動作する
- [ ] ページネーション（10/25/50 件切替）が動作する
- [ ] 通話詳細で録音ファイル（mixed.wav）が再生できる
- [ ] 文字起こしと AI 要約が表示される
- [ ] recordingUrl が null の場合「準備中」と表示される
- [ ] モックデータ関連ファイル（lib/mock-data.ts）を削除または無効化する
```

---

## 3. 段取り（Who / When）

### 3.1 起票

| 項目 | 値 |
|------|-----|
| 起票者 | @MasanoriSuda |
| 起票日 | 2026-02-07 |
| 起票理由 | Frontend UI 実装完了後、Backend 連携を実施してデータフローを確立するため |

### 3.2 仕様作成

| 項目 | 値 |
|------|-----|
| 作成者 | Claude Code (Sonnet 4.5) |
| 作成日 | 2026-02-07 |
| 指示者 | @MasanoriSuda |
| 指示内容 | "Issue #119 で Frontend UI と Backend の連携実装を行う。モックデータから実データへ切り替え、AC 検証を含む" |

### 3.3 レビュー

| # | レビュアー | 日付 | 結果 | コメント |
|---|-----------|------|------|---------|
| - | - | - | - | |

### 3.4 承認

| 項目 | 値 |
|------|-----|
| 承認者 | @MasanoriSuda |
| 承認日 | 2026-02-07 |
| 承認コメント | Frontend DB 集計（Backend 起点冪等取り込み前提）、波形表示は Phase 2 対応、モックデータ切り替え機能の方針を承認。実装は Codex へ引き継ぎ。 |

### 3.5 実装

| 項目 | 値 |
|------|-----|
| 実装者 | Codex (Frontend) |
| 実装日 | |
| 指示者 | @MasanoriSuda |
| 指示内容 | |
| コードレビュー | |

### 3.6 マージ

| 項目 | 値 |
|------|-----|
| マージ実行者 | |
| マージ日 | |
| マージ先 | RD-005, DD-006 (新規作成) |

---

## 4. 影響範囲

### 4.1 影響するドキュメント

| ドキュメント | 変更種別 | 概要 |
|-------------|---------|------|
| docs/requirements/RD-005_frontend.md | 修正 | § 6.1 前提条件を更新（POST /api/ingest/call → POST /api/ingest/sync + recording-file） |
| docs/design/detail/DD-006_ingest-api.md | 新規 | Frontend Ingest API 詳細設計（STEER-116 § 5.6 で計画済み） |
| docs/design/detail/DD-007_data-layer.md | 新規 | データ取得層（lib/api.ts, Prisma）の詳細設計 |
| docs/test/unit/UT-006_api-layer.md | 新規 | lib/api.ts のユニットテスト仕様 |
| docs/test/system/ST-005_e2e-flow.md | 修正 | Backend → Frontend E2E データフローのテストケース追加 |

### 4.2 影響するコード

| モジュール | 変更種別 | 概要 |
|-----------|---------|------|
| **lib/api.ts** | 修正 | モックデータから Prisma クライアント使用に変更 |
| lib/mock-data.ts | 削除検討 | モックデータファイル（開発用に残す選択肢もあり） |
| lib/db/prisma.ts | 新規 | Prisma クライアントのシングルトンインスタンス |
| lib/db/queries.ts | 新規 | Prisma クエリヘルパー（フィルタ、ページネーション） |
| lib/aggregations.ts | 新規 | KPI 集計ロジック（通話数、平均通話時間、応答率） |
| app/api/calls/route.ts | 新規 | GET /api/calls（通話履歴取得 API） |
| app/api/kpi/route.ts | 新規 | GET /api/kpi（KPI データ取得 API） |
| app/api/recordings/[id]/route.ts | 新規 | GET /api/recordings/{id}（録音ファイル配信） |
| components/dashboard/kpi-cards.tsx | 修正 | 静的 mockKPI から API フェッチに変更 |
| components/call-history-content.tsx | 修正 | フィルタ・ページネーションの実装確認 |
| components/call-detail-view.tsx | 修正 | recordingUrl が null の場合の「準備中」表示確認 |

---

## 5. 差分仕様（What / How）

### 5.1 RD-005 の修正（§ 6.1 前提条件）

**旧記載**:
```markdown
### 6.1 前提条件

- Backend が `POST /api/ingest/call` で通話データを Frontend に送信する
- 録音ファイルは `recordingUrl` 経由で取得可能（Range 対応）
- MVP では認証なし
```

**新記載**:
```markdown
### 6.1 前提条件

- Backend Serversync が `POST /api/ingest/sync` でメタデータを Frontend に送信する（STEER-096, STEER-116）
- Backend Serversync が `POST /api/ingest/recording-file` で録音ファイルを Frontend に送信する
- 録音ファイルは Frontend ローカルストレージ `/storage/recordings/{callLogId}/mixed.wav` に保存される
- Frontend はローカル DB（PostgreSQL）からデータを取得して UI に表示する
- MVP では認証なし
```

**マージ先**: RD-005 § 6.1

---

### 5.2 DD-007 の新規作成（データ取得層設計）

#### DD-007-FN-01: Prisma クライアント初期化

**ファイル**: `lib/db/prisma.ts`

```typescript
import { PrismaClient } from '@prisma/client';

// グローバルシングルトン（開発時のホットリロード対応）
const globalForPrisma = global as unknown as { prisma: PrismaClient };

export const prisma =
  globalForPrisma.prisma ||
  new PrismaClient({
    log: process.env.NODE_ENV === 'development' ? ['query', 'error', 'warn'] : ['error'],
  });

if (process.env.NODE_ENV !== 'production') globalForPrisma.prisma = prisma;
```

**目的**: Prisma クライアントのシングルトンインスタンスを提供

**トレース**:
- → UT: UT-006-TC-01（Prisma クライアント初期化テスト）

---

#### DD-007-FN-02: getCalls（通話履歴取得）

**ファイル**: `lib/api.ts`

**シグネチャ**:
```typescript
export interface CallFilters {
  dateRange?: {
    start: Date;
    end: Date;
  };
  direction?: 'inbound' | 'outbound' | 'missed' | null;
  keyword?: string;
  page?: number;
  pageSize?: number;
}

export async function getCalls(filters?: CallFilters): Promise<{
  calls: Call[];
  total: number;
  page: number;
  pageSize: number;
}>
```

**実装**:
```typescript
import { prisma } from './db/prisma';

export async function getCalls(filters?: CallFilters) {
  const {
    dateRange,
    direction,
    keyword,
    page = 1,
    pageSize = 10,
  } = filters || {};

  // WHERE 条件構築
  const where: any = {};

  if (dateRange) {
    where.startedAt = {
      gte: dateRange.start,
      lte: dateRange.end,
    };
  }

  if (direction) {
    where.direction = direction;
  }

  if (keyword) {
    where.OR = [
      { callerNumber: { contains: keyword } },
      { recipientNumber: { contains: keyword } },
      { callerName: { contains: keyword } },
    ];
  }

  // クエリ実行
  const [calls, total] = await Promise.all([
    prisma.callLog.findMany({
      where,
      orderBy: { startedAt: 'desc' },
      skip: (page - 1) * pageSize,
      take: pageSize,
    }),
    prisma.callLog.count({ where }),
  ]);

  return {
    calls: calls.map(mapCallLogToCall),
    total,
    page,
    pageSize,
  };
}

// CallLog → Call 型変換
function mapCallLogToCall(callLog: any): Call {
  return {
    id: callLog.id,
    callId: callLog.externalCallId || callLog.sipCallId,
    from: callLog.callerNumber,
    fromName: callLog.callerName,
    to: callLog.recipientNumber,
    direction: callLog.direction as 'inbound' | 'outbound' | 'missed',
    status: callLog.callStatus,
    startedAt: callLog.startedAt.toISOString(),
    endedAt: callLog.endedAt?.toISOString() || null,
    durationSec: callLog.durationSeconds,
  };
}
```

**トレース**:
- ← RD: RD-005-FR-03（発着信履歴）
- → UT: UT-006-TC-02（getCalls フィルタテスト）

---

#### DD-007-FN-03: getCallDetail（通話詳細取得）

**ファイル**: `lib/api.ts`

**シグネチャ**:
```typescript
export async function getCallDetail(callId: string): Promise<CallDetail | null>
```

**実装**:
```typescript
export async function getCallDetail(callId: string): Promise<CallDetail | null> {
  const callLog = await prisma.callLog.findFirst({
    where: {
      OR: [
        { id: callId },
        { externalCallId: callId },
        { sipCallId: callId },
      ],
    },
    include: {
      recordings: true,
    },
  });

  if (!callLog) return null;

  // Recording 情報の集約
  const recording = callLog.recordings[0]; // MVP では1通話1録音想定

  return {
    ...mapCallLogToCall(callLog),
    recordingUrl: recording?.fileUrl || null,
    transcript: recording?.transcriptJson || null,
    summary: recording?.summaryText || null,
  };
}
```

**トレース**:
- ← RD: RD-005-FR-04（通話詳細）
- → UT: UT-006-TC-03（getCallDetail テスト）

---

#### DD-007-FN-04: getKPI（KPI データ集計）

**ファイル**: `lib/aggregations.ts`

**シグネチャ**:
```typescript
export interface KPI {
  totalCalls: number;
  totalCallsChange: number; // 前日比 %
  avgDuration: number; // 秒
  avgDurationChange: number;
  answerRate: number; // %
  answerRateChange: number;
}

export async function getKPI(date: Date = new Date()): Promise<KPI>
```

**実装**:
```typescript
import { prisma } from './db/prisma';

export async function getKPI(date: Date = new Date()): Promise<KPI> {
  const today = new Date(date.setHours(0, 0, 0, 0));
  const tomorrow = new Date(today);
  tomorrow.setDate(tomorrow.getDate() + 1);

  const yesterday = new Date(today);
  yesterday.setDate(yesterday.getDate() - 1);

  // 本日の通話
  const todayCalls = await prisma.callLog.findMany({
    where: {
      startedAt: {
        gte: today,
        lt: tomorrow,
      },
    },
  });

  // 前日の通話
  const yesterdayCalls = await prisma.callLog.findMany({
    where: {
      startedAt: {
        gte: yesterday,
        lt: today,
      },
    },
  });

  // 本日 KPI 計算
  const totalCalls = todayCalls.length;
  const answeredCalls = todayCalls.filter(c => c.callStatus === 'answered').length;
  const avgDuration = todayCalls.reduce((sum, c) => sum + (c.durationSeconds || 0), 0) / (answeredCalls || 1);
  const answerRate = totalCalls > 0 ? (answeredCalls / totalCalls) * 100 : 0;

  // 前日 KPI 計算
  const yesterdayTotal = yesterdayCalls.length;
  const yesterdayAnswered = yesterdayCalls.filter(c => c.callStatus === 'answered').length;
  const yesterdayAvgDuration = yesterdayCalls.reduce((sum, c) => sum + (c.durationSeconds || 0), 0) / (yesterdayAnswered || 1);
  const yesterdayAnswerRate = yesterdayTotal > 0 ? (yesterdayAnswered / yesterdayTotal) * 100 : 0;

  // 前日比計算
  const totalCallsChange = yesterdayTotal > 0 ? ((totalCalls - yesterdayTotal) / yesterdayTotal) * 100 : 0;
  const avgDurationChange = yesterdayAvgDuration > 0 ? ((avgDuration - yesterdayAvgDuration) / yesterdayAvgDuration) * 100 : 0;
  const answerRateChange = yesterdayAnswerRate > 0 ? answerRate - yesterdayAnswerRate : 0;

  return {
    totalCalls,
    totalCallsChange,
    avgDuration,
    avgDurationChange,
    answerRate,
    answerRateChange,
  };
}
```

**トレース**:
- ← RD: RD-005-FR-02（Dashboard）
- → UT: UT-006-TC-04（getKPI 集計ロジックテスト）

**注記**:
- **MVP 方針**: Frontend DB で集計（上記実装）
- **データ前提**: Backend 起点で冪等に取り込まれた「正」の通話イベントを集計
- **将来展望**: Backend 集計 API 化を検討（§8.4 参照）

---

#### DD-007-FN-05: getHourlyStats（時間帯別通話数）

**ファイル**: `lib/aggregations.ts`

**シグネチャ**:
```typescript
export interface HourlyStat {
  hour: number; // 0-23
  calls: number;
}

export async function getHourlyStats(date: Date = new Date()): Promise<HourlyStat[]>
```

**実装**:
```typescript
export async function getHourlyStats(date: Date = new Date()): Promise<HourlyStat[]> {
  const today = new Date(date.setHours(0, 0, 0, 0));
  const tomorrow = new Date(today);
  tomorrow.setDate(tomorrow.getDate() + 1);

  const calls = await prisma.callLog.findMany({
    where: {
      startedAt: {
        gte: today,
        lt: tomorrow,
      },
    },
    select: {
      startedAt: true,
    },
  });

  // 時間帯別集計
  const hourlyMap: Record<number, number> = {};
  for (let i = 0; i < 24; i++) {
    hourlyMap[i] = 0;
  }

  calls.forEach(call => {
    const hour = call.startedAt.getHours();
    hourlyMap[hour]++;
  });

  return Object.entries(hourlyMap).map(([hour, calls]) => ({
    hour: parseInt(hour),
    calls,
  }));
}
```

**トレース**:
- ← RD: RD-005-FR-02（Dashboard グラフ）
- → UT: UT-006-TC-05（getHourlyStats 集計テスト）

---

### 5.3 API Routes の実装

#### DD-007-FN-06: GET /api/calls（通話履歴 API）

**ファイル**: `app/api/calls/route.ts`

```typescript
import { NextRequest, NextResponse } from 'next/server';
import { getCalls } from '@/lib/api';

export async function GET(request: NextRequest) {
  const searchParams = request.nextUrl.searchParams;

  // クエリパラメータ解析
  const filters = {
    dateRange: searchParams.get('startDate') && searchParams.get('endDate') ? {
      start: new Date(searchParams.get('startDate')!),
      end: new Date(searchParams.get('endDate')!),
    } : undefined,
    direction: searchParams.get('direction') as any,
    keyword: searchParams.get('keyword') || undefined,
    page: parseInt(searchParams.get('page') || '1'),
    pageSize: parseInt(searchParams.get('pageSize') || '10'),
  };

  const result = await getCalls(filters);

  return NextResponse.json(result);
}
```

**トレース**:
- ← DD: DD-007-FN-02（getCalls）
- → ST: ST-005-TC-01（通話履歴取得 E2E テスト）

---

#### DD-007-FN-07: GET /api/kpi（KPI データ API）

**ファイル**: `app/api/kpi/route.ts`

```typescript
import { NextRequest, NextResponse } from 'next/server';
import { getKPI } from '@/lib/aggregations';

export async function GET(request: NextRequest) {
  const searchParams = request.nextUrl.searchParams;
  const date = searchParams.get('date') ? new Date(searchParams.get('date')!) : new Date();

  const kpi = await getKPI(date);

  return NextResponse.json(kpi);
}
```

**トレース**:
- ← DD: DD-007-FN-04（getKPI）
- → ST: ST-005-TC-02（KPI 取得 E2E テスト）

---

#### DD-007-FN-08: GET /api/recordings/[id]（録音ファイル配信）

**ファイル**: `app/api/recordings/[id]/route.ts`

```typescript
import { NextRequest, NextResponse } from 'next/server';
import { readFile, stat } from 'fs/promises';
import path from 'path';

export async function GET(
  request: NextRequest,
  { params }: { params: { id: string } }
) {
  const recordingId = params.id;

  // ファイルパス構築
  const filePath = path.join(
    process.cwd(),
    'storage',
    'recordings',
    recordingId,
    'mixed.wav'
  );

  try {
    // ファイル存在確認
    const stats = await stat(filePath);

    // Range リクエスト対応（音声シーク用）
    const range = request.headers.get('range');
    if (range) {
      const parts = range.replace(/bytes=/, '').split('-');
      const start = parseInt(parts[0], 10);
      const end = parts[1] ? parseInt(parts[1], 10) : stats.size - 1;
      const chunksize = (end - start) + 1;

      const file = await readFile(filePath);
      const chunk = file.slice(start, end + 1);

      return new NextResponse(chunk, {
        status: 206,
        headers: {
          'Content-Range': `bytes ${start}-${end}/${stats.size}`,
          'Accept-Ranges': 'bytes',
          'Content-Length': chunksize.toString(),
          'Content-Type': 'audio/wav',
        },
      });
    }

    // 通常レスポンス
    const file = await readFile(filePath);
    return new NextResponse(file, {
      headers: {
        'Content-Type': 'audio/wav',
        'Content-Length': stats.size.toString(),
      },
    });
  } catch (error) {
    return NextResponse.json(
      { error: 'Recording not found' },
      { status: 404 }
    );
  }
}
```

**トレース**:
- ← RD: RD-005-FR-04（通話詳細 - 録音再生）
- → ST: ST-005-TC-03（録音ファイル配信テスト）

---

### 5.4 コンポーネント修正

#### DD-007-FN-09: KpiCards の API 連携

**ファイル**: `components/dashboard/kpi-cards.tsx`

**変更前**:
```typescript
import { mockKPI } from "@/lib/mock-data";

export function KpiCards() {
  const kpi = mockKPI;
  // ...
}
```

**変更後**:
```typescript
"use client";

import { useEffect, useState } from 'react';

export function KpiCards() {
  const [kpi, setKpi] = useState<KPI | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    fetch('/api/kpi')
      .then(res => res.json())
      .then(data => {
        setKpi(data);
        setLoading(false);
      })
      .catch(err => {
        console.error('Failed to fetch KPI:', err);
        setLoading(false);
      });
  }, []);

  if (loading) return <div>Loading...</div>;
  if (!kpi) return <div>Failed to load KPI data</div>;

  // 既存のレンダリングロジック
}
```

**トレース**:
- ← DD: DD-007-FN-07（GET /api/kpi）
- → UT: UT-006-TC-06（KpiCards コンポーネントテスト）

---

#### DD-007-FN-10: CallHistoryContent の API 連携

**ファイル**: `components/call-history-content.tsx`

**変更前**:
```typescript
import { mockCalls } from "@/lib/mock-data";

export function CallHistoryContent() {
  const [calls, setCalls] = useState(mockCalls);
  // ...
}
```

**変更後**:
```typescript
"use client";

import { useEffect, useState } from 'react';

export function CallHistoryContent() {
  const [calls, setCalls] = useState<Call[]>([]);
  const [loading, setLoading] = useState(true);
  const [filters, setFilters] = useState<CallFilters>({
    page: 1,
    pageSize: 10,
  });
  const [total, setTotal] = useState(0);

  useEffect(() => {
    const params = new URLSearchParams();
    if (filters.dateRange) {
      params.set('startDate', filters.dateRange.start.toISOString());
      params.set('endDate', filters.dateRange.end.toISOString());
    }
    if (filters.direction) params.set('direction', filters.direction);
    if (filters.keyword) params.set('keyword', filters.keyword);
    params.set('page', filters.page.toString());
    params.set('pageSize', filters.pageSize.toString());

    fetch(`/api/calls?${params.toString()}`)
      .then(res => res.json())
      .then(data => {
        setCalls(data.calls);
        setTotal(data.total);
        setLoading(false);
      })
      .catch(err => {
        console.error('Failed to fetch calls:', err);
        setLoading(false);
      });
  }, [filters]);

  // フィルタ変更ハンドラ
  const handleFilterChange = (newFilters: Partial<CallFilters>) => {
    setFilters(prev => ({ ...prev, ...newFilters }));
  };

  // 既存のレンダリングロジック
}
```

**トレース**:
- ← DD: DD-007-FN-06（GET /api/calls）
- → UT: UT-006-TC-07（CallHistoryContent フィルタテスト）

---

#### DD-007-FN-11: CallDetailView の recordingUrl null 対応

**ファイル**: `components/call-detail-view.tsx`

**追加ロジック**:
```typescript
export function CallDetailView({ callId }: { callId: string }) {
  const [detail, setDetail] = useState<CallDetail | null>(null);

  // ... データ取得ロジック

  // Recording タブ
  const renderRecordingTab = () => {
    if (!detail?.recordingUrl) {
      return (
        <div className="flex items-center justify-center h-64">
          <div className="text-center">
            <p className="text-lg font-medium">録音準備中</p>
            <p className="text-sm text-muted-foreground mt-2">
              録音ファイルの処理が完了するまでお待ちください
            </p>
          </div>
        </div>
      );
    }

    return <AudioPlayer src={detail.recordingUrl} />;
  };

  // ...
}
```

**トレース**:
- ← RD: RD-005 AC-14（recordingUrl が null の場合「準備中」表示）
- → ST: ST-005-TC-04（録音準備中表示テスト）

---

### 5.5 モックデータの取り扱い

**決定事項**: lib/mock-data.ts は削除せず、開発モードで切り替え可能にする

**実装**: 環境変数 `NEXT_PUBLIC_USE_MOCK_DATA` で制御

**lib/api.ts の修正**:
```typescript
const USE_MOCK = process.env.NEXT_PUBLIC_USE_MOCK_DATA === 'true';

export async function getCalls(filters?: CallFilters) {
  if (USE_MOCK) {
    // 既存のモックデータロジック
    return { calls: mockCalls, total: mockCalls.length, page: 1, pageSize: 10 };
  }

  // 実データ取得ロジック（上記参照）
}
```

**メリット**:
- Backend 未起動時の開発継続が可能
- デザイン確認用の安定したダミーデータ提供
- E2E テスト時のモックモード切替

---

### 5.6 テストケース追加（UT-006 へマージ）

#### UT-006-TC-01: Prisma クライアント初期化

**対象**: DD-007-FN-01

**目的**: Prisma クライアントがシングルトンとして正しく初期化されることを確認

**入力**: なし

**期待結果**: `prisma` インスタンスが取得でき、同一インスタンスが返される

**実装**（Vitest）:
```typescript
import { describe, it, expect } from 'vitest';
import { prisma } from '@/lib/db/prisma';

describe('Prisma Client', () => {
  it('should return singleton instance', () => {
    const instance1 = prisma;
    const instance2 = prisma;
    expect(instance1).toBe(instance2);
  });
});
```

---

#### UT-006-TC-02: getCalls フィルタテスト

**対象**: DD-007-FN-02

**目的**: フィルタ条件が正しく適用されることを確認

**入力**:
```typescript
const filters: CallFilters = {
  dateRange: {
    start: new Date('2026-02-01'),
    end: new Date('2026-02-07'),
  },
  direction: 'inbound',
  keyword: '090',
  page: 1,
  pageSize: 10,
};
```

**期待結果**:
- 日付範囲内の通話のみ返される
- direction='inbound' の通話のみ返される
- keyword='090' を含む通話のみ返される
- ページネーションが適用される

**実装**（Vitest + Prisma Mock）:
```typescript
import { describe, it, expect, vi } from 'vitest';
import { getCalls } from '@/lib/api';
import { prisma } from '@/lib/db/prisma';

vi.mock('@/lib/db/prisma', () => ({
  prisma: {
    callLog: {
      findMany: vi.fn(),
      count: vi.fn(),
    },
  },
}));

describe('getCalls', () => {
  it('should apply filters correctly', async () => {
    // Mock データセットアップ
    const mockCalls = [/* ... */];
    vi.mocked(prisma.callLog.findMany).mockResolvedValue(mockCalls);
    vi.mocked(prisma.callLog.count).mockResolvedValue(mockCalls.length);

    const result = await getCalls({
      direction: 'inbound',
      keyword: '090',
    });

    expect(result.calls).toHaveLength(mockCalls.length);
    expect(prisma.callLog.findMany).toHaveBeenCalledWith(
      expect.objectContaining({
        where: expect.objectContaining({
          direction: 'inbound',
          OR: expect.any(Array),
        }),
      })
    );
  });
});
```

---

#### UT-006-TC-03: getCallDetail テスト

**対象**: DD-007-FN-03

**目的**: callId から通話詳細が取得できることを確認

**入力**: `callId = "test-call-123"`

**期待結果**: CallDetail オブジェクトが返される、または null

---

#### UT-006-TC-04: getKPI 集計ロジックテスト

**対象**: DD-007-FN-04

**目的**: KPI 集計が正しく計算されることを確認

**入力**: テスト用の通話データ（本日5件、前日3件）

**期待結果**:
- totalCalls = 5
- totalCallsChange = (5-3)/3 * 100 ≈ 66.67%
- avgDuration が正しく計算される
- answerRate が正しく計算される

---

#### UT-006-TC-05: getHourlyStats 集計テスト

**対象**: DD-007-FN-05

**目的**: 時間帯別集計が正しく行われることを確認

**入力**: テスト用の通話データ（9時に2件、14時に3件）

**期待結果**:
```typescript
[
  { hour: 9, calls: 2 },
  { hour: 14, calls: 3 },
  // ... 他の時間帯は 0
]
```

---

### 5.7 システムテスト追加（ST-005 へマージ）

#### ST-005-TC-01: Backend → Frontend E2E データフロー

**対象**: 全体フロー

**目的**: Backend Serversync → Frontend Ingest API → Frontend DB → UI 表示の E2E 動作確認

**手順**:
1. Backend で通話を終了（call_logs + recordings + sync_outbox に INSERT）
2. Serversync を起動（5分待機 or 手動トリガー）
3. Frontend POST /api/ingest/sync, POST /api/ingest/recording-file にデータが届く
4. Frontend DB（call_logs, recordings）にデータが保存される
5. Frontend UI で通話履歴一覧に表示される
6. 通話詳細を開き、録音再生・文字起こし・要約が表示される

**期待結果**: 全手順が正常に完了し、UI で実データが表示される

**トレース**:
- ← RD: RD-005 全機能要件
- ← STEER-096: Backend Serversync 実装
- ← STEER-116: Frontend Ingest API 実装

---

## 6. トレーサビリティ

| From | To | 関係 |
|------|-----|------|
| Issue #119 | STEER-119 | 起票 |
| STEER-119 | RD-005 § 6.1 | 前提条件更新 |
| STEER-119 | DD-007 | データ取得層詳細設計 |
| DD-007-FN-02 | UT-006-TC-02 | 単体テスト |
| DD-007-FN-04 | UT-006-TC-04 | 単体テスト |
| RD-005 全機能要件 | ST-005-TC-01 | E2E システムテスト |

---

## 7. レビューチェックリスト

### 7.1 仕様レビュー（Review → Approved）

- [ ] データ取得層（lib/api.ts）の設計が明確か
- [ ] Prisma クエリのパフォーマンスが考慮されているか（N+1 問題等）
- [ ] KPI 集計ロジックが正しいか（前日比計算、応答率計算）
- [ ] 録音ファイル配信の Range 対応が実装されているか
- [ ] モックデータとの切り替え方針が明確か
- [ ] RD-005 の受入条件（AC-1 〜 AC-14）がカバーされているか
- [ ] トレーサビリティが維持されているか

### 7.2 マージ前チェック（Approved → Merged）

- [ ] lib/api.ts の Prisma 実装完了
- [ ] lib/aggregations.ts の KPI 集計実装完了
- [ ] API Routes（GET /api/calls, GET /api/kpi, GET /api/recordings/[id]）実装完了
- [ ] コンポーネント修正（KpiCards, CallHistoryContent, CallDetailView）完了
- [ ] UT-006（ユニットテスト）実装・PASS
- [ ] ST-005-TC-01（E2E テスト）PASS
- [ ] RD-005 § 6.1 更新完了
- [ ] DD-007 作成完了
- [ ] モックデータ切り替え機能確認完了

---

## 8. 備考

### 8.1 パフォーマンス考慮

- **N+1 問題対策**: `prisma.callLog.findMany({ include: { recordings: true } })` で JOIN 取得
- **インデックス**: `startedAt`, `direction`, `callerNumber` にインデックス推奨（Prisma スキーマで定義）
- **ページネーション**: デフォルト10件、最大100件に制限
- **キャッシュ**: KPI データは1分間キャッシュ（将来的に Redis 導入検討）

### 8.2 エラーハンドリング

- **DB 接続失敗**: フォールバック UI（「データ取得に失敗しました」）
- **録音ファイル不在**: 「準備中」表示（AC-14）
- **フィルタ不正値**: バリデーション + 400 Bad Request

### 8.3 開発モード

- **環境変数**:
  - `NEXT_PUBLIC_USE_MOCK_DATA=true` — モックデータ使用
  - `DATABASE_URL` — Frontend DB 接続文字列
- **開発時の切り替え**: `.env.local` で `NEXT_PUBLIC_USE_MOCK_DATA` を設定

### 8.4 将来の拡張

- **リアルタイム更新**: WebSocket ストリーミング（別イシュー #121 等）
- **CSV エクスポート**: 既存実装の整理・テスト追加（別イシュー #120 等）
- **音声波形表示**: Phase 2 で peak 事前生成方式を検討
  - 録音ファイル保存時に波形データ（peak 配列）を事前生成
  - Frontend は事前生成された peak データを取得して描画
  - レビュー効率が課題になった時点で優先度を再評価
- **Backend KPI API**: 現在は Frontend DB で集計、将来は Backend 集計 API から取得も検討
  - Backend で集計した KPI を GET /api/kpi で取得
  - Frontend は取得・表示のみ（RD-005 § 3.2 の元の方針に準拠）
  - Backend 負荷と Frontend 集計コストのトレードオフを評価後に判断

---

## 9. Open Questions（解決済み）

- [x] Q1: Prisma スキーマに追加のインデックスが必要か？
  - **回答**: §8.1 パフォーマンス考慮を参照。startedAt, direction, callerNumber にインデックス推奨
- [x] Q2: KPI 集計は Frontend DB で行うか、Backend API 経由か？
  - **回答**: MVP は Frontend DB で集計。Backend 起点で冪等に取り込まれた「正」の通話イベントを集計。将来は Backend 集計 API 化を検討
- [x] Q3: 音声プレイヤーに波形表示機能があるか？
  - **回答**: MVP では非必須。イベントタイムライン + 再生操作で運用。レビュー効率が課題になったら Phase 2 で peak 事前生成方式を追加
- [x] Q4: CSV エクスポート機能をこのイシューに含めるか？
  - **回答**: 別イシューで対応（#120 等）

---

## 変更履歴

| 日付 | 変更内容 | 作成者 |
|------|---------|--------|
| 2026-02-07 | 初版作成 | Claude Code (Sonnet 4.5) |
| 2026-02-07 | Q2（KPI 集計）、Q3（波形表示）の方針を明確化。Frontend DB 集計（MVP）、Backend 起点冪等取り込み前提、波形は Phase 2 で検討 | Claude Code (Sonnet 4.5) |
| 2026-02-07 | Draft → Approved（承認者: @MasanoriSuda） | Claude Code (Sonnet 4.5) |
