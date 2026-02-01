import { NumberGroupsContent } from "@/components/number-groups-content"

export const metadata = {
  title: "番号グループ - Number Groups | VoiceBot Admin",
  description: "電話番号グループの管理",
}

export default function GroupsPage() {
  return (
    <div className="h-full">
      <NumberGroupsContent />
    </div>
  )
}
