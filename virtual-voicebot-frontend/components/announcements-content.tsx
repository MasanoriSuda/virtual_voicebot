"use client"

import { useState } from "react"
import {
  ChevronDown,
  ChevronRight,
  Copy,
  Edit,
  Folder,
  FolderOpen,
  MoreHorizontal,
  Plus,
  Search,
  Trash2,
  Volume2,
  Play,
  Pause,
  Upload,
  Mic,
  FileAudio,
  MessageSquare,
  Phone,
  PhoneOff,
  Megaphone,
} from "lucide-react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Badge } from "@/components/ui/badge"
import { Switch } from "@/components/ui/switch"
import { ScrollArea } from "@/components/ui/scroll-area"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger,
} from "@/components/ui/context-menu"
import { cn } from "@/lib/utils"
import type { AnnouncementFolder, Announcement, AnnouncementType } from "@/lib/types"

// Mock data
const mockAnnouncementTree: AnnouncementFolder[] = [
  {
    id: "root-1",
    name: "挨拶メッセージ",
    description: "着信時の挨拶音声",
    parentId: null,
    type: "folder",
    createdAt: "2024-01-01T00:00:00Z",
    updatedAt: "2024-01-15T00:00:00Z",
    children: [
      {
        id: "ann-1",
        name: "日本語挨拶",
        description: "日本語の挨拶メッセージ",
        parentId: "root-1",
        type: "announcement",
        createdAt: "2024-01-01T00:00:00Z",
        updatedAt: "2024-01-10T00:00:00Z",
        announcements: [
          {
            id: "a1",
            name: "営業時間内挨拶",
            description: "通常の営業時間内の挨拶",
            type: "greeting",
            enabled: true,
            audioUrl: "/audio/greeting-ja.mp3",
            duration: 8,
            language: "ja-JP",
          },
          {
            id: "a2",
            name: "混雑時挨拶",
            description: "混雑時の待機案内",
            type: "greeting",
            enabled: true,
            textToSpeech: "お電話ありがとうございます。ただいま大変混み合っております。しばらくお待ちください。",
            duration: 6,
            language: "ja-JP",
          },
        ],
      },
      {
        id: "ann-2",
        name: "英語挨拶",
        description: "英語の挨拶メッセージ",
        parentId: "root-1",
        type: "announcement",
        createdAt: "2024-01-02T00:00:00Z",
        updatedAt: "2024-01-12T00:00:00Z",
        announcements: [
          {
            id: "a3",
            name: "English Greeting",
            description: "Standard English greeting",
            type: "greeting",
            enabled: true,
            audioUrl: "/audio/greeting-en.mp3",
            duration: 7,
            language: "en-US",
          },
        ],
      },
    ],
  },
  {
    id: "root-2",
    name: "保留音",
    description: "保留中の音声",
    parentId: null,
    type: "folder",
    createdAt: "2024-01-01T00:00:00Z",
    updatedAt: "2024-01-20T00:00:00Z",
    children: [
      {
        id: "ann-3",
        name: "BGM",
        description: "保留中のBGM",
        parentId: "root-2",
        type: "announcement",
        createdAt: "2024-01-01T00:00:00Z",
        updatedAt: "2024-01-05T00:00:00Z",
        announcements: [
          {
            id: "a4",
            name: "クラシック保留音",
            type: "hold",
            enabled: true,
            audioUrl: "/audio/hold-classic.mp3",
            duration: 120,
            language: "universal",
          },
          {
            id: "a5",
            name: "ジャズ保留音",
            type: "hold",
            enabled: false,
            audioUrl: "/audio/hold-jazz.mp3",
            duration: 180,
            language: "universal",
          },
        ],
      },
      {
        id: "ann-4",
        name: "待機案内",
        description: "待機中のアナウンス",
        parentId: "root-2",
        type: "announcement",
        createdAt: "2024-02-01T00:00:00Z",
        updatedAt: "2024-02-10T00:00:00Z",
        announcements: [
          {
            id: "a6",
            name: "お待たせアナウンス",
            type: "hold",
            enabled: true,
            textToSpeech: "お待たせしております。まもなくオペレーターにお繋ぎいたします。",
            duration: 5,
            language: "ja-JP",
          },
        ],
      },
    ],
  },
  {
    id: "root-3",
    name: "IVRメニュー",
    description: "自動音声応答メニュー",
    parentId: null,
    type: "folder",
    createdAt: "2024-01-01T00:00:00Z",
    updatedAt: "2024-01-25T00:00:00Z",
    children: [
      {
        id: "ann-5",
        name: "メインメニュー",
        description: "IVRメインメニュー音声",
        parentId: "root-3",
        type: "announcement",
        createdAt: "2024-01-01T00:00:00Z",
        updatedAt: "2024-01-08T00:00:00Z",
        announcements: [
          {
            id: "a7",
            name: "部署選択",
            type: "ivr",
            enabled: true,
            textToSpeech: "お電話ありがとうございます。営業部は1を、サポート部は2を、その他のお問い合わせは3を押してください。",
            duration: 10,
            language: "ja-JP",
          },
        ],
      },
    ],
  },
  {
    id: "root-4",
    name: "時間外案内",
    description: "営業時間外のアナウンス",
    parentId: null,
    type: "folder",
    createdAt: "2024-01-01T00:00:00Z",
    updatedAt: "2024-01-28T00:00:00Z",
    children: [
      {
        id: "ann-6",
        name: "休業案内",
        description: "休業日・時間外の案内",
        parentId: "root-4",
        type: "announcement",
        createdAt: "2024-01-01T00:00:00Z",
        updatedAt: "2024-01-10T00:00:00Z",
        announcements: [
          {
            id: "a8",
            name: "営業時間外",
            type: "closed",
            enabled: true,
            textToSpeech: "お電話ありがとうございます。本日の営業は終了いたしました。営業時間は平日9時から18時までとなっております。",
            duration: 8,
            language: "ja-JP",
          },
          {
            id: "a9",
            name: "祝日休業",
            type: "closed",
            enabled: true,
            textToSpeech: "お電話ありがとうございます。本日は祝日のため休業とさせていただいております。",
            duration: 6,
            language: "ja-JP",
          },
        ],
      },
    ],
  },
]

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
    icon: Megaphone,
    color: "bg-purple-500/10 text-purple-600",
  },
  closed: {
    label: "時間外",
    icon: PhoneOff,
    color: "bg-red-500/10 text-red-600",
  },
  custom: {
    label: "カスタム",
    icon: FileAudio,
    color: "bg-amber-500/10 text-amber-600",
  },
}

