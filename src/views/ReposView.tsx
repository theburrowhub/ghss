import { useMemo, useState } from "react";
import type { RepoInfo } from "../types";

interface Props {
  repos: RepoInfo[];
  reference: string | null;
  targets: Set<string>;
  onReference: (repo: string | null) => void;
  onTargets: (next: Set<string>) => void;
  onAudit: () => void;
  busy: boolean;
}

export function ReposView({ repos, reference, targets, onReference, onTargets, onAudit, busy }: Props) {
  const [q, setQ] = useState("");
  const [owner, setOwner] = useState("(todos)");
  const owners = useMemo(() => ["(todos)", ...Array.from(new Set(repos.map((r) => r.owner))).sort()], [repos]);
  const visible = repos.filter(
    (r) => (owner === "(todos)" || r.owner === owner) && r.full_name.toLowerCase().includes(q.toLowerCase()),
  );

  const toggleTarget = (full: string) => {
    const next = new Set(targets);
    next.has(full) ? next.delete(full) : next.add(full);
    onTargets(next);
  };

  return (
    <div className="view">
      <div style={{ display: "flex", gap: 10, marginBottom: 14, alignItems: "center" }}>
        <input type="text" placeholder="Buscar repos…" value={q} onChange={(e) => setQ(e.target.value)} style={{ maxWidth: 320 }} />
        <select value={owner} onChange={(e) => setOwner(e.target.value)}>
          {owners.map((o) => <option key={o}>{o}</option>)}
        </select>
        <div className="spacer" style={{ flex: 1 }} />
        <span className="muted">⭐ referencia: {reference ?? "ninguna"} · {targets.size} destinos</span>
        <button className="primary" disabled={!reference || targets.size === 0 || busy} onClick={onAudit}>
          {busy ? "Auditando…" : "Auditar diferencias"}
        </button>
      </div>
      <div className="card" style={{ padding: 0 }}>
        {visible.map((r) => (
          <div className="repo-row" key={r.full_name}>
            <button
              className={`star${reference === r.full_name ? " active" : ""}`}
              title="Marcar como referencia"
              onClick={() => onReference(reference === r.full_name ? null : r.full_name)}
            >
              {reference === r.full_name ? "★" : "☆"}
            </button>
            <input
              type="checkbox"
              title={r.admin ? "Seleccionar como destino" : "Necesitas permiso admin para sincronizar este repo"}
              disabled={!r.admin || reference === r.full_name}
              checked={targets.has(r.full_name) && reference !== r.full_name}
              onChange={() => toggleTarget(r.full_name)}
            />
            <span className="mono">{r.full_name}</span>
            {r.private && <span className="badge">private</span>}
            {!r.admin && <span className="badge muted">sin admin</span>}
            <span className="muted" style={{ flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
              {r.description}
            </span>
          </div>
        ))}
        {visible.length === 0 && <p className="muted" style={{ padding: 16 }}>Sin resultados.</p>}
      </div>
    </div>
  );
}
