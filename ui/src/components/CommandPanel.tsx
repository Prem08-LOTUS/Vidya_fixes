import { submitJob } from "../api";
import { useState } from "react";

export default function CommandPanel({ safe }: { safe: boolean }) {
  const [path, setPath] = useState("");

  return (
    <div className="flex gap-3">
      <input
        className="flex-1 bg-zinc-900 border border-zinc-800 px-3 py-2 rounded font-mono"
        placeholder="/path/to/design.gds"
        value={path}
        onChange={e => setPath(e.target.value)}
      />
      <button
        disabled={!safe}
        onClick={() => submitJob(path)}
        className={`px-6 rounded font-bold ${
          safe ? "bg-blue-600 hover:bg-blue-500" : "bg-zinc-700 cursor-not-allowed"
        }`}
      >
        EXECUTE
      </button>
    </div>
  );
}
