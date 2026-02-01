"use client"

import { useState } from "react"
import { cn } from "@/lib/utils"
import type { RoutingFolder, RoutingRule } from "@/lib/types"
import {
  Folder,
  FolderOpen,
  ChevronRight,
  ChevronDown,
  Plus,
  MoreHorizontal,
  Search,
  Edit2,
  Trash2,
  Copy,
  FolderPlus,
  Route,
  Clock,
  Phone,
  GitBranch,
  Users,
  Voicemail,
  ArrowRight,
  Power,
  GripVertical,
} from "lucide-react"
import { Button } from "./ui/button"
import { Input } from "./ui/input"
import { Badge } from "./ui/badge"
import { Card, CardContent } from "./ui/card"
import { Switch } from "./ui/switch"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "./ui/dropdown-menu"
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger,
} from "./ui/context-menu"
import { ScrollArea } from "./ui/scroll-area"

// Mock data for routing
const mockRouting: RoutingFolder[] = [
  {
    id: "root-1",
    name: "営業時間ルーティング",
    description: "営業時間に基づくルーティング設定",
    parentId: null,
    type: "folder",
    createdAt: "2024-01-01T00:00:00Z",
    updatedAt: "2024-01-15T00:00:00Z",
    children: [
      {
        id: "route-1-1",
        name: "平日営業時間",
        description: "平日9:00-18:00の着信ルール",
        parentId: "root-1",
        type: "route",
        createdAt: "2024-01-01T00:00:00Z",
        updatedAt: "2024-01-15T00:00:00Z",
        rules: [
          {
            id: "rule-1",
            name: "営業時間内",
            description: "営業時間内は営業部に転送",
            type: "time",
            enabled: true,
            priority: 1,
            conditions: {
              timeRange: { start: "09:00", end: "18:00" },
              daysOfWeek: [1, 2, 3, 4, 5],
            },
            destination: { type: "group", target: "営業部" },
          },
          {
            id: "rule-2",
            name: "営業時間外",
            description: "営業時間外はボイスメールへ",
            type: "time",
            enabled: true,
            priority: 2,
            conditions: {
              timeRange: { start: "18:00", end: "09:00" },
            },
            destination: { type: "voicemail", target: "営業部VM" },
          },
        ],
      },
      {
        id: "route-1-2",
        name: "休日対応",
        description: "土日祝日の着信ルール",
        parentId: "root-1",
        type: "route",
        createdAt: "2024-01-02T00:00:00Z",
        updatedAt: "2024-01-16T00:00:00Z",
        rules: [
          {
            id: "rule-3",
            name: "休日転送",
            description: "休日は緊急連絡先へ",
            type: "time",
            enabled: true,
            priority: 1,
            conditions: {
              daysOfWeek: [0, 6],
            },
            destination: { type: "number", target: "090-1234-5678" },
          },
        ],
      },
    ],
  },
  {
    id: "root-2",
    name: "発信者ベース",
    description: "発信者番号に基づくルーティング",
    parentId: null,
    type: "folder",
    createdAt: "2024-01-05T00:00:00Z",
    updatedAt: "2024-01-20T00:00:00Z",
    children: [
      {
        id: "route-2-1",
        name: "VIP顧客",
        description: "VIP顧客専用ルーティング",
        parentId: "root-2",
        type: "route",
        createdAt: "2024-01-05T00:00:00Z",
        updatedAt: "2024-01-20T00:00:00Z",
        rules: [
          {
            id: "rule-4",
            name: "VIP直通",
            description: "VIP顧客は専任担当へ",
            type: "caller",
            enabled: true,
            priority: 1,
            conditions: {
              callerPatterns: ["03-1234-*", "080-*-1234"],
            },
            destination: { type: "number", target: "03-9999-0001" },
          },
        ],
      },
      {
        id: "folder-2-2",
        name: "地域別",
        description: "地域別ルーティング",
        parentId: "root-2",
        type: "folder",
        createdAt: "2024-01-06T00:00:00Z",
        updatedAt: "2024-01-21T00:00:00Z",
        children: [
          {
            id: "route-2-2-1",
            name: "関東エリア",
            parentId: "folder-2-2",
            type: "route",
            createdAt: "2024-01-06T00:00:00Z",
            updatedAt: "2024-01-21T00:00:00Z",
            rules: [
              {
                id: "rule-5",
                name: "東京",
                type: "caller",
                enabled: true,
                priority: 1,
                conditions: { callerPatterns: ["03-*"] },
                destination: { type: "group", target: "東京営業" },
              },
              {
                id: "rule-6",
                name: "神奈川",
                type: "caller",
                enabled: true,
                priority: 2,
                conditions: { callerPatterns: ["045-*", "044-*"] },
                destination: { type: "group", target: "神奈川営業" },
              },
            ],
          },
          {
            id: "route-2-2-2",
            name: "関西エリア",
            parentId: "folder-2-2",
            type: "route",
            createdAt: "2024-01-06T00:00:00Z",
            updatedAt: "2024-01-21T00:00:00Z",
            rules: [
              {
                id: "rule-7",
                name: "大阪",
                type: "caller",
                enabled: true,
                priority: 1,
                conditions: { callerPatterns: ["06-*"] },
                destination: { type: "group", target: "大阪営業" },
              },
            ],
          },
        ],
      },
    ],
  },
  {
    id: "root-3",
    name: "IVRメニュー",
    description: "IVR自動応答設定",
    parentId: null,
    type: "route",
    createdAt: "2024-01-10T00:00:00Z",
    updatedAt: "2024-01-25T00:00:00Z",
    rules: [
      {
        id: "rule-8",
        name: "メインメニュー",
        description: "1:営業, 2:サポート, 3:その他",
        type: "ivr",
        enabled: true,
        priority: 1,
        destination: { type: "ivr", target: "main-menu" },
      },
      {
        id: "rule-9",
        name: "オーバーフロー",
        description: "待機30秒超過時",
        type: "overflow",
        enabled: false,
        priority: 2,
        destination: { type: "voicemail", target: "general-vm" },
      },
    ],
  },
]

