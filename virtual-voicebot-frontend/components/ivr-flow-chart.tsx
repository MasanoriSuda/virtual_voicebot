import type { IvrSessionEvent } from "@/lib/types"

function unique(values: Array<string | null>): string[] {
  const set = new Set<string>()
  for (const value of values) {
    if (value && value.trim().length > 0) {
      set.add(value)
    }
  }
  return [...set]
}

export function IvrFlowChart({ events }: { events: IvrSessionEvent[] }) {
  const nodePath = events
    .filter((event) => event.eventType === "node_enter")
    .map((event) => event.nodeId)
    .filter((value): value is string => Boolean(value))

  const transitionPath = events
    .filter((event) => event.eventType === "transition")
    .map((event) => event.transitionId)
    .filter((value): value is string => Boolean(value))

  const visitedNodes = unique(nodePath)

  if (events.length === 0) {
    return <p className="text-sm text-muted-foreground">表示できる経路データがありません。</p>
  }

  return (
    <div className="space-y-4">
      <section className="rounded-xl border p-4">
        <h3 className="mb-3 text-sm font-semibold">訪問ノード</h3>
        {visitedNodes.length === 0 ? (
          <p className="text-sm text-muted-foreground">ノード訪問イベントなし</p>
        ) : (
          <div className="flex flex-wrap gap-2">
            {visitedNodes.map((nodeId, index) => (
              <span
                key={nodeId}
                className="rounded-full border bg-muted/40 px-3 py-1 text-xs font-mono"
              >
                {index + 1}. {nodeId}
              </span>
            ))}
          </div>
        )}
      </section>

      <section className="rounded-xl border p-4">
        <h3 className="mb-3 text-sm font-semibold">遷移ID</h3>
        {transitionPath.length === 0 ? (
          <p className="text-sm text-muted-foreground">遷移イベントなし</p>
        ) : (
          <ol className="space-y-2">
            {transitionPath.map((transitionId, index) => (
              <li key={`${transitionId}-${index}`} className="text-xs font-mono">
                {index + 1}. {transitionId}
              </li>
            ))}
          </ol>
        )}
      </section>
    </div>
  )
}
