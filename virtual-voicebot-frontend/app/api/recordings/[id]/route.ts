import { promises as fs } from "node:fs"
import * as path from "node:path"

import { NextRequest, NextResponse } from "next/server"

export const runtime = "nodejs"

const UUID_RE =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i

interface ByteRange {
  start: number
  end: number
}

function parseRange(rangeHeader: string, fileSize: number): ByteRange | null {
  if (!rangeHeader.startsWith("bytes=")) {
    return null
  }
  const [startRaw, endRaw] = rangeHeader.replace("bytes=", "").split("-")
  const start = Number.parseInt(startRaw, 10)
  const end = endRaw ? Number.parseInt(endRaw, 10) : fileSize - 1
  if (!Number.isFinite(start) || !Number.isFinite(end)) {
    return null
  }
  if (start < 0 || end < start || start >= fileSize) {
    return null
  }
  return { start, end: Math.min(end, fileSize - 1) }
}

async function buildResponse(
  request: NextRequest,
  id: string,
  method: "GET" | "HEAD",
): Promise<NextResponse> {
  if (!UUID_RE.test(id)) {
    return NextResponse.json({ ok: false, error: "invalid recording id" }, { status: 400 })
  }

  const filePath = path.join(process.cwd(), "storage", "recordings", id, "mixed.wav")
  let stats
  try {
    stats = await fs.stat(filePath)
  } catch (error) {
    const err = error as NodeJS.ErrnoException
    if (err.code === "ENOENT") {
      return NextResponse.json({ ok: false, error: "recording not found" }, { status: 404 })
    }
    return NextResponse.json({ ok: false, error: "failed to read file" }, { status: 500 })
  }

  const baseHeaders = {
    "Accept-Ranges": "bytes",
    "Content-Type": "audio/wav",
    "Cache-Control": "private, max-age=0, must-revalidate",
  }

  const rangeHeader = request.headers.get("range")
  if (rangeHeader) {
    const range = parseRange(rangeHeader, stats.size)
    if (!range) {
      return new NextResponse(null, {
        status: 416,
        headers: {
          ...baseHeaders,
          "Content-Range": `bytes */${stats.size}`,
        },
      })
    }
    const length = range.end - range.start + 1
    if (method === "HEAD") {
      return new NextResponse(null, {
        status: 206,
        headers: {
          ...baseHeaders,
          "Content-Range": `bytes ${range.start}-${range.end}/${stats.size}`,
          "Content-Length": String(length),
        },
      })
    }

    const handle = await fs.open(filePath, "r")
    try {
      const buffer = Buffer.alloc(length)
      await handle.read(buffer, 0, length, range.start)
      return new NextResponse(buffer, {
        status: 206,
        headers: {
          ...baseHeaders,
          "Content-Range": `bytes ${range.start}-${range.end}/${stats.size}`,
          "Content-Length": String(length),
        },
      })
    } finally {
      await handle.close()
    }
  }

  if (method === "HEAD") {
    return new NextResponse(null, {
      status: 200,
      headers: {
        ...baseHeaders,
        "Content-Length": String(stats.size),
      },
    })
  }

  const data = await fs.readFile(filePath)
  return new NextResponse(data, {
    status: 200,
    headers: {
      ...baseHeaders,
      "Content-Length": String(stats.size),
    },
  })
}

export async function GET(
  request: NextRequest,
  context: { params: Promise<{ id: string }> },
) {
  const { id } = await context.params
  return buildResponse(request, id, "GET")
}

export async function HEAD(
  request: NextRequest,
  context: { params: Promise<{ id: string }> },
) {
  const { id } = await context.params
  return buildResponse(request, id, "HEAD")
}
