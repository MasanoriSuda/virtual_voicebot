import type { ActionConfig } from "@/lib/call-actions"

export type DtmfKey = "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "#" | "*"

export type IvrTerminalAction =
  | { actionCode: "VR" }
  | { actionCode: "VM"; announcementId: string | null }
  | { actionCode: "AN"; announcementId: string | null }
  | { actionCode: "IV"; ivrFlowId: string }
  | {
      actionCode: "VB"
      scenarioId: string
      welcomeAnnouncementId: string | null
      recordingEnabled: boolean
      includeAnnouncement: boolean
    }

export type IvrFallbackAction =
  | { actionCode: "VR" }
  | { actionCode: "VM"; announcementId: string | null }
  | { actionCode: "AN"; announcementId: string | null }
  | {
      actionCode: "VB"
      scenarioId: string
      welcomeAnnouncementId: string | null
      recordingEnabled: boolean
      includeAnnouncement: boolean
    }

export interface IvrRoute {
  dtmfKey: DtmfKey
  label: string
  destination: IvrTerminalAction
}

export interface IvrFlowDefinition {
  id: string
  name: string
  description: string | null
  isActive: boolean
  announcementId: string | null
  timeoutSec: number
  maxRetries: number
  invalidInputAnnouncementId: string | null
  timeoutAnnouncementId: string | null
  routes: IvrRoute[]
  fallbackAction: IvrFallbackAction
  createdAt: string
  updatedAt: string
}

export interface IvrFlowsDatabase {
  flows: IvrFlowDefinition[]
}

export interface ValidationResult {
  isValid: boolean
  errors: string[]
  warnings: string[]
}

export const DTMF_KEYS: DtmfKey[] = ["1", "2", "3", "4", "5", "6", "7", "8", "9", "0", "#", "*"]
export const DEFAULT_TIMEOUT_SEC = 10
export const DEFAULT_MAX_RETRIES = 2
export const MAX_IVR_DEPTH = 3

function createId(): string {
  if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
    return crypto.randomUUID()
  }

  const bytes = new Uint8Array(16)
  if (typeof crypto !== "undefined" && typeof crypto.getRandomValues === "function") {
    crypto.getRandomValues(bytes)
  } else {
    for (let i = 0; i < bytes.length; i += 1) {
      bytes[i] = Math.floor(Math.random() * 256)
    }
  }

  bytes[6] = (bytes[6] & 0x0f) | 0x40
  bytes[8] = (bytes[8] & 0x3f) | 0x80
  const hex = Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0"))
  return `${hex.slice(0, 4).join("")}-${hex.slice(4, 6).join("")}-${hex
    .slice(6, 8)
    .join("")}-${hex.slice(8, 10).join("")}-${hex.slice(10, 16).join("")}`
}

function nowIso(): string {
  return new Date().toISOString()
}

function normalizeNullableString(value: string | null | undefined): string | null {
  if (typeof value !== "string") {
    return null
  }
  const trimmed = value.trim()
  return trimmed.length > 0 ? trimmed : null
}

function flowLabel(flowId: string, flows: IvrFlowDefinition[]): string {
  return flows.find((flow) => flow.id === flowId)?.name ?? flowId
}

export function createDefaultIvrFlow(): IvrFlowDefinition {
  const timestamp = nowIso()
  return {
    id: createId(),
    name: "新規IVRフロー",
    description: null,
    isActive: true,
    announcementId: null,
    timeoutSec: DEFAULT_TIMEOUT_SEC,
    maxRetries: DEFAULT_MAX_RETRIES,
    invalidInputAnnouncementId: null,
    timeoutAnnouncementId: null,
    routes: [],
    fallbackAction: {
      actionCode: "VR",
    },
    createdAt: timestamp,
    updatedAt: timestamp,
  }
}

export function cloneIvrFlow(flow: IvrFlowDefinition): IvrFlowDefinition {
  return JSON.parse(JSON.stringify(flow)) as IvrFlowDefinition
}

