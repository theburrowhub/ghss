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
  // Filters controlled from App (persist when navigating to/from audit).
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

  const owners = useMemo(() => ["(all)", ...Array.from(new Set(repos.map((r) => r.owner))).sort()], [repos]);

  useEffect(() => {
    onStatus(`${targets.size} target repos selected · reference: ${reference ?? "none"}`);
  }, [targets, reference, onStatus]);

  const matchesFilter = (r: RepoInfo) =>
    (owner === "(all)" || r.owner === owner) &&
    (teamRepos === null || teamRepos.has(r.full_name)) &&
    (showArchived || !r.archived) &&
    r.full_name.toLowerCase().includes(search.toLowerCase());

  const refRepo = repos.find((r) => r.full_name === reference) ?? null;
  // The reference is removed from the common list: shown pinned at the top as "featured".
  const visible = repos.filter((r) => r.full_name !== reference && matchesFilter(r));
  // Archived repos are read-only: they can't be sync targets.
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
        <input type="text" placeholder="Search repos…" value={search} onChange={(e) => onSearch(e.target.value)} style={{ maxWidth: 320 }} />
        <select value={owner} onChange={(e) => onOwner(e.target.value)}>
          {owners.map((o) => <option key={o}>{o}</option>)}
        </select>
        {owner !== "(all)" && teams.length > 0 && (
          <select value={teamSlug} onChange={(e) => onTeamSlug(e.target.value)} title="Filter by organization team">
            <option value="(all)">All teams</option>
            {teams.map((t) => <option key={t.slug} value={t.slug}>{t.name}</option>)}
          </select>
        )}
        {teamBusy && <span className="spinner spinner-sm" />}
        <label className="muted" style={{ display: "inline-flex", alignItems: "center", gap: 4 }}>
          <input type="checkbox" checked={showArchived} onChange={(e) => onShowArchived(e.target.checked)} /> archived
        </label>
        <div className="spacer" style={{ flex: 1 }} />
        <span className="muted">{targets.size} targets</span>
        <button className="primary" disabled={!reference || targets.size === 0 || busy} onClick={onAudit}>
          {busy ? "Auditing…" : "Audit differences"}
        </button>
      </div>

      {refRepo ? (
        <div className="card ref-panel">
          <span className="ref-icon">★</span>
          <div style={{ flex: 1 }}>
            <div className="ref-label">Reference · configuration source</div>
            <span className="mono">{refRepo.full_name}</span>
            {refRepo.private && <span className="badge" style={{ marginLeft: 8 }}>private</span>}
          </div>
          <button onClick={() => onReference(null)}>Remove reference</button>
        </div>
      ) : (
        <div className="card muted" style={{ marginBottom: 14 }}>
          Mark a repository as <strong>reference</strong> using the "Use as reference" button on its row. It will be the source from which configuration is copied.
        </div>
      )}

      <div className="list-toolbar">
        <button onClick={selectAll} disabled={selectableVisible.length === 0 || allSelected}>
          Select all ({selectableVisible.length})
        </button>
        <button onClick={deselectAll} disabled={selectedVisible === 0}>Deselect all</button>
        <span className="muted">
          {selectedVisible} of {selectableVisible.length} selectable in current filter
        </span>
      </div>

      <div className="card" style={{ padding: 0 }}>
        {visible.map((r) => (
          <div className="repo-row" key={r.full_name}>
            <input
              type="checkbox"
              title={
                r.archived ? "Archived repo (read-only): can't be a target, only a reference"
                : r.admin ? "Select as target"
                : "You need admin permission to sync this repo"
              }
              disabled={!r.admin || r.archived}
              checked={targets.has(r.full_name)}
              onChange={() => toggleTarget(r.full_name)}
            />
            <span className="mono">{r.full_name}</span>
            {r.private && <span className="badge">private</span>}
            {r.archived && <span className="badge muted">archived</span>}
            {!r.admin && <span className="badge muted">no admin</span>}
            <span className="muted" style={{ flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
              {r.description}
            </span>
            <button className="ref-btn" title="Use as reference (configuration source)" onClick={() => markReference(r.full_name)}>
              Use as reference
            </button>
          </div>
        ))}
        {visible.length === 0 && <p className="muted" style={{ padding: 16 }}>No results.</p>}
      </div>
    </div>
  );
}
