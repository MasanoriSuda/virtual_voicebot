export type ServiceStatus = "ok" | "error" | "disabled"

export interface LocalServiceEntry {
  status: ServiceStatus
  displayUrl: string | null
}

export interface LocalServicesStatusResponse {
  ok: boolean
  localServices: {
    asr: LocalServiceEntry
    llm: LocalServiceEntry
    tts: LocalServiceEntry
  }
}

export async function fetchLocalServicesStatus(): Promise<LocalServicesStatusResponse> {
  const response = await fetch("/api/local-services-status", {
    method: "GET",
    cache: "no-store",
  })

  if (!response.ok) {
    throw new Error(
      `Failed to fetch local services status: ${response.status} ${response.statusText}`.trimEnd(),
    )
  }

  let parsed: unknown
  try {
    parsed = await response.json()
  } catch {
    throw new Error("Failed to parse local services status response as JSON")
  }
  return parsed as LocalServicesStatusResponse
}
