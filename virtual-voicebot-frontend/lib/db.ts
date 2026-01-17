import Database from "better-sqlite3"
import fs from "fs"
import path from "path"

const DATA_DIR = path.join(process.cwd(), "data")
const DB_PATH = path.join(DATA_DIR, "calls.db")

let db: Database.Database

function ensureDb() {
  if (!db) {
    if (!fs.existsSync(DATA_DIR)) {
      fs.mkdirSync(DATA_DIR, { recursive: true })
    }
    db = new Database(DB_PATH)
    db.pragma("journal_mode = WAL")
    db.exec(`
      CREATE TABLE IF NOT EXISTS calls (
        id TEXT PRIMARY KEY,
        from_number TEXT,
        to_number TEXT,
        start_time TEXT,
        end_time TEXT,
        status TEXT,
        summary TEXT,
        duration_sec INTEGER,
        recording_url TEXT,
        sample_rate INTEGER,
        channels INTEGER,
        created_at TEXT DEFAULT (datetime('now'))
      );
    `)
  }
  return db
}

export interface DbCallRecord {
  id: string
  from_number: string
  to_number: string
  start_time: string
  end_time: string | null
  status: string
  summary: string
  duration_sec: number
  recording_url: string | null
  sample_rate: number | null
  channels: number | null
}

export function upsertCall(record: DbCallRecord) {
  const database = ensureDb()
  const stmt = database.prepare(`
    INSERT INTO calls (id, from_number, to_number, start_time, end_time, status, summary, duration_sec, recording_url, sample_rate, channels)
    VALUES (@id, @from_number, @to_number, @start_time, @end_time, @status, @summary, @duration_sec, @recording_url, @sample_rate, @channels)
    ON CONFLICT(id) DO UPDATE SET
      from_number=excluded.from_number,
      to_number=excluded.to_number,
      start_time=excluded.start_time,
      end_time=excluded.end_time,
      status=excluded.status,
      summary=excluded.summary,
      duration_sec=excluded.duration_sec,
      recording_url=excluded.recording_url,
      sample_rate=excluded.sample_rate,
      channels=excluded.channels;
  `)
  stmt.run(record)
}

export function listCalls(): DbCallRecord[] {
  const database = ensureDb()
  const stmt = database.prepare("SELECT * FROM calls ORDER BY start_time DESC")
  return stmt.all() as DbCallRecord[]
}

export function getCallById(id: string): DbCallRecord | null {
  const database = ensureDb()
  const stmt = database.prepare("SELECT * FROM calls WHERE id = ?")
  return (stmt.get(id) as DbCallRecord | undefined) || null
}