interface TreeItemProps {
  item: AnnouncementFolder
  level: number
  isExpanded: boolean
  isSelected: boolean
  onToggle: () => void
  onSelect: () => void
}

function TreeItem({
  item,
  level,
  isExpanded,
  isSelected,
  onToggle,
  onSelect,
}: TreeItemProps) {
  const hasChildren = item.children && item.children.length > 0
  const isFolder = item.type === "folder"

  return (
    <ContextMenu>
      <ContextMenuTrigger>
        <div
          className={cn(
            "group flex items-center gap-1 px-2 py-1.5 rounded-md cursor-pointer transition-colors",
            isSelected
              ? "bg-primary/10 text-primary"
              : "hover:bg-muted text-foreground"
          )}
          style={{ paddingLeft: `${level * 16 + 8}px` }}
          onClick={onSelect}
        >
          {hasChildren ? (
            <button
              type="button"
              className="p-0.5 hover:bg-muted-foreground/10 rounded"
              onClick={(e) => {
                e.stopPropagation()
                onToggle()
              }}
            >
              {isExpanded ? (
                <ChevronDown className="h-4 w-4 text-muted-foreground" />
              ) : (
                <ChevronRight className="h-4 w-4 text-muted-foreground" />
              )}
            </button>
          ) : (
            <span className="w-5" />
          )}

          {isFolder ? (
            isExpanded ? (
              <FolderOpen className="h-4 w-4 text-amber-500 shrink-0" />
            ) : (
              <Folder className="h-4 w-4 text-amber-500 shrink-0" />
            )
          ) : (
            <Volume2 className="h-4 w-4 text-primary shrink-0" />
          )}

          <span className="truncate text-sm flex-1">{item.name}</span>

          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button
                variant="ghost"
                size="icon"
                className="h-6 w-6 opacity-0 group-hover:opacity-100"
                onClick={(e) => e.stopPropagation()}
              >
                <MoreHorizontal className="h-4 w-4" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem>
                <Edit className="h-4 w-4 mr-2" />
                編集
              </DropdownMenuItem>
              <DropdownMenuItem>
                <Copy className="h-4 w-4 mr-2" />
                複製
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem className="text-destructive">
                <Trash2 className="h-4 w-4 mr-2" />
                削除
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </ContextMenuTrigger>
      <ContextMenuContent>
        <ContextMenuItem>
          <Edit className="h-4 w-4 mr-2" />
          編集
        </ContextMenuItem>
        <ContextMenuItem>
          <Copy className="h-4 w-4 mr-2" />
          複製
        </ContextMenuItem>
        {isFolder && (
          <>
            <ContextMenuSeparator />
            <ContextMenuItem>
              <Plus className="h-4 w-4 mr-2" />
              新規アナウンス追加
            </ContextMenuItem>
            <ContextMenuItem>
              <Folder className="h-4 w-4 mr-2" />
              サブフォルダ追加
            </ContextMenuItem>
          </>
        )}
        <ContextMenuSeparator />
        <ContextMenuItem className="text-destructive">
          <Trash2 className="h-4 w-4 mr-2" />
          削除
        </ContextMenuItem>
      </ContextMenuContent>
    </ContextMenu>
  )
}

