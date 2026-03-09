import { useEffect, useState, useRef } from "react";
import { getStatus, getSystemHealth } from "./api";
import { SystemStatus } from "./types";
import { invoke } from "@tauri-apps/api/tauri";

import StatusBar from "./components/StatusBar";
import SensorPanel from "./components/SensorPanel";
import SafetyIndicator from "./components/SafetyIndicator";
import ZChart from "./components/ZChart";
import ConsoleLog from "./components/ConsoleLog";
import CommandPanel from "./components/CommandPanel";
import { RecoveryScreen } from "./components/RecoveryScreen";
import { Power } from "lucide-react";

export default function App() {
  const [status, setStatus] = useState<SystemStatus | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [history, setHistory] = useState<{ t: number; z: number }[]>([]);
  const [halted, setHalted] = useState(false);
  const [recoveryVisible, setRecoveryVisible] = useState(false);
  const pollRef = useRef(false);

  const handleRecovery = async (action: 'RESUME' | 'REHOME' | 'ABORT') => {
    try {
      await invoke("perform_recovery", { action });
      // Force immediate re-poll or state reset if needed
      // If we resumed, recovery screen will hide itself on next check?
      // Actually RecoveryScreen only checks on mount.
      // So we might need to force reload or manually hide.
      // For now, we assume backend state updates and we might reload page or similar.
      // But purely for UI fix:
      if (action !== 'ABORT') {
          // If we are recovering, we might want to hide the overlay to show progress?
          // But RecoveryScreen logic is static.
          // Let's stick to the visibility coordination.
      }
    } catch (e) {
      console.error("Recovery Failed:", e);
      setError("RECOVERY FAILED: " + String(e));
    }
  };

  useEffect(() => {
    pollRef.current = true;

    const poll = async () => {
      try {
        // Check health first (Binary Truth)
        const health = await getSystemHealth();
        if (health === "HALTED") {
          setHalted(true);
        } else {
            setHalted(false);
        }

        const s = await getStatus();
        setStatus(s);
        setError(null);

        // Process burst buffer if available, else standard push
        if (s.history && s.history.length > 0) {
             // Append burst data
             const newPoints = s.history.map(p => ({ t: p.timestamp_ms, z: p.z_nm }));
             setHistory(h => {
                 const combined = [...h, ...newPoints];
                 // Sort and dedupe if necessary, but simple slice is ok for now
                 return combined.slice(-500); // Larger buffer for burst data
             });
        } else {
             // Fallback
             setHistory(h => [...h, { t: Date.now(), z: s.z_nm }].slice(-200));
        }

      } catch (e) {
        setError("BACKEND DISCONNECTED");
        // Fix: If disconnected, ensure status reflects unsafe or is cleared
        setStatus(null);
      }

      if (pollRef.current) {
        setTimeout(poll, 200); // 5 Hz UI refresh
      }
    };

    poll();
    return () => { pollRef.current = false; };
  }, []);

  return (
    <>
      <RecoveryScreen
        onRecover={handleRecovery}
        onVisibilityChange={setRecoveryVisible}
      />

      {halted && !recoveryVisible ? (
        <div className="w-screen h-screen bg-red-950 text-red-200 flex items-center justify-center">
          <div className="text-center">
            <h1 className="text-4xl font-bold">SYSTEM HALTED</h1>
            <p className="mt-4 font-mono text-xl mb-8">
              Supervisor triggered emergency stop.<br/>
              Hardware is de-energized.<br/>
              Manual intervention required.
            </p>
            <button
              onClick={() => handleRecovery('ABORT')}
              className="flex items-center gap-2 px-6 py-3 bg-red-800 hover:bg-red-700 text-white rounded font-bold mx-auto transition-colors"
            >
              <Power className="w-6 h-6" />
              EMERGENCY RESET
            </button>
          </div>
        </div>
      ) : (
        <div className="h-screen p-4 grid grid-rows-[auto_1fr_auto] gap-4">
          <StatusBar error={error} status={status} />

          <div className="grid grid-cols-4 gap-4 overflow-hidden">
            <div className="col-span-1 flex flex-col gap-4">
              <SafetyIndicator status={status} />
              <SensorPanel status={status} />
              <ConsoleLog status={status} error={error} />
            </div>

            <div className="col-span-3">
              <ZChart data={history} />
            </div>
          </div>

          <CommandPanel safe={status?.safe ?? false} />
        </div>
      )}
    </>
  );
}
