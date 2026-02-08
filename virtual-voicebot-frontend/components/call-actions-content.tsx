"use client"

import Link from "next/link"
import { useEffect, useMemo, useState } from "react"
import {
  AlertTriangle,
  ArrowDown,
  ArrowUp,
  CheckCircle2,
  Plus,
  Save,
  ShieldQuestion,
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
  actionCodeLabel,
  actionTypeLabel,
  cloneActionConfig,
  createActionConfig,
  createDefaultCallActionsDatabase,
  isAllowActionCode,
  isDenyActionCode,
  type ActionConfig,
  type CallActionCode,
  type CallActionType,
  type CallerGroup,
  type IncomingRule,
  type StoredAction,
} from "@/lib/call-actions"
import type { IvrFlowDefinition } from "@/lib/ivr-flows"
import type { VoicebotScenario } from "@/lib/scenarios"
import { cn } from "@/lib/utils"

interface CallActionsApiResponse {
  ok: boolean
  rules?: IncomingRule[]
  anonymousAction?: StoredAction
  defaultAction?: StoredAction
  error?: string
}

interface NumberGroupsApiResponse {
  ok: boolean
  callerGroups?: CallerGroup[]
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

interface IvrFlowsApiResponse {
  ok: boolean
  flows?: IvrFlowDefinition[]
  error?: string
}

interface ScenariosApiResponse {
  ok: boolean
  scenarios?: VoicebotScenario[]
  error?: string
}

const NONE_ANNOUNCEMENT_VALUE = "__none__"
const NONE_IVR_VALUE = "__none_ivr__"
const NONE_SCENARIO_VALUE = "__none_scenario__"
const ALLOW_ACTION_CODES: CallActionCode[] = ["VR", "IV", "VM", "VB"]
const DENY_ACTION_CODES: CallActionCode[] = ["BZ", "NR", "AN"]

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

function cloneRule(rule: IncomingRule): IncomingRule {
  return {
    ...rule,
    actionConfig: cloneActionConfig(rule.actionConfig),
  }
}

function cloneStoredAction(action: StoredAction): StoredAction {
  return {
    actionType: action.actionType,
    actionConfig: cloneActionConfig(action.actionConfig),
  }
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

function getAnnouncementId(config: ActionConfig): string | null {
  switch (config.actionCode) {
    case "VR":
      return config.announcementId
    case "VM":
      return config.announcementId
    case "VB":
      return config.welcomeAnnouncementId
    case "AN":
      return config.announcementId
    default:
      return null
  }
}

function withAnnouncementId(config: ActionConfig, announcementId: string | null): ActionConfig {
  switch (config.actionCode) {
    case "VR":
      return { ...config, announcementId }
    case "VM":
      return { ...config, announcementId }
    case "VB":
      return { ...config, welcomeAnnouncementId: announcementId }
    case "AN":
      return { ...config, announcementId }
    default:
      return config
  }
}

function buildActionSummary(actionType: CallActionType, actionConfig: ActionConfig): string {
  const head = `${actionTypeLabel(actionType)} > ${actionCodeLabel(actionConfig.actionCode)}`

  if (actionConfig.actionCode === "VR") {
    const flags = [
      actionConfig.recordingEnabled ? "録音あり" : "録音なし",
      actionConfig.announceEnabled ? "事前アナウンスあり" : "事前アナウンスなし",
    ]
    return `${head} (${flags.join(" / ")})`
  }

  if (actionConfig.actionCode === "IV") {
    return `${head}${actionConfig.ivrFlowId ? ` (flow: ${actionConfig.ivrFlowId})` : ""}`
  }

  if (actionConfig.actionCode === "VB") {
    return `${head}${actionConfig.scenarioId ? ` (scenario: ${actionConfig.scenarioId})` : ""}`
  }

  return head
}

function applyActionType(
  currentConfig: ActionConfig,
  nextType: CallActionType,
): ActionConfig {
  return withAnnouncementId(
    createActionConfig(nextType),
    getAnnouncementId(currentConfig),
  )
}

function applyActionCode(
  currentType: CallActionType,
  currentConfig: ActionConfig,
  nextCode: string,
): ActionConfig {
  if (currentType === "allow") {
    if (!isAllowActionCode(nextCode)) {
      return currentConfig
    }

    return withAnnouncementId(
      createActionConfig("allow", nextCode),
      getAnnouncementId(currentConfig),
    )
  }

  if (!isDenyActionCode(nextCode)) {
    return currentConfig
  }

  return withAnnouncementId(
    createActionConfig("deny", nextCode),
    getAnnouncementId(currentConfig),
  )
}

function collectReferencedIvrFlowIds(flows: IvrFlowDefinition[]): Set<string> {
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

function collectRootIvrFlows(flows: IvrFlowDefinition[]): IvrFlowDefinition[] {
  const referenced = collectReferencedIvrFlowIds(flows)
  return flows.filter((flow) => !referenced.has(flow.id))
}

export function CallActionsContent() {
  const defaults = useMemo(() => createDefaultCallActionsDatabase(), [])

  const [callerGroups, setCallerGroups] = useState<CallerGroup[]>([])
  const [rules, setRules] = useState<IncomingRule[]>([])
  const [anonymousAction, setAnonymousAction] = useState<StoredAction>(() =>
    cloneStoredAction(defaults.anonymousAction),
  )
  const [defaultAction, setDefaultAction] = useState<StoredAction>(() =>
    cloneStoredAction(defaults.defaultAction),
  )
  const [announcements, setAnnouncements] = useState<StoredAnnouncement[]>([])
  const [ivrFlows, setIvrFlows] = useState<IvrFlowDefinition[]>([])
  const [scenarios, setScenarios] = useState<VoicebotScenario[]>([])

  const [selectedRuleId, setSelectedRuleId] = useState<string | null>(null)
  const [editorMode, setEditorMode] = useState<"rule" | "anonymous" | "default">("default")

  const [ruleDraft, setRuleDraft] = useState<IncomingRule | null>(null)
  const [anonymousDraft, setAnonymousDraft] = useState<StoredAction>(() =>
    cloneStoredAction(defaults.anonymousAction),
  )
  const [defaultDraft, setDefaultDraft] = useState<StoredAction>(() =>
    cloneStoredAction(defaults.defaultAction),
  )

  const [loading, setLoading] = useState(true)
  const [busy, setBusy] = useState(false)
  const [errorMessage, setErrorMessage] = useState<string | null>(null)
  const [infoMessage, setInfoMessage] = useState<string | null>(null)

  const selectedRule = useMemo(
    () => rules.find((rule) => rule.id === selectedRuleId) ?? null,
    [rules, selectedRuleId],
  )

  const groupNameById = useMemo(
    () => new Map(callerGroups.map((group) => [group.id, group.name])),
    [callerGroups],
  )

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

  const ivrFlowById = useMemo(
    () => new Map(ivrFlows.map((flow) => [flow.id, flow])),
    [ivrFlows],
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

  const rootIvrFlows = useMemo(() => collectRootIvrFlows(ivrFlows), [ivrFlows])

  const rootIvrFlowById = useMemo(
    () => new Map(rootIvrFlows.map((flow) => [flow.id, flow])),
    [rootIvrFlows],
  )

  useEffect(() => {
    let cancelled = false

    const load = async () => {
      setLoading(true)
      setErrorMessage(null)
      setInfoMessage(null)

      try {
        const [groupsRes, callActionsRes, announcementsRes, ivrFlowsRes, scenariosRes] = await Promise.all([
          fetch("/api/number-groups", { cache: "no-store" }),
          fetch("/api/call-actions", { cache: "no-store" }),
          fetch("/api/announcements", { cache: "no-store" }).catch(() => null),
          fetch("/api/ivr-flows", { cache: "no-store" }).catch(() => null),
          fetch("/api/scenarios", { cache: "no-store" }).catch(() => null),
        ])

        const groupsBody = (await groupsRes.json()) as NumberGroupsApiResponse
        if (!groupsRes.ok || !groupsBody.ok) {
          throw new Error(groupsBody.error ?? "failed to load number groups")
        }

        const callActionsBody = (await callActionsRes.json()) as CallActionsApiResponse
        if (!callActionsRes.ok || !callActionsBody.ok) {
          throw new Error(callActionsBody.error ?? "failed to load call actions")
        }

        const loadedCallerGroups = Array.isArray(groupsBody.callerGroups)
          ? groupsBody.callerGroups
          : []
        const loadedRules = Array.isArray(callActionsBody.rules) ? callActionsBody.rules : []
        const loadedAnonymousAction =
          callActionsBody.anonymousAction ?? defaults.anonymousAction
        const loadedDefaultAction = callActionsBody.defaultAction ?? defaults.defaultAction

        let loadedAnnouncements: StoredAnnouncement[] = []
        if (announcementsRes && announcementsRes.ok) {
          const announcementsBody = (await announcementsRes.json()) as AnnouncementsApiResponse
          if (announcementsBody.ok && Array.isArray(announcementsBody.announcements)) {
            loadedAnnouncements = announcementsBody.announcements
          }
        }

        let loadedIvrFlows: IvrFlowDefinition[] = []
        if (ivrFlowsRes && ivrFlowsRes.ok) {
          const ivrFlowsBody = (await ivrFlowsRes.json()) as IvrFlowsApiResponse
          if (ivrFlowsBody.ok && Array.isArray(ivrFlowsBody.flows)) {
            loadedIvrFlows = ivrFlowsBody.flows
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

        setCallerGroups(loadedCallerGroups)
        setRules(loadedRules)
        setAnonymousAction(cloneStoredAction(loadedAnonymousAction))
        setDefaultAction(cloneStoredAction(loadedDefaultAction))
        setAnnouncements(loadedAnnouncements)
        setIvrFlows(loadedIvrFlows)
        setScenarios(loadedScenarios)

        setSelectedRuleId(loadedRules[0]?.id ?? null)
        setEditorMode(loadedRules.length > 0 ? "rule" : "anonymous")
      } catch (error) {
        if (cancelled) {
          return
        }
        setErrorMessage(
          error instanceof Error ? error.message : "着信アクションの読み込みに失敗しました",
        )
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
  }, [defaults.anonymousAction, defaults.defaultAction])

  useEffect(() => {
    if (!selectedRule) {
      setRuleDraft(null)
      return
    }

    setRuleDraft(cloneRule(selectedRule))
  }, [selectedRule])

  useEffect(() => {
    setAnonymousDraft(cloneStoredAction(anonymousAction))
  }, [anonymousAction])

  useEffect(() => {
    setDefaultDraft(cloneStoredAction(defaultAction))
  }, [defaultAction])

  const saveDatabase = async (
    next: {
      rules: IncomingRule[]
      anonymousAction: StoredAction
      defaultAction: StoredAction
    },
    options?: {
      message?: string
      nextSelectedRuleId?: string | null
      nextEditorMode?: "rule" | "anonymous" | "default"
    },
  ): Promise<boolean> => {
    const missingScenarioSelections: string[] = []
    const deletedScenarioReferences: string[] = []
    const collectScenarioIssues = (label: string, actionConfig: ActionConfig) => {
      if (actionConfig.actionCode !== "VB") {
        return
      }
      const scenarioId = actionConfig.scenarioId.trim()
      if (scenarioId.length === 0) {
        missingScenarioSelections.push(label)
        return
      }
      if (!scenarioById.has(scenarioId)) {
        deletedScenarioReferences.push(`${label} (${scenarioId})`)
      }
    }

    for (const rule of next.rules) {
      collectScenarioIssues(`ルール「${rule.name || rule.id}」`, rule.actionConfig)
    }
    collectScenarioIssues("非通知時アクション", next.anonymousAction.actionConfig)
    collectScenarioIssues("デフォルトアクション", next.defaultAction.actionConfig)

    if (missingScenarioSelections.length > 0) {
      setErrorMessage(
        `シナリオを選択してください:\n${missingScenarioSelections
          .map((item) => `- ${item}`)
          .join("\n")}`,
      )
      return false
    }

    const invalidIvrReferences: string[] = []
    for (const rule of next.rules) {
      if (rule.actionConfig.actionCode === "IV" && rule.actionConfig.ivrFlowId) {
        if (!rootIvrFlowById.has(rule.actionConfig.ivrFlowId)) {
          invalidIvrReferences.push(`ルール「${rule.name || rule.id}」`)
        }
      }
    }
    if (
      next.anonymousAction.actionConfig.actionCode === "IV" &&
      next.anonymousAction.actionConfig.ivrFlowId &&
      !rootIvrFlowById.has(next.anonymousAction.actionConfig.ivrFlowId)
    ) {
      invalidIvrReferences.push("非通知時アクション")
    }
    if (
      next.defaultAction.actionConfig.actionCode === "IV" &&
      next.defaultAction.actionConfig.ivrFlowId &&
      !rootIvrFlowById.has(next.defaultAction.actionConfig.ivrFlowId)
    ) {
      invalidIvrReferences.push("デフォルトアクション")
    }
    if (invalidIvrReferences.length > 0) {
      setErrorMessage(
        `2層目以降のIVRフローは着信アクションに設定できません:\n${invalidIvrReferences
          .map((item) => `- ${item}`)
          .join("\n")}`,
      )
      return false
    }

    setBusy(true)
    setErrorMessage(null)
    setInfoMessage(null)

    try {
      const response = await fetch("/api/call-actions", {
        method: "PUT",
        headers: {
          "content-type": "application/json",
        },
        body: JSON.stringify(next),
      })
      const body = (await response.json()) as { ok: boolean; error?: string }

      if (!response.ok || !body.ok) {
        throw new Error(body.error ?? "failed to save call actions")
      }

      setRules(next.rules)
      setAnonymousAction(cloneStoredAction(next.anonymousAction))
      setDefaultAction(cloneStoredAction(next.defaultAction))

      const nextSelectedRule =
        options?.nextSelectedRuleId !== undefined
          ? options.nextSelectedRuleId
          : next.rules.find((rule) => rule.id === selectedRuleId)?.id ?? next.rules[0]?.id ?? null

      setSelectedRuleId(nextSelectedRule)
      setEditorMode(options?.nextEditorMode ?? editorMode)

      const infoMessages: string[] = []
      if (options?.message) {
        infoMessages.push(options.message)
      }
      if (deletedScenarioReferences.length > 0) {
        infoMessages.push(
          `（削除済みシナリオ）参照を保持しています:\n${deletedScenarioReferences
            .map((item) => `- ${item}`)
            .join("\n")}`,
        )
      }
      if (infoMessages.length > 0) {
        setInfoMessage(infoMessages.join("\n"))
      }
      return true
    } catch (error) {
      setErrorMessage(error instanceof Error ? error.message : "保存に失敗しました")
      return false
    } finally {
      setBusy(false)
    }
  }

  const addRule = async () => {
    if (callerGroups.length === 0) {
      setErrorMessage("先に番号グループタブでグループを作成してください")
      return
    }

    const timestamp = nowIso()
    const newRule: IncomingRule = {
      id: createId(),
      name: `ルール ${rules.length + 1}`,
      callerGroupId: callerGroups[0].id,
      actionType: "allow",
      actionConfig: createActionConfig("allow", "VR"),
      isActive: true,
      createdAt: timestamp,
      updatedAt: timestamp,
    }

    await saveDatabase(
      {
        rules: [...rules, newRule],
        anonymousAction,
        defaultAction,
      },
      {
        message: "ルールを追加しました",
        nextSelectedRuleId: newRule.id,
        nextEditorMode: "rule",
      },
    )
  }

  const deleteRule = async (ruleId: string) => {
    const nextRules = rules.filter((rule) => rule.id !== ruleId)
    const nextSelectedRuleId =
      selectedRuleId === ruleId ? (nextRules[0]?.id ?? null) : (selectedRuleId ?? null)

    await saveDatabase(
      {
        rules: nextRules,
        anonymousAction,
        defaultAction,
      },
      {
        message: "ルールを削除しました",
        nextSelectedRuleId,
        nextEditorMode: nextSelectedRuleId ? "rule" : "anonymous",
      },
    )
  }

  const moveRule = async (ruleId: string, direction: "up" | "down") => {
    const currentIndex = rules.findIndex((rule) => rule.id === ruleId)
    if (currentIndex < 0) {
      return
    }

    const targetIndex = direction === "up" ? currentIndex - 1 : currentIndex + 1
    if (targetIndex < 0 || targetIndex >= rules.length) {
      return
    }

    const nextRules = [...rules]
    const [moved] = nextRules.splice(currentIndex, 1)
    nextRules.splice(targetIndex, 0, moved)

    await saveDatabase(
      {
        rules: nextRules,
        anonymousAction,
        defaultAction,
      },
      {
        message: "ルールの優先順位を更新しました",
      },
    )
  }

  const updateRuleActive = async (ruleId: string, isActive: boolean) => {
    const timestamp = nowIso()
    const nextRules = rules.map((rule) =>
      rule.id === ruleId
        ? {
            ...rule,
            isActive,
            updatedAt: timestamp,
          }
        : rule,
    )

    await saveDatabase({
      rules: nextRules,
      anonymousAction,
      defaultAction,
    })
  }

  const updateRuleDraft = (updater: (rule: IncomingRule) => IncomingRule) => {
    setRuleDraft((current) => (current ? updater(current) : current))
  }

  const saveRuleDraft = async () => {
    if (!ruleDraft) {
      return
    }

    const name = ruleDraft.name.trim()
    if (!name) {
      setErrorMessage("ルール名を入力してください")
      return
    }

    if (!callerGroups.some((group) => group.id === ruleDraft.callerGroupId)) {
      setErrorMessage("番号グループを選択してください")
      return
    }

    const timestamp = nowIso()
    const nextRules = rules.map((rule) =>
      rule.id === ruleDraft.id
        ? {
            ...ruleDraft,
            name,
            updatedAt: timestamp,
          }
        : rule,
    )

    await saveDatabase(
      {
        rules: nextRules,
        anonymousAction,
        defaultAction,
      },
      {
        message: "ルールを保存しました",
        nextSelectedRuleId: ruleDraft.id,
        nextEditorMode: "rule",
      },
    )
  }

  const saveAnonymousDraft = async () => {
    await saveDatabase(
      {
        rules,
        anonymousAction: anonymousDraft,
        defaultAction,
      },
      {
        message: "非通知アクションを保存しました",
        nextEditorMode: "anonymous",
      },
    )
  }

  const saveDefaultDraft = async () => {
    await saveDatabase(
      {
        rules,
        anonymousAction,
        defaultAction: defaultDraft,
      },
      {
        message: "デフォルトアクションを保存しました",
        nextEditorMode: "default",
      },
    )
  }

  const onAnnouncementChange = (
    selectedValue: string,
    onConfigChange: (updater: (config: ActionConfig) => ActionConfig) => void,
  ) => {
    const nextAnnouncementId =
      selectedValue === NONE_ANNOUNCEMENT_VALUE ? null : selectedValue
    onConfigChange((config) => withAnnouncementId(config, nextAnnouncementId))
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
        value={selectedId ?? NONE_ANNOUNCEMENT_VALUE}
        onValueChange={onValueChange}
        disabled={selectDisabled}
      >
        <SelectTrigger>
          <SelectValue placeholder="アナウンスを選択" />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value={NONE_ANNOUNCEMENT_VALUE}>なし</SelectItem>
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

  const renderIvrFlowSelect = (
    selectedFlowId: string | null,
    onValueChange: (value: string) => void,
    disabled: boolean,
  ) => {
    const hasSelectedFlow = selectedFlowId ? ivrFlowById.has(selectedFlowId) : true
    const selectedIsRoot = selectedFlowId ? rootIvrFlowById.has(selectedFlowId) : true
    const selectDisabled = disabled || (rootIvrFlows.length === 0 && !selectedFlowId)

    return (
      <Select
        value={selectedFlowId ?? NONE_IVR_VALUE}
        onValueChange={onValueChange}
        disabled={selectDisabled}
      >
        <SelectTrigger>
          <SelectValue placeholder="IVRフローを選択" />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value={NONE_IVR_VALUE}>なし</SelectItem>
          {!hasSelectedFlow && selectedFlowId && (
            <SelectItem value={selectedFlowId}>（削除済み IVR）</SelectItem>
          )}
          {hasSelectedFlow && !selectedIsRoot && selectedFlowId && (
            <SelectItem value={selectedFlowId}>（下層IVR: 選択対象外）</SelectItem>
          )}
          {rootIvrFlows.length === 0 ? (
            <SelectItem value="__empty_ivr__" disabled>
              （トップIVRフロー未登録）
            </SelectItem>
          ) : (
            rootIvrFlows.map((flow) => (
              <SelectItem key={flow.id} value={flow.id}>
                {flow.name || flow.id}
                {!flow.isActive ? " [無効]" : ""}
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

  const renderActionEditor = (
    actionType: CallActionType,
    actionConfig: ActionConfig,
    callbacks: {
      onActionTypeChange: (nextType: CallActionType) => void
      onActionCodeChange: (nextCode: string) => void
      onActionConfigChange: (updater: (config: ActionConfig) => ActionConfig) => void
    },
  ) => {
    return (
      <div className="space-y-4">
        <div className="grid gap-4 md:grid-cols-2">
          <div className="space-y-2">
            <Label>アクション種別</Label>
            <Select
              value={actionType}
              onValueChange={(value) =>
                callbacks.onActionTypeChange(value === "deny" ? "deny" : "allow")
              }
              disabled={busy}
            >
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="allow">Allow</SelectItem>
                <SelectItem value="deny">Deny</SelectItem>
              </SelectContent>
            </Select>
          </div>

          <div className="space-y-2">
            <Label>アクション</Label>
            <Select
              value={actionConfig.actionCode}
              onValueChange={callbacks.onActionCodeChange}
              disabled={busy}
            >
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {(actionType === "allow" ? ALLOW_ACTION_CODES : DENY_ACTION_CODES).map((code) => (
                  <SelectItem key={code} value={code}>
                    {actionCodeLabel(code)} ({code})
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        </div>

        {actionConfig.actionCode === "VR" && (
          <div className="space-y-3 rounded-md border p-3">
            <p className="text-sm font-medium">通常着信設定</p>
            <div className="h-10 rounded-md border px-3 flex items-center justify-between">
              <span className="text-sm">録音あり</span>
              <Switch
                checked={actionConfig.recordingEnabled}
                onCheckedChange={(checked) =>
                  callbacks.onActionConfigChange((config) =>
                    config.actionCode === "VR"
                      ? {
                          ...config,
                          recordingEnabled: checked,
                        }
                      : config,
                  )
                }
                disabled={busy}
              />
            </div>
            <div className="h-10 rounded-md border px-3 flex items-center justify-between">
              <span className="text-sm">事前アナウンスあり</span>
              <Switch
                checked={actionConfig.announceEnabled}
                onCheckedChange={(checked) =>
                  callbacks.onActionConfigChange((config) =>
                    config.actionCode === "VR"
                      ? {
                          ...config,
                          announceEnabled: checked,
                        }
                      : config,
                  )
                }
                disabled={busy}
              />
            </div>
            {actionConfig.announceEnabled && (
              <div className="space-y-2">
                <Label>アナウンス</Label>
                {renderAnnouncementSelect(
                  actionConfig.announcementId,
                  (value) => onAnnouncementChange(value, callbacks.onActionConfigChange),
                  busy,
                )}
              </div>
            )}
          </div>
        )}

        {actionConfig.actionCode === "IV" && (
          <div className="space-y-3 rounded-md border p-3">
            <p className="text-sm font-medium">IVR設定</p>
            <div className="space-y-2">
              <Label>IVRフロー</Label>
              {renderIvrFlowSelect(
                actionConfig.ivrFlowId,
                (value) =>
                  callbacks.onActionConfigChange((config) =>
                    config.actionCode === "IV"
                      ? {
                          ...config,
                          ivrFlowId: value === NONE_IVR_VALUE ? null : value,
                        }
                      : config,
                ),
                busy,
              )}
              {actionConfig.ivrFlowId && !ivrFlowById.has(actionConfig.ivrFlowId) && (
                <p className="text-xs text-amber-600">（削除済み IVR）参照を保持しています</p>
              )}
              {actionConfig.ivrFlowId &&
                ivrFlowById.has(actionConfig.ivrFlowId) &&
                !rootIvrFlowById.has(actionConfig.ivrFlowId) && (
                  <p className="text-xs text-amber-600">
                    このIVRは下層フローです。着信アクションでは1層目フローのみ設定できます。
                  </p>
                )}
            </div>
            <div className="h-10 rounded-md border px-3 flex items-center justify-between">
              <span className="text-sm">includeAnnouncement</span>
              <Switch
                checked={actionConfig.includeAnnouncement}
                onCheckedChange={(checked) =>
                  callbacks.onActionConfigChange((config) =>
                    config.actionCode === "IV"
                      ? {
                          ...config,
                          includeAnnouncement: checked,
                        }
                      : config,
                  )
                }
                disabled={busy}
              />
            </div>
          </div>
        )}

        {actionConfig.actionCode === "VB" && (
          <div className="space-y-3 rounded-md border p-3">
            <p className="text-sm font-medium">ボイスボット設定</p>
            <div className="space-y-2">
              <Label>シナリオ</Label>
              {renderScenarioSelect(
                actionConfig.scenarioId,
                (value) =>
                  callbacks.onActionConfigChange((config) =>
                    config.actionCode === "VB"
                      ? {
                          ...config,
                          scenarioId: value === NONE_SCENARIO_VALUE ? "" : value,
                        }
                      : config,
                  ),
                busy,
              )}
              {actionConfig.scenarioId && !scenarioById.has(actionConfig.scenarioId) && (
                <p className="text-xs text-amber-600">（削除済みシナリオ）参照を保持しています</p>
              )}
            </div>
            <div className="space-y-2">
              <Label>開始前アナウンス</Label>
              {renderAnnouncementSelect(
                actionConfig.welcomeAnnouncementId,
                (value) => onAnnouncementChange(value, callbacks.onActionConfigChange),
                busy,
              )}
            </div>
            <div className="h-10 rounded-md border px-3 flex items-center justify-between">
              <span className="text-sm">録音あり（PoC固定）</span>
              <Switch checked={actionConfig.recordingEnabled} disabled />
            </div>
            <div className="h-10 rounded-md border px-3 flex items-center justify-between">
              <span className="text-sm">includeAnnouncement</span>
              <Switch
                checked={actionConfig.includeAnnouncement}
                onCheckedChange={(checked) =>
                  callbacks.onActionConfigChange((config) =>
                    config.actionCode === "VB"
                      ? {
                          ...config,
                          includeAnnouncement: checked,
                        }
                      : config,
                  )
                }
                disabled={busy}
              />
            </div>
          </div>
        )}

        {actionConfig.actionCode === "VM" && (
          <div className="space-y-2 rounded-md border p-3">
            <Label>留守電アナウンス</Label>
            {renderAnnouncementSelect(
              actionConfig.announcementId,
              (value) => onAnnouncementChange(value, callbacks.onActionConfigChange),
              busy,
            )}
          </div>
        )}

        {actionConfig.actionCode === "AN" && (
          <div className="space-y-2 rounded-md border p-3">
            <Label>再生アナウンス</Label>
            {renderAnnouncementSelect(
              actionConfig.announcementId,
              (value) => onAnnouncementChange(value, callbacks.onActionConfigChange),
              busy,
            )}
          </div>
        )}

        {(actionConfig.actionCode === "BZ" || actionConfig.actionCode === "NR") && (
          <div className="rounded-md border p-3 text-sm text-muted-foreground">
            追加設定はありません。
          </div>
        )}
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
      <div>
        <h1 className="text-2xl font-bold text-balance">着信アクション</h1>
        <p className="text-muted-foreground">Call Actions</p>
      </div>

      {errorMessage && (
        <div className="rounded-md border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive flex items-start gap-2">
          <AlertTriangle className="h-4 w-4 mt-0.5" />
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
          <CardHeader>
            <CardTitle className="text-lg">番号グループ（参照のみ）</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <ScrollArea className="h-[420px] rounded-md border">
              <div className="p-2 space-y-1">
                {callerGroups.length === 0 ? (
                  <div className="p-3 text-sm text-muted-foreground space-y-2">
                    <p>番号グループがありません。</p>
                    <p>先に番号グループタブでグループを作成してください。</p>
                  </div>
                ) : (
                  callerGroups.map((group) => (
                    <div key={group.id} className="rounded-md border px-3 py-2">
                      <div className="font-medium truncate">{group.name}</div>
                      <div className="text-xs text-muted-foreground flex items-center justify-between">
                        <span>{group.description ?? "説明なし"}</span>
                        <Badge variant="secondary">{group.phoneNumbers.length}</Badge>
                      </div>
                    </div>
                  ))
                )}
              </div>
            </ScrollArea>

            <Button asChild variant="outline" className="w-full bg-transparent">
              <Link href="/groups">番号グループタブで編集</Link>
            </Button>
          </CardContent>
        </Card>

        <div className="space-y-6">
          <Card>
            <CardHeader className="pb-3">
              <div className="flex items-center justify-between gap-3">
                <CardTitle className="text-lg">ルール一覧（上から優先）</CardTitle>
                <Button onClick={addRule} disabled={busy || callerGroups.length === 0}>
                  <Plus className="h-4 w-4 mr-1" />
                  ルール追加
                </Button>
              </div>
            </CardHeader>
            <CardContent className="space-y-3">
              {rules.length === 0 ? (
                <p className="text-sm text-muted-foreground">ルールがありません</p>
              ) : (
                <div className="space-y-2">
                  {rules.map((rule, index) => {
                    const groupName = groupNameById.get(rule.callerGroupId) ?? "（削除済み）"
                    return (
                      <div
                        key={rule.id}
                        className={cn(
                          "rounded-md border p-3",
                          selectedRuleId === rule.id && editorMode === "rule"
                            ? "border-primary bg-primary/5"
                            : "",
                        )}
                      >
                        <div className="flex flex-col gap-2 lg:flex-row lg:items-center lg:justify-between">
                          <button
                            type="button"
                            onClick={() => {
                              setSelectedRuleId(rule.id)
                              setEditorMode("rule")
                              setErrorMessage(null)
                              setInfoMessage(null)
                            }}
                            className="text-left flex-1"
                          >
                            <div className="font-medium text-sm">
                              #{index + 1} {rule.name}
                            </div>
                            <div className="text-xs text-muted-foreground mt-1">
                              {groupName} / {buildActionSummary(rule.actionType, rule.actionConfig)}
                            </div>
                          </button>

                          <div className="flex items-center gap-2">
                            <div className="flex items-center gap-1 text-xs text-muted-foreground">
                              有効
                              <Switch
                                checked={rule.isActive}
                                onCheckedChange={(checked) => void updateRuleActive(rule.id, checked)}
                                disabled={busy}
                              />
                            </div>
                            <Button
                              variant="outline"
                              size="icon"
                              className="h-8 w-8"
                              onClick={() => void moveRule(rule.id, "up")}
                              disabled={busy || index === 0}
                            >
                              <ArrowUp className="h-4 w-4" />
                            </Button>
                            <Button
                              variant="outline"
                              size="icon"
                              className="h-8 w-8"
                              onClick={() => void moveRule(rule.id, "down")}
                              disabled={busy || index === rules.length - 1}
                            >
                              <ArrowDown className="h-4 w-4" />
                            </Button>
                            <Button
                              variant="destructive"
                              size="icon"
                              className="h-8 w-8"
                              onClick={() => void deleteRule(rule.id)}
                              disabled={busy}
                            >
                              <Trash2 className="h-4 w-4" />
                            </Button>
                          </div>
                        </div>
                      </div>
                    )
                  })}
                </div>
              )}

              <div className="rounded-md border border-dashed p-3 space-y-2">
                <div className="flex items-center justify-between gap-2">
                  <div>
                    <p className="text-sm font-medium">非通知アクション</p>
                    <p className="text-xs text-muted-foreground mt-1">
                      {buildActionSummary(anonymousAction.actionType, anonymousAction.actionConfig)}
                    </p>
                  </div>
                  <Button
                    variant={editorMode === "anonymous" ? "default" : "outline"}
                    onClick={() => {
                      setEditorMode("anonymous")
                      setErrorMessage(null)
                    }}
                    disabled={busy}
                  >
                    編集
                  </Button>
                </div>

                <div className="flex items-center justify-between gap-2">
                  <div>
                    <p className="text-sm font-medium">デフォルトアクション</p>
                    <p className="text-xs text-muted-foreground mt-1">
                      {buildActionSummary(defaultAction.actionType, defaultAction.actionConfig)}
                    </p>
                  </div>
                  <Button
                    variant={editorMode === "default" ? "default" : "outline"}
                    onClick={() => {
                      setEditorMode("default")
                      setErrorMessage(null)
                    }}
                    disabled={busy}
                  >
                    編集
                  </Button>
                </div>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle className="text-lg flex items-center gap-2">
                {editorMode === "rule" && "ルール詳細"}
                {editorMode === "anonymous" && (
                  <>
                    <ShieldQuestion className="h-5 w-5" />
                    非通知アクション
                  </>
                )}
                {editorMode === "default" && "デフォルトアクション"}
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              {editorMode === "rule" ? (
                ruleDraft ? (
                  <>
                    <div className="space-y-2">
                      <Label htmlFor="rule-name">ルール名</Label>
                      <Input
                        id="rule-name"
                        value={ruleDraft.name}
                        onChange={(event) =>
                          updateRuleDraft((rule) => ({ ...rule, name: event.target.value }))
                        }
                        disabled={busy}
                      />
                    </div>

                    <div className="grid gap-4 md:grid-cols-2">
                      <div className="space-y-2">
                        <Label>番号グループ</Label>
                        <Select
                          value={ruleDraft.callerGroupId}
                          onValueChange={(value) =>
                            updateRuleDraft((rule) => ({ ...rule, callerGroupId: value }))
                          }
                          disabled={busy || callerGroups.length === 0}
                        >
                          <SelectTrigger>
                            <SelectValue placeholder="グループを選択" />
                          </SelectTrigger>
                          <SelectContent>
                            {callerGroups.map((group) => (
                              <SelectItem key={group.id} value={group.id}>
                                {group.name}
                              </SelectItem>
                            ))}
                          </SelectContent>
                        </Select>
                      </div>

                      <div className="space-y-2">
                        <Label>有効/無効</Label>
                        <div className="h-10 rounded-md border px-3 flex items-center justify-between">
                          <span className="text-sm">このルールを有効にする</span>
                          <Switch
                            checked={ruleDraft.isActive}
                            onCheckedChange={(checked) =>
                              updateRuleDraft((rule) => ({ ...rule, isActive: checked }))
                            }
                            disabled={busy}
                          />
                        </div>
                      </div>
                    </div>

                    {renderActionEditor(ruleDraft.actionType, ruleDraft.actionConfig, {
                      onActionTypeChange: (nextType) =>
                        updateRuleDraft((rule) => ({
                          ...rule,
                          actionType: nextType,
                          actionConfig: applyActionType(rule.actionConfig, nextType),
                        })),
                      onActionCodeChange: (nextCode) =>
                        updateRuleDraft((rule) => ({
                          ...rule,
                          actionConfig: applyActionCode(rule.actionType, rule.actionConfig, nextCode),
                        })),
                      onActionConfigChange: (updater) =>
                        updateRuleDraft((rule) => ({
                          ...rule,
                          actionConfig: updater(rule.actionConfig),
                        })),
                    })}

                    <div className="flex flex-wrap items-center justify-end gap-2">
                      <Button
                        variant="outline"
                        onClick={() => {
                          if (!selectedRule) {
                            return
                          }
                          setRuleDraft(cloneRule(selectedRule))
                          setErrorMessage(null)
                        }}
                        disabled={busy}
                      >
                        <X className="h-4 w-4 mr-1" />
                        キャンセル
                      </Button>
                      <Button onClick={() => void saveRuleDraft()} disabled={busy}>
                        <Save className="h-4 w-4 mr-1" />
                        保存
                      </Button>
                    </div>
                  </>
                ) : (
                  <p className="text-sm text-muted-foreground">ルールを選択してください</p>
                )
              ) : editorMode === "anonymous" ? (
                <>
                  {renderActionEditor(anonymousDraft.actionType, anonymousDraft.actionConfig, {
                    onActionTypeChange: (nextType) =>
                      setAnonymousDraft((current) => ({
                        ...current,
                        actionType: nextType,
                        actionConfig: applyActionType(current.actionConfig, nextType),
                      })),
                    onActionCodeChange: (nextCode) =>
                      setAnonymousDraft((current) => ({
                        ...current,
                        actionConfig: applyActionCode(
                          current.actionType,
                          current.actionConfig,
                          nextCode,
                        ),
                      })),
                    onActionConfigChange: (updater) =>
                      setAnonymousDraft((current) => ({
                        ...current,
                        actionConfig: updater(current.actionConfig),
                      })),
                  })}

                  <div className="flex flex-wrap items-center justify-end gap-2">
                    <Button
                      variant="outline"
                      onClick={() => {
                        setAnonymousDraft(cloneStoredAction(anonymousAction))
                        setErrorMessage(null)
                      }}
                      disabled={busy}
                    >
                      <X className="h-4 w-4 mr-1" />
                      キャンセル
                    </Button>
                    <Button onClick={() => void saveAnonymousDraft()} disabled={busy}>
                      <Save className="h-4 w-4 mr-1" />
                      保存
                    </Button>
                  </div>
                </>
              ) : (
                <>
                  {renderActionEditor(defaultDraft.actionType, defaultDraft.actionConfig, {
                    onActionTypeChange: (nextType) =>
                      setDefaultDraft((current) => ({
                        ...current,
                        actionType: nextType,
                        actionConfig: applyActionType(current.actionConfig, nextType),
                      })),
                    onActionCodeChange: (nextCode) =>
                      setDefaultDraft((current) => ({
                        ...current,
                        actionConfig: applyActionCode(current.actionType, current.actionConfig, nextCode),
                      })),
                    onActionConfigChange: (updater) =>
                      setDefaultDraft((current) => ({
                        ...current,
                        actionConfig: updater(current.actionConfig),
                      })),
                  })}

                  <div className="flex flex-wrap items-center justify-end gap-2">
                    <Button
                      variant="outline"
                      onClick={() => {
                        setDefaultDraft(cloneStoredAction(defaultAction))
                        setErrorMessage(null)
                      }}
                      disabled={busy}
                    >
                      <X className="h-4 w-4 mr-1" />
                      キャンセル
                    </Button>
                    <Button onClick={() => void saveDefaultDraft()} disabled={busy}>
                      <Save className="h-4 w-4 mr-1" />
                      保存
                    </Button>
                  </div>
                </>
              )}
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  )
}