interface TreeNodeProps {
  node: RoutingFolder
  level: number
  selectedId: string | null
  expandedIds: Set<string>
  onSelect: (node: RoutingFolder) => void
  onToggle: (id: string) => void
}

function TreeNode({ node, level, selectedId, expandedIds, onSelect, onToggle }: TreeNodeProps) {
  const isExpanded = expandedIds.has(node.id)
  const isSelected = selectedId === node.id
  const hasChildren = node.children && node.children.length > 0
  const isFolder = node.type === "folder"

  const handleClick = () => {
    onSelect(node)
    if (hasChildren) {
      onToggle(node.id)
    }
  }

  const countRules = (folder: RoutingFolder): number => {
    let count = folder.rules?.length || 0
    if (folder.children) {
      for (const child of folder.children) {
        count += countRules(child)
      }
    }
    return count
  }

  const ruleCount = countRules(node)

  return (
    <ContextMenu>
      <ContextMenuTrigger>
        <div>
          <div
            className={cn(
              "flex items-center gap-2 px-2 py-1.5 rounded-md cursor-pointer transition-colors group",
              "hover:bg-accent",
              isSelected && "bg-accent text-accent-foreground"
            )}
            style={{ paddingLeft: `${level * 16 + 8}px` }}
            onClick={handleClick}
          >
            <span className="w-4 h-4 flex items-center justify-center shrink-0">
              {hasChildren ? (
                isExpanded ? (
                  <ChevronDown className="h-4 w-4 text-muted-foreground" />
                ) : (
                  <ChevronRight className="h-4 w-4 text-muted-foreground" />
                )
              ) : null}
            </span>

            {isFolder ? (
              isExpanded ? (
                <FolderOpen className="h-4 w-4 text-primary shrink-0" />
              ) : (
                <Folder className="h-4 w-4 text-primary shrink-0" />
              )
            ) : (
              <GitBranch className="h-4 w-4 text-muted-foreground shrink-0" />
            )}

            <span className="flex-1 truncate text-sm">{node.name}</span>

            {ruleCount > 0 && (
              <Badge variant="secondary" className="text-xs h-5 px-1.5">
                {ruleCount}
              </Badge>
            )}

            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-6 w-6 opacity-0 group-hover:opacity-100 transition-opacity"
                  onClick={(e) => e.stopPropagation()}
                >
                  <MoreHorizontal className="h-4 w-4" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                <DropdownMenuItem>
                  <Edit2 className="h-4 w-4 mr-2" />
                  編集
                </DropdownMenuItem>
                <DropdownMenuItem>
                  <Copy className="h-4 w-4 mr-2" />
                  複製
                </DropdownMenuItem>
                {isFolder && (
                  <>
                    <DropdownMenuSeparator />
                    <DropdownMenuItem>
                      <FolderPlus className="h-4 w-4 mr-2" />
                      フォルダを追加
                    </DropdownMenuItem>
                    <DropdownMenuItem>
                      <Route className="h-4 w-4 mr-2" />
                      ルートを追加
                    </DropdownMenuItem>
                  </>
                )}
                <DropdownMenuSeparator />
                <DropdownMenuItem className="text-destructive">
                  <Trash2 className="h-4 w-4 mr-2" />
                  削除
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </div>

          {hasChildren && isExpanded && (
            <div>
              {node.children!.map((child) => (
                <TreeNode
                  key={child.id}
                  node={child}
                  level={level + 1}
                  selectedId={selectedId}
                  expandedIds={expandedIds}
                  onSelect={onSelect}
                  onToggle={onToggle}
                />
              ))}
            </div>
          )}
        </div>
      </ContextMenuTrigger>
      <ContextMenuContent>
        <ContextMenuItem>
          <Edit2 className="h-4 w-4 mr-2" />
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
              <FolderPlus className="h-4 w-4 mr-2" />
              フォルダを追加
            </ContextMenuItem>
            <ContextMenuItem>
              <Route className="h-4 w-4 mr-2" />
              ルートを追加
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

function getRuleTypeIcon(type: RoutingRule["type"]) {
  switch (type) {
    case "time":
      return <Clock className="h-4 w-4" />
    case "caller":
      return <Phone className="h-4 w-4" />
    case "ivr":
      return <GitBranch className="h-4 w-4" />
    case "overflow":
      return <Users className="h-4 w-4" />
    default:
      return <Route className="h-4 w-4" />
  }
}

function getRuleTypeLabel(type: RoutingRule["type"]) {
  switch (type) {
    case "time":
      return "時間帯"
    case "caller":
      return "発信者"
    case "ivr":
      return "IVR"
    case "overflow":
      return "オーバーフロー"
    default:
      return "デフォルト"
  }
}

function getDestinationIcon(type: RoutingRule["destination"]["type"]) {
  switch (type) {
    case "group":
      return <Users className="h-4 w-4" />
    case "number":
      return <Phone className="h-4 w-4" />
    case "voicemail":
      return <Voicemail className="h-4 w-4" />
    case "ivr":
      return <GitBranch className="h-4 w-4" />
    default:
      return <ArrowRight className="h-4 w-4" />
  }
}

interface RoutingDetailProps {
  folder: RoutingFolder | null
}

function RoutingDetail({ folder }: RoutingDetailProps) {
  const [rules, setRules] = useState<RoutingRule[]>([])

  // Update rules when folder changes
  useState(() => {
    if (folder?.rules) {
      setRules(folder.rules)
    }
  })

  if (!folder) {
    return (
      <div className="flex-1 flex items-center justify-center text-muted-foreground">
        <div className="text-center">
          <GitBranch className="h-12 w-12 mx-auto mb-4 opacity-50" />
          <p>ルートを選択してください</p>
        </div>
      </div>
    )
  }

  const isFolder = folder.type === "folder"
  const displayRules = folder.rules || rules

  const handleToggleRule = (ruleId: string) => {
    // In real app, this would update via API
    console.log("Toggle rule:", ruleId)
  }

  return (
    <div className="flex-1 flex flex-col">
      <div className="p-4 border-b">
        <div className="flex items-start justify-between">
          <div className="flex items-center gap-3">
            {isFolder ? (
              <div className="w-10 h-10 rounded-lg bg-primary/10 flex items-center justify-center">
                <Folder className="h-5 w-5 text-primary" />
              </div>
            ) : (
              <div className="w-10 h-10 rounded-lg bg-muted flex items-center justify-center">
                <GitBranch className="h-5 w-5 text-muted-foreground" />
              </div>
            )}
            <div>
              <h2 className="text-lg font-semibold">{folder.name}</h2>
              {folder.description && (
                <p className="text-sm text-muted-foreground">{folder.description}</p>
              )}
            </div>
          </div>
          <Button variant="outline" size="sm">
            <Edit2 className="h-4 w-4 mr-2" />
            編集
          </Button>
        </div>
      </div>

      <ScrollArea className="flex-1">
        <div className="p-4">
          {isFolder ? (
            <div className="space-y-4">
              <p className="text-muted-foreground text-sm">
                このフォルダには {folder.children?.length || 0} 個のサブアイテムがあります。
              </p>
              {folder.children && folder.children.length > 0 && (
                <div className="grid gap-3">
                  {folder.children.map((child) => (
                    <Card key={child.id} className="hover:bg-accent/50 transition-colors cursor-pointer">
                      <CardContent className="p-4 flex items-center gap-3">
                        {child.type === "folder" ? (
                          <Folder className="h-5 w-5 text-primary shrink-0" />
                        ) : (
                          <GitBranch className="h-5 w-5 text-muted-foreground shrink-0" />
                        )}
                        <div className="flex-1 min-w-0">
                          <p className="font-medium truncate">{child.name}</p>
                          {child.description && (
                            <p className="text-sm text-muted-foreground truncate">{child.description}</p>
                          )}
                        </div>
                        <ChevronRight className="h-4 w-4 text-muted-foreground shrink-0" />
                      </CardContent>
                    </Card>
                  ))}
                </div>
              )}
            </div>
          ) : (
            <div className="space-y-4">
              <div className="flex items-center justify-between">
                <p className="text-sm text-muted-foreground">
                  {displayRules.length} 件のルール
                </p>
                <Button size="sm">
                  <Plus className="h-4 w-4 mr-2" />
                  ルールを追加
                </Button>
              </div>

              {displayRules.length > 0 ? (
                <div className="space-y-2">
                  {displayRules.map((rule, index) => (
                    <RuleRow
                      key={rule.id}
                      rule={rule}
                      index={index}
                      onToggle={() => handleToggleRule(rule.id)}
                    />
                  ))}
                </div>
              ) : (
                <Card className="border-dashed">
                  <CardContent className="p-6 text-center">
                    <Route className="h-8 w-8 mx-auto mb-3 text-muted-foreground opacity-50" />
                    <p className="text-muted-foreground">ルールがありません</p>
                    <Button variant="outline" size="sm" className="mt-4 bg-transparent">
                      <Plus className="h-4 w-4 mr-2" />
                      ルールを追加
                    </Button>
                  </CardContent>
                </Card>
              )}
            </div>
          )}
        </div>
      </ScrollArea>
    </div>
  )
}

function RuleRow({ rule, index, onToggle }: { rule: RoutingRule; index: number; onToggle: () => void }) {
  return (
    <div
      className={cn(
        "flex items-center gap-3 p-3 rounded-lg border bg-card transition-colors group",
        rule.enabled ? "hover:bg-accent/50" : "opacity-60"
      )}
    >
      <div className="cursor-grab text-muted-foreground hover:text-foreground">
        <GripVertical className="h-4 w-4" />
      </div>

      <div className="w-6 h-6 rounded-full bg-muted flex items-center justify-center text-xs font-medium shrink-0">
        {index + 1}
      </div>

      <div
        className={cn(
          "w-8 h-8 rounded-lg flex items-center justify-center shrink-0",
          rule.type === "time" && "bg-blue-500/10 text-blue-600",
          rule.type === "caller" && "bg-green-500/10 text-green-600",
          rule.type === "ivr" && "bg-purple-500/10 text-purple-600",
          rule.type === "overflow" && "bg-orange-500/10 text-orange-600",
          rule.type === "default" && "bg-gray-500/10 text-gray-600"
        )}
      >
        {getRuleTypeIcon(rule.type)}
      </div>

      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <span className="font-medium truncate">{rule.name}</span>
          <Badge variant="outline" className="text-xs shrink-0">
            {getRuleTypeLabel(rule.type)}
          </Badge>
        </div>
        {rule.description && (
          <p className="text-sm text-muted-foreground truncate">{rule.description}</p>
        )}
        {rule.conditions && (
          <div className="flex items-center gap-2 mt-1 text-xs text-muted-foreground">
            {rule.conditions.timeRange && (
              <span>{rule.conditions.timeRange.start} - {rule.conditions.timeRange.end}</span>
            )}
            {rule.conditions.daysOfWeek && (
              <span>
                {rule.conditions.daysOfWeek.map(d => ["日", "月", "火", "水", "木", "金", "土"][d]).join(", ")}
              </span>
            )}
            {rule.conditions.callerPatterns && (
              <span>{rule.conditions.callerPatterns.join(", ")}</span>
            )}
          </div>
        )}
      </div>

      <div className="flex items-center gap-2 text-sm text-muted-foreground shrink-0">
        <ArrowRight className="h-4 w-4" />
        <div className="flex items-center gap-1.5">
          {getDestinationIcon(rule.destination.type)}
          <span className="max-w-24 truncate">{rule.destination.target}</span>
        </div>
      </div>

      <Switch checked={rule.enabled} onCheckedChange={onToggle} />

      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button
            variant="ghost"
            size="icon"
            className="h-8 w-8 opacity-0 group-hover:opacity-100 transition-opacity"
          >
            <MoreHorizontal className="h-4 w-4" />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
          <DropdownMenuItem>
            <Edit2 className="h-4 w-4 mr-2" />
            編集
          </DropdownMenuItem>
          <DropdownMenuItem>
            <Copy className="h-4 w-4 mr-2" />
            複製
          </DropdownMenuItem>
          <DropdownMenuItem>
            <Power className="h-4 w-4 mr-2" />
            {rule.enabled ? "無効にする" : "有効にする"}
          </DropdownMenuItem>
          <DropdownMenuSeparator />
          <DropdownMenuItem className="text-destructive">
            <Trash2 className="h-4 w-4 mr-2" />
            削除
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  )
}

export function RoutingContent() {
  const [searchQuery, setSearchQuery] = useState("")
  const [selectedFolder, setSelectedFolder] = useState<RoutingFolder | null>(null)
  const [expandedIds, setExpandedIds] = useState<Set<string>>(new Set(["root-1", "root-2"]))

  const handleToggle = (id: string) => {
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

  const filterFolders = (folders: RoutingFolder[], query: string): RoutingFolder[] => {
    if (!query) return folders

    return folders
      .map((folder) => {
        const matchesName = folder.name.toLowerCase().includes(query.toLowerCase())
        const matchesRule = folder.rules?.some((r) =>
          r.name.toLowerCase().includes(query.toLowerCase()) ||
          r.description?.toLowerCase().includes(query.toLowerCase())
        )
        const filteredChildren = folder.children ? filterFolders(folder.children, query) : []

        if (matchesName || matchesRule || filteredChildren.length > 0) {
          return {
            ...folder,
            children: filteredChildren.length > 0 ? filteredChildren : folder.children,
          }
        }
        return null
      })
      .filter((f): f is RoutingFolder => f !== null)
  }

  const filteredFolders = filterFolders(mockRouting, searchQuery)

  return (
    <div className="flex flex-col h-full">
      <div className="p-6 border-b">
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-2xl font-bold text-balance">ルーティング</h1>
            <p className="text-muted-foreground">通話のルーティングルールを設定</p>
          </div>
          <div className="flex items-center gap-2">
            <Button variant="outline">
              <FolderPlus className="h-4 w-4 mr-2" />
              フォルダ作成
            </Button>
            <Button>
              <Plus className="h-4 w-4 mr-2" />
              ルート作成
            </Button>
          </div>
        </div>
      </div>

      <div className="flex flex-1 min-h-0">
        <div className="w-80 border-r flex flex-col bg-card">
          <div className="p-3 border-b">
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
            <div className="p-2">
              {filteredFolders.map((folder) => (
                <TreeNode
                  key={folder.id}
                  node={folder}
                  level={0}
                  selectedId={selectedFolder?.id || null}
                  expandedIds={expandedIds}
                  onSelect={setSelectedFolder}
                  onToggle={handleToggle}
                />
              ))}

              {filteredFolders.length === 0 && (
                <div className="p-4 text-center text-muted-foreground text-sm">
                  該当するルートがありません
                </div>
              )}
            </div>
          </ScrollArea>
        </div>

        <RoutingDetail folder={selectedFolder} />
      </div>
    </div>
  )
}
