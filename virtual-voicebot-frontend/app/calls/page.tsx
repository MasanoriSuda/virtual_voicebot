import { getCalls } from "@/lib/api"
import { CallHistoryContent } from "@/components/call-history-content"

export const metadata = {
  title: "発着信履歴 - Call History | VoiceBot Admin",
  description: "通話履歴の一覧を表示します",
}

export default async function CallsPage() {
  const calls = await getCalls()

  return <CallHistoryContent initialCalls={calls} />
}
