export type CallStatus = "active" | "completed" | "failed"

export interface Call {
  id: string
  from: string // caller number
  fromName?: string
  to: string // bot/service number
  callerNumber: string // for backward compatibility
  callId?: string
  direction?: "inbound" | "outbound" | "missed"
  startTime: string
  endedAt?: string | null
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

// Number Group Types
export interface PhoneNumber {
  id: string
  number: string
  label: string
  status: "active" | "inactive"
  assignedTo?: string
}

export interface NumberGroup {
  id: string
  name: string
  description?: string
  parentId: string | null
  type: "folder" | "group"
  children?: NumberGroup[]
  numbers?: PhoneNumber[]
  createdAt: string
  updatedAt: string
}

// Routing Types
export type RoutingRuleType = "time" | "caller" | "ivr" | "overflow" | "default"

export interface RoutingRule {
  id: string
  name: string
  description?: string
  type: RoutingRuleType
  enabled: boolean
  priority: number
  conditions?: {
    timeRange?: { start: string; end: string }
    daysOfWeek?: number[]
    callerPatterns?: string[]
  }
  destination: {
    type: "group" | "number" | "voicemail" | "ivr"
    target: string
  }
}

export interface RoutingFolder {
  id: string
  name: string
  description?: string
  parentId: string | null
  type: "folder" | "route"
  children?: RoutingFolder[]
  rules?: RoutingRule[]
  createdAt: string
  updatedAt: string
}

// IVR Types
export type IvrNodeType =
  | "start"
  | "menu"
  | "input"
  | "playback"
  | "transfer"
  | "voicemail"
  | "hangup"
  | "condition"

export interface IvrNode {
  id: string
  type: IvrNodeType
  name: string
  description?: string
  config?: {
    audioFile?: string
    prompt?: string
    timeout?: number
    maxRetries?: number
    options?: { key: string; label: string; nextNodeId: string }[]
    transferTarget?: string
    condition?: string
  }
}

export interface IvrFlow {
  id: string
  name: string
  description?: string
  enabled: boolean
  nodes: IvrNode[]
  createdAt: string
  updatedAt: string
}

export interface IvrFolder {
  id: string
  name: string
  description?: string
  parentId: string | null
  type: "folder" | "ivr"
  children?: IvrFolder[]
  flows?: IvrFlow[]
  createdAt: string
  updatedAt: string
}

// Schedule Types
export type ScheduleType = "business" | "holiday" | "special" | "override"

export interface TimeSlot {
  start: string // HH:mm format
  end: string // HH:mm format
}

export interface Schedule {
  id: string
  name: string
  description?: string
  type: ScheduleType
  enabled: boolean
  daysOfWeek?: number[] // 0-6, Sunday-Saturday
  dateRange?: { start: string; end: string }
  timeSlots: TimeSlot[]
  action: {
    type: "route" | "voicemail" | "announcement" | "closed"
    target?: string
  }
}

export interface ScheduleFolder {
  id: string
  name: string
  description?: string
  parentId: string | null
  type: "folder" | "schedule"
  children?: ScheduleFolder[]
  schedules?: Schedule[]
  createdAt: string
  updatedAt: string
}

// Announcement Types
export type AnnouncementType = "greeting" | "hold" | "ivr" | "closed" | "custom"

export interface Announcement {
  id: string
  name: string
  description?: string
  type: AnnouncementType
  enabled: boolean
  audioUrl?: string
  textToSpeech?: string
  duration?: number // in seconds
  language: string
}

export interface AnnouncementFolder {
  id: string
  name: string
  description?: string
  parentId: string | null
  type: "folder" | "announcement"
  children?: AnnouncementFolder[]
  announcements?: Announcement[]
  createdAt: string
  updatedAt: string
}
