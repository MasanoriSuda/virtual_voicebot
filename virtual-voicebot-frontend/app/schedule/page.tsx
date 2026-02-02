import { ScheduleContent } from "@/components/schedule-content"

export const metadata = {
  title: "スケジュール - Schedule | VoiceBot Admin",
  description: "営業時間・休日スケジュールの管理",
}

export default function SchedulePage() {
  return (
    <div className="h-full">
      <ScheduleContent />
    </div>
  )
}
