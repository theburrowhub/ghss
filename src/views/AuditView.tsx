import { useEffect, useMemo, useRef, useState } from "react";
import { DiffTree } from "../components/DiffTree";
import { friendlyError } from "./StatusBar";
import type { AuditResult, SettingChange } from "../types";

interface Props {
  reference: string;
  result: AuditResult;
  onBack: () => void;
  onSync: (plans: { repo: string; changes: SettingChange[] }[]) => void;
  onStatus: (s: string) => void;
  busy: boolean;
}

export function AuditView({ reference, result, onBack, onSync, onStatus, busy }: Props) {
  const [onlyDiverged, setOnlyDiverged] = useState(true);
  const [open, setOpen] = useState<Set<string>>(new Set());
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const known = useRef<Set<string>>(new Set());

  const streaming = result.streaming === true;
  const processed = result.diffs.length + result.errors.length;
  const total = result.total ?? processed;
  const diverged = result.diffs.filter((d) => d.changes.length > 0);
  const visible = onlyDiverged ? diverged : result.diffs;

  // Por defecto, cada cambio aplicable entra seleccionado; los toggles del usuario se respetan
  // (known guarda lo ya visto para no re-seleccionar lo que el usuario desmarcó al llegar repos nuevos).
  useEffect(() => {
    setSelected((prev) => {
      const next = new Set(prev);
      for (const d of result.diffs) {
        for (const c of d.changes) {
          if (!c.applicable) continue;
          const k = `${d.repo}::${c.key}`;
          if (!known.current.has(k)) {
            known.current.add(k);
            next.add(k);
          }
        }
      }
      return next;
    });
  }, [result.diffs]);

  const allKeys = useMemo(() => {
    const ks: string[] = [];
    for (const d of diverged) for (const c of d.changes) if (c.applicable) ks.push(`${d.repo}::${c.key}`);
    return ks;
  }, [diverged]);

  const selectedCount = allKeys.filter((k) => selected.has(k)).length;
  const selectAll = () => setSelected(new Set(allKeys));
  const deselectAll = () => setSelected(new Set());

  const plans = diverged
    .map((d) => ({ repo: d.repo, changes: d.changes.filter((c) => selected.has(`${d.repo}::${c.key}`)) }))
    .filter((p) => p.changes.length > 0);
  const totalChanges = plans.reduce((n, p) => n + p.changes.length, 0);

  useEffect(() => {
    if (streaming) onStatus(`Auditando… ${processed} de ${total} repos`);
    else onStatus(`${selectedCount} de ${allKeys.length} cambios marcados · ${plans.length} repos a sincronizar`);
  }, [streaming, processed, total, selectedCount, allKeys.length, plans.length, onStatus]);

  return (
    <div className="view">
      <div style={{ display: "flex", gap: 10, alignItems: "center", marginBottom: 14 }}>
        <button onClick={onBack}>← Repos</button>
        <h3 style={{ margin: 0 }}>Auditoría contra <span className="mono">{reference}</span></h3>
        <div style={{ flex: 1 }} />
        {streaming && (
          <span className="muted" style={{ display: "inline-flex", alignItems: "center", gap: 8 }}>
            <span className="spinner spinner-sm" /> Auditando… {processed} de {total}
          </span>
        )}
        <label><input type="checkbox" checked={onlyDiverged} onChange={(e) => setOnlyDiverged(e.target.checked)} /> solo desincronizados</label>
        <button className="primary" disabled={streaming || totalChanges === 0 || busy} onClick={() => onSync(plans)}>
          {streaming ? "Esperando…" : `Sincronizar ${totalChanges} cambios en ${plans.length} repos`}
        </button>
      </div>

      {streaming && (
        <div className="progress-bar" style={{ marginBottom: 12 }}>
          <div className="progress-fill" style={{ width: total > 0 ? `${(processed / total) * 100}%` : "0%" }} />
        </div>
      )}

      <div className="list-toolbar">
        <button onClick={selectAll} disabled={allKeys.length === 0}>Marcar todo</button>
        <button onClick={deselectAll} disabled={selectedCount === 0}>Desmarcar todo</button>
        <span className="muted">{selectedCount} de {allKeys.length} cambios seleccionados · {diverged.length} repos divergentes</span>
      </div>

      {result.errors.map(([repo, err]) => (
        <div className="card" key={repo} style={{ marginBottom: 8, borderColor: "var(--danger)" }}>
          <span className="mono">{repo}</span> <span className="badge err">no auditado</span> <span className="muted">{friendlyError(err)}</span>
        </div>
      ))}

      {visible.map((d) => {
        const isOpen = open.has(d.repo);
        const repoSelected = d.changes.filter((c) => c.applicable && selected.has(`${d.repo}::${c.key}`)).length;
        const repoApplicable = d.changes.filter((c) => c.applicable).length;
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
              {d.changes.length > 0 && (
                <span className="muted" style={{ marginLeft: "auto" }}>{repoSelected}/{repoApplicable} marcados</span>
              )}
            </div>
            {isOpen && d.changes.length > 0 && (
              <div style={{ marginTop: 10 }}>
                <DiffTree
                  changes={d.changes}
                  selectable={true}
                  selected={new Set(d.changes.filter((c) => selected.has(`${d.repo}::${c.key}`)).map((c) => c.key))}
                  onSelectedChange={(next) =>
                    setSelected((prev) => {
                      const out = new Set(prev);
                      d.changes.forEach((c) => {
                        const comp = `${d.repo}::${c.key}`;
                        next.has(c.key) ? out.add(comp) : out.delete(comp);
                      });
                      return out;
                    })
                  }
                />
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}
