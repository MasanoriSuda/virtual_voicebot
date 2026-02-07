import "server-only"

import { promises as fs } from "node:fs"
import * as path from "node:path"

import {
  createDefaultNumberGroupsDatabase,
  normalizePhoneNumber,
  type CallerGroup,
  type NumberGroupsDatabase,
} from "@/lib/call-actions"

const STORAGE_ROOT = path.join(process.cwd(), "storage")
const DB_DIR = path.join(STORAGE_ROOT, "db")
const NUMBER_GROUPS_DB_FILE = path.join(DB_DIR, "number-groups.json")

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

function normalizeCallerGroups(input: unknown, nowIso: string): CallerGroup[] {
  if (!Array.isArray(input)) {
    return []
  }

  const callerGroups: CallerGroup[] = []
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

    const normalizedNumbers = Array.isArray(item.phoneNumbers)
      ? item.phoneNumbers
          .filter((value): value is string => typeof value === "string")
          .map((value) => normalizePhoneNumber(value))
          .filter((value) => value.length > 0)
      : []

    const createdAt = parseIsoOr(item.createdAt, nowIso)
    const updatedAt = parseIsoOr(item.updatedAt, createdAt)

    callerGroups.push({
      id: item.id,
      name: item.name.trim(),
      description: normalizeNullableString(item.description),
      phoneNumbers: Array.from(new Set(normalizedNumbers)),
      createdAt,
      updatedAt,
    })
  }

  return callerGroups
}

function normalizeDatabase(input: unknown): NumberGroupsDatabase {
  const nowIso = new Date().toISOString()
  if (!isRecord(input)) {
    return createDefaultNumberGroupsDatabase()
  }

  return {
    callerGroups: normalizeCallerGroups(input.callerGroups, nowIso),
  }
}

async function ensureDbDir() {
  await fs.mkdir(DB_DIR, { recursive: true })
}

async function writeDb(db: NumberGroupsDatabase): Promise<void> {
  await ensureDbDir()
  const tempFile = `${NUMBER_GROUPS_DB_FILE}.tmp`
  await fs.writeFile(tempFile, JSON.stringify(db, null, 2), "utf8")
  await fs.rename(tempFile, NUMBER_GROUPS_DB_FILE)
}

async function readDb(): Promise<NumberGroupsDatabase> {
  await ensureDbDir()
  try {
    const raw = await fs.readFile(NUMBER_GROUPS_DB_FILE, "utf8")
    return normalizeDatabase(JSON.parse(raw))
  } catch (error) {
    const err = error as NodeJS.ErrnoException
    if (err.code === "ENOENT") {
      const initial = createDefaultNumberGroupsDatabase()
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

function parseCallerGroupsStrict(input: unknown, nowIso: string): CallerGroup[] {
  if (!Array.isArray(input)) {
    throw new Error("callerGroups must be an array")
  }

  return input.map((rawGroup, index) => {
    const group = requireRecord(rawGroup, `callerGroups[${index}]`)
    const id = requireUuid(group.id, `callerGroups[${index}].id`)
    const name = requireString(group.name, `callerGroups[${index}].name`)

    let phoneNumbers: string[] = []
    if (group.phoneNumbers !== undefined) {
      if (!Array.isArray(group.phoneNumbers)) {
        throw new Error(`callerGroups[${index}].phoneNumbers must be an array`)
      }

      phoneNumbers = group.phoneNumbers.map((rawPhoneNumber, phoneIndex) => {
        if (typeof rawPhoneNumber !== "string") {
          throw new Error(`callerGroups[${index}].phoneNumbers[${phoneIndex}] must be string`)
        }
        const normalized = normalizePhoneNumber(rawPhoneNumber)
        if (normalized.length === 0) {
          throw new Error(`callerGroups[${index}].phoneNumbers[${phoneIndex}] is invalid`)
        }
        return normalized
      })
    }

    const createdAt = optionalIso(group.createdAt, nowIso, `callerGroups[${index}].createdAt`)
    const updatedAt = optionalIso(group.updatedAt, createdAt, `callerGroups[${index}].updatedAt`)

    return {
      id,
      name,
      description: optionalNullableString(group.description),
      phoneNumbers: Array.from(new Set(phoneNumbers)),
      createdAt,
      updatedAt,
    }
  })
}

export function parseNumberGroupsPayload(payload: unknown): NumberGroupsDatabase {
  const body = requireRecord(payload, "body")
  const nowIso = new Date().toISOString()
  const callerGroups = parseCallerGroupsStrict(body.callerGroups, nowIso)
  return { callerGroups }
}

export async function readNumberGroups(): Promise<NumberGroupsDatabase> {
  return readDb()
}

export async function writeNumberGroups(db: NumberGroupsDatabase): Promise<void> {
  return withWriteLock(async () => {
    const normalized = normalizeDatabase(db)
    await writeDb(normalized)
  })
}
