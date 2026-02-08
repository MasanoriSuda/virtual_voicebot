"use client"

import { useEffect, useMemo, useState, type ReactNode } from "react"
import {
  AlertTriangle,
  CheckCircle2,
  Copy,
  Plus,
  Save,
  Search,
  Trash2,
  X,
} from "lucide-react"

import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { ScrollArea } from "@/components/ui/scroll-area"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { Switch } from "@/components/ui/switch"
import {
  DTMF_KEYS,
  MAX_IVR_DEPTH,
  cloneIvrFlow,
  createDefaultIvrFlow,
  terminalActionLabel,
  validateIvrFlows,
  type DtmfKey,
  type IvrFallbackAction,
  type IvrFlowDefinition,
  type IvrRoute,
  type IvrTerminalAction,
} from "@/lib/ivr-flows"
import type { VoicebotScenario } from "@/lib/scenarios"
import { cn } from "@/lib/utils"

interface IvrFlowsApiResponse {
  ok: boolean
  flows?: IvrFlowDefinition[]
  error?: string
}

interface StoredAnnouncement {
  id: string
  name: string
  announcementType: string
  isActive: boolean
}

interface AnnouncementsApiResponse {
  ok: boolean
  announcements?: StoredAnnouncement[]
  error?: string
}

interface ScenariosApiResponse {
  ok: boolean
  scenarios?: VoicebotScenario[]
  error?: string
}

interface RouteDraft {
  dtmfKey: DtmfKey
  label: string
  destinationType: IvrTerminalAction["actionCode"]
  announcementId: string | null
  ivrFlowId: string | null
  scenarioId: string
  welcomeAnnouncementId: string | null
  recordingEnabled: boolean
  includeAnnouncement: boolean
}

type RouteDrafts = Record<string, RouteDraft | undefined>

const NONE_VALUE = "__none__"
const REQUIRED_ANNOUNCEMENT_VALUE = "__required_announcement__"
const NONE_SCENARIO_VALUE = "__none_scenario__"

function nowIso(): string {
  return new Date().toISOString()
}

function cloneFlows(flows: IvrFlowDefinition[]): IvrFlowDefinition[] {
  return flows.map((flow) => cloneIvrFlow(flow))
}

function announcementTypeLabel(type: string): string {
  switch (type) {
    case "greeting":
      return "挨拶"
    case "hold":
      return "保留"
    case "ivr":
      return "IVR"
    case "closed":
      return "時間外"
    case "recording_notice":
      return "録音通知"
    case "custom":
      return "カスタム"
    default:
      return type
  }
}

function normalizeAnnouncementValue(value: string): string | null {
  return value === NONE_VALUE ? null : value
}

function routeDestinationFromDraft(draft: RouteDraft): IvrTerminalAction {
  switch (draft.destinationType) {
    case "VM":
      return {
        actionCode: "VM",
        announcementId: draft.announcementId,
      }
    case "AN":
      return {
        actionCode: "AN",
        announcementId: draft.announcementId,
      }
    case "IV":
      return {
        actionCode: "IV",
        ivrFlowId: draft.ivrFlowId ?? "",
      }
    case "VB":
      return {
        actionCode: "VB",
        scenarioId: draft.scenarioId,
        welcomeAnnouncementId: draft.welcomeAnnouncementId,
        recordingEnabled: draft.recordingEnabled,
        includeAnnouncement: draft.includeAnnouncement,
      }
    case "VR":
    default:
      return {
        actionCode: "VR",
      }
  }
}

function fallbackDestinationFromCode(actionCode: "VR" | "VM" | "AN" | "VB"): IvrFallbackAction {
  if (actionCode === "VM") {
    return { actionCode: "VM", announcementId: null }
  }
  if (actionCode === "AN") {
    return { actionCode: "AN", announcementId: null }
  }
  if (actionCode === "VB") {
    return {
      actionCode: "VB",
      scenarioId: "",
      welcomeAnnouncementId: null,
      recordingEnabled: true,
      includeAnnouncement: false,
    }
  }
  return { actionCode: "VR" }
}

function routeDraftForFlow(flow: IvrFlowDefinition): RouteDraft | null {
  const used = new Set(flow.routes.map((route) => route.dtmfKey))
  const firstKey = DTMF_KEYS.find((key) => !used.has(key))
  if (!firstKey) {
    return null
  }
  return {
    dtmfKey: firstKey,
    label: "",
    destinationType: "VR",
    announcementId: null,
    ivrFlowId: null,
    scenarioId: "",
    welcomeAnnouncementId: null,
    recordingEnabled: true,
    includeAnnouncement: false,
  }
}

function collectReferencedFlowIds(flows: IvrFlowDefinition[]): Set<string> {
  const referenced = new Set<string>()
  for (const flow of flows) {
    for (const route of flow.routes) {
      if (route.destination.actionCode === "IV" && route.destination.ivrFlowId.trim().length > 0) {
        referenced.add(route.destination.ivrFlowId)
      }
    }
  }
  return referenced
}

function collectRootFlows(flows: IvrFlowDefinition[]): IvrFlowDefinition[] {
  const referenced = collectReferencedFlowIds(flows)
  return flows.filter((flow) => !referenced.has(flow.id))
}

