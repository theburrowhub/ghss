import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { applySync, audit, listOrgTeams, listRepos, listTeamRepos, logout } from "./api";
import type { AuditRepoEvent, AuditResult, AuditStartedEvent, RepoInfo, RepoSyncResult, SettingChange, TeamInfo, UserInfo } from "./types";
import { AuthView } from "./views/AuthView";
import { ReposView } from "./views/ReposView";
import { AuditView } from "./views/AuditView";
import { ExecutionView } from "./views/ExecutionView";
import { LoadingView } from "./views/LoadingView";
import { StatusBar } from "./views/StatusBar";

type Stage = "auth" | "loading" | "repos" | "audit" | "exec";

export default function App() {
  const [stage, setStage] = useState<Stage>("auth");
  const [user, setUser] = useState<UserInfo | null>(null);
  const [repos, setRepos] = useState<RepoInfo[]>([]);
  const [reference, setReference] = useState<string | null>(null);
  const [targets, setTargets] = useState<Set<string>>(new Set());
  const [auditResult, setAuditResult] = useState<AuditResult | null>(null);
  const [syncResults, setSyncResults] = useState<RepoSyncResult[] | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [warning, setWarning] = useState<string | null>(null);
  const [status, setStatus] = useState("");
  const [loading, setLoading] = useState<{ title: string; detail?: string }>({ title: "" });

  // Estado de filtros de la vista de repos: vive aquí para persistir al ir y volver de la auditoría.
  const [search, setSearch] = useState("");
  const [ownerFilter, setOwnerFilter] = useState("(todos)");
  const [teamSlug, setTeamSlug] = useState("(todos)");
  const [teams, setTeams] = useState<TeamInfo[]>([]);
  const [teamRepos, setTeamRepos] = useState<Set<string> | null>(null); // null = sin filtro de equipo
  const [teamBusy, setTeamBusy] = useState(false);
  const [showArchived, setShowArchived] = useState(false);

  // Al cambiar de owner: reseteamos el equipo y cargamos los equipos de esa organización.
  useEffect(() => {
    setTeamSlug("(todos)");
    setTeamRepos(null);
    if (ownerFilter === "(todos)") {
      setTeams([]);
      return;
    }
    let cancelled = false;
    listOrgTeams(ownerFilter)
      .then((t) => { if (!cancelled) setTeams(t); })
      .catch(() => { if (!cancelled) setTeams([]); });
    return () => { cancelled = true; };
  }, [ownerFilter]);

  // Al elegir un equipo: cargamos sus repos para filtrar la lista.
  useEffect(() => {
    if (ownerFilter === "(todos)" || teamSlug === "(todos)") {
      setTeamRepos(null);
      return;
    }
    let cancelled = false;
    setTeamBusy(true);
    listTeamRepos(ownerFilter, teamSlug)
      .then((r) => { if (!cancelled) setTeamRepos(new Set(r)); })
      .catch(() => { if (!cancelled) setTeamRepos(new Set()); })
      .finally(() => { if (!cancelled) setTeamBusy(false); });
    return () => { cancelled = true; };
  }, [ownerFilter, teamSlug]);

  // Clasifica el error: un 401 (token inválido/caducado) corta la sesión y vuelve a login.
  const handleError = useCallback((e: unknown) => {
    const s = String(e);
    if (/\b401\b/.test(s) || s.includes("sesión no válida") || s.includes("no autenticado")) {
      setUser(null);
      setStatus("");
      setStage("auth");
    }
    setError(s);
  }, []);

  const onLogin = useCallback(async (u: UserInfo) => {
    setUser(u);
    setError(null);
    setWarning(u.scope_warning ?? null);
    setLoading({ title: "Cargando repositorios…", detail: `Conectado como ${u.login}. Obteniendo la lista de repos accesibles.` });
    setStage("loading");
    try {
      const list = await listRepos();
      setRepos(list);
      setStage("repos");
    } catch (e) {
      handleError(e);
      setStage("auth");
    }
  }, [handleError]);

  const runAudit = async () => {
    if (!reference) return;
    setError(null);
    const targetList = Array.from(targets).filter((t) => t !== reference);
    setAuditResult({ reference: null, diffs: [], errors: [], streaming: true, total: targetList.length });
    setStage("audit");

    const accumulate = (payload: AuditRepoEvent) =>
      setAuditResult((prev) => {
        if (!prev) return prev;
        if (payload.diff) return { ...prev, diffs: [...prev.diffs, payload.diff] };
        if (payload.error) return { ...prev, errors: [...prev.errors, [payload.repo, payload.error]] };
        return prev;
      });

    const unStarted = await listen<AuditStartedEvent>("audit-started", (e) =>
      setAuditResult((prev) => (prev ? { ...prev, total: e.payload.total } : prev)),
    );
    const unRepo = await listen<AuditRepoEvent>("audit-repo", (e) => accumulate(e.payload));

    try {
      const final = await audit(reference, targetList);
      setAuditResult({ ...final, streaming: false, total: targetList.length });
    } catch (e) {
      handleError(e);
      if (!/\b401\b/.test(String(e))) setStage("repos");
    } finally {
      unStarted();
      unRepo();
    }
  };

  const runSync = async (plans: { repo: string; changes: SettingChange[] }[]) => {
    setBusy(true);
    setError(null);
    setSyncResults(null);
    setStage("exec");
    try {
      setSyncResults(await applySync(plans));
    } catch (e) {
      handleError(e);
      if (!/\b401\b/.test(String(e))) setStage("audit");
    } finally {
      setBusy(false);
    }
  };

  const doLogout = async () => {
    await logout().catch(() => {});
    setUser(null);
    setStatus("");
    setWarning(null);
    setStage("auth");
  };

  return (
    <>
      <div className="topbar">
        <strong>ghss</strong>
        <span className="muted">GitHub Settings Sync</span>
        <div className="spacer" />
        {user && (
          <>
            <img className="avatar" src={user.avatar_url} alt="" />
            <span>{user.login}</span>
            <button onClick={doLogout}>Salir</button>
          </>
        )}
      </div>

      {stage === "auth" && <AuthView onLogin={onLogin} />}
      {stage === "loading" && <LoadingView title={loading.title} detail={loading.detail} />}
      {stage === "repos" && (
        <ReposView
          repos={repos}
          reference={reference}
          targets={targets}
          onReference={setReference}
          onTargets={setTargets}
          onAudit={runAudit}
          onStatus={setStatus}
          busy={busy}
          search={search}
          onSearch={setSearch}
          owner={ownerFilter}
          onOwner={setOwnerFilter}
          teamSlug={teamSlug}
          onTeamSlug={setTeamSlug}
          teams={teams}
          teamRepos={teamRepos}
          teamBusy={teamBusy}
          showArchived={showArchived}
          onShowArchived={setShowArchived}
        />
      )}
      {stage === "audit" && reference && auditResult && (
        <AuditView
          reference={reference}
          result={auditResult}
          onBack={() => setStage("repos")}
          onSync={runSync}
          onStatus={setStatus}
          busy={busy}
        />
      )}
      {stage === "exec" && (
        <ExecutionView
          results={syncResults}
          onDone={async () => { setSyncResults(null); setAuditResult(null); setStage("repos"); }}
        />
      )}

      <StatusBar
        error={error}
        warning={warning}
        status={status}
        onDismiss={() => setError(null)}
        onDismissWarning={() => setWarning(null)}
      />
    </>
  );
}
