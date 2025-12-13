import { getCallDetail } from "@/lib/api"
import { notFound } from "next/navigation"
import { CallDetailView } from "@/components/call-detail-view"

interface PageProps {
  params: Promise<{ callId: string }>
}

export async function generateMetadata({ params }: PageProps) {
  const { callId } = await params
  const call = await getCallDetail(callId)

  if (!call) {
    return {
      title: "通話が見つかりません",
    }
  }

  return {
    title: `通話詳細 - ${call.callerNumber}`,
    description: call.summary,
  }
}

export default async function CallDetailPage({ params }: PageProps) {
  const { callId } = await params
  const call = await getCallDetail(callId)

  if (!call) {
    notFound()
  }

  return <CallDetailView call={call} />
}
