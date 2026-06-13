import { useEffect, useMemo } from "react";
import type { RepoInfo, TeamInfo } from "../types";

interface Props {
  repos: RepoInfo[];
  reference: string | null;
  targets: Set<string>;
  onReference: (repo: string | null) => void;
  onTargets: (next: Set<string>) => void;
  onAudit: () => void;
  onStatus: (s: string) => void;
  busy: boolean;
  // Filtros controlados desde App (persisten al ir y volver de la auditoría).
  search: string;
  onSearch: (s: string) => void;
  owner: string;
  onOwner: (o: string) => void;
  teamSlug: string;
  onTeamSlug: (s: string) => void;
  teams: TeamInfo[];
  teamRepos: Set<string> | null;
  teamBusy: boolean;
  showArchived: boolean;
  onShowArchived: (v: boolean) => void;
}

export function ReposView(props: Props) {
  const {
    repos, reference, targets, onReference, onTargets, onAudit, onStatus, busy,
    search, onSearch, owner, onOwner, teamSlug, onTeamSlug, teams, teamRepos, teamBusy,
    showArchived, onShowArchived,
  } = props;

  const owners = useMemo(() => ["(todos)", ...Array.from(new Set(repos.map((r) => r.owner))).sort()], [repos]);

  useEffect(() => {
    onStatus(`${targets.size} repos destino seleccionados · referencia: ${reference ?? "ninguna"}`);
  }, [targets, reference, onStatus]);

  const matchesFilter = (r: RepoInfo) =>
    (owner === "(todos)" || r.owner === owner) &&
    (teamRepos === null || teamRepos.has(r.full_name)) &&
    (showArchived || !r.archived) &&
    r.full_name.toLowerCase().includes(search.toLowerCase());

  const refRepo = repos.find((r) => r.full_name === reference) ?? null;
  // La referencia sale de la lista común: se muestra fija arriba como "destacado".
  const visible = repos.filter((r) => r.full_name !== reference && matchesFilter(r));
  // Los repos archivados son read-only: no pueden ser destino de sincronización.
  const selectableVisible = visible.filter((r) => r.admin && !r.archived);
  const selectedVisible = selectableVisible.filter((r) => targets.has(r.full_name)).length;
  const allSelected = selectableVisible.length > 0 && selectedVisible === selectableVisible.length;

  const toggleTarget = (full: string) => {
    const next = new Set(targets);
    next.has(full) ? next.delete(full) : next.add(full);
    onTargets(next);
  };

  const markReference = (full: string) => {
    if (targets.has(full)) {
      const next = new Set(targets);
      next.delete(full);
      onTargets(next);
    }
    onReference(full);
  };

  const selectAll = () => {
    const next = new Set(targets);
    selectableVisible.forEach((r) => next.add(r.full_name));
    onTargets(next);
  };

  const deselectAll = () => {
    const next = new Set(targets);
    selectableVisible.forEach((r) => next.delete(r.full_name));
    onTargets(next);
  };

  return (
    <div className="view">
      <div style={{ display: "flex", gap: 10, marginBottom: 14, alignItems: "center" }}>
        <input type="text" placeholder="Buscar repos…" value={search} onChange={(e) => onSearch(e.target.value)} style={{ maxWidth: 320 }} />
        <select value={owner} onChange={(e) => onOwner(e.target.value)}>
          {owners.map((o) => <option key={o}>{o}</option>)}
        </select>
        {owner !== "(todos)" && teams.length > 0 && (
          <select value={teamSlug} onChange={(e) => onTeamSlug(e.target.value)} title="Filtrar por equipo de la organización">
            <option value="(todos)">Todos los equipos</option>
            {teams.map((t) => <option key={t.slug} value={t.slug}>{t.name}</option>)}
          </select>
        )}
        {teamBusy && <span className="spinner spinner-sm" />}
        <label className="muted" style={{ display: "inline-flex", alignItems: "center", gap: 4 }}>
          <input type="checkbox" checked={showArchived} onChange={(e) => onShowArchived(e.target.checked)} /> archivados
        </label>
        <div className="spacer" style={{ flex: 1 }} />
        <span className="muted">{targets.size} destinos</span>
        <button className="primary" disabled={!reference || targets.size === 0 || busy} onClick={onAudit}>
          {busy ? "Auditando…" : "Auditar diferencias"}
        </button>
      </div>

      {refRepo ? (
        <div className="card ref-panel">
          <span className="ref-icon">★</span>
          <div style={{ flex: 1 }}>
            <div className="ref-label">Referencia · origen de la configuración</div>
            <span className="mono">{refRepo.full_name}</span>
            {refRepo.private && <span className="badge" style={{ marginLeft: 8 }}>private</span>}
          </div>
          <button onClick={() => onReference(null)}>Quitar referencia</button>
        </div>
      ) : (
        <div className="card muted" style={{ marginBottom: 14 }}>
          Marca un repositorio como <strong>referencia</strong> con el botón «Usar como referencia» de su fila. Será el origen del que se copia la configuración.
        </div>
      )}

      <div className="list-toolbar">
        <button onClick={selectAll} disabled={selectableVisible.length === 0 || allSelected}>
          Seleccionar todo ({selectableVisible.length})
        </button>
        <button onClick={deselectAll} disabled={selectedVisible === 0}>Deseleccionar todo</button>
        <span className="muted">
          {selectedVisible} de {selectableVisible.length} seleccionables en el filtro actual
        </span>
      </div>

      <div className="card" style={{ padding: 0 }}>
        {visible.map((r) => (
          <div className="repo-row" key={r.full_name}>
            <input
              type="checkbox"
              title={
                r.archived ? "Repo archivado (read-only): no puede ser destino, solo referencia"
                : r.admin ? "Seleccionar como destino"
                : "Necesitas permiso admin para sincronizar este repo"
              }
              disabled={!r.admin || r.archived}
              checked={targets.has(r.full_name)}
              onChange={() => toggleTarget(r.full_name)}
            />
            <span className="mono">{r.full_name}</span>
            {r.private && <span className="badge">private</span>}
            {r.archived && <span className="badge muted">archivado</span>}
            {!r.admin && <span className="badge muted">sin admin</span>}
            <span className="muted" style={{ flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
              {r.description}
            </span>
            <button className="ref-btn" title="Usar como referencia (origen de la configuración)" onClick={() => markReference(r.full_name)}>
              Usar como referencia
            </button>
          </div>
        ))}
        {visible.length === 0 && <p className="muted" style={{ padding: 16 }}>Sin resultados.</p>}
      </div>
    </div>
  );
}
