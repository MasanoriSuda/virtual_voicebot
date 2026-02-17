import type {
  ActionCode,
  Call,
  CallDisposition,
  CallStatus,
  CallerCategory,
  FinalAction,
  TransferStatus,
} from "./types"

export type CallDirection = "inbound" | "outbound" | "missed"

export type CallRecordStatus = "ended" | "missed" | "in_call"

export type CallRecord = {
  id: string
  callId: string
  actionCode: ActionCode
  from: string
  fromName: string
  to: string
  startedAt: string
  endedAt: string | null
  status: CallRecordStatus
  callDisposition: CallDisposition
  finalAction: FinalAction | null
  transferStatus: TransferStatus
  durationSec: number
  summary: string
  recordingUrl: string | null
  direction: CallDirection
}

type CallPresentation = {
  fromName: string
  to: string
  summary: string
  recordingUrl: string | null
  direction: CallDirection
}

export const mockCalls: Call[] = [
  {
    id: "1",
    externalCallId: "c_001",
    callerNumber: "+81-90-1234-5678",
    callerCategory: "registered",
    actionCode: "VR",
    status: "ended",
    startedAt: "2026-02-02T10:30:00Z",
    answeredAt: "2026-02-02T10:30:05Z",
    endedAt: "2026-02-02T10:35:00Z",
    durationSec: 300,
    endReason: "normal",
    callDisposition: "allowed",
    finalAction: "normal_call",
    transferStatus: "answered",
    transferStartedAt: "2026-02-02T10:30:08Z",
    transferAnsweredAt: "2026-02-02T10:30:12Z",
    transferEndedAt: "2026-02-02T10:35:00Z",
  },
  {
    id: "2",
    externalCallId: "c_002",
    callerNumber: "+81-80-2222-1111",
    callerCategory: "registered",
    actionCode: "IV",
    status: "ended",
    startedAt: "2026-02-02T09:05:00Z",
    answeredAt: "2026-02-02T09:05:03Z",
    endedAt: "2026-02-02T09:07:45Z",
    durationSec: 165,
    endReason: "normal",
    callDisposition: "allowed",
    finalAction: "ivr",
    transferStatus: "none",
    transferStartedAt: null,
    transferAnsweredAt: null,
    transferEndedAt: null,
  },
  {
    id: "3",
    externalCallId: "c_003",
    callerNumber: "+81-50-8888-1111",
    callerCategory: "unknown",
    actionCode: "IV",
    status: "in_call",
    startedAt: "2026-02-02T08:12:00Z",
    answeredAt: "2026-02-02T08:12:04Z",
    endedAt: null,
    durationSec: 72,
    endReason: "normal",
    callDisposition: "allowed",
    finalAction: "ivr",
    transferStatus: "trying",
    transferStartedAt: "2026-02-02T08:12:20Z",
    transferAnsweredAt: null,
    transferEndedAt: null,
  },
  {
    id: "4",
    externalCallId: "c_004",
    callerNumber: "+81-70-3333-2222",
    callerCategory: "registered",
    actionCode: "VR",
    status: "ended",
    startedAt: "2026-02-01T17:48:00Z",
    answeredAt: "2026-02-01T17:48:02Z",
    endedAt: "2026-02-01T17:50:10Z",
    durationSec: 130,
    endReason: "normal",
    callDisposition: "allowed",
    finalAction: "normal_call",
    transferStatus: "answered",
    transferStartedAt: "2026-02-01T17:48:10Z",
    transferAnsweredAt: "2026-02-01T17:48:20Z",
    transferEndedAt: "2026-02-01T17:50:10Z",
  },
  {
    id: "5",
    externalCallId: "c_005",
    callerNumber: "+81-90-9999-0001",
    callerCategory: "spam",
    actionCode: "RJ",
    status: "error",
    startedAt: "2026-02-01T16:20:00Z",
    answeredAt: null,
    endedAt: "2026-02-01T16:20:20Z",
    durationSec: 0,
    endReason: "rejected",
    callDisposition: "denied",
    finalAction: "rejected",
    transferStatus: "no_transfer",
    transferStartedAt: null,
    transferAnsweredAt: null,
    transferEndedAt: null,
  },
  {
    id: "6",
    externalCallId: "c_006",
    callerNumber: "+81-3-4444-5555",
    callerCategory: "registered",
    actionCode: "VR",
    status: "ended",
    startedAt: "2026-02-01T14:12:00Z",
    answeredAt: "2026-02-01T14:12:03Z",
    endedAt: "2026-02-01T14:18:20Z",
    durationSec: 380,
    endReason: "normal",
    callDisposition: "allowed",
    finalAction: "normal_call",
    transferStatus: "answered",
    transferStartedAt: "2026-02-01T14:12:08Z",
    transferAnsweredAt: "2026-02-01T14:12:15Z",
    transferEndedAt: "2026-02-01T14:18:20Z",
  },
  {
    id: "7",
    externalCallId: "c_007",
    callerNumber: "+81-90-5555-1212",
    callerCategory: "registered",
    actionCode: "VR",
    status: "ended",
    startedAt: "2026-01-31T11:05:00Z",
    answeredAt: "2026-01-31T11:05:04Z",
    endedAt: "2026-01-31T11:08:00Z",
    durationSec: 180,
    endReason: "normal",
    callDisposition: "allowed",
    finalAction: "normal_call",
    transferStatus: "answered",
    transferStartedAt: "2026-01-31T11:05:12Z",
    transferAnsweredAt: "2026-01-31T11:05:20Z",
    transferEndedAt: "2026-01-31T11:08:00Z",
  },
  {
    id: "8",
    externalCallId: "c_008",
    callerNumber: "+81-90-7777-8888",
    callerCategory: "registered",
    actionCode: "VR",
    status: "ended",
    startedAt: "2026-01-31T09:32:00Z",
    answeredAt: "2026-01-31T09:32:02Z",
    endedAt: "2026-01-31T09:36:30Z",
    durationSec: 270,
    endReason: "normal",
    callDisposition: "allowed",
    finalAction: "normal_call",
    transferStatus: "answered",
    transferStartedAt: "2026-01-31T09:32:08Z",
    transferAnsweredAt: "2026-01-31T09:32:14Z",
    transferEndedAt: "2026-01-31T09:36:30Z",
  },
  {
    id: "9",
    externalCallId: "c_009",
    callerNumber: "+81-80-2323-4545",
    callerCategory: "registered",
    actionCode: "IV",
    status: "ended",
    startedAt: "2026-01-30T18:40:00Z",
    answeredAt: "2026-01-30T18:40:06Z",
    endedAt: "2026-01-30T18:42:50Z",
    durationSec: 170,
    endReason: "normal",
    callDisposition: "allowed",
    finalAction: "ivr",
    transferStatus: "none",
    transferStartedAt: null,
    transferAnsweredAt: null,
    transferEndedAt: null,
  },
  {
    id: "10",
    externalCallId: "c_010",
    callerNumber: "+81-90-1111-2222",
    callerCategory: "registered",
    actionCode: "VR",
    status: "ringing",
    startedAt: "2026-01-30T15:22:00Z",
    answeredAt: null,
    endedAt: null,
    durationSec: null,
    endReason: "timeout",
    callDisposition: "no_answer",
    finalAction: null,
    transferStatus: "no_transfer",
    transferStartedAt: null,
    transferAnsweredAt: null,
    transferEndedAt: null,
  },
]

