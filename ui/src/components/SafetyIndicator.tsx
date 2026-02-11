import { ShieldCheck, ShieldAlert } from "lucide-react";
import { SystemStatus } from "../types";

export default function SafetyIndicator({ status }: { status: SystemStatus | null }) {
  // Fix UI Safety: If status is missing, it is UNSAFE.
  // We default to unsafe if null.
  const safe = status ? status.safe : false;

  return (
    <div className={`p-4 rounded-lg border ${
      safe ? "border-green-700 bg-green-900/20" : "border-red-700 bg-red-900/30"
    }`}>
      <div className="flex items-center gap-2 font-bold">
        {safe ? <ShieldCheck /> : <ShieldAlert />}
        {safe ? "SAFE TO OPERATE" : (status ? "INTERLOCK ACTIVE" : "CONNECTION LOST")}
      </div>
    </div>
  );
}
