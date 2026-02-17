"use client"

import { useEffect, useMemo, useRef, useState, type ReactNode } from "react"
import {
  AlertTriangle,
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  Clock3,
  Copy,
  FolderTree,
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
  buildBreadcrumb,
  buildIvrTree,
  cloneIvrFlow,
  createDefaultIvrFlow,
  terminalActionLabel,
  validateIvrFlows,
  type BreadcrumbItem,
  type DtmfKey,
  type IvrFallbackAction,
  type IvrFlowDefinition,
  type IvrRoute,
  type IvrTerminalAction,
  type IvrTreeNode,
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
type SelectedSection = "basic" | "routes" | "invalid" | "timeout" | "fallback" | null

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

function flowDisplayName(flow: IvrFlowDefinition): string {
  const name = flow.name.trim()
  return name.length > 0 ? name : flow.id
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

function flowNodeKey(flowId: string): string {
  return `flow:${flowId}`
}

function validSlotNodeKey(flowId: string): string {
  return `slot:valid:${flowId}`
}

function flowIdFromTreeKey(key: string): string | null {
  if (key.startsWith("flow:")) {
    return key.slice("flow:".length)
  }
  if (key.startsWith("slot:valid:")) {
    return key.slice("slot:valid:".length)
  }
  return null
}

function isFlowSearchMatched(flow: IvrFlowDefinition, keyword: string): boolean {
  const text = [flow.name, flow.description].filter(Boolean).join(" ").toLowerCase()
  return text.includes(keyword)
}

function routeLabel(route: IvrRoute): string {
  return route.label.trim().length > 0 ? route.label : "無題"
}

export function IvrContent() {
  const [flows, setFlows] = useState<IvrFlowDefinition[]>([])
  const [savedFlows, setSavedFlows] = useState<IvrFlowDefinition[]>([])
  const [selectedFlowId, setSelectedFlowId] = useState<string | null>(null)
  const [selectedSection, setSelectedSection] = useState<SelectedSection>("basic")
  const [focusRouteIndex, setFocusRouteIndex] = useState<number | null>(null)
  const [treeExpandedNodes, setTreeExpandedNodes] = useState<Set<string>>(new Set())
  const [announcements, setAnnouncements] = useState<StoredAnnouncement[]>([])
  const [scenarios, setScenarios] = useState<VoicebotScenario[]>([])
  const [searchQuery, setSearchQuery] = useState("")
  const [routeDrafts, setRouteDrafts] = useState<RouteDrafts>({})

  const [loading, setLoading] = useState(true)
  const [busy, setBusy] = useState(false)
  const [errorMessage, setErrorMessage] = useState<string | null>(null)
  const [infoMessage, setInfoMessage] = useState<string | null>(null)

  const [highlightedSection, setHighlightedSection] = useState<SelectedSection>(null)
  const [highlightedRouteIndex, setHighlightedRouteIndex] = useState<number | null>(null)

  const basicSectionRef = useRef<HTMLDivElement | null>(null)
  const routesSectionRef = useRef<HTMLDivElement | null>(null)
  const invalidSectionRef = useRef<HTMLDivElement | null>(null)
  const timeoutSectionRef = useRef<HTMLDivElement | null>(null)
  const fallbackSectionRef = useRef<HTMLDivElement | null>(null)
  const routeRowRefs = useRef<Record<number, HTMLDivElement | null>>({})

  const selectedFlow = useMemo(
    () => flows.find((flow) => flow.id === selectedFlowId) ?? null,
    [flows, selectedFlowId],
  )

  const rootFlows = useMemo(() => collectRootFlows(flows), [flows])

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

  const breadcrumb = useMemo<BreadcrumbItem[]>(() => {
    if (!selectedFlowId) {
      return []
    }
    return buildBreadcrumb(selectedFlowId, flows)
  }, [selectedFlowId, flows])

  const selectedFlowDepth = Math.max(1, breadcrumb.length)

  const normalizedSearchQuery = searchQuery.trim().toLowerCase()
  const hasSearchQuery = normalizedSearchQuery.length > 0

  const visibleFlowIds = useMemo(() => {
    if (!hasSearchQuery) {
      return new Set(flows.map((flow) => flow.id))
    }

    const visible = new Set<string>()
    for (const flow of flows) {
      if (!isFlowSearchMatched(flow, normalizedSearchQuery)) {
        continue
      }
      const path = buildBreadcrumb(flow.id, flows)
      if (path.length === 0) {
        visible.add(flow.id)
        continue
      }
      for (const item of path) {
        visible.add(item.flowId)
      }
    }
    return visible
  }, [flows, hasSearchQuery, normalizedSearchQuery])

  const autoExpandedNodes = useMemo(() => {
    const autoExpanded = new Set<string>()
    if (!hasSearchQuery) {
      return autoExpanded
    }
    for (const flowId of visibleFlowIds) {
      autoExpanded.add(flowNodeKey(flowId))
      autoExpanded.add(validSlotNodeKey(flowId))
    }
    return autoExpanded
  }, [hasSearchQuery, visibleFlowIds])

  const effectiveExpandedNodes = useMemo(() => {
    const merged = new Set<string>(treeExpandedNodes)
    for (const key of autoExpandedNodes) {
      merged.add(key)
    }
    return merged
  }, [treeExpandedNodes, autoExpandedNodes])

  const treeRoots = useMemo(() => {
    const rootCandidates = rootFlows.length > 0 ? rootFlows : flows
    const allFlows = new Map(flows.map((flow) => [flow.id, flow]))

    const nodes: IvrTreeNode[] = []
    for (const root of rootCandidates) {
      const tree = buildIvrTree(flows, root.id, allFlows)
      if (!tree) {
        continue
      }
      if (hasSearchQuery && !visibleFlowIds.has(tree.flowId)) {
        continue
      }
      nodes.push(tree)
    }
    return nodes
  }, [flows, hasSearchQuery, rootFlows, visibleFlowIds])

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

        const loadedRoots = collectRootFlows(loadedFlows)
        const initialSelectedId = loadedRoots[0]?.id ?? loadedFlows[0]?.id ?? null

        setFlows(cloneFlows(loadedFlows))
        setSavedFlows(cloneFlows(loadedFlows))
        setSelectedFlowId(initialSelectedId)
        setSelectedSection("basic")
        setFocusRouteIndex(null)
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
      const fallbackRoots = rootFlows.length > 0 ? rootFlows : flows
      if (fallbackRoots[0]) {
        setSelectedFlowId(fallbackRoots[0].id)
      }
      return
    }

    if (!flowById.has(selectedFlowId)) {
      const fallbackRoots = rootFlows.length > 0 ? rootFlows : flows
      setSelectedFlowId(fallbackRoots[0]?.id ?? null)
      setSelectedSection("basic")
      setFocusRouteIndex(null)
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

  useEffect(() => {
    const validFlowIds = new Set(flows.map((flow) => flow.id))
    setTreeExpandedNodes((prev) => {
      const next = new Set<string>()
      for (const key of prev) {
        const flowId = flowIdFromTreeKey(key)
        if (flowId && validFlowIds.has(flowId)) {
          next.add(key)
        }
      }

      if (next.size === 0 && flows.length > 0) {
        const roots = rootFlows.length > 0 ? rootFlows : flows
        for (const flow of roots) {
          next.add(flowNodeKey(flow.id))
          next.add(validSlotNodeKey(flow.id))
        }
      }

      if (next.size === prev.size) {
        let identical = true
        for (const key of next) {
          if (!prev.has(key)) {
            identical = false
            break
          }
        }
        if (identical) {
          return prev
        }
      }
      return next
    })
  }, [flows, rootFlows])

  useEffect(() => {
    routeRowRefs.current = {}
  }, [selectedFlowId])

  useEffect(() => {
    if (!selectedFlowId || !selectedSection) {
      return
    }

    const sectionElement =
      selectedSection === "basic"
        ? basicSectionRef.current
        : selectedSection === "routes"
          ? routesSectionRef.current
          : selectedSection === "invalid"
            ? invalidSectionRef.current
            : selectedSection === "timeout"
              ? timeoutSectionRef.current
              : fallbackSectionRef.current

    if (!sectionElement) {
      return
    }

    sectionElement.scrollIntoView({ behavior: "smooth", block: "start" })
    setHighlightedSection(selectedSection)

    const timer = window.setTimeout(() => {
      setHighlightedSection((current) => (current === selectedSection ? null : current))
    }, 1200)

    return () => {
      window.clearTimeout(timer)
    }
  }, [selectedFlowId, selectedSection])

  useEffect(() => {
    if (!selectedFlowId || selectedSection !== "routes" || focusRouteIndex === null) {
      return
    }

    const row = routeRowRefs.current[focusRouteIndex]
    if (!row) {
      return
    }

    row.scrollIntoView({ behavior: "smooth", block: "center" })
    setHighlightedRouteIndex(focusRouteIndex)

    const timer = window.setTimeout(() => {
      setHighlightedRouteIndex((current) => (current === focusRouteIndex ? null : current))
    }, 1200)

    return () => {
      window.clearTimeout(timer)
    }
  }, [focusRouteIndex, selectedFlow?.routes.length, selectedFlowId, selectedSection])

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
    setTreeExpandedNodes((prev) => {
      const next = new Set(prev)
      next.add(flowNodeKey(flowId))
      next.add(validSlotNodeKey(flowId))
      return next
    })
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

  const selectNode = (flowId: string, section: SelectedSection, routeIndex?: number) => {
    setSelectedFlowId(flowId)
    setSelectedSection(section)
    setFocusRouteIndex(section === "routes" ? routeIndex ?? null : null)
    setErrorMessage(null)
  }

  const createFlow = () => {
    const newFlow = createDefaultIvrFlow()
    setFlows((prev) => [...prev, newFlow])
    setSelectedFlowId(newFlow.id)
    setSelectedSection("basic")
    setFocusRouteIndex(null)
    setTreeExpandedNodes((prev) => {
      const next = new Set(prev)
      next.add(flowNodeKey(newFlow.id))
      next.add(validSlotNodeKey(newFlow.id))
      return next
    })
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
    setSelectedSection("basic")
    setFocusRouteIndex(null)
    setTreeExpandedNodes((prev) => {
      const next = new Set(prev)
      next.add(flowNodeKey(copied.id))
      next.add(validSlotNodeKey(copied.id))
      return next
    })
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
    setSelectedSection("basic")
    setFocusRouteIndex(null)
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

    if (routeDraft.destinationType === "IV" && depth >= MAX_IVR_DEPTH) {
      setErrorMessage(`ネスト上限(${MAX_IVR_DEPTH}層)のため、サブIVRを指定できません`)
      return
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

    setTreeExpandedNodes((prev) => {
      const next = new Set(prev)
      next.add(flowNodeKey(flowId))
      next.add(validSlotNodeKey(flowId))
      return next
    })
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
    setTreeExpandedNodes((prev) => {
      const next = new Set(prev)
      next.add(flowNodeKey(flowId))
      next.add(validSlotNodeKey(flowId))
      return next
    })
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
    const selectedExists = selectedFlowId !== null && restored.some((flow) => flow.id === selectedFlowId)

    if (selectedExists && selectedFlowId) {
      setSelectedFlowId(selectedFlowId)
    } else {
      setSelectedFlowId(restoredRoots[0]?.id ?? restored[0]?.id ?? null)
    }

    setSelectedSection("basic")
    setFocusRouteIndex(null)
    setRouteDrafts({})
    setErrorMessage(null)
    setInfoMessage("変更を取り消しました")
  }

  const toggleExpandNode = (nodeKey: string) => {
    setTreeExpandedNodes((prev) => {
      const next = new Set(prev)
      if (next.has(nodeKey)) {
        next.delete(nodeKey)
      } else {
        next.add(nodeKey)
      }
      return next
    })
  }

  const onBreadcrumbRouteClick = (
    parentFlowId: string,
    childFlowId: string,
    viaRoute: { dtmfKey: DtmfKey; label: string },
  ) => {
    const parentFlow = flowById.get(parentFlowId)
    if (!parentFlow) {
      return
    }

    const routeIndex = parentFlow.routes.findIndex(
      (route) =>
        route.destination.actionCode === "IV" &&
        route.destination.ivrFlowId === childFlowId &&
        route.dtmfKey === viaRoute.dtmfKey,
    )

    selectNode(parentFlowId, "routes", routeIndex >= 0 ? routeIndex : undefined)
  }

  const sectionClass = (section: Exclude<SelectedSection, null>) =>
    cn(
      "rounded-md border p-3 space-y-3 transition-colors",
      highlightedSection === section && "bg-primary/5 ring-2 ring-primary/30",
    )

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

  const renderTreeNode = (node: IvrTreeNode, level: number): ReactNode => {
    if (node.type === "flow") {
      if (hasSearchQuery && !visibleFlowIds.has(node.flowId)) {
        return null
      }

      const flow = flowById.get(node.flowId)
      if (!flow) {
        return null
      }

      const nodeKey = flowNodeKey(node.flowId)
      const expanded = effectiveExpandedNodes.has(nodeKey)
      const selected = selectedFlowId === node.flowId && selectedSection === "basic"

      return (
        <div key={nodeKey} className="space-y-1">
          <div className="flex items-center gap-1" style={{ paddingLeft: `${8 + level * 14}px` }}>
            {node.children.length > 0 ? (
              <button
                type="button"
                className="inline-flex h-6 w-6 shrink-0 items-center justify-center rounded-sm hover:bg-muted"
                onClick={() => toggleExpandNode(nodeKey)}
              >
                {expanded ? <ChevronDown className="h-3.5 w-3.5" /> : <ChevronRight className="h-3.5 w-3.5" />}
              </button>
            ) : (
              <span className="inline-block h-6 w-6 shrink-0" />
            )}
            <button
              type="button"
              onClick={() => selectNode(node.flowId, "basic")}
              className={cn(
                "w-full rounded-md border px-2 py-1.5 text-left transition-colors",
                selected ? "border-primary bg-primary/10" : "border-transparent hover:bg-accent",
              )}
            >
              <div className="flex items-center gap-2">
                <span className="truncate font-medium">{flowDisplayName(flow)}</span>
                <Badge variant="secondary">{flow.routes.length}件</Badge>
                {node.depth > 1 && <Badge variant="outline">Lv.{node.depth}</Badge>}
                {!flow.isActive && <Badge variant="outline">無効</Badge>}
                {node.meta.hasWarning && (
                  <Badge variant="outline" className="text-amber-700 border-amber-400/70">
                    ⚠ アナウンス未設定
                  </Badge>
                )}
              </div>
            </button>
          </div>

          {expanded && (
            <div className="space-y-1">
              {node.children.map((child) => renderTreeNode(child, level + 1))}
            </div>
          )}
        </div>
      )
    }

    if (node.type === "valid-slot") {
      const nodeKey = validSlotNodeKey(node.flowId)
      const expanded = effectiveExpandedNodes.has(nodeKey)
      const selected =
        selectedFlowId === node.flowId &&
        selectedSection === "routes" &&
        focusRouteIndex === null

      return (
        <div key={nodeKey} className="space-y-1">
          <div className="flex items-center gap-1" style={{ paddingLeft: `${8 + level * 14}px` }}>
            <button
              type="button"
              className="inline-flex h-6 w-6 shrink-0 items-center justify-center rounded-sm hover:bg-muted"
              onClick={() => toggleExpandNode(nodeKey)}
            >
              {expanded ? <ChevronDown className="h-3.5 w-3.5" /> : <ChevronRight className="h-3.5 w-3.5" />}
            </button>
            <button
              type="button"
              onClick={() => selectNode(node.flowId, "routes")}
              className={cn(
                "w-full rounded-md border px-2 py-1.5 text-left transition-colors",
                selected ? "border-primary bg-primary/10" : "border-transparent hover:bg-accent",
              )}
            >
              <div className="flex items-center gap-2">
                <CheckCircle2 className="h-4 w-4 text-emerald-600" />
                <span className="font-medium">Valid Input</span>
                <Badge variant="secondary">{node.children.length}ルート</Badge>
              </div>
            </button>
          </div>

          {expanded && (
            <div className="space-y-1">
              {node.children.map((child) => renderTreeNode(child, level + 1))}
            </div>
          )}
        </div>
      )
    }

    if (node.type === "invalid-slot") {
      const flow = flowById.get(node.flowId)
      if (!flow) {
        return null
      }

      const selected = selectedFlowId === node.flowId && selectedSection === "invalid"
      const hasAnnouncement =
        flow.invalidInputAnnouncementId && announcementById.has(flow.invalidInputAnnouncementId)
      const announcementLabel = hasAnnouncement
        ? announcementById.get(flow.invalidInputAnnouncementId ?? "")?.name
        : "ガイダンスなし"

      return (
        <button
          key={`slot:invalid:${node.flowId}`}
          type="button"
          onClick={() => selectNode(node.flowId, "invalid")}
          className={cn(
            "w-full rounded-md border px-2 py-1.5 text-left transition-colors",
            selected ? "border-primary bg-primary/10" : "border-transparent hover:bg-accent",
          )}
          style={{ paddingLeft: `${8 + level * 14}px` }}
        >
          <div className="flex items-center gap-2">
            <X className="h-4 w-4 text-rose-600" />
            <span className="font-medium">Invalid Input</span>
            <span className="text-xs text-muted-foreground truncate">{announcementLabel}</span>
          </div>
        </button>
      )
    }

    if (node.type === "timeout-slot") {
      const flow = flowById.get(node.flowId)
      if (!flow) {
        return null
      }

      const selected = selectedFlowId === node.flowId && selectedSection === "timeout"
      const hasAnnouncement =
        flow.timeoutAnnouncementId && announcementById.has(flow.timeoutAnnouncementId)
      const announcementLabel = hasAnnouncement
        ? announcementById.get(flow.timeoutAnnouncementId ?? "")?.name
        : "ガイダンスなし"

      return (
        <button
          key={`slot:timeout:${node.flowId}`}
          type="button"
          onClick={() => selectNode(node.flowId, "timeout")}
          className={cn(
            "w-full rounded-md border px-2 py-1.5 text-left transition-colors",
            selected ? "border-primary bg-primary/10" : "border-transparent hover:bg-accent",
          )}
          style={{ paddingLeft: `${8 + level * 14}px` }}
        >
          <div className="flex items-center gap-2">
            <Clock3 className="h-4 w-4 text-amber-600" />
            <span className="font-medium">Timeout</span>
            <span className="text-xs text-muted-foreground truncate">{announcementLabel}</span>
          </div>
        </button>
      )
    }

    const flow = flowById.get(node.flowId)
    const routeIndex = node.routeIndex ?? -1
    const route = routeIndex >= 0 ? flow?.routes[routeIndex] : undefined
    if (!flow || !route) {
      return null
    }

    const selected =
      selectedFlowId === node.flowId &&
      selectedSection === "routes" &&
      focusRouteIndex === routeIndex

    return (
      <div key={`route:${node.flowId}:${routeIndex}`} className="space-y-1">
        <button
          type="button"
          onClick={() => selectNode(node.flowId, "routes", routeIndex)}
          className={cn(
            "w-full rounded-md border px-2 py-1.5 text-left transition-colors",
            selected ? "border-primary bg-primary/10" : "border-transparent hover:bg-accent",
          )}
          style={{ paddingLeft: `${8 + level * 14}px` }}
        >
          <div className="flex items-center gap-2">
            <span className="font-mono text-xs text-muted-foreground">{route.dtmfKey}</span>
            <span className="truncate font-medium">{routeLabel(route)}</span>
            <span className="text-xs text-muted-foreground truncate">
              {node.meta.actionLabel ?? terminalActionLabel(route.destination, flows, scenarioNameById)}
            </span>
            {node.meta.hasWarning && (
              <AlertTriangle className="h-3.5 w-3.5 text-amber-600" />
            )}
          </div>
        </button>
        {node.children.map((child) => renderTreeNode(child, level + 1))}
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

  const selectedRouteDraft = selectedFlow ? routeDrafts[selectedFlow.id] ?? null : null
  const canNestFurther = selectedFlowDepth < MAX_IVR_DEPTH
  const nestDepthLabel = Math.min(selectedFlowDepth + 1, MAX_IVR_DEPTH)

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
              <CardTitle className="text-lg flex items-center gap-2">
                <FolderTree className="h-5 w-5" />
                IVR ツリー
              </CardTitle>
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
              {hasSearchQuery
                ? "一致フローとその祖先のみ表示しています。"
                : "Valid / Invalid / Timeout の3分岐をツリーで表示します。"}
            </p>
            <ScrollArea className="h-[620px] rounded-md border">
              <div className="p-2 space-y-1">
                {treeRoots.length === 0 ? (
                  <p className="text-sm text-muted-foreground p-2">該当するフローがありません</p>
                ) : (
                  treeRoots.map((root) => renderTreeNode(root, 0))
                )}
              </div>
            </ScrollArea>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="text-lg">フロー詳細 / 編集</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            {selectedFlow ? (
              <>
                <div className="rounded-md border bg-muted/30 px-3 py-2">
                  <div className="flex flex-wrap items-center gap-1 text-sm">
                    {breadcrumb.length === 0 ? (
                      <span className="font-medium">{flowDisplayName(selectedFlow)}</span>
                    ) : (
                      breadcrumb.map((item, index) => {
                        const isLast = index === breadcrumb.length - 1
                        const parent = index > 0 ? breadcrumb[index - 1] : null
                        const viaRoute = item.viaRoute
                        return (
                          <div key={`crumb:${item.flowId}:${index}`} className="flex items-center gap-1">
                            {index > 0 && <span className="text-muted-foreground">&gt;</span>}

                            {index > 0 && viaRoute && parent && (
                              <>
                                <button
                                  type="button"
                                  className="text-primary hover:underline"
                                  onClick={() =>
                                    onBreadcrumbRouteClick(parent.flowId, item.flowId, viaRoute)
                                  }
                                >
                                  Key {viaRoute.dtmfKey}: {viaRoute.label || "無題"}
                                </button>
                                <span className="text-muted-foreground">&gt;</span>
                              </>
                            )}

                            {isLast ? (
                              <span className="font-medium">{item.flowName}</span>
                            ) : (
                              <button
                                type="button"
                                className="text-primary hover:underline"
                                onClick={() => selectNode(item.flowId, "basic")}
                              >
                                {item.flowName}
                              </button>
                            )}
                          </div>
                        )
                      })
                    )}
                  </div>
                </div>

                <div ref={basicSectionRef} className={sectionClass("basic")}>
                  <h3 className="font-medium">① 基本情報</h3>

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
                </div>

                <div className="rounded-md border p-3 space-y-3">
                  <h3 className="font-medium">② メニュー設定</h3>

                  <div className="space-y-2">
                    <Label>案内アナウンス（必須）</Label>
                    {renderRequiredAnnouncementSelect(
                      selectedFlow.announcementId,
                      (value) =>
                        updateSelectedFlow((flow) => ({
                          ...flow,
                          announcementId: value === REQUIRED_ANNOUNCEMENT_VALUE ? flow.announcementId : value,
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
                        value={selectedFlow.timeoutSec}
                        onChange={(event) =>
                          updateSelectedFlow((flow) => ({
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
                        value={selectedFlow.maxRetries}
                        onChange={(event) =>
                          updateSelectedFlow((flow) => ({
                            ...flow,
                            maxRetries: Number(event.target.value || 0),
                          }))
                        }
                        disabled={busy}
                      />
                    </div>
                  </div>
                </div>

                <div ref={routesSectionRef} className={sectionClass("routes")}>
                  <div className="flex items-center justify-between gap-2">
                    <h3 className="font-medium">③ DTMF ルート</h3>
                    <Badge variant="secondary">{selectedFlowDepth}層</Badge>
                  </div>

                  <div className="flex justify-end">
                    <Button
                      size="sm"
                      variant="outline"
                      onClick={() => startAddRoute(selectedFlow.id)}
                      disabled={busy || selectedFlow.routes.length >= DTMF_KEYS.length}
                    >
                      <Plus className="h-4 w-4 mr-1" />
                      ルート追加
                    </Button>
                  </div>

                  {selectedFlow.routes.length === 0 ? (
                    <p className="text-sm text-muted-foreground">ルートが未設定です</p>
                  ) : (
                    <div className="space-y-3">
                      {selectedFlow.routes.map((route, routeIndex) => {
                        const usedKeys = new Set(
                          selectedFlow.routes
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
                          breadcrumb.some((item) => item.flowId === selectedIvrFlow.id)

                        return (
                          <div
                            key={`${selectedFlow.id}-${route.dtmfKey}-${routeIndex}`}
                            ref={(element) => {
                              routeRowRefs.current[routeIndex] = element
                            }}
                            className={cn(
                              "rounded-md border p-3 space-y-3 transition-colors",
                              (highlightedRouteIndex === routeIndex ||
                                (selectedSection === "routes" && focusRouteIndex === routeIndex)) &&
                                "bg-primary/5 ring-2 ring-primary/30",
                            )}
                          >
                            <div className="grid gap-3 md:grid-cols-[120px_1fr_180px_auto] md:items-end">
                              <div className="space-y-2">
                                <Label>キー</Label>
                                <Select
                                  value={route.dtmfKey}
                                  onValueChange={(value) =>
                                    updateRoute(selectedFlow.id, routeIndex, (current) => ({
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
                                    updateRoute(selectedFlow.id, routeIndex, (current) => ({
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
                                      selectedFlow.id,
                                      routeIndex,
                                      value as IvrTerminalAction["actionCode"],
                                      selectedFlowDepth,
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
                                onClick={() => removeRoute(selectedFlow.id, routeIndex)}
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
                                    updateRoute(selectedFlow.id, routeIndex, (current) =>
                                      current.destination.actionCode === "VM" ||
                                      current.destination.actionCode === "AN"
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
                                      updateRoute(selectedFlow.id, routeIndex, (current) =>
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
                                      updateRoute(selectedFlow.id, routeIndex, (current) =>
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
                                      updateRoute(selectedFlow.id, routeIndex, (current) =>
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
                              <div className="space-y-2 rounded-md border p-3">
                                <Label>次層IVRフロー</Label>
                                <div className="rounded-md border px-3 py-2 text-sm">
                                  {selectedIvrFlow
                                    ? `${selectedIvrFlow.name || selectedIvrFlow.id} [${nestDepthLabel}層]`
                                    : "未作成（保存前または削除済み）"}
                                </div>
                                <p className="text-xs text-muted-foreground">
                                  深さ: {selectedFlowDepth}層 → {nestDepthLabel}層
                                </p>
                                <Button
                                  variant="outline"
                                  size="sm"
                                  onClick={() =>
                                    createSubFlowForRoute(selectedFlow.id, routeIndex, selectedFlowDepth)
                                  }
                                  disabled={busy || !canNestFurther}
                                >
                                  次層IVRを新規作成
                                </Button>
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
                                    循環参照になるため、展開はツリー側で停止しています: {selectedIvrFlow.name || selectedIvrFlow.id}
                                  </p>
                                )}
                              </div>
                            )}

                            <p className="text-xs text-muted-foreground">
                              {terminalActionLabel(route.destination, flows, scenarioNameById)}
                            </p>
                          </div>
                        )
                      })}
                    </div>
                  )}

                  {selectedRouteDraft && (
                    <div className="rounded-md border border-dashed p-3 space-y-3">
                      <h4 className="text-sm font-medium">ルート追加（{selectedFlowDepth}層）</h4>

                      <div className="grid gap-3 md:grid-cols-[120px_1fr_180px]">
                        <div className="space-y-2">
                          <Label>キー</Label>
                          <Select
                            value={selectedRouteDraft.dtmfKey}
                            onValueChange={(value) =>
                              updateRouteDraft(selectedFlow.id, (current) => ({
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
                                const used = selectedFlow.routes.some((route) => route.dtmfKey === key)
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
                            value={selectedRouteDraft.label}
                            onChange={(event) =>
                              updateRouteDraft(selectedFlow.id, (current) => ({
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
                            value={selectedRouteDraft.destinationType}
                            onValueChange={(value) =>
                              updateRouteDraft(selectedFlow.id, (current) => {
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

                      {(selectedRouteDraft.destinationType === "VM" ||
                        selectedRouteDraft.destinationType === "AN") && (
                        <div className="space-y-2">
                          <Label>アナウンス</Label>
                          {renderAnnouncementSelect(
                            selectedRouteDraft.announcementId,
                            (value) =>
                              updateRouteDraft(selectedFlow.id, (current) => ({
                                ...current,
                                announcementId: normalizeAnnouncementValue(value),
                              })),
                            busy,
                          )}
                        </div>
                      )}

                      {selectedRouteDraft.destinationType === "VB" && (
                        <div className="space-y-3 rounded-md border p-3">
                          <div className="space-y-2">
                            <Label>シナリオ</Label>
                            {renderScenarioSelect(
                              selectedRouteDraft.scenarioId,
                              (value) =>
                                updateRouteDraft(selectedFlow.id, (current) => ({
                                  ...current,
                                  scenarioId: value === NONE_SCENARIO_VALUE ? "" : value,
                                })),
                              busy,
                            )}
                          </div>

                          <div className="space-y-2">
                            <Label>開始前アナウンス</Label>
                            {renderAnnouncementSelect(
                              selectedRouteDraft.welcomeAnnouncementId,
                              (value) =>
                                updateRouteDraft(selectedFlow.id, (current) => ({
                                  ...current,
                                  welcomeAnnouncementId: normalizeAnnouncementValue(value),
                                })),
                              busy,
                            )}
                          </div>

                          <div className="h-10 rounded-md border px-3 flex items-center justify-between">
                            <span className="text-sm">録音あり（PoC固定）</span>
                            <Switch checked={selectedRouteDraft.recordingEnabled} disabled />
                          </div>

                          <div className="h-10 rounded-md border px-3 flex items-center justify-between">
                            <span className="text-sm">includeAnnouncement</span>
                            <Switch
                              checked={selectedRouteDraft.includeAnnouncement}
                              onCheckedChange={(checked) =>
                                updateRouteDraft(selectedFlow.id, (current) => ({
                                  ...current,
                                  includeAnnouncement: checked,
                                }))
                              }
                              disabled={busy}
                            />
                          </div>
                        </div>
                      )}

                      {selectedRouteDraft.destinationType === "IV" && (
                        <div className="space-y-2">
                          <Label>次層IVRフロー</Label>
                          <div className="rounded-md border px-3 py-2 text-sm">
                            未作成（このルートを追加した時点で自動作成）
                          </div>
                          <p className="text-xs text-muted-foreground">
                            深さ: {selectedFlowDepth}層 → {nestDepthLabel}層
                          </p>
                          <p className="text-xs text-muted-foreground">
                            参照選択は不要です。`追加` 実行時に次層IVRを作成します。
                          </p>
                          {!canNestFurther && (
                            <p className="text-xs text-amber-600">
                              この層では新しいサブIVRを追加できません（上限 {MAX_IVR_DEPTH} 層）
                            </p>
                          )}
                        </div>
                      )}

                      <div className="flex items-center justify-end gap-2">
                        <Button variant="outline" onClick={() => clearRouteDraft(selectedFlow.id)} disabled={busy}>
                          <X className="h-4 w-4 mr-1" />
                          キャンセル
                        </Button>
                        <Button onClick={() => addRoute(selectedFlow.id, selectedFlowDepth)} disabled={busy}>
                          <Plus className="h-4 w-4 mr-1" />
                          追加
                        </Button>
                      </div>
                    </div>
                  )}
                </div>

                <div ref={invalidSectionRef} className={sectionClass("invalid")}>
                  <h3 className="font-medium">④ 無効入力時</h3>
                  <div className="space-y-2">
                    <Label>アナウンス</Label>
                    {renderAnnouncementSelect(
                      selectedFlow.invalidInputAnnouncementId,
                      (value) =>
                        updateSelectedFlow((flow) => ({
                          ...flow,
                          invalidInputAnnouncementId: normalizeAnnouncementValue(value),
                        })),
                      busy,
                    )}
                  </div>
                </div>

                <div ref={timeoutSectionRef} className={sectionClass("timeout")}>
                  <h3 className="font-medium">⑤ タイムアウト時</h3>
                  <div className="space-y-2">
                    <Label>アナウンス</Label>
                    {renderAnnouncementSelect(
                      selectedFlow.timeoutAnnouncementId,
                      (value) =>
                        updateSelectedFlow((flow) => ({
                          ...flow,
                          timeoutAnnouncementId: normalizeAnnouncementValue(value),
                        })),
                      busy,
                    )}
                  </div>
                </div>

                <div ref={fallbackSectionRef} className={sectionClass("fallback")}>
                  <h3 className="font-medium">⑥ リトライ超過時（Fallback）</h3>

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

                <div className="flex items-center justify-between gap-2">
                  <p className="text-xs text-muted-foreground">
                    ネスト上限: {MAX_IVR_DEPTH}層（保存時に検証）
                  </p>

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
                </div>
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
