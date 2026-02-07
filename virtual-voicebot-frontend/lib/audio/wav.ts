export function isWavBuffer(buffer: Buffer): boolean {
  if (buffer.length < 12) {
    return false
  }
  return (
    buffer.toString("ascii", 0, 4) === "RIFF" &&
    buffer.toString("ascii", 8, 12) === "WAVE"
  )
}

export function parseWavDurationSec(buffer: Buffer): number | null {
  if (!isWavBuffer(buffer)) {
    return null
  }

  let offset = 12
  let sampleRate: number | null = null
  let channels: number | null = null
  let bitsPerSample: number | null = null
  let dataSize: number | null = null

  while (offset + 8 <= buffer.length) {
    const chunkId = buffer.toString("ascii", offset, offset + 4)
    const chunkSize = buffer.readUInt32LE(offset + 4)
    const chunkStart = offset + 8
    const chunkEnd = chunkStart + chunkSize

    if (chunkEnd > buffer.length) {
      break
    }

    if (chunkId === "fmt " && chunkSize >= 16) {
      channels = buffer.readUInt16LE(chunkStart + 2)
      sampleRate = buffer.readUInt32LE(chunkStart + 4)
      bitsPerSample = buffer.readUInt16LE(chunkStart + 14)
    } else if (chunkId === "data") {
      dataSize = chunkSize
    }

    if (sampleRate && channels && bitsPerSample && dataSize !== null) {
      break
    }

    // WAV chunks are aligned to 2-byte boundaries.
    offset = chunkEnd + (chunkSize % 2)
  }

  if (!sampleRate || !channels || !bitsPerSample || dataSize === null) {
    return null
  }

  const bytesPerSecond = sampleRate * channels * (bitsPerSample / 8)
  if (!Number.isFinite(bytesPerSecond) || bytesPerSecond <= 0) {
    return null
  }

  const duration = dataSize / bytesPerSecond
  if (!Number.isFinite(duration) || duration < 0) {
    return null
  }

  return Math.round(duration * 1000) / 1000
}