function AudioPreview({
  announcement,
}: {
  announcement: Announcement
}) {
  const [isPlaying, setIsPlaying] = useState(false)

  const formatDuration = (seconds: number) => {
    const mins = Math.floor(seconds / 60)
    const secs = seconds % 60
    return `${mins}:${secs.toString().padStart(2, "0")}`
  }

  return (
    <div className="flex items-center gap-3 p-3 bg-muted/50 rounded-lg">
      <Button
        variant="outline"
        size="icon"
        className="h-10 w-10 rounded-full shrink-0 bg-transparent"
        onClick={() => setIsPlaying(!isPlaying)}
      >
        {isPlaying ? (
          <Pause className="h-4 w-4" />
        ) : (
          <Play className="h-4 w-4 ml-0.5" />
        )}
      </Button>
      <div className="flex-1 min-w-0">
        <div className="h-8 bg-muted rounded flex items-center px-2">
          <div className="flex-1 flex items-center gap-0.5">
            {Array.from({ length: 40 }).map((_, i) => (
              <div
                key={i}
                className="w-1 bg-primary/40 rounded-full"
                style={{
                  height: `${Math.random() * 20 + 4}px`,
                }}
              />
            ))}
          </div>
        </div>
      </div>
      <span className="text-sm text-muted-foreground shrink-0">
        {announcement.duration ? formatDuration(announcement.duration) : "--:--"}
      </span>
    </div>
  )
}

