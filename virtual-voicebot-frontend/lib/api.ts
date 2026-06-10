import type { Call, CallDetail, CallReview, CallReviewStatus, IvrSessionEvent, Utterance } from "./types"
import { mockCallPresentationById, mockCalls } from "./mock-data"
import { queryCallByAnyId, queryCalls, queryIvrSessionEvents, type CallDirection } from "./db/queries"
import type { StoredCallLog, StoredIvrSessionEvent, StoredRecording } from "./db/sync"

export interface CallFilters {
  dateRange?: {
    start: Date
    end: Date
  }
  direction?: CallDirection | null
  keyword?: string
  page?: number
  pageSize?: number
}

export interface CallsPageResult {
  calls: Call[]
  total: number
  page: number
  pageSize: number
}

const USE_MOCK = process.env.NEXT_PUBLIC_USE_MOCK_DATA === "true"

const mockUtterances: Record<string, Utterance[]> = {
  "1": [
    {
      seq: 1,
      speaker: "bot",
      text: "お電話ありがとうございます。どのようなご用件でしょうか？",
      timestamp: "2026-02-02T10:30:05Z",
      isFinal: true,
      startSec: 5,
      endSec: 12,
    },
    {
      seq: 2,
      speaker: "caller",
      text: "配送状況を確認したいのですが",
      timestamp: "2026-02-02T10:30:15Z",
      isFinal: true,
      startSec: 15,
      endSec: 20,
    },
  ],
}

function asISOString(value: string | null | undefined, fallback = new Date().toISOString()): string {
  if (!value) {
    return fallback
  }
  const parsed = Date.parse(value)
  return Number.isNaN(parsed) ? fallback : new Date(parsed).toISOString()
}

function mapStoredCallToCall(callLog: StoredCallLog): Call {
  const isOutbound = callLog.direction === "outbound" || callLog.actionCode === "AR"
  return {
    id: callLog.id,
    externalCallId: callLog.externalCallId,
    callerNumber: callLog.callerNumber,
    direction: isOutbound ? "outbound" : "inbound",
    calleeNumber: callLog.calleeNumber ?? null,
    callerCategory: (callLog.callerCategory as Call["callerCategory"]) ?? "unknown",
    actionCode: (callLog.actionCode as Call["actionCode"]) ?? "IV",
    status: (callLog.status as Call["status"]) ?? "ended",
    startedAt: asISOString(callLog.startedAt),
    answeredAt: callLog.answeredAt ? asISOString(callLog.answeredAt) : null,
    endedAt: callLog.endedAt ? asISOString(callLog.endedAt) : null,
    durationSec: callLog.durationSec,
    endReason: (callLog.endReason as Call["endReason"]) ?? "normal",
    callDisposition: (callLog.callDisposition as Call["callDisposition"]) ?? "allowed",
    finalAction: (callLog.finalAction as Call["finalAction"]) ?? null,
    transferStatus: (callLog.transferStatus as Call["transferStatus"]) ?? "no_transfer",
    transferStartedAt: callLog.transferStartedAt ? asISOString(callLog.transferStartedAt) : null,
    transferAnsweredAt: callLog.transferAnsweredAt ? asISOString(callLog.transferAnsweredAt) : null,
    transferEndedAt: callLog.transferEndedAt ? asISOString(callLog.transferEndedAt) : null,
  }
}

function mapStoredIvrEvent(event: StoredIvrSessionEvent): IvrSessionEvent {
  return {
    id: event.id,
    callLogId: event.callLogId,
    sequence: event.sequence,
    eventType: event.eventType as IvrSessionEvent["eventType"],
    occurredAt: asISOString(event.occurredAt),
    nodeId: event.nodeId,
    dtmfKey: event.dtmfKey,
    transitionId: event.transitionId,
    exitAction: event.exitAction,
    exitReason: event.exitReason,
    metadata: event.metadata,
  }
}

function buildRecordingUrl(callLogId: string, recording: StoredRecording | null): string | null {
  if (!recording) {
    return null
  }
  if (recording.s3Url && recording.s3Url.trim().length > 0) {
    if (recording.s3Url.includes("/storage/recordings/")) {
      return `/api/recordings/${encodeURIComponent(callLogId)}`
    }
    return recording.s3Url
  }
  if (recording.uploadStatus === "uploaded") {
    return `/api/recordings/${encodeURIComponent(callLogId)}`
  }
  return null
}