export const mockCallPresentationById: Record<string, CallPresentation> = {
  "1": {
    fromName: "田中太郎",
    to: "+81-3-1234-5678",
    summary: "配送状況の確認。住所変更あり。",
    recordingUrl: "/mock/recording.wav",
    direction: "inbound",
  },
  "2": {
    fromName: "佐藤花子",
    to: "+81-3-1234-5678",
    summary: "IVRで担当部署へ転送。",
    recordingUrl: "/mock/recording.wav",
    direction: "inbound",
  },
  "3": {
    fromName: "株式会社アーク",
    to: "+81-3-1234-5678",
    summary: "契約更新の問い合わせ。",
    recordingUrl: null,
    direction: "inbound",
  },
  "4": {
    fromName: "山本一郎",
    to: "+81-3-1234-5678",
    summary: "不在着信の折り返し。",
    recordingUrl: "/mock/recording.wav",
    direction: "outbound",
  },
  "5": {
    fromName: "匿名",
    to: "+81-3-1234-5678",
    summary: "迷惑電話。",
    recordingUrl: null,
    direction: "missed",
  },
  "6": {
    fromName: "株式会社ミドリ",
    to: "+81-3-1234-5678",
    summary: "請求書の送付依頼。",
    recordingUrl: "/mock/recording.wav",
    direction: "inbound",
  },
  "7": {
    fromName: "川村莉子",
    to: "+81-3-1234-5678",
    summary: "納期確認。",
    recordingUrl: "/mock/recording.wav",
    direction: "inbound",
  },
  "8": {
    fromName: "高橋光",
    to: "+81-3-1234-5678",
    summary: "録音確認の依頼。",
    recordingUrl: null,
    direction: "outbound",
  },
  "9": {
    fromName: "森田梢",
    to: "+81-3-1234-5678",
    summary: "IVRの操作で担当者へ接続。",
    recordingUrl: "/mock/recording.wav",
    direction: "inbound",
  },
  "10": {
    fromName: "営業担当",
    to: "+81-3-1234-5678",
    summary: "新規案件の相談。",
    recordingUrl: null,
    direction: "inbound",
  },
}

