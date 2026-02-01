"use client"

import Link from "next/link"
import { usePathname } from "next/navigation"
import { cn } from "@/lib/utils"
import {
  LayoutDashboard,
  Phone,
  Users,
  GitBranch,
  Workflow,
  Calendar,
  Volume2,
  Settings,
  FileText,
  ChevronLeft,
  ChevronRight,
} from "lucide-react"
import { Button } from "./ui/button"
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "./ui/tooltip"

interface AdminSidebarProps {
  collapsed: boolean
  onToggle: () => void
}

const primaryNavItems = [
  { href: "/", icon: LayoutDashboard, label: "Dashboard", labelJa: "ダッシュボード" },
  { href: "/calls", icon: Phone, label: "Call History", labelJa: "発着信履歴" },
  { href: "/groups", icon: Users, label: "Number Groups", labelJa: "番号グループ" },
  { href: "/routing", icon: GitBranch, label: "Routing", labelJa: "ルーティング" },
  { href: "/ivr", icon: Workflow, label: "IVR Flow", labelJa: "IVRフロー" },
  { href: "/schedule", icon: Calendar, label: "Schedule", labelJa: "スケジュール" },
  { href: "/announcements", icon: Volume2, label: "Announcements", labelJa: "アナウンス" },
]

const secondaryNavItems = [
  { href: "/settings", icon: Settings, label: "Settings", labelJa: "設定" },
  { href: "/audit", icon: FileText, label: "Audit Log", labelJa: "監査ログ" },
]

export function AdminSidebar({ collapsed, onToggle }: AdminSidebarProps) {
  const pathname = usePathname()

  const NavItem = ({ item }: { item: (typeof primaryNavItems)[0] }) => {
    const isActive = pathname === item.href || (item.href !== "/" && pathname.startsWith(item.href))
    const Icon = item.icon

    const content = (
      <Link
        href={item.href}
        className={cn(
          "flex items-center gap-3 px-3 py-2.5 rounded-lg transition-colors",
          "hover:bg-sidebar-accent hover:text-sidebar-accent-foreground",
          isActive && "bg-sidebar-accent text-primary font-medium",
          collapsed && "justify-center px-2"
        )}
      >
        <Icon className={cn("h-5 w-5 shrink-0", isActive && "text-primary")} />
        {!collapsed && <span className="truncate">{item.labelJa}</span>}
      </Link>
    )

    if (collapsed) {
      return (
        <Tooltip delayDuration={0}>
          <TooltipTrigger asChild>{content}</TooltipTrigger>
          <TooltipContent side="right" className="flex items-center gap-2">
            <span>{item.labelJa}</span>
            <span className="text-muted-foreground text-xs">({item.label})</span>
          </TooltipContent>
        </Tooltip>
      )
    }

    return content
  }

  return (
    <TooltipProvider>
      <aside
        className={cn(
          "flex flex-col h-screen bg-sidebar border-r border-sidebar-border transition-all duration-300",
          collapsed ? "w-16" : "w-60"
        )}
      >
        {/* Logo */}
        <div className={cn("flex items-center h-16 px-4 border-b border-sidebar-border", collapsed && "justify-center px-2")}>
          <div className="flex items-center gap-2">
            <div className="w-8 h-8 rounded-lg bg-primary flex items-center justify-center">
              <Phone className="h-4 w-4 text-primary-foreground" />
            </div>
            {!collapsed && <span className="font-semibold text-sidebar-foreground">VoiceBot Admin</span>}
          </div>
        </div>

        {/* Primary Nav */}
        <nav className="flex-1 p-3 space-y-1 overflow-y-auto">
          {primaryNavItems.map((item) => (
            <NavItem key={item.href} item={item} />
          ))}
        </nav>

        {/* Secondary Nav */}
        <nav className="p-3 space-y-1 border-t border-sidebar-border">
          {secondaryNavItems.map((item) => (
            <NavItem key={item.href} item={item} />
          ))}
        </nav>

        {/* Toggle Button */}
        <div className="p-3 border-t border-sidebar-border">
          <Button
            variant="ghost"
            size="sm"
            onClick={onToggle}
            className={cn("w-full justify-center", !collapsed && "justify-start gap-2")}
          >
            {collapsed ? <ChevronRight className="h-4 w-4" /> : <ChevronLeft className="h-4 w-4" />}
            {!collapsed && <span>サイドバーを閉じる</span>}
          </Button>
        </div>
      </aside>
    </TooltipProvider>
  )
}
