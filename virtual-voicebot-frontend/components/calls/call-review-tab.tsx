"use client"

import { AlertTriangle, CheckCircle2, ClipboardList, MessageSquareQuote, type LucideIcon } from "lucide-react"

import { Badge } from "@/components/ui/badge"
import type { CallReview, CallReviewStatus } from "@/lib/types"
import { cn } from "@/lib/utils"

interface CallReviewTabProps {
  reviewStatus?: CallReviewStatus
  review?: CallReview | null
  isLoading?: boolean
}

export function CallReviewTab({ reviewStatus, review, isLoading = false }: CallReviewTabProps) {
  const status = reviewStatus ?? (review ? "completed" : "pending")

  if (isLoading) {
    return <StatePanel label="レビュー結果を読み込み中です" />
  }

  if (status === "failed") {
    return <StatePanel label="レビュー生成に失敗しました" tone="error" />
  }

  if (status === "skipped") {
    return <StatePanel label="通話後レビューは無効です" />
  }

  if (!review || status === "pending" || status === "processing") {
    return <StatePanel label="レビュー生成中です" />
  }

  return (
    <div className="space-y-5">
      <div className="flex items-center justify-between gap-3">
        <div>
          <p className="text-sm font-medium">通話カルテ</p>
          <p className="text-xs text-muted-foreground">AmiVoice 文字起こしとLLMレビュー</p>
        </div>
        <StatusBadge status={status} />
      </div>

      <section className="space-y-2">
        <SectionTitle icon={ClipboardList} label="概要" />
        <p className="rounded-xl border bg-card/60 p-3 text-sm leading-relaxed">{review.summary}</p>
      </section>

      <section className="space-y-2">
        <SectionTitle icon={MessageSquareQuote} label="顧客の用件" />
        <p className="text-sm leading-relaxed">{review.customerIntent || "-"}</p>
      </section>

      <section className="space-y-2">
        <SectionTitle icon={CheckCircle2} label="応答評価" />
        <div className="flex flex-wrap items-center gap-2">
          <Badge className={cn("px-2 py-0.5 text-xs", evaluationClass(review.responseEvaluation.status))}>
            {evaluationLabel(review.responseEvaluation.status)}
          </Badge>
          <span className="text-sm text-muted-foreground">{review.responseEvaluation.notes || "-"}</span>
        </div>
      </section>

      <ListSection title="未解決事項" values={review.unresolvedItems} />

      <section className="space-y-2">
        <SectionTitle icon={ClipboardList} label="次アクション" />
        {review.nextActions.length === 0 ? (
          <EmptyText />
        ) : (
          <div className="space-y-2">
            {review.nextActions.map((action, index) => (
              <div key={`${action.type}-${index}`} className="rounded-xl border bg-background/80 p-3">
                <div className="mb-1 flex items-center gap-2">
                  <Badge variant="outline" className="text-xs">
                    {priorityLabel(action.priority)}
                  </Badge>
                  <span className="text-xs text-muted-foreground">{nextActionTypeLabel(action.type)}</span>
                </div>
                <p className="text-sm leading-relaxed">{action.label}</p>
              </div>
            ))}
          </div>
        )}
      </section>

      <section className="space-y-2">
        <SectionTitle icon={AlertTriangle} label="リスクシグナル" />
        {review.riskSignals.length === 0 ? (
          <EmptyText />
        ) : (
          <div className="space-y-2">
            {review.riskSignals.map((risk, index) => (
              <div key={`${risk.type}-${index}`} className="rounded-xl border bg-background/80 p-3">
                <div className="mb-1 flex items-center gap-2">
                  <Badge className={cn("px-2 py-0.5 text-xs", severityClass(risk.severity))}>
                    {severityLabel(risk.severity)}
                  </Badge>
                  <span className="text-xs text-muted-foreground">{riskTypeLabel(risk.type)}</span>
                </div>
                <p className="text-sm leading-relaxed">{risk.label}</p>
              </div>
            ))}
          </div>
        )}
      </section>

      <section className="space-y-2">
        <SectionTitle icon={MessageSquareQuote} label="証拠発話" />
        {review.evidence.length === 0 ? (
          <EmptyText />
        ) : (
          <div className="space-y-2">
            {review.evidence.map((evidence, index) => (
              <div key={`${evidence.label}-${index}`} className="rounded-xl border bg-background/80 p-3">
                <div className="mb-1 text-xs text-muted-foreground">
                  {evidence.label} / {speakerLabel(evidence.speaker)} / {formatRange(evidence.startSec, evidence.endSec)}
                </div>
                <p className="text-sm leading-relaxed">{evidence.text}</p>
              </div>
            ))}
          </div>
        )}
      </section>
    </div>
  )
}

