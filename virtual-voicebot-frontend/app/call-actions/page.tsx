import { CallActionsContent } from "@/components/call-actions-content"

export const metadata = {
  title: "着信アクション - Call Actions | VoiceBot Admin",
  description: "発信者番号グループごとの着信アクション設定",
}

export default function CallActionsPage() {
  return (
    <div className="h-full">
      <CallActionsContent />
    </div>
  )
}