export function terminalActionLabel(
  action: IvrTerminalAction,
  flows: IvrFlowDefinition[],
  scenarioNameById?: Map<string, string>,
): string {
  switch (action.actionCode) {
    case "VR":
      return "転送(VR)"
    case "VM":
      return "留守電(VM)"
    case "AN":
      return "アナウンス→切断(AN)"
    case "IV": {
      const label = flowLabel(action.ivrFlowId, flows)
      return `IVR(${label})`
    }
    case "VB": {
      const scenarioId = action.scenarioId.trim()
      const scenarioLabel = scenarioNameById?.get(scenarioId) ?? scenarioId
      return `ボイスボット(${scenarioLabel || "未選択"})`
    }
  }
}

function isValidDtmfKey(value: string): value is DtmfKey {
  return DTMF_KEYS.includes(value as DtmfKey)
}

function referencedIvrIds(flow: IvrFlowDefinition): string[] {
  const ids: string[] = []
  for (const route of flow.routes) {
    if (route.destination.actionCode === "IV") {
      ids.push(route.destination.ivrFlowId)
    }
  }
  return ids
}

export function detectCycles(flows: IvrFlowDefinition[]): string[][] | null {
  const byId = new Map(flows.map((flow) => [flow.id, flow]))
  const state = new Map<string, 0 | 1 | 2>()
  const stack: string[] = []
  const cycles = new Map<string, string[]>()

  const dfs = (flowId: string) => {
    state.set(flowId, 1)
    stack.push(flowId)

    const flow = byId.get(flowId)
    if (flow) {
      for (const nextId of referencedIvrIds(flow)) {
        if (!byId.has(nextId)) {
          continue
        }

        const nextState = state.get(nextId) ?? 0
        if (nextState === 0) {
          dfs(nextId)
          continue
        }

        if (nextState === 1) {
          const startIndex = stack.indexOf(nextId)
          if (startIndex >= 0) {
            const cycle = [...stack.slice(startIndex), nextId]
            const key = cycle.join("->")
            cycles.set(key, cycle)
          }
        }
      }
    }

    stack.pop()
    state.set(flowId, 2)
  }

  for (const flow of flows) {
    if ((state.get(flow.id) ?? 0) === 0) {
      dfs(flow.id)
    }
  }

  if (cycles.size === 0) {
    return null
  }
  return Array.from(cycles.values())
}

export function getMaxDepth(flowId: string, flows: IvrFlowDefinition[]): number {
  const byId = new Map(flows.map((flow) => [flow.id, flow]))

  const dfs = (id: string, depth: number, path: Set<string>): number => {
    const flow = byId.get(id)
    if (!flow) {
      return depth
    }

    let maxDepth = depth
    for (const nextId of referencedIvrIds(flow)) {
      if (!byId.has(nextId)) {
        continue
      }
      if (path.has(nextId)) {
        maxDepth = Math.max(maxDepth, depth + 1)
        continue
      }

      const nextPath = new Set(path)
      nextPath.add(nextId)
      maxDepth = Math.max(maxDepth, dfs(nextId, depth + 1, nextPath))
    }

    return maxDepth
  }

  return dfs(flowId, 1, new Set([flowId]))
}

