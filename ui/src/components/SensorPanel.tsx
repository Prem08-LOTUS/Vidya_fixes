import { SystemStatus } from "../types";

export default function SensorPanel({ status }: { status: SystemStatus | null }) {
  if (!status) return null;

  return (
    <div className="bg-zinc-900 border border-zinc-800 p-4 rounded-lg space-y-2">
      <div>🌡 Temp: {status.temperature.toFixed(2)} °C</div>

      {/* ASML-Grade Metrics */}
      <div className="flex justify-between text-sm">
        <span className="text-zinc-400">∇T:</span>
        <span className={status.thermal_gradient && status.thermal_gradient > 0.1 ? "text-yellow-500" : "text-green-500"}>
          {status.thermal_gradient?.toFixed(3) ?? "0.000"} K/m
        </span>
      </div>

      <div>💧 RH: {status.rh.toFixed(2)} %</div>

      <div className="border-t border-zinc-800 pt-2">
        <div>📏 Z: {status.z_nm.toFixed(2)} nm</div>
        <div className="flex justify-between text-sm">
           <span className="text-zinc-400">σ:</span>
           <span className={status.z_sigma && status.z_sigma > 1.0 ? "text-red-500" : "text-zinc-300"}>
             ±{status.z_sigma?.toFixed(2) ?? "0.00"} nm
           </span>
        </div>
      </div>

      <div className="text-xs font-mono text-zinc-400 pt-1">
        Updated: {new Date(status.timestamp).toLocaleTimeString()}
      </div>
    </div>
  );
}
