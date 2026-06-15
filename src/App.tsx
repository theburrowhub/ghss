import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { applySync, audit, listOrgTeams, listOwners, listReposForOwner, listTeamRepos, logout } from "./api";
import type { AuditRepoEvent, AuditResult, AuditStartedEvent, OwnerInfo, RepoInfo, RepoSyncResult, SettingChange, TeamInfo, UserInfo } from "./types";
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
  const [owners, setOwners] = useState<OwnerInfo[]>([]);
  const [repos, setRepos] = useState<RepoInfo[]>([]);
  const [reposBusy, setReposBusy] = useState(false);
  const [reference, setReference] = useState<string | null>(null);
  const [targets, setTargets] = useState<Set<string>>(new Set());
  const [auditResult, setAuditResult] = useState<AuditResult | null>(null);
  const [syncResults, setSyncResults] = useState<RepoSyncResult[] | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [warning, setWarning] = useState<string | null>(null);
  const [status, setStatus] = useState("");
  const [loading, setLoading] = useState<{ title: string; detail?: string }>({ title: "" });

  // Repo view filter state: lives here so it persists when navigating to/from audit.
  const [search, setSearch] = useState("");
  const [ownerFilter, setOwnerFilter] = useState(""); // "" = ninguno elegido (no se lista nada)
  const [teamSlug, setTeamSlug] = useState("(all)");
  const [teams, setTeams] = useState<TeamInfo[]>([]);
  const [teamRepos, setTeamRepos] = useState<Set<string> | null>(null); // null = no team filter
  const [teamBusy, setTeamBusy] = useState(false);
  const [showArchived, setShowArchived] = useState(false);

  // Classify the error: a 401 (invalid/expired token) ends the session and returns to login.
  const handleError = useCallback((e: unknown) => {
    const s = String(e);
    if (/\b401\b/.test(s) || s.includes("invalid session") || s.includes("unauthenticated")) {
      setUser(null);
      setStatus("");
      setStage("auth");
    }
    setError(s);
  }, []);

  // When owner changes: reset team, load teams, and fetch repos ONLY for that owner.
  // No repos are downloaded until an owner is picked (avoids the slow full /user/repos sweep).
  useEffect(() => {
    setTeamSlug("(all)");
    setTeamRepos(null);
    if (ownerFilter === "") {
      setTeams([]);
      setRepos([]);
      return;
    }
    let cancelled = false;
    listOrgTeams(ownerFilter)
      .then((t) => { if (!cancelled) setTeams(t); })
      .catch(() => { if (!cancelled) setTeams([]); });

    const isOrg = owners.find((o) => o.login === ownerFilter)?.kind === "org";
    setReposBusy(true);
    setRepos([]);
    listReposForOwner(ownerFilter, isOrg)
      .then((list) => { if (!cancelled) setRepos(list); })
      .catch((e) => { if (!cancelled) { setRepos([]); handleError(e); } })
      .finally(() => { if (!cancelled) setReposBusy(false); });
    return () => { cancelled = true; };
  }, [ownerFilter, owners, handleError]);

  // When a team is chosen: load its repos to filter the list.
  useEffect(() => {
    if (ownerFilter === "" || teamSlug === "(all)") {
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

  const onLogin = useCallback(async (u: UserInfo) => {
    setUser(u);
    setError(null);
    setWarning(u.scope_warning ?? null);
    setOwnerFilter("");
    setRepos([]);
    setLoading({ title: "Loading organizations…", detail: `Connected as ${u.login}. Fetching your organizations and personal account.` });
    setStage("loading");
    try {
      const list = await listOwners();
      setOwners(list);
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
            <button onClick={doLogout}>Sign out</button>
          </>
        )}
      </div>

      {stage === "auth" && <AuthView onLogin={onLogin} />}
      {stage === "loading" && <LoadingView title={loading.title} detail={loading.detail} />}
      {stage === "repos" && (
        <ReposView
          repos={repos}
          owners={owners}
          reposBusy={reposBusy}
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
