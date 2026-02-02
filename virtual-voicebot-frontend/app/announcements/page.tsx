import { AnnouncementsContent } from "@/components/announcements-content"

export const metadata = {
  title: "アナウンス - Announcements | VoiceBot Admin",
  description: "音声アナウンスの管理",
}

export default function AnnouncementsPage() {
  return (
    <div className="h-full">
      <AnnouncementsContent />
    </div>
  )
}
