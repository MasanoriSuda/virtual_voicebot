"use client"

import { Bar, BarChart, ResponsiveContainer, XAxis, YAxis, Tooltip, Legend } from "recharts"

const data = [
  { hour: "00", inbound: 2, outbound: 1 },
  { hour: "01", inbound: 1, outbound: 0 },
  { hour: "02", inbound: 0, outbound: 0 },
  { hour: "03", inbound: 0, outbound: 0 },
  { hour: "04", inbound: 1, outbound: 0 },
  { hour: "05", inbound: 2, outbound: 1 },
  { hour: "06", inbound: 5, outbound: 2 },
  { hour: "07", inbound: 8, outbound: 4 },
  { hour: "08", inbound: 15, outbound: 8 },
  { hour: "09", inbound: 22, outbound: 12 },
  { hour: "10", inbound: 28, outbound: 15 },
  { hour: "11", inbound: 25, outbound: 14 },
  { hour: "12", inbound: 18, outbound: 10 },
  { hour: "13", inbound: 24, outbound: 13 },
  { hour: "14", inbound: 26, outbound: 14 },
  { hour: "15", inbound: 23, outbound: 12 },
  { hour: "16", inbound: 20, outbound: 11 },
  { hour: "17", inbound: 16, outbound: 8 },
  { hour: "18", inbound: 10, outbound: 5 },
  { hour: "19", inbound: 6, outbound: 3 },
  { hour: "20", inbound: 4, outbound: 2 },
  { hour: "21", inbound: 3, outbound: 1 },
  { hour: "22", inbound: 2, outbound: 1 },
  { hour: "23", inbound: 2, outbound: 0 },
]

export function HourlyCallChart() {
  return (
    <div className="h-[300px]">
      <ResponsiveContainer width="100%" height="100%">
        <BarChart data={data} margin={{ top: 10, right: 10, left: -20, bottom: 0 }}>
          <XAxis
            dataKey="hour"
            tick={{ fontSize: 12 }}
            tickLine={false}
            axisLine={false}
            className="text-muted-foreground"
          />
          <YAxis
            tick={{ fontSize: 12 }}
            tickLine={false}
            axisLine={false}
            className="text-muted-foreground"
          />
          <Tooltip
            contentStyle={{
              backgroundColor: "hsl(var(--card))",
              border: "1px solid hsl(var(--border))",
              borderRadius: "8px",
            }}
            labelFormatter={(value) => `${value}:00`}
          />
          <Legend
            formatter={(value) => (value === "inbound" ? "着信" : "発信")}
            wrapperStyle={{ paddingTop: 10 }}
          />
          <Bar
            dataKey="inbound"
            fill="hsl(var(--primary))"
            radius={[4, 4, 0, 0]}
            name="inbound"
          />
          <Bar
            dataKey="outbound"
            fill="hsl(var(--chart-2))"
            radius={[4, 4, 0, 0]}
            name="outbound"
          />
        </BarChart>
      </ResponsiveContainer>
    </div>
  )
}
