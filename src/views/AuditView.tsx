import { useState } from "react";
import { DiffTree } from "../components/DiffTree";
import type { AuditResult } from "../types";

interface Props {
  reference: string;
  result: AuditResult;
  onBack: () => void;
  onProceed: (repos: string[]) => void;
}

export function AuditView({ reference, result, onBack, onProceed }: Props) {
  const [onlyDiverged, setOnlyDiverged] = useState(false);
  const [open, setOpen] = useState<Set<string>>(new Set());
  const diverged = result.diffs.filter((d) => d.changes.length > 0);
  const visible = onlyDiverged ? diverged : result.diffs;

  return (
    <div className="view">
      <div style={{ display: "flex", gap: 10, alignItems: "center", marginBottom: 14 }}>
        <button onClick={onBack}>← Repos</button>
        <h3 style={{ margin: 0 }}>Auditoría contra <span className="mono">{reference}</span></h3>
        <div style={{ flex: 1 }} />
        <label><input type="checkbox" checked={onlyDiverged} onChange={(e) => setOnlyDiverged(e.target.checked)} /> solo desincronizados</label>
        <button className="primary" disabled={diverged.length === 0} onClick={() => onProceed(diverged.map((d) => d.repo))}>
          Sincronizar los {diverged.length} divergentes →
        </button>
      </div>

      {result.errors.map(([repo, err]) => (
        <div className="card" key={repo} style={{ marginBottom: 8, borderColor: "var(--danger)" }}>
          <span className="mono">{repo}</span> <span className="badge err">error</span> <span className="muted">{err}</span>
        </div>
      ))}

      {visible.map((d) => {
        const isOpen = open.has(d.repo);
        return (
          <div className="card" key={d.repo} style={{ marginBottom: 8 }}>
            <div
              style={{ display: "flex", gap: 10, alignItems: "center", cursor: "pointer" }}
              onClick={() => setOpen((prev) => { const n = new Set(prev); n.has(d.repo) ? n.delete(d.repo) : n.add(d.repo); return n; })}
            >
              <span>{isOpen ? "▼" : "▶"}</span>
              <span className="mono">{d.repo}</span>
              {d.changes.length === 0
                ? <span className="badge ok">✓ en sync</span>
                : <span className="badge diff">✗ {d.changes.length} diferencias</span>}
            </div>
            {isOpen && d.changes.length > 0 && (
              <div style={{ marginTop: 10 }}>
                <DiffTree changes={d.changes} selectable={false} selected={new Set()} onSelectedChange={() => {}} />
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}