export function validateIvrFlows(
  flows: IvrFlowDefinition[],
  knownScenarioIds?: Set<string>,
): ValidationResult {
  const errors: string[] = []
  const warnings: string[] = []
  const byId = new Map(flows.map((flow) => [flow.id, flow]))

  flows.forEach((flow, flowIndex) => {
    const flowName = flow.name.trim().length > 0 ? flow.name.trim() : `Flow#${flowIndex + 1}`

    if (flow.name.trim().length === 0) {
      errors.push("フロー名を入力してください")
    }

    if (flow.routes.length === 0) {
      errors.push(`${flowName}: 少なくとも1つのルートを追加してください`)
    }

    if (!Number.isFinite(flow.timeoutSec) || flow.timeoutSec <= 0) {
      errors.push(`${flowName}: timeoutSec は1以上にしてください`)
    }

    if (!Number.isFinite(flow.maxRetries) || flow.maxRetries < 0) {
      errors.push(`${flowName}: maxRetries は0以上にしてください`)
    }

    const usedKeys = new Set<string>()
    flow.routes.forEach((route, routeIndex) => {
      if (!isValidDtmfKey(route.dtmfKey)) {
        errors.push(`${flowName}: 不正な DTMF キーです (${route.dtmfKey})`)
      }

      if (usedKeys.has(route.dtmfKey)) {
        errors.push(`${flowName}: キー ${route.dtmfKey} が重複しています`)
      }
      usedKeys.add(route.dtmfKey)

      if (route.label.trim().length === 0) {
        errors.push(`${flowName}: ルート ${routeIndex + 1} のラベルを入力してください`)
      }

      if (route.destination.actionCode === "IV") {
        if (route.destination.ivrFlowId.trim().length === 0) {
          errors.push(`${flowName}: キー ${route.dtmfKey} の参照先 IVR が未設定です`)
        } else if (!byId.has(route.destination.ivrFlowId)) {
          warnings.push(
            `${flowName}: キー ${route.dtmfKey} の参照先 IVR (${route.destination.ivrFlowId}) が見つかりません`,
          )
        }
      } else if (route.destination.actionCode === "VB") {
        const scenarioId = route.destination.scenarioId.trim()
        if (scenarioId.length === 0) {
          errors.push(`${flowName}: キー ${route.dtmfKey} のシナリオを選択してください`)
        } else if (knownScenarioIds && !knownScenarioIds.has(scenarioId)) {
          warnings.push(
            `${flowName}: キー ${route.dtmfKey} の参照先シナリオ (${scenarioId}) が見つかりません`,
          )
        }
      }
    })

    if (flow.fallbackAction.actionCode === "VB") {
      const scenarioId = flow.fallbackAction.scenarioId.trim()
      if (scenarioId.length === 0) {
        errors.push(`${flowName}: fallback のシナリオを選択してください`)
      } else if (knownScenarioIds && !knownScenarioIds.has(scenarioId)) {
        warnings.push(`${flowName}: fallback の参照先シナリオ (${scenarioId}) が見つかりません`)
      }
    }
  })

  const cycles = detectCycles(flows)
  if (cycles) {
    for (const cycle of cycles) {
      const labels = cycle.map((id) => flowLabel(id, flows))
      errors.push(`循環参照が検出されました: ${labels.join(" -> ")}`)
    }
  }

  for (const flow of flows) {
    const depth = getMaxDepth(flow.id, flows)
    if (depth > MAX_IVR_DEPTH) {
      errors.push(`${flow.name || flow.id}: IVR のネストは${MAX_IVR_DEPTH}層までです`)
    }
  }

  return {
    isValid: errors.length === 0,
    errors,
    warnings,
  }
}

export function toIvrDestinationFromCallAction(config: ActionConfig): IvrTerminalAction | null {
  switch (config.actionCode) {
    case "VR":
      return { actionCode: "VR" }
    case "VM":
      return { actionCode: "VM", announcementId: normalizeNullableString(config.announcementId) }
    case "AN":
      return { actionCode: "AN", announcementId: normalizeNullableString(config.announcementId) }
    case "IV":
      return config.ivrFlowId ? { actionCode: "IV", ivrFlowId: config.ivrFlowId } : null
    case "VB":
      return config.scenarioId
        ? {
            actionCode: "VB",
            scenarioId: config.scenarioId,
            welcomeAnnouncementId: normalizeNullableString(config.welcomeAnnouncementId),
            recordingEnabled: config.recordingEnabled,
            includeAnnouncement: config.includeAnnouncement,
          }
        : null
    default:
      return null
  }
}
