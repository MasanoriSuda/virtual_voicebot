import {
  readSyncSnapshot,
  type StoredCallLog,
  type StoredIvrSessionEvent,
  type StoredRecording,
} from "@/lib/db/sync"

export type CallDirection = "inbound" | "outbound" | "missed"

export interface CallQueryFilters {
  startDate?: Date
  endDate?: Date
  direction?: CallDirection | null
  keyword?: string
  page?: number
  pageSize?: number
}

export interface QueryCallResult {
  calls: StoredCallLog[]
  total: number
  page: number
  pageSize: number
}

export interface QueryCallWithRecording extends StoredCallLog {
  recording: StoredRecording | null
}

export function deriveDirection(callLog: StoredCallLog): CallDirection {
  if (callLog.status === "error" || callLog.endReason === "rejected") {
    return "missed"
  }
  if (callLog.actionCode === "AR") {
    return "outbound"
  }
  return "inbound"
}

export async function queryCalls(filters: CallQueryFilters = {}): Promise<QueryCallResult> {
  const { callLogs } = await readSyncSnapshot()
  const page = filters.page && filters.page > 0 ? filters.page : 1
  const pageSize =
    filters.pageSize && filters.pageSize > 0 ? Math.min(filters.pageSize, 100) : 10
  const keyword = filters.keyword?.trim().toLowerCase()

  const filtered = callLogs
    .filter((callLog) => {
      const started = new Date(callLog.startedAt)
      if (filters.startDate && started < filters.startDate) {
        return false
      }
      if (filters.endDate && started > filters.endDate) {
        return false
      }
      if (filters.direction && deriveDirection(callLog) !== filters.direction) {
        return false
      }
      if (keyword) {
        const target = [callLog.externalCallId, callLog.sipCallId, callLog.callerNumber]
          .filter(Boolean)
          .join(" ")
          .toLowerCase()
        if (!target.includes(keyword)) {
          return false
        }
      }
      return true
    })
    .sort((a, b) => Date.parse(b.startedAt) - Date.parse(a.startedAt))

  const total = filtered.length
  const from = (page - 1) * pageSize
  const to = from + pageSize
  return {
    calls: filtered.slice(from, to),
    total,
    page,
    pageSize,
  }
}

export async function queryCallByAnyId(id: string): Promise<QueryCallWithRecording | null> {
  const { callLogs, recordings } = await readSyncSnapshot()
  const callLog = callLogs.find(
    (item) => item.id === id || item.externalCallId === id || item.sipCallId === id,
  )
  if (!callLog) {
    return null
  }
  const recording = recordings
    .filter((item) => item.callLogId === callLog.id)
    .sort((a, b) => a.sequenceNumber - b.sequenceNumber)[0]
  return {
    ...callLog,
    recording: recording ?? null,
  }
}

export async function queryIvrSessionEvents(callLogId: string): Promise<StoredIvrSessionEvent[]> {
  const { ivrSessionEvents } = await readSyncSnapshot()
  return ivrSessionEvents
    .filter((item) => item.callLogId === callLogId)
    .sort((a, b) => {
      if (a.sequence !== b.sequence) {
        return a.sequence - b.sequence
      }
      return Date.parse(a.occurredAt) - Date.parse(b.occurredAt)
    })
}

export async function queryActiveCallCount(): Promise<number> {
  const { callLogs } = await readSyncSnapshot()
  return callLogs.filter((item) => item.status === "ringing" || item.status === "in_call").length
}

export interface HourlyStat {
  hour: number
  inbound: number
  outbound: number
}

export async function queryHourlyStats(baseDate: Date = new Date()): Promise<HourlyStat[]> {
  const { callLogs } = await readSyncSnapshot()
  const start = new Date(baseDate)
  start.setHours(0, 0, 0, 0)
  const end = new Date(start)
  end.setDate(end.getDate() + 1)

  const stats = Array.from({ length: 24 }, (_, hour) => ({
    hour,
    inbound: 0,
    outbound: 0,
  }))
  for (const callLog of callLogs) {
    const startedAt = new Date(callLog.startedAt)
    if (startedAt < start || startedAt >= end) {
      continue
    }
    const hour = startedAt.getHours()
    const direction = deriveDirection(callLog)
    if (direction === "outbound") {
      stats[hour].outbound += 1
    } else {
      stats[hour].inbound += 1
    }
  }
  return stats
}

export interface KpiResult {
  totalCalls: number
  totalCallsChange: number
  avgDurationSec: number
  avgDurationChange: number
  answerRate: number
  answerRateChange: number
  activeCalls: number
}

function normalizeDay(date: Date): Date {
  const cloned = new Date(date)
  cloned.setHours(0, 0, 0, 0)
  return cloned
}

function calculateDayMetrics(callLogs: StoredCallLog[]): {
  total: number
  answered: number
  avgDurationSec: number
  answerRate: number
} {
  const total = callLogs.length
  const answeredCalls = callLogs.filter((item) => item.status === "ended" || item.status === "in_call")
  const answered = answeredCalls.length
  const durationTotal = answeredCalls.reduce((acc, item) => acc + (item.durationSec ?? 0), 0)
  const avgDurationSec = answered > 0 ? durationTotal / answered : 0
  const answerRate = total > 0 ? answered / total : 0
  return {
    total,
    answered,
    avgDurationSec,
    answerRate,
  }
}

export async function queryKpi(baseDate: Date = new Date()): Promise<KpiResult> {
  const { callLogs } = await readSyncSnapshot()
  const todayStart = normalizeDay(baseDate)
  const tomorrowStart = new Date(todayStart)
  tomorrowStart.setDate(tomorrowStart.getDate() + 1)

  const yesterdayStart = new Date(todayStart)
  yesterdayStart.setDate(yesterdayStart.getDate() - 1)

  const todayCalls = callLogs.filter((item) => {
    const started = new Date(item.startedAt)
    return started >= todayStart && started < tomorrowStart
  })
  const yesterdayCalls = callLogs.filter((item) => {
    const started = new Date(item.startedAt)
    return started >= yesterdayStart && started < todayStart
  })

  const today = calculateDayMetrics(todayCalls)
  const yesterday = calculateDayMetrics(yesterdayCalls)
  const activeCalls = callLogs.filter((item) => item.status === "ringing" || item.status === "in_call").length

  return {
    totalCalls: today.total,
    totalCallsChange: yesterday.total > 0 ? ((today.total - yesterday.total) / yesterday.total) * 100 : 0,
    avgDurationSec: Math.round(today.avgDurationSec),
    avgDurationChange:
      yesterday.avgDurationSec > 0
        ? ((today.avgDurationSec - yesterday.avgDurationSec) / yesterday.avgDurationSec) * 100
        : 0,
    answerRate: today.answerRate,
    answerRateChange: (today.answerRate - yesterday.answerRate) * 100,
    activeCalls,
  }
}
