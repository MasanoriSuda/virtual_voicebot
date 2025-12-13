import type { Call, CallDetail, Utterance } from "./types"

// Mock data
const mockCalls: Call[] = [
  {
    id: "1",
    from: "090-1234-5678",
    to: "0120-xxx-xxx",
    callerNumber: "090-1234-5678",
    startTime: "2025-01-15T10:30:00Z",
    duration: 180,
    durationSec: 180,
    status: "completed",
    summary: "お問い合わせ対応完了。配送状況の確認",
    recordingUrl: "/mock-audio.mp3", // Mock recording URL
  },
  {
    id: "2",
    from: "080-9876-5432",
    to: "0120-xxx-xxx",
    callerNumber: "080-9876-5432",
    startTime: "2025-01-15T11:15:00Z",
    duration: 120,
    durationSec: 120,
    status: "completed",
    summary: "商品の返品手続きについて案内",
    recordingUrl: "/mock-audio.mp3",
  },
  {
    id: "3",
    from: "070-5555-1234",
    to: "0120-xxx-xxx",
    callerNumber: "070-5555-1234",
    startTime: "2025-01-15T14:20:00Z",
    duration: 0,
    durationSec: 0,
    status: "active",
    summary: "通話中...",
    // No recordingUrl for active calls
  },
  {
    id: "4",
    from: "090-1111-2222",
    to: "0120-xxx-xxx",
    callerNumber: "090-1111-2222",
    startTime: "2025-01-14T09:00:00Z",
    duration: 90,
    durationSec: 90,
    status: "failed",
    summary: "接続エラー",
  },
]

const mockUtterances: Record<string, Utterance[]> = {
  "1": [
    {
      seq: 1,
      speaker: "bot",
      text: "お電話ありがとうございます。どのようなご用件でしょうか？",
      timestamp: "2025-01-15T10:30:05Z",
      isFinal: true,
      startSec: 5,
      endSec: 12,
    },
    {
      seq: 2,
      speaker: "caller",
      text: "配送状況を確認したいのですが",
      timestamp: "2025-01-15T10:30:15Z",
      isFinal: true,
      startSec: 15,
      endSec: 20,
    },
    {
      seq: 3,
      speaker: "bot",
      text: "かしこまりました。ご注文番号をお教えいただけますでしょうか？",
      timestamp: "2025-01-15T10:30:25Z",
      isFinal: true,
      startSec: 25,
      endSec: 33,
    },
    {
      seq: 4,
      speaker: "caller",
      text: "注文番号は12345です",
      timestamp: "2025-01-15T10:30:35Z",
      isFinal: true,
      startSec: 35,
      endSec: 40,
    },
    {
      seq: 5,
      speaker: "bot",
      text: "ご注文番号12345ですね。確認いたします。本日中に配達予定となっております。",
      timestamp: "2025-01-15T10:30:50Z",
      isFinal: true,
      startSec: 50,
      endSec: 62,
    },
    {
      seq: 6,
      speaker: "caller",
      text: "ありがとうございます",
      timestamp: "2025-01-15T10:31:00Z",
      isFinal: true,
      startSec: 70,
      endSec: 74,
    },
  ],
  "2": [
    {
      seq: 1,
      speaker: "bot",
      text: "お電話ありがとうございます。どのようなご用件でしょうか？",
      timestamp: "2025-01-15T11:15:05Z",
      isFinal: true,
      startSec: 5,
      endSec: 12,
    },
    {
      seq: 2,
      speaker: "caller",
      text: "商品を返品したいのですが",
      timestamp: "2025-01-15T11:15:15Z",
      isFinal: true,
      startSec: 15,
      endSec: 20,
    },
    {
      seq: 3,
      speaker: "bot",
      text: "承知いたしました。返品の手続きについてご案内いたします。",
      timestamp: "2025-01-15T11:15:25Z",
      isFinal: true,
      startSec: 25,
      endSec: 33,
    },
  ],
  "3": [
    {
      seq: 1,
      speaker: "bot",
      text: "お電話ありがとうございます。",
      timestamp: "2025-01-15T14:20:05Z",
      isFinal: true,
      startSec: 5,
      endSec: 8,
    },
    {
      seq: 2,
      speaker: "caller",
      text: "こんにちは",
      timestamp: "2025-01-15T14:20:15Z",
      isFinal: true,
      startSec: 15,
      endSec: 17,
    },
  ],
}

// API functions with mock implementation
export async function getCalls(): Promise<Call[]> {
  // Simulate API delay
  await new Promise((resolve) => setTimeout(resolve, 500))
  return mockCalls
}

export async function getCall(callId: string): Promise<Call | null> {
  await new Promise((resolve) => setTimeout(resolve, 300))
  const call = mockCalls.find((c) => c.id === callId)
  return call || null
}

export async function getCallDetail(callId: string): Promise<CallDetail | null> {
  // Simulate API delay
  await new Promise((resolve) => setTimeout(resolve, 300))

  const call = mockCalls.find((c) => c.id === callId)
  if (!call) return null

  return {
    ...call,
    utterances: mockUtterances[callId] || [],
  }
}

export async function getUtterances(callId: string): Promise<Utterance[]> {
  await new Promise((resolve) => setTimeout(resolve, 300))
  return mockUtterances[callId] || []
}

// Keep old name for backward compatibility
export async function getCallUtterances(callId: string): Promise<Utterance[]> {
  return getUtterances(callId)
}

export async function getRecordingUrl(callId: string): Promise<string | null> {
  await new Promise((resolve) => setTimeout(resolve, 200))
  const call = mockCalls.find((c) => c.id === callId)
  return call?.recordingUrl || null
}
