import { promises as fs } from "node:fs"
import * as path from "node:path"

import { NextRequest, NextResponse } from "next/server"

export const runtime = "nodejs"

const UUID_RE =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i

const ALLOWED_FILES = new Set(["mixed.wav", "meta.json"])

function contentType(fileName: string): string {
  if (fileName.endsWith(".wav")) {
    return "audio/wav"
  }
  if (fileName.endsWith(".json")) {
    return "application/json"
  }
  return "application/octet-stream"
}

export async function GET(
  _req: NextRequest,
  context: { params: Promise<{ callLogId: string; fileName: string }> },
) {
  const { callLogId, fileName } = await context.params

  if (!UUID_RE.test(callLogId)) {
    return NextResponse.json({ ok: false, error: "invalid callLogId" }, { status: 400 })
  }
  if (!ALLOWED_FILES.has(fileName)) {
    return NextResponse.json({ ok: false, error: "file not allowed" }, { status: 404 })
  }

  const fullPath = path.join(process.cwd(), "storage", "recordings", callLogId, fileName)
  try {
    const buffer = await fs.readFile(fullPath)
    return new NextResponse(buffer, {
      status: 200,
      headers: {
        "Content-Type": contentType(fileName),
        "Cache-Control": "private, max-age=0, must-revalidate",
      },
    })
  } catch (error) {
    const err = error as NodeJS.ErrnoException
    if (err.code === "ENOENT") {
      return NextResponse.json({ ok: false, error: "not found" }, { status: 404 })
    }
    console.error("[recordings/get] failed to read file", error)
    return NextResponse.json({ ok: false, error: "internal server error" }, { status: 500 })
  }
}
