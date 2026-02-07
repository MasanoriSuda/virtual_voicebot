import type { Call, CallDetail, Utterance } from "./types"
import { mockCalls, mockCallPresentationById } from "./mock-data"

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
    {
      seq: 3,
      speaker: "bot",
      text: "かしこまりました。ご注文番号をお教えいただけますでしょうか？",
      timestamp: "2026-02-02T10:30:25Z",
      isFinal: true,
      startSec: 25,
      endSec: 33,
    },
    {
      seq: 4,
      speaker: "caller",
      text: "注文番号は12345です",
      timestamp: "2026-02-02T10:30:35Z",
      isFinal: true,
      startSec: 35,
      endSec: 40,
    },
  ],
  "2": [
    {
      seq: 1,
      speaker: "bot",
      text: "お電話ありがとうございます。どのようなご用件でしょうか？",
      timestamp: "2026-02-02T09:05:05Z",
      isFinal: true,
      startSec: 5,
      endSec: 12,
    },
    {
      seq: 2,
      speaker: "caller",
      text: "商品を返品したいのですが",
      timestamp: "2026-02-02T09:05:15Z",
      isFinal: true,
      startSec: 15,
      endSec: 20,
    },
    {
      seq: 3,
      speaker: "bot",
      text: "承知いたしました。返品の手続きについてご案内いたします。",
      timestamp: "2026-02-02T09:05:25Z",
      isFinal: true,
      startSec: 25,
      endSec: 33,
    },
  ],
}

function toCallDetail(call: Call): CallDetail {
  const view = mockCallPresentationById[call.id]
  return {
    ...call,
    from: call.callerNumber ?? "非通知",
    to: view?.to ?? "未設定",
    startTime: call.startedAt,
    duration: call.durationSec ?? 0,
    summary: view?.summary ?? "",
    recordingUrl: view?.recordingUrl ?? undefined,
    utterances: mockUtterances[call.id] || [],
  }
}

export async function getCalls(): Promise<Call[]> {
  await new Promise((resolve) => setTimeout(resolve, 500))
  return mockCalls
}

export async function getCall(callId: string): Promise<Call | null> {
  await new Promise((resolve) => setTimeout(resolve, 300))
  const call = mockCalls.find((item) => item.id === callId)
  return call || null
}

export async function getCallDetail(callId: string): Promise<CallDetail | null> {
  await new Promise((resolve) => setTimeout(resolve, 300))

  const call = mockCalls.find((item) => item.id === callId)
  if (!call) return null

  return toCallDetail(call)
}

export async function getUtterances(callId: string): Promise<Utterance[]> {
  await new Promise((resolve) => setTimeout(resolve, 300))
  return mockUtterances[callId] || []
}

export async function getCallUtterances(callId: string): Promise<Utterance[]> {
  return getUtterances(callId)
}

export async function getRecordingUrl(callId: string): Promise<string | null> {
  await new Promise((resolve) => setTimeout(resolve, 200))
  return mockCallPresentationById[callId]?.recordingUrl ?? null
}
