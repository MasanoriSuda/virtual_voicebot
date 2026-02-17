import "server-only"

import { randomUUID } from "node:crypto"
import { promises as fs } from "node:fs"
import * as path from "node:path"

import {
  DEFAULT_MAX_RETRIES,
  DEFAULT_TIMEOUT_SEC,
  DTMF_KEYS,
  type DtmfKey,
  type IvrFallbackAction,
  type IvrFlowDefinition,
  type IvrFlowsDatabase,
  type IvrRoute,
  type IvrTerminalAction,
} from "@/lib/ivr-flows"

const STORAGE_ROOT = path.join(process.cwd(), "storage")
const DB_DIR = path.join(STORAGE_ROOT, "db")
const IVR_FLOWS_DB_FILE = path.join(DB_DIR, "ivr-flows.json")

let writeQueue: Promise<unknown> = Promise.resolve()

function withWriteLock<T>(fn: () => Promise<T>): Promise<T> {
  const run = writeQueue.then(fn, fn)
  writeQueue = run.then(
    () => undefined,
    () => undefined,
  )
  return run
}

function emptyDb(): IvrFlowsDatabase {
  return {
    flows: [],
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value)
}

function asTrimmedString(value: unknown): string | null {
  if (typeof value !== "string") {
    return null
  }
  const trimmed = value.trim()
  return trimmed.length > 0 ? trimmed : null
}

function asNullableString(value: unknown): string | null {
  if (value === null || value === undefined) {
    return null
  }
  return asTrimmedString(value)
}

function asNumber(value: unknown, fallback: number): number {
  if (typeof value === "number" && Number.isFinite(value)) {
    return value
  }
  if (typeof value === "string" && value.trim().length > 0) {
    const parsed = Number(value)
    if (Number.isFinite(parsed)) {
      return parsed
    }
  }
  return fallback
}

function asIso(value: unknown, fallback: string): string {
  if (typeof value !== "string") {
    return fallback
  }
  const parsed = Date.parse(value)
  if (Number.isNaN(parsed)) {
    return fallback
  }
  return new Date(parsed).toISOString()
}

function toDtmfKey(value: unknown): DtmfKey | null {
  const str = asTrimmedString(value)
  if (!str) {
    return null
  }
  return DTMF_KEYS.includes(str as DtmfKey) ? (str as DtmfKey) : null
}

function normalizeTerminalAction(raw: unknown): IvrTerminalAction {
  if (!isRecord(raw)) {
    return { actionCode: "VR" }
  }

  const actionCode = asTrimmedString(raw.actionCode)
  switch (actionCode) {
    case "VM":
      return {
        actionCode: "VM",
        announcementId: asNullableString(raw.announcementId),
      }
    case "AN":
      return {
        actionCode: "AN",
        announcementId: asNullableString(raw.announcementId),
      }
    case "IV": {
      const ivrFlowId = asTrimmedString(raw.ivrFlowId)
      if (!ivrFlowId) {
        return { actionCode: "VR" }
      }
      return {
        actionCode: "IV",
        ivrFlowId,
      }
    }
    case "VB": {
      const scenarioId = asTrimmedString(raw.scenarioId)
      if (!scenarioId) {
        return { actionCode: "VR" }
      }
      return {
        actionCode: "VB",
        scenarioId,
        welcomeAnnouncementId: asNullableString(raw.welcomeAnnouncementId),
        recordingEnabled: typeof raw.recordingEnabled === "boolean" ? raw.recordingEnabled : true,
        includeAnnouncement:
          typeof raw.includeAnnouncement === "boolean" ? raw.includeAnnouncement : false,
      }
    }
    case "VR":
    default:
      return { actionCode: "VR" }
  }
}

function normalizeFallbackAction(raw: unknown): IvrFallbackAction {
  if (!isRecord(raw)) {
    return { actionCode: "VR" }
  }

  const actionCode = asTrimmedString(raw.actionCode)
  switch (actionCode) {
    case "VM":
      return {
        actionCode: "VM",
        announcementId: asNullableString(raw.announcementId),
      }
    case "AN":
      return {
        actionCode: "AN",
        announcementId: asNullableString(raw.announcementId),
      }
    case "VB": {
      const scenarioId = asTrimmedString(raw.scenarioId)
      if (!scenarioId) {
        return { actionCode: "VR" }
      }
      return {
        actionCode: "VB",
        scenarioId,
        welcomeAnnouncementId: asNullableString(raw.welcomeAnnouncementId),
        recordingEnabled: typeof raw.recordingEnabled === "boolean" ? raw.recordingEnabled : true,
        includeAnnouncement:
          typeof raw.includeAnnouncement === "boolean" ? raw.includeAnnouncement : false,
      }
    }
    case "VR":
    default:
      return { actionCode: "VR" }
  }
}

