"use client"

import { useState } from "react"
import { cn } from "@/lib/utils"
import type { NumberGroup, PhoneNumber } from "@/lib/types"
import {
  Folder,
  FolderOpen,
  ChevronRight,
  ChevronDown,
  Phone,
  Plus,
  MoreHorizontal,
  Search,
  Users,
  Edit2,
  Trash2,
  Copy,
  FolderPlus,
  PhoneCall,
} from "lucide-react"
import { Button } from "./ui/button"
import { Input } from "./ui/input"
import { Badge } from "./ui/badge"
import { Card, CardContent, CardHeader, CardTitle } from "./ui/card"
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

// Mock data for number groups - tree structure
const mockGroups: NumberGroup[] = [
  {
    id: "root-1",
    name: "東京本社",
    description: "東京本社の電話番号グループ",
    parentId: null,
    type: "folder",
    createdAt: "2024-01-01T00:00:00Z",
    updatedAt: "2024-01-15T00:00:00Z",
    children: [
      {
        id: "group-1-1",
        name: "営業部",
        description: "営業部門の代表番号",
        parentId: "root-1",
        type: "group",
        createdAt: "2024-01-01T00:00:00Z",
        updatedAt: "2024-01-15T00:00:00Z",
        numbers: [
          { id: "num-1", number: "03-1234-5678", label: "代表", status: "active" },
          { id: "num-2", number: "03-1234-5679", label: "直通1", status: "active" },
          { id: "num-3", number: "03-1234-5680", label: "直通2", status: "inactive" },
        ],
      },
      {
        id: "group-1-2",
        name: "サポート部",
        description: "カスタマーサポート",
        parentId: "root-1",
        type: "group",
        createdAt: "2024-01-02T00:00:00Z",
        updatedAt: "2024-01-16T00:00:00Z",
        numbers: [
          { id: "num-4", number: "0120-123-456", label: "フリーダイヤル", status: "active" },
          { id: "num-5", number: "03-1234-5681", label: "有料番号", status: "active" },
        ],
      },
      {
        id: "folder-1-3",
        name: "開発部",
        description: "開発部門",
        parentId: "root-1",
        type: "folder",
        createdAt: "2024-01-03T00:00:00Z",
        updatedAt: "2024-01-17T00:00:00Z",
        children: [
          {
            id: "group-1-3-1",
            name: "フロントエンドチーム",
            parentId: "folder-1-3",
            type: "group",
            createdAt: "2024-01-03T00:00:00Z",
            updatedAt: "2024-01-17T00:00:00Z",
            numbers: [
              { id: "num-6", number: "03-1234-5682", label: "チーム代表", status: "active" },
            ],
          },
          {
            id: "group-1-3-2",
            name: "バックエンドチーム",
            parentId: "folder-1-3",
            type: "group",
            createdAt: "2024-01-03T00:00:00Z",
            updatedAt: "2024-01-17T00:00:00Z",
            numbers: [
              { id: "num-7", number: "03-1234-5683", label: "チーム代表", status: "active" },
            ],
          },
        ],
      },
    ],
  },
  {
    id: "root-2",
    name: "大阪支社",
    description: "大阪支社の電話番号グループ",
    parentId: null,
    type: "folder",
    createdAt: "2024-01-05T00:00:00Z",
    updatedAt: "2024-01-20T00:00:00Z",
    children: [
      {
        id: "group-2-1",
        name: "営業部",
        description: "大阪営業部門",
        parentId: "root-2",
        type: "group",
        createdAt: "2024-01-05T00:00:00Z",
        updatedAt: "2024-01-20T00:00:00Z",
        numbers: [
          { id: "num-8", number: "06-1234-5678", label: "代表", status: "active" },
          { id: "num-9", number: "06-1234-5679", label: "直通", status: "active" },
        ],
      },
    ],
  },
  {
    id: "root-3",
    name: "共通番号",
    description: "全社共通の電話番号",
    parentId: null,
    type: "group",
    createdAt: "2024-01-10T00:00:00Z",
    updatedAt: "2024-01-25T00:00:00Z",
    numbers: [
      { id: "num-10", number: "0570-000-001", label: "ナビダイヤル", status: "active" },
      { id: "num-11", number: "050-1234-5678", label: "IP電話", status: "active" },
    ],
  },
]

