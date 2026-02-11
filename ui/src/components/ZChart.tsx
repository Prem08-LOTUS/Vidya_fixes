import { LineChart, Line, ResponsiveContainer } from "recharts";

export default function ZChart({ data }: { data: { t: number; z: number }[] }) {
  return (
    <div className="h-full bg-zinc-900 border border-zinc-800 rounded-lg p-2">
      <ResponsiveContainer width="100%" height="100%">
        <LineChart data={data}>
          <Line
            dataKey="z"
            stroke="#3b82f6"
            dot={false}
            isAnimationActive={false}
          />
        </LineChart>
      </ResponsiveContainer>
    </div>
  );
}
