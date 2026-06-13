import { useMemo, useState } from "react";
import { DiffTree } from "../components/DiffTree";
import type { RepoDiff, SettingChange } from "../types";

interface Props {
  reference: string;
  diffs: RepoDiff[]; // solo repos divergentes
  onBack: () => void;
  onSync: (plans: { repo: string; changes: SettingChange[] }[]) => void;
  busy: boolean;
}

export function PreSyncView({ reference, diffs, onBack, onSync, busy }: Props) {
  const allKeys = useMemo(
    () => new Set(diffs.flatMap((d) => d.changes.filter((c) => c.applicable).map((c) => `${d.repo}::${c.key}`))),
    [diffs],
  );
  const [selected, setSelected] = useState<Set<string>>(allKeys);

  const plans = diffs
    .map((d) => ({ repo: d.repo, changes: d.changes.filter((c) => selected.has(`${d.repo}::${c.key}`)) }))
    .filter((p) => p.changes.length > 0);
  const totalChanges = plans.reduce((n, p) => n + p.changes.length, 0);

  return (
    <div className="view">
      <div style={{ display: "flex", gap: 10, alignItems: "center", marginBottom: 14 }}>
        <button onClick={onBack}>← Auditoría</button>
        <h3 style={{ margin: 0 }}>Pre-sync desde <span className="mono">{reference}</span></h3>
        <div style={{ flex: 1 }} />
        <button className="primary" disabled={totalChanges === 0 || busy} onClick={() => onSync(plans)}>
          {busy ? "Sincronizando…" : `Sincronizar ${totalChanges} cambios en ${plans.length} repos`}
        </button>
      </div>
      <p className="muted">Desmarca los settings que no quieras propagar. Los no aplicables aparecen deshabilitados.</p>
      {diffs.map((d) => (
        <div className="card" key={d.repo} style={{ marginBottom: 10 }}>
          <h4 style={{ marginTop: 0 }} className="mono">{d.repo}</h4>
          <DiffTree
            changes={d.changes}
            selectable={true}
            selected={new Set(d.changes.filter((c) => selected.has(`${d.repo}::${c.key}`)).map((c) => c.key))}
            onSelectedChange={(next) => {
              setSelected((prev) => {
                const out = new Set(prev);
                d.changes.forEach((c) => {
                  const composite = `${d.repo}::${c.key}`;
                  next.has(c.key) ? out.add(composite) : out.delete(composite);
                });
                return out;
              });
            }}
          />
        </div>
      ))}
    </div>
  );
}
