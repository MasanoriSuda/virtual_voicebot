// ============================================================
// Canonical Types — contract.md v2 準拠
// Backend DB スキーマと 1:1 対応
// ============================================================

// --- Enums ---

export type CallStatus = "ringing" | "in_call" | "ended" | "error"

export type CallerCategory = "spam" | "registered" | "unknown" | "anonymous"

export type ActionCode = "VB" | "VR" | "NR" | "RJ" | "BZ" | "AN" | "AR" | "VM" | "IV"

export type EndReason = "normal" | "cancelled" | "rejected" | "timeout" | "error"

export type IvrNodeType = "ANNOUNCE" | "KEYPAD" | "FORWARD" | "TRANSFER" | "RECORD" | "EXIT"

export type IvrInputType = "DTMF" | "TIMEOUT" | "INVALID" | "COMPLETE"

export type RecordingType = "full_call" | "ivr_segment" | "voicemail" | "transfer" | "one_way"

export type UploadStatus = "local_only" | "uploading" | "uploaded" | "upload_failed"

export type ScheduleType = "business" | "holiday" | "special" | "override"

export type AnnouncementType = "greeting" | "hold" | "ivr" | "closed" | "recording_notice" | "custom"

export type ScheduleActionType = "route" | "voicemail" | "announcement" | "closed"

export type FolderEntityType = "phone_number" | "routing_rule" | "ivr_flow" | "schedule" | "announcement"

// --- Core DTOs ---

export interface Call {
  id: string
  externalCallId: string
  callerNumber: string | null
  callerCategory: CallerCategory
  actionCode: ActionCode
  status: CallStatus
  startedAt: string
  answeredAt: string | null
  endedAt: string | null
  durationSec: number | null
  endReason: EndReason
}

export interface Recording {
  id: string
  callLogId: string
  recordingType: RecordingType
  sequenceNumber: number
  recordingUrl: string
  durationSec: number | null
  format: "wav" | "mp3"
  fileSizeBytes: number | null
  startedAt: string
  endedAt: string | null
}

// --- Settings DTOs ---

export interface SpamNumber {
  id: string
  phoneNumber: string
  reason: string | null
  source: "manual" | "import" | "report"
  folderId: string | null
  createdAt: string
}

export interface RegisteredNumber {
  id: string
  phoneNumber: string
  name: string | null
  category: CallerCategory
  actionCode: ActionCode
  ivrFlowId: string | null
  recordingEnabled: boolean
  announceEnabled: boolean
  notes: string | null
  folderId: string | null
  version: number
  createdAt: string
  updatedAt: string
}

export interface RoutingRule {
  id: string
  callerCategory: CallerCategory
  actionCode: ActionCode
  ivrFlowId: string | null
  priority: number
  isActive: boolean
  folderId: string | null
  version: number
  createdAt: string
  updatedAt: string
}

// --- IVR DTOs ---

export interface IvrFlow {
  id: string
  name: string
  description: string | null
  isActive: boolean
  folderId: string | null
  nodes: IvrNode[]
  createdAt: string
  updatedAt: string
}

export interface IvrNode {
  id: string
  flowId: string
  parentId: string | null
  nodeType: IvrNodeType
  actionCode: string | null
  audioFileUrl: string | null
  ttsText: string | null
  timeoutSec: number
  maxRetries: number
  depth: number
  exitAction: string
  transitions: IvrTransition[]
}

export interface IvrTransition {
  id: string
  fromNodeId: string
  inputType: IvrInputType
  dtmfKey: string | null
  toNodeId: string | null
}

// --- Schedule DTOs ---

export interface Schedule {
  id: string
  name: string
  description: string | null
  scheduleType: ScheduleType
  isActive: boolean
  folderId: string | null
  dateRangeStart: string | null
  dateRangeEnd: string | null
  actionType: ScheduleActionType
  actionTarget: string | null
  actionCode: string | null
  timeSlots: ScheduleTimeSlot[]
  version: number
  createdAt: string
  updatedAt: string
}

export interface ScheduleTimeSlot {
  id: string
  dayOfWeek: number | null
  startTime: string
  endTime: string
}

// --- Announcement DTOs ---

export interface Announcement {
  id: string
  name: string
  description: string | null
  announcementType: AnnouncementType
  isActive: boolean
  folderId: string | null
  audioFileUrl: string | null
  ttsText: string | null
  durationSec: number | null
  language: string
  version: number
  createdAt: string
  updatedAt: string
}

// --- Folder DTO ---

export interface Folder {
  id: string
  parentId: string | null
  entityType: FolderEntityType
  name: string
  description: string | null
  sortOrder: number
}

// --- System Settings DTO ---

export interface SystemSettings {
  recordingRetentionDays: number
  historyRetentionDays: number
  syncEndpointUrl: string | null
  defaultActionCode: ActionCode
  maxConcurrentCalls: number
  extra: Record<string, unknown>
  version: number
}

// ============================================================
// Transitional Compatibility Types
// - Existing UI modules still depend on these legacy shapes.
// - Remove after UI migration is completed.
// ============================================================

export interface Utterance {
  seq: number
  speaker: "caller" | "bot" | "system"
  text: string
  timestamp: string
  isFinal: boolean
  startSec?: number
  endSec?: number
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

export interface CallDetail extends Call {
  from: string
  to: string
  startTime: string
  duration: number
  summary: string
  recordingUrl?: string
  utterances: Utterance[]
}

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

export type LegacyRoutingRuleType = "time" | "caller" | "ivr" | "overflow" | "default"

export interface LegacyRoutingRule {
  id: string
  name: string
  description?: string
  type: LegacyRoutingRuleType
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

export interface LegacyRoutingFolder {
  id: string
  name: string
  description?: string
  parentId: string | null
  type: "folder" | "route"
  children?: LegacyRoutingFolder[]
  rules?: LegacyRoutingRule[]
  createdAt: string
  updatedAt: string
}

export interface LegacyTimeSlot {
  start: string
  end: string
}

export interface LegacySchedule {
  id: string
  name: string
  description?: string
  type: ScheduleType
  enabled: boolean
  daysOfWeek?: number[]
  dateRange?: { start: string; end: string }
  timeSlots: LegacyTimeSlot[]
  action: {
    type: ScheduleActionType
    target?: string
  }
}

export interface LegacyScheduleFolder {
  id: string
  name: string
  description?: string
  parentId: string | null
  type: "folder" | "schedule"
  children?: LegacyScheduleFolder[]
  schedules?: LegacySchedule[]
  createdAt: string
  updatedAt: string
}

export interface LegacyAnnouncement {
  id: string
  name: string
  description?: string
  type: AnnouncementType
  enabled: boolean
  audioUrl?: string
  textToSpeech?: string
  duration?: number
  language: string
}

export interface LegacyAnnouncementFolder {
  id: string
  name: string
  description?: string
  parentId: string | null
  type: "folder" | "announcement"
  children?: LegacyAnnouncementFolder[]
  announcements?: LegacyAnnouncement[]
  createdAt: string
  updatedAt: string
}