export const mockCallRecords: CallRecord[] = mockCalls.map((call) => {
  const view = mockCallPresentationById[call.id]
  return {
    id: call.id,
    callId: call.externalCallId,
    actionCode: call.actionCode,
    from: call.callerNumber ?? "非通知",
    fromName: view?.fromName ?? categoryLabel(call.callerCategory),
    to: view?.to ?? "未設定",
    startedAt: call.startedAt,
    endedAt: call.endedAt,
    status: toCallRecordStatus(call.status),
    callDisposition: call.callDisposition,
    finalAction: call.finalAction,
    transferStatus: call.transferStatus,
    durationSec: call.durationSec ?? 0,
    summary: view?.summary ?? "",
    recordingUrl: view?.recordingUrl ?? null,
    direction: view?.direction ?? "inbound",
  }
})

function toCallRecordStatus(status: CallStatus): CallRecordStatus {
  switch (status) {
    case "in_call":
    case "ringing":
      return "in_call"
    case "error":
      return "missed"
    default:
      return "ended"
  }
}

function categoryLabel(category: CallerCategory): string {
  switch (category) {
    case "spam":
      return "迷惑電話"
    case "anonymous":
      return "匿名"
    case "registered":
      return "登録済み"
    default:
      return "未登録"
  }
}

export const mockKPI = {
  totalCalls: 142,
  totalCallsChange: 12.5,
  avgDurationSec: 154,
  avgDurationChange: -5.2,
  answerRate: 0.87,
  answerRateChange: 3.1,
  activeCalls: 3,
}

export const mockHourlyVolume = [
  { hour: 0, inbound: 2, outbound: 0 },
  { hour: 1, inbound: 1, outbound: 0 },
  { hour: 2, inbound: 0, outbound: 1 },
  { hour: 3, inbound: 0, outbound: 0 },
  { hour: 4, inbound: 0, outbound: 0 },
  { hour: 5, inbound: 1, outbound: 0 },
  { hour: 6, inbound: 2, outbound: 1 },
  { hour: 7, inbound: 3, outbound: 2 },
  { hour: 8, inbound: 7, outbound: 3 },
  { hour: 9, inbound: 12, outbound: 6 },
  { hour: 10, inbound: 14, outbound: 8 },
  { hour: 11, inbound: 10, outbound: 5 },
  { hour: 12, inbound: 8, outbound: 4 },
  { hour: 13, inbound: 9, outbound: 6 },
  { hour: 14, inbound: 11, outbound: 7 },
  { hour: 15, inbound: 13, outbound: 9 },
  { hour: 16, inbound: 9, outbound: 5 },
  { hour: 17, inbound: 6, outbound: 4 },
  { hour: 18, inbound: 5, outbound: 3 },
  { hour: 19, inbound: 4, outbound: 2 },
  { hour: 20, inbound: 3, outbound: 2 },
  { hour: 21, inbound: 2, outbound: 1 },
  { hour: 22, inbound: 2, outbound: 1 },
  { hour: 23, inbound: 1, outbound: 0 },
]

export const mockTranscript = [
  { time: "00:00", speaker: "A", text: "お電話ありがとうございます。" },
  { time: "00:05", speaker: "B", text: "配送状況を確認したいのですが。" },
  { time: "00:10", speaker: "A", text: "かしこまりました。お名前をお願いします。" },
]
