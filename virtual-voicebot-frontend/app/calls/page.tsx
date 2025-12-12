import { getCalls } from "@/lib/api"
import { CallsTable } from "@/components/calls-table"
import { Phone } from "lucide-react"

export const metadata = {
  title: "通話履歴 - Call History",
  description: "通話履歴の一覧を表示します",
}

export default async function CallsPage() {
  const calls = await getCalls()

  return (
    <div className="min-h-screen bg-background">
      <div className="container mx-auto py-8 px-4">
        <div className="flex items-center gap-3 mb-8">
          <div className="p-3 bg-primary/10 rounded-lg">
            <Phone className="h-6 w-6 text-primary" />
          </div>
          <div>
            <h1 className="text-3xl font-bold text-balance">通話履歴</h1>
            <p className="text-muted-foreground">すべての通話記録を表示</p>
          </div>
        </div>

        <CallsTable calls={calls} />
      </div>
    </div>
  )
}
