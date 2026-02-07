"use client"

import { useState } from "react"
import {
  Calendar,
  ChevronDown,
  ChevronRight,
  Clock,
  Copy,
  Edit,
  Folder,
  FolderOpen,
  MoreHorizontal,
  Plus,
  Search,
  Trash2,
  CalendarDays,
  CalendarOff,
  CalendarClock,
  CalendarCheck,
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
import type {
  LegacyScheduleFolder as ScheduleFolder,
  LegacySchedule as Schedule,
  ScheduleType,
} from "@/lib/types"

// Mock data
const mockScheduleTree: ScheduleFolder[] = [
  {
    id: "root-1",
    name: "営業時間",
    description: "通常営業時間のスケジュール",
    parentId: null,
    type: "folder",
    createdAt: "2024-01-01T00:00:00Z",
    updatedAt: "2024-01-15T00:00:00Z",
    children: [
      {
        id: "schedule-1",
        name: "平日営業",
        description: "月曜〜金曜の営業時間",
        parentId: "root-1",
        type: "schedule",
        createdAt: "2024-01-01T00:00:00Z",
        updatedAt: "2024-01-10T00:00:00Z",
        schedules: [
          {
            id: "s1",
            name: "午前営業",
            description: "午前の受付時間",
            type: "business",
            enabled: true,
            daysOfWeek: [1, 2, 3, 4, 5],
            timeSlots: [{ start: "09:00", end: "12:00" }],
            action: { type: "route", target: "main-queue" },
          },
          {
            id: "s2",
            name: "午後営業",
            description: "午後の受付時間",
            type: "business",
            enabled: true,
            daysOfWeek: [1, 2, 3, 4, 5],
            timeSlots: [{ start: "13:00", end: "18:00" }],
            action: { type: "route", target: "main-queue" },
          },
        ],
      },
      {
        id: "schedule-2",
        name: "土曜営業",
        description: "土曜日の短縮営業",
        parentId: "root-1",
        type: "schedule",
        createdAt: "2024-01-02T00:00:00Z",
        updatedAt: "2024-01-12T00:00:00Z",
        schedules: [
          {
            id: "s3",
            name: "土曜受付",
            type: "business",
            enabled: true,
            daysOfWeek: [6],
            timeSlots: [{ start: "10:00", end: "15:00" }],
            action: { type: "route", target: "weekend-queue" },
          },
        ],
      },
    ],
  },
  {
    id: "root-2",
    name: "休日設定",
    description: "祝日・特別休日の設定",
    parentId: null,
    type: "folder",
    createdAt: "2024-01-01T00:00:00Z",
    updatedAt: "2024-01-20T00:00:00Z",
    children: [
      {
        id: "schedule-3",
        name: "2024年祝日",
        description: "2024年の祝日カレンダー",
        parentId: "root-2",
        type: "schedule",
        createdAt: "2024-01-01T00:00:00Z",
        updatedAt: "2024-01-05T00:00:00Z",
        schedules: [
          {
            id: "s4",
            name: "元日",
            type: "holiday",
            enabled: true,
            dateRange: { start: "2024-01-01", end: "2024-01-01" },
            timeSlots: [{ start: "00:00", end: "23:59" }],
            action: { type: "announcement", target: "holiday-msg" },
          },
          {
            id: "s5",
            name: "成人の日",
            type: "holiday",
            enabled: true,
            dateRange: { start: "2024-01-08", end: "2024-01-08" },
            timeSlots: [{ start: "00:00", end: "23:59" }],
            action: { type: "announcement", target: "holiday-msg" },
          },
          {
            id: "s6",
            name: "年末年始休暇",
            type: "holiday",
            enabled: true,
            dateRange: { start: "2024-12-29", end: "2025-01-03" },
            timeSlots: [{ start: "00:00", end: "23:59" }],
            action: { type: "closed" },
          },
        ],
      },
      {
        id: "schedule-4",
        name: "特別休業日",
        description: "臨時休業の設定",
        parentId: "root-2",
        type: "schedule",
        createdAt: "2024-02-01T00:00:00Z",
        updatedAt: "2024-02-10T00:00:00Z",
        schedules: [
          {
            id: "s7",
            name: "社内研修日",
            type: "special",
            enabled: false,
            dateRange: { start: "2024-03-15", end: "2024-03-15" },
            timeSlots: [{ start: "00:00", end: "23:59" }],
            action: { type: "voicemail" },
          },
        ],
      },
    ],
  },
  {
    id: "root-3",
    name: "時間外設定",
    description: "営業時間外の対応",
    parentId: null,
    type: "folder",
    createdAt: "2024-01-01T00:00:00Z",
    updatedAt: "2024-01-25T00:00:00Z",
    children: [
      {
        id: "schedule-5",
        name: "夜間対応",
        description: "18時以降の対応設定",
        parentId: "root-3",
        type: "schedule",
        createdAt: "2024-01-01T00:00:00Z",
        updatedAt: "2024-01-08T00:00:00Z",
        schedules: [
          {
            id: "s8",
            name: "夜間転送",
            type: "override",
            enabled: true,
            daysOfWeek: [1, 2, 3, 4, 5],
            timeSlots: [{ start: "18:00", end: "23:59" }],
            action: { type: "voicemail" },
          },
        ],
      },
    ],
  },
]

const scheduleTypeConfig: Record<
  ScheduleType,
  { label: string; icon: typeof Calendar; color: string }
> = {
  business: {
    label: "営業時間",
    icon: CalendarCheck,
    color: "bg-green-500/10 text-green-600",
  },
  holiday: {
    label: "休日",
    icon: CalendarOff,
    color: "bg-red-500/10 text-red-600",
  },
  special: {
    label: "特別",
    icon: CalendarDays,
    color: "bg-amber-500/10 text-amber-600",
  },
  override: {
    label: "上書き",
    icon: CalendarClock,
    color: "bg-purple-500/10 text-purple-600",
  },
}

const daysOfWeekLabels = ["日", "月", "火", "水", "木", "金", "土"]

interface TreeItemProps {
  item: ScheduleFolder
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
            <Calendar className="h-4 w-4 text-primary shrink-0" />
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
              新規スケジュール追加
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

export function ScheduleContent() {
  const [searchQuery, setSearchQuery] = useState("")
  const [selectedItem, setSelectedItem] = useState<ScheduleFolder | null>(null)
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

  const renderTree = (items: ScheduleFolder[], level = 0) => {
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
            <h2 className="font-semibold text-lg">スケジュール</h2>
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
                <DropdownMenuItem>
                  <Calendar className="h-4 w-4 mr-2" />
                  新規スケジュール
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
          <div className="p-2">{renderTree(mockScheduleTree)}</div>
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
                    <Calendar className="h-6 w-6 text-primary" />
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
                            <Calendar className="h-5 w-5 text-primary" />
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
                    <CardTitle className="text-base">スケジュール一覧</CardTitle>
                    <Button size="sm">
                      <Plus className="h-4 w-4 mr-1" />
                      追加
                    </Button>
                  </CardHeader>
                  <CardContent>
                    {selectedItem.schedules &&
                    selectedItem.schedules.length > 0 ? (
                      <div className="space-y-3">
                        {selectedItem.schedules.map((schedule) => {
                          const config = scheduleTypeConfig[schedule.type]
                          const TypeIcon = config.icon
                          return (
                            <div
                              key={schedule.id}
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
                                        {schedule.name}
                                      </p>
                                      <Badge
                                        variant="outline"
                                        className={config.color}
                                      >
                                        {config.label}
                                      </Badge>
                                    </div>
                                    {schedule.description && (
                                      <p className="text-sm text-muted-foreground">
                                        {schedule.description}
                                      </p>
                                    )}
                                  </div>
                                </div>
                                <div className="flex items-center gap-2">
                                  <Switch checked={schedule.enabled} />
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

                              <div className="grid grid-cols-2 gap-4 text-sm">
                                {schedule.daysOfWeek && (
                                  <div>
                                    <p className="text-muted-foreground mb-1">
                                      曜日
                                    </p>
                                    <div className="flex gap-1">
                                      {[0, 1, 2, 3, 4, 5, 6].map((day) => (
                                        <span
                                          key={day}
                                          className={cn(
                                            "w-6 h-6 flex items-center justify-center rounded text-xs",
                                            schedule.daysOfWeek?.includes(day)
                                              ? "bg-primary text-primary-foreground"
                                              : "bg-muted text-muted-foreground"
                                          )}
                                        >
                                          {daysOfWeekLabels[day]}
                                        </span>
                                      ))}
                                    </div>
                                  </div>
                                )}
                                {schedule.dateRange && (
                                  <div>
                                    <p className="text-muted-foreground mb-1">
                                      期間
                                    </p>
                                    <p>
                                      {schedule.dateRange.start} 〜{" "}
                                      {schedule.dateRange.end}
                                    </p>
                                  </div>
                                )}
                                <div>
                                  <p className="text-muted-foreground mb-1">
                                    時間帯
                                  </p>
                                  <div className="flex flex-wrap gap-1">
                                    {schedule.timeSlots.map((slot, i) => (
                                      <Badge key={i} variant="secondary">
                                        <Clock className="h-3 w-3 mr-1" />
                                        {slot.start} - {slot.end}
                                      </Badge>
                                    ))}
                                  </div>
                                </div>
                                <div>
                                  <p className="text-muted-foreground mb-1">
                                    アクション
                                  </p>
                                  <Badge variant="outline">
                                    {schedule.action.type === "route" &&
                                      `転送: ${schedule.action.target}`}
                                    {schedule.action.type === "voicemail" &&
                                      "留守番電話"}
                                    {schedule.action.type === "announcement" &&
                                      `アナウンス: ${schedule.action.target}`}
                                    {schedule.action.type === "closed" &&
                                      "休業"}
                                  </Badge>
                                </div>
                              </div>
                            </div>
                          )
                        })}
                      </div>
                    ) : (
                      <p className="text-muted-foreground text-center py-8">
                        スケジュールがありません
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
              <Calendar className="h-12 w-12 mx-auto mb-4 opacity-50" />
              <p>左側からスケジュールを選択してください</p>
            </div>
          </div>
        )}
      </div>
    </div>
  )
}