function StatePanel({ label, tone = "muted" }: { label: string; tone?: "muted" | "error" }) {
  return (
    <div
      className={cn(
        "rounded-xl border border-dashed py-10 text-center text-sm",
        tone === "error" ? "border-rose-300 bg-rose-50 text-rose-700" : "bg-muted/30 text-muted-foreground",
      )}
    >
      {label}
    </div>
  )
}

function StatusBadge({ status }: { status: CallReviewStatus }) {
  return (
    <Badge className={cn("px-2 py-0.5 text-xs", statusClass(status))}>
      {statusLabel(status)}
    </Badge>
  )
}

function SectionTitle({ icon: Icon, label }: { icon: LucideIcon; label: string }) {
  return (
    <div className="flex items-center gap-2 text-sm font-medium">
      <Icon className="h-4 w-4 text-muted-foreground" />
      {label}
    </div>
  )
}

function ListSection({ title, values }: { title: string; values: string[] }) {
  return (
    <section className="space-y-2">
      <SectionTitle icon={ClipboardList} label={title} />
      {values.length === 0 ? (
        <EmptyText />
      ) : (
        <ul className="space-y-1 text-sm">
          {values.map((value, index) => (
            <li key={`${value}-${index}`} className="rounded-lg bg-muted/30 px-3 py-2">
              {value}
            </li>
          ))}
        </ul>
      )}
    </section>
  )
}

function EmptyText() {
  return <p className="text-sm text-muted-foreground">該当なし</p>
}

function statusLabel(status: CallReviewStatus): string {
  switch (status) {
    case "completed":
      return "完了"
    case "processing":
      return "生成中"
    case "pending":
      return "待機中"
    case "failed":
      return "失敗"
    case "skipped":
      return "無効"
  }
}

function statusClass(status: CallReviewStatus): string {
  switch (status) {
    case "completed":
      return "bg-emerald-500/10 text-emerald-700 dark:text-emerald-300"
    case "failed":
      return "bg-rose-500/10 text-rose-700 dark:text-rose-300"
    case "skipped":
      return "bg-muted text-muted-foreground"
    default:
      return "bg-amber-500/10 text-amber-700 dark:text-amber-300"
  }
}

function evaluationLabel(status: CallReview["responseEvaluation"]["status"]): string {
  switch (status) {
    case "good":
      return "良好"
    case "needs_attention":
      return "要確認"
    case "poor":
      return "要改善"
    default:
      return "不明"
  }
}

function evaluationClass(status: CallReview["responseEvaluation"]["status"]): string {
  switch (status) {
    case "good":
      return "bg-emerald-500/10 text-emerald-700 dark:text-emerald-300"
    case "needs_attention":
      return "bg-amber-500/10 text-amber-700 dark:text-amber-300"
    case "poor":
      return "bg-rose-500/10 text-rose-700 dark:text-rose-300"
    default:
      return "bg-muted text-muted-foreground"
  }
}

function nextActionTypeLabel(type: CallReview["nextActions"][number]["type"]): string {
  switch (type) {
    case "follow_up":
      return "折り返し"
    case "confirm":
      return "確認"
    case "escalate":
      return "エスカレーション"
    case "none":
      return "対応不要"
    default:
      return "その他"
  }
}

function priorityLabel(priority: CallReview["nextActions"][number]["priority"]): string {
  switch (priority) {
    case "high":
      return "高"
    case "medium":
      return "中"
    default:
      return "低"
  }
}

function riskTypeLabel(type: CallReview["riskSignals"][number]["type"]): string {
  switch (type) {
    case "complaint_risk":
      return "クレーム予兆"
    case "confusion":
      return "混乱"
    case "urgent":
      return "緊急"
    default:
      return "その他"
  }
}

function severityLabel(severity: CallReview["riskSignals"][number]["severity"]): string {
  switch (severity) {
    case "high":
      return "高"
    case "medium":
      return "中"
    default:
      return "低"
  }
}

function severityClass(severity: CallReview["riskSignals"][number]["severity"]): string {
  switch (severity) {
    case "high":
      return "bg-rose-500/10 text-rose-700 dark:text-rose-300"
    case "medium":
      return "bg-amber-500/10 text-amber-700 dark:text-amber-300"
    default:
      return "bg-muted text-muted-foreground"
  }
}

function speakerLabel(speaker: CallReview["evidence"][number]["speaker"]): string {
  switch (speaker) {
    case "bot":
      return "Bot"
    case "caller":
      return "Caller"
    case "system":
      return "System"
    default:
      return "Unknown"
  }
}

function formatRange(startSec: number | null, endSec: number | null): string {
  if (startSec === null || endSec === null) {
    return "--:--"
  }
  return `${formatSeconds(startSec)}-${formatSeconds(endSec)}`
}

function formatSeconds(value: number): string {
  if (!Number.isFinite(value)) return "--:--"
  const mins = Math.floor(value / 60)
  const secs = Math.floor(value % 60)
  return `${mins.toString().padStart(2, "0")}:${secs.toString().padStart(2, "0")}`
}
