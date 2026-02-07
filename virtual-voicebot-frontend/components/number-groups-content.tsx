"use client"

import { useEffect, useMemo, useState } from "react"
import { AlertTriangle, CheckCircle2, Plus, Save, Trash2 } from "lucide-react"

import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { ScrollArea } from "@/components/ui/scroll-area"
import { normalizePhoneNumber, type CallerGroup } from "@/lib/call-actions"
import { cn } from "@/lib/utils"

interface NumberGroupsApiResponse {
  ok: boolean
  callerGroups?: CallerGroup[]
  error?: string
}

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

export function NumberGroupsContent() {
  const [callerGroups, setCallerGroups] = useState<CallerGroup[]>([])
  const [selectedGroupId, setSelectedGroupId] = useState<string | null>(null)

  const [newGroupName, setNewGroupName] = useState("")
  const [groupNameInput, setGroupNameInput] = useState("")
  const [groupDescriptionInput, setGroupDescriptionInput] = useState("")
  const [newPhoneNumber, setNewPhoneNumber] = useState("")

  const [loading, setLoading] = useState(true)
  const [busy, setBusy] = useState(false)
  const [errorMessage, setErrorMessage] = useState<string | null>(null)
  const [infoMessage, setInfoMessage] = useState<string | null>(null)

  const selectedGroup = useMemo(
    () => callerGroups.find((group) => group.id === selectedGroupId) ?? null,
    [callerGroups, selectedGroupId],
  )

  useEffect(() => {
    let cancelled = false

    const load = async () => {
      setLoading(true)
      setErrorMessage(null)
      setInfoMessage(null)

      try {
        const response = await fetch("/api/number-groups", { cache: "no-store" })
        const body = (await response.json()) as NumberGroupsApiResponse

        if (!response.ok || !body.ok) {
          throw new Error(body.error ?? "failed to load number groups")
        }

        const groups = Array.isArray(body.callerGroups) ? body.callerGroups : []
        if (cancelled) {
          return
        }

        setCallerGroups(groups)
        setSelectedGroupId(groups[0]?.id ?? null)
      } catch (error) {
        if (cancelled) {
          return
        }
        setErrorMessage(
          error instanceof Error ? error.message : "番号グループの読み込みに失敗しました",
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
  }, [])

  useEffect(() => {
    if (!selectedGroup) {
      setGroupNameInput("")
      setGroupDescriptionInput("")
      return
    }

    setGroupNameInput(selectedGroup.name)
    setGroupDescriptionInput(selectedGroup.description ?? "")
  }, [selectedGroup])

  const saveDatabase = async (
    nextCallerGroups: CallerGroup[],
    options?: {
      message?: string
      nextSelectedGroupId?: string | null
    },
  ): Promise<boolean> => {
    setBusy(true)
    setErrorMessage(null)
    setInfoMessage(null)

    try {
      const response = await fetch("/api/number-groups", {
        method: "PUT",
        headers: {
          "content-type": "application/json",
        },
        body: JSON.stringify({ callerGroups: nextCallerGroups }),
      })
      const body = (await response.json()) as { ok: boolean; error?: string }

      if (!response.ok || !body.ok) {
        throw new Error(body.error ?? "failed to save number groups")
      }

      setCallerGroups(nextCallerGroups)
      const nextSelected =
        options?.nextSelectedGroupId !== undefined
          ? options.nextSelectedGroupId
          : nextCallerGroups.find((group) => group.id === selectedGroupId)?.id ??
            nextCallerGroups[0]?.id ??
            null
      setSelectedGroupId(nextSelected)

      if (options?.message) {
        setInfoMessage(options.message)
      }
      return true
    } catch (error) {
      setErrorMessage(error instanceof Error ? error.message : "保存に失敗しました")
      return false
    } finally {
      setBusy(false)
    }
  }

  const addCallerGroup = async () => {
    const name = newGroupName.trim()
    if (!name) {
      setErrorMessage("グループ名を入力してください")
      return
    }

    const timestamp = nowIso()
    const newGroup: CallerGroup = {
      id: createId(),
      name,
      description: null,
      phoneNumbers: [],
      createdAt: timestamp,
      updatedAt: timestamp,
    }

    const saved = await saveDatabase([...callerGroups, newGroup], {
      message: "グループを追加しました",
      nextSelectedGroupId: newGroup.id,
    })
    if (saved) {
      setNewGroupName("")
    }
  }

  const saveSelectedGroup = async () => {
    if (!selectedGroup) {
      return
    }

    const name = groupNameInput.trim()
    if (!name) {
      setErrorMessage("グループ名を入力してください")
      return
    }

    const description = groupDescriptionInput.trim()
    const timestamp = nowIso()

    const nextCallerGroups = callerGroups.map((group) =>
      group.id === selectedGroup.id
        ? {
            ...group,
            name,
            description: description.length > 0 ? description : null,
            updatedAt: timestamp,
          }
        : group,
    )

    await saveDatabase(nextCallerGroups, {
      message: "グループ情報を更新しました",
    })
  }

  const deleteSelectedGroup = async () => {
    if (!selectedGroup) {
      return
    }

    const nextCallerGroups = callerGroups.filter((group) => group.id !== selectedGroup.id)
    await saveDatabase(nextCallerGroups, {
      message: "グループを削除しました",
      nextSelectedGroupId: nextCallerGroups[0]?.id ?? null,
    })
  }

  const addPhoneNumberToSelectedGroup = async () => {
    if (!selectedGroup) {
      return
    }

    const normalized = normalizePhoneNumber(newPhoneNumber)
    if (!normalized) {
      setErrorMessage("有効な電話番号を入力してください")
      return
    }

    if (selectedGroup.phoneNumbers.includes(normalized)) {
      setErrorMessage("この番号は既に登録されています")
      return
    }

    const duplicateAcrossGroup = callerGroups.some(
      (group) => group.id !== selectedGroup.id && group.phoneNumbers.includes(normalized),
    )

    const timestamp = nowIso()
    const nextCallerGroups = callerGroups.map((group) =>
      group.id === selectedGroup.id
        ? {
            ...group,
            phoneNumbers: [...group.phoneNumbers, normalized],
            updatedAt: timestamp,
          }
        : group,
    )

    const saved = await saveDatabase(nextCallerGroups, {
      message: duplicateAcrossGroup
        ? "番号を追加しました（別グループにも同一番号があります）"
        : "番号を追加しました",
    })
    if (saved) {
      setNewPhoneNumber("")
    }
  }

  const removePhoneNumberFromSelectedGroup = async (phoneNumber: string) => {
    if (!selectedGroup) {
      return
    }

    const timestamp = nowIso()
    const nextCallerGroups = callerGroups.map((group) =>
      group.id === selectedGroup.id
        ? {
            ...group,
            phoneNumbers: group.phoneNumbers.filter((value) => value !== phoneNumber),
            updatedAt: timestamp,
          }
        : group,
    )

    await saveDatabase(nextCallerGroups, {
      message: "番号を削除しました",
    })
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
        <h1 className="text-2xl font-bold text-balance">番号グループ</h1>
        <p className="text-muted-foreground">電話番号をグループで管理</p>
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
            <CardTitle className="text-lg">グループ一覧</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="flex gap-2">
              <Input
                value={newGroupName}
                onChange={(event) => setNewGroupName(event.target.value)}
                placeholder="新規グループ名"
                disabled={busy}
              />
              <Button onClick={addCallerGroup} disabled={busy}>
                <Plus className="h-4 w-4 mr-1" />
                追加
              </Button>
            </div>

            <ScrollArea className="h-[360px] rounded-md border">
              <div className="p-2 space-y-1">
                {callerGroups.length === 0 ? (
                  <p className="text-sm text-muted-foreground p-2">グループがありません</p>
                ) : (
                  callerGroups.map((group) => (
                    <button
                      key={group.id}
                      type="button"
                      onClick={() => {
                        setSelectedGroupId(group.id)
                        setErrorMessage(null)
                        setInfoMessage(null)
                      }}
                      className={cn(
                        "w-full text-left px-3 py-2 rounded-md border transition-colors",
                        selectedGroupId === group.id
                          ? "border-primary bg-primary/10"
                          : "border-transparent hover:bg-accent",
                      )}
                    >
                      <div className="font-medium truncate">{group.name}</div>
                      <div className="text-xs text-muted-foreground flex items-center justify-between">
                        <span>{group.description ?? "説明なし"}</span>
                        <Badge variant="secondary">{group.phoneNumbers.length}</Badge>
                      </div>
                    </button>
                  ))
                )}
              </div>
            </ScrollArea>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <div className="flex items-center justify-between gap-3">
              <CardTitle className="text-lg">グループ詳細</CardTitle>
              <Button
                variant="destructive"
                size="sm"
                onClick={deleteSelectedGroup}
                disabled={!selectedGroup || busy}
              >
                <Trash2 className="h-4 w-4 mr-1" />
                削除
              </Button>
            </div>
          </CardHeader>
          <CardContent className="space-y-4">
            {selectedGroup ? (
              <>
                <div className="space-y-2">
                  <Label htmlFor="group-name">グループ名</Label>
                  <Input
                    id="group-name"
                    value={groupNameInput}
                    onChange={(event) => setGroupNameInput(event.target.value)}
                    disabled={busy}
                  />
                </div>

                <div className="space-y-2">
                  <Label htmlFor="group-description">説明</Label>
                  <Input
                    id="group-description"
                    value={groupDescriptionInput}
                    onChange={(event) => setGroupDescriptionInput(event.target.value)}
                    placeholder="任意"
                    disabled={busy}
                  />
                </div>

                <Button onClick={saveSelectedGroup} disabled={busy}>
                  <Save className="h-4 w-4 mr-1" />
                  グループ保存
                </Button>

                <div className="space-y-2">
                  <Label>電話番号</Label>
                  <div className="flex gap-2">
                    <Input
                      value={newPhoneNumber}
                      onChange={(event) => setNewPhoneNumber(event.target.value)}
                      placeholder="090-1234-5678"
                      disabled={busy}
                    />
                    <Button onClick={addPhoneNumberToSelectedGroup} disabled={busy}>
                      <Plus className="h-4 w-4" />
                    </Button>
                  </div>

                  <div className="space-y-1 max-h-56 overflow-auto pr-1">
                    {selectedGroup.phoneNumbers.length === 0 ? (
                      <p className="text-xs text-muted-foreground">番号が未登録です</p>
                    ) : (
                      selectedGroup.phoneNumbers.map((phoneNumber) => (
                        <div
                          key={phoneNumber}
                          className="flex items-center justify-between rounded-md border px-2 py-1"
                        >
                          <code className="text-xs">{phoneNumber}</code>
                          <Button
                            variant="ghost"
                            size="sm"
                            className="h-7 px-2"
                            onClick={() => removePhoneNumberFromSelectedGroup(phoneNumber)}
                            disabled={busy}
                          >
                            <Trash2 className="h-3.5 w-3.5" />
                          </Button>
                        </div>
                      ))
                    )}
                  </div>
                </div>
              </>
            ) : (
              <p className="text-sm text-muted-foreground">グループを選択してください</p>
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  )
}
