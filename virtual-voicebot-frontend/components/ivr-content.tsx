"use client"

import { useState } from "react"
import {
  ChevronRight,
  ChevronDown,
  Folder,
  FolderOpen,
  GitBranch,
  Plus,
  Search,
  MoreHorizontal,
  Pencil,
  Copy,
  Trash2,
  Play,
  Square,
  Phone,
  Mic,
  MessageSquare,
  PhoneForwarded,
  Voicemail,
  PhoneOff,
  GitFork,
} from "lucide-react"
import { cn } from "@/lib/utils"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Badge } from "@/components/ui/badge"
import { Switch } from "@/components/ui/switch"
import { ScrollArea } from "@/components/ui/scroll-area"
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
import type {
  LegacyIvrFolder as IvrFolder,
  LegacyIvrFlow as IvrFlow,
  LegacyIvrNode as IvrNode,
  LegacyIvrNodeType as IvrNodeType,
} from "@/lib/types"

// Mock data
const mockIvrData: IvrFolder[] = [
  {
    id: "root-1",
    name: "メインIVR",
    description: "メインの自動応答フロー",
    parentId: null,
    type: "folder",
    createdAt: "2024-01-01T00:00:00Z",
    updatedAt: "2024-01-15T00:00:00Z",
    children: [
      {
        id: "folder-1-1",
        name: "営業時間内",
        description: "営業時間内のIVRフロー",
        parentId: "root-1",
        type: "folder",
        createdAt: "2024-01-02T00:00:00Z",
        updatedAt: "2024-01-10T00:00:00Z",
        flows: [
          {
            id: "flow-1",
            name: "受付メニュー",
            description: "最初の振り分けメニュー",
            enabled: true,
            nodes: [
              { id: "n1", type: "start", name: "開始" },
              {
                id: "n2",
                type: "playback",
                name: "ウェルカムメッセージ",
                config: { prompt: "お電話ありがとうございます" },
              },
              {
                id: "n3",
                type: "menu",
                name: "メインメニュー",
                config: {
                  options: [
                    { key: "1", label: "営業", nextNodeId: "n4" },
                    { key: "2", label: "サポート", nextNodeId: "n5" },
                    { key: "9", label: "オペレーター", nextNodeId: "n6" },
                  ],
                },
              },
              {
                id: "n4",
                type: "transfer",
                name: "営業転送",
                config: { transferTarget: "sales" },
              },
              {
                id: "n5",
                type: "transfer",
                name: "サポート転送",
                config: { transferTarget: "support" },
              },
              {
                id: "n6",
                type: "transfer",
                name: "オペレーター転送",
                config: { transferTarget: "operator" },
              },
            ],
            createdAt: "2024-01-03T00:00:00Z",
            updatedAt: "2024-01-12T00:00:00Z",
          },
          {
            id: "flow-2",
            name: "サポート受付",
            description: "サポート部門への振り分け",
            enabled: true,
            nodes: [
              { id: "n1", type: "start", name: "開始" },
              {
                id: "n2",
                type: "menu",
                name: "サポートメニュー",
                config: {
                  options: [
                    { key: "1", label: "技術サポート", nextNodeId: "n3" },
                    { key: "2", label: "契約サポート", nextNodeId: "n4" },
                  ],
                },
              },
            ],
            createdAt: "2024-01-04T00:00:00Z",
            updatedAt: "2024-01-11T00:00:00Z",
          },
        ],
      },
      {
        id: "folder-1-2",
        name: "営業時間外",
        description: "営業時間外のIVRフロー",
        parentId: "root-1",
        type: "folder",
        createdAt: "2024-01-02T00:00:00Z",
        updatedAt: "2024-01-10T00:00:00Z",
        flows: [
          {
            id: "flow-3",
            name: "時間外アナウンス",
            description: "営業時間外のメッセージ再生",
            enabled: true,
            nodes: [
              { id: "n1", type: "start", name: "開始" },
              {
                id: "n2",
                type: "playback",
                name: "時間外メッセージ",
                config: { prompt: "本日の営業は終了しました" },
              },
              {
                id: "n3",
                type: "voicemail",
                name: "留守電",
              },
              { id: "n4", type: "hangup", name: "終了" },
            ],
            createdAt: "2024-01-05T00:00:00Z",
            updatedAt: "2024-01-09T00:00:00Z",
          },
        ],
      },
    ],
  },
  {
    id: "root-2",
    name: "キャンペーンIVR",
    description: "キャンペーン用の特別IVR",
    parentId: null,
    type: "folder",
    createdAt: "2024-01-10T00:00:00Z",
    updatedAt: "2024-01-20T00:00:00Z",
    flows: [
      {
        id: "flow-4",
        name: "新春キャンペーン",
        description: "新春セールの案内",
        enabled: false,
        nodes: [
          { id: "n1", type: "start", name: "開始" },
          {
            id: "n2",
            type: "playback",
            name: "キャンペーン案内",
            config: { prompt: "新春キャンペーン実施中" },
          },
          {
            id: "n3",
            type: "condition",
            name: "時間判定",
            config: { condition: "time >= 09:00 && time <= 18:00" },
          },
        ],
        createdAt: "2024-01-10T00:00:00Z",
        updatedAt: "2024-01-18T00:00:00Z",
      },
    ],
  },
  {
    id: "root-3",
    name: "緊急対応IVR",
    description: "緊急時のIVRフロー",
    parentId: null,
    type: "ivr",
    createdAt: "2024-01-15T00:00:00Z",
    updatedAt: "2024-01-25T00:00:00Z",
    flows: [
      {
        id: "flow-5",
        name: "緊急連絡",
        description: "緊急時の直接転送",
        enabled: true,
        nodes: [
          { id: "n1", type: "start", name: "開始" },
          {
            id: "n2",
            type: "playback",
            name: "緊急メッセージ",
            config: { prompt: "緊急連絡です" },
          },
          {
            id: "n3",
            type: "transfer",
            name: "緊急転送",
            config: { transferTarget: "emergency" },
          },
        ],
        createdAt: "2024-01-15T00:00:00Z",
        updatedAt: "2024-01-20T00:00:00Z",
      },
    ],
  },
]

