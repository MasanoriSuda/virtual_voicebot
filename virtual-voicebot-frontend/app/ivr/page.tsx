import { IvrContent } from "@/components/ivr-content"

export const metadata = {
  title: "IVRフロー - IVR Flows | VoiceBot Admin",
  description: "IVRフローの管理",
}

export default function IvrPage() {
  return (
    <div className="h-full">
      <IvrContent />
    </div>
  )
}