interface TreeNodeProps {
  node: NumberGroup
  level: number
  selectedId: string | null
  expandedIds: Set<string>
  onSelect: (node: NumberGroup) => void
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

  const countNumbers = (group: NumberGroup): number => {
    let count = group.numbers?.length || 0
    if (group.children) {
      for (const child of group.children) {
        count += countNumbers(child)
      }
    }
    return count
  }

  const numberCount = countNumbers(node)

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
            {/* Expand/Collapse icon */}
            <span className="w-4 h-4 flex items-center justify-center shrink-0">
              {hasChildren ? (
                isExpanded ? (
                  <ChevronDown className="h-4 w-4 text-muted-foreground" />
                ) : (
                  <ChevronRight className="h-4 w-4 text-muted-foreground" />
                )
              ) : null}
            </span>

            {/* Folder/Group icon */}
            {isFolder ? (
              isExpanded ? (
                <FolderOpen className="h-4 w-4 text-primary shrink-0" />
              ) : (
                <Folder className="h-4 w-4 text-primary shrink-0" />
              )
            ) : (
              <Users className="h-4 w-4 text-muted-foreground shrink-0" />
            )}

            {/* Name */}
            <span className="flex-1 truncate text-sm">{node.name}</span>

            {/* Count badge */}
            {numberCount > 0 && (
              <Badge variant="secondary" className="text-xs h-5 px-1.5">
                {numberCount}
              </Badge>
            )}

            {/* Actions (visible on hover) */}
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
                      <Users className="h-4 w-4 mr-2" />
                      グループを追加
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

