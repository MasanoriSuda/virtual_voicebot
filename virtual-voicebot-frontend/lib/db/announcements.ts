import "server-only"

import { randomUUID } from "node:crypto"
import { promises as fs } from "node:fs"
import * as path from "node:path"

import type { AnnouncementType } from "@/lib/types"

const STORAGE_ROOT = path.join(process.cwd(), "storage")
const DB_DIR = path.join(STORAGE_ROOT, "db")
const ANNOUNCEMENTS_DB_FILE = path.join(DB_DIR, "announcements.json")
const ANNOUNCEMENTS_AUDIO_DIR = path.join(process.cwd(), "public", "audio", "announcements")
const ANNOUNCEMENTS_AUDIO_URL_PREFIX = "/audio/announcements"
const UUID_RE =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i

const ANNOUNCEMENT_TYPES: AnnouncementType[] = [
  "greeting",
  "hold",
  "ivr",
  "closed",
  "recording_notice",
  "custom",
]
const ANNOUNCEMENT_TYPE_SET = new Set<AnnouncementType>(ANNOUNCEMENT_TYPES)

export interface StoredAnnouncement {
  id: string
  name: string
  description: string | null
  announcementType: AnnouncementType
  isActive: boolean
  folderId: string | null
  audioFileUrl: string | null
  ttsText: string | null
  speakerId: number | null
  speakerName: string | null
  durationSec: number | null
  language: string
  source: "upload" | "tts"
  createdAt: string
  updatedAt: string
}

export interface StoredFolder {
  id: string
  name: string
  description: string | null
  parentId: string | null
  sortOrder: number
  createdAt: string
  updatedAt: string
}

interface AnnouncementsDatabase {
  announcements: Record<string, StoredAnnouncement>
  folders: Record<string, StoredFolder>
  updatedAt: string
}

export interface AnnouncementsSnapshot {
  announcements: StoredAnnouncement[]
  folders: StoredFolder[]
  updatedAt: string
}

export interface CreateAnnouncementInput {
  id?: string
  name: string
  description?: string | null
  announcementType: AnnouncementType
  isActive?: boolean
  folderId?: string | null
  audioFileUrl?: string | null
  ttsText?: string | null
  speakerId?: number | null
  speakerName?: string | null
  durationSec?: number | null
  language?: string
  source: "upload" | "tts"
}

export interface UpdateAnnouncementInput {
  name?: string
  isActive?: boolean
}

