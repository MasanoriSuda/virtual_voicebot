export type CallActionType = "allow" | "deny"

export type AllowActionCode = "VR" | "IV" | "VM" | "VB"
export type DenyActionCode = "BZ" | "NR" | "AN"
export type CallActionCode = AllowActionCode | DenyActionCode

export interface CallerGroup {
  id: string
  name: string
  description: string | null
  phoneNumbers: string[]
  createdAt: string
  updatedAt: string
}

export interface NumberGroupsDatabase {
  callerGroups: CallerGroup[]
}

export type AllowVR = {
  actionCode: "VR"
  recordingEnabled: boolean
  announceEnabled: boolean
  announcementId: string | null
}

export type AllowIV = {
  actionCode: "IV"
  ivrFlowId: string | null
  includeAnnouncement: boolean
}

export type AllowVM = {
  actionCode: "VM"
  announcementId: string | null
}

export type AllowVB = {
  actionCode: "VB"
  scenarioId: string
  welcomeAnnouncementId: string | null
  recordingEnabled: boolean
  announceEnabled: boolean
}

export type DenyBZ = {
  actionCode: "BZ"
}

export type DenyNR = {
  actionCode: "NR"
}

export type DenyAN = {
  actionCode: "AN"
  announcementId: string | null
}

export type ActionConfig = AllowVR | AllowIV | AllowVM | AllowVB | DenyBZ | DenyNR | DenyAN

export interface IncomingRule {
  id: string
  name: string
  callerGroupId: string
  actionType: CallActionType
  actionConfig: ActionConfig
  isActive: boolean
  createdAt: string
  updatedAt: string
}

export interface StoredAction {
  actionType: CallActionType
  actionConfig: ActionConfig
}

export interface CallActionsDatabase {
  rules: IncomingRule[]
  anonymousAction: StoredAction
  defaultAction: StoredAction
}

const PHONE_NORMALIZE_RE = /[-\s()（）]/g

const ALLOW_CODES: AllowActionCode[] = ["VR", "IV", "VM", "VB"]
const DENY_CODES: DenyActionCode[] = ["BZ", "NR", "AN"]

export function normalizePhoneNumber(raw: string): string {
  return raw.replace(PHONE_NORMALIZE_RE, "")
}

export function isAllowActionCode(value: string): value is AllowActionCode {
  return ALLOW_CODES.includes(value as AllowActionCode)
}

export function isDenyActionCode(value: string): value is DenyActionCode {
  return DENY_CODES.includes(value as DenyActionCode)
}

export function createActionConfig(
  actionType: CallActionType,
  actionCode?: CallActionCode,
): ActionConfig {
  if (actionType === "allow") {
    const code = actionCode && isAllowActionCode(actionCode) ? actionCode : "VR"
    switch (code) {
      case "IV":
        return {
          actionCode: "IV",
          ivrFlowId: null,
          includeAnnouncement: false,
        }
      case "VM":
        return {
          actionCode: "VM",
          announcementId: null,
        }
      case "VB":
        return {
          actionCode: "VB",
          scenarioId: "",
          welcomeAnnouncementId: null,
          recordingEnabled: true,
          announceEnabled: false,
        }
      case "VR":
      default:
        return {
          actionCode: "VR",
          recordingEnabled: false,
          announceEnabled: false,
          announcementId: null,
        }
    }
  }

  const code = actionCode && isDenyActionCode(actionCode) ? actionCode : "BZ"
  if (code === "AN") {
    return {
      actionCode: "AN",
      announcementId: null,
    }
  }
  if (code === "NR") {
    return { actionCode: "NR" }
  }
  return { actionCode: "BZ" }
}

export function createDefaultNumberGroupsDatabase(): NumberGroupsDatabase {
  return {
    callerGroups: [],
  }
}

export function createDefaultCallActionsDatabase(): CallActionsDatabase {
  return {
    rules: [],
    anonymousAction: {
      actionType: "deny",
      actionConfig: createActionConfig("deny", "BZ"),
    },
    defaultAction: {
      actionType: "allow",
      actionConfig: createActionConfig("allow", "VR"),
    },
  }
}

export function actionCodeLabel(actionCode: CallActionCode): string {
  switch (actionCode) {
    case "VR":
      return "通常着信"
    case "IV":
      return "IVR"
    case "VM":
      return "留守電"
    case "VB":
      return "ボイスボット"
    case "BZ":
      return "BUSY"
    case "NR":
      return "RING_FOREVER"
    case "AN":
      return "ANNOUNCE_AND_HANGUP"
    default:
      return actionCode
  }
}

export function actionTypeLabel(actionType: CallActionType): string {
  return actionType === "allow" ? "Allow" : "Deny"
}

export function cloneActionConfig(config: ActionConfig): ActionConfig {
  return JSON.parse(JSON.stringify(config)) as ActionConfig
}
