import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import type { RepoSyncResult } from "../types";
import { friendlyError } from "./StatusBar";

interface Props {
  results: RepoSyncResult[] | null; // null = in progress
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

  // Status per repo: ok (all applied), partial (some failures), failed (nothing applied).
  const repoStatus = (r: RepoSyncResult): "ok" | "partial" | "failed" => {
    if (r.fatal) return "failed";
    const failed = r.results.filter((a) => !a.ok).length;
    if (failed === 0) return "ok";
    return failed === r.results.length ? "failed" : "partial";
  };

  const okRepos = results?.filter((r) => repoStatus(r) === "ok").length ?? 0;
  const failedActions = results?.reduce((n, r) => n + r.results.filter((a) => !a.ok).length + (r.fatal ? 1 : 0), 0) ?? 0;

  return (
    <div className="view">
      <h3>{results ? "Sync results" : "Syncing…"}</h3>

      {!results && (
        <div className="card">
          {progress.map((p, i) => (
            <div key={i}><span className="mono">{p.repo}</span> — {p.action}</div>
          ))}
          {progress.length === 0 && <p className="muted">Preparing…</p>}
        </div>
      )}

      {results && (
        <div className="card" style={{ marginBottom: 12, display: "flex", gap: 16, alignItems: "center" }}>
          <span><strong>{okRepos}</strong> of {results.length} repos with no issues</span>
          {failedActions > 0 && <span className="badge err">{failedActions} failed actions</span>}
          <span className="muted">Failures on one repo don't affect the others.</span>
        </div>
      )}

      {results?.map((r) => {
        const st = repoStatus(r);
        return (
          <div className="card" key={r.repo} style={{ marginBottom: 8 }}>
            <div style={{ display: "flex", gap: 10, alignItems: "center", marginBottom: r.results.length || r.fatal ? 8 : 0 }}>
              <span className="mono">{r.repo}</span>
              {st === "ok" && <span className="badge ok">✓ applied</span>}
              {st === "partial" && <span className="badge diff">partial</span>}
              {st === "failed" && <span className="badge err">failed</span>}
            </div>
            {r.fatal && <p className="muted" style={{ color: "var(--danger)", margin: 0 }}>{friendlyError(r.fatal)}</p>}
            {r.results.map((a, i) => (
              <div key={i} style={{ display: "flex", gap: 8, alignItems: "center", padding: "2px 0" }}>
                <span>{a.ok ? "✅" : "❌"}</span>
                <span style={{ flex: 1 }}>{a.description}</span>
                {a.error && <span className="muted" style={{ color: "var(--danger)" }}>{friendlyError(a.error)}</span>}
              </div>
            ))}
          </div>
        );
      })}

      {results && <button className="primary" onClick={onDone} style={{ marginTop: 8 }}>Back to repos</button>}
    </div>
  );
}
