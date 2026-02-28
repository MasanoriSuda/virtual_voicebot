import { promises as fs } from "node:fs"
import * as path from "node:path"

const STORAGE_ROOT = path.join(process.cwd(), "storage")
const NOTIFICATIONS_DIR = path.join(STORAGE_ROOT, "notifications")
const NOTIFICATIONS_FILE = path.join(NOTIFICATIONS_DIR, "notifications.json")

export type IncomingCallTrigger = "direct" | "ivr_transfer"

export interface IncomingCallIvrData {
  dwellTimeSec: number
  dtmfHistory: string[]
}

export interface IncomingCallNotification {
  id: string
  callerNumber: string
  trigger: IncomingCallTrigger
  receivedAt: string
  ivrData: IncomingCallIvrData | null
}

export interface IncomingCallNotificationInput {
  callerNumber: string
  trigger: IncomingCallTrigger
  receivedAt: string
  ivrData?: IncomingCallIvrData | null
}

let writeQueue: Promise<unknown> = Promise.resolve()

function withWriteLock<T>(fn: () => Promise<T>): Promise<T> {
  const run = writeQueue.then(fn, fn)
  writeQueue = run.then(
    () => undefined,
    () => undefined,
  )
  return run
}

async function ensureNotificationsDir(): Promise<void> {
  await fs.mkdir(NOTIFICATIONS_DIR, { recursive: true })
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value)
}

function normalizeReceivedAt(value: unknown): string {
  if (typeof value === "string" && !Number.isNaN(Date.parse(value))) {
    return new Date(value).toISOString()
  }
  return new Date().toISOString()
}

function normalizeTrigger(value: unknown): IncomingCallTrigger {
  return value === "ivr_transfer" ? "ivr_transfer" : "direct"
}

function normalizeIvrData(
  trigger: IncomingCallTrigger,
  value: unknown,
): IncomingCallIvrData | null {
  if (trigger !== "ivr_transfer") {
    return null
  }
  if (!isRecord(value)) {
    return {
      dwellTimeSec: 0,
      dtmfHistory: [],
    }
  }
  const dwellRaw = value.dwellTimeSec
  const dwellTimeSec =
    typeof dwellRaw === "number" && Number.isFinite(dwellRaw) && dwellRaw >= 0
      ? Math.floor(dwellRaw)
      : 0
  const historyRaw = value.dtmfHistory
  const dtmfHistory = Array.isArray(historyRaw)
    ? historyRaw.filter((item): item is string => typeof item === "string")
    : []
  return {
    dwellTimeSec,
    dtmfHistory,
  }
}

function normalizeNotification(value: unknown): IncomingCallNotification | null {
  if (!isRecord(value)) {
    return null
  }
  const id = typeof value.id === "string" ? value.id : ""
  if (id.trim() === "") {
    return null
  }
  const callerNumber = typeof value.callerNumber === "string" ? value.callerNumber : "unknown"
  const trigger = normalizeTrigger(value.trigger)
  return {
    id,
    callerNumber,
    trigger,
    receivedAt: normalizeReceivedAt(value.receivedAt),
    ivrData: normalizeIvrData(trigger, value.ivrData),
  }
}

async function readNotificationsUnsafe(): Promise<IncomingCallNotification[]> {
  await ensureNotificationsDir()
  try {
    const raw = await fs.readFile(NOTIFICATIONS_FILE, "utf8")
    const parsed = JSON.parse(raw)
    if (!Array.isArray(parsed)) {
      return []
    }
    return parsed
      .map((item) => normalizeNotification(item))
      .filter((item): item is IncomingCallNotification => item !== null)
  } catch (error) {
    const err = error as NodeJS.ErrnoException
    if (err.code === "ENOENT") {
      return []
    }
    throw error
  }
}

async function writeNotificationsUnsafe(
  notifications: IncomingCallNotification[],
): Promise<void> {
  await ensureNotificationsDir()
  const tempFile = `${NOTIFICATIONS_FILE}.tmp`
  await fs.writeFile(tempFile, JSON.stringify(notifications, null, 2), "utf8")
  await fs.rename(tempFile, NOTIFICATIONS_FILE)
}

function normalizeInput(input: IncomingCallNotificationInput): Omit<IncomingCallNotification, "id"> {
  const caller = input.callerNumber.trim()
  const callerNumber = caller === "" ? "unknown" : caller
  const trigger = input.trigger === "ivr_transfer" ? "ivr_transfer" : "direct"
  return {
    callerNumber,
    trigger,
    receivedAt: normalizeReceivedAt(input.receivedAt),
    ivrData: normalizeIvrData(trigger, input.ivrData),
  }
}

export async function addIncomingCallNotification(
  input: IncomingCallNotificationInput,
): Promise<IncomingCallNotification> {
  return withWriteLock(async () => {
    const notifications = await readNotificationsUnsafe()
    const normalized = normalizeInput(input)
    const entry: IncomingCallNotification = {
      id: crypto.randomUUID(),
      callerNumber: normalized.callerNumber,
      trigger: normalized.trigger,
      receivedAt: normalized.receivedAt,
      ivrData: normalized.ivrData,
    }
    notifications.push(entry)
    await writeNotificationsUnsafe(notifications)
    return entry
  })
}

export async function listIncomingCallNotifications(): Promise<IncomingCallNotification[]> {
  const notifications = await readNotificationsUnsafe()
  return [...notifications].sort((a, b) => Date.parse(b.receivedAt) - Date.parse(a.receivedAt))
}

export async function deleteIncomingCallNotification(id: string): Promise<boolean> {
  return withWriteLock(async () => {
    const notifications = await readNotificationsUnsafe()
    const next = notifications.filter((item) => item.id !== id)
    if (next.length === notifications.length) {
      return false
    }
    await writeNotificationsUnsafe(next)
    return true
  })
}
