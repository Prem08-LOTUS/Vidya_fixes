import { SystemStatus } from "../types";

export default function StatusBar({ error, status }: { error: string | null, status?: SystemStatus | null }) {
  // Use status.last_error if available, otherwise global error
  const displayError = error || status?.last_error;

  return (
    <div className="flex justify-between items-center bg-zinc-900 border border-zinc-800 p-3 rounded-lg">
      <div className="font-bold text-blue-400">AGNIX SCIENTIFIC CONSOLE</div>
      <div className={`font-mono text-sm ${displayError ? "text-red-500" : "text-green-400"}`}>
        {displayError ?? "SYSTEM ONLINE"}
      </div>
    </div>
  );
}
