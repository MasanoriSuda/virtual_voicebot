import "server-only"

import { promises as fs } from "node:fs"
import * as path from "node:path"

import {
  createDefaultScenariosDatabase,
  type ScenariosDatabase,
  type VoicebotScenario,
} from "@/lib/scenarios"

const STORAGE_ROOT = path.join(process.cwd(), "storage")
const DB_DIR = path.join(STORAGE_ROOT, "db")
const SCENARIOS_DB_FILE = path.join(DB_DIR, "scenarios.json")

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

function normalizeScenario(rawScenario: unknown, nowIso: string): VoicebotScenario | null {
  if (!isRecord(rawScenario)) {
    return null
  }

  const id = asTrimmedString(rawScenario.id)
  const name = asTrimmedString(rawScenario.name)
  if (!id || !name) {
    return null
  }

  const createdAt = asIso(rawScenario.createdAt, nowIso)
  return {
    id,
    name,
    description: asNullableString(rawScenario.description),
    isActive: typeof rawScenario.isActive === "boolean" ? rawScenario.isActive : true,
    voicevoxStyleId: asNumber(rawScenario.voicevoxStyleId, 0),
    systemPrompt: asNullableString(rawScenario.systemPrompt),
    createdAt,
    updatedAt: asIso(rawScenario.updatedAt, createdAt),
  }
}

function normalizeDb(input: unknown): ScenariosDatabase {
  if (!isRecord(input) || !Array.isArray(input.scenarios)) {
    return createDefaultScenariosDatabase()
  }

  const nowIso = new Date().toISOString()
  const scenarios: VoicebotScenario[] = []
  for (const rawScenario of input.scenarios) {
    const normalized = normalizeScenario(rawScenario, nowIso)
    if (normalized) {
      scenarios.push(normalized)
    }
  }

  return { scenarios }
}

async function ensureDbDir() {
  await fs.mkdir(DB_DIR, { recursive: true })
}

export async function readScenariosDatabase(): Promise<ScenariosDatabase> {
  await ensureDbDir()
  try {
    const raw = await fs.readFile(SCENARIOS_DB_FILE, "utf8")
    return normalizeDb(JSON.parse(raw))
  } catch (error) {
    const err = error as NodeJS.ErrnoException
    if (err.code === "ENOENT") {
      return createDefaultScenariosDatabase()
    }
    throw error
  }
}
