import React, { useState, useEffect } from 'react';
import { AlertTriangle, Power, CheckCircle, RotateCcw, Anchor } from 'lucide-react';
import { invoke } from "@tauri-apps/api/tauri";

type RecoveryState =
  | { type: 'FreshStart' }
  | { type: 'CleanShutdown', snapshot: any }
  | { type: 'PowerLossDuringMotion', snapshot: any, intent: any }
  | { type: 'CorruptedData' };

interface Props {
  onRecover: (action: 'RESUME' | 'REHOME' | 'ABORT') => void;
  onVisibilityChange?: (visible: boolean) => void;
}

export const RecoveryScreen: React.FC<Props> = ({ onRecover, onVisibilityChange }) => {
  const [state, setState] = useState<RecoveryState | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    // Corrected to use Tauri Invoke instead of fetch
    invoke('get_recovery_state')
      .then((data: any) => {
        setState(data);
        setLoading(false);
        const isVisible = data && data.type !== 'FreshStart';
        if (onVisibilityChange) onVisibilityChange(isVisible);
      })
      .catch(err => {
         console.error("Recovery Check Failed", err);
         setLoading(false);
      });
  }, []);

  if (loading) return <div className="bg-zinc-950 h-screen flex items-center justify-center text-white font-mono">ANALYZING BLACK BOX...</div>;
  if (!state || state.type === 'FreshStart') return null;

  const isCritical = state.type === 'PowerLossDuringMotion' || state.type === 'CorruptedData';

  return (
    <div className="fixed inset-0 z-50 bg-black/95 backdrop-blur-md flex items-center justify-center p-4 font-mono">
      <div className="bg-zinc-900 border border-zinc-700 max-w-3xl w-full rounded-xl shadow-2xl overflow-hidden">

        {/* Header */}
        <div className={`p-6 ${isCritical ? 'bg-red-950/40 border-b border-red-900' : 'bg-emerald-950/40 border-b border-emerald-900'} flex items-center gap-6`}>
          {isCritical ? <AlertTriangle className="text-red-500 w-12 h-12" /> : <CheckCircle className="text-emerald-500 w-12 h-12" />}
          <div>
            <h1 className="text-2xl font-bold text-white tracking-tight">SYSTEM RECOVERY MODE</h1>
            <p className="text-zinc-400 mt-1">
              {state.type === 'PowerLossDuringMotion' ? 'UNEXPECTED SHUTDOWN DETECTED' : 'SYSTEM READY TO RESUME'}
            </p>
          </div>
        </div>

        {/* Content */}
        <div className="p-8 space-y-8">

          {/* Diagnostic Info */}
          <div className="grid grid-cols-2 gap-6">
            <div className="bg-zinc-950 p-4 rounded border border-zinc-800">
              <span className="text-xs text-zinc-500 uppercase font-bold tracking-wider">Last Known Position (Z)</span>
              <div className="text-3xl font-bold text-cyan-400 mt-2">
                {state.type !== 'CorruptedData' && state.type !== 'FreshStart' ?
                  `${state.snapshot.z_pos_nm.toFixed(2)} nm` : 'UNKNOWN'}
              </div>
            </div>
            <div className="bg-zinc-950 p-4 rounded border border-zinc-800">
              <span className="text-xs text-zinc-500 uppercase font-bold tracking-wider">Shutdown Reason</span>
              <div className="text-xl text-white mt-2">
                {state.type === 'PowerLossDuringMotion' ? 'POWER LOSS IN MOTION' : 'CLEAN SHUTDOWN'}
              </div>
            </div>
          </div>

          {/* Context for User */}
          {state.type === 'PowerLossDuringMotion' && (
             <div className="bg-red-950/30 border border-red-900/50 p-4 rounded text-red-200 text-sm leading-relaxed">
               <strong className="text-red-400">WARNING:</strong> Power was lost while the system was moving.
               <div className="mt-2 font-mono text-xs bg-black/50 p-2 rounded border border-red-900/30">
                 INTENT: {JSON.stringify(state.intent)}
               </div>
               <div className="mt-2">Physical position may vary from snapshot. <strong>Re-homing is strongly recommended.</strong></div>
             </div>
          )}

          {/* Actions */}
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4 pt-4">

            <button
              onClick={() => onRecover('REHOME')}
              className="group flex flex-col items-center justify-center p-6 bg-zinc-800 hover:bg-cyan-950/30 border border-zinc-700 hover:border-cyan-500/50 rounded-lg transition-all"
            >
              <Anchor className="w-8 h-8 text-cyan-500 mb-3" />
              <span className="font-bold text-white">RE-HOME & RESTART</span>
              <span className="text-xs text-zinc-500 mt-1">Full Calibration</span>
            </button>

            <button
              onClick={() => onRecover('RESUME')}
              disabled={isCritical}
              className={`flex flex-col items-center justify-center p-6 border rounded-lg transition-all ${
                isCritical
                ? 'bg-zinc-900 border-zinc-800 opacity-50 cursor-not-allowed'
                : 'bg-zinc-800 hover:bg-emerald-950/30 border-zinc-700 hover:border-emerald-500/50'
              }`}
            >
              <RotateCcw className={`w-8 h-8 mb-3 ${isCritical ? 'text-zinc-600' : 'text-emerald-500'}`} />
              <span className="font-bold text-white">RESUME JOB</span>
              <span className="text-xs text-zinc-500 mt-1">Continue from Snapshot</span>
            </button>

            <button
              onClick={() => onRecover('ABORT')}
              className="flex flex-col items-center justify-center p-6 bg-zinc-800 hover:bg-red-950/30 border border-zinc-700 hover:border-red-500/50 rounded-lg transition-all"
            >
              <Power className="w-8 h-8 text-red-500 mb-3" />
              <span className="font-bold text-white">ABORT JOB</span>
              <span className="text-xs text-zinc-500 mt-1">Reset to Idle</span>
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};