export class AnnouncementsStoreError extends Error {
  constructor(
    message: string,
    public readonly code: "VALIDATION" | "NOT_FOUND" | "READ_FAILED" | "WRITE_FAILED",
  ) {
    super(message)
    this.name = "AnnouncementsStoreError"
  }
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

function seedFolders(nowIso: string): Record<string, StoredFolder> {
  const entries: StoredFolder[] = [
    {
      id: "folder-greetings",
      name: "挨拶メッセージ",
      description: "着信時の挨拶音声",
      parentId: null,
      sortOrder: 1,
      createdAt: nowIso,
      updatedAt: nowIso,
    },
    {
      id: "folder-hold",
      name: "保留音",
      description: "保留中の音声",
      parentId: null,
      sortOrder: 2,
      createdAt: nowIso,
      updatedAt: nowIso,
    },
    {
      id: "folder-ivr",
      name: "IVRメニュー",
      description: "自動音声応答メニュー",
      parentId: null,
      sortOrder: 3,
      createdAt: nowIso,
      updatedAt: nowIso,
    },
    {
      id: "folder-closed",
      name: "時間外案内",
      description: "営業時間外のアナウンス",
      parentId: null,
      sortOrder: 4,
      createdAt: nowIso,
      updatedAt: nowIso,
    },
  ]

  return Object.fromEntries(entries.map((folder) => [folder.id, folder]))
}

function emptyDb(nowIso: string): AnnouncementsDatabase {
  return {
    announcements: {},
    folders: seedFolders(nowIso),
    updatedAt: nowIso,
  }
}

function isErrnoException(error: unknown): error is NodeJS.ErrnoException {
  return error instanceof Error && "code" in error
}

function parseDb(raw: string, nowIso: string): AnnouncementsDatabase {
  const parsed = JSON.parse(raw) as Partial<AnnouncementsDatabase>
  const folders =
    parsed.folders && Object.keys(parsed.folders).length > 0 ? parsed.folders : seedFolders(nowIso)

  return {
    announcements: parsed.announcements ?? {},
    folders,
    updatedAt: parsed.updatedAt ?? nowIso,
  }
}

async function ensureDirectories() {
  await fs.mkdir(DB_DIR, { recursive: true })
  await fs.mkdir(ANNOUNCEMENTS_AUDIO_DIR, { recursive: true })
}

async function writeDb(db: AnnouncementsDatabase): Promise<void> {
  const tempFile = `${ANNOUNCEMENTS_DB_FILE}.tmp`
  await fs.writeFile(tempFile, JSON.stringify(db, null, 2), "utf8")
  await fs.rename(tempFile, ANNOUNCEMENTS_DB_FILE)
}

async function readDb(): Promise<AnnouncementsDatabase> {
  await ensureDirectories()
  const nowIso = new Date().toISOString()
  try {
    const raw = await fs.readFile(ANNOUNCEMENTS_DB_FILE, "utf8")
    return parseDb(raw, nowIso)
  } catch (error) {
    if (isErrnoException(error) && error.code === "ENOENT") {
      const initial = emptyDb(nowIso)
      await writeDb(initial)
      return initial
    }
    throw new AnnouncementsStoreError(
      error instanceof Error ? error.message : "failed to read announcements db",
      "READ_FAILED",
    )
  }
}

function sortFolders(a: StoredFolder, b: StoredFolder): number {
  if (a.sortOrder !== b.sortOrder) {
    return a.sortOrder - b.sortOrder
  }
  return a.name.localeCompare(b.name, "ja")
}

function sortAnnouncements(a: StoredAnnouncement, b: StoredAnnouncement): number {
  return Date.parse(b.updatedAt) - Date.parse(a.updatedAt)
}

function toSnapshot(db: AnnouncementsDatabase): AnnouncementsSnapshot {
  return {
    announcements: Object.values(db.announcements).sort(sortAnnouncements),
    folders: Object.values(db.folders).sort(sortFolders),
    updatedAt: db.updatedAt,
  }
}

function trimOrNull(value: string | null | undefined): string | null {
  if (typeof value !== "string") {
    return null
  }
  const trimmed = value.trim()
  return trimmed.length > 0 ? trimmed : null
}

function requireAnnouncementType(value: string): AnnouncementType {
  if (!ANNOUNCEMENT_TYPE_SET.has(value as AnnouncementType)) {
    throw new AnnouncementsStoreError("invalid announcementType", "VALIDATION")
  }
  return value as AnnouncementType
}

function normalizeFolderId(folderId: string | null | undefined): string | null {
  if (folderId === undefined || folderId === null) {
    return null
  }
  const trimmed = folderId.trim()
  return trimmed.length > 0 ? trimmed : null
}

export function isAnnouncementType(value: string): value is AnnouncementType {
  return ANNOUNCEMENT_TYPE_SET.has(value as AnnouncementType)
}

export function newAnnouncementId(): string {
  return randomUUID()
}

export function announcementAudioUrl(id: string): string {
  return `${ANNOUNCEMENTS_AUDIO_URL_PREFIX}/${id}.wav`
}

export function announcementAudioPath(id: string): string {
  return path.join(ANNOUNCEMENTS_AUDIO_DIR, `${id}.wav`)
}

export async function listAnnouncementsSnapshot(): Promise<AnnouncementsSnapshot> {
  const db = await readDb()
  return toSnapshot(db)
}

export async function createAnnouncement(
  input: CreateAnnouncementInput,
): Promise<StoredAnnouncement> {
  return withWriteLock(async () => {
    const db = await readDb()
    const name = input.name.trim()
    if (!name) {
      throw new AnnouncementsStoreError("name is required", "VALIDATION")
    }

    const idRaw = input.id?.trim() || newAnnouncementId()
    if (!UUID_RE.test(idRaw)) {
      throw new AnnouncementsStoreError("id must be UUID", "VALIDATION")
    }

    const folderId = normalizeFolderId(input.folderId)
    if (folderId && !db.folders[folderId]) {
      throw new AnnouncementsStoreError("folderId not found", "VALIDATION")
    }
    if (db.announcements[idRaw]) {
      throw new AnnouncementsStoreError("announcement already exists", "VALIDATION")
    }

    const nowIso = new Date().toISOString()
    const announcement: StoredAnnouncement = {
      id: idRaw,
      name,
      description: trimOrNull(input.description),
      announcementType: requireAnnouncementType(input.announcementType),
      isActive: input.isActive ?? true,
      folderId,
      audioFileUrl: trimOrNull(input.audioFileUrl),
      ttsText: trimOrNull(input.ttsText),
      speakerId:
        typeof input.speakerId === "number" && Number.isFinite(input.speakerId)
          ? input.speakerId
          : null,
      speakerName: trimOrNull(input.speakerName),
      durationSec:
        typeof input.durationSec === "number" && Number.isFinite(input.durationSec)
          ? input.durationSec
          : null,
      language: trimOrNull(input.language) ?? "ja",
      source: input.source,
      createdAt: nowIso,
      updatedAt: nowIso,
    }

    db.announcements[announcement.id] = announcement
    db.updatedAt = nowIso
    await writeDb(db)

    return announcement
  })
}

export async function updateAnnouncement(
  id: string,
  patch: UpdateAnnouncementInput,
): Promise<StoredAnnouncement | null> {
  return withWriteLock(async () => {
    const db = await readDb()
    const current = db.announcements[id]
    if (!current) {
      return null
    }

    let changed = false
    if (patch.name !== undefined) {
      const nextName = patch.name.trim()
      if (!nextName) {
        throw new AnnouncementsStoreError("name must not be empty", "VALIDATION")
      }
      if (current.name !== nextName) {
        current.name = nextName
        changed = true
      }
    }

    if (patch.isActive !== undefined && current.isActive !== patch.isActive) {
      current.isActive = patch.isActive
      changed = true
    }

    if (!changed) {
      return current
    }

    const nowIso = new Date().toISOString()
    current.updatedAt = nowIso
    db.updatedAt = nowIso
    await writeDb(db)
    return current
  })
}

export async function deleteAnnouncement(id: string): Promise<StoredAnnouncement | null> {
  return withWriteLock(async () => {
    const db = await readDb()
    const current = db.announcements[id]
    if (!current) {
      return null
    }
    delete db.announcements[id]
    db.updatedAt = new Date().toISOString()
    await writeDb(db)
    return current
  })
}

export async function deleteAnnouncementAudioFile(audioFileUrl: string | null): Promise<void> {
  if (!audioFileUrl) {
    return
  }

  let pathname: string
  try {
    pathname = new URL(audioFileUrl, "http://localhost").pathname
  } catch {
    return
  }

  if (!pathname.startsWith(`${ANNOUNCEMENTS_AUDIO_URL_PREFIX}/`)) {
    return
  }

  const fileName = path.basename(pathname)
  if (!fileName.toLowerCase().endsWith(".wav")) {
    return
  }

  const absolutePath = path.join(ANNOUNCEMENTS_AUDIO_DIR, fileName)
  if (!absolutePath.startsWith(`${ANNOUNCEMENTS_AUDIO_DIR}${path.sep}`)) {
    return
  }

  try {
    await fs.unlink(absolutePath)
  } catch (error) {
    if (isErrnoException(error) && error.code === "ENOENT") {
      return
    }
    throw new AnnouncementsStoreError(
      error instanceof Error ? error.message : "failed to delete audio file",
      "WRITE_FAILED",
    )
  }
}
