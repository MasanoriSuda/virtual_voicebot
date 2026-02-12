import "server-only"

import { promises as fs } from "node:fs"
import * as path from "node:path"

import {
  createActionConfig,
  createDefaultCallActionsDatabase,
  isAllowActionCode,
  isDenyActionCode,
  type ActionConfig,
  type CallActionType,
  type CallActionsDatabase,
  type IncomingRule,
  type StoredAction,
} from "@/lib/call-actions"

const STORAGE_ROOT = path.join(process.cwd(), "storage")
const DB_DIR = path.join(STORAGE_ROOT, "db")
const CALL_ACTIONS_DB_FILE = path.join(DB_DIR, "call-actions.json")

const UUID_RE =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i

let writeQueue: Promise<unknown> = Promise.resolve()

function withWriteLock<T>(fn: () => Promise<T>): Promise<T> {
  const run = writeQueue.then(fn, fn)
  writeQueue = run.then(
    () => undefined,
    () => undefined,
  )
  return run
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value)
}

function parseIsoOr(value: unknown, fallback: string): string {
  if (typeof value !== "string") {
    return fallback
  }
  const parsed = Date.parse(value)
  if (Number.isNaN(parsed)) {
    return fallback
  }
  return new Date(parsed).toISOString()
}

function normalizeNullableString(value: unknown): string | null {
  if (typeof value !== "string") {
    return null
  }
  const trimmed = value.trim()
  return trimmed.length > 0 ? trimmed : null
}

function normalizeActionConfig(actionType: CallActionType, raw: unknown): ActionConfig {
  if (!isRecord(raw)) {
    return createActionConfig(actionType)
  }

  const rawCode = typeof raw.actionCode === "string" ? raw.actionCode : undefined
  if (actionType === "allow") {
    if (!rawCode || !isAllowActionCode(rawCode)) {
      return createActionConfig("allow")
    }

    switch (rawCode) {
      case "VR":
        return {
          actionCode: "VR",
          recordingEnabled: typeof raw.recordingEnabled === "boolean" ? raw.recordingEnabled : false,
          announceEnabled: typeof raw.announceEnabled === "boolean" ? raw.announceEnabled : false,
          announcementId: normalizeNullableString(raw.announcementId),
        }
      case "IV":
        return {
          actionCode: "IV",
          ivrFlowId: normalizeNullableString(raw.ivrFlowId),
          includeAnnouncement:
            typeof raw.includeAnnouncement === "boolean" ? raw.includeAnnouncement : false,
        }
      case "VB":
        return {
          actionCode: "VB",
          scenarioId: typeof raw.scenarioId === "string" ? raw.scenarioId.trim() : "",
          welcomeAnnouncementId: normalizeNullableString(raw.welcomeAnnouncementId),
          recordingEnabled:
            typeof raw.recordingEnabled === "boolean" ? raw.recordingEnabled : true,
          announceEnabled:
            typeof raw.announceEnabled === "boolean"
              ? raw.announceEnabled
              : typeof raw.includeAnnouncement === "boolean"
                ? raw.includeAnnouncement
                : false,
        }
      case "VM":
      default:
        return {
          actionCode: "VM",
          announcementId: normalizeNullableString(raw.announcementId),
        }
    }
  }

  if (!rawCode || !isDenyActionCode(rawCode)) {
    return createActionConfig("deny")
  }

  switch (rawCode) {
    case "AN":
      return {
        actionCode: "AN",
        announcementId: normalizeNullableString(raw.announcementId),
      }
    case "NR":
      return { actionCode: "NR" }
    case "BZ":
    default:
      return { actionCode: "BZ" }
  }
}

function normalizeStoredAction(input: unknown, fallback: StoredAction): StoredAction {
  if (!isRecord(input)) {
    return fallback
  }

  const actionType: CallActionType = input.actionType === "deny" ? "deny" : "allow"
  return {
    actionType,
    actionConfig: normalizeActionConfig(actionType, input.actionConfig),
  }
}

function normalizeRules(input: unknown, nowIso: string): IncomingRule[] {
  if (!Array.isArray(input)) {
    return []
  }

  const rules: IncomingRule[] = []
  for (const item of input) {
    if (!isRecord(item)) {
      continue
    }
    if (typeof item.id !== "string" || item.id.trim().length === 0) {
      continue
    }
    if (typeof item.name !== "string" || item.name.trim().length === 0) {
      continue
    }
    if (typeof item.callerGroupId !== "string" || item.callerGroupId.trim().length === 0) {
      continue
    }

    const actionType: CallActionType = item.actionType === "deny" ? "deny" : "allow"
    const createdAt = parseIsoOr(item.createdAt, nowIso)

    rules.push({
      id: item.id,
      name: item.name.trim(),
      callerGroupId: item.callerGroupId.trim(),
      actionType,
      actionConfig: normalizeActionConfig(actionType, item.actionConfig),
      isActive: typeof item.isActive === "boolean" ? item.isActive : true,
      createdAt,
      updatedAt: parseIsoOr(item.updatedAt, createdAt),
    })
  }

  return rules
}