export function IvrContent() {
  const [flows, setFlows] = useState<IvrFlowDefinition[]>([])
  const [savedFlows, setSavedFlows] = useState<IvrFlowDefinition[]>([])
  const [selectedFlowId, setSelectedFlowId] = useState<string | null>(null)
  const [announcements, setAnnouncements] = useState<StoredAnnouncement[]>([])
  const [scenarios, setScenarios] = useState<VoicebotScenario[]>([])
  const [searchQuery, setSearchQuery] = useState("")
  const [routeDrafts, setRouteDrafts] = useState<RouteDrafts>({})

  const [loading, setLoading] = useState(true)
  const [busy, setBusy] = useState(false)
  const [errorMessage, setErrorMessage] = useState<string | null>(null)
  const [infoMessage, setInfoMessage] = useState<string | null>(null)

  const selectedFlow = useMemo(
    () => flows.find((flow) => flow.id === selectedFlowId) ?? null,
    [flows, selectedFlowId],
  )

  const rootFlows = useMemo(() => collectRootFlows(flows), [flows])

  const filteredAllFlows = useMemo(() => {
    const keyword = searchQuery.trim().toLowerCase()
    if (!keyword) {
      return flows
    }
    return flows.filter((flow) => {
      const text = [flow.name, flow.description].filter(Boolean).join(" ").toLowerCase()
      return text.includes(keyword)
    })
  }, [flows, searchQuery])

  const filteredRootFlows = useMemo(() => {
    const keyword = searchQuery.trim().toLowerCase()
    if (!keyword) {
      return rootFlows
    }
    return rootFlows.filter((flow) => {
      const text = [flow.name, flow.description].filter(Boolean).join(" ").toLowerCase()
      return text.includes(keyword)
    })
  }, [rootFlows, searchQuery])

  const listFlows = rootFlows.length > 0 ? filteredRootFlows : filteredAllFlows

  const announcementOptions = useMemo(
    () =>
      [...announcements].sort((a, b) => {
        if (a.isActive !== b.isActive) {
          return a.isActive ? -1 : 1
        }
        return a.name.localeCompare(b.name, "ja")
      }),
    [announcements],
  )

  const announcementById = useMemo(
    () => new Map(announcements.map((announcement) => [announcement.id, announcement])),
    [announcements],
  )

  const scenarioOptions = useMemo(
    () =>
      [...scenarios].sort((a, b) => {
        if (a.isActive !== b.isActive) {
          return a.isActive ? -1 : 1
        }
        return a.name.localeCompare(b.name, "ja")
      }),
    [scenarios],
  )

  const scenarioById = useMemo(
    () => new Map(scenarios.map((scenario) => [scenario.id, scenario])),
    [scenarios],
  )

  const scenarioNameById = useMemo(
    () => new Map(scenarios.map((scenario) => [scenario.id, scenario.name])),
    [scenarios],
  )

  const flowById = useMemo(
    () => new Map(flows.map((flow) => [flow.id, flow])),
    [flows],
  )

  useEffect(() => {
    let cancelled = false

    const load = async () => {
      setLoading(true)
      setErrorMessage(null)
      setInfoMessage(null)

      try {
        const [flowsRes, announcementsRes, scenariosRes] = await Promise.all([
          fetch("/api/ivr-flows", { cache: "no-store" }),
          fetch("/api/announcements", { cache: "no-store" }).catch(() => null),
          fetch("/api/scenarios", { cache: "no-store" }).catch(() => null),
        ])

        const flowsBody = (await flowsRes.json()) as IvrFlowsApiResponse
        if (!flowsRes.ok || !flowsBody.ok) {
          throw new Error(flowsBody.error ?? "failed to load ivr flows")
        }

        const loadedFlows = Array.isArray(flowsBody.flows) ? flowsBody.flows : []

        let loadedAnnouncements: StoredAnnouncement[] = []
        if (announcementsRes && announcementsRes.ok) {
          const announcementsBody = (await announcementsRes.json()) as AnnouncementsApiResponse
          if (announcementsBody.ok && Array.isArray(announcementsBody.announcements)) {
            loadedAnnouncements = announcementsBody.announcements
          }
        }

        let loadedScenarios: VoicebotScenario[] = []
        if (scenariosRes && scenariosRes.ok) {
          const scenariosBody = (await scenariosRes.json()) as ScenariosApiResponse
          if (scenariosBody.ok && Array.isArray(scenariosBody.scenarios)) {
            loadedScenarios = scenariosBody.scenarios
          }
        }

        if (cancelled) {
          return
        }

        setFlows(cloneFlows(loadedFlows))
        setSavedFlows(cloneFlows(loadedFlows))
        const loadedRoots = collectRootFlows(loadedFlows)
        setSelectedFlowId(loadedRoots[0]?.id ?? loadedFlows[0]?.id ?? null)
        setAnnouncements(loadedAnnouncements)
        setScenarios(loadedScenarios)
        setRouteDrafts({})
      } catch (error) {
        if (cancelled) {
          return
        }
        setErrorMessage(error instanceof Error ? error.message : "IVR フローの読み込みに失敗しました")
      } finally {
        if (!cancelled) {
          setLoading(false)
        }
      }
    }

    void load()

    return () => {
      cancelled = true
    }
  }, [])

  useEffect(() => {
    if (!selectedFlowId) {
      return
    }
    if (!flowById.has(selectedFlowId)) {
      setSelectedFlowId(rootFlows[0]?.id ?? flows[0]?.id ?? null)
    }
  }, [flowById, flows, rootFlows, selectedFlowId])

  useEffect(() => {
    setRouteDrafts((prev) => {
      const validIds = new Set(flows.map((flow) => flow.id))
      let changed = false
      const next: RouteDrafts = {}
      for (const [flowId, draft] of Object.entries(prev)) {
        if (!validIds.has(flowId)) {
          changed = true
          continue
        }
        next[flowId] = draft
      }
      return changed ? next : prev
    })
  }, [flows])

  const updateFlowById = (flowId: string, updater: (flow: IvrFlowDefinition) => IvrFlowDefinition) => {
    setFlows((prev) =>
      prev.map((flow) => {
        if (flow.id !== flowId) {
          return flow
        }
        const next = updater(flow)
        return {
          ...next,
          updatedAt: nowIso(),
        }
      }),
    )
    setInfoMessage("未保存の変更があります")
  }

  const updateSelectedFlow = (updater: (flow: IvrFlowDefinition) => IvrFlowDefinition) => {
    if (!selectedFlowId) {
      return
    }
    updateFlowById(selectedFlowId, updater)
  }

  const updateRoute = (
    flowId: string,
    routeIndex: number,
    updater: (route: IvrRoute) => IvrRoute,
  ) => {
    updateFlowById(flowId, (flow) => ({
      ...flow,
      routes: flow.routes.map((route, index) => (index === routeIndex ? updater(route) : route)),
    }))
  }

  const setRouteDestinationType = (
    flowId: string,
    routeIndex: number,
    actionCode: IvrTerminalAction["actionCode"],
    depth: number,
  ) => {
    if (actionCode === "VM") {
      updateRoute(flowId, routeIndex, (current) => ({
        ...current,
        destination: {
          actionCode: "VM",
          announcementId: null,
        },
      }))
      return
    }
    if (actionCode === "AN") {
      updateRoute(flowId, routeIndex, (current) => ({
        ...current,
        destination: {
          actionCode: "AN",
          announcementId: null,
        },
      }))
      return
    }
    if (actionCode === "VB") {
      updateRoute(flowId, routeIndex, (current) => ({
        ...current,
        destination: {
          actionCode: "VB",
          scenarioId: "",
          welcomeAnnouncementId: null,
          recordingEnabled: true,
          includeAnnouncement: false,
        },
      }))
      return
    }
    if (actionCode === "VR") {
      updateRoute(flowId, routeIndex, (current) => ({
        ...current,
        destination: { actionCode: "VR" },
      }))
      return
    }

    const parentFlow = flowById.get(flowId)
    if (!parentFlow) {
      return
    }
    const currentRoute = parentFlow.routes[routeIndex]
    if (!currentRoute) {
      return
    }
    if (currentRoute.destination.actionCode === "IV") {
      return
    }
    if (depth >= MAX_IVR_DEPTH) {
      setErrorMessage(`ネスト上限(${MAX_IVR_DEPTH}層)のため、次層IVRを追加できません`)
      return
    }

    const newFlow = createDefaultIvrFlow()
    newFlow.name = `${parentFlow.name || "IVRフロー"}-${currentRoute.dtmfKey}`
    const timestamp = nowIso()

    setFlows((prev) =>
      prev
        .map((flow) =>
          flow.id === flowId
            ? {
                ...flow,
                routes: flow.routes.map((item, index) =>
                  index === routeIndex
                    ? {
                        ...item,
                        destination: {
                          actionCode: "IV" as const,
                          ivrFlowId: newFlow.id,
                        },
                      }
                    : item,
                ),
                updatedAt: timestamp,
              }
            : flow,
        )
        .concat(newFlow),
    )
    setInfoMessage("次層IVRを新規作成して紐付けました（未保存）")
    setErrorMessage(null)
  }

  const updateRouteDraft = (flowId: string, updater: (draft: RouteDraft) => RouteDraft) => {
    setRouteDrafts((prev) => {
      const current = prev[flowId]
      if (!current) {
        return prev
      }
      return {
        ...prev,
        [flowId]: updater(current),
      }
    })
  }

  const clearRouteDraft = (flowId: string) => {
    setRouteDrafts((prev) => {
      if (!prev[flowId]) {
        return prev
      }
      const next = { ...prev }
      delete next[flowId]
      return next
    })
  }

  const createFlow = () => {
    const newFlow = createDefaultIvrFlow()
    setFlows((prev) => [...prev, newFlow])
    setSelectedFlowId(newFlow.id)
    setInfoMessage("新規フローを作成しました（未保存）")
    setErrorMessage(null)
  }

  const duplicateSelectedFlow = () => {
    if (!selectedFlow) {
      return
    }

    const copied = cloneIvrFlow(selectedFlow)
    const timestamp = nowIso()
    copied.id = createDefaultIvrFlow().id
    copied.name = `${selectedFlow.name || "IVRフロー"}(コピー)`
    copied.createdAt = timestamp
    copied.updatedAt = timestamp

    setFlows((prev) => [...prev, copied])
    setSelectedFlowId(copied.id)
    setInfoMessage("フローを複製しました（未保存）")
    setErrorMessage(null)
  }

  const deleteSelectedFlow = () => {
    if (!selectedFlow) {
      return
    }

    const ok = window.confirm(`IVR フロー「${selectedFlow.name || selectedFlow.id}」を削除しますか？`)
    if (!ok) {
      return
    }

    setFlows((prev) => {
      const next = prev.filter((flow) => flow.id !== selectedFlow.id)
      const nextRoots = collectRootFlows(next)
      setSelectedFlowId(nextRoots[0]?.id ?? next[0]?.id ?? null)
      return next
    })
    clearRouteDraft(selectedFlow.id)
    setInfoMessage("フローを削除しました（未保存）")
    setErrorMessage(null)
  }

  const startAddRoute = (flowId: string) => {
    const flow = flowById.get(flowId)
    if (!flow) {
      return
    }
    const draft = routeDraftForFlow(flow)
    if (!draft) {
      setErrorMessage("使用可能な DTMF キーがありません")
      return
    }
    setRouteDrafts((prev) => ({
      ...prev,
      [flowId]: draft,
    }))
    setErrorMessage(null)
  }

  const addRoute = (flowId: string, depth: number) => {
    const flow = flowById.get(flowId)
    const routeDraft = routeDrafts[flowId]
    if (!flow || !routeDraft) {
      return
    }

    if (!routeDraft.label.trim()) {
      setErrorMessage("ラベルを入力してください")
      return
    }

    if (flow.routes.some((route) => route.dtmfKey === routeDraft.dtmfKey)) {
      setErrorMessage(`キー ${routeDraft.dtmfKey} が重複しています`)
      return
    }

    if (routeDraft.destinationType === "IV") {
      if (depth >= MAX_IVR_DEPTH) {
        setErrorMessage(`ネスト上限(${MAX_IVR_DEPTH}層)のため、サブIVRを指定できません`)
        return
      }
    }

    if (routeDraft.destinationType === "VB" && routeDraft.scenarioId.trim().length === 0) {
      setErrorMessage("シナリオを選択してください")
      return
    }

    let autoCreatedFlow: IvrFlowDefinition | null = null
    let destination = routeDestinationFromDraft(routeDraft)
    if (routeDraft.destinationType === "IV" && !routeDraft.ivrFlowId) {
      autoCreatedFlow = createDefaultIvrFlow()
      autoCreatedFlow.name = `${flow.name || "IVRフロー"}-${routeDraft.dtmfKey}`
      destination = {
        actionCode: "IV",
        ivrFlowId: autoCreatedFlow.id,
      }
    }

    const nextRoute: IvrRoute = {
      dtmfKey: routeDraft.dtmfKey,
      label: routeDraft.label.trim(),
      destination,
    }

    if (autoCreatedFlow) {
      const timestamp = nowIso()
      setFlows((prev) =>
        prev
          .map((current) =>
            current.id === flowId
              ? {
                  ...current,
                  routes: [...current.routes, nextRoute],
                  updatedAt: timestamp,
                }
              : current,
          )
          .concat(autoCreatedFlow),
      )
      setInfoMessage("ルート追加時に次層IVRを自動作成しました（未保存）")
    } else {
      updateFlowById(flowId, (current) => ({
        ...current,
        routes: [...current.routes, nextRoute],
      }))
    }
    clearRouteDraft(flowId)
    setErrorMessage(null)
  }

  const removeRoute = (flowId: string, routeIndex: number) => {
    updateFlowById(flowId, (flow) => ({
      ...flow,
      routes: flow.routes.filter((_, index) => index !== routeIndex),
    }))
  }

  const createSubFlowForRoute = (flowId: string, routeIndex: number, depth: number) => {
    const parentFlow = flowById.get(flowId)
    if (!parentFlow) {
      return
    }
    if (depth >= MAX_IVR_DEPTH) {
      setErrorMessage(`ネスト上限(${MAX_IVR_DEPTH}層)のため、次層IVRを追加できません`)
      return
    }

    const route = parentFlow.routes[routeIndex]
    const newFlow = createDefaultIvrFlow()
    newFlow.name = `${parentFlow.name || "IVRフロー"}-${route?.dtmfKey ?? "sub"}`
    const timestamp = nowIso()

    setFlows((prev) =>
      prev
        .map((flow) =>
          flow.id === flowId
            ? {
                ...flow,
                routes: flow.routes.map((item, index) =>
                  index === routeIndex
                    ? {
                        ...item,
                        destination: {
                          actionCode: "IV" as const,
                          ivrFlowId: newFlow.id,
                        },
                      }
                    : item,
                ),
                updatedAt: timestamp,
              }
            : flow,
        )
        .concat(newFlow),
    )
    setInfoMessage("次層IVRを新規作成して紐付けました（未保存）")
    setErrorMessage(null)
  }

  const saveAll = async () => {
    const validation = validateIvrFlows(
      flows,
      new Set(scenarios.map((scenario) => scenario.id)),
    )
    if (!validation.isValid) {
      setErrorMessage(validation.errors.join("\n"))
      return
    }

    const missingPromptFlowNames = flows
      .filter((flow) => !flow.announcementId || !announcementById.has(flow.announcementId))
      .map((flow) => flow.name || flow.id)
    if (missingPromptFlowNames.length > 0) {
      setErrorMessage(
        `以下のフローで案内アナウンスが未設定です:\n${missingPromptFlowNames
          .map((name) => `- ${name}`)
          .join("\n")}`,
      )
      return
    }

    if (validation.warnings.length > 0) {
      const confirmed = window.confirm(
        `${validation.warnings.join("\n")}\n\n参照先が見つからないルートがあります。保存しますか？`,
      )
      if (!confirmed) {
        return
      }
    }

    setBusy(true)
    setErrorMessage(null)
    setInfoMessage(null)

    try {
      const response = await fetch("/api/ivr-flows", {
        method: "PUT",
        headers: {
          "content-type": "application/json",
        },
        body: JSON.stringify({ flows }),
      })
      const body = (await response.json()) as { ok: boolean; error?: string }

      if (!response.ok || !body.ok) {
        throw new Error(body.error ?? "failed to save ivr flows")
      }

      setSavedFlows(cloneFlows(flows))
      setRouteDrafts({})
      setInfoMessage("IVR フローを保存しました")
    } catch (error) {
      setErrorMessage(error instanceof Error ? error.message : "保存に失敗しました")
    } finally {
      setBusy(false)
    }
  }

  const cancelAll = () => {
    const restored = cloneFlows(savedFlows)
    setFlows(restored)
    const restoredRoots = collectRootFlows(restored)
    const selectedIsRoot =
      selectedFlowId !== null && restoredRoots.some((flow) => flow.id === selectedFlowId)
    if (selectedFlowId && selectedIsRoot) {
      setSelectedFlowId(selectedFlowId)
    } else {
      setSelectedFlowId(restoredRoots[0]?.id ?? restored[0]?.id ?? null)
    }
    setRouteDrafts({})
    setErrorMessage(null)
    setInfoMessage("変更を取り消しました")
  }

  const renderAnnouncementSelect = (
    selectedId: string | null,
    onValueChange: (value: string) => void,
    disabled: boolean,
  ) => {
    const hasAnnouncement = selectedId ? announcementById.has(selectedId) : true
    const selectDisabled = disabled || (announcementOptions.length === 0 && !selectedId)

    return (
      <Select
        value={selectedId ?? NONE_VALUE}
        onValueChange={onValueChange}
        disabled={selectDisabled}
      >
        <SelectTrigger>
          <SelectValue placeholder="アナウンスを選択" />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value={NONE_VALUE}>なし</SelectItem>
          {!hasAnnouncement && selectedId && (
            <SelectItem value={selectedId}>（削除済み）</SelectItem>
          )}
          {announcementOptions.length === 0 ? (
            <SelectItem value="__empty__" disabled>
              （アナウンス未登録）
            </SelectItem>
          ) : (
            announcementOptions.map((announcement) => (
              <SelectItem key={announcement.id} value={announcement.id}>
                {announcement.name} ({announcementTypeLabel(announcement.announcementType)})
                {!announcement.isActive ? " [無効]" : ""}
              </SelectItem>
            ))
          )}
        </SelectContent>
      </Select>
    )
  }

  const renderRequiredAnnouncementSelect = (
    selectedId: string | null,
    onValueChange: (value: string) => void,
    disabled: boolean,
  ) => {
    const hasAnnouncement = selectedId ? announcementById.has(selectedId) : false
    const selectDisabled = disabled || (announcementOptions.length === 0 && !selectedId)
    const value = selectedId ?? REQUIRED_ANNOUNCEMENT_VALUE

    return (
      <Select value={value} onValueChange={onValueChange} disabled={selectDisabled}>
        <SelectTrigger>
          <SelectValue placeholder="アナウンスを選択" />
        </SelectTrigger>
        <SelectContent>
          {!selectedId && (
            <SelectItem value={REQUIRED_ANNOUNCEMENT_VALUE} disabled>
              選択してください
            </SelectItem>
          )}
          {!hasAnnouncement && selectedId && (
            <SelectItem value={selectedId}>（削除済み）</SelectItem>
          )}
          {announcementOptions.length === 0 ? (
            <SelectItem value="__empty_required__" disabled>
              （アナウンス未登録）
            </SelectItem>
          ) : (
            announcementOptions.map((announcement) => (
              <SelectItem key={announcement.id} value={announcement.id}>
                {announcement.name} ({announcementTypeLabel(announcement.announcementType)})
                {!announcement.isActive ? " [無効]" : ""}
              </SelectItem>
            ))
          )}
        </SelectContent>
      </Select>
    )
  }

  const renderScenarioSelect = (
    selectedScenarioId: string,
    onValueChange: (value: string) => void,
    disabled: boolean,
  ) => {
    const hasSelectedScenario = selectedScenarioId ? scenarioById.has(selectedScenarioId) : true
    const selectDisabled = disabled || (scenarioOptions.length === 0 && selectedScenarioId.length === 0)

    return (
      <Select
        value={selectedScenarioId || NONE_SCENARIO_VALUE}
        onValueChange={onValueChange}
        disabled={selectDisabled}
      >
        <SelectTrigger>
          <SelectValue placeholder="シナリオを選択" />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value={NONE_SCENARIO_VALUE}>選択してください</SelectItem>
          {!hasSelectedScenario && selectedScenarioId && (
            <SelectItem value={selectedScenarioId}>（削除済みシナリオ）</SelectItem>
          )}
          {scenarioOptions.length === 0 ? (
            <SelectItem value="__empty_scenario__" disabled>
              （シナリオ未登録）
            </SelectItem>
          ) : (
            scenarioOptions.map((scenario) => (
              <SelectItem
                key={scenario.id}
                value={scenario.id}
                disabled={!scenario.isActive && scenario.id !== selectedScenarioId}
              >
                {scenario.name}
                {!scenario.isActive ? " [無効]" : ""}
              </SelectItem>
            ))
          )}
        </SelectContent>
      </Select>
    )
  }

  const renderRoutesEditor = (
    flowId: string,
    depth: number,
    ancestry: string[],
    edgeLabel?: string,
  ): ReactNode => {
    const editingFlow = flowById.get(flowId)
    if (!editingFlow) {
      return null
    }

    const routeDraft = routeDrafts[flowId] ?? null
    const canNestFurther = depth < MAX_IVR_DEPTH
    const nestDepthLabel = Math.min(depth + 1, MAX_IVR_DEPTH)

    return (
      <div className={cn("space-y-2", depth > 1 && "ml-4 pl-4 border-l-2 border-dashed border-border/70")}>
        {depth > 1 && (
          <div className="flex items-center gap-2 text-xs text-muted-foreground">
            <span className="font-mono">└─</span>
            <span>{edgeLabel ?? `${depth}層`}</span>
          </div>
        )}
        <div className={cn("rounded-md border p-3 space-y-3", depth > 1 && "bg-muted/20")}>
        <div className="flex items-center justify-between gap-2">
          <div className="flex items-center gap-2">
            <h3 className="font-medium">DTMF ルート</h3>
            <Badge variant="secondary">{depth}層</Badge>
            <span className="text-xs text-muted-foreground">{editingFlow.name || editingFlow.id}</span>
          </div>
          <Button
            size="sm"
            variant="outline"
            onClick={() => startAddRoute(flowId)}
            disabled={busy || editingFlow.routes.length >= DTMF_KEYS.length}
          >
            <Plus className="h-4 w-4 mr-1" />
            ルート追加
          </Button>
        </div>

        <div className="rounded-md border p-3 space-y-3">
          <h4 className="text-sm font-medium">メニュー設定（{depth}層）</h4>

          <div className="space-y-2">
            <Label>案内アナウンス（必須）</Label>
            {renderRequiredAnnouncementSelect(
              editingFlow.announcementId,
              (value) =>
                updateFlowById(flowId, (flow) => ({
                  ...flow,
                  announcementId:
                    value === REQUIRED_ANNOUNCEMENT_VALUE ? flow.announcementId : value,
                })),
              busy,
            )}
          </div>

          <div className="grid gap-4 md:grid-cols-2">
            <div className="space-y-2">
              <Label>タイムアウト秒数</Label>
              <Input
                type="number"
                min={1}
                value={editingFlow.timeoutSec}
                onChange={(event) =>
                  updateFlowById(flowId, (flow) => ({
                    ...flow,
                    timeoutSec: Number(event.target.value || 0),
                  }))
                }
                disabled={busy}
              />
            </div>

            <div className="space-y-2">
              <Label>リトライ上限</Label>
              <Input
                type="number"
                min={0}
                value={editingFlow.maxRetries}
                onChange={(event) =>
                  updateFlowById(flowId, (flow) => ({
                    ...flow,
                    maxRetries: Number(event.target.value || 0),
                  }))
                }
                disabled={busy}
              />
            </div>
          </div>

          <div className="grid gap-4 md:grid-cols-2">
            <div className="space-y-2">
              <Label>無効入力時アナウンス</Label>
              {renderAnnouncementSelect(
                editingFlow.invalidInputAnnouncementId,
                (value) =>
                  updateFlowById(flowId, (flow) => ({
                    ...flow,
                    invalidInputAnnouncementId: normalizeAnnouncementValue(value),
                  })),
                busy,
              )}
            </div>
            <div className="space-y-2">
              <Label>タイムアウト時アナウンス</Label>
              {renderAnnouncementSelect(
                editingFlow.timeoutAnnouncementId,
                (value) =>
                  updateFlowById(flowId, (flow) => ({
                    ...flow,
                    timeoutAnnouncementId: normalizeAnnouncementValue(value),
                  })),
                busy,
              )}
            </div>
          </div>
        </div>

        {editingFlow.routes.length === 0 ? (
          <p className="text-sm text-muted-foreground">ルートが未設定です</p>
        ) : (
          <div className="space-y-3">
            {editingFlow.routes.map((route, routeIndex) => {
              const usedKeys = new Set(
                editingFlow.routes
                  .filter((_, index) => index !== routeIndex)
                  .map((item) => item.dtmfKey),
              )
              const selectedIvrFlow =
                route.destination.actionCode === "IV"
                  ? flowById.get(route.destination.ivrFlowId)
                  : null
              const hasCycle =
                route.destination.actionCode === "IV" &&
                selectedIvrFlow &&
                ancestry.includes(selectedIvrFlow.id)

              return (
                <div key={`${flowId}-${route.dtmfKey}-${routeIndex}`} className="rounded-md border p-3 space-y-3">
                  <div className="grid gap-3 md:grid-cols-[120px_1fr_180px_auto] md:items-end">
                    <div className="space-y-2">
                      <Label>キー</Label>
                      <Select
                        value={route.dtmfKey}
                        onValueChange={(value) =>
                          updateRoute(flowId, routeIndex, (current) => ({
                            ...current,
                            dtmfKey: value as DtmfKey,
                          }))
                        }
                        disabled={busy}
                      >
                        <SelectTrigger>
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          {DTMF_KEYS.map((key) => (
                            <SelectItem key={key} value={key} disabled={usedKeys.has(key)}>
                              {key}
                            </SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                    </div>

                    <div className="space-y-2">
                      <Label>ラベル</Label>
                      <Input
                        value={route.label}
                        onChange={(event) =>
                          updateRoute(flowId, routeIndex, (current) => ({
                            ...current,
                            label: event.target.value,
                          }))
                        }
                        disabled={busy}
                      />
                    </div>

                    <div className="space-y-2">
                      <Label>遷移先</Label>
                      <Select
                        value={route.destination.actionCode}
                        onValueChange={(value) =>
                          setRouteDestinationType(
                            flowId,
                            routeIndex,
                            value as IvrTerminalAction["actionCode"],
                            depth,
                          )
                        }
                        disabled={busy}
                      >
                        <SelectTrigger>
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value="VR">転送(VR)</SelectItem>
                          <SelectItem value="VM">留守電(VM)</SelectItem>
                          <SelectItem value="AN">アナウンス→切断(AN)</SelectItem>
                          <SelectItem value="VB">ボイスボット(VB)</SelectItem>
                          <SelectItem
                            value="IV"
                            disabled={!canNestFurther && route.destination.actionCode !== "IV"}
                          >
                            サブIVR(IV)
                          </SelectItem>
                        </SelectContent>
                      </Select>
                    </div>

                    <Button
                      variant="destructive"
                      size="icon"
                      className="h-10 w-10"
                      onClick={() => removeRoute(flowId, routeIndex)}
                      disabled={busy}
                    >
                      <Trash2 className="h-4 w-4" />
                    </Button>
                  </div>

                  {(route.destination.actionCode === "VM" || route.destination.actionCode === "AN") && (
                    <div className="space-y-2">
                      <Label>アナウンス</Label>
                      {renderAnnouncementSelect(
                        route.destination.announcementId,
                        (value) =>
                          updateRoute(flowId, routeIndex, (current) =>
                            current.destination.actionCode === "VM" || current.destination.actionCode === "AN"
                              ? {
                                  ...current,
                                  destination: {
                                    ...current.destination,
                                    announcementId: normalizeAnnouncementValue(value),
                                  },
                                }
                              : current,
                          ),
                        busy,
                      )}
                    </div>
                  )}

                  {route.destination.actionCode === "VB" && (
                    <div className="space-y-3 rounded-md border p-3">
                      <div className="space-y-2">
                        <Label>シナリオ</Label>
                        {renderScenarioSelect(
                          route.destination.scenarioId,
                          (value) =>
                            updateRoute(flowId, routeIndex, (current) =>
                              current.destination.actionCode === "VB"
                                ? {
                                    ...current,
                                    destination: {
                                      ...current.destination,
                                      scenarioId: value === NONE_SCENARIO_VALUE ? "" : value,
                                    },
                                  }
                                : current,
                            ),
                          busy,
                        )}
                        {route.destination.scenarioId &&
                          !scenarioById.has(route.destination.scenarioId) && (
                            <p className="text-xs text-amber-600">
                              （削除済みシナリオ）参照を保持しています
                            </p>
                          )}
                      </div>
                      <div className="space-y-2">
                        <Label>開始前アナウンス</Label>
                        {renderAnnouncementSelect(
                          route.destination.welcomeAnnouncementId,
                          (value) =>
                            updateRoute(flowId, routeIndex, (current) =>
                              current.destination.actionCode === "VB"
                                ? {
                                    ...current,
                                    destination: {
                                      ...current.destination,
                                      welcomeAnnouncementId: normalizeAnnouncementValue(value),
                                    },
                                  }
                                : current,
                            ),
                          busy,
                        )}
                      </div>
                      <div className="h-10 rounded-md border px-3 flex items-center justify-between">
                        <span className="text-sm">録音あり（PoC固定）</span>
                        <Switch checked={route.destination.recordingEnabled} disabled />
                      </div>
                      <div className="h-10 rounded-md border px-3 flex items-center justify-between">
                        <span className="text-sm">includeAnnouncement</span>
                        <Switch
                          checked={route.destination.includeAnnouncement}
                          onCheckedChange={(checked) =>
                            updateRoute(flowId, routeIndex, (current) =>
                              current.destination.actionCode === "VB"
                                ? {
                                    ...current,
                                    destination: {
                                      ...current.destination,
                                      includeAnnouncement: checked,
                                    },
                                  }
                                : current,
                            )
                          }
                          disabled={busy}
                        />
                      </div>
                    </div>
                  )}

                  {route.destination.actionCode === "IV" && (
                    <div className="space-y-2">
                      <Label>次層IVRフロー</Label>
                      <div className="rounded-md border px-3 py-2 text-sm">
                        {selectedIvrFlow
                          ? `${selectedIvrFlow.name || selectedIvrFlow.id} [${nestDepthLabel}層]`
                          : "未作成（保存前または削除済み）"}
                      </div>
                      <p className="text-xs text-muted-foreground">
                        深さ: {depth}層 → {nestDepthLabel}層
                      </p>
                      <div className="flex flex-wrap gap-2">
                        <Button
                          variant="outline"
                          size="sm"
                          onClick={() => createSubFlowForRoute(flowId, routeIndex, depth)}
                          disabled={busy || !canNestFurther}
                        >
                          次層IVRを新規作成
                        </Button>
                      </div>
                      {!canNestFurther && (
                        <p className="text-xs text-amber-600">
                          この層では新しいサブIVRを追加できません（上限 {MAX_IVR_DEPTH} 層）
                        </p>
                      )}
                      {!selectedIvrFlow && route.destination.ivrFlowId && (
                        <p className="text-xs text-amber-600">
                          次層IVRが見つかりません。必要なら「次層IVRを新規作成」で再作成してください
                        </p>
                      )}
                      {hasCycle && selectedIvrFlow && (
                        <p className="text-xs text-amber-600">
                          循環参照になるため展開を停止しています: {selectedIvrFlow.name || selectedIvrFlow.id}
                        </p>
                      )}
                    </div>
                  )}

                  <p className="text-xs text-muted-foreground">
                    {terminalActionLabel(route.destination, flows, scenarioNameById)}
                  </p>

                  {route.destination.actionCode === "IV" &&
                    selectedIvrFlow &&
                    !hasCycle &&
                    canNestFurther &&
                    renderRoutesEditor(
                      selectedIvrFlow.id,
                      depth + 1,
                      [...ancestry, selectedIvrFlow.id],
                      `キー ${route.dtmfKey} (${route.label || "無題"})`,
                    )}
                </div>
              )
            })}
          </div>
        )}

        {routeDraft && (
          <div className="rounded-md border border-dashed p-3 space-y-3">
            <h4 className="text-sm font-medium">ルート追加（{depth}層）</h4>

            <div className="grid gap-3 md:grid-cols-[120px_1fr_180px]">
              <div className="space-y-2">
                <Label>キー</Label>
                <Select
                  value={routeDraft.dtmfKey}
                  onValueChange={(value) =>
                    updateRouteDraft(flowId, (current) => ({
                      ...current,
                      dtmfKey: value as DtmfKey,
                    }))
                  }
                  disabled={busy}
                >
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {DTMF_KEYS.map((key) => {
                      const used = editingFlow.routes.some((route) => route.dtmfKey === key)
                      return (
                        <SelectItem key={key} value={key} disabled={used}>
                          {key}
                        </SelectItem>
                      )
                    })}
                  </SelectContent>
                </Select>
              </div>

              <div className="space-y-2">
                <Label>ラベル</Label>
                <Input
                  value={routeDraft.label}
                  onChange={(event) =>
                    updateRouteDraft(flowId, (current) => ({
                      ...current,
                      label: event.target.value,
                    }))
                  }
                  disabled={busy}
                />
              </div>

              <div className="space-y-2">
                <Label>遷移先</Label>
                <Select
                  value={routeDraft.destinationType}
                  onValueChange={(value) =>
                    updateRouteDraft(flowId, (current) => {
                      if (value === "VM" || value === "AN" || value === "VR") {
                        return {
                          ...current,
                          destinationType: value,
                          announcementId: null,
                          ivrFlowId: null,
                          scenarioId: "",
                          welcomeAnnouncementId: null,
                          recordingEnabled: true,
                          includeAnnouncement: false,
                        }
                      }
                      if (value === "VB") {
                        return {
                          ...current,
                          destinationType: "VB",
                          announcementId: null,
                          ivrFlowId: null,
                          scenarioId: "",
                          welcomeAnnouncementId: null,
                          recordingEnabled: true,
                          includeAnnouncement: false,
                        }
                      }
                      if (!canNestFurther) {
                        return current
                      }
                      return {
                        ...current,
                        destinationType: "IV",
                        announcementId: null,
                        ivrFlowId: null,
                        scenarioId: "",
                        welcomeAnnouncementId: null,
                        recordingEnabled: true,
                        includeAnnouncement: false,
                      }
                    })
                  }
                  disabled={busy}
                >
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="VR">転送(VR)</SelectItem>
                    <SelectItem value="VM">留守電(VM)</SelectItem>
                    <SelectItem value="AN">アナウンス→切断(AN)</SelectItem>
                    <SelectItem value="VB">ボイスボット(VB)</SelectItem>
                    <SelectItem value="IV" disabled={!canNestFurther}>
                      サブIVR(IV)
                    </SelectItem>
                  </SelectContent>
                </Select>
              </div>
            </div>

            {(routeDraft.destinationType === "VM" || routeDraft.destinationType === "AN") && (
              <div className="space-y-2">
                <Label>アナウンス</Label>
                {renderAnnouncementSelect(
                  routeDraft.announcementId,
                  (value) =>
                    updateRouteDraft(flowId, (current) => ({
                      ...current,
                      announcementId: normalizeAnnouncementValue(value),
                    })),
                  busy,
                )}
              </div>
            )}

            {routeDraft.destinationType === "VB" && (
              <div className="space-y-3 rounded-md border p-3">
                <div className="space-y-2">
                  <Label>シナリオ</Label>
                  {renderScenarioSelect(
                    routeDraft.scenarioId,
                    (value) =>
                      updateRouteDraft(flowId, (current) => ({
                        ...current,
                        scenarioId: value === NONE_SCENARIO_VALUE ? "" : value,
                      })),
                    busy,
                  )}
                </div>
                <div className="space-y-2">
                  <Label>開始前アナウンス</Label>
                  {renderAnnouncementSelect(
                    routeDraft.welcomeAnnouncementId,
                    (value) =>
                      updateRouteDraft(flowId, (current) => ({
                        ...current,
                        welcomeAnnouncementId: normalizeAnnouncementValue(value),
                      })),
                    busy,
                  )}
                </div>
                <div className="h-10 rounded-md border px-3 flex items-center justify-between">
                  <span className="text-sm">録音あり（PoC固定）</span>
                  <Switch checked={routeDraft.recordingEnabled} disabled />
                </div>
                <div className="h-10 rounded-md border px-3 flex items-center justify-between">
                  <span className="text-sm">includeAnnouncement</span>
                  <Switch
                    checked={routeDraft.includeAnnouncement}
                    onCheckedChange={(checked) =>
                      updateRouteDraft(flowId, (current) => ({
                        ...current,
                        includeAnnouncement: checked,
                      }))
                    }
                    disabled={busy}
                  />
                </div>
              </div>
            )}

            {routeDraft.destinationType === "IV" && (
              <div className="space-y-2">
                <Label>次層IVRフロー</Label>
                <div className="rounded-md border px-3 py-2 text-sm">
                  未作成（このルートを追加した時点で自動作成）
                </div>
                <p className="text-xs text-muted-foreground">
                  深さ: {depth}層 → {nestDepthLabel}層
                </p>
                <p className="text-xs text-muted-foreground">
                  参照選択は不要です。`追加` の実行時にだけ次層IVRを作成します。
                </p>
                {!canNestFurther && (
                  <p className="text-xs text-amber-600">
                    この層では新しいサブIVRを追加できません（上限 {MAX_IVR_DEPTH} 層）
                  </p>
                )}
              </div>
            )}

            <div className="flex items-center justify-end gap-2">
              <Button variant="outline" onClick={() => clearRouteDraft(flowId)} disabled={busy}>
                <X className="h-4 w-4 mr-1" />
                キャンセル
              </Button>
              <Button onClick={() => addRoute(flowId, depth)} disabled={busy}>
                <Plus className="h-4 w-4 mr-1" />
                追加
              </Button>
            </div>
          </div>
        )}
      </div>
      </div>
    )
  }

  if (loading) {
    return (
      <div className="p-6">
        <p className="text-muted-foreground">読み込み中...</p>
      </div>
    )
  }

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-center justify-between gap-3">
        <div>
          <h1 className="text-2xl font-bold text-balance">IVRフロー</h1>
          <p className="text-muted-foreground">DTMF メニュー定義</p>
        </div>

        <div className="flex items-center gap-2">
          <Button variant="outline" onClick={cancelAll} disabled={busy}>
            <X className="h-4 w-4 mr-1" />
            キャンセル
          </Button>
          <Button onClick={saveAll} disabled={busy}>
            <Save className="h-4 w-4 mr-1" />
            保存
          </Button>
        </div>
      </div>

      {errorMessage && (
        <div className="rounded-md border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive whitespace-pre-line flex items-start gap-2">
          <AlertTriangle className="h-4 w-4 mt-0.5 shrink-0" />
          <span>{errorMessage}</span>
        </div>
      )}

      {infoMessage && (
        <div className="rounded-md border border-emerald-500/30 bg-emerald-500/10 p-3 text-sm text-emerald-700 dark:text-emerald-300 flex items-start gap-2">
          <CheckCircle2 className="h-4 w-4 mt-0.5" />
          <span>{infoMessage}</span>
        </div>
      )}

      <div className="grid gap-6 xl:grid-cols-[360px_1fr]">
        <Card>
          <CardHeader className="space-y-3">
            <div className="flex items-center justify-between gap-2">
              <CardTitle className="text-lg">トップフロー一覧</CardTitle>
              <Button size="sm" onClick={createFlow} disabled={busy}>
                <Plus className="h-4 w-4 mr-1" />
                新規
              </Button>
            </div>

            <div className="relative">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
              <Input
                value={searchQuery}
                onChange={(event) => setSearchQuery(event.target.value)}
                placeholder="フロー名で検索"
                className="pl-9"
              />
            </div>
          </CardHeader>
          <CardContent className="space-y-3">
            <p className="text-xs text-muted-foreground">
              {rootFlows.length > 0
                ? "子IVRは一覧に出さず、DTMFツリー内で編集します。"
                : "トップフローがないため、全フローを表示しています。"}
            </p>
            <ScrollArea className="h-[420px] rounded-md border">
              <div className="p-2 space-y-1">
                {listFlows.length === 0 ? (
                  <p className="text-sm text-muted-foreground p-2">該当するフローがありません</p>
                ) : (
                  listFlows.map((flow) => (
                    <button
                      key={flow.id}
                      type="button"
                      onClick={() => {
                        setSelectedFlowId(flow.id)
                        setErrorMessage(null)
                      }}
                      className={cn(
                        "w-full text-left px-3 py-2 rounded-md border transition-colors",
                        selectedFlowId === flow.id
                          ? "border-primary bg-primary/10"
                          : "border-transparent hover:bg-accent",
                      )}
                    >
                      <div className="font-medium truncate">{flow.name || "(名称未設定)"}</div>
                      <div className="text-xs text-muted-foreground flex items-center justify-between">
                        <span>{flow.description ?? "説明なし"}</span>
                        <Badge variant={flow.isActive ? "default" : "secondary"}>
                          {flow.isActive ? "有効" : "無効"}
                        </Badge>
                      </div>
                    </button>
                  ))
                )}
              </div>
            </ScrollArea>

            <div className="grid grid-cols-2 gap-2">
              <Button variant="outline" onClick={duplicateSelectedFlow} disabled={!selectedFlow || busy}>
                <Copy className="h-4 w-4 mr-1" />
                複製
              </Button>
              <Button variant="destructive" onClick={deleteSelectedFlow} disabled={!selectedFlow || busy}>
                <Trash2 className="h-4 w-4 mr-1" />
                削除
              </Button>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="text-lg">フロー詳細 / 編集</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            {selectedFlow ? (
              <>
                <div className="grid gap-4 md:grid-cols-2">
                  <div className="space-y-2">
                    <Label htmlFor="flow-name">フロー名</Label>
                    <Input
                      id="flow-name"
                      value={selectedFlow.name}
                      onChange={(event) =>
                        updateSelectedFlow((flow) => ({ ...flow, name: event.target.value }))
                      }
                      disabled={busy}
                    />
                  </div>
                  <div className="space-y-2">
                    <Label>有効/無効</Label>
                    <div className="h-10 rounded-md border px-3 flex items-center justify-between">
                      <span className="text-sm">このフローを有効にする</span>
                      <Switch
                        checked={selectedFlow.isActive}
                        onCheckedChange={(checked) =>
                          updateSelectedFlow((flow) => ({ ...flow, isActive: checked }))
                        }
                        disabled={busy}
                      />
                    </div>
                  </div>
                </div>

                <div className="space-y-2">
                  <Label htmlFor="flow-description">説明</Label>
                  <Input
                    id="flow-description"
                    value={selectedFlow.description ?? ""}
                    onChange={(event) =>
                      updateSelectedFlow((flow) => ({
                        ...flow,
                        description: event.target.value.trim().length > 0 ? event.target.value : null,
                      }))
                    }
                    placeholder="任意"
                    disabled={busy}
                  />
                </div>

                {renderRoutesEditor(selectedFlow.id, 1, [selectedFlow.id])}

                <div className="rounded-md border p-3 space-y-3">
                  <h3 className="font-medium">リトライ超過時の遷移先</h3>

                  <div className="grid gap-4 md:grid-cols-2">
                    <div className="space-y-2">
                      <Label>fallback</Label>
                      <Select
                        value={selectedFlow.fallbackAction.actionCode}
                        onValueChange={(value) =>
                          updateSelectedFlow((flow) => ({
                            ...flow,
                            fallbackAction: fallbackDestinationFromCode(
                              value as "VR" | "VM" | "AN" | "VB",
                            ),
                          }))
                        }
                        disabled={busy}
                      >
                        <SelectTrigger>
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value="VR">転送(VR)</SelectItem>
                          <SelectItem value="VM">留守電(VM)</SelectItem>
                          <SelectItem value="AN">アナウンス→切断(AN)</SelectItem>
                          <SelectItem value="VB">ボイスボット(VB)</SelectItem>
                        </SelectContent>
                      </Select>
                    </div>

                    {(selectedFlow.fallbackAction.actionCode === "VM" ||
                      selectedFlow.fallbackAction.actionCode === "AN") && (
                      <div className="space-y-2">
                        <Label>アナウンス</Label>
                        {renderAnnouncementSelect(
                          selectedFlow.fallbackAction.announcementId,
                          (value) =>
                            updateSelectedFlow((flow) => ({
                              ...flow,
                              fallbackAction:
                                flow.fallbackAction.actionCode === "VM" ||
                                flow.fallbackAction.actionCode === "AN"
                                  ? {
                                      ...flow.fallbackAction,
                                      announcementId: normalizeAnnouncementValue(value),
                                    }
                                  : flow.fallbackAction,
                            })),
                          busy,
                        )}
                      </div>
                    )}

                    {selectedFlow.fallbackAction.actionCode === "VB" && (
                      <div className="space-y-3 md:col-span-2 rounded-md border p-3">
                        <div className="space-y-2">
                          <Label>シナリオ</Label>
                          {renderScenarioSelect(
                            selectedFlow.fallbackAction.scenarioId,
                            (value) =>
                              updateSelectedFlow((flow) => ({
                                ...flow,
                                fallbackAction:
                                  flow.fallbackAction.actionCode === "VB"
                                    ? {
                                        ...flow.fallbackAction,
                                        scenarioId: value === NONE_SCENARIO_VALUE ? "" : value,
                                      }
                                    : flow.fallbackAction,
                              })),
                            busy,
                          )}
                          {selectedFlow.fallbackAction.scenarioId &&
                            !scenarioById.has(selectedFlow.fallbackAction.scenarioId) && (
                              <p className="text-xs text-amber-600">
                                （削除済みシナリオ）参照を保持しています
                              </p>
                            )}
                        </div>
                        <div className="space-y-2">
                          <Label>開始前アナウンス</Label>
                          {renderAnnouncementSelect(
                            selectedFlow.fallbackAction.welcomeAnnouncementId,
                            (value) =>
                              updateSelectedFlow((flow) => ({
                                ...flow,
                                fallbackAction:
                                  flow.fallbackAction.actionCode === "VB"
                                    ? {
                                        ...flow.fallbackAction,
                                        welcomeAnnouncementId: normalizeAnnouncementValue(value),
                                      }
                                    : flow.fallbackAction,
                              })),
                            busy,
                          )}
                        </div>
                        <div className="h-10 rounded-md border px-3 flex items-center justify-between">
                          <span className="text-sm">録音あり（PoC固定）</span>
                          <Switch checked={selectedFlow.fallbackAction.recordingEnabled} disabled />
                        </div>
                        <div className="h-10 rounded-md border px-3 flex items-center justify-between">
                          <span className="text-sm">includeAnnouncement</span>
                          <Switch
                            checked={selectedFlow.fallbackAction.includeAnnouncement}
                            onCheckedChange={(checked) =>
                              updateSelectedFlow((flow) => ({
                                ...flow,
                                fallbackAction:
                                  flow.fallbackAction.actionCode === "VB"
                                    ? {
                                        ...flow.fallbackAction,
                                        includeAnnouncement: checked,
                                      }
                                    : flow.fallbackAction,
                              }))
                            }
                            disabled={busy}
                          />
                        </div>
                      </div>
                    )}
                  </div>
                </div>

                <p className="text-xs text-muted-foreground">
                  ネスト上限: {MAX_IVR_DEPTH}層（保存時に検証）
                </p>
              </>
            ) : (
              <p className="text-sm text-muted-foreground">IVR フローを選択してください</p>
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  )
}