          {/* Children */}
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
              <Users className="h-4 w-4 mr-2" />
              グループを追加
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

interface NumberDetailProps {
  group: NumberGroup | null
}

function NumberDetail({ group }: NumberDetailProps) {
  if (!group) {
    return (
      <div className="flex-1 flex items-center justify-center text-muted-foreground">
        <div className="text-center">
          <Users className="h-12 w-12 mx-auto mb-4 opacity-50" />
          <p>グループを選択してください</p>
        </div>
      </div>
    )
  }

  const isFolder = group.type === "folder"

  return (
    <div className="flex-1 flex flex-col">
      {/* Header */}
      <div className="p-4 border-b">
        <div className="flex items-start justify-between">
          <div className="flex items-center gap-3">
            {isFolder ? (
              <div className="w-10 h-10 rounded-lg bg-primary/10 flex items-center justify-center">
                <Folder className="h-5 w-5 text-primary" />
              </div>
            ) : (
              <div className="w-10 h-10 rounded-lg bg-muted flex items-center justify-center">
                <Users className="h-5 w-5 text-muted-foreground" />
              </div>
            )}
            <div>
              <h2 className="text-lg font-semibold">{group.name}</h2>
              {group.description && (
                <p className="text-sm text-muted-foreground">{group.description}</p>
              )}
            </div>
          </div>
          <Button variant="outline" size="sm">
            <Edit2 className="h-4 w-4 mr-2" />
            編集
          </Button>
        </div>
      </div>

      {/* Content */}
      <ScrollArea className="flex-1">
        <div className="p-4">
          {isFolder ? (
            <div className="space-y-4">
              <p className="text-muted-foreground text-sm">
                このフォルダには {group.children?.length || 0} 個のサブアイテムがあります。
              </p>
              {group.children && group.children.length > 0 && (
                <div className="grid gap-3">
                  {group.children.map((child) => (
                    <Card key={child.id} className="hover:bg-accent/50 transition-colors cursor-pointer">
                      <CardContent className="p-4 flex items-center gap-3">
                        {child.type === "folder" ? (
                          <Folder className="h-5 w-5 text-primary shrink-0" />
                        ) : (
                          <Users className="h-5 w-5 text-muted-foreground shrink-0" />
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
                  {group.numbers?.length || 0} 件の電話番号
                </p>
                <Button size="sm">
                  <Plus className="h-4 w-4 mr-2" />
                  番号を追加
                </Button>
              </div>

              {group.numbers && group.numbers.length > 0 ? (
                <div className="space-y-2">
                  {group.numbers.map((num) => (
                    <NumberRow key={num.id} number={num} />
                  ))}
                </div>
              ) : (
                <Card className="border-dashed">
                  <CardContent className="p-6 text-center">
                    <Phone className="h-8 w-8 mx-auto mb-3 text-muted-foreground opacity-50" />
                    <p className="text-muted-foreground">電話番号がありません</p>
                    <Button variant="outline" size="sm" className="mt-4 bg-transparent">
                      <Plus className="h-4 w-4 mr-2" />
                      番号を追加
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

function NumberRow({ number }: { number: PhoneNumber }) {
  return (
    <div className="flex items-center gap-3 p-3 rounded-lg border bg-card hover:bg-accent/50 transition-colors group">
      <div className="w-8 h-8 rounded-full bg-primary/10 flex items-center justify-center shrink-0">
        <PhoneCall className="h-4 w-4 text-primary" />
      </div>
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <span className="font-mono font-medium">{number.number}</span>
          <Badge
            variant={number.status === "active" ? "default" : "secondary"}
            className={cn(
              "text-xs",
              number.status === "active" && "bg-green-500/10 text-green-600 hover:bg-green-500/20"
            )}
          >
            {number.status === "active" ? "有効" : "無効"}
          </Badge>
        </div>
        <p className="text-sm text-muted-foreground">{number.label}</p>
      </div>
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
            番号をコピー
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

export function NumberGroupsContent() {
  const [searchQuery, setSearchQuery] = useState("")
  const [selectedGroup, setSelectedGroup] = useState<NumberGroup | null>(null)
  const [expandedIds, setExpandedIds] = useState<Set<string>>(new Set(["root-1"]))

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

  const filterGroups = (groups: NumberGroup[], query: string): NumberGroup[] => {
    if (!query) return groups

    return groups
      .map((group) => {
        const matchesName = group.name.toLowerCase().includes(query.toLowerCase())
        const matchesNumber = group.numbers?.some((n) =>
          n.number.includes(query) || n.label.toLowerCase().includes(query.toLowerCase())
        )
        const filteredChildren = group.children ? filterGroups(group.children, query) : []

        if (matchesName || matchesNumber || filteredChildren.length > 0) {
          return {
            ...group,
            children: filteredChildren.length > 0 ? filteredChildren : group.children,
          }
        }
        return null
      })
      .filter((g): g is NumberGroup => g !== null)
  }

  const filteredGroups = filterGroups(mockGroups, searchQuery)

  return (
    <div className="flex flex-col h-full">
      {/* Page Header */}
      <div className="p-6 border-b">
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-2xl font-bold text-balance">番号グループ</h1>
            <p className="text-muted-foreground">電話番号をグループで管理</p>
          </div>
          <div className="flex items-center gap-2">
            <Button variant="outline">
              <FolderPlus className="h-4 w-4 mr-2" />
              フォルダ作成
            </Button>
            <Button>
              <Plus className="h-4 w-4 mr-2" />
              グループ作成
            </Button>
          </div>
        </div>
      </div>

      {/* Content Area */}
      <div className="flex flex-1 min-h-0">
        {/* Tree Panel */}
        <div className="w-80 border-r flex flex-col bg-card">
          {/* Search */}
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

          {/* Tree */}
          <ScrollArea className="flex-1">
            <div className="p-2">
              {filteredGroups.map((group) => (
                <TreeNode
                  key={group.id}
                  node={group}
                  level={0}
                  selectedId={selectedGroup?.id || null}
                  expandedIds={expandedIds}
                  onSelect={setSelectedGroup}
                  onToggle={handleToggle}
                />
              ))}

              {filteredGroups.length === 0 && (
                <div className="p-4 text-center text-muted-foreground text-sm">
                  該当するグループがありません
                </div>
              )}
            </div>
          </ScrollArea>
        </div>

        {/* Detail Panel */}
        <NumberDetail group={selectedGroup} />
      </div>
    </div>
  )
}