function toSpeaker(value: unknown): Utterance["speaker"] {
  if (value === "caller" || value === "bot" || value === "system" || value === "unknown") {
    return value
  }
  return "system"
}

function toUtterances(raw: unknown): Utterance[] {
  const source = Array.isArray(raw)
    ? raw
    : typeof raw === "object" && raw !== null && Array.isArray((raw as { utterances?: unknown[] }).utterances)
      ? (raw as { utterances: unknown[] }).utterances
      : []
  return source.reduce<Utterance[]>((acc, item, index) => {
    if (typeof item !== "object" || item === null) {
      return acc
    }

    const row = item as Record<string, unknown>
    const text = typeof row.text === "string" ? row.text : ""
    if (!text) {
      return acc
    }

    const seqRaw = row.seq
    const seq = typeof seqRaw === "number" && Number.isFinite(seqRaw) ? seqRaw : index + 1
    const timestampRaw = typeof row.timestamp === "string" ? row.timestamp : new Date().toISOString()
    const utterance: Utterance = {
      seq,
      speaker: toSpeaker(row.speaker),
      text,
      timestamp: asISOString(timestampRaw),
      isFinal: row.isFinal === false ? false : true,
    }

    if (typeof row.startSec === "number") {
      utterance.startSec = row.startSec
    }
    if (typeof row.endSec === "number") {
      utterance.endSec = row.endSec
    }

    acc.push(utterance)
    return acc
  }, [])
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value)
}

function normalizeReviewStatus(value: string | null | undefined): CallReviewStatus | undefined {
  switch (value) {
    case "pending":
    case "processing":
    case "completed":
    case "failed":
    case "skipped":
      return value
    default:
      return undefined
  }
}

function toCallReview(raw: unknown): CallReview | null {
  if (!isRecord(raw)) {
    return null
  }
  const summary = typeof raw.summary === "string" ? raw.summary : ""
  if (!summary.trim()) {
    return null
  }
  const responseEvaluation = isRecord(raw.responseEvaluation) ? raw.responseEvaluation : {}
  return {
    version: typeof raw.version === "number" ? raw.version : 1,
    summary,
    customerIntent: typeof raw.customerIntent === "string" ? raw.customerIntent : "",
    responseEvaluation: {
      status: toReviewEvaluationStatus(responseEvaluation.status),
      notes: typeof responseEvaluation.notes === "string" ? responseEvaluation.notes : "",
    },
    unresolvedItems: toStringArray(raw.unresolvedItems),
    nextActions: toReviewNextActions(raw.nextActions),
    riskSignals: toReviewRiskSignals(raw.riskSignals),
    evidence: toReviewEvidence(raw.evidence),
  }
}

function toReviewEvaluationStatus(value: unknown): CallReview["responseEvaluation"]["status"] {
  return value === "good" || value === "needs_attention" || value === "poor" || value === "unknown"
    ? value
    : "unknown"
}

function toStringArray(value: unknown): string[] {
  return Array.isArray(value)
    ? value.filter((item): item is string => typeof item === "string" && item.trim().length > 0)
    : []
}

function toReviewNextActions(value: unknown): CallReview["nextActions"] {
  if (!Array.isArray(value)) {
    return []
  }
  return value.flatMap((item) => {
    if (!isRecord(item) || typeof item.label !== "string" || item.label.trim() === "") {
      return []
    }
    return [
      {
        type:
          item.type === "follow_up" ||
          item.type === "confirm" ||
          item.type === "escalate" ||
          item.type === "none" ||
          item.type === "other"
            ? item.type
            : "other",
        priority:
          item.priority === "low" || item.priority === "medium" || item.priority === "high"
            ? item.priority
            : "medium",
        label: item.label,
      },
    ]
  })
}

function toReviewRiskSignals(value: unknown): CallReview["riskSignals"] {
  if (!Array.isArray(value)) {
    return []
  }
  return value.flatMap((item) => {
    if (!isRecord(item) || typeof item.label !== "string" || item.label.trim() === "") {
      return []
    }
    return [
      {
        type:
          item.type === "complaint_risk" ||
          item.type === "confusion" ||
          item.type === "urgent" ||
          item.type === "other"
            ? item.type
            : "other",
        severity:
          item.severity === "low" || item.severity === "medium" || item.severity === "high"
            ? item.severity
            : "low",
        label: item.label,
      },
    ]
  })
}