function getNodeIcon(type: IvrNodeType) {
  switch (type) {
    case "start":
      return Play
    case "menu":
      return MessageSquare
    case "input":
      return Mic
    case "playback":
      return Play
    case "transfer":
      return PhoneForwarded
    case "voicemail":
      return Voicemail
    case "hangup":
      return PhoneOff
    case "condition":
      return GitFork
    default:
      return Phone
  }
}

function getNodeTypeLabel(type: IvrNodeType) {
  switch (type) {
    case "start":
      return "開始"
    case "menu":
      return "メニュー"
    case "input":
      return "入力"
    case "playback":
      return "再生"
    case "transfer":
      return "転送"
    case "voicemail":
      return "留守電"
    case "hangup":
      return "終了"
    case "condition":
      return "条件分岐"
    default:
      return type
  }
}

interface TreeItemProps {
  item: IvrFolder
  level: number
  selectedId: string | null
  expandedIds: Set<string>
  onSelect: (item: IvrFolder) => void
  onToggle: (id: string) => void
}

function TreeItem({
  item,
  level,
  selectedId,
  expandedIds,
  onSelect,
  onToggle,
}: TreeItemProps) {
  const isExpanded = expandedIds.has(item.id)
  const isSelected = selectedId === item.id
  const hasChildren =
    (item.children && item.children.length > 0) ||
    (item.flows && item.flows.length > 0)
  const isFolder = item.type === "folder"

  return (
    <ContextMenu>
      <ContextMenuTrigger>
        <div>
          <div
            className={cn(
              "group flex items-center gap-1 py-1.5 px-2 rounded-md cursor-pointer transition-colors",
              isSelected
                ? "bg-primary/10 text-primary"
                : "hover:bg-muted text-foreground"
            )}
            style={{ paddingLeft: `${level * 16 + 8}px` }}
            onClick={() => onSelect(item)}
          >
            <button
              type="button"
              className={cn(
                "p-0.5 rounded hover:bg-muted-foreground/10",
                !hasChildren && "invisible"
              )}
              onClick={(e) => {
                e.stopPropagation()
                onToggle(item.id)
              }}
            >
              {isExpanded ? (
                <ChevronDown className="h-4 w-4" />
              ) : (
                <ChevronRight className="h-4 w-4" />
              )}
            </button>

            {isFolder ? (
              isExpanded ? (
                <FolderOpen className="h-4 w-4 text-primary" />
              ) : (
                <Folder className="h-4 w-4 text-primary" />
              )
            ) : (
              <GitBranch className="h-4 w-4 text-muted-foreground" />
            )}

            <span className="flex-1 truncate text-sm">{item.name}</span>

            {item.flows && item.flows.length > 0 && (
              <Badge variant="secondary" className="text-xs px-1.5 py-0">
                {item.flows.length}
              </Badge>
            )}

            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <button
                  type="button"
                  className="p-1 rounded opacity-0 group-hover:opacity-100 hover:bg-muted-foreground/10"
                  onClick={(e) => e.stopPropagation()}
                >
                  <MoreHorizontal className="h-4 w-4" />
                </button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                <DropdownMenuItem>
                  <Pencil className="h-4 w-4 mr-2" />
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

          {isExpanded && (
            <>
              {item.children?.map((child) => (
                <TreeItem
                  key={child.id}
                  item={child}
                  level={level + 1}
                  selectedId={selectedId}
                  expandedIds={expandedIds}
                  onSelect={onSelect}
                  onToggle={onToggle}
                />
              ))}
              {item.flows?.map((flow) => (
                <FlowItem
                  key={flow.id}
                  flow={flow}
                  level={level + 1}
                  selectedId={selectedId}
                  onSelect={() =>
                    onSelect({ ...item, id: flow.id, name: flow.name })
                  }
                />
              ))}
            </>
          )}
        </div>
      </ContextMenuTrigger>
      <ContextMenuContent>
        <ContextMenuItem>
          <Plus className="h-4 w-4 mr-2" />
          新規フローを追加
        </ContextMenuItem>
        <ContextMenuItem>
          <Folder className="h-4 w-4 mr-2" />
          新規フォルダを追加
        </ContextMenuItem>
        <ContextMenuSeparator />
        <ContextMenuItem>
          <Pencil className="h-4 w-4 mr-2" />
          編集
        </ContextMenuItem>
        <ContextMenuItem>
          <Copy className="h-4 w-4 mr-2" />
          複製
        </ContextMenuItem>
        <ContextMenuSeparator />
        <ContextMenuItem className="text-destructive">
          <Trash2 className="h-4 w-4 mr-2" />
          削除
        </ContextMenuItem>
      </ContextMenuContent>
    </ContextMenu>
  )
}

