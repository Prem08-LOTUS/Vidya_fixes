import { SystemStatus } from "../types";

export default function ConsoleLog({
  status,
  error
}: {
  status: SystemStatus | null;
  error: string | null;
}) {
  return (
    <div className="bg-black text-green-400 font-mono text-xs p-3 rounded-lg h-48 overflow-auto">
      {error && `ERROR: ${error}\n`}
      {status && `SAFE=${status.safe} Z=${status.z_nm.toFixed(2)}nm\n`}
      _
    </div>
  );
}
