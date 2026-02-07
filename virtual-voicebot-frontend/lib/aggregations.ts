import { mockHourlyVolume, mockKPI } from "@/lib/mock-data"
import { queryHourlyStats, queryKpi, type HourlyStat, type KpiResult } from "@/lib/db/queries"

const USE_MOCK = process.env.NEXT_PUBLIC_USE_MOCK_DATA === "true"

export interface DashboardMetrics {
  kpi: KpiResult
  hourlyStats: HourlyStat[]
}

export async function getDashboardMetrics(baseDate: Date = new Date()): Promise<DashboardMetrics> {
  if (USE_MOCK) {
    return {
      kpi: {
        totalCalls: mockKPI.totalCalls,
        totalCallsChange: mockKPI.totalCallsChange,
        avgDurationSec: mockKPI.avgDurationSec,
        avgDurationChange: mockKPI.avgDurationChange,
        answerRate: mockKPI.answerRate,
        answerRateChange: mockKPI.answerRateChange,
        activeCalls: mockKPI.activeCalls,
      },
      hourlyStats: mockHourlyVolume.map((item) => ({
        hour: item.hour,
        inbound: item.inbound,
        outbound: item.outbound,
      })),
    }
  }

  const [kpi, hourlyStats] = await Promise.all([queryKpi(baseDate), queryHourlyStats(baseDate)])
  return { kpi, hourlyStats }
}