interface FlowItemProps {
  flow: IvrFlow
  level: number
  selectedId: string | null
  onSelect: () => void
}

function FlowItem({ flow, level, selectedId, onSelect }: FlowItemProps) {
  const isSelected = selectedId === flow.id

  return (
    <div
      className={cn(
        "group flex items-center gap-2 py-1.5 px-2 rounded-md cursor-pointer transition-colors",
        isSelected
          ? "bg-primary/10 text-primary"
          : "hover:bg-muted text-foreground"
      )}
      style={{ paddingLeft: `${level * 16 + 28}px` }}
      onClick={onSelect}
    >
      <GitBranch className="h-4 w-4 text-muted-foreground" />
      <span className="flex-1 truncate text-sm">{flow.name}</span>
      <Badge
        variant={flow.enabled ? "default" : "secondary"}
        className="text-xs px-1.5 py-0"
      >
        {flow.enabled ? "有効" : "無効"}
      </Badge>
    </div>
  )
}

function findFlowById(folders: IvrFolder[], id: string): IvrFlow | null {
  for (const folder of folders) {
    if (folder.flows) {
      const flow = folder.flows.find((f) => f.id === id)
      if (flow) return flow
    }
    if (folder.children) {
      const found = findFlowById(folder.children, id)
      if (found) return found
    }
  }
  return null
}

