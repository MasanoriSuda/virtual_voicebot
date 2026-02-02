import { RoutingContent } from "@/components/routing-content"

export const metadata = {
  title: "ルーティング - Routing | VoiceBot Admin",
  description: "通話ルーティングの設定",
}

export default function RoutingPage() {
  return (
    <div className="h-full">
      <RoutingContent />
    </div>
  )
}