function isLegacyVbActionConfig(input: unknown): boolean {
  if (!isRecord(input) || input.actionCode !== "VB") {
    return false
  }
  return (
    typeof input.includeAnnouncement === "boolean" &&
    typeof input.announceEnabled !== "boolean"
  )
}

function hasLegacyVbAnnouncementFlag(input: unknown): boolean {
  if (!isRecord(input)) {
    return false
  }

  const hasLegacyStoredAction = (value: unknown): boolean => {
    if (!isRecord(value)) {
      return false
    }
    return isLegacyVbActionConfig(value.actionConfig)
  }

  if (hasLegacyStoredAction(input.anonymousAction) || hasLegacyStoredAction(input.defaultAction)) {
    return true
  }

  if (!Array.isArray(input.rules)) {
    return false
  }

  return input.rules.some((rule) => isRecord(rule) && isLegacyVbActionConfig(rule.actionConfig))
}

function normalizeDatabase(input: unknown): CallActionsDatabase {
  const defaults = createDefaultCallActionsDatabase()
  const nowIso = new Date().toISOString()
  if (!isRecord(input)) {
    return defaults
  }

  return {
    rules: normalizeRules(input.rules, nowIso),
    anonymousAction: normalizeStoredAction(input.anonymousAction, defaults.anonymousAction),
    defaultAction: normalizeStoredAction(input.defaultAction, defaults.defaultAction),
  }
}

async function ensureDbDir() {
  await fs.mkdir(DB_DIR, { recursive: true })
}

async function writeDb(db: CallActionsDatabase): Promise<void> {
  await ensureDbDir()
  const tempFile = `${CALL_ACTIONS_DB_FILE}.tmp`
  await fs.writeFile(tempFile, JSON.stringify(db, null, 2), "utf8")
  await fs.rename(tempFile, CALL_ACTIONS_DB_FILE)
}

async function readDb(): Promise<CallActionsDatabase> {
  await ensureDbDir()
  try {
    const raw = await fs.readFile(CALL_ACTIONS_DB_FILE, "utf8")
    const parsed = JSON.parse(raw)
    const normalized = normalizeDatabase(parsed)
    if (hasLegacyVbAnnouncementFlag(parsed)) {
      await writeDb(normalized)
    }
    return normalized
  } catch (error) {
    const err = error as NodeJS.ErrnoException
    if (err.code === "ENOENT") {
      const initial = createDefaultCallActionsDatabase()
      await writeDb(initial)
      return initial
    }
    throw error
  }
}

function requireRecord(value: unknown, field: string): Record<string, unknown> {
  if (!isRecord(value)) {
    throw new Error(`${field} must be an object`)
  }
  return value
}

function requireString(value: unknown, field: string): string {
  if (typeof value !== "string" || value.trim().length === 0) {
    throw new Error(`${field} is required`)
  }
  return value.trim()
}

function requireUuid(value: unknown, field: string): string {
  const str = requireString(value, field)
  if (!UUID_RE.test(str)) {
    throw new Error(`${field} must be UUID`)
  }
  return str
}

function optionalIso(value: unknown, fallback: string, field: string): string {
  if (value === undefined || value === null || value === "") {
    return fallback
  }
  if (typeof value !== "string") {
    throw new Error(`${field} must be ISO8601`)
  }
  const parsed = Date.parse(value)
  if (Number.isNaN(parsed)) {
    throw new Error(`${field} must be ISO8601`)
  }
  return new Date(parsed).toISOString()
}

function optionalNullableString(value: unknown): string | null {
  if (value === undefined || value === null) {
    return null
  }
  if (typeof value !== "string") {
    return null
  }
  const trimmed = value.trim()
  return trimmed.length > 0 ? trimmed : null
}