function findFolderById(folders: IvrFolder[], id: string): IvrFolder | null {
  for (const folder of folders) {
    if (folder.id === id) return folder
    if (folder.children) {
      const found = findFolderById(folder.children, id)
      if (found) return found
    }
  }
  return null
}

interface FlowDetailProps {
  flow: IvrFlow
}

function FlowDetail({ flow }: FlowDetailProps) {
  return (
    <div className="space-y-6">
      <div className="flex items-start justify-between">
        <div>
          <h2 className="text-xl font-semibold">{flow.name}</h2>
          {flow.description && (
            <p className="text-muted-foreground mt-1">{flow.description}</p>
          )}
        </div>
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-2">
            <span className="text-sm text-muted-foreground">有効</span>
            <Switch checked={flow.enabled} />
          </div>
          <Button variant="outline" size="sm">
            <Pencil className="h-4 w-4 mr-2" />
            編集
          </Button>
        </div>
      </div>

      <div className="grid grid-cols-2 gap-4 text-sm">
        <div className="bg-muted/50 rounded-lg p-3">
          <div className="text-muted-foreground">ノード数</div>
          <div className="text-lg font-medium">{flow.nodes.length}</div>
        </div>
        <div className="bg-muted/50 rounded-lg p-3">
          <div className="text-muted-foreground">ステータス</div>
          <div className="text-lg font-medium">
            {flow.enabled ? "有効" : "無効"}
          </div>
        </div>
      </div>

      <div>
        <h3 className="font-medium mb-3">フローノード</h3>
        <div className="space-y-2">
          {flow.nodes.map((node, index) => {
            const Icon = getNodeIcon(node.type)
            return (
              <div
                key={node.id}
                className="flex items-center gap-3 p-3 bg-muted/30 rounded-lg border"
              >
                <div className="flex items-center justify-center w-8 h-8 rounded-full bg-primary/10 text-primary">
                  <Icon className="h-4 w-4" />
                </div>
                <div className="flex-1">
                  <div className="font-medium text-sm">{node.name}</div>
                  <div className="text-xs text-muted-foreground">
                    {getNodeTypeLabel(node.type)}
                    {node.config?.prompt && ` - "${node.config.prompt}"`}
                  </div>
                </div>
                <Badge variant="outline" className="text-xs">
                  {index + 1}
                </Badge>
              </div>
            )
          })}
        </div>
      </div>

      <div className="grid grid-cols-2 gap-4 text-sm text-muted-foreground">
        <div>
          作成日: {new Date(flow.createdAt).toLocaleDateString("ja-JP")}
        </div>
        <div>
          更新日: {new Date(flow.updatedAt).toLocaleDateString("ja-JP")}
        </div>
      </div>
    </div>
  )
}

interface FolderDetailProps {
  folder: IvrFolder
}

