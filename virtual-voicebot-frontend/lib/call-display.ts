import type { Call } from "@/lib/types"

export type DisplayStatus =
  | "通話中"
  | "常時着信拒否"
  | "不在着信"
  | "応答なし"
  | "IVR離脱"
  | "留守電"
  | "通話終了"

export function resolveDisplayStatus(call: Call): DisplayStatus {
  if (call.status === "ringing" || call.status === "in_call") return "通話中"
  if (call.direction === "outbound") {
    return call.transferAnsweredAt !== null ? "通話終了" : "応答なし"
  }
  if (call.callDisposition === "denied") return "常時着信拒否"
  if (call.endReason === "cancelled" && call.answeredAt === null) return "不在着信"

  if (call.actionCode === "IV") {
    if (call.transferStatus === "answered") return "通話終了"
    if (call.transferStatus === "trying" || call.transferStatus === "failed") return "不在着信"
    return "IVR離脱"
  }

  if (call.actionCode === "VM") return "留守電"
  if (call.actionCode === "VB") return "通話終了"

  if (call.actionCode === "VR") {
    return call.answeredAt === null ? "不在着信" : "通話終了"
  }

  return "通話終了"
}

export function resolveDisplayDuration(call: Call): number {
  if (call.callDisposition === "denied") return 0
  if (call.actionCode === "VR" && call.answeredAt === null) return 0
  return call.durationSec ?? 0
}

export function displayStatusClass(status: DisplayStatus): string {
  switch (status) {
    case "通話中":
      return "bg-sky-500/15 text-sky-600 dark:text-sky-300"
    case "常時着信拒否":
      return "bg-neutral-500/15 text-neutral-600 dark:text-neutral-300"
    case "不在着信":
      return "bg-rose-500/15 text-rose-600 dark:text-rose-300"
    case "応答なし":
      return "bg-orange-500/15 text-orange-600 dark:text-orange-300"
    case "IVR離脱":
      return "bg-amber-500/15 text-amber-600 dark:text-amber-300"
    case "留守電":
      return "bg-blue-500/15 text-blue-600 dark:text-blue-300"
    case "通話終了":
      return "bg-emerald-500/15 text-emerald-600 dark:text-emerald-300"
    default:
      return "bg-muted text-muted-foreground"
  }
}
