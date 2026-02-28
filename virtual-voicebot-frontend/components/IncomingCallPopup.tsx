"use client"

import { useState } from "react"
import { PhoneIncoming, X } from "lucide-react"

import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { useIncomingCallNotifications } from "@/hooks/useIncomingCallNotifications"

function triggerLabel(trigger: "direct" | "ivr_transfer"): string {
  return trigger === "ivr_transfer" ? "IVR 転送" : "直接着信"
}

function formatReceivedAt(value: string): string {
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) {
    return value
  }
  return date.toLocaleTimeString("ja-JP", { hour12: false })
}

export function IncomingCallPopup() {
  const { notifications, error, dismiss } = useIncomingCallNotifications()
  const [dismissError, setDismissError] = useState<string | null>(null)

  const onDismiss = async (id: string) => {
    try {
      await dismiss(id)
      setDismissError(null)
    } catch (err) {
      setDismissError(err instanceof Error ? err.message : "通知の削除に失敗しました")
    }
  }

  if (notifications.length === 0 && !error && !dismissError) {
    return null
  }

  return (
    <div className="pointer-events-none fixed top-4 right-4 z-50 w-[calc(100%-2rem)] max-w-sm space-y-3">
      {error ? (
        <Card className="pointer-events-auto border-destructive/30 bg-destructive/5">
          <CardContent className="p-3 text-sm text-destructive">
            着信通知の取得に失敗しました: {error}
          </CardContent>
        </Card>
      ) : null}

      {dismissError ? (
        <Card className="pointer-events-auto border-destructive/30 bg-destructive/5">
          <CardContent className="p-3 text-sm text-destructive">{dismissError}</CardContent>
        </Card>
      ) : null}

      {notifications.map((notification) => (
        <Card key={notification.id} className="pointer-events-auto border-primary/30 shadow-lg">
          <CardHeader className="pb-2">
            <div className="flex items-start justify-between gap-3">
              <CardTitle className="flex items-center gap-2 text-base">
                <PhoneIncoming className="h-4 w-4 text-primary" />
                着信通知
              </CardTitle>
              <Button
                type="button"
                variant="ghost"
                size="icon"
                className="h-7 w-7"
                onClick={() => void onDismiss(notification.id)}
                aria-label="通知を閉じる"
              >
                <X className="h-4 w-4" />
              </Button>
            </div>
          </CardHeader>
          <CardContent className="space-y-1.5 text-sm">
            <div>
              <span className="text-muted-foreground">発信者番号:</span> {notification.callerNumber}
            </div>
            <div>
              <span className="text-muted-foreground">着信種別:</span>{" "}
              {triggerLabel(notification.trigger)}
            </div>
            <div>
              <span className="text-muted-foreground">検知時刻:</span>{" "}
              {formatReceivedAt(notification.receivedAt)}
            </div>
            {notification.trigger === "ivr_transfer" && notification.ivrData ? (
              <>
                <div>
                  <span className="text-muted-foreground">IVR 滞留時間:</span>{" "}
                  {notification.ivrData.dwellTimeSec} 秒
                </div>
                <div>
                  <span className="text-muted-foreground">押下番号:</span>{" "}
                  {notification.ivrData.dtmfHistory.join(" → ")}
                </div>
              </>
            ) : null}
          </CardContent>
        </Card>
      ))}
    </div>
  )
}
