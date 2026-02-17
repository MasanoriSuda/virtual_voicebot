import Link from "next/link"
import { notFound } from "next/navigation"

import { IvrFlowChart } from "@/components/ivr-flow-chart"
import { IvrTraceTimeline } from "@/components/ivr-trace-timeline"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { getCall, getIvrSessionEvents } from "@/lib/api"

interface PageProps {
  params: Promise<{ callId: string }>
}

function formatDateTime(value: string | null): string {
  if (!value) return "-"
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) {
    return "-"
  }
  return new Intl.DateTimeFormat("ja-JP", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  }).format(date)
}

export default async function IvrTracePage({ params }: PageProps) {
  const { callId } = await params
  const call = await getCall(callId)

  if (!call) {
    notFound()
  }

  const events = await getIvrSessionEvents(call.id)

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-center justify-between gap-3">
        <div>
          <h1 className="text-2xl font-bold">IVR経路詳細</h1>
          <p className="text-sm text-muted-foreground">通話ID: {call.externalCallId}</p>
        </div>
        <Link href="/calls" className="text-sm text-primary underline-offset-2 hover:underline">
          発着信履歴へ戻る
        </Link>
      </div>

      <section className="rounded-2xl border bg-card/70 p-4 text-sm">
        <div className="grid gap-2 md:grid-cols-2">
          <p>発信者: {call.callerNumber ?? "非通知"}</p>
          <p>着信応答: {call.callDisposition}</p>
          <p>開始: {formatDateTime(call.startedAt)}</p>
          <p>終了: {formatDateTime(call.endedAt)}</p>
          <p>実行アクション: {call.finalAction ?? "-"}</p>
          <p>転送状況: {call.transferStatus}</p>
        </div>
      </section>

      <Tabs defaultValue="timeline">
        <TabsList>
          <TabsTrigger value="timeline">タイムライン</TabsTrigger>
          <TabsTrigger value="flowchart">フローチャート</TabsTrigger>
        </TabsList>
        <TabsContent value="timeline" className="mt-4">
          <IvrTraceTimeline events={events} call={call} />
        </TabsContent>
        <TabsContent value="flowchart" className="mt-4">
          <IvrFlowChart events={events} />
        </TabsContent>
      </Tabs>
    </div>
  )
}