export function AnnouncementsContent() {
  const [searchQuery, setSearchQuery] = useState("")
  const [selectedItem, setSelectedItem] = useState<AnnouncementFolder | null>(null)
  const [expandedIds, setExpandedIds] = useState<Set<string>>(
    new Set(["root-1"])
  )

  const toggleExpand = (id: string) => {
    setExpandedIds((prev) => {
      const next = new Set(prev)
      if (next.has(id)) {
        next.delete(id)
      } else {
        next.add(id)
      }
      return next
    })
  }

  const renderTree = (items: AnnouncementFolder[], level = 0) => {
    return items.map((item) => (
      <div key={item.id}>
        <TreeItem
          item={item}
          level={level}
          isExpanded={expandedIds.has(item.id)}
          isSelected={selectedItem?.id === item.id}
          onToggle={() => toggleExpand(item.id)}
          onSelect={() => setSelectedItem(item)}
        />
        {expandedIds.has(item.id) && item.children && (
          <div>{renderTree(item.children, level + 1)}</div>
        )}
      </div>
    ))
  }

  return (
    <div className="flex h-full">
      {/* Left Panel - Tree */}
      <div className="w-80 border-r flex flex-col bg-card">
        <div className="p-4 border-b">
          <div className="flex items-center justify-between mb-4">
            <h2 className="font-semibold text-lg">アナウンス</h2>
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button size="sm">
                  <Plus className="h-4 w-4 mr-1" />
                  追加
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                <DropdownMenuItem>
                  <Folder className="h-4 w-4 mr-2" />
                  新規フォルダ
                </DropdownMenuItem>
                <DropdownMenuSeparator />
                <DropdownMenuItem>
                  <Upload className="h-4 w-4 mr-2" />
                  音声ファイルをアップロード
                </DropdownMenuItem>
                <DropdownMenuItem>
                  <Mic className="h-4 w-4 mr-2" />
                  録音する
                </DropdownMenuItem>
                <DropdownMenuItem>
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
              onChange={(e) => setSearchQuery(e.target.value)}
              className="pl-9"
            />
          </div>
        </div>

        <ScrollArea className="flex-1">
          <div className="p-2">{renderTree(mockAnnouncementTree)}</div>
        </ScrollArea>
      </div>

      {/* Right Panel - Detail */}
      <div className="flex-1 overflow-auto bg-background">
        {selectedItem ? (
          <div className="p-6">
            <div className="flex items-start justify-between mb-6">
              <div className="flex items-center gap-3">
                {selectedItem.type === "folder" ? (
                  <div className="p-2 bg-amber-500/10 rounded-lg">
                    <Folder className="h-6 w-6 text-amber-500" />
                  </div>
                ) : (
                  <div className="p-2 bg-primary/10 rounded-lg">
                    <Volume2 className="h-6 w-6 text-primary" />
                  </div>
                )}
                <div>
                  <h1 className="text-2xl font-bold">{selectedItem.name}</h1>
                  {selectedItem.description && (
                    <p className="text-muted-foreground">
                      {selectedItem.description}
                    </p>
                  )}
                </div>
              </div>
              <div className="flex items-center gap-2">
                <Button variant="outline" size="sm">
                  <Edit className="h-4 w-4 mr-1" />
                  編集
                </Button>
                <Button variant="outline" size="sm">
                  <Copy className="h-4 w-4 mr-1" />
                  複製
                </Button>
              </div>
            </div>

            {selectedItem.type === "folder" ? (
              <Card>
                <CardHeader>
                  <CardTitle className="text-base">フォルダ内容</CardTitle>
                </CardHeader>
                <CardContent>
                  {selectedItem.children && selectedItem.children.length > 0 ? (
                    <div className="space-y-2">
                      {selectedItem.children.map((child) => (
                        <div
                          key={child.id}
                          className="flex items-center gap-3 p-3 rounded-lg border hover:bg-muted/50 cursor-pointer transition-colors"
                          onClick={() => {
                            setSelectedItem(child)
                            setExpandedIds((prev) => {
                              const next = new Set(prev)
                              next.add(selectedItem.id)
                              return next
                            })
                          }}
                        >
                          {child.type === "folder" ? (
                            <Folder className="h-5 w-5 text-amber-500" />
                          ) : (
                            <Volume2 className="h-5 w-5 text-primary" />
                          )}
                          <div className="flex-1">
                            <p className="font-medium">{child.name}</p>
                            {child.description && (
                              <p className="text-sm text-muted-foreground">
                                {child.description}
                              </p>
                            )}
                          </div>
                          <ChevronRight className="h-4 w-4 text-muted-foreground" />
                        </div>
                      ))}
                    </div>
                  ) : (
                    <p className="text-muted-foreground text-center py-8">
                      このフォルダは空です
                    </p>
                  )}
                </CardContent>
              </Card>
            ) : (
              <div className="space-y-4">
                <Card>
                  <CardHeader className="flex flex-row items-center justify-between">
                    <CardTitle className="text-base">アナウンス一覧</CardTitle>
                    <DropdownMenu>
                      <DropdownMenuTrigger asChild>
                        <Button size="sm">
                          <Plus className="h-4 w-4 mr-1" />
                          追加
                        </Button>
                      </DropdownMenuTrigger>
                      <DropdownMenuContent align="end">
                        <DropdownMenuItem>
                          <Upload className="h-4 w-4 mr-2" />
                          音声ファイルをアップロード
                        </DropdownMenuItem>
                        <DropdownMenuItem>
                          <Mic className="h-4 w-4 mr-2" />
                          録音する
                        </DropdownMenuItem>
                        <DropdownMenuItem>
                          <MessageSquare className="h-4 w-4 mr-2" />
                          テキスト読み上げ
                        </DropdownMenuItem>
                      </DropdownMenuContent>
                    </DropdownMenu>
                  </CardHeader>
                  <CardContent>
                    {selectedItem.announcements &&
                    selectedItem.announcements.length > 0 ? (
                      <div className="space-y-4">
                        {selectedItem.announcements.map((announcement) => {
                          const config = announcementTypeConfig[announcement.type]
                          const TypeIcon = config.icon
                          return (
                            <div
                              key={announcement.id}
                              className="p-4 rounded-lg border"
                            >
                              <div className="flex items-start justify-between mb-3">
                                <div className="flex items-center gap-3">
                                  <div className={cn("p-2 rounded-lg", config.color)}>
                                    <TypeIcon className="h-4 w-4" />
                                  </div>
                                  <div>
                                    <div className="flex items-center gap-2">
                                      <p className="font-medium">
                                        {announcement.name}
                                      </p>
                                      <Badge
                                        variant="outline"
                                        className={config.color}
                                      >
                                        {config.label}
                                      </Badge>
                                      <Badge variant="secondary">
                                        {announcement.language}
                                      </Badge>
                                    </div>
                                    {announcement.description && (
                                      <p className="text-sm text-muted-foreground">
                                        {announcement.description}
                                      </p>
                                    )}
                                  </div>
                                </div>
                                <div className="flex items-center gap-2">
                                  <Switch checked={announcement.enabled} />
                                  <DropdownMenu>
                                    <DropdownMenuTrigger asChild>
                                      <Button variant="ghost" size="icon">
                                        <MoreHorizontal className="h-4 w-4" />
                                      </Button>
                                    </DropdownMenuTrigger>
                                    <DropdownMenuContent align="end">
                                      <DropdownMenuItem>
                                        <Edit className="h-4 w-4 mr-2" />
                                        編集
                                      </DropdownMenuItem>
                                      <DropdownMenuItem>
                                        <Copy className="h-4 w-4 mr-2" />
                                        複製
                                      </DropdownMenuItem>
                                      <DropdownMenuSeparator />
                                      <DropdownMenuItem className="text-destructive">
                                        <Trash2 className="h-4 w-4 mr-2" />
                                        削除
                                      </DropdownMenuItem>
                                    </DropdownMenuContent>
                                  </DropdownMenu>
                                </div>
                              </div>

                              <AudioPreview announcement={announcement} />

                              {announcement.textToSpeech && (
                                <div className="mt-3 p-3 bg-muted/30 rounded-lg">
                                  <p className="text-xs text-muted-foreground mb-1">
                                    テキスト読み上げ
                                  </p>
                                  <p className="text-sm">
                                    {announcement.textToSpeech}
                                  </p>
                                </div>
                              )}

                              {announcement.audioUrl && (
                                <div className="mt-3 flex items-center gap-2 text-sm text-muted-foreground">
                                  <FileAudio className="h-4 w-4" />
                                  <span>{announcement.audioUrl}</span>
                                </div>
                              )}
                            </div>
                          )
                        })}
                      </div>
                    ) : (
                      <p className="text-muted-foreground text-center py-8">
                        アナウンスがありません
                      </p>
                    )}
                  </CardContent>
                </Card>
              </div>
            )}
          </div>
        ) : (
          <div className="flex items-center justify-center h-full text-muted-foreground">
            <div className="text-center">
              <Volume2 className="h-12 w-12 mx-auto mb-4 opacity-50" />
              <p>左側からアナウンスを選択してください</p>
            </div>
          </div>
        )}
      </div>
    </div>
  )
}
