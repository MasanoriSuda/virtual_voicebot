export interface VoicebotScenario {
  id: string
  name: string
  description: string | null
  isActive: boolean
  voicevoxStyleId: number
  systemPrompt: string | null
  createdAt: string
  updatedAt: string
}

export interface ScenariosDatabase {
  scenarios: VoicebotScenario[]
}

export function createDefaultScenariosDatabase(): ScenariosDatabase {
  return {
    scenarios: [],
  }
}