function toReviewEvidence(value: unknown): CallReview["evidence"] {
  if (!Array.isArray(value)) {
    return []
  }
  return value.flatMap((item) => {
    if (!isRecord(item) || typeof item.text !== "string" || item.text.trim() === "") {
      return []
    }
    return [
      {
        label: typeof item.label === "string" ? item.label : "根拠",
        speaker: toSpeaker(item.speaker),
        startSec: typeof item.startSec === "number" ? item.startSec : null,
        endSec: typeof item.endSec === "number" ? item.endSec : null,
        text: item.text,
      },
    ]
  })
}

function toMockCallDetail(call: Call): CallDetail {
  const view = mockCallPresentationById[call.id]
  return {
    ...call,
    from: call.callerNumber ?? "非通知",
    to: call.direction === "outbound" ? (call.calleeNumber ?? "未設定") : (view?.to ?? "未設定"),
    startTime: call.startedAt,
    duration: call.durationSec ?? 0,
    summary: view?.summary ?? "",
    recordingUrl: view?.recordingUrl ?? undefined,
    utterances: mockUtterances[call.id] || [],
  }
}

export async function getCallsPage(filters: CallFilters = {}): Promise<CallsPageResult> {
  if (USE_MOCK) {
    const page = filters.page ?? 1
    const pageSize = filters.pageSize ?? 10
    const from = (page - 1) * pageSize
    const to = from + pageSize
    return {
      calls: mockCalls.slice(from, to),
      total: mockCalls.length,
      page,
      pageSize,
    }
  }

  const result = await queryCalls({
    startDate: filters.dateRange?.start,
    endDate: filters.dateRange?.end,
    direction: filters.direction ?? null,
    keyword: filters.keyword,
    page: filters.page,
    pageSize: filters.pageSize,
  })

  return {
    calls: result.calls.map(mapStoredCallToCall),
    total: result.total,
    page: result.page,
    pageSize: result.pageSize,
  }
}

export async function getCalls(filters: CallFilters = {}): Promise<Call[]> {
  const result = await getCallsPage({
    ...filters,
    page: 1,
    pageSize: 1000,
  })
  return result.calls
}

export async function getCall(callId: string): Promise<Call | null> {
  if (USE_MOCK) {
    return mockCalls.find((item) => item.id === callId) ?? null
  }
  const row = await queryCallByAnyId(callId)
  if (!row) {
    return null
  }
  return mapStoredCallToCall(row)
}

export async function getCallDetail(callId: string): Promise<CallDetail | null> {
  if (USE_MOCK) {
    const call = mockCalls.find((item) => item.id === callId)
    return call ? toMockCallDetail(call) : null
  }

  const row = await queryCallByAnyId(callId)
  if (!row) {
    return null
  }

  const call = mapStoredCallToCall(row)
  const utterances = toUtterances(row.recording?.transcriptJson)
  const summary = row.recording?.summaryText ?? ""
  const recordingUrl = buildRecordingUrl(call.id, row.recording)
  const reviewStatus = normalizeReviewStatus(row.recording?.reviewStatus)
  const review = toCallReview(row.recording?.reviewJson)

  return {
    ...call,
    from: call.callerNumber ?? "非通知",
    to: call.direction === "outbound" ? (call.calleeNumber ?? "未設定") : "未設定",
    startTime: call.startedAt,
    duration: call.durationSec ?? 0,
    summary,
    recordingUrl: recordingUrl ?? undefined,
    utterances,
    reviewStatus,
    review,
  }
}

export async function getUtterances(callId: string): Promise<Utterance[]> {
  const detail = await getCallDetail(callId)
  return detail?.utterances ?? []
}

export async function getCallUtterances(callId: string): Promise<Utterance[]> {
  return getUtterances(callId)
}

export async function getRecordingUrl(callId: string): Promise<string | null> {
  const detail = await getCallDetail(callId)
  return detail?.recordingUrl ?? null
}

export async function getIvrSessionEvents(callId: string): Promise<IvrSessionEvent[]> {
  if (USE_MOCK) {
    return []
  }
  const row = await queryCallByAnyId(callId)
  if (!row) {
    return []
  }
  const events = await queryIvrSessionEvents(row.id)
  return events.map(mapStoredIvrEvent)
}
