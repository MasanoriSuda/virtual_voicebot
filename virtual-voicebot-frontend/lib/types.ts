export type CallStatus = "active" | "completed" | "failed"

export interface Call {
  id: string
  from: string // caller number
  to: string // bot/service number
  callerNumber: string // for backward compatibility
  startTime: string
  duration: number // in seconds
  durationSec: number // explicit duration in seconds
  status: CallStatus
  summary: string
  recordingUrl?: string // URL to the recording audio file
}

export interface Utterance {
  seq: number
  speaker: "caller" | "bot" | "system"
  text: string
  timestamp: string
  isFinal: boolean
  startSec?: number // start time in the recording
  endSec?: number // end time in the recording
}

export interface CallDetail {
  id: string
  from: string
  to: string
  callerNumber: string
  startTime: string
  duration: number
  durationSec: number
  status: CallStatus
  summary: string
  recordingUrl?: string
  utterances: Utterance[]
}

export type WebSocketMessage =
  | {
      type: "utterance.partial"
      seq: number
      speaker: "caller" | "bot" | "system"
      text: string
      timestamp: string
      startSec?: number
      endSec?: number
    }
  | {
      type: "utterance.final"
      seq: number
      speaker: "caller" | "bot" | "system"
      text: string
      timestamp: string
      startSec?: number
      endSec?: number
    }
  | {
      type: "summary.updated"
      summary: string
    }