function parseActionConfigStrict(
  actionType: CallActionType,
  raw: unknown,
  fieldPrefix: string,
): ActionConfig {
  const record = requireRecord(raw, fieldPrefix)
  const actionCode = requireString(record.actionCode, `${fieldPrefix}.actionCode`)

  if (actionType === "allow") {
    if (!isAllowActionCode(actionCode)) {
      throw new Error(`${fieldPrefix}.actionCode is invalid for allow`)
    }

    switch (actionCode) {
      case "VR":
        return {
          actionCode: "VR",
          recordingEnabled:
            typeof record.recordingEnabled === "boolean" ? record.recordingEnabled : false,
          announceEnabled:
            typeof record.announceEnabled === "boolean" ? record.announceEnabled : false,
          announcementId: optionalNullableString(record.announcementId),
        }
      case "IV":
        return {
          actionCode: "IV",
          ivrFlowId: optionalNullableString(record.ivrFlowId),
          includeAnnouncement:
            typeof record.includeAnnouncement === "boolean" ? record.includeAnnouncement : false,
        }
      case "VB":
        return {
          actionCode: "VB",
          scenarioId: requireString(record.scenarioId, `${fieldPrefix}.scenarioId`),
          welcomeAnnouncementId: optionalNullableString(record.welcomeAnnouncementId),
          recordingEnabled:
            typeof record.recordingEnabled === "boolean" ? record.recordingEnabled : true,
          announceEnabled:
            typeof record.announceEnabled === "boolean"
              ? record.announceEnabled
              : typeof record.includeAnnouncement === "boolean"
                ? record.includeAnnouncement
                : false,
        }
      case "VM":
      default:
        return {
          actionCode: "VM",
          announcementId: optionalNullableString(record.announcementId),
        }
    }
  }

  if (!isDenyActionCode(actionCode)) {
    throw new Error(`${fieldPrefix}.actionCode is invalid for deny`)
  }

  switch (actionCode) {
    case "AN":
      return {
        actionCode: "AN",
        announcementId: optionalNullableString(record.announcementId),
      }
    case "NR":
      return { actionCode: "NR" }
    case "BZ":
    default:
      return { actionCode: "BZ" }
  }
}

function parseStoredActionStrict(input: unknown, fieldName: string): StoredAction {
  const action = requireRecord(input, fieldName)
  const actionTypeRaw = requireString(action.actionType, `${fieldName}.actionType`)
  if (actionTypeRaw !== "allow" && actionTypeRaw !== "deny") {
    throw new Error(`${fieldName}.actionType must be allow or deny`)
  }

  return {
    actionType: actionTypeRaw,
    actionConfig: parseActionConfigStrict(
      actionTypeRaw,
      action.actionConfig,
      `${fieldName}.actionConfig`,
    ),
  }
}

function parseRulesStrict(input: unknown, nowIso: string): IncomingRule[] {
  if (!Array.isArray(input)) {
    throw new Error("rules must be an array")
  }

  return input.map((rawRule, index) => {
    const rule = requireRecord(rawRule, `rules[${index}]`)

    const id = requireUuid(rule.id, `rules[${index}].id`)
    const name = requireString(rule.name, `rules[${index}].name`)
    const callerGroupId = requireUuid(rule.callerGroupId, `rules[${index}].callerGroupId`)

    const actionTypeRaw = requireString(rule.actionType, `rules[${index}].actionType`)
    if (actionTypeRaw !== "allow" && actionTypeRaw !== "deny") {
      throw new Error(`rules[${index}].actionType must be allow or deny`)
    }

    const createdAt = optionalIso(rule.createdAt, nowIso, `rules[${index}].createdAt`)
    const updatedAt = optionalIso(rule.updatedAt, createdAt, `rules[${index}].updatedAt`)

    return {
      id,
      name,
      callerGroupId,
      actionType: actionTypeRaw,
      actionConfig: parseActionConfigStrict(
        actionTypeRaw,
        rule.actionConfig,
        `rules[${index}].actionConfig`,
      ),
      isActive: typeof rule.isActive === "boolean" ? rule.isActive : true,
      createdAt,
      updatedAt,
    }
  })
}

export function parseCallActionsPayload(payload: unknown): CallActionsDatabase {
  const body = requireRecord(payload, "body")
  const nowIso = new Date().toISOString()

  return {
    rules: parseRulesStrict(body.rules, nowIso),
    anonymousAction: parseStoredActionStrict(body.anonymousAction, "anonymousAction"),
    defaultAction: parseStoredActionStrict(body.defaultAction, "defaultAction"),
  }
}

export async function readCallActions(): Promise<CallActionsDatabase> {
  return readDb()
}

export async function writeCallActions(db: CallActionsDatabase): Promise<void> {
  return withWriteLock(async () => {
    const normalized = normalizeDatabase(db)
    await writeDb(normalized)
  })
}
