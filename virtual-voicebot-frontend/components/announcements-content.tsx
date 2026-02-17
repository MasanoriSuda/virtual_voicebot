"use client"

import { useEffect, useMemo, useState, type FormEvent } from "react"
import {
  ChevronDown,
  ChevronRight,
  Edit,
  FileAudio,
  Folder,
  FolderOpen,
  MessageSquare,
  Mic,
  MoreHorizontal,
  Phone,
  PhoneOff,
  Plus,
  Search,
  Trash2,
  Upload,
  Volume2,
} from "lucide-react"

import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { Input } from "@/components/ui/input"
import { ScrollArea } from "@/components/ui/scroll-area"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { Switch } from "@/components/ui/switch"
import { cn } from "@/lib/utils"
import type { AnnouncementType } from "@/lib/types"

interface StoredAnnouncement {
  id: string
  name: string
  description: string | null
  announcementType: AnnouncementType
  isActive: boolean
  folderId: string | null
  audioFileUrl: string | null
  ttsText: string | null
  speakerId: number | null
  speakerName: string | null
  durationSec: number | null
  language: string
  source: "upload" | "tts"
  createdAt: string
  updatedAt: string
}

interface StoredFolder {
  id: string
  name: string
  description: string | null
  parentId: string | null
  sortOrder: number
  createdAt: string
  updatedAt: string
}

interface AnnouncementsApiResponse {
  ok: boolean
  announcements: StoredAnnouncement[]
  folders: StoredFolder[]
  error?: string
}

interface VoiceVoxSpeakersResponse {
  ok: boolean
  speakers: Array<{
    name: string
    styles: Array<{ id: number; name: string }>
  }>
  error?: string
}

interface FolderNode extends StoredFolder {
  children: FolderNode[]
}

interface SpeakerOption {
  id: number
  label: string
  speakerName: string
}

const announcementTypeConfig: Record<
  AnnouncementType,
  { label: string; icon: typeof Volume2; color: string }
> = {
  greeting: {
    label: "挨拶",
    icon: MessageSquare,
    color: "bg-blue-500/10 text-blue-600",
  },
  hold: {
    label: "保留",
    icon: Phone,
    color: "bg-green-500/10 text-green-600",
  },
  ivr: {
    label: "IVR",
    icon: Volume2,
    color: "bg-indigo-500/10 text-indigo-600",
  },
  closed: {
    label: "時間外",
    icon: PhoneOff,
    color: "bg-red-500/10 text-red-600",
  },
  recording_notice: {
    label: "録音通知",
    icon: Mic,
    color: "bg-teal-500/10 text-teal-600",
  },
  custom: {
    label: "カスタム",
    icon: FileAudio,
    color: "bg-amber-500/10 text-amber-600",
  },
}

const announcementTypes: AnnouncementType[] = [
  "greeting",
  "hold",
  "ivr",
  "closed",
  "recording_notice",
  "custom",
]

function buildFolderTree(folders: StoredFolder[]): FolderNode[] {
  const byId = new Map<string, FolderNode>()
  for (const folder of folders) {
    byId.set(folder.id, { ...folder, children: [] })
  }

  const roots: FolderNode[] = []
  for (const folder of byId.values()) {
    if (folder.parentId && byId.has(folder.parentId)) {
      byId.get(folder.parentId)?.children.push(folder)
    } else {
      roots.push(folder)
    }
  }

  const sortNodes = (nodes: FolderNode[]) => {
    nodes.sort((a, b) => {
      if (a.sortOrder !== b.sortOrder) {
        return a.sortOrder - b.sortOrder
      }
      return a.name.localeCompare(b.name, "ja")
    })
    for (const node of nodes) {
      sortNodes(node.children)
    }
  }

  sortNodes(roots)
  return roots
}

function formatDuration(durationSec: number | null): string {
  if (typeof durationSec !== "number" || !Number.isFinite(durationSec)) {
    return "--:--"
  }
  const totalSeconds = Math.max(0, Math.round(durationSec))
  const min = Math.floor(totalSeconds / 60)
  const sec = totalSeconds % 60
  return `${min}:${String(sec).padStart(2, "0")}`
}

function inferNameFromFile(fileName: string): string {
  const base = fileName.replace(/\.[^.]+$/, "")
  const trimmed = base.trim()
  return trimmed.length > 0 ? trimmed : "新規アナウンス"
}