function normalizeRoutes(input: unknown): IvrRoute[] {
  if (!Array.isArray(input)) {
    return []
  }

  const routes: IvrRoute[] = []
  for (const rawRoute of input) {
    if (!isRecord(rawRoute)) {
      continue
    }

    const dtmfKey = toDtmfKey(rawRoute.dtmfKey)
    if (!dtmfKey) {
      continue
    }

    routes.push({
      dtmfKey,
      label: asTrimmedString(rawRoute.label) ?? "",
      destination: normalizeTerminalAction(rawRoute.destination),
    })
  }

  return routes
}

function normalizeFlow(rawFlow: unknown, nowIso: string): IvrFlowDefinition | null {
  if (!isRecord(rawFlow)) {
    return null
  }

  const id = asTrimmedString(rawFlow.id) ?? randomUUID()
  const createdAt = asIso(rawFlow.createdAt, nowIso)

  return {
    id,
    name: asTrimmedString(rawFlow.name) ?? "",
    description: asNullableString(rawFlow.description),
    isActive: typeof rawFlow.isActive === "boolean" ? rawFlow.isActive : true,
    announcementId: asNullableString(rawFlow.announcementId),
    timeoutSec: asNumber(rawFlow.timeoutSec, DEFAULT_TIMEOUT_SEC),
    maxRetries: asNumber(rawFlow.maxRetries, DEFAULT_MAX_RETRIES),
    invalidInputAnnouncementId: asNullableString(rawFlow.invalidInputAnnouncementId),
    timeoutAnnouncementId: asNullableString(rawFlow.timeoutAnnouncementId),
    routes: normalizeRoutes(rawFlow.routes),
    fallbackAction: normalizeFallbackAction(rawFlow.fallbackAction),
    createdAt,
    updatedAt: asIso(rawFlow.updatedAt, createdAt),
  }
}

function normalizeDb(input: unknown): IvrFlowsDatabase {
  if (!isRecord(input) || !Array.isArray(input.flows)) {
    return emptyDb()
  }

  const nowIso = new Date().toISOString()
  const flows: IvrFlowDefinition[] = []
  for (const rawFlow of input.flows) {
    const normalized = normalizeFlow(rawFlow, nowIso)
    if (normalized) {
      flows.push(normalized)
    }
  }

  return { flows }
}

async function ensureDbDir() {
  await fs.mkdir(DB_DIR, { recursive: true })
}

async function writeDb(db: IvrFlowsDatabase): Promise<void> {
  await ensureDbDir()
  const tempFile = `${IVR_FLOWS_DB_FILE}.tmp`
  await fs.writeFile(tempFile, JSON.stringify(db, null, 2), "utf8")
  await fs.rename(tempFile, IVR_FLOWS_DB_FILE)
}

async function readDb(): Promise<IvrFlowsDatabase> {
  await ensureDbDir()
  try {
    const raw = await fs.readFile(IVR_FLOWS_DB_FILE, "utf8")
    return normalizeDb(JSON.parse(raw))
  } catch (error) {
    const err = error as NodeJS.ErrnoException
    if (err.code === "ENOENT") {
      const initial = emptyDb()
      await writeDb(initial)
      return initial
    }
    throw error
  }
}

export function parseIvrFlowsPayload(payload: unknown): IvrFlowsDatabase {
  if (!isRecord(payload) || !Array.isArray(payload.flows)) {
    throw new Error("flows must be an array")
  }

  return normalizeDb(payload)
}

export async function readIvrFlows(): Promise<IvrFlowsDatabase> {
  return readDb()
}

export async function writeIvrFlows(db: IvrFlowsDatabase): Promise<void> {
  return withWriteLock(async () => {
    const normalized = normalizeDb(db)
    await writeDb(normalized)
  })
}