function FolderDetail({ folder }: FolderDetailProps) {
  const childCount = folder.children?.length ?? 0
  const flowCount = folder.flows?.length ?? 0

  return (
    <div className="space-y-6">
      <div className="flex items-start justify-between">
        <div className="flex items-center gap-3">
          <div className="p-3 bg-primary/10 rounded-lg">
            <Folder className="h-6 w-6 text-primary" />
          </div>
          <div>
            <h2 className="text-xl font-semibold">{folder.name}</h2>
            {folder.description && (
              <p className="text-muted-foreground mt-1">{folder.description}</p>
            )}
          </div>
        </div>
        <Button variant="outline" size="sm">
          <Pencil className="h-4 w-4 mr-2" />
          編集
        </Button>
      </div>

      <div className="grid grid-cols-2 gap-4 text-sm">
        <div className="bg-muted/50 rounded-lg p-3">
          <div className="text-muted-foreground">サブフォルダ</div>
          <div className="text-lg font-medium">{childCount}</div>
        </div>
        <div className="bg-muted/50 rounded-lg p-3">
          <div className="text-muted-foreground">IVRフロー</div>
          <div className="text-lg font-medium">{flowCount}</div>
        </div>
      </div>

      {folder.flows && folder.flows.length > 0 && (
        <div>
          <h3 className="font-medium mb-3">IVRフロー</h3>
          <div className="space-y-2">
            {folder.flows.map((flow) => (
              <div
                key={flow.id}
                className="flex items-center justify-between p-3 bg-muted/30 rounded-lg border"
              >
                <div className="flex items-center gap-3">
                  <GitBranch className="h-4 w-4 text-muted-foreground" />
                  <div>
                    <div className="font-medium text-sm">{flow.name}</div>
                    {flow.description && (
                      <div className="text-xs text-muted-foreground">
                        {flow.description}
                      </div>
                    )}
                  </div>
                </div>
                <div className="flex items-center gap-2">
                  <Badge variant="secondary" className="text-xs">
                    {flow.nodes.length} ノード
                  </Badge>
                  <Badge
                    variant={flow.enabled ? "default" : "outline"}
                    className="text-xs"
                  >
                    {flow.enabled ? "有効" : "無効"}
                  </Badge>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      <div className="grid grid-cols-2 gap-4 text-sm text-muted-foreground">
        <div>
          作成日: {new Date(folder.createdAt).toLocaleDateString("ja-JP")}
        </div>
        <div>
          更新日: {new Date(folder.updatedAt).toLocaleDateString("ja-JP")}
        </div>
      </div>
    </div>
  )
}

export function IvrContent() {
  const [searchQuery, setSearchQuery] = useState("")
  const [selectedId, setSelectedId] = useState<string | null>(null)
  const [expandedIds, setExpandedIds] = useState<Set<string>>(
    new Set(["root-1"])
  )

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

  const handleSelect = (item: IvrFolder) => {
    setSelectedId(item.id)
  }

  const selectedFlow = selectedId ? findFlowById(mockIvrData, selectedId) : null
  const selectedFolder =
    selectedId && !selectedFlow ? findFolderById(mockIvrData, selectedId) : null

  return (
    <div className="flex h-full">
      {/* Left Panel - Tree */}
      <div className="w-80 border-r bg-card flex flex-col">
        <div className="p-4 border-b space-y-3">
          <div className="flex items-center justify-between">
            <h1 className="font-semibold">IVRフロー</h1>
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button size="sm">
                  <Plus className="h-4 w-4 mr-1" />
                  新規
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                <DropdownMenuItem>
                  <Folder className="h-4 w-4 mr-2" />
                  フォルダを作成
                </DropdownMenuItem>
                <DropdownMenuItem>
                  <GitBranch className="h-4 w-4 mr-2" />
                  IVRフローを作成
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
          <div className="relative">
            <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
            <Input
              placeholder="検索..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="pl-9 h-9"
            />
          </div>
        </div>

        <ScrollArea className="flex-1">
          <div className="p-2">
            {mockIvrData.map((item) => (
              <TreeItem
                key={item.id}
                item={item}
                level={0}
                selectedId={selectedId}
                expandedIds={expandedIds}
                onSelect={handleSelect}
                onToggle={handleToggle}
              />
            ))}
          </div>
        </ScrollArea>
      </div>

      {/* Right Panel - Detail */}
      <div className="flex-1 bg-background">
        {selectedFlow ? (
          <div className="p-6">
            <FlowDetail flow={selectedFlow} />
          </div>
        ) : selectedFolder ? (
          <div className="p-6">
            <FolderDetail folder={selectedFolder} />
          </div>
        ) : (
          <div className="h-full flex items-center justify-center text-muted-foreground">
            <div className="text-center">
              <GitBranch className="h-12 w-12 mx-auto mb-4 opacity-20" />
              <p>IVRフローを選択してください</p>
            </div>
          </div>
        )}
      </div>
    </div>
  )
}