function inferNameFromText(text: string): string {
  const normalized = text.replace(/\s+/g, " ").trim()
  if (!normalized) {
    return "新規TTS"
  }
  return normalized.slice(0, 20)
}

export function AnnouncementsContent() {
  const [folders, setFolders] = useState<StoredFolder[]>([])
  const [announcements, setAnnouncements] = useState<StoredAnnouncement[]>([])
  const [selectedFolderId, setSelectedFolderId] = useState<string | null>(null)
  const [expandedFolderIds, setExpandedFolderIds] = useState<Set<string>>(new Set())
  const [searchQuery, setSearchQuery] = useState("")
  const [loading, setLoading] = useState(true)
  const [busy, setBusy] = useState(false)
  const [errorMessage, setErrorMessage] = useState<string | null>(null)
  const [infoMessage, setInfoMessage] = useState<string | null>(null)

  const [editingId, setEditingId] = useState<string | null>(null)
  const [editingName, setEditingName] = useState("")

  const [dialogMode, setDialogMode] = useState<"upload" | "tts" | null>(null)

  const [uploadFile, setUploadFile] = useState<File | null>(null)
  const [uploadName, setUploadName] = useState("")
  const [uploadType, setUploadType] = useState<AnnouncementType>("custom")

  const [ttsText, setTtsText] = useState("")
  const [ttsSpeakerId, setTtsSpeakerId] = useState<string>("")
  const [ttsName, setTtsName] = useState("")
  const [ttsType, setTtsType] = useState<AnnouncementType>("custom")
  const [speakersLoading, setSpeakersLoading] = useState(false)
  const [speakerOptions, setSpeakerOptions] = useState<SpeakerOption[]>([])

  const folderTree = useMemo(() => buildFolderTree(folders), [folders])

  const selectedFolder = useMemo(
    () => folders.find((folder) => folder.id === selectedFolderId) ?? null,
    [folders, selectedFolderId],
  )

  const visibleAnnouncements = useMemo(() => {
    const keyword = searchQuery.trim().toLowerCase()
    return announcements
      .filter((item) => {
        if (selectedFolderId !== null && item.folderId !== selectedFolderId) {
          return false
        }
        if (!keyword) {
          return true
        }
        const searchable = [
          item.name,
          item.description,
          item.ttsText,
          item.speakerName,
          item.language,
        ]
          .filter((v): v is string => typeof v === "string")
          .join(" ")
          .toLowerCase()
        return searchable.includes(keyword)
      })
      .sort((a, b) => Date.parse(b.updatedAt) - Date.parse(a.updatedAt))
  }, [announcements, searchQuery, selectedFolderId])

  const loadAnnouncements = async () => {
    setLoading(true)
    setErrorMessage(null)
    try {
      const response = await fetch("/api/announcements", { cache: "no-store" })
      const payload = (await response.json()) as AnnouncementsApiResponse
      if (!response.ok || !payload.ok) {
        throw new Error(payload.error || "failed to fetch announcements")
      }

      setFolders(payload.folders)
      setAnnouncements(payload.announcements)
      setExpandedFolderIds((prev) => {
        if (prev.size > 0) {
          return prev
        }
        return new Set(payload.folders.filter((item) => item.parentId === null).map((item) => item.id))
      })
      setSelectedFolderId((prev) => {
        if (prev && payload.folders.some((folder) => folder.id === prev)) {
          return prev
        }
        const firstRoot = payload.folders
          .filter((item) => item.parentId === null)
          .sort((a, b) => a.sortOrder - b.sortOrder)[0]
        return firstRoot?.id ?? null
      })
    } catch (error) {
      setErrorMessage(error instanceof Error ? error.message : "failed to load announcements")
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    void loadAnnouncements()
  }, [])

  const loadSpeakers = async () => {
    setSpeakersLoading(true)
    setErrorMessage(null)
    try {
      const response = await fetch("/api/announcements/speakers", { cache: "no-store" })
      const payload = (await response.json()) as VoiceVoxSpeakersResponse
      if (!response.ok || !payload.ok) {
        throw new Error(payload.error || "failed to load speakers")
      }

      const options = payload.speakers.flatMap((speaker) =>
        speaker.styles.map((style) => ({
          id: style.id,
          label: `${speaker.name} - ${style.name}`,
          speakerName: speaker.name,
        })),
      )
      setSpeakerOptions(options)
      if (options.length > 0 && !options.some((item) => item.id === Number(ttsSpeakerId))) {
        setTtsSpeakerId(String(options[0].id))
      }
    } catch (error) {
      setErrorMessage(error instanceof Error ? error.message : "failed to load speakers")
    } finally {
      setSpeakersLoading(false)
    }
  }

  const openUpload = () => {
    setDialogMode("upload")
    setUploadFile(null)
    setUploadName("")
    setUploadType("custom")
    setInfoMessage(null)
    setErrorMessage(null)
  }

  const openTts = async () => {
    setDialogMode("tts")
    setTtsText("")
    setTtsName("")
    setTtsType("custom")
    setInfoMessage(null)
    setErrorMessage(null)
    await loadSpeakers()
  }

  const closeDialog = () => {
    setDialogMode(null)
  }

  const refreshAfterMutation = async (message: string) => {
    setInfoMessage(message)
    await loadAnnouncements()
  }

  const handleUploadSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    if (!uploadFile) {
      setErrorMessage("WAV ファイルを選択してください")
      return
    }
    if (!uploadName.trim()) {
      setErrorMessage("アナウンス名を入力してください")
      return
    }

    setBusy(true)
    setErrorMessage(null)
    setInfoMessage(null)
    try {
      const formData = new FormData()
      formData.set("file", uploadFile)
      formData.set("name", uploadName.trim())
      formData.set("announcementType", uploadType)
      if (selectedFolderId) {
        formData.set("folderId", selectedFolderId)
      }

      const response = await fetch("/api/announcements/upload", {
        method: "POST",
        body: formData,
      })
      const payload = (await response.json()) as { ok?: boolean; error?: string }
      if (!response.ok || !payload.ok) {
        throw new Error(payload.error || "failed to upload")
      }

      closeDialog()
      await refreshAfterMutation("アナウンスをアップロードしました")
    } catch (error) {
      setErrorMessage(error instanceof Error ? error.message : "アップロードに失敗しました")
    } finally {
      setBusy(false)
    }
  }

  const handleTtsSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    if (!ttsText.trim()) {
      setErrorMessage("読み上げテキストを入力してください")
      return
    }
    if (!ttsName.trim()) {
      setErrorMessage("アナウンス名を入力してください")
      return
    }
    if (!ttsSpeakerId) {
      setErrorMessage("キャラクターを選択してください")
      return
    }

    setBusy(true)
    setErrorMessage(null)
    setInfoMessage(null)
    try {
      const response = await fetch("/api/announcements/tts", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          text: ttsText,
          speakerId: Number(ttsSpeakerId),
          name: ttsName.trim(),
          announcementType: ttsType,
          folderId: selectedFolderId,
        }),
      })
      const payload = (await response.json()) as { ok?: boolean; error?: string }
      if (!response.ok || !payload.ok) {
        throw new Error(payload.error || "failed to synthesize")
      }

      closeDialog()
      await refreshAfterMutation("音声を生成しました")
    } catch (error) {
      setErrorMessage(error instanceof Error ? error.message : "TTS 生成に失敗しました")
    } finally {
      setBusy(false)
    }
  }

  const handleToggleActive = async (announcement: StoredAnnouncement) => {
    setBusy(true)
    setErrorMessage(null)
    try {
      const response = await fetch(`/api/announcements/${encodeURIComponent(announcement.id)}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ isActive: !announcement.isActive }),
      })
      const payload = (await response.json()) as {
        ok?: boolean
        error?: string
        announcement?: StoredAnnouncement
      }
      if (!response.ok || !payload.ok || !payload.announcement) {
        throw new Error(payload.error || "failed to update announcement")
      }
      setAnnouncements((prev) =>
        prev.map((item) => (item.id === payload.announcement?.id ? payload.announcement : item)),
      )
      setInfoMessage("状態を更新しました")
    } catch (error) {
      setErrorMessage(error instanceof Error ? error.message : "更新に失敗しました")
    } finally {
      setBusy(false)
    }
  }

  const handleDelete = async (announcement: StoredAnnouncement) => {
    const ok = window.confirm("このアナウンスを削除します。元に戻せません。")
    if (!ok) {
      return
    }

    setBusy(true)
    setErrorMessage(null)
    try {
      const response = await fetch(`/api/announcements/${encodeURIComponent(announcement.id)}`, {
        method: "DELETE",
      })
      const payload = (await response.json()) as { ok?: boolean; error?: string }
      if (!response.ok || !payload.ok) {
        throw new Error(payload.error || "failed to delete announcement")
      }
      setAnnouncements((prev) => prev.filter((item) => item.id !== announcement.id))
      setInfoMessage("アナウンスを削除しました")
    } catch (error) {
      setErrorMessage(error instanceof Error ? error.message : "削除に失敗しました")
    } finally {
      setBusy(false)
    }
  }

  const startRename = (announcement: StoredAnnouncement) => {
    setEditingId(announcement.id)
    setEditingName(announcement.name)
  }

  const cancelRename = () => {
    setEditingId(null)
    setEditingName("")
  }

  const submitRename = async (announcementId: string) => {
    if (!editingName.trim()) {
      setErrorMessage("名称を入力してください")
      return
    }

    setBusy(true)
    setErrorMessage(null)
    try {
      const response = await fetch(`/api/announcements/${encodeURIComponent(announcementId)}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ name: editingName.trim() }),
      })
      const payload = (await response.json()) as {
        ok?: boolean
        error?: string
        announcement?: StoredAnnouncement
      }
      if (!response.ok || !payload.ok || !payload.announcement) {
        throw new Error(payload.error || "failed to rename announcement")
      }
      setAnnouncements((prev) =>
        prev.map((item) => (item.id === payload.announcement?.id ? payload.announcement : item)),
      )
      setEditingId(null)
      setEditingName("")
      setInfoMessage("名称を更新しました")
    } catch (error) {
      setErrorMessage(error instanceof Error ? error.message : "名称更新に失敗しました")
    } finally {
      setBusy(false)
    }
  }

  const toggleFolderExpand = (folderId: string) => {
    setExpandedFolderIds((prev) => {
      const next = new Set(prev)
      if (next.has(folderId)) {
        next.delete(folderId)
      } else {
        next.add(folderId)
      }
      return next
    })
  }

  const renderFolderTree = (nodes: FolderNode[], level = 0): React.ReactNode => {
    return nodes.map((node) => {
      const expanded = expandedFolderIds.has(node.id)
      const selected = selectedFolderId === node.id
      const hasChildren = node.children.length > 0

      return (
        <div key={node.id}>
          <button
            type="button"
            className={cn(
              "w-full text-left flex items-center gap-2 rounded-md px-2 py-1.5 hover:bg-muted",
              selected && "bg-primary/10 text-primary",
            )}
            style={{ paddingLeft: `${level * 16 + 8}px` }}
            onClick={() => setSelectedFolderId(node.id)}
          >
            {hasChildren ? (
              <span
                className="flex items-center"
                onClick={(event) => {
                  event.stopPropagation()
                  toggleFolderExpand(node.id)
                }}
              >
                {expanded ? <ChevronDown className="h-4 w-4" /> : <ChevronRight className="h-4 w-4" />}
              </span>
            ) : (
              <span className="w-4" />
            )}
            {expanded ? <FolderOpen className="h-4 w-4 text-amber-500" /> : <Folder className="h-4 w-4 text-amber-500" />}
            <span className="truncate text-sm">{node.name}</span>
          </button>
          {hasChildren && expanded ? renderFolderTree(node.children, level + 1) : null}
        </div>
      )
    })
  }

  return (
    <div className="flex h-full">
      <div className="w-80 border-r flex flex-col bg-card">
        <div className="p-4 border-b space-y-3">
          <div className="flex items-center justify-between">
            <h2 className="font-semibold text-lg">アナウンス</h2>
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button size="sm" disabled={busy}>
                  <Plus className="h-4 w-4 mr-1" />
                  追加
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                <DropdownMenuItem onClick={openUpload}>
                  <Upload className="h-4 w-4 mr-2" />
                  音声ファイルをアップロード
                </DropdownMenuItem>
                <DropdownMenuItem onClick={() => void openTts()}>
                  <MessageSquare className="h-4 w-4 mr-2" />
                  テキスト読み上げ
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
          <div className="relative">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
            <Input
              placeholder="検索..."
              value={searchQuery}
              onChange={(event) => setSearchQuery(event.target.value)}
              className="pl-9"
            />
          </div>
          <Button
            type="button"
            variant={selectedFolderId === null ? "default" : "outline"}
            size="sm"
            className="w-full"
            onClick={() => setSelectedFolderId(null)}
          >
            すべて表示
          </Button>
        </div>

        <ScrollArea className="flex-1">
          <div className="p-2">
            {folderTree.length > 0 ? (
              renderFolderTree(folderTree)
            ) : (
              <p className="text-sm text-muted-foreground p-2">フォルダがありません</p>
            )}
          </div>
        </ScrollArea>
      </div>

      <div className="flex-1 overflow-auto bg-background">
        <div className="p-6 space-y-4">
          <div>
            <h1 className="text-2xl font-bold">
              {selectedFolder ? selectedFolder.name : "すべてのアナウンス"}
            </h1>
            <p className="text-sm text-muted-foreground">
              {selectedFolder?.description ?? "アナウンスを検索・再生・管理できます"}
            </p>
          </div>

          {loading ? <p className="text-muted-foreground">読み込み中...</p> : null}
          {errorMessage ? (
            <div className="rounded-md border border-destructive/30 bg-destructive/5 px-3 py-2 text-sm text-destructive">
              {errorMessage}
            </div>
          ) : null}
          {infoMessage ? (
            <div className="rounded-md border border-emerald-500/30 bg-emerald-500/5 px-3 py-2 text-sm text-emerald-700">
              {infoMessage}
            </div>
          ) : null}

          {dialogMode === "upload" ? (
            <Card>
              <CardHeader>
                <CardTitle className="text-base">音声ファイルをアップロード</CardTitle>
              </CardHeader>
              <CardContent>
                <form className="space-y-3" onSubmit={handleUploadSubmit}>
                  <input
                    type="file"
                    accept=".wav,audio/wav,audio/x-wav"
                    onChange={(event) => {
                      const file = event.target.files?.[0] ?? null
                      setUploadFile(file)
                      if (file) {
                        setUploadName((prev) => (prev.trim().length > 0 ? prev : inferNameFromFile(file.name)))
                      }
                    }}
                  />
                  <Input
                    placeholder="アナウンス名"
                    value={uploadName}
                    onChange={(event) => setUploadName(event.target.value)}
                  />
                  <Select value={uploadType} onValueChange={(value) => setUploadType(value as AnnouncementType)}>
                    <SelectTrigger>
                      <SelectValue placeholder="タイプ" />
                    </SelectTrigger>
                    <SelectContent>
                      {announcementTypes.map((type) => (
                        <SelectItem key={type} value={type}>
                          {announcementTypeConfig[type].label}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                  <div className="flex items-center gap-2">
                    <Button type="submit" disabled={busy}>
                      保存
                    </Button>
                    <Button type="button" variant="outline" onClick={closeDialog} disabled={busy}>
                      キャンセル
                    </Button>
                  </div>
                </form>
              </CardContent>
            </Card>
          ) : null}

          {dialogMode === "tts" ? (
            <Card>
              <CardHeader>
                <CardTitle className="text-base">テキスト読み上げ（VoiceVox）</CardTitle>
              </CardHeader>
              <CardContent>
                <form className="space-y-3" onSubmit={handleTtsSubmit}>
                  <textarea
                    className="w-full min-h-28 rounded-md border border-input bg-background px-3 py-2 text-sm"
                    placeholder="読み上げテキスト（1〜1000文字）"
                    value={ttsText}
                    onChange={(event) => {
                      const text = event.target.value
                      setTtsText(text)
                      setTtsName((prev) => (prev.trim().length > 0 ? prev : inferNameFromText(text)))
                    }}
                  />
                  <Select value={ttsSpeakerId} onValueChange={setTtsSpeakerId}>
                    <SelectTrigger>
                      <SelectValue placeholder={speakersLoading ? "読み込み中..." : "キャラクターを選択"} />
                    </SelectTrigger>
                    <SelectContent>
                      {speakerOptions.map((speaker) => (
                        <SelectItem key={speaker.id} value={String(speaker.id)}>
                          {speaker.label}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                  <Input
                    placeholder="アナウンス名"
                    value={ttsName}
                    onChange={(event) => setTtsName(event.target.value)}
                  />
                  <Select value={ttsType} onValueChange={(value) => setTtsType(value as AnnouncementType)}>
                    <SelectTrigger>
                      <SelectValue placeholder="タイプ" />
                    </SelectTrigger>
                    <SelectContent>
                      {announcementTypes.map((type) => (
                        <SelectItem key={type} value={type}>
                          {announcementTypeConfig[type].label}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                  <div className="flex items-center gap-2">
                    <Button type="submit" disabled={busy || speakersLoading}>
                      生成
                    </Button>
                    <Button type="button" variant="outline" onClick={closeDialog} disabled={busy}>
                      キャンセル
                    </Button>
                  </div>
                </form>
              </CardContent>
            </Card>
          ) : null}

          <Card>
            <CardHeader>
              <CardTitle className="text-base">アナウンス一覧 ({visibleAnnouncements.length}件)</CardTitle>
            </CardHeader>
            <CardContent>
              {visibleAnnouncements.length === 0 ? (
                <p className="text-sm text-muted-foreground py-8 text-center">
                  該当するアナウンスがありません
                </p>
              ) : (
                <div className="space-y-4">
                  {visibleAnnouncements.map((announcement) => {
                    const config = announcementTypeConfig[announcement.announcementType]
                    const TypeIcon = config.icon
                    return (
                      <div key={announcement.id} className="rounded-lg border p-4 space-y-3">
                        <div className="flex items-start justify-between gap-3">
                          <div className="space-y-1 min-w-0 flex-1">
                            {editingId === announcement.id ? (
                              <div className="flex items-center gap-2">
                                <Input
                                  value={editingName}
                                  onChange={(event) => setEditingName(event.target.value)}
                                />
                                <Button
                                  size="sm"
                                  onClick={() => void submitRename(announcement.id)}
                                  disabled={busy}
                                >
                                  保存
                                </Button>
                                <Button size="sm" variant="outline" onClick={cancelRename} disabled={busy}>
                                  キャンセル
                                </Button>
                              </div>
                            ) : (
                              <p className="font-medium truncate">{announcement.name}</p>
                            )}
                            {announcement.description ? (
                              <p className="text-sm text-muted-foreground">{announcement.description}</p>
                            ) : null}
                            {announcement.ttsText ? (
                              <p className="text-xs text-muted-foreground line-clamp-2">
                                {announcement.ttsText}
                              </p>
                            ) : null}
                            <div className="flex items-center gap-2 flex-wrap">
                              <Badge variant="outline" className={config.color}>
                                <TypeIcon className="h-3 w-3 mr-1" />
                                {config.label}
                              </Badge>
                              <Badge variant="secondary">{announcement.language}</Badge>
                              <Badge variant="secondary">
                                {announcement.source === "tts" ? "TTS" : "Upload"}
                              </Badge>
                              <Badge variant="secondary">{formatDuration(announcement.durationSec)}</Badge>
                            </div>
                            {announcement.speakerName ? (
                              <p className="text-xs text-muted-foreground">{announcement.speakerName}</p>
                            ) : null}
                          </div>
                          <div className="flex items-center gap-2">
                            <Switch
                              checked={announcement.isActive}
                              onCheckedChange={() => void handleToggleActive(announcement)}
                              disabled={busy}
                            />
                            <DropdownMenu>
                              <DropdownMenuTrigger asChild>
                                <Button variant="ghost" size="icon" disabled={busy}>
                                  <MoreHorizontal className="h-4 w-4" />
                                </Button>
                              </DropdownMenuTrigger>
                              <DropdownMenuContent align="end">
                                <DropdownMenuItem onClick={() => startRename(announcement)}>
                                  <Edit className="h-4 w-4 mr-2" />
                                  名称変更
                                </DropdownMenuItem>
                                <DropdownMenuSeparator />
                                <DropdownMenuItem
                                  className="text-destructive"
                                  onClick={() => void handleDelete(announcement)}
                                >
                                  <Trash2 className="h-4 w-4 mr-2" />
                                  削除
                                </DropdownMenuItem>
                              </DropdownMenuContent>
                            </DropdownMenu>
                          </div>
                        </div>

                        {announcement.audioFileUrl ? (
                          <div className="space-y-2">
                            <audio className="w-full" controls preload="metadata" src={announcement.audioFileUrl} />
                            <div className="text-xs text-muted-foreground break-all flex items-center gap-1">
                              <FileAudio className="h-3.5 w-3.5" />
                              <span>{announcement.audioFileUrl}</span>
                            </div>
                          </div>
                        ) : (
                          <p className="text-sm text-muted-foreground">音声ファイル未登録</p>
                        )}
                      </div>
                    )
                  })}
                </div>
              )}
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  )
}
