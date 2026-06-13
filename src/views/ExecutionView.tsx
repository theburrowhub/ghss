import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import type { RepoSyncResult } from "../types";

interface Props {
  results: RepoSyncResult[] | null; // null = en curso
  onDone: () => void;
}

export function ExecutionView({ results, onDone }: Props) {
  const [progress, setProgress] = useState<{ repo: string; action: string }[]>([]);

  useEffect(() => {
    const un = listen<{ repo: string; action: string }>("sync-progress", (e) => {
      setProgress((p) => [...p, e.payload]);
    });
    return () => { un.then((f) => f()); };
  }, []);

  return (
    <div className="view">
      <h3>{results ? "Resultado de la sincronización" : "Sincronizando…"}</h3>
      {!results && (
        <div className="card">
          {progress.map((p, i) => (
            <div key={i}><span className="mono">{p.repo}</span> — {p.action}</div>
          ))}
          {progress.length === 0 && <p className="muted">Preparando…</p>}
        </div>
      )}
      {results?.map((r) => (
        <div className="card" key={r.repo} style={{ marginBottom: 8 }}>
          <h4 style={{ marginTop: 0 }} className="mono">{r.repo}</h4>
          {r.fatal && <p style={{ color: "var(--danger)" }}>Error: {r.fatal}</p>}
          {r.results.map((a, i) => (
            <div key={i} style={{ display: "flex", gap: 8 }}>
              <span>{a.ok ? "✅" : "❌"}</span>
              <span style={{ flex: 1 }}>{a.description}</span>
              {a.error && <span className="muted">{a.error}</span>}
            </div>
          ))}
        </div>
      ))}
      {results && <button className="primary" onClick={onDone}>Volver a repos</button>}
    </div>
  );
}
