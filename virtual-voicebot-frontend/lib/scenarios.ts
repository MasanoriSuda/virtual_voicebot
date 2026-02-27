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
  const now = new Date().toISOString()
  return {
    scenarios: [
      {
        id: "scenario-default",
        name: "Temporary Default Scenario",
        description: "Temporary seed for Refs #212",
        isActive: true,
        voicevoxStyleId: 3,
        systemPrompt: "You are a polite phone assistant. Respond briefly in Japanese.",
        createdAt: now,
        updatedAt: now,
      },
    ],
  }
}
